use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::HashMap;

use super::{Card, Suit};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpiderSuitMode {
    One,
    Two,
    Three,
    Four,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SpiderGame {
    suit_mode: SpiderSuitMode,
    stock: Vec<Card>,
    tableau: [Vec<Card>; 10],
    completed_runs: usize,
    completed_run_suits: Vec<Suit>,
}

impl SpiderGame {
    pub fn new_with_seed(seed: u64) -> Self {
        Self::new_with_seed_and_mode(seed, SpiderSuitMode::One)
    }

    pub fn new_with_seed_and_mode(seed: u64, suit_mode: SpiderSuitMode) -> Self {
        let mut deck = spider_deck(suit_mode);
        let mut rng = StdRng::seed_from_u64(seed);
        deck.shuffle(&mut rng);

        let mut game = Self {
            suit_mode,
            stock: Vec::new(),
            tableau: std::array::from_fn(|_| Vec::new()),
            completed_runs: 0,
            completed_run_suits: Vec::new(),
        };

        let mut draw = deck.into_iter();
        for col in 0..10 {
            let col_size = if col < 4 { 6 } else { 5 };
            for row in 0..col_size {
                let mut card = draw.next().expect("spider setup consumes 54 cards");
                card.face_up = row == col_size - 1;
                game.tableau[col].push(card);
            }
        }

        for mut card in draw {
            card.face_up = false;
            game.stock.push(card);
        }

        game
    }

    pub fn suit_mode(&self) -> SpiderSuitMode {
        self.suit_mode
    }

    pub fn stock_len(&self) -> usize {
        self.stock.len()
    }

    pub fn tableau(&self) -> &[Vec<Card>; 10] {
        &self.tableau
    }

    pub fn completed_runs(&self) -> usize {
        self.completed_runs
    }

    pub fn completed_run_suits(&self) -> &[Suit] {
        &self.completed_run_suits
    }

    pub fn is_won(&self) -> bool {
        self.completed_runs >= 8
    }

    pub fn can_deal_from_stock(&self) -> bool {
        self.stock.len() >= 10 && self.tableau.iter().all(|pile| !pile.is_empty())
    }

    pub fn deal_from_stock(&mut self) -> bool {
        if !self.can_deal_from_stock() {
            return false;
        }

        for col in 0..10 {
            let Some(mut card) = self.stock.pop() else {
                return false;
            };
            card.face_up = true;
            self.tableau[col].push(card);
        }

        self.remove_completed_runs();
        true
    }

    pub fn can_move_run(&self, src: usize, start: usize, dst: usize) -> bool {
        if src == dst || src >= self.tableau.len() || dst >= self.tableau.len() {
            return false;
        }

        let source = &self.tableau[src];
        if start >= source.len() || !source[start].face_up {
            return false;
        }

        if !is_descending_run(&source[start..]) {
            return false;
        }

        let first = source[start];
        match self.tableau[dst].last() {
            None => true,
            Some(top) => top.face_up && top.rank == first.rank + 1,
        }
    }

    pub fn move_run(&mut self, src: usize, start: usize, dst: usize) -> bool {
        if !self.can_move_run(src, start, dst) {
            return false;
        }

        let moved = self.tableau[src].split_off(start);
        self.tableau[dst].extend(moved);
        self.flip_top_if_needed(src);
        self.remove_completed_runs();
        true
    }

    pub fn extract_completed_runs(&mut self) -> usize {
        let before = self.completed_runs;
        self.remove_completed_runs();
        self.completed_runs.saturating_sub(before)
    }

    pub fn cyclone_shuffle_tableau(&mut self) -> bool {
        let original = self.tableau.clone();
        let column_geometry: Vec<(usize, usize)> = self
            .tableau
            .iter()
            .map(|pile| {
                let face_down = pile.iter().filter(|card| !card.face_up).count();
                let face_up = pile.len().saturating_sub(face_down);
                (face_down, face_up)
            })
            .collect();

        let mut cards: Vec<Card> = self
            .tableau
            .iter()
            .flat_map(|pile| pile.iter().copied())
            .collect();
        if cards.len() < 2 {
            return false;
        }

        let mut rng = rand::thread_rng();
        for _ in 0..8 {
            cards.shuffle(&mut rng);

            let mut cursor = 0_usize;
            for (col, pile) in self.tableau.iter_mut().enumerate() {
                let (face_down, face_up) = column_geometry[col];
                let pile_len = face_down + face_up;
                pile.clear();
                for idx in 0..pile_len {
                    let mut card = cards[cursor];
                    cursor += 1;
                    card.face_up = idx >= face_down;
                    pile.push(card);
                }
            }

            if self.tableau != original {
                return true;
            }
        }

        false
    }

    pub fn tableau_card(&self, col: usize, index: usize) -> Option<Card> {
        self.tableau
            .get(col)
            .and_then(|pile| pile.get(index))
            .copied()
    }

    pub fn encode_for_session(&self) -> String {
        let parts = [
            format!("mode={}", self.suit_mode.session_tag()),
            format!("done={}", self.completed_runs),
            format!(
                "runs={}",
                encode_spider_completed_run_suits(&self.completed_run_suits)
            ),
            format!("stock={}", encode_spider_pile(&self.stock)),
            format!("t0={}", encode_spider_pile(&self.tableau[0])),
            format!("t1={}", encode_spider_pile(&self.tableau[1])),
            format!("t2={}", encode_spider_pile(&self.tableau[2])),
            format!("t3={}", encode_spider_pile(&self.tableau[3])),
            format!("t4={}", encode_spider_pile(&self.tableau[4])),
            format!("t5={}", encode_spider_pile(&self.tableau[5])),
            format!("t6={}", encode_spider_pile(&self.tableau[6])),
            format!("t7={}", encode_spider_pile(&self.tableau[7])),
            format!("t8={}", encode_spider_pile(&self.tableau[8])),
            format!("t9={}", encode_spider_pile(&self.tableau[9])),
        ];
        parts.join(";")
    }

    pub fn decode_from_session(data: &str) -> Option<Self> {
        let mut fields = HashMap::<&str, &str>::new();
        for part in data.split(';') {
            let (key, value) = part.split_once('=')?;
            fields.insert(key, value);
        }

        let suit_mode = SpiderSuitMode::from_session_tag(fields.get("mode")?)?;
        let completed_runs = fields.get("done")?.parse::<usize>().ok()?;
        if completed_runs > 8 {
            return None;
        }
        let completed_run_suits = match fields.get("runs") {
            Some(encoded) => decode_spider_completed_run_suits(encoded, completed_runs)?,
            None => vec![Suit::Spades; completed_runs],
        };

        let stock = decode_spider_pile(fields.get("stock")?)?;
        let tableau = [
            decode_spider_pile(fields.get("t0")?)?,
            decode_spider_pile(fields.get("t1")?)?,
            decode_spider_pile(fields.get("t2")?)?,
            decode_spider_pile(fields.get("t3")?)?,
            decode_spider_pile(fields.get("t4")?)?,
            decode_spider_pile(fields.get("t5")?)?,
            decode_spider_pile(fields.get("t6")?)?,
            decode_spider_pile(fields.get("t7")?)?,
            decode_spider_pile(fields.get("t8")?)?,
            decode_spider_pile(fields.get("t9")?)?,
        ];

        let tableau_count: usize = tableau.iter().map(Vec::len).sum();
        if stock.len() + tableau_count + (completed_runs * 13) != 104 {
            return None;
        }

        Some(Self {
            suit_mode,
            stock,
            tableau,
            completed_runs,
            completed_run_suits,
        })
    }

    fn flip_top_if_needed(&mut self, col: usize) {
        if let Some(card) = self.tableau[col].last_mut() {
            card.face_up = true;
        }
    }

    fn remove_completed_runs(&mut self) {
        for col in 0..self.tableau.len() {
            while let Some(suit) = complete_suited_run_suit(&self.tableau[col]) {
                let new_len = self.tableau[col]
                    .len()
                    .checked_sub(13)
                    .expect("complete run requires at least 13 cards");
                self.tableau[col].truncate(new_len);
                self.completed_runs += 1;
                self.completed_run_suits.push(suit);
                self.flip_top_if_needed(col);
            }
        }
    }
}

impl SpiderSuitMode {
    pub fn suit_count(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
        }
    }

    pub fn from_suit_count(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::One),
            2 => Some(Self::Two),
            3 => Some(Self::Three),
            4 => Some(Self::Four),
            _ => None,
        }
    }

    fn session_tag(self) -> &'static str {
        match self {
            Self::One => "1",
            Self::Two => "2",
            Self::Three => "3",
            Self::Four => "4",
        }
    }

    fn from_session_tag(value: &str) -> Option<Self> {
        match value {
            "1" => Some(Self::One),
            "2" => Some(Self::Two),
            "3" => Some(Self::Three),
            "4" => Some(Self::Four),
            _ => None,
        }
    }
}

#[cfg(test)]
impl SpiderGame {
    pub(crate) fn debug_new(
        suit_mode: SpiderSuitMode,
        stock: Vec<Card>,
        tableau: [Vec<Card>; 10],
        completed_runs: usize,
    ) -> Self {
        Self {
            suit_mode,
            stock,
            tableau,
            completed_runs,
            completed_run_suits: vec![Suit::Spades; completed_runs],
        }
    }
}

fn spider_deck(suit_mode: SpiderSuitMode) -> Vec<Card> {
    let mut deck = Vec::with_capacity(104);
    match suit_mode {
        SpiderSuitMode::One => {
            for _ in 0..8 {
                for rank in 1..=13 {
                    deck.push(Card {
                        suit: Suit::Spades,
                        rank,
                        face_up: false,
                    });
                }
            }
        }
        SpiderSuitMode::Two => {
            for _ in 0..4 {
                for suit in [Suit::Spades, Suit::Hearts] {
                    for rank in 1..=13 {
                        deck.push(Card {
                            suit,
                            rank,
                            face_up: false,
                        });
                    }
                }
            }
        }
        SpiderSuitMode::Four => {
            for _ in 0..2 {
                for suit in Suit::ALL {
                    for rank in 1..=13 {
                        deck.push(Card {
                            suit,
                            rank,
                            face_up: false,
                        });
                    }
                }
            }
        }
        SpiderSuitMode::Three => {
            let suits = [Suit::Spades, Suit::Hearts, Suit::Clubs];
            let full_set = suits.len() * 13;
            let base_copies = 104 / full_set;
            let remainder = 104 % full_set;

            for _ in 0..base_copies {
                for suit in suits {
                    for rank in 1..=13 {
                        deck.push(Card {
                            suit,
                            rank,
                            face_up: false,
                        });
                    }
                }
            }
            for idx in 0..remainder {
                let suit = suits[idx % suits.len()];
                let rank = u8::try_from((idx / suits.len()) + 1).unwrap_or(13);
                deck.push(Card {
                    suit,
                    rank,
                    face_up: false,
                });
            }
        }
    }
    deck
}

fn is_descending_run(cards: &[Card]) -> bool {
    cards.windows(2).all(|pair| {
        let a = pair[0];
        let b = pair[1];
        a.face_up && b.face_up && a.suit == b.suit && a.rank == b.rank + 1
    })
}

fn complete_suited_run_suit(pile: &[Card]) -> Option<Suit> {
    if pile.len() < 13 {
        return None;
    }

    let run = &pile[pile.len() - 13..];
    let Some(first) = run.first().copied() else {
        return None;
    };
    if first.rank != 13 || !first.face_up {
        return None;
    }

    let valid = run.windows(2).all(|pair| {
        let a = pair[0];
        let b = pair[1];
        a.face_up && b.face_up && a.suit == b.suit && a.rank == b.rank + 1
    }) && run.last().is_some_and(|card| card.rank == 1);
    if valid {
        Some(first.suit)
    } else {
        None
    }
}

fn encode_spider_pile(cards: &[Card]) -> String {
    if cards.is_empty() {
        return "-".to_string();
    }
    cards
        .iter()
        .map(|card| {
            let suit = match card.suit {
                Suit::Clubs => 'C',
                Suit::Diamonds => 'D',
                Suit::Hearts => 'H',
                Suit::Spades => 'S',
            };
            let face = if card.face_up { 'U' } else { 'D' };
            format!("{suit}{}{}", card.rank, face)
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn decode_spider_pile(encoded: &str) -> Option<Vec<Card>> {
    if encoded == "-" {
        return Some(Vec::new());
    }
    let mut cards = Vec::new();
    for token in encoded.split('.') {
        let mut chars = token.chars();
        let suit = match chars.next()? {
            'C' => Suit::Clubs,
            'D' => Suit::Diamonds,
            'H' => Suit::Hearts,
            'S' => Suit::Spades,
            _ => return None,
        };
        let face = match token.chars().last()? {
            'U' => true,
            'D' => false,
            _ => return None,
        };
        if token.len() < 3 {
            return None;
        }
        let rank_raw = &token[1..token.len() - 1];
        let rank = rank_raw.parse::<u8>().ok()?;
        if !(1..=13).contains(&rank) {
            return None;
        }
        cards.push(Card {
            suit,
            rank,
            face_up: face,
        });
    }
    Some(cards)
}

fn encode_spider_completed_run_suits(suits: &[Suit]) -> String {
    if suits.is_empty() {
        return "-".to_string();
    }

    suits
        .iter()
        .map(|suit| match suit {
            Suit::Clubs => 'C',
            Suit::Diamonds => 'D',
            Suit::Hearts => 'H',
            Suit::Spades => 'S',
        })
        .collect()
}

fn decode_spider_completed_run_suits(encoded: &str, completed_runs: usize) -> Option<Vec<Suit>> {
    if encoded == "-" {
        return if completed_runs == 0 {
            Some(Vec::new())
        } else {
            None
        };
    }

    let mut suits = Vec::with_capacity(encoded.len());
    for ch in encoded.chars() {
        let suit = match ch {
            'C' => Suit::Clubs,
            'D' => Suit::Diamonds,
            'H' => Suit::Hearts,
            'S' => Suit::Spades,
            _ => return None,
        };
        suits.push(suit);
    }

    if suits.len() != completed_runs {
        return None;
    }

    Some(suits)
}

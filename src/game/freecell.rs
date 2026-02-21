use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;
use std::collections::HashMap;

use super::{Card, Suit};

pub const FREECELL_MIN_CELL_COUNT: u8 = 1;
pub const FREECELL_DEFAULT_CELL_COUNT: u8 = 4;
pub const FREECELL_MAX_CELL_COUNT: u8 = 6;
const FREECELL_MAX_CELL_COUNT_USIZE: usize = FREECELL_MAX_CELL_COUNT as usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FreecellCardCountMode {
    TwentySix,
    ThirtyNine,
    FiftyTwo,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FreecellGame {
    card_count_mode: FreecellCardCountMode,
    foundations: [Vec<Card>; 4],
    freecell_count: u8,
    freecells: [Option<Card>; FREECELL_MAX_CELL_COUNT_USIZE],
    tableau: [Vec<Card>; 8],
}

impl FreecellGame {
    pub fn new_with_seed(seed: u64) -> Self {
        Self::new_with_seed_and_card_count_and_cells(
            seed,
            FreecellCardCountMode::FiftyTwo,
            FREECELL_DEFAULT_CELL_COUNT,
        )
    }

    pub fn new_with_seed_and_card_count(seed: u64, card_count_mode: FreecellCardCountMode) -> Self {
        Self::new_with_seed_and_card_count_and_cells(
            seed,
            card_count_mode,
            FREECELL_DEFAULT_CELL_COUNT,
        )
    }

    pub fn new_with_seed_and_card_count_and_cells(
        seed: u64,
        card_count_mode: FreecellCardCountMode,
        freecell_count: u8,
    ) -> Self {
        let mut deck = freecell_deck(card_count_mode);
        let mut rng = StdRng::seed_from_u64(seed);
        deck.shuffle(&mut rng);

        let mut game = Self {
            card_count_mode,
            foundations: std::array::from_fn(|_| Vec::new()),
            freecell_count: Self::normalize_freecell_cell_count(freecell_count),
            freecells: [None; FREECELL_MAX_CELL_COUNT_USIZE],
            tableau: std::array::from_fn(|_| Vec::new()),
        };

        for (idx, mut card) in deck.into_iter().enumerate() {
            card.face_up = true;
            game.tableau[idx % 8].push(card);
        }

        game
    }

    pub fn foundations(&self) -> &[Vec<Card>; 4] {
        &self.foundations
    }

    pub fn freecell_count(&self) -> usize {
        usize::from(self.freecell_count)
    }

    pub fn occupied_freecell_cells(&self) -> usize {
        self.freecells()
            .iter()
            .filter(|slot| slot.is_some())
            .count()
    }

    pub fn try_set_freecell_count(&mut self, freecell_count: u8) -> Result<(), usize> {
        let normalized = Self::normalize_freecell_cell_count(freecell_count);
        if normalized == self.freecell_count {
            return Ok(());
        }

        let occupied = self.occupied_freecell_cells();
        if usize::from(normalized) < occupied {
            return Err(occupied);
        }

        let old_count = self.freecell_count();
        let new_count = usize::from(normalized);
        if new_count < old_count {
            let mut overflow_cards = Vec::new();
            for idx in new_count..old_count {
                if let Some(card) = self.freecells[idx].take() {
                    overflow_cards.push(card);
                }
            }
            for card in overflow_cards {
                if let Some(empty_idx) = (0..new_count).find(|&idx| self.freecells[idx].is_none()) {
                    self.freecells[empty_idx] = Some(card);
                }
            }
            for idx in new_count..FREECELL_MAX_CELL_COUNT_USIZE {
                self.freecells[idx] = None;
            }
        }

        self.freecell_count = normalized;
        Ok(())
    }

    pub fn freecells(&self) -> &[Option<Card>] {
        &self.freecells[..self.freecell_count()]
    }

    pub fn freecells_storage(&self) -> &[Option<Card>; FREECELL_MAX_CELL_COUNT_USIZE] {
        &self.freecells
    }

    pub fn tableau(&self) -> &[Vec<Card>; 8] {
        &self.tableau
    }

    pub fn tableau_card(&self, col: usize, index: usize) -> Option<Card> {
        self.tableau
            .get(col)
            .and_then(|pile| pile.get(index))
            .copied()
    }

    pub fn tableau_top(&self, col: usize) -> Option<Card> {
        self.tableau.get(col).and_then(|pile| pile.last().copied())
    }

    pub fn freecell_card(&self, cell: usize) -> Option<Card> {
        if cell >= self.freecell_count() {
            return None;
        }
        self.freecells.get(cell).and_then(|slot| *slot)
    }

    pub fn card_count_mode(&self) -> FreecellCardCountMode {
        self.card_count_mode
    }

    pub fn is_won(&self) -> bool {
        let foundation_count: usize = self.foundations.iter().map(Vec::len).sum();
        foundation_count == self.card_count_mode.card_count() as usize
    }

    pub fn has_legal_moves(&self) -> bool {
        if self.is_won() {
            return false;
        }

        for cell in 0..self.freecell_count() {
            if self.can_move_freecell_to_foundation(cell) {
                return true;
            }
            for dst in 0..8 {
                if self.can_move_freecell_to_tableau(cell, dst) {
                    return true;
                }
            }
        }

        for src in 0..8 {
            if self.can_move_tableau_top_to_foundation(src) {
                return true;
            }
            for cell in 0..self.freecell_count() {
                if self.can_move_tableau_top_to_freecell(src, cell) {
                    return true;
                }
            }
            let len = self.tableau.get(src).map(Vec::len).unwrap_or(0);
            for start in 0..len {
                for dst in 0..8 {
                    if self.can_move_tableau_run_to_tableau(src, start, dst) {
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn is_lost(&self) -> bool {
        !self.is_won() && !self.has_legal_moves()
    }

    pub fn can_move_tableau_top_to_foundation(&self, src: usize) -> bool {
        let Some(card) = self.tableau_top(src) else {
            return false;
        };
        self.can_place_on_foundation(card)
    }

    pub fn move_tableau_top_to_foundation(&mut self, src: usize) -> bool {
        if !self.can_move_tableau_top_to_foundation(src) {
            return false;
        }
        let Some(card) = self.tableau[src].pop() else {
            return false;
        };
        let foundation_idx = card.suit.foundation_index();
        self.foundations[foundation_idx].push(card);
        true
    }

    pub fn can_move_freecell_to_foundation(&self, cell: usize) -> bool {
        let Some(card) = self.freecell_card(cell) else {
            return false;
        };
        self.can_place_on_foundation(card)
    }

    pub fn move_freecell_to_foundation(&mut self, cell: usize) -> bool {
        if !self.can_move_freecell_to_foundation(cell) {
            return false;
        }
        let Some(card) = self.freecells[cell].take() else {
            return false;
        };
        let foundation_idx = card.suit.foundation_index();
        self.foundations[foundation_idx].push(card);
        true
    }

    pub fn can_move_tableau_top_to_freecell(&self, src: usize, cell: usize) -> bool {
        if cell >= self.freecell_count() || src >= self.tableau.len() {
            return false;
        }
        self.freecells[cell].is_none() && !self.tableau[src].is_empty()
    }

    pub fn move_tableau_top_to_freecell(&mut self, src: usize, cell: usize) -> bool {
        if !self.can_move_tableau_top_to_freecell(src, cell) {
            return false;
        }
        let Some(card) = self.tableau[src].pop() else {
            return false;
        };
        self.freecells[cell] = Some(card);
        true
    }

    pub fn can_move_freecell_to_tableau(&self, cell: usize, dst: usize) -> bool {
        if dst >= self.tableau.len() {
            return false;
        }
        let Some(card) = self.freecell_card(cell) else {
            return false;
        };
        self.can_place_on_tableau(card, dst)
    }

    pub fn move_freecell_to_tableau(&mut self, cell: usize, dst: usize) -> bool {
        if !self.can_move_freecell_to_tableau(cell, dst) {
            return false;
        }
        let Some(card) = self.freecells[cell].take() else {
            return false;
        };
        self.tableau[dst].push(card);
        true
    }

    pub fn can_move_foundation_top_to_tableau(&self, foundation_idx: usize, dst: usize) -> bool {
        let _ = foundation_idx;
        let _ = dst;
        false
    }

    pub fn move_foundation_top_to_tableau(&mut self, foundation_idx: usize, dst: usize) -> bool {
        let _ = foundation_idx;
        let _ = dst;
        false
    }

    pub fn can_move_tableau_run_to_tableau(&self, src: usize, start: usize, dst: usize) -> bool {
        if src == dst || src >= self.tableau.len() || dst >= self.tableau.len() {
            return false;
        }
        let source = &self.tableau[src];
        if start >= source.len() {
            return false;
        }

        let run = &source[start..];
        if run.is_empty() || !is_descending_alternating_run(run) {
            return false;
        }

        let first = run[0];
        if !self.can_place_on_tableau(first, dst) {
            return false;
        }

        run.len() <= self.max_movable_cards(dst)
    }

    pub fn move_tableau_run_to_tableau(&mut self, src: usize, start: usize, dst: usize) -> bool {
        if !self.can_move_tableau_run_to_tableau(src, start, dst) {
            return false;
        }
        let moved = self.tableau[src].split_off(start);
        self.tableau[dst].extend(moved);
        true
    }

    pub fn cyclone_shuffle_tableau(&mut self) -> bool {
        let geometry: Vec<usize> = self.tableau.iter().map(Vec::len).collect();
        let mut cards: Vec<Card> = self
            .tableau
            .iter()
            .flat_map(|pile| pile.iter().copied())
            .collect();
        if cards.len() < 2 {
            return false;
        }
        let original = self.tableau.clone();
        let mut rng = rand::thread_rng();
        for _ in 0..8 {
            cards.shuffle(&mut rng);
            let mut cursor = 0_usize;
            for (col, pile) in self.tableau.iter_mut().enumerate() {
                let len = geometry[col];
                pile.clear();
                for _ in 0..len {
                    let mut card = cards[cursor];
                    card.face_up = true;
                    pile.push(card);
                    cursor += 1;
                }
            }
            if self.tableau != original {
                return true;
            }
        }
        false
    }

    pub fn encode_for_session(&self) -> String {
        let mut parts = Vec::with_capacity(17 + FREECELL_MAX_CELL_COUNT_USIZE);
        parts.push(format!("fc={}", self.freecell_count));
        for idx in 0..FREECELL_MAX_CELL_COUNT_USIZE {
            parts.push(format!("c{idx}={}", encode_slot(self.freecells[idx])));
        }
        parts.push(format!("f0={}", encode_pile(&self.foundations[0])));
        parts.push(format!("f1={}", encode_pile(&self.foundations[1])));
        parts.push(format!("f2={}", encode_pile(&self.foundations[2])));
        parts.push(format!("f3={}", encode_pile(&self.foundations[3])));
        parts.push(format!("t0={}", encode_pile(&self.tableau[0])));
        parts.push(format!("t1={}", encode_pile(&self.tableau[1])));
        parts.push(format!("t2={}", encode_pile(&self.tableau[2])));
        parts.push(format!("t3={}", encode_pile(&self.tableau[3])));
        parts.push(format!("t4={}", encode_pile(&self.tableau[4])));
        parts.push(format!("t5={}", encode_pile(&self.tableau[5])));
        parts.push(format!("t6={}", encode_pile(&self.tableau[6])));
        parts.push(format!("t7={}", encode_pile(&self.tableau[7])));
        parts.join(";")
    }

    pub fn decode_from_session(data: &str) -> Option<Self> {
        let mut fields = HashMap::<&str, &str>::new();
        for part in data.split(';') {
            let (key, value) = part.split_once('=')?;
            fields.insert(key, value);
        }

        let mut freecells = [None; FREECELL_MAX_CELL_COUNT_USIZE];
        for (idx, slot) in freecells.iter_mut().enumerate() {
            let key = format!("c{idx}");
            if let Some(raw) = fields.get(key.as_str()) {
                *slot = decode_slot(raw)?;
            } else if idx < usize::from(FREECELL_DEFAULT_CELL_COUNT) {
                return None;
            }
        }

        let mut freecell_count = fields
            .get("fc")
            .and_then(|raw| raw.parse::<u8>().ok())
            .map(Self::normalize_freecell_cell_count)
            .unwrap_or(FREECELL_DEFAULT_CELL_COUNT);
        let highest_occupied = freecells
            .iter()
            .rposition(|slot| slot.is_some())
            .map(|idx| idx + 1)
            .unwrap_or(0);
        if highest_occupied > usize::from(freecell_count) {
            freecell_count = highest_occupied.min(FREECELL_MAX_CELL_COUNT_USIZE) as u8;
        }

        let foundations = [
            decode_pile(fields.get("f0")?)?,
            decode_pile(fields.get("f1")?)?,
            decode_pile(fields.get("f2")?)?,
            decode_pile(fields.get("f3")?)?,
        ];
        let tableau = [
            decode_pile(fields.get("t0")?)?,
            decode_pile(fields.get("t1")?)?,
            decode_pile(fields.get("t2")?)?,
            decode_pile(fields.get("t3")?)?,
            decode_pile(fields.get("t4")?)?,
            decode_pile(fields.get("t5")?)?,
            decode_pile(fields.get("t6")?)?,
            decode_pile(fields.get("t7")?)?,
        ];

        let freecell_cards = freecells.iter().filter(|card| card.is_some()).count();
        let foundations_count: usize = foundations.iter().map(Vec::len).sum();
        let tableau_count: usize = tableau.iter().map(Vec::len).sum();
        let total_cards = freecell_cards + foundations_count + tableau_count;
        let Some(card_count_mode) = FreecellCardCountMode::from_card_count(total_cards as u8)
        else {
            return None;
        };

        Some(Self {
            card_count_mode,
            freecell_count,
            foundations,
            freecells,
            tableau,
        })
    }

    fn max_movable_cards(&self, dst: usize) -> usize {
        let free_empty = self
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let mut empty_tableau = self.tableau.iter().filter(|pile| pile.is_empty()).count();
        if self.tableau.get(dst).is_some_and(|pile| pile.is_empty()) {
            empty_tableau = empty_tableau.saturating_sub(1);
        }
        (free_empty + 1) * (1usize << empty_tableau)
    }

    fn can_place_on_foundation(&self, card: Card) -> bool {
        let foundation = &self.foundations[card.suit.foundation_index()];
        match foundation.last() {
            None => card.rank == 1,
            Some(top) => top.suit == card.suit && top.rank + 1 == card.rank,
        }
    }

    fn can_place_on_tableau(&self, card: Card, dst: usize) -> bool {
        match self.tableau.get(dst).and_then(|pile| pile.last()).copied() {
            None => true,
            Some(top) => top.rank == card.rank + 1 && top.color_red() != card.color_red(),
        }
    }

    pub(crate) fn from_parts_unchecked_with_cell_count(
        card_count_mode: FreecellCardCountMode,
        foundations: [Vec<Card>; 4],
        freecell_count: u8,
        freecells: [Option<Card>; FREECELL_MAX_CELL_COUNT_USIZE],
        tableau: [Vec<Card>; 8],
    ) -> Self {
        Self {
            card_count_mode,
            freecell_count: Self::normalize_freecell_cell_count(freecell_count),
            foundations,
            freecells,
            tableau,
        }
    }

    fn normalize_freecell_cell_count(value: u8) -> u8 {
        if (FREECELL_MIN_CELL_COUNT..=FREECELL_MAX_CELL_COUNT).contains(&value) {
            value
        } else {
            FREECELL_DEFAULT_CELL_COUNT
        }
    }
}

#[cfg(test)]
impl FreecellGame {
    pub(crate) fn debug_new(
        foundations: [Vec<Card>; 4],
        freecells: [Option<Card>; 4],
        tableau: [Vec<Card>; 8],
    ) -> Self {
        Self::debug_new_with_mode_and_cells(
            FreecellCardCountMode::FiftyTwo,
            FREECELL_DEFAULT_CELL_COUNT,
            foundations,
            freecells,
            tableau,
        )
    }

    pub(crate) fn debug_new_with_mode(
        card_count_mode: FreecellCardCountMode,
        foundations: [Vec<Card>; 4],
        freecells: [Option<Card>; 4],
        tableau: [Vec<Card>; 8],
    ) -> Self {
        Self::debug_new_with_mode_and_cells(
            card_count_mode,
            FREECELL_DEFAULT_CELL_COUNT,
            foundations,
            freecells,
            tableau,
        )
    }

    pub(crate) fn debug_new_with_mode_and_cells(
        card_count_mode: FreecellCardCountMode,
        freecell_count: u8,
        foundations: [Vec<Card>; 4],
        freecells: [Option<Card>; 4],
        tableau: [Vec<Card>; 8],
    ) -> Self {
        let mut freecells_storage = [None; FREECELL_MAX_CELL_COUNT_USIZE];
        freecells_storage[..4].copy_from_slice(&freecells);
        Self {
            card_count_mode,
            freecell_count: Self::normalize_freecell_cell_count(freecell_count),
            foundations,
            freecells: freecells_storage,
            tableau,
        }
    }
}

fn freecell_deck(card_count_mode: FreecellCardCountMode) -> Vec<Card> {
    let mut deck = Vec::with_capacity(card_count_mode.card_count() as usize);
    let suit_count = card_count_mode.suit_count() as usize;
    for suit in Suit::ALL.into_iter().take(suit_count) {
        for rank in 1..=13 {
            deck.push(Card {
                suit,
                rank,
                face_up: true,
            });
        }
    }
    deck
}

impl FreecellCardCountMode {
    pub fn card_count(self) -> u8 {
        match self {
            Self::TwentySix => 26,
            Self::ThirtyNine => 39,
            Self::FiftyTwo => 52,
        }
    }

    pub fn suit_count(self) -> u8 {
        self.card_count() / 13
    }

    pub fn from_card_count(value: u8) -> Option<Self> {
        match value {
            26 => Some(Self::TwentySix),
            39 => Some(Self::ThirtyNine),
            52 => Some(Self::FiftyTwo),
            _ => None,
        }
    }
}

fn is_descending_alternating_run(cards: &[Card]) -> bool {
    cards.windows(2).all(|pair| {
        let a = pair[0];
        let b = pair[1];
        a.rank == b.rank + 1 && a.color_red() != b.color_red()
    })
}

fn encode_slot(card: Option<Card>) -> String {
    match card {
        Some(card) => encode_card(card),
        None => "-".to_string(),
    }
}

fn decode_slot(encoded: &str) -> Option<Option<Card>> {
    if encoded == "-" {
        return Some(None);
    }
    decode_card(encoded).map(Some)
}

fn encode_pile(cards: &[Card]) -> String {
    if cards.is_empty() {
        return "-".to_string();
    }
    cards
        .iter()
        .map(|card| encode_card(*card))
        .collect::<Vec<_>>()
        .join(".")
}

fn decode_pile(encoded: &str) -> Option<Vec<Card>> {
    if encoded == "-" {
        return Some(Vec::new());
    }
    encoded.split('.').map(decode_card).collect()
}

fn encode_card(card: Card) -> String {
    let suit = match card.suit {
        Suit::Clubs => 'C',
        Suit::Diamonds => 'D',
        Suit::Hearts => 'H',
        Suit::Spades => 'S',
    };
    format!("{suit}{}U", card.rank)
}

fn decode_card(token: &str) -> Option<Card> {
    if token.len() < 3 {
        return None;
    }
    let mut chars = token.chars();
    let suit = match chars.next()? {
        'C' => Suit::Clubs,
        'D' => Suit::Diamonds,
        'H' => Suit::Hearts,
        'S' => Suit::Spades,
        _ => return None,
    };
    if !token.ends_with('U') && !token.ends_with('D') {
        return None;
    }
    let rank = token[1..token.len() - 1].parse::<u8>().ok()?;
    if !(1..=13).contains(&rank) {
        return None;
    }
    Some(Card {
        suit,
        rank,
        face_up: true,
    })
}

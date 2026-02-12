use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

use super::*;

impl KlondikeGame {
    pub fn new_shuffled() -> Self {
        let mut rng = rand::thread_rng();
        Self::new_with_seed(rng.gen())
    }

    pub fn new_with_seed(seed: u64) -> Self {
        let mut deck = full_deck();
        let mut rng = StdRng::seed_from_u64(seed);
        deck.shuffle(&mut rng);

        let mut game = Self {
            draw_mode: DrawMode::One,
            stock: Vec::new(),
            waste: Vec::new(),
            foundations: std::array::from_fn(|_| Vec::new()),
            tableau: std::array::from_fn(|_| Vec::new()),
        };

        let mut draw = deck.into_iter();
        for col in 0..7 {
            for row in 0..=col {
                let mut card = draw.next().expect("full deck has enough cards");
                card.face_up = row == col;
                game.tableau[col].push(card);
            }
        }

        for mut card in draw {
            card.face_up = false;
            game.stock.push(card);
        }

        game
    }

    pub fn draw_or_recycle_with_count(&mut self, draw_count: u8) -> DrawResult {
        if !self.stock.is_empty() {
            let draw_count = usize::from(draw_count.clamp(1, 5));
            for _ in 0..draw_count {
                let Some(mut card) = self.stock.pop() else {
                    break;
                };
                card.face_up = true;
                self.waste.push(card);
            }
            return DrawResult::DrewFromStock;
        }

        if self.waste.is_empty() {
            return DrawResult::NoOp;
        }

        while let Some(mut card) = self.waste.pop() {
            card.face_up = false;
            self.stock.push(card);
        }
        DrawResult::RecycledWaste
    }

    pub fn draw_or_recycle(&mut self) -> DrawResult {
        self.draw_or_recycle_with_count(self.draw_mode.count())
    }

    pub fn set_draw_mode(&mut self, mode: DrawMode) {
        self.draw_mode = mode;
    }

    pub fn draw_mode(&self) -> DrawMode {
        self.draw_mode
    }

    pub fn stock_len(&self) -> usize {
        self.stock.len()
    }

    pub fn waste_len(&self) -> usize {
        self.waste.len()
    }

    pub fn foundations(&self) -> &[Vec<Card>; 4] {
        &self.foundations
    }

    pub fn tableau(&self) -> &[Vec<Card>; 7] {
        &self.tableau
    }

    pub fn is_won(&self) -> bool {
        self.foundations.iter().all(|pile| pile.len() == 13)
    }
}

fn full_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);
    for suit in Suit::ALL {
        for rank in 1..=13 {
            deck.push(Card {
                suit,
                rank,
                face_up: false,
            });
        }
    }
    deck
}

use rand::seq::SliceRandom;

use super::*;

impl KlondikeGame {
    pub fn move_waste_to_foundation(&mut self) -> bool {
        let Some(card) = self.waste.last().copied() else {
            return false;
        };

        let idx = card.suit.foundation_index();
        if !can_stack_foundation(self.foundations[idx].last(), card) {
            return false;
        }

        self.waste.pop();
        self.foundations[idx].push(card);
        true
    }

    pub fn can_move_waste_to_foundation(&self) -> bool {
        let Some(card) = self.waste.last().copied() else {
            return false;
        };
        can_stack_foundation(self.foundations[card.suit.foundation_index()].last(), card)
    }

    pub fn move_waste_to_tableau(&mut self, dst: usize) -> bool {
        let Some(card) = self.waste.last().copied() else {
            return false;
        };

        if dst >= self.tableau.len() || !can_stack_tableau(self.tableau[dst].last(), card) {
            return false;
        }

        self.waste.pop();
        self.tableau[dst].push(card);
        true
    }

    pub fn can_move_waste_to_tableau(&self, dst: usize) -> bool {
        let Some(card) = self.waste.last().copied() else {
            return false;
        };
        if dst >= self.tableau.len() {
            return false;
        }
        can_stack_tableau(self.tableau[dst].last(), card)
    }

    pub fn move_tableau_top_to_foundation(&mut self, src: usize) -> bool {
        if src >= self.tableau.len() {
            return false;
        }

        let Some(card) = self.tableau[src].last().copied() else {
            return false;
        };

        if !card.face_up {
            return false;
        }

        let idx = card.suit.foundation_index();
        if !can_stack_foundation(self.foundations[idx].last(), card) {
            return false;
        }

        self.tableau[src].pop();
        self.foundations[idx].push(card);
        self.flip_top_tableau_if_needed(src);
        true
    }

    pub fn can_move_tableau_top_to_foundation(&self, src: usize) -> bool {
        if src >= self.tableau.len() {
            return false;
        }
        let Some(card) = self.tableau[src].last().copied() else {
            return false;
        };
        if !card.face_up {
            return false;
        }
        can_stack_foundation(self.foundations[card.suit.foundation_index()].last(), card)
    }

    pub fn can_move_foundation_top_to_tableau(&self, foundation_idx: usize, dst: usize) -> bool {
        if foundation_idx >= self.foundations.len() || dst >= self.tableau.len() {
            return false;
        }
        let Some(card) = self.foundations[foundation_idx].last().copied() else {
            return false;
        };
        can_stack_tableau(self.tableau[dst].last(), card)
    }

    pub fn move_foundation_top_to_tableau(&mut self, foundation_idx: usize, dst: usize) -> bool {
        if !self.can_move_foundation_top_to_tableau(foundation_idx, dst) {
            return false;
        }
        let Some(card) = self.foundations[foundation_idx].pop() else {
            return false;
        };
        self.tableau[dst].push(card);
        true
    }

    pub fn move_tableau_top_to_tableau(&mut self, src: usize, dst: usize) -> bool {
        if src == dst || src >= self.tableau.len() || dst >= self.tableau.len() {
            return false;
        }

        let Some(card) = self.tableau[src].last().copied() else {
            return false;
        };

        if !card.face_up || !can_stack_tableau(self.tableau[dst].last(), card) {
            return false;
        }

        self.tableau[src].pop();
        self.tableau[dst].push(card);
        self.flip_top_tableau_if_needed(src);
        true
    }

    pub fn can_move_tableau_run_to_tableau(&self, src: usize, start: usize, dst: usize) -> bool {
        if src == dst || src >= self.tableau.len() || dst >= self.tableau.len() {
            return false;
        }

        let source = &self.tableau[src];
        if start >= source.len() {
            return false;
        }

        let first = source[start];
        if !first.face_up {
            return false;
        }

        if !self.is_valid_face_up_run(source, start) {
            return false;
        }

        can_stack_tableau(self.tableau[dst].last(), first)
    }

    pub fn move_tableau_run_to_tableau(&mut self, src: usize, start: usize, dst: usize) -> bool {
        if !self.can_move_tableau_run_to_tableau(src, start, dst) {
            return false;
        }

        let moved = self.tableau[src].split_off(start);
        self.tableau[dst].extend(moved);
        self.flip_top_tableau_if_needed(src);
        true
    }

    pub fn can_move_tableau_top_to_tableau(&self, src: usize, dst: usize) -> bool {
        if src == dst || src >= self.tableau.len() || dst >= self.tableau.len() {
            return false;
        }
        let Some(card) = self.tableau[src].last().copied() else {
            return false;
        };
        card.face_up && can_stack_tableau(self.tableau[dst].last(), card)
    }

    pub fn tableau_top(&self, col: usize) -> Option<Card> {
        self.tableau.get(col).and_then(|pile| pile.last().copied())
    }

    pub fn tableau_len(&self, col: usize) -> Option<usize> {
        self.tableau.get(col).map(Vec::len)
    }

    pub fn tableau_card(&self, col: usize, index: usize) -> Option<Card> {
        self.tableau
            .get(col)
            .and_then(|pile| pile.get(index))
            .copied()
    }

    pub fn foundation_top_rank(&self, suit: Suit) -> u8 {
        self.foundations[suit.foundation_index()]
            .last()
            .map(|c| c.rank)
            .unwrap_or(0)
    }

    pub fn waste_top(&self) -> Option<Card> {
        self.waste.last().copied()
    }

    pub fn waste_top_n(&self, n: usize) -> Vec<Card> {
        if n == 0 {
            return Vec::new();
        }
        let mut cards: Vec<Card> = self.waste.iter().rev().take(n).copied().collect();
        cards.reverse();
        cards
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

    fn flip_top_tableau_if_needed(&mut self, col: usize) {
        if let Some(card) = self.tableau[col].last_mut() {
            card.face_up = true;
        }
    }

    fn is_valid_face_up_run(&self, source: &[Card], start: usize) -> bool {
        source[start..].windows(2).all(|pair| {
            let a = pair[0];
            let b = pair[1];
            a.face_up && b.face_up && a.color_red() != b.color_red() && a.rank == b.rank + 1
        })
    }
}

fn can_stack_foundation(top: Option<&Card>, card: Card) -> bool {
    match top {
        None => card.rank == 1,
        Some(top_card) => top_card.suit == card.suit && card.rank == top_card.rank + 1,
    }
}

fn can_stack_tableau(top: Option<&Card>, card: Card) -> bool {
    match top {
        None => card.rank == 13,
        Some(top_card) => {
            top_card.face_up
                && top_card.color_red() != card.color_red()
                && top_card.rank == card.rank + 1
        }
    }
}

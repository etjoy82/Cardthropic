use super::*;
use crate::engine::boundary;

impl CardthropicWindow {
    pub(super) fn foundation_slot_suit(&self, slot: usize) -> Option<Suit> {
        self.imp()
            .foundation_slot_suits
            .borrow()
            .get(slot)
            .copied()
            .flatten()
    }

    pub(super) fn foundation_slot_for_suit(&self, suit: Suit) -> Option<usize> {
        self.imp()
            .foundation_slot_suits
            .borrow()
            .iter()
            .position(|assigned| *assigned == Some(suit))
    }

    pub(super) fn foundation_suit_index_for_slot(&self, slot: usize) -> Option<usize> {
        self.foundation_slot_suit(slot).map(Suit::foundation_index)
    }

    pub(super) fn foundation_slot_suits_snapshot(&self) -> [Option<Suit>; 4] {
        *self.imp().foundation_slot_suits.borrow()
    }

    pub(super) fn set_foundation_slot_suits(&self, slots: [Option<Suit>; 4]) {
        *self.imp().foundation_slot_suits.borrow_mut() = slots;
    }

    fn foundation_lengths_for_mode(&self) -> Option<[usize; 4]> {
        let mode = self.active_game_mode();
        match mode {
            GameMode::Klondike => {
                let draw = self.current_klondike_draw_mode();
                boundary::clone_klondike_for_automation(&self.imp().game.borrow(), mode, draw).map(
                    |game| {
                        [
                            game.foundations()[0].len(),
                            game.foundations()[1].len(),
                            game.foundations()[2].len(),
                            game.foundations()[3].len(),
                        ]
                    },
                )
            }
            GameMode::Freecell => {
                let game = self.imp().game.borrow();
                let f = game.freecell().foundations();
                Some([f[0].len(), f[1].len(), f[2].len(), f[3].len()])
            }
            GameMode::Spider => None,
        }
    }

    pub(super) fn sync_foundation_slots_with_state(&self) {
        let Some(lengths) = self.foundation_lengths_for_mode() else {
            self.set_foundation_slot_suits([None, None, None, None]);
            return;
        };

        let mut slots = self.foundation_slot_suits_snapshot();

        if lengths.iter().all(|len| *len == 0) {
            slots = [None, None, None, None];
            self.set_foundation_slot_suits(slots);
            return;
        }

        // Keep at most one slot per suit if stale duplicates exist.
        let mut seen = [false; 4];
        for slot in &mut slots {
            if let Some(suit) = *slot {
                let idx = suit.foundation_index();
                if seen[idx] {
                    *slot = None;
                } else {
                    seen[idx] = true;
                }
            }
        }

        // Any non-empty suit stack must have a slot assignment.
        for suit in Suit::ALL {
            let idx = suit.foundation_index();
            if lengths[idx] == 0 || slots.contains(&Some(suit)) {
                continue;
            }
            if let Some(empty_slot) = slots.iter().position(|slot| slot.is_none()) {
                slots[empty_slot] = Some(suit);
            }
        }

        self.set_foundation_slot_suits(slots);
    }

    pub(super) fn foundation_slot_accepts_card(&self, card: Card, slot: usize) -> bool {
        if slot >= 4 {
            return false;
        }
        self.sync_foundation_slots_with_state();
        let slots = self.foundation_slot_suits_snapshot();

        if let Some(existing) = slots[slot] {
            return existing == card.suit;
        }

        // Empty slot can only start a suit with an Ace that is not already assigned.
        card.rank == 1 && !slots.contains(&Some(card.suit))
    }

    pub(super) fn resolve_foundation_slot_for_card(
        &self,
        card: Card,
        preferred_slot: Option<usize>,
    ) -> Option<usize> {
        self.sync_foundation_slots_with_state();
        let slots = self.foundation_slot_suits_snapshot();

        if let Some(existing_slot) = self.foundation_slot_for_suit(card.suit) {
            if preferred_slot.is_some_and(|slot| slot != existing_slot) {
                return None;
            }
            return Some(existing_slot);
        }

        if card.rank != 1 {
            return None;
        }

        if let Some(slot) = preferred_slot {
            if slot < 4 && slots[slot].is_none() {
                return Some(slot);
            }
            return None;
        }

        slots.iter().position(|slot| slot.is_none())
    }

    pub(super) fn establish_foundation_slot_for_card(&self, card: Card, slot: usize) {
        if slot >= 4 {
            return;
        }
        self.sync_foundation_slots_with_state();
        let mut slots = self.foundation_slot_suits_snapshot();

        if slots[slot].is_some() || slots.contains(&Some(card.suit)) {
            return;
        }
        slots[slot] = Some(card.suit);
        self.set_foundation_slot_suits(slots);
    }
}

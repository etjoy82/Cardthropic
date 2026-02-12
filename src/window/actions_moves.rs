use super::*;

impl CardthropicWindow {
    pub(super) fn draw_card(&self) -> bool {
        if !self.guard_mode_engine("Draw") {
            return false;
        }
        let draw_mode = self.current_klondike_draw_mode();
        {
            let mut game = self.imp().game.borrow_mut();
            if game.draw_mode() != draw_mode {
                game.set_draw_mode(draw_mode);
            }
        }
        let snapshot = self.snapshot();
        let result = self
            .imp()
            .game
            .borrow_mut()
            .draw_or_recycle_with_count(draw_mode.count());
        let changed = match result {
            DrawResult::DrewFromStock => true,
            DrawResult::RecycledWaste => true,
            DrawResult::NoOp => false,
        };

        if !self.apply_changed_move(snapshot, changed) {
            *self.imp().status_override.borrow_mut() = Some("Nothing to draw.".to_string());
        }
        self.render();
        changed
    }

    pub(super) fn cyclone_shuffle_tableau(&self) -> bool {
        if !self.guard_mode_engine("Cyclone shuffle") {
            return false;
        }

        let snapshot = self.snapshot();
        let changed = self.imp().game.borrow_mut().cyclone_shuffle_tableau();
        let changed = self.apply_changed_move(snapshot, changed);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            *self.imp().status_override.borrow_mut() = Some(
                "Cyclone shuffle complete: rerolled tableau while preserving each column's geometry."
                    .to_string(),
            );
        } else {
            *self.imp().status_override.borrow_mut() =
                Some("Cyclone shuffle had no effect.".to_string());
        }
        self.render();
        changed
    }

    pub(super) fn trigger_peek(&self) {
        if !self.guard_mode_engine("Peek") {
            return;
        }
        let imp = self.imp();
        let generation = imp.peek_generation.get().wrapping_add(1);
        imp.peek_generation.set(generation);
        imp.peek_active.set(true);
        self.render();

        glib::timeout_add_local_once(
            Duration::from_secs(3),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move || {
                    let imp = window.imp();
                    if imp.peek_generation.get() != generation {
                        return;
                    }
                    imp.peek_active.set(false);
                    window.render();
                }
            ),
        );
    }

    pub(super) fn move_waste_to_foundation(&self) -> bool {
        if !self.guard_mode_engine("Waste-to-foundation move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self.imp().game.borrow_mut().move_waste_to_foundation();
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn move_waste_to_tableau(&self, dst: usize) -> bool {
        if !self.guard_mode_engine("Waste-to-tableau move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self.imp().game.borrow_mut().move_waste_to_tableau(dst);
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn move_tableau_run_to_tableau(&self, src: usize, start: usize, dst: usize) -> bool {
        if !self.guard_mode_engine("Tableau move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self
            .imp()
            .game
            .borrow_mut()
            .move_tableau_run_to_tableau(src, start, dst);
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn move_tableau_to_foundation(&self, src: usize) -> bool {
        if !self.guard_mode_engine("Tableau-to-foundation move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self
            .imp()
            .game
            .borrow_mut()
            .move_tableau_top_to_foundation(src);
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn move_foundation_to_tableau(&self, foundation_idx: usize, dst: usize) -> bool {
        if !self.guard_mode_engine("Foundation-to-tableau move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self
            .imp()
            .game
            .borrow_mut()
            .move_foundation_top_to_tableau(foundation_idx, dst);
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn can_auto_move_waste_to_foundation(&self, game: &KlondikeGame) -> bool {
        let Some(card) = game.waste_top() else {
            return false;
        };
        game.can_move_waste_to_foundation() && self.is_safe_auto_foundation(game, card)
    }

    pub(super) fn can_auto_move_tableau_to_foundation(
        &self,
        game: &KlondikeGame,
        src: usize,
    ) -> bool {
        let Some(card) = game.tableau_top(src) else {
            return false;
        };
        game.can_move_tableau_top_to_foundation(src) && self.is_safe_auto_foundation(game, card)
    }

    fn is_safe_auto_foundation(&self, game: &KlondikeGame, card: Card) -> bool {
        if card.rank <= 2 {
            return true;
        }

        match card.suit {
            Suit::Hearts | Suit::Diamonds => {
                game.foundation_top_rank(Suit::Clubs) >= card.rank - 1
                    && game.foundation_top_rank(Suit::Spades) >= card.rank - 1
            }
            Suit::Clubs | Suit::Spades => {
                game.foundation_top_rank(Suit::Hearts) >= card.rank - 1
                    && game.foundation_top_rank(Suit::Diamonds) >= card.rank - 1
            }
        }
    }
}

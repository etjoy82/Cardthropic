use super::*;
use crate::engine::boundary;
use crate::engine::commands::EngineCommand;

impl CardthropicWindow {
    pub(super) fn draw_card(&self) -> bool {
        if !self.guard_mode_engine("Draw") {
            return false;
        }
        let mode = self.active_game_mode();
        let draw_mode = self.current_klondike_draw_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::DrawOrRecycle { draw_mode },
        )
        .changed;

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

        let mode = self.active_game_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::CycloneShuffleTableau,
        )
        .changed;
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
        let mode = self.active_game_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveWasteToFoundation,
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn move_waste_to_tableau(&self, dst: usize) -> bool {
        if !self.guard_mode_engine("Waste-to-tableau move") {
            return false;
        }
        let mode = self.active_game_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveWasteToTableau { dst },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn move_tableau_run_to_tableau(&self, src: usize, start: usize, dst: usize) -> bool {
        if !self.guard_mode_engine("Tableau move") {
            return false;
        }
        let mode = self.active_game_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveTableauRunToTableau { src, start, dst },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn move_tableau_to_foundation(&self, src: usize) -> bool {
        if !self.guard_mode_engine("Tableau-to-foundation move") {
            return false;
        }
        let mode = self.active_game_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveTableauTopToFoundation { src },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn move_foundation_to_tableau(&self, foundation_idx: usize, dst: usize) -> bool {
        if !self.guard_mode_engine("Foundation-to-tableau move") {
            return false;
        }
        let mode = self.active_game_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveFoundationTopToTableau {
                foundation_idx,
                dst,
            },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }
}

use super::*;
use crate::engine::boundary;

impl CardthropicWindow {
    pub(super) fn snapshot(&self) -> Snapshot {
        let imp = self.imp();
        let mode = self.active_game_mode();
        let game = imp.game.borrow();
        Snapshot {
            mode,
            runtime: game.runtime_for_mode(mode),
            draw_mode: imp.klondike_draw_mode.get(),
            selected_run: *imp.selected_run.borrow(),
            selected_waste: imp.waste_selected.get(),
            move_count: imp.move_count.get(),
            elapsed_seconds: imp.elapsed_seconds.get(),
            timer_started: imp.timer_started.get(),
            apm_samples: imp.apm_samples.borrow().clone(),
        }
    }

    pub(super) fn apply_changed_move(&self, snapshot: Snapshot, changed: bool) -> bool {
        if changed {
            let imp = self.imp();
            self.clear_hint_effects();
            imp.waste_selected.set(false);
            imp.history.borrow_mut().push(snapshot);
            imp.future.borrow_mut().clear();
            imp.move_count.set(imp.move_count.get() + 1);
            imp.timer_started.set(true);
            *imp.status_override.borrow_mut() = None;
            self.note_current_state_for_hint_cycle();
            if imp.auto_playing_move.get() {
                self.note_current_state_for_auto_play();
            } else {
                self.reset_auto_play_memory();
            }
            let state_hash = self.current_game_hash();
            self.start_hint_loss_analysis_if_needed(state_hash);
            let mode = self.active_game_mode();
            if boundary::is_won(&imp.game.borrow(), mode) {
                imp.timer_started.set(false);
            }
        }
        changed
    }

    pub(super) fn undo(&self) {
        if !self.guard_mode_engine("Undo") {
            return;
        }
        let imp = self.imp();
        let Some(snapshot) = imp.history.borrow_mut().pop() else {
            *imp.status_override.borrow_mut() = Some("Nothing to undo yet.".to_string());
            self.render();
            return;
        };

        self.clear_hint_effects();
        imp.future.borrow_mut().push(self.snapshot());
        self.restore_snapshot(snapshot);
        *imp.status_override.borrow_mut() = Some("Undid last move.".to_string());
        self.render();
    }

    pub(super) fn redo(&self) {
        if !self.guard_mode_engine("Redo") {
            return;
        }
        let imp = self.imp();
        let Some(snapshot) = imp.future.borrow_mut().pop() else {
            *imp.status_override.borrow_mut() = Some("Nothing to redo yet.".to_string());
            self.render();
            return;
        };

        self.clear_hint_effects();
        imp.history.borrow_mut().push(self.snapshot());
        self.restore_snapshot(snapshot);
        *imp.status_override.borrow_mut() = Some("Redid move.".to_string());
        self.render();
    }

    fn restore_snapshot(&self, snapshot: Snapshot) {
        let imp = self.imp();
        imp.current_game_mode.set(snapshot.mode);
        imp.klondike_draw_mode.set(snapshot.draw_mode);
        {
            let mut game = imp.game.borrow_mut();
            game.set_runtime(snapshot.runtime);
            let _ = boundary::set_draw_mode(&mut game, snapshot.mode, snapshot.draw_mode);
        }
        *imp.selected_run.borrow_mut() = snapshot.selected_run;
        imp.waste_selected.set(snapshot.selected_waste);
        imp.move_count.set(snapshot.move_count);
        imp.elapsed_seconds.set(snapshot.elapsed_seconds);
        imp.timer_started.set(snapshot.timer_started);
        *imp.apm_samples.borrow_mut() = snapshot.apm_samples;
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        let state_hash = self.current_game_hash();
        self.start_hint_loss_analysis_if_needed(state_hash);
    }
}

use super::*;
use crate::engine::boundary;

impl CardthropicWindow {
    pub(super) fn snapshot(&self) -> Snapshot {
        let imp = self.imp();
        let mode = self.active_game_mode();
        let runtime = {
            let game = imp.game.borrow();
            game.runtime_for_mode(mode)
        };
        let chess_mode_active = imp.chess_mode_active.get();
        Snapshot {
            mode,
            runtime,
            draw_mode: imp.klondike_draw_mode.get(),
            selected_run: *imp.selected_run.borrow(),
            selected_waste: imp.waste_selected.get(),
            move_count: imp.move_count.get(),
            elapsed_seconds: imp.elapsed_seconds.get(),
            timer_started: imp.timer_started.get(),
            apm_elapsed_offset_seconds: imp.apm_elapsed_offset_seconds.get(),
            apm_samples: imp.apm_samples.borrow().clone(),
            foundation_slot_suits: self.foundation_slot_suits_snapshot(),
            chess_mode_active,
            chess_variant: imp.chess_variant.get(),
            chess_position: if chess_mode_active {
                Some(imp.chess_position.borrow().clone())
            } else {
                None
            },
            chess_selected_square: if chess_mode_active {
                imp.chess_selected_square.get()
            } else {
                None
            },
            chess_last_move_from: if chess_mode_active {
                imp.chess_last_move_from.get()
            } else {
                None
            },
            chess_last_move_to: if chess_mode_active {
                imp.chess_last_move_to.get()
            } else {
                None
            },
            chess_history: if chess_mode_active {
                imp.chess_history.borrow().clone()
            } else {
                Vec::new()
            },
            chess_future: if chess_mode_active {
                imp.chess_future.borrow().clone()
            } else {
                Vec::new()
            },
        }
    }

    pub(super) fn apply_changed_move(&self, snapshot: Snapshot, changed: bool) -> bool {
        if changed {
            let imp = self.imp();
            self.clear_hint_effects();
            imp.waste_selected.set(false);
            imp.selected_freecell.set(None);
            imp.history.borrow_mut().push(snapshot);
            imp.future.borrow_mut().clear();
            imp.move_count.set(imp.move_count.get() + 1);
            imp.timer_started.set(true);
            if !(imp.robot_mode_running.get() || imp.auto_playing_move.get()) {
                *imp.status_override.borrow_mut() = None;
            }
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
        if self.imp().history.borrow().is_empty()
            && self.imp().chess_mode_active.get()
            && !self.imp().chess_history.borrow().is_empty()
        {
            let _ = self.chess_undo();
            return;
        }
        if !self.guard_mode_engine("Undo") {
            return;
        }
        let imp = self.imp();
        let Some(snapshot) = imp.history.borrow_mut().pop() else {
            *imp.status_override.borrow_mut() = Some("Nothing to undo yet.".to_string());
            self.render();
            return;
        };
        let previous_mode = self.active_game_mode();
        let restored_mode = snapshot.mode;
        let restored_move_count = snapshot.move_count;
        let restored_elapsed = snapshot.elapsed_seconds;
        let restored_draw = snapshot.draw_mode;

        self.clear_hint_effects();
        imp.future.borrow_mut().push(self.snapshot());
        self.restore_snapshot(snapshot);
        if restored_mode != previous_mode {
            let mode_detail = match restored_mode {
                GameMode::Klondike => format!("deal={}", restored_draw.count()),
                GameMode::Spider => format!(
                    "suits={}",
                    self.imp().game.borrow().spider().suit_mode().suit_count()
                ),
                GameMode::Freecell => "suits=na".to_string(),
            };
            *imp.status_override.borrow_mut() = Some(format!(
                "Undo restored mode={} {} moves={} elapsed={}.",
                restored_mode.id(),
                mode_detail,
                restored_move_count,
                restored_elapsed
            ));
        } else {
            *imp.status_override.borrow_mut() = Some("Undid last move.".to_string());
        }
        self.render();
    }

    pub(super) fn redo(&self) {
        if self.imp().future.borrow().is_empty()
            && self.imp().chess_mode_active.get()
            && !self.imp().chess_future.borrow().is_empty()
        {
            let _ = self.chess_redo();
            return;
        }
        if !self.guard_mode_engine("Redo") {
            return;
        }
        let imp = self.imp();
        let Some(snapshot) = imp.future.borrow_mut().pop() else {
            *imp.status_override.borrow_mut() = Some("Nothing to redo yet.".to_string());
            self.render();
            return;
        };
        let previous_mode = self.active_game_mode();
        let restored_mode = snapshot.mode;
        let restored_move_count = snapshot.move_count;
        let restored_elapsed = snapshot.elapsed_seconds;
        let restored_draw = snapshot.draw_mode;

        self.clear_hint_effects();
        imp.history.borrow_mut().push(self.snapshot());
        self.restore_snapshot(snapshot);
        if restored_mode != previous_mode {
            let mode_detail = match restored_mode {
                GameMode::Klondike => format!("deal={}", restored_draw.count()),
                GameMode::Spider => format!(
                    "suits={}",
                    self.imp().game.borrow().spider().suit_mode().suit_count()
                ),
                GameMode::Freecell => "suits=na".to_string(),
            };
            *imp.status_override.borrow_mut() = Some(format!(
                "Redo restored mode={} {} moves={} elapsed={}.",
                restored_mode.id(),
                mode_detail,
                restored_move_count,
                restored_elapsed
            ));
        } else {
            *imp.status_override.borrow_mut() = Some("Redid move.".to_string());
        }
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
            imp.spider_suit_mode.set(game.spider().suit_mode());
        }
        *imp.selected_run.borrow_mut() = snapshot.selected_run;
        imp.selected_freecell.set(None);
        imp.waste_selected.set(snapshot.selected_waste);
        self.set_foundation_slot_suits(snapshot.foundation_slot_suits);
        imp.chess_mode_active.set(snapshot.chess_mode_active);
        imp.chess_variant.set(snapshot.chess_variant);
        *imp.chess_position.borrow_mut() = snapshot
            .chess_position
            .unwrap_or_else(|| crate::game::standard_position());
        imp.chess_selected_square
            .set(if snapshot.chess_mode_active {
                snapshot.chess_selected_square
            } else {
                None
            });
        imp.chess_last_move_from.set(if snapshot.chess_mode_active {
            snapshot.chess_last_move_from
        } else {
            None
        });
        imp.chess_last_move_to.set(if snapshot.chess_mode_active {
            snapshot.chess_last_move_to
        } else {
            None
        });
        imp.chess_keyboard_square
            .set(if snapshot.chess_mode_active {
                snapshot.chess_selected_square
            } else {
                None
            });
        *imp.chess_history.borrow_mut() = snapshot.chess_history;
        *imp.chess_future.borrow_mut() = snapshot.chess_future;
        imp.move_count.set(snapshot.move_count);
        imp.elapsed_seconds.set(snapshot.elapsed_seconds);
        imp.timer_started.set(snapshot.timer_started);
        imp.apm_elapsed_offset_seconds
            .set(snapshot.apm_elapsed_offset_seconds);
        *imp.apm_samples.borrow_mut() = snapshot.apm_samples;
        self.update_game_mode_menu_selection();
        self.update_game_settings_menu();
        if snapshot.chess_mode_active {
            self.cancel_hint_loss_analysis();
        } else {
            self.reset_hint_cycle_memory();
            self.reset_auto_play_memory();
            let state_hash = self.current_game_hash();
            self.start_hint_loss_analysis_if_needed(state_hash);
        }
        let _ = self.maybe_auto_flip_chess_board_to_side_to_move(false);
    }
}

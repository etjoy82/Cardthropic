use crate::engine::chess::boundary as chess_boundary;
use crate::engine::chess::commands::ChessCommand;
use crate::game::{legal_moves, ChessColor, ChessVariant};
use crate::CardthropicWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::glib;
use std::time::Duration;

impl CardthropicWindow {
    pub(crate) fn launch_chess_standard_placeholder(&self) {
        self.start_chess_variant(ChessVariant::Standard);
    }

    pub(crate) fn launch_chess960_placeholder(&self) {
        self.start_chess_variant(ChessVariant::Chess960);
    }

    pub(crate) fn launch_chess_atomic_placeholder(&self) {
        self.start_chess_variant(ChessVariant::Atomic);
    }

    fn start_chess_variant(&self, variant: ChessVariant) {
        let imp = self.imp();
        let undo_anchor = self.snapshot();
        let seed = imp.current_seed.get();
        imp.chess_variant.set(variant);
        imp.chess_mode_active.set(true);
        self.reset_chess_session_state();
        imp.move_count.set(0);
        imp.elapsed_seconds.set(0);
        imp.timer_started.set(false);
        let legal_count = {
            let mut position = imp.chess_position.borrow_mut();
            let result =
                chess_boundary::execute(&mut position, ChessCommand::NewGame { seed, variant });
            if !result.changed {
                *imp.status_override.borrow_mut() = Some(format!(
                    "{} initialization is not available yet.",
                    variant.label()
                ));
                self.popdown_main_menu_later();
                self.render();
                return;
            }
            if position.side_to_move() != ChessColor::White {
                position.set_side_to_move(ChessColor::White);
                self.append_status_history_only(
                    "chess_turn_guard: start variant corrected opening side from black to white",
                );
            }
            legal_moves(&position).len()
        };
        *imp.status_override.borrow_mut() = Some(format!(
            "{} initialized from seed {}. {} legal opening moves. White to move.",
            variant.label(),
            seed,
            legal_count
        ));
        self.append_status_history_only(&self.new_game_started_timestamp_status());
        imp.history.borrow_mut().push(undo_anchor);
        imp.future.borrow_mut().clear();
        imp.last_metrics_key.set(0);
        imp.last_stock_waste_foundation_size
            .set((0, 0, imp.current_game_mode.get(), 0));
        self.handle_window_geometry_change();
        self.popdown_main_menu_later();
        let rendered_by_flip = self.maybe_auto_flip_chess_board_to_side_to_move(false);
        if !rendered_by_flip {
            self.render();
        }
        self.schedule_chess_viewport_sync();
        self.maybe_autoplay_chess_opening_white_move();
    }

    pub(in crate::window) fn start_new_chess_game_with_seed(&self, seed: u64, status: String) {
        self.start_new_chess_game_with_seed_internal(seed, status, false);
    }

    pub(in crate::window) fn start_new_chess_game_with_seed_preserving_robot(
        &self,
        seed: u64,
        status: String,
    ) {
        self.start_new_chess_game_with_seed_internal(seed, status, true);
    }

    fn start_new_chess_game_with_seed_internal(
        &self,
        seed: u64,
        status: String,
        preserve_robot: bool,
    ) {
        let imp = self.imp();
        let variant = imp.chess_variant.get();

        if !preserve_robot {
            self.stop_robot_mode();
        }
        self.cancel_hint_loss_analysis();
        self.cancel_seed_winnable_check(None);
        self.clear_hint_effects();

        imp.chess_mode_active.set(true);
        imp.chess_variant.set(variant);
        self.reset_chess_session_state();

        imp.current_seed.set(seed);
        self.set_seed_input_text(&seed.to_string());
        self.clear_seed_entry_feedback();
        *imp.selected_run.borrow_mut() = None;
        imp.selected_freecell.set(None);
        imp.waste_selected.set(false);
        imp.history.borrow_mut().clear();
        imp.future.borrow_mut().clear();
        self.roll_apm_timeline_forward();
        imp.move_count.set(0);
        imp.elapsed_seconds.set(0);
        imp.timer_started.set(false);
        self.note_seed_play_started(seed);
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        if preserve_robot {
            self.reset_robot_search_tracking_for_current_deal();
        }

        let changed = {
            let mut position = imp.chess_position.borrow_mut();
            let result =
                chess_boundary::execute(&mut position, ChessCommand::NewGame { seed, variant });
            if result.changed && position.side_to_move() != ChessColor::White {
                position.set_side_to_move(ChessColor::White);
                self.append_status_history_only(
                    "chess_turn_guard: new seeded game corrected opening side from black to white",
                );
            }
            result.changed
        };
        if !changed {
            *imp.status_override.borrow_mut() = Some(format!(
                "{} initialization is not available yet.",
                variant.label()
            ));
            self.render();
            return;
        }

        let status = if status.to_ascii_lowercase().contains("white to move") {
            status
        } else {
            let mut decorated = status.trim().to_string();
            if !decorated.ends_with('.') {
                decorated.push('.');
            }
            decorated.push_str(" White to move.");
            decorated
        };
        *imp.status_override.borrow_mut() = Some(status);
        self.append_status_history_only(&self.new_game_started_timestamp_status());
        let rendered_by_flip = self.maybe_auto_flip_chess_board_to_side_to_move(false);
        if !rendered_by_flip {
            self.render();
        }
        self.maybe_autoplay_chess_opening_white_move();
    }

    fn maybe_autoplay_chess_opening_white_move(&self) {
        if !(self.chess_wand_ai_opponent_auto_response_enabled()
            && self.chess_auto_response_plays_white_enabled())
        {
            return;
        }
        let imp = self.imp();
        if !imp.chess_mode_active.get() || imp.move_count.get() != 0 || imp.robot_mode_running.get()
        {
            return;
        }
        let position = imp.chess_position.borrow().clone();
        if position.side_to_move() != ChessColor::White || legal_moves(&position).is_empty() {
            return;
        }

        glib::idle_add_local_once(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move || {
                let imp = window.imp();
                if !imp.chess_mode_active.get()
                    || imp.move_count.get() != 0
                    || imp.robot_mode_running.get()
                {
                    return;
                }
                let position = imp.chess_position.borrow().clone();
                if position.side_to_move() != ChessColor::White || legal_moves(&position).is_empty()
                {
                    return;
                }
                let _ = window.play_chess_ai_hint_move_single();
            }
        ));
    }

    fn schedule_chess_viewport_sync(&self) {
        glib::timeout_add_local_once(
            Duration::from_millis(20),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move || {
                    if !window.imp().chess_mode_active.get() {
                        return;
                    }
                    window.update_tableau_metrics();
                    window.render();
                }
            ),
        );
    }
}

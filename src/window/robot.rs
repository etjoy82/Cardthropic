use super::*;
use crate::engine::boundary;
use crate::engine::seed_ops;

impl CardthropicWindow {
    pub(super) fn trigger_rapid_wand(&self) {
        if !self.guard_mode_engine("Rapid Wand") {
            return;
        }
        if self.imp().rapid_wand_running.get() {
            return;
        }
        self.imp().rapid_wand_running.set(true);

        self.stop_rapid_wand();
        self.imp().rapid_wand_running.set(true);

        let profile = self.automation_profile();
        self.play_hint_for_player();
        let remaining_steps = Rc::new(Cell::new(profile.rapid_wand_total_steps.saturating_sub(1)));
        let timer = glib::timeout_add_local(
            Duration::from_millis(profile.rapid_wand_interval_ms),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                remaining_steps,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    if remaining_steps.get() == 0 {
                        window.finish_rapid_wand();
                        return glib::ControlFlow::Break;
                    }

                    window.play_hint_for_player();
                    remaining_steps.set(remaining_steps.get().saturating_sub(1));
                    if remaining_steps.get() == 0 {
                        window.finish_rapid_wand();
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                }
            ),
        );
        *self.imp().rapid_wand_timer.borrow_mut() = Some(timer);
    }

    pub(super) fn stop_rapid_wand(&self) {
        self.imp().rapid_wand_running.set(false);
        if let Some(source_id) = self.imp().rapid_wand_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
    }

    pub(super) fn finish_rapid_wand(&self) {
        self.imp().rapid_wand_running.set(false);
        let _ = self.imp().rapid_wand_timer.borrow_mut().take();
    }

    pub(super) fn toggle_robot_mode(&self) {
        if self.imp().robot_mode_running.get() {
            self.stop_robot_mode();
        } else {
            let mode = self.active_game_mode();
            if boundary::is_won(&self.imp().game.borrow(), mode) {
                self.start_random_seed_game();
                if boundary::is_won(&self.imp().game.borrow(), mode) {
                    return;
                }
            }
            let use_solver_line = self.robot_solver_anchor_matches_current_state()
                && self.imp().robot_playback.borrow().has_scripted_line();
            self.imp()
                .robot_playback
                .borrow_mut()
                .set_use_scripted_line(use_solver_line);
            self.start_robot_mode();
        }
    }

    pub(super) fn start_robot_mode(&self) {
        if self.imp().robot_mode_running.get() {
            return;
        }
        if !self.guard_mode_engine("Robot Mode") {
            return;
        }

        self.stop_rapid_wand();
        self.imp().robot_mode_running.set(true);
        self.imp().robot_deals_tried.set(0);
        let using_solver = self.imp().robot_playback.borrow().use_scripted_line();
        *self.imp().status_override.borrow_mut() = Some(if using_solver {
            "Robot Mode active (solver line armed). Deals tried: 0. Click anywhere to stop."
                .to_string()
        } else {
            "Robot Mode active. Deals tried: 0. Click anywhere to stop.".to_string()
        });
        self.render();

        self.robot_mode_step();

        let profile = self.automation_profile();
        let timer = glib::timeout_add_local(
            Duration::from_millis(profile.robot_step_interval_ms),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    if !window.imp().robot_mode_running.get() {
                        return glib::ControlFlow::Break;
                    }
                    window.robot_mode_step();
                    glib::ControlFlow::Continue
                }
            ),
        );
        *self.imp().robot_mode_timer.borrow_mut() = Some(timer);
    }

    pub(super) fn robot_mode_step(&self) {
        if !self.imp().robot_mode_running.get() {
            return;
        }

        let mode = self.active_game_mode();
        let was_won = boundary::is_won(&self.imp().game.borrow(), mode);
        if was_won {
            self.stop_robot_mode_with_message("Robot Mode stopped: game won.");
            return;
        }

        let moved = if self.imp().robot_playback.borrow().use_scripted_line() {
            if let Some(hint_move) = self.imp().robot_playback.borrow_mut().pop_scripted_move() {
                self.imp().auto_playing_move.set(true);
                let changed = self.apply_hint_move(hint_move);
                self.imp().auto_playing_move.set(false);
                if changed {
                    *self.imp().selected_run.borrow_mut() = None;
                    self.render();
                    true
                } else {
                    self.imp().robot_playback.borrow_mut().clear_scripted_line();
                    self.stop_robot_mode_with_message(
                        "Robot Mode stopped: stored solver line became invalid.",
                    );
                    return;
                }
            } else {
                self.stop_robot_mode_with_message(
                    "Robot Mode stopped: stored solver line unavailable.",
                );
                return;
            }
        } else {
            self.play_hint_for_player()
        };
        let now_won = boundary::is_won(&self.imp().game.borrow(), mode);
        if now_won {
            self.stop_robot_mode_with_message("Robot Mode stopped: game won.");
            return;
        }

        if !moved {
            self.imp()
                .robot_playback
                .borrow_mut()
                .set_use_scripted_line(false);
            let next_deals_tried = self.imp().robot_deals_tried.get().saturating_add(1);
            self.imp().robot_deals_tried.set(next_deals_tried);
            let seed = seed_ops::random_seed();
            self.start_new_game_with_seed_internal(
                seed,
                format!(
                    "Robot Mode: stuck/lost. Dealt new seed {seed}. Deals tried: {next_deals_tried}."
                ),
                true,
            );
        }
    }

    pub(super) fn stop_robot_mode(&self) {
        if !self.imp().robot_mode_running.replace(false) {
            return;
        }
        self.imp().robot_playback.borrow_mut().clear_scripted_line();
        if let Some(source_id) = self.imp().robot_mode_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        let deals_tried = self.imp().robot_deals_tried.get();
        *self.imp().status_override.borrow_mut() =
            Some(format!("Robot Mode stopped. Deals tried: {deals_tried}."));
        self.render();
    }

    pub(super) fn stop_robot_mode_with_message(&self, message: &str) {
        if !self.imp().robot_mode_running.replace(false) {
            return;
        }
        self.imp().robot_playback.borrow_mut().clear_scripted_line();
        if let Some(source_id) = self.imp().robot_mode_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        let deals_tried = self.imp().robot_deals_tried.get();
        *self.imp().status_override.borrow_mut() =
            Some(format!("{message} Deals tried: {deals_tried}."));
        self.render();
    }

    pub(super) fn arm_robot_solver_anchor_for_current_state(&self, line: Vec<HintMove>) {
        self.imp().robot_playback.borrow_mut().arm(
            self.imp().current_seed.get(),
            self.current_klondike_draw_mode(),
            self.imp().move_count.get(),
            self.current_game_hash(),
            line,
        );
    }

    pub(super) fn robot_solver_anchor_matches_current_state(&self) -> bool {
        self.imp().robot_playback.borrow().matches_current(
            self.imp().current_seed.get(),
            self.current_klondike_draw_mode(),
            self.imp().move_count.get(),
            self.current_game_hash(),
        )
    }
}

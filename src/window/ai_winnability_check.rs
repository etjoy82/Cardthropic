use super::*;
use crate::engine::seed_ops;

impl CardthropicWindow {
    pub(super) fn cancel_seed_winnable_check(&self, status: Option<&str>) {
        let imp = self.imp();
        if !imp.seed_check_running.get() {
            return;
        }

        imp.seed_check_running.set(false);
        imp.seed_check_generation
            .set(imp.seed_check_generation.get().wrapping_add(1));
        if let Some(cancel_flag) = imp.seed_check_cancel.borrow_mut().take() {
            cancel_flag.store(true, Ordering::Relaxed);
        }
        if let Some(source_id) = imp.seed_check_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        imp.seed_winnable_button
            .set_label(SEED_WINNABLE_BUTTON_LABEL);

        if let Some(message) = status {
            *imp.status_override.borrow_mut() = Some(message.to_string());
            self.render();
        }
    }

    pub(super) fn finish_seed_winnable_check(&self, generation: u64) -> bool {
        let imp = self.imp();
        if !imp.seed_check_running.get() || imp.seed_check_generation.get() != generation {
            return false;
        }

        imp.seed_check_running.set(false);
        imp.seed_check_cancel.borrow_mut().take();
        if let Some(source_id) = imp.seed_check_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        imp.seed_winnable_button
            .set_label(SEED_WINNABLE_BUTTON_LABEL);
        true
    }

    pub(super) fn toggle_seed_winnable_check(&self) {
        if !self.guard_mode_engine("Winnability analysis") {
            return;
        }
        if self.imp().seed_check_running.get() {
            let deal_count = self.current_klondike_draw_mode().count();
            self.cancel_seed_winnable_check(Some(&seed_ops::msg_winnability_check_canceled(
                deal_count,
            )));
            return;
        }

        self.clear_seed_entry_feedback();
        let seed = match self.seed_from_controls_or_random() {
            Ok(seed) => seed,
            Err(message) => {
                if let Some(entry) = self.seed_text_entry() {
                    entry.add_css_class("error");
                }
                *self.imp().status_override.borrow_mut() = Some(message);
                self.render();
                return;
            }
        };

        let imp = self.imp();
        imp.seed_check_running.set(true);
        let generation = imp.seed_check_generation.get().wrapping_add(1);
        imp.seed_check_generation.set(generation);
        imp.seed_check_seconds.set(1);
        imp.seed_winnable_button.set_label("Checking 1s");
        if let Some(source_id) = imp.seed_check_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }

        let tick = glib::timeout_add_seconds_local(
            1,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let imp = window.imp();
                    if !imp.seed_check_running.get()
                        || imp.seed_check_generation.get() != generation
                    {
                        return glib::ControlFlow::Break;
                    }

                    let next = imp.seed_check_seconds.get().saturating_add(1);
                    imp.seed_check_seconds.set(next);
                    imp.seed_winnable_button
                        .set_label(&format!("Checking {next}s"));
                    glib::ControlFlow::Continue
                }
            ),
        );
        *self.imp().seed_check_timer.borrow_mut() = Some(tick);

        let cancel_flag = Arc::new(AtomicBool::new(false));
        *self.imp().seed_check_cancel.borrow_mut() = Some(Arc::clone(&cancel_flag));

        let (sender, receiver) = mpsc::channel::<Option<winnability::SeedWinnabilityCheckResult>>();
        let draw_mode = self.current_klondike_draw_mode();
        let deal_count = draw_mode.count();
        let profile = self.automation_profile();
        thread::spawn(move || {
            let result = winnability::is_seed_winnable(
                seed,
                draw_mode,
                profile.dialog_seed_guided_budget,
                profile.dialog_seed_exhaustive_budget,
                &cancel_flag,
            );
            let _ = sender.send(result);
        });

        glib::timeout_add_local(
            Duration::from_millis(40),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    if !window.imp().seed_check_running.get()
                        || window.imp().seed_check_generation.get() != generation
                    {
                        return glib::ControlFlow::Break;
                    }

                    match receiver.try_recv() {
                        Ok(Some(result)) => {
                            if !window.finish_seed_winnable_check(generation) {
                                return glib::ControlFlow::Break;
                            }

                            let current_seed =
                                seed_ops::parse_seed_input(&window.seed_input_text())
                                    .ok()
                                    .flatten();
                            if current_seed != Some(seed) {
                                return glib::ControlFlow::Break;
                            }

                            window.clear_seed_entry_feedback();
                            if result.winnable {
                                if let Some(entry) = window.seed_text_entry() {
                                    entry.add_css_class("seed-winnable");
                                }
                                if let Some(line) = result.solver_line.as_ref().and_then(|line| {
                                    let game = window.imp().game.borrow().clone();
                                    map_solver_line_to_hint_line(&game, line.as_slice())
                                }) {
                                    window.arm_robot_solver_anchor_for_current_state(line);
                                }
                                let moves = result.moves_to_win.unwrap_or(0);
                                *window.imp().status_override.borrow_mut() =
                                    Some(seed_ops::msg_seed_winnable(
                                        seed,
                                        deal_count,
                                        moves,
                                        result.iterations,
                                    ));
                            } else {
                                if let Some(entry) = window.seed_text_entry() {
                                    entry.add_css_class("seed-unwinnable");
                                }
                                let message = if result.hit_state_limit {
                                    seed_ops::msg_seed_unwinnable_limited(
                                        seed,
                                        deal_count,
                                        result.iterations,
                                    )
                                } else {
                                    seed_ops::msg_seed_unwinnable(
                                        seed,
                                        deal_count,
                                        result.iterations,
                                    )
                                };
                                *window.imp().status_override.borrow_mut() = Some(message);
                            }
                            window.render();
                            glib::ControlFlow::Break
                        }
                        Ok(None) => {
                            window.finish_seed_winnable_check(generation);
                            glib::ControlFlow::Break
                        }
                        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            if window.finish_seed_winnable_check(generation) {
                                *window.imp().status_override.borrow_mut() =
                                    Some(seed_ops::msg_winnability_check_stopped_unexpectedly(
                                        deal_count,
                                    ));
                                window.render();
                            }
                            glib::ControlFlow::Break
                        }
                    }
                }
            ),
        );
    }
}

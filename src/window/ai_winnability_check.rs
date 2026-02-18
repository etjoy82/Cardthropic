use super::*;
use crate::engine::seed_ops;

const SEED_WINNABILITY_TIMEOUT_SECS: u32 = 300;
const SEED_WINNABILITY_MEMORY_HEADROOM_MIB: u64 = 512;
const SEED_WINNABILITY_MEMORY_MAX_MIB: u64 = 1024;

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
        imp.seed_check_memory_guard_triggered.set(false);
        imp.seed_check_memory_limit_mib.set(0);
        self.trim_process_memory_if_supported();

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
        imp.seed_check_memory_guard_triggered.set(false);
        imp.seed_check_memory_limit_mib.set(0);
        self.trim_process_memory_if_supported();
        true
    }

    pub(super) fn toggle_seed_winnable_check(&self) {
        if !self.guard_mode_engine("Winnability analysis") {
            return;
        }
        let mode = self.active_game_mode();
        if self.imp().seed_check_running.get() {
            let cancel_message = if mode == GameMode::Spider {
                let suit_count = self.current_spider_suit_mode().suit_count();
                format!("Winnability check canceled (Spider {suit_count}-suit).")
            } else if mode == GameMode::Freecell {
                "Winnability check canceled (FreeCell).".to_string()
            } else {
                seed_ops::msg_winnability_check_canceled(self.current_klondike_draw_mode().count())
            };
            self.cancel_seed_winnable_check(Some(cancel_message.as_str()));
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
        imp.seed_check_memory_guard_triggered.set(false);
        let baseline_mib = self.current_memory_mib();
        let memory_limit_mib = baseline_mib
            .map(|m| m.saturating_add(SEED_WINNABILITY_MEMORY_HEADROOM_MIB))
            .unwrap_or(SEED_WINNABILITY_MEMORY_MAX_MIB)
            .min(SEED_WINNABILITY_MEMORY_MAX_MIB);
        imp.seed_check_memory_limit_mib.set(memory_limit_mib);
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
                    if window
                        .current_memory_mib()
                        .is_some_and(|mib| mib > imp.seed_check_memory_limit_mib.get())
                    {
                        imp.seed_check_memory_guard_triggered.set(true);
                        if let Some(cancel_flag) = imp.seed_check_cancel.borrow().as_ref() {
                            cancel_flag.store(true, Ordering::Relaxed);
                        }
                        imp.seed_winnable_button.set_label("Stopping...");
                        return glib::ControlFlow::Continue;
                    }
                    if next >= SEED_WINNABILITY_TIMEOUT_SECS {
                        if let Some(cancel_flag) = imp.seed_check_cancel.borrow().as_ref() {
                            cancel_flag.store(true, Ordering::Relaxed);
                        }
                        imp.seed_winnable_button.set_label("Stopping...");
                        return glib::ControlFlow::Continue;
                    }
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
        let spider_suit_mode = self.current_spider_suit_mode();
        let spider_suit_count = spider_suit_mode.suit_count();
        let freecell_card_count_mode = self.current_freecell_card_count_mode();
        let freecell_card_count = freecell_card_count_mode.card_count();
        let profile = self.automation_profile();
        *self.imp().status_override.borrow_mut() = Some(match mode {
            GameMode::Spider => format!(
                "W? checking seed {seed} for Spider {spider_suit_count}-suit (up to {}s)...",
                SEED_WINNABILITY_TIMEOUT_SECS
            ),
            GameMode::Freecell => format!(
                "W? checking seed {seed} for FreeCell {freecell_card_count} (up to {}s)...",
                SEED_WINNABILITY_TIMEOUT_SECS
            ),
            GameMode::Klondike => format!(
                "W? checking seed {seed} for Deal {deal_count} (up to {}s)...",
                SEED_WINNABILITY_TIMEOUT_SECS
            ),
        });
        self.render();
        thread::spawn(move || {
            let result = if mode == GameMode::Spider {
                winnability::is_spider_seed_winnable(
                    seed,
                    spider_suit_mode,
                    profile.dialog_seed_guided_budget,
                    profile.dialog_seed_exhaustive_budget,
                    &cancel_flag,
                )
            } else if mode == GameMode::Freecell {
                winnability::is_freecell_seed_winnable(
                    seed,
                    freecell_card_count_mode,
                    profile.dialog_seed_guided_budget,
                    profile.dialog_seed_exhaustive_budget,
                    &cancel_flag,
                )
            } else {
                winnability::is_seed_winnable(
                    seed,
                    draw_mode,
                    profile.dialog_seed_guided_budget,
                    profile.dialog_seed_exhaustive_budget,
                    &cancel_flag,
                )
            };
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

                            if result.canceled {
                                let memory_guarded =
                                    window.imp().seed_check_memory_guard_triggered.get();
                                let memory_limit = window.imp().seed_check_memory_limit_mib.get();
                                let message = if memory_guarded {
                                    match mode {
                                        GameMode::Spider => format!(
                                            "Winnability check stopped by memory guard at ~{} MiB (Spider {spider_suit_count}-suit): no wins found in {} iterations.",
                                            memory_limit, result.iterations
                                        ),
                                        GameMode::Freecell => format!(
                                            "Winnability check stopped by memory guard at ~{} MiB (FreeCell {freecell_card_count}): no wins found in {} iterations.",
                                            memory_limit, result.iterations
                                        ),
                                        GameMode::Klondike => format!(
                                            "Winnability check stopped by memory guard at ~{} MiB (Deal {deal_count}): no wins found in {} iterations.",
                                            memory_limit, result.iterations
                                        ),
                                    }
                                } else {
                                    match mode {
                                        GameMode::Spider => format!(
                                            "Winnability check timed out after {}s (Spider {spider_suit_count}-suit): no wins found in {} iterations before giving up.",
                                            SEED_WINNABILITY_TIMEOUT_SECS, result.iterations
                                        ),
                                        GameMode::Freecell => format!(
                                            "Winnability check timed out after {}s (FreeCell {freecell_card_count}): no wins found in {} iterations before giving up.",
                                            SEED_WINNABILITY_TIMEOUT_SECS, result.iterations
                                        ),
                                        GameMode::Klondike => seed_ops::msg_winnability_check_timed_out(
                                            deal_count,
                                            SEED_WINNABILITY_TIMEOUT_SECS,
                                            result.iterations,
                                        ),
                                    }
                                };
                                *window.imp().status_override.borrow_mut() = Some(message);
                                window.render();
                                return glib::ControlFlow::Break;
                            }

                            window.clear_seed_entry_feedback();
                            if result.winnable {
                                if let Some(entry) = window.seed_text_entry() {
                                    entry.add_css_class("seed-winnable");
                                }
                                if mode == GameMode::Freecell {
                                    if window.imp().move_count.get() == 0 {
                                        if let Some(line) = result.freecell_line.clone() {
                                            window
                                                .arm_robot_freecell_solver_anchor_for_current_state(
                                                    line,
                                                );
                                        }
                                    }
                                } else if let Some(line) = result.hint_line.clone().or_else(|| {
                                    result.solver_line.as_ref().and_then(|line| {
                                        let game = window.imp().game.borrow().clone();
                                        map_solver_line_to_hint_line(&game, line.as_slice())
                                    })
                                }) {
                                    window.arm_robot_solver_anchor_for_current_state(line);
                                }
                                let moves = result.moves_to_win.unwrap_or(0);
                                let message = match mode {
                                    GameMode::Spider => format!(
                                        "Seed {seed} is winnable for Spider {spider_suit_count}-suit from a fresh deal (solver line: {moves} moves, {} iterations). Use Robot as first action to see win.",
                                        result.iterations
                                    ),
                                    GameMode::Freecell if window.imp().move_count.get() == 0 => format!(
                                        "Seed {seed} is winnable for FreeCell {freecell_card_count} from a fresh deal (solver line: {moves} moves, {} iterations). Use Robot as first action to see win.",
                                        result.iterations
                                    ),
                                    GameMode::Freecell => format!(
                                        "Seed {seed} is winnable for FreeCell {freecell_card_count} from a fresh deal (solver line: {moves} moves, {} iterations). Start a fresh deal and use Robot as first action to see win.",
                                        result.iterations
                                    ),
                                    GameMode::Klondike => seed_ops::msg_seed_winnable(
                                        seed,
                                        deal_count,
                                        moves,
                                        result.iterations,
                                    ),
                                };
                                *window.imp().status_override.borrow_mut() = Some(message);
                            } else {
                                if let Some(entry) = window.seed_text_entry() {
                                    entry.add_css_class("seed-unwinnable");
                                }
                                let message = match mode {
                                    GameMode::Spider if result.hit_state_limit => format!(
                                        "Seed {seed} not proven winnable for Spider {spider_suit_count}-suit from a fresh deal ({} iterations, limits hit).",
                                        result.iterations
                                    ),
                                    GameMode::Spider => format!(
                                        "Seed {seed} is not winnable for Spider {spider_suit_count}-suit from a fresh deal ({} iterations).",
                                        result.iterations
                                    ),
                                    GameMode::Freecell if result.hit_state_limit => format!(
                                        "Seed {seed} not proven winnable for FreeCell {freecell_card_count} from a fresh deal ({} iterations, limits hit).",
                                        result.iterations
                                    ),
                                    GameMode::Freecell => format!(
                                        "Seed {seed} is not winnable for FreeCell {freecell_card_count} from a fresh deal ({} iterations).",
                                        result.iterations
                                    ),
                                    GameMode::Klondike if result.hit_state_limit => {
                                        seed_ops::msg_seed_unwinnable_limited(
                                            seed,
                                            deal_count,
                                            result.iterations,
                                        )
                                    }
                                    GameMode::Klondike => {
                                        seed_ops::msg_seed_unwinnable(
                                            seed,
                                            deal_count,
                                            result.iterations,
                                        )
                                    }
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
                                let message = match mode {
                                    GameMode::Spider => format!(
                                        "Winnability check stopped unexpectedly (Spider {spider_suit_count}-suit)."
                                    ),
                                    GameMode::Freecell => {
                                        format!(
                                            "Winnability check stopped unexpectedly (FreeCell {freecell_card_count})."
                                        )
                                    }
                                    GameMode::Klondike => {
                                        seed_ops::msg_winnability_check_stopped_unexpectedly(
                                            deal_count,
                                        )
                                    }
                                };
                                *window.imp().status_override.borrow_mut() = Some(message);
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

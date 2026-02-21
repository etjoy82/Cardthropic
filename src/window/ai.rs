use super::*;
use crate::engine::seed_ops;
use std::sync::atomic::AtomicU32;

const FIND_WINNABLE_MEMORY_HEADROOM_MIB: u64 = 512;
const FIND_WINNABLE_MEMORY_MAX_MIB: u64 = 1024;
const SPIDER_FIND_WINNABLE_ATTEMPTS_ONE_SUIT: u32 = 20_000;
const SPIDER_FIND_WINNABLE_ATTEMPTS_TWO_SUIT: u32 = 4_000;
const SPIDER_FIND_WINNABLE_ATTEMPTS_THREE_SUIT: u32 = 8_000;
const SPIDER_FIND_WINNABLE_ATTEMPTS_FOUR_SUIT: u32 = 20_000;

impl CardthropicWindow {
    pub(super) fn cancel_winnable_seed_search(&self, status: Option<&str>) {
        if let Some(cancel) = self.imp().seed_search_cancel.borrow_mut().take() {
            cancel.store(true, Ordering::Relaxed);
        }
        if let Some(message) = status {
            *self.imp().status_override.borrow_mut() = Some(message.to_string());
            self.render();
        }
    }

    pub(super) fn start_random_winnable_seed_game(&self) {
        if self.imp().chess_mode_active.get() {
            *self.imp().status_override.borrow_mut() =
                Some("Find Winnable Seed not applicable to Chess".to_string());
            self.render();
            return;
        }
        if !self.guard_mode_engine("Starting a winnable deal") {
            return;
        }
        if self.imp().seed_search_in_progress.get() {
            self.cancel_winnable_seed_search(Some("Canceled winnable-seed search."));
            return;
        }

        self.cancel_seed_winnable_check(None);
        self.clear_seed_entry_feedback();
        let start_seed = seed_ops::random_seed();
        self.set_seed_input_text(&start_seed.to_string());

        let attempts = if self.active_game_mode() == GameMode::Freecell {
            10_000
        } else if self.active_game_mode() == GameMode::Spider {
            match self.current_spider_suit_mode() {
                SpiderSuitMode::One => SPIDER_FIND_WINNABLE_ATTEMPTS_ONE_SUIT,
                SpiderSuitMode::Two => SPIDER_FIND_WINNABLE_ATTEMPTS_TWO_SUIT,
                SpiderSuitMode::Three => SPIDER_FIND_WINNABLE_ATTEMPTS_THREE_SUIT,
                SpiderSuitMode::Four => SPIDER_FIND_WINNABLE_ATTEMPTS_FOUR_SUIT,
            }
        } else {
            winnability::default_find_winnable_attempts()
        };
        self.begin_winnable_seed_search(
            start_seed,
            attempts,
            self.automation_profile().dialog_find_winnable_state_budget,
            false,
        );
    }

    pub(super) fn begin_winnable_seed_search(
        &self,
        start_seed: u64,
        attempts: u32,
        max_states: usize,
        preserve_robot: bool,
    ) {
        let imp = self.imp();
        if imp.seed_search_in_progress.replace(true) {
            *imp.status_override.borrow_mut() = Some(seed_ops::msg_seed_search_running());
            self.render();
            return;
        }

        let draw_mode = self.current_klondike_draw_mode();
        let deal_count = draw_mode.count();
        let mode = self.active_game_mode();
        let spider_suit_mode = self.current_spider_suit_mode();
        let spider_suit_count = spider_suit_mode.suit_count();
        let spider_step_budget =
            winnability::spider_find_step_budget(spider_suit_mode, max_states, 0);
        let freecell_card_count_mode = self.current_freecell_card_count_mode();
        let freecell_card_count = freecell_card_count_mode.card_count();
        *imp.status_override.borrow_mut() = Some(if mode == GameMode::Freecell {
            format!(
                "Searching FreeCell {freecell_card_count} winnable seed from {start_seed} (attempts: {attempts}, one-pass wand per seed)..."
            )
        } else if mode == GameMode::Spider {
            format!(
                "Searching Spider {spider_suit_count}-suit winnable seed from {start_seed} (attempts: {attempts}, 2-path ensemble per seed, step budget: {spider_step_budget})..."
            )
        } else {
            seed_ops::msg_searching_winnable_seed(start_seed, deal_count, attempts, max_states)
        });
        self.render();

        enum WinnableSeedSearchResult {
            Standard {
                seed: u64,
                tested: u32,
                solver_line: Vec<SolverMove>,
            },
            Freecell {
                seed: u64,
                tested: u32,
                line: Vec<FreecellPlannerAction>,
            },
            Spider {
                seed: u64,
                tested: u32,
                line: Vec<HintMove>,
            },
        }
        let memory_limit_mib = self
            .current_memory_mib()
            .map(|m| m.saturating_add(FIND_WINNABLE_MEMORY_HEADROOM_MIB))
            .unwrap_or(FIND_WINNABLE_MEMORY_MAX_MIB)
            .min(FIND_WINNABLE_MEMORY_MAX_MIB);
        let memory_guard_triggered = Rc::new(Cell::new(false));
        let search_cancel = Arc::new(AtomicBool::new(false));
        let search_cancel_worker = Arc::clone(&search_cancel);
        *imp.seed_search_cancel.borrow_mut() = Some(Arc::clone(&search_cancel));
        let freecell_progress_checked = Arc::new(AtomicU32::new(0));
        let freecell_progress_checked_worker = Arc::clone(&freecell_progress_checked);
        let freecell_progress_stats = Arc::new(winnability::FreecellFindProgress::default());
        let freecell_progress_stats_worker = Arc::clone(&freecell_progress_stats);
        let freecell_last_progress_shown = Rc::new(Cell::new(0_u32));
        let freecell_last_progress_update_us = Rc::new(Cell::new(0_i64));
        let spider_progress_checked = Arc::new(AtomicU32::new(0));
        let spider_progress_checked_worker = Arc::clone(&spider_progress_checked);
        let spider_progress_stats = Arc::new(winnability::SpiderFindProgress::default());
        let spider_progress_stats_worker = Arc::clone(&spider_progress_stats);
        let spider_last_progress_shown = Rc::new(Cell::new(0_u32));
        let spider_last_progress_update_us = Rc::new(Cell::new(0_i64));

        let (sender, receiver) = mpsc::channel::<Option<WinnableSeedSearchResult>>();
        thread::spawn(move || {
            let result =
                if mode == GameMode::Freecell {
                    let profile = AutomationProfile::for_mode(mode);
                    let guided_budget = profile.dialog_seed_guided_budget;
                    let exhaustive_budget = profile.dialog_seed_exhaustive_budget;
                    winnability::find_winnable_freecell_seed_parallel(
                        start_seed,
                        attempts,
                        guided_budget,
                        exhaustive_budget,
                        freecell_card_count_mode,
                        Arc::clone(&search_cancel_worker),
                        Some(Arc::clone(&freecell_progress_checked_worker)),
                        Some(Arc::clone(&freecell_progress_stats_worker)),
                    )
                    .map(|(seed, tested, line)| {
                        WinnableSeedSearchResult::Freecell { seed, tested, line }
                    })
                } else if mode == GameMode::Spider {
                    winnability::find_winnable_spider_seed_parallel(
                        start_seed,
                        attempts,
                        max_states,
                        0,
                        spider_suit_mode,
                        Arc::clone(&search_cancel_worker),
                        Some(Arc::clone(&spider_progress_checked_worker)),
                        Some(Arc::clone(&spider_progress_stats_worker)),
                    )
                    .map(|(seed, tested, line)| {
                        WinnableSeedSearchResult::Spider { seed, tested, line }
                    })
                } else {
                    winnability::find_winnable_seed_parallel(
                        start_seed,
                        attempts,
                        max_states,
                        draw_mode,
                        Arc::clone(&search_cancel_worker),
                    )
                    .map(|(seed, tested, solver_line)| {
                        WinnableSeedSearchResult::Standard {
                            seed,
                            tested,
                            solver_line,
                        }
                    })
                };
            let _ = sender.send(result);
        });

        glib::timeout_add_local(
            Duration::from_millis(40),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                memory_guard_triggered,
                #[strong]
                search_cancel,
                #[strong]
                freecell_last_progress_shown,
                #[strong]
                freecell_last_progress_update_us,
                #[strong]
                freecell_progress_checked,
                #[strong]
                spider_last_progress_shown,
                #[strong]
                spider_last_progress_update_us,
                #[strong]
                spider_progress_checked,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || match receiver.try_recv() {
                    Err(mpsc::TryRecvError::Empty) => {
                        if mode == GameMode::Freecell {
                            let checked = freecell_progress_checked.load(Ordering::Relaxed);
                            let last = freecell_last_progress_shown.get();
                            let now_us = glib::monotonic_time();
                            let last_update_us = freecell_last_progress_update_us.get();
                            let enough_delta = checked.saturating_sub(last) >= 8;
                            let enough_time = now_us.saturating_sub(last_update_us) >= 200_000;
                            if checked > last
                                && (enough_delta || enough_time || checked == attempts)
                            {
                                freecell_last_progress_shown.set(checked);
                                freecell_last_progress_update_us.set(now_us);
                                let current_seed =
                                    start_seed.wrapping_add(u64::from(checked.saturating_sub(1)));
                                let expanded = freecell_progress_stats
                                    .last_expanded_states
                                    .load(Ordering::Relaxed);
                                let branches = freecell_progress_stats
                                    .last_generated_branches
                                    .load(Ordering::Relaxed);
                                let elapsed_ms = freecell_progress_stats
                                    .last_elapsed_ms
                                    .load(Ordering::Relaxed);
                                let stop_reason = winnability::freecell_find_stop_reason_label(
                                    freecell_progress_stats
                                        .last_stop_reason
                                        .load(Ordering::Relaxed),
                                );
                                window.set_seed_input_text(&current_seed.to_string());
                                *window.imp().status_override.borrow_mut() = Some(format!(
                                    "Searching FreeCell {freecell_card_count} winnable seed from {start_seed} (attempts: {attempts}, one-pass wand per seed)... checked {checked}/{attempts}, current seed {current_seed}, expanded={expanded}, branches={branches}, elapsed_ms={elapsed_ms}, stop={stop_reason}"
                                ));
                                window.render();
                            }
                        } else if mode == GameMode::Spider {
                            let checked = spider_progress_checked.load(Ordering::Relaxed);
                            let last = spider_last_progress_shown.get();
                            let now_us = glib::monotonic_time();
                            let last_update_us = spider_last_progress_update_us.get();
                            let enough_delta = checked.saturating_sub(last) >= 8;
                            let enough_time = now_us.saturating_sub(last_update_us) >= 200_000;
                            if checked > last
                                && (enough_delta || enough_time || checked == attempts)
                            {
                                spider_last_progress_shown.set(checked);
                                spider_last_progress_update_us.set(now_us);
                                let current_seed =
                                    start_seed.wrapping_add(u64::from(checked.saturating_sub(1)));
                                let expanded = spider_progress_stats
                                    .last_expanded_states
                                    .load(Ordering::Relaxed);
                                let branches = spider_progress_stats
                                    .last_generated_branches
                                    .load(Ordering::Relaxed);
                                let completed_runs = spider_progress_stats
                                    .last_completed_runs
                                    .load(Ordering::Relaxed);
                                let stock_cards = spider_progress_stats
                                    .last_stock_cards
                                    .load(Ordering::Relaxed);
                                let deals_left = stock_cards / 10;
                                let face_down = spider_progress_stats
                                    .last_face_down_cards
                                    .load(Ordering::Relaxed);
                                let empty_cols = spider_progress_stats
                                    .last_empty_cols
                                    .load(Ordering::Relaxed);
                                let suited_edges = spider_progress_stats
                                    .last_suited_edges
                                    .load(Ordering::Relaxed);
                                let max_tail_run = spider_progress_stats
                                    .last_max_tail_run
                                    .load(Ordering::Relaxed);
                                let draw_moves = spider_progress_stats
                                    .last_draw_moves
                                    .load(Ordering::Relaxed);
                                let tableau_moves = spider_progress_stats
                                    .last_tableau_moves
                                    .load(Ordering::Relaxed);
                                let reveal_moves = spider_progress_stats
                                    .last_reveal_moves
                                    .load(Ordering::Relaxed);
                                let peak_suited_edges = spider_progress_stats
                                    .last_peak_suited_edges
                                    .load(Ordering::Relaxed);
                                let peak_tail_run = spider_progress_stats
                                    .last_peak_tail_run
                                    .load(Ordering::Relaxed);
                                let peak_empty_cols = spider_progress_stats
                                    .last_peak_empty_cols
                                    .load(Ordering::Relaxed);
                                let empty_creates = spider_progress_stats
                                    .last_empty_creates
                                    .load(Ordering::Relaxed);
                                let adapt_events = spider_progress_stats
                                    .last_adapt_events
                                    .load(Ordering::Relaxed);
                                let best_peak_empty = spider_progress_stats
                                    .best_peak_empty_cols
                                    .load(Ordering::Relaxed);
                                let attempts_with_empty_create = spider_progress_stats
                                    .attempts_with_empty_create
                                    .load(Ordering::Relaxed);
                                let cycle_period = spider_progress_stats
                                    .last_cycle_period
                                    .load(Ordering::Relaxed);
                                let cycle_blocks = spider_progress_stats
                                    .last_cycle_blocks
                                    .load(Ordering::Relaxed);
                                let stop_reason = winnability::spider_find_stop_reason_label(
                                    spider_progress_stats
                                        .last_stop_reason
                                        .load(Ordering::Relaxed),
                                );
                                let cycle_suffix = if cycle_period > 0 {
                                    format!(", cycle={}x{}", cycle_period, cycle_blocks)
                                } else {
                                    String::new()
                                };
                                window.set_seed_input_text(&current_seed.to_string());
                                *window.imp().status_override.borrow_mut() = Some(format!(
                                    "Searching Spider {spider_suit_count}-suit winnable seed from {start_seed} (attempts: {attempts}, 2-path ensemble per seed, step budget: {spider_step_budget})... checked {checked}/{attempts}, current seed {current_seed}, expanded={expanded}, branches={branches}, runs={completed_runs}, stock={stock_cards}({deals_left} deals), facedown={face_down}, empty={empty_cols}, suited_edges={suited_edges}, tail_run={max_tail_run}, draws={draw_moves}, t_moves={tableau_moves}, reveals={reveal_moves}, peak_edges={peak_suited_edges}, peak_tail={peak_tail_run}, peak_empty={peak_empty_cols}, empty_creates={empty_creates}, best_peak_empty={best_peak_empty}, attempts_with_empty={attempts_with_empty_create}, adapt={adapt_events}, stop={stop_reason}{cycle_suffix}"
                                ));
                                window.render();
                            }
                        }
                        if window
                            .current_memory_mib()
                            .is_some_and(|mib| mib > memory_limit_mib)
                        {
                            memory_guard_triggered.set(true);
                            search_cancel.store(true, Ordering::Relaxed);
                        }
                        glib::ControlFlow::Continue
                    }
                    Ok(Some(WinnableSeedSearchResult::Standard {
                        seed,
                        tested,
                        solver_line,
                    })) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        imp.seed_search_cancel.borrow_mut().take();
                        let status = seed_ops::msg_started_winnable_seed(seed, deal_count, tested);
                        if preserve_robot {
                            window.start_new_game_with_seed_internal(seed, status, true);
                        } else {
                            window.start_new_game_with_seed(seed, status);
                        }
                        if let Some(line) = map_solver_line_to_hint_line(
                            &window.imp().game.borrow().clone(),
                            solver_line.as_slice(),
                        ) {
                            window.arm_robot_solver_anchor_for_current_state(line);
                        }
                        window.trim_process_memory_if_supported();
                        glib::ControlFlow::Break
                    }
                    Ok(Some(WinnableSeedSearchResult::Freecell { seed, tested, line })) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        imp.seed_search_cancel.borrow_mut().take();
                        let status = format!(
                            "Started FreeCell {freecell_card_count} winnable game. Seed {seed} (checked {tested} seed(s)). Use Robot as first action to see win."
                        );
                        if preserve_robot {
                            window.start_new_game_with_seed_internal(seed, status, true);
                        } else {
                            window.start_new_game_with_seed(seed, status);
                        }
                        window.arm_robot_freecell_solver_anchor_for_current_state(line);
                        window.trim_process_memory_if_supported();
                        glib::ControlFlow::Break
                    }
                    Ok(Some(WinnableSeedSearchResult::Spider { seed, tested, line })) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        imp.seed_search_cancel.borrow_mut().take();
                        let spider_suit_count = window.current_spider_suit_mode().suit_count();
                        let status = format!(
                            "Started Spider {spider_suit_count}-suit winnable game. Seed {seed} (checked {tested} seed(s)). Use Robot as first action to see win."
                        );
                        if preserve_robot {
                            window.start_new_game_with_seed_internal(seed, status, true);
                        } else {
                            window.start_new_game_with_seed(seed, status);
                        }
                        window.arm_robot_solver_anchor_for_current_state(line);
                        window.trim_process_memory_if_supported();
                        glib::ControlFlow::Break
                    }
                    Ok(None) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        imp.seed_search_cancel.borrow_mut().take();
                        let search_canceled = search_cancel.load(Ordering::Relaxed);
                        *imp.status_override.borrow_mut() = Some(if memory_guard_triggered.get() {
                            if mode == GameMode::Freecell {
                                format!(
                                    "Find winnable stopped by memory guard at ~{} MiB (FreeCell {freecell_card_count}).",
                                    memory_limit_mib
                                )
                            } else if mode == GameMode::Spider {
                                let spider_suit_count =
                                    window.current_spider_suit_mode().suit_count();
                                format!(
                                    "Find winnable stopped by memory guard at ~{} MiB (Spider {spider_suit_count}-suit).",
                                    memory_limit_mib
                                )
                            } else {
                                format!(
                                    "Find winnable stopped by memory guard at ~{} MiB (Deal {deal_count}).",
                                    memory_limit_mib
                                )
                            }
                        } else if search_canceled {
                            if mode == GameMode::Freecell {
                                let checked = freecell_progress_checked.load(Ordering::Relaxed);
                                format!(
                                    "Canceled FreeCell {freecell_card_count} winnable-seed search after checking {checked} seed(s) from seed {start_seed}."
                                )
                            } else if mode == GameMode::Spider {
                                let checked = spider_progress_checked.load(Ordering::Relaxed);
                                let spider_suit_count =
                                    window.current_spider_suit_mode().suit_count();
                                format!(
                                    "Canceled Spider {spider_suit_count}-suit winnable-seed search after checking {checked} seed(s) from seed {start_seed}."
                                )
                            } else {
                                format!("Canceled winnable-seed search from seed {start_seed}.")
                            }
                        } else if mode == GameMode::Freecell {
                            format!(
                                "No FreeCell {freecell_card_count} winnable seed found in {attempts} attempt(s) from seed {start_seed}."
                            )
                        } else if mode == GameMode::Spider {
                            let spider_suit_count = window.current_spider_suit_mode().suit_count();
                            format!(
                                "No Spider {spider_suit_count}-suit winnable seed found in {attempts} attempt(s) from seed {start_seed}."
                            )
                        } else {
                            seed_ops::msg_no_winnable_seed(start_seed, deal_count, attempts)
                        });
                        window.render();
                        window.trim_process_memory_if_supported();
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Disconnected) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        imp.seed_search_cancel.borrow_mut().take();
                        let search_canceled = search_cancel.load(Ordering::Relaxed);
                        *imp.status_override.borrow_mut() = Some(if memory_guard_triggered.get() {
                            if mode == GameMode::Freecell {
                                format!(
                                    "Find winnable stopped by memory guard at ~{} MiB (FreeCell {freecell_card_count}).",
                                    memory_limit_mib
                                )
                            } else if mode == GameMode::Spider {
                                let spider_suit_count =
                                    window.current_spider_suit_mode().suit_count();
                                format!(
                                    "Find winnable stopped by memory guard at ~{} MiB (Spider {spider_suit_count}-suit).",
                                    memory_limit_mib
                                )
                            } else {
                                format!(
                                    "Find winnable stopped by memory guard at ~{} MiB (Deal {deal_count}).",
                                    memory_limit_mib
                                )
                            }
                        } else if search_canceled {
                            if mode == GameMode::Freecell {
                                let checked = freecell_progress_checked.load(Ordering::Relaxed);
                                format!(
                                    "Canceled FreeCell {freecell_card_count} winnable-seed search after checking {checked} seed(s) from seed {start_seed}."
                                )
                            } else if mode == GameMode::Spider {
                                let checked = spider_progress_checked.load(Ordering::Relaxed);
                                let spider_suit_count =
                                    window.current_spider_suit_mode().suit_count();
                                format!(
                                    "Canceled Spider {spider_suit_count}-suit winnable-seed search after checking {checked} seed(s) from seed {start_seed}."
                                )
                            } else {
                                format!("Canceled winnable-seed search from seed {start_seed}.")
                            }
                        } else if mode == GameMode::Freecell {
                            format!(
                                "FreeCell {freecell_card_count} seed search stopped unexpectedly."
                            )
                        } else if mode == GameMode::Spider {
                            let spider_suit_count = window.current_spider_suit_mode().suit_count();
                            format!(
                                "Spider {spider_suit_count}-suit seed search stopped unexpectedly."
                            )
                        } else {
                            seed_ops::msg_seed_search_stopped_unexpectedly(deal_count)
                        });
                        window.render();
                        window.trim_process_memory_if_supported();
                        glib::ControlFlow::Break
                    }
                }
            ),
        );
    }
}

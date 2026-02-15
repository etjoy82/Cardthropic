use super::*;
use crate::engine::boundary;
use crate::engine::seed_ops;

impl CardthropicWindow {
    fn robot_total_runs(&self) -> u32 {
        self.imp()
            .robot_wins
            .get()
            .saturating_add(self.imp().robot_losses.get())
    }

    fn robot_benchmark_summary_line(&self, trigger: &str) -> String {
        let wins = self.imp().robot_wins.get();
        let losses = self.imp().robot_losses.get();
        let total = self.robot_total_runs();
        let win_rate_pct = if total == 0 {
            0.0
        } else {
            (f64::from(wins) / f64::from(total)) * 100.0
        };
        let memory = self.current_memory_mib_text().replace(' ', "");
        format!(
            "bench_v=1 trigger={} runs={} wins={} losses={} win_rate_pct={:.1} strategy={} mode={} draw={} forever={} robot_moves={} deals={} elapsed_s={} mem={}",
            trigger,
            total,
            wins,
            losses,
            win_rate_pct,
            self.robot_strategy().as_setting(),
            self.active_game_mode().id(),
            self.current_klondike_draw_mode().count(),
            self.imp().robot_forever_enabled.get(),
            self.imp().robot_moves_applied.get(),
            self.imp().robot_deals_tried.get(),
            self.imp().elapsed_seconds.get(),
            memory
        )
    }

    fn maybe_emit_periodic_benchmark_dump(&self, trigger: &str) {
        let total = self.robot_total_runs();
        if total == 0 || total % ROBOT_BENCHMARK_DUMP_INTERVAL != 0 {
            return;
        }
        if self.imp().robot_last_benchmark_dump_total.get() == total {
            return;
        }
        self.imp().robot_last_benchmark_dump_total.set(total);
        *self.imp().status_override.borrow_mut() = Some(self.robot_benchmark_summary_line(trigger));
        self.render();
    }

    pub(super) fn copy_benchmark_snapshot(&self) {
        let line = self.robot_benchmark_summary_line("manual_copy");
        self.clipboard().set_text(&line);
        *self.imp().status_override.borrow_mut() = Some(line);
        self.render();
    }

    fn robot_outcome_fields(&self) -> String {
        let wins = self.imp().robot_wins.get();
        let losses = self.imp().robot_losses.get();
        let total = wins.saturating_add(losses);
        let win_rate = if total == 0 {
            0.0
        } else {
            (f64::from(wins) / f64::from(total)) * 100.0
        };
        format!(" wins={} losses={} win_rate_pct={:.1}", wins, losses, win_rate)
    }

    fn handle_robot_win(&self) -> bool {
        let wins = self.imp().robot_wins.get().saturating_add(1);
        self.imp().robot_wins.set(wins);
        self.maybe_emit_periodic_benchmark_dump("win");
        if !self.imp().robot_forever_enabled.get() {
            self.stop_robot_mode_with_message("Robot Mode stopped: game won.");
            return true;
        }

        self.imp()
            .robot_playback
            .borrow_mut()
            .set_use_scripted_line(false);
        let seed = seed_ops::random_seed();
        self.start_new_game_with_seed_internal(
            seed,
            format!(
                "robot_v=1{} strategy={} state=running event=reseed mode={} seed={} draw={} app_moves={} robot_moves={} deals_tried={} elapsed={} timer={} solver_source=search move_changed=false detail=\"won_reseed\" reason=\"forever mode\"{}{} move_kind=na src_col=na src_start=na dst_col=na cards_moved_total=na draw_from_stock_cards=na recycle_cards=na{}",
                self.robot_outcome_fields(),
                self.robot_strategy().as_setting(),
                self.active_game_mode().id(),
                seed,
                self.current_klondike_draw_mode().count(),
                self.imp().move_count.get(),
                self.imp().robot_moves_applied.get(),
                self.imp().robot_deals_tried.get(),
                self.imp().elapsed_seconds.get(),
                if self.imp().timer_started.get() { 1 } else { 0 },
                self.robot_progress_fields(),
                self.robot_board_fields(),
                self.robot_debug_tail()
            ),
            true,
        );
        true
    }

    pub(super) fn start_robot_mode_forever(&self) {
        self.set_robot_forever_enabled(true, true);
        if !self.imp().robot_mode_running.get() {
            self.toggle_robot_mode();
        }
    }

    fn robot_quote(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    fn robot_debug_tail(&self) -> String {
        if !self.imp().robot_debug_enabled.get() {
            return String::new();
        }

        let state_hash = self.current_game_hash();
        let playback = self.imp().robot_playback.borrow();
        let scripted_enabled = playback.use_scripted_line();
        let scripted_remaining = playback.scripted_line_len();
        let scripted_ready = playback.has_scripted_line();
        format!(
            " state_hash={} scripted_enabled={} scripted_ready={} scripted_remaining={}",
            state_hash,
            scripted_enabled,
            scripted_ready,
            scripted_remaining
        )
    }

    fn robot_progress_fields(&self) -> String {
        let imp = self.imp();
        match self.active_game_mode() {
            GameMode::Spider => {
                let runs = imp.game.borrow().spider().completed_runs();
                format!(
                    " progress_kind=completed_runs progress_value={} progress_score={}",
                    runs,
                    runs * 100
                )
            }
            GameMode::Klondike => {
                let foundation_cards: usize = imp
                    .game
                    .borrow()
                    .klondike()
                    .foundations()
                    .iter()
                    .map(Vec::len)
                    .sum();
                format!(
                    " progress_kind=foundation_cards progress_value={} progress_score={}",
                    foundation_cards,
                    foundation_cards * 10
                )
            }
            GameMode::Freecell => " progress_kind=na progress_value=na progress_score=na".to_string(),
        }
    }

    fn robot_board_fields(&self) -> String {
        let imp = self.imp();
        match self.active_game_mode() {
            GameMode::Spider => {
                let game = imp.game.borrow();
                let spider = game.spider();
                let stock_cards = spider.stock_len();
                let completed_runs = spider.completed_runs();
                let tableau_empty_cols = spider.tableau().iter().filter(|col| col.is_empty()).count();
                let tableau_nonempty_cols = spider.tableau().len().saturating_sub(tableau_empty_cols);
                let tableau_face_up_cards = spider
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| card.face_up)
                    .count();
                let tableau_face_down_cards = spider
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| !card.face_up)
                    .count();
                format!(
                    " stock_cards={} waste_cards=na foundation_cards=na completed_runs={} tableau_empty_cols={} tableau_nonempty_cols={} tableau_face_up_cards={} tableau_face_down_cards={}",
                    stock_cards,
                    completed_runs,
                    tableau_empty_cols,
                    tableau_nonempty_cols,
                    tableau_face_up_cards,
                    tableau_face_down_cards
                )
            }
            GameMode::Klondike => {
                let game = imp.game.borrow();
                let klondike = game.klondike();
                let stock_cards = klondike.stock_len();
                let waste_cards = klondike.waste_len();
                let foundation_cards: usize =
                    klondike.foundations().iter().map(Vec::len).sum();
                let tableau_empty_cols = klondike.tableau().iter().filter(|col| col.is_empty()).count();
                let tableau_nonempty_cols =
                    klondike.tableau().len().saturating_sub(tableau_empty_cols);
                let tableau_face_up_cards = klondike
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| card.face_up)
                    .count();
                let tableau_face_down_cards = klondike
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| !card.face_up)
                    .count();
                format!(
                    " stock_cards={} waste_cards={} foundation_cards={} completed_runs=na tableau_empty_cols={} tableau_nonempty_cols={} tableau_face_up_cards={} tableau_face_down_cards={}",
                    stock_cards,
                    waste_cards,
                    foundation_cards,
                    tableau_empty_cols,
                    tableau_nonempty_cols,
                    tableau_face_up_cards,
                    tableau_face_down_cards
                )
            }
            GameMode::Freecell => {
                " stock_cards=na waste_cards=na foundation_cards=na completed_runs=na tableau_empty_cols=na tableau_nonempty_cols=na tableau_face_up_cards=na tableau_face_down_cards=na".to_string()
            }
        }
    }

    fn robot_move_description(hint_move: HintMove) -> &'static str {
        match hint_move {
            HintMove::WasteToFoundation => "waste -> foundation",
            HintMove::TableauTopToFoundation { .. } => "tableau top -> foundation",
            HintMove::WasteToTableau { .. } => "waste -> tableau",
            HintMove::TableauRunToTableau { .. } => "tableau run -> tableau",
            HintMove::Draw => "draw from stock",
        }
    }

    fn robot_move_fields(&self, hint_move: Option<HintMove>) -> String {
        let Some(hint_move) = hint_move else {
            return " move_kind=na src_col=na src_start=na dst_col=na cards_moved_total=na draw_from_stock_cards=na recycle_cards=na".to_string();
        };

        match hint_move {
            HintMove::WasteToFoundation => {
                " move_kind=waste_to_foundation src_col=na src_start=na dst_col=foundation cards_moved_total=1 draw_from_stock_cards=na recycle_cards=na".to_string()
            }
            HintMove::TableauTopToFoundation { src } => {
                format!(
                    " move_kind=tableau_top_to_foundation src_col={} src_start=top dst_col=foundation cards_moved_total=1 draw_from_stock_cards=na recycle_cards=na",
                    src
                )
            }
            HintMove::WasteToTableau { dst } => {
                format!(
                    " move_kind=waste_to_tableau src_col=waste src_start=top dst_col={} cards_moved_total=1 draw_from_stock_cards=na recycle_cards=na",
                    dst
                )
            }
            HintMove::TableauRunToTableau { src, start, dst } => {
                let cards = match self.active_game_mode() {
                    GameMode::Spider => self
                        .imp()
                        .game
                        .borrow()
                        .spider()
                        .tableau()
                        .get(src)
                        .map(Vec::len)
                        .unwrap_or(0)
                        .saturating_sub(start),
                    GameMode::Klondike => self
                        .imp()
                        .game
                        .borrow()
                        .klondike()
                        .tableau_len(src)
                        .unwrap_or(0)
                        .saturating_sub(start),
                    GameMode::Freecell => 0,
                };
                format!(
                    " move_kind=tableau_run_to_tableau src_col={} src_start={} dst_col={} cards_moved_total={} draw_from_stock_cards=na recycle_cards=na",
                    src, start, dst, cards
                )
            }
            HintMove::Draw => {
                match self.active_game_mode() {
                    GameMode::Spider => {
                        let draw_from_stock_cards = self.imp().game.borrow().spider().stock_len().min(10);
                        format!(
                            " move_kind=draw src_col=stock src_start=top dst_col=tableau cards_moved_total={} draw_from_stock_cards={} recycle_cards=0",
                            draw_from_stock_cards,
                            draw_from_stock_cards
                        )
                    }
                    GameMode::Klondike => {
                        let game = self.imp().game.borrow();
                        let klondike = game.klondike();
                        let stock_before = klondike.stock_len();
                        let waste_before = klondike.waste_len();
                        let draw_n = self.current_klondike_draw_mode().count() as usize;
                        let draw_from_stock_cards = if stock_before > 0 {
                            stock_before.min(draw_n)
                        } else {
                            0
                        };
                        let recycle_cards = if stock_before == 0 { waste_before } else { 0 };
                        let cards_moved_total = draw_from_stock_cards + recycle_cards;
                        format!(
                            " move_kind=draw src_col=stock src_start=top dst_col=waste cards_moved_total={} draw_from_stock_cards={} recycle_cards={}",
                            cards_moved_total,
                            draw_from_stock_cards,
                            recycle_cards
                        )
                    }
                    GameMode::Freecell => {
                        " move_kind=draw src_col=na src_start=na dst_col=na cards_moved_total=0 draw_from_stock_cards=0 recycle_cards=0".to_string()
                    }
                }
            }
        }
    }

    fn emit_robot_status(
        &self,
        state: &str,
        event: &str,
        detail: &str,
        reason: Option<&str>,
        move_fields_override: Option<&str>,
        move_changed: Option<bool>,
        solver_source: &str,
    ) {
        let deals_tried = self.imp().robot_deals_tried.get();
        let moves_applied = self.imp().robot_moves_applied.get();
        let outcome = self.robot_outcome_fields();
        let app_moves = self.imp().move_count.get();
        let elapsed = self.imp().elapsed_seconds.get();
        let timer = if self.imp().timer_started.get() { 1 } else { 0 };
        let mode = self.active_game_mode();
        let draw = self.current_klondike_draw_mode().count();
        let seed = self.imp().current_seed.get();
        let reason_field = reason
            .map(|value| format!(" reason=\"{}\"", Self::robot_quote(value)))
            .unwrap_or_default();
        let strategy = self.robot_strategy().as_setting();
        let move_changed_field = move_changed
            .map(|value| value.to_string())
            .unwrap_or_else(|| "na".to_string());
        let move_fields = move_fields_override
            .map(str::to_string)
            .unwrap_or_else(|| self.robot_move_fields(None));
        let status = if self.imp().robot_debug_enabled.get() {
            format!(
                "robot_v=1{} strategy={} state={} event={} mode={} seed={} draw={} app_moves={} robot_moves={} deals_tried={} elapsed={} timer={} solver_source={} move_changed={} detail=\"{}\"{}{}{}{}{}",
                outcome,
                strategy,
                state,
                event,
                mode.id(),
                seed,
                draw,
                app_moves,
                moves_applied,
                deals_tried,
                elapsed,
                timer,
                solver_source,
                move_changed_field,
                Self::robot_quote(detail),
                reason_field,
                self.robot_progress_fields(),
                self.robot_board_fields(),
                move_fields,
                self.robot_debug_tail()
            )
        } else {
            let progress = self.robot_progress_fields();
            format!(
                "robot{} strategy={} state={} event={} detail=\"{}\" moves={} deals={}{}",
                outcome,
                strategy,
                state,
                event,
                Self::robot_quote(detail),
                moves_applied,
                deals_tried,
                progress
            )
        };
        *self.imp().status_override.borrow_mut() = Some(status);
    }

    fn set_robot_status_running(&self, detail: &str, solver_source: &str) {
        self.emit_robot_status("running", "status", detail, None, None, None, solver_source);
    }

    fn record_robot_move_and_maybe_pulse(
        &self,
        move_fields: &str,
        detail: &str,
        solver_source: &str,
    ) {
        let next_moves = self.imp().robot_moves_applied.get().saturating_add(1);
        self.imp().robot_moves_applied.set(next_moves);
        self.emit_robot_status(
            "running",
            "move_applied",
            &format!("move {next_moves}: {detail}"),
            None,
            Some(move_fields),
            Some(true),
            solver_source,
        );
    }

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
        self.cancel_hint_loss_analysis();
        self.imp().robot_mode_running.set(true);
        self.imp().robot_deals_tried.set(0);
        self.imp().robot_moves_applied.set(0);
        self.imp().robot_wins.set(0);
        self.imp().robot_losses.set(0);
        self.imp().robot_last_benchmark_dump_total.set(0);
        let using_solver = self.imp().robot_playback.borrow().use_scripted_line();
        self.set_robot_status_running(if using_solver {
            "solver armed"
        } else {
            "searching"
        }, if using_solver { "scripted" } else { "search" });
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
            let _ = self.handle_robot_win();
            return;
        }

        let moved = if self.imp().robot_playback.borrow().use_scripted_line() {
            let scripted_move = {
                let mut playback = self.imp().robot_playback.borrow_mut();
                playback.pop_scripted_move()
            };
            if let Some(hint_move) = scripted_move {
                let desc = Self::robot_move_description(hint_move);
                let move_fields = self.robot_move_fields(Some(hint_move));
                self.imp().auto_playing_move.set(true);
                let changed = self.apply_hint_move(hint_move);
                self.imp().auto_playing_move.set(false);
                if changed {
                    *self.imp().selected_run.borrow_mut() = None;
                    self.record_robot_move_and_maybe_pulse(
                        &move_fields,
                        &format!("solver {desc}"),
                        "scripted",
                    );
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
            let suggestion = self.compute_auto_play_suggestion();
            match suggestion.hint_move {
                Some(hint_move) => {
                    let desc = suggestion
                        .message
                        .strip_prefix("Hint: ")
                        .unwrap_or(suggestion.message.as_str())
                        .to_string();
                    let move_fields = self.robot_move_fields(Some(hint_move));
                    self.imp().auto_playing_move.set(true);
                    let changed = self.apply_hint_move(hint_move);
                    self.imp().auto_playing_move.set(false);
                    if changed {
                        *self.imp().selected_run.borrow_mut() = None;
                        self.record_robot_move_and_maybe_pulse(&move_fields, &desc, "search");
                        self.render();
                        true
                    } else {
                        self.emit_robot_status(
                            "running",
                            "move_invalid",
                            "move invalid; recalculating",
                            Some("apply_hint_move returned false"),
                            Some(&move_fields),
                            Some(false),
                            "search",
                        );
                        self.render();
                        false
                    }
                }
                None => {
                    self.emit_robot_status(
                        "running",
                        "no_move",
                        &suggestion.message,
                        Some("no candidate move from suggestion engine"),
                        None,
                        Some(false),
                        "search",
                    );
                    self.render();
                    false
                }
            }
        };
        let now_won = boundary::is_won(&self.imp().game.borrow(), mode);
        if now_won {
            let _ = self.handle_robot_win();
            return;
        }

        if !moved {
            let losses = self.imp().robot_losses.get().saturating_add(1);
            self.imp().robot_losses.set(losses);
            self.maybe_emit_periodic_benchmark_dump("loss");
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
                    "robot_v=1{} strategy={} state=running event=reseed mode={} seed={} draw={} app_moves={} robot_moves={} deals_tried={} elapsed={} timer={} solver_source=search move_changed=false detail=\"stuck_or_lost\" reason=\"no productive move\"{}{} move_kind=na src_col=na src_start=na dst_col=na cards_moved_total=na draw_from_stock_cards=na recycle_cards=na{}",
                    self.robot_outcome_fields(),
                    self.robot_strategy().as_setting(),
                    self.active_game_mode().id(),
                    seed,
                    self.current_klondike_draw_mode().count(),
                    self.imp().move_count.get(),
                    self.imp().robot_moves_applied.get(),
                    next_deals_tried,
                    self.imp().elapsed_seconds.get(),
                    if self.imp().timer_started.get() { 1 } else { 0 },
                    self.robot_progress_fields(),
                    self.robot_board_fields(),
                    self.robot_debug_tail()
                ),
                true,
            );
        }
    }

    pub(super) fn stop_robot_mode(&self) {
        if !self.imp().robot_mode_running.replace(false) {
            return;
        }
        self.cancel_hint_loss_analysis();
        self.imp().robot_playback.borrow_mut().clear_scripted_line();
        if let Some(source_id) = self.imp().robot_mode_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        self.trim_process_memory_if_supported();
        self.emit_robot_status(
            "stopped",
            "stop",
            "robot stop requested",
            None,
            None,
            None,
            "na",
        );
        self.render();
    }

    pub(super) fn stop_robot_mode_with_message(&self, message: &str) {
        if !self.imp().robot_mode_running.replace(false) {
            return;
        }
        self.cancel_hint_loss_analysis();
        self.imp().robot_playback.borrow_mut().clear_scripted_line();
        if let Some(source_id) = self.imp().robot_mode_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        self.trim_process_memory_if_supported();
        self.emit_robot_status("stopped", "stop", message, Some(message), None, None, "na");
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

use super::*;
use crate::engine::boundary;
use crate::engine::render_plan;
use crate::engine::status_text;
use crate::engine::variant_engine::engine_for_mode;
use crate::game::{FreecellGame, SpiderGame};
use std::time::Instant;

impl CardthropicWindow {
    fn parse_robot_moves_from_status(status: &str) -> Option<u32> {
        let marker = " robot_moves=";
        let start = status.find(marker)? + marker.len();
        let bytes = status.as_bytes();
        let mut end = start;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
        if end == start {
            return None;
        }
        status[start..end].parse::<u32>().ok()
    }

    fn should_throttle_robot_history_line(&self, status: &str) -> bool {
        let imp = self.imp();
        if !imp.robot_debug_enabled.get() || !imp.robot_ludicrous_enabled.get() {
            return false;
        }
        if !status.starts_with("robot_v=") {
            return false;
        }

        let sample_mod = if status.contains(" event=planner_wait ") {
            16_u32
        } else if status.contains(" event=planner_ready ") {
            8_u32
        } else if status.contains(" event=move_applied ") {
            4_u32
        } else {
            0_u32
        };

        if sample_mod == 0 {
            return false;
        }

        if let Some(robot_moves) = Self::parse_robot_moves_from_status(status) {
            return robot_moves % sample_mod != 0;
        }

        false
    }

    fn sanitize_status_for_display(&self, status: &str) -> String {
        if self.imp().robot_debug_enabled.get() {
            return status.to_string();
        }

        let has_advanced_robot_telemetry = [
            "robot_v=",
            "bench_v=",
            " strategy=",
            " state=",
            " event=",
            " detail=\"",
            " move_kind=",
            "state_hash=",
            "progress_kind=",
            "stock_cards=",
            "tableau_empty_cols=",
            "app_moves=",
            "solver_source=",
            "draw_from_stock_cards=",
            "recycle_cards=",
            "win_rate_pct=",
        ]
        .iter()
        .any(|marker| status.contains(marker));

        if has_advanced_robot_telemetry {
            "Robot telemetry hidden. Enable Robot Debug to view detailed diagnostics.".to_string()
        } else {
            status.to_string()
        }
    }

    fn set_tableau_frame_inner_compactness(&self, mobile: bool) {
        let imp = self.imp();
        let Some(child) = imp.tableau_frame.child() else {
            return;
        };
        let Ok(inner) = child.downcast::<gtk::Box>() else {
            return;
        };
        if mobile {
            inner.set_spacing(2);
            inner.set_margin_top(1);
            inner.set_margin_bottom(1);
            inner.set_margin_start(1);
            inner.set_margin_end(1);
            imp.tableau_scroller.set_min_content_height(24);
        } else {
            // Keep desktop behavior, but carry over mobile's compact padding.
            inner.set_spacing(2);
            inner.set_margin_top(1);
            inner.set_margin_bottom(1);
            inner.set_margin_start(1);
            inner.set_margin_end(1);
            imp.tableau_scroller.set_min_content_height(60);
        }
    }

    pub(super) fn append_status_history_only(&self, status: &str) {
        const MAX_STATUS_LINES: usize = 240;

        if self.should_throttle_robot_history_line(status) {
            return;
        }

        let imp = self.imp();
        let mut history = imp.status_history.borrow_mut();
        history.push_back(status.to_string());
        while history.len() > MAX_STATUS_LINES {
            let _ = history.pop_front();
        }
        let history_joined = if imp.status_history_buffer.borrow().is_some() {
            Some(
                history
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        } else {
            None
        };
        drop(history);
        if let Some(joined) = history_joined {
            if let Some(buffer) = imp.status_history_buffer.borrow().as_ref() {
                buffer.set_text(&joined);
            }
        }
    }

    pub(super) fn append_layout_debug_history_line(&self, status: &str) {
        let imp = self.imp();
        if !imp.robot_debug_enabled.get() {
            return;
        }
        if imp.layout_debug_last_appended.borrow().as_str() == status {
            return;
        }
        *imp.layout_debug_last_appended.borrow_mut() = status.to_string();

        let timestamp = glib::DateTime::now_local()
            .ok()
            .and_then(|now| now.format("%H:%M:%S").ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "--:--:--".to_string());
        self.append_status_history_only(&format!("layout_debug t={} {}", timestamp, status));
    }

    fn maybe_append_resize_perf_line(&self) {
        let imp = self.imp();
        if !imp.robot_debug_enabled.get() {
            return;
        }
        let now_us = glib::monotonic_time();
        let last_us = imp.perf_last_report_mono_us.get();
        if last_us != 0 && now_us.saturating_sub(last_us) < 1_000_000 {
            return;
        }
        imp.perf_last_report_mono_us.set(now_us);

        let resize_events = imp.perf_resize_event_count.get();
        let geometry_renders = imp.perf_geometry_render_count.get();
        let render_count = imp.perf_render_count.get();
        let render_total_us = imp.perf_render_total_us.get();

        let delta_resize_events =
            resize_events.saturating_sub(imp.perf_last_report_resize_events.get());
        let resize_from_poll = imp
            .perf_resize_from_poll_count
            .get()
            .saturating_sub(imp.perf_last_report_resize_from_poll.get());
        let resize_from_notify_width = imp
            .perf_resize_from_notify_width_count
            .get()
            .saturating_sub(imp.perf_last_report_resize_from_notify_width.get());
        let resize_from_notify_height = imp
            .perf_resize_from_notify_height_count
            .get()
            .saturating_sub(imp.perf_last_report_resize_from_notify_height.get());
        let resize_from_notify_max = imp
            .perf_resize_from_notify_maximized_count
            .get()
            .saturating_sub(imp.perf_last_report_resize_from_notify_maximized.get());
        let delta_geometry_renders =
            geometry_renders.saturating_sub(imp.perf_last_report_geometry_renders.get());
        let delta_render_count =
            render_count.saturating_sub(imp.perf_last_report_render_count.get());
        let delta_render_total_us =
            render_total_us.saturating_sub(imp.perf_last_report_render_total_us.get());

        imp.perf_last_report_resize_events.set(resize_events);
        imp.perf_last_report_resize_from_poll
            .set(imp.perf_resize_from_poll_count.get());
        imp.perf_last_report_resize_from_notify_width
            .set(imp.perf_resize_from_notify_width_count.get());
        imp.perf_last_report_resize_from_notify_height
            .set(imp.perf_resize_from_notify_height_count.get());
        imp.perf_last_report_resize_from_notify_maximized
            .set(imp.perf_resize_from_notify_maximized_count.get());
        imp.perf_last_report_geometry_renders.set(geometry_renders);
        imp.perf_last_report_render_count.set(render_count);
        imp.perf_last_report_render_total_us.set(render_total_us);

        let (deck_hits, deck_misses, deck_inserts, deck_clears, deck_cache_len) =
            if let Some(deck) = imp.deck.borrow().as_ref() {
                let stats = deck.scaled_cache_stats();
                (
                    stats.hits,
                    stats.misses,
                    stats.inserts,
                    stats.clears,
                    deck.scaled_cache_len() as u64,
                )
            } else {
                (0, 0, 0, 0, 0)
            };

        let delta_deck_hits = deck_hits.saturating_sub(imp.perf_last_report_deck_hits.get());
        let delta_deck_misses = deck_misses.saturating_sub(imp.perf_last_report_deck_misses.get());
        let delta_deck_inserts =
            deck_inserts.saturating_sub(imp.perf_last_report_deck_inserts.get());
        let delta_deck_clears = deck_clears.saturating_sub(imp.perf_last_report_deck_clears.get());

        imp.perf_last_report_deck_hits.set(deck_hits);
        imp.perf_last_report_deck_misses.set(deck_misses);
        imp.perf_last_report_deck_inserts.set(deck_inserts);
        imp.perf_last_report_deck_clears.set(deck_clears);

        if delta_resize_events == 0 && delta_geometry_renders == 0 && delta_render_count == 0 {
            return;
        }

        let avg_render_ms = if delta_render_count > 0 {
            (delta_render_total_us as f64 / delta_render_count as f64) / 1000.0
        } else {
            0.0
        };
        let max_render_ms = imp.perf_render_max_us.get() as f64 / 1000.0;

        self.append_status_history_only(&format!(
            "resize_perf events={} src(poll/w/h/max)={}/{}/{}/{} geo_renders={} renders={} avg_ms={:.2} max_ms={:.2} deck_hit={} deck_miss={} deck_ins={} deck_clear={} deck_cache={}",
            delta_resize_events,
            resize_from_poll,
            resize_from_notify_width,
            resize_from_notify_height,
            resize_from_notify_max,
            delta_geometry_renders,
            delta_render_count,
            avg_render_ms,
            max_render_ms,
            delta_deck_hits,
            delta_deck_misses,
            delta_deck_inserts,
            delta_deck_clears,
            deck_cache_len
        ));

        imp.perf_render_max_us.set(0);
    }

    fn record_render_timing(&self, elapsed: std::time::Duration) {
        let imp = self.imp();
        let elapsed_us = elapsed.as_micros().min(u128::from(u64::MAX)) as u64;
        imp.perf_render_count
            .set(imp.perf_render_count.get().saturating_add(1));
        imp.perf_render_total_us
            .set(imp.perf_render_total_us.get().saturating_add(elapsed_us));
        if elapsed_us > imp.perf_render_max_us.get() {
            imp.perf_render_max_us.set(elapsed_us);
        }
        self.maybe_append_resize_perf_line();
    }

    pub(super) fn apply_mobile_phone_mode_overrides(&self) {
        let imp = self.imp();
        let mobile = imp.mobile_phone_mode.get();
        let hud_visible = imp.hud_enabled.get() && !mobile;

        // HUD rows are always suppressed in mobile-phone mode.
        imp.seed_controls_row.set_visible(hud_visible);
        imp.status_block_box.set_visible(hud_visible);

        // In mobile-phone mode, reduce visual chrome and keep gameplay controls.
        if mobile {
            imp.toolbar_box.set_visible(false);
            imp.board_box.set_spacing(2);
            imp.board_box.set_margin_top(1);
            imp.board_box.set_margin_bottom(1);
            imp.board_box.set_margin_start(1);
            imp.board_box.set_margin_end(1);
            imp.stock_heading_box.set_visible(false);
            imp.waste_heading_box.set_visible(false);
            imp.foundations_heading_box.set_visible(false);
            imp.stock_label.set_visible(false);
            imp.waste_label.set_visible(false);
            imp.tableau_frame.set_label(None);
            imp.stock_waste_foundation_spacer_box.set_visible(false);
            imp.foundations_area_box.set_spacing(2);

            // Bare-bones compact spacing at tiny sizes.
            imp.playfield_inner_box.set_spacing(2);
            imp.playfield_inner_box.set_margin_top(1);
            imp.playfield_inner_box.set_margin_bottom(1);
            imp.playfield_inner_box.set_margin_start(1);
            imp.playfield_inner_box.set_margin_end(1);
            imp.top_heading_row_box.set_spacing(2);
            imp.stock_waste_foundations_row_box.set_spacing(2);

            imp.board_color_menu_button.set_visible(false);
            imp.cyclone_shuffle_button.set_visible(false);
            imp.peek_button.set_visible(false);
            imp.tableau_row.set_margin_start(10);
            imp.tableau_row.set_margin_end(0);
            self.set_tableau_frame_inner_compactness(true);
        } else {
            imp.toolbar_box.set_visible(true);
            imp.board_box.set_spacing(2);
            imp.board_box.set_margin_top(1);
            imp.board_box.set_margin_bottom(1);
            imp.board_box.set_margin_start(1);
            imp.board_box.set_margin_end(1);
            imp.board_color_menu_button.set_visible(true);
            imp.cyclone_shuffle_button.set_visible(true);
            imp.peek_button.set_visible(true);
            imp.tableau_row.set_margin_start(0);
            imp.tableau_row.set_margin_end(0);
            imp.tableau_frame.set_label(Some("Tableau"));
            imp.stock_waste_foundation_spacer_box.set_visible(true);
            imp.foundations_area_box.set_spacing(8);

            imp.playfield_inner_box.set_spacing(2);
            imp.playfield_inner_box.set_margin_top(1);
            imp.playfield_inner_box.set_margin_bottom(1);
            imp.playfield_inner_box.set_margin_start(1);
            imp.playfield_inner_box.set_margin_end(1);
            imp.top_heading_row_box.set_spacing(2);
            imp.stock_waste_foundations_row_box.set_spacing(2);
            self.set_tableau_frame_inner_compactness(false);
        }
    }

    pub(super) fn render(&self) {
        let render_started = Instant::now();
        match self.active_game_mode() {
            GameMode::Spider => {
                self.render_spider();
                self.record_render_timing(render_started.elapsed());
                return;
            }
            GameMode::Freecell => {
                self.render_freecell();
                self.record_render_timing(render_started.elapsed());
                return;
            }
            GameMode::Klondike => {}
        }

        let imp = self.imp();
        imp.stock_picture.set_visible(true);
        imp.stock_label.set_visible(true);
        imp.stock_heading_box.set_visible(true);
        imp.stock_heading_label.set_label("Stock");
        imp.waste_overlay.set_visible(true);
        imp.waste_label.set_visible(true);
        imp.waste_heading_box.set_visible(true);
        imp.waste_heading_label.set_label("Waste");
        imp.top_row_spacer_box.set_visible(true);
        imp.stock_waste_foundation_spacer_box.set_visible(true);
        imp.selected_freecell.set(None);
        imp.foundations_heading_box.set_visible(true);
        imp.foundations_heading_label.set_label("Foundations");
        imp.foundations_area_box.set_visible(true);
        let view = boundary::game_view_model(
            &imp.game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        );
        let game = view.klondike();
        let mode = view.mode();
        let engine_ready = view.engine_ready();
        let caps = engine_for_mode(mode).capabilities();
        if engine_ready {
            self.note_current_seed_win_if_needed();
            if game.is_won() && imp.timer_started.get() {
                imp.timer_started.set(false);
            }
        }

        imp.stock_label
            .set_label(&render_plan::card_count_label(game.stock_len()));

        imp.waste_label
            .set_label(&render_plan::card_count_label(game.waste_len()));

        let foundation_labels = [
            &imp.foundation_label_1,
            &imp.foundation_label_2,
            &imp.foundation_label_3,
            &imp.foundation_label_4,
        ];

        for label in foundation_labels {
            label.set_label("");
        }

        let selected_tuple = render_plan::sanitize_selected_run(
            game,
            (*imp.selected_run.borrow()).map(|run| (run.col, run.start)),
        );
        let selected = selected_tuple.map(|(col, start)| SelectedRun { col, start });
        *imp.selected_run.borrow_mut() = selected;
        if imp.waste_selected.get() && game.waste_top().is_none() {
            imp.waste_selected.set(false);
        }

        self.render_card_images(game);

        let controls =
            render_plan::plan_controls(caps, imp.history.borrow().len(), imp.future.borrow().len());
        imp.undo_button.set_sensitive(controls.undo_enabled);
        imp.redo_button.set_sensitive(controls.redo_enabled);
        imp.auto_hint_button
            .set_sensitive(controls.auto_hint_enabled);
        imp.cyclone_shuffle_button
            .set_sensitive(controls.cyclone_enabled);
        imp.peek_button.set_sensitive(controls.peek_enabled);
        imp.robot_button.set_sensitive(controls.robot_enabled);
        imp.seed_random_button
            .set_sensitive(controls.seed_random_enabled);
        imp.seed_rescue_button
            .set_sensitive(controls.seed_rescue_enabled);
        imp.seed_winnable_button
            .set_sensitive(controls.seed_winnable_enabled);
        imp.seed_repeat_button
            .set_sensitive(controls.seed_repeat_enabled);
        imp.seed_go_button.set_sensitive(controls.seed_go_enabled);
        imp.seed_combo.set_sensitive(controls.seed_combo_enabled);

        self.update_keyboard_focus_style();
        let selected_status = selected.map(|run| (run.col, run.start));
        let show_controls_hint = imp.pending_deal_instructions.replace(false);
        let status = status_text::build_status_text(
            game,
            selected_status,
            imp.waste_selected.get(),
            imp.peek_active.get(),
            engine_ready,
            show_controls_hint,
            mode.label(),
            self.smart_move_mode().as_setting(),
            imp.deck_error.borrow().as_deref(),
            imp.status_override.borrow().as_deref(),
        );
        if !status.is_empty() {
            self.append_status_line(&status);
        }

        self.apply_mobile_phone_mode_overrides();
        self.update_tableau_overflow_hints();
        self.update_stats_label();
        self.mark_session_dirty();
        self.record_render_timing(render_started.elapsed());
    }

    fn render_spider(&self) {
        let imp = self.imp();
        imp.stock_picture.set_visible(true);
        imp.stock_label.set_visible(true);
        imp.stock_heading_box.set_visible(true);
        imp.stock_heading_label.set_label("Stock");
        imp.waste_overlay.set_visible(true);
        imp.waste_label.set_visible(true);
        imp.waste_heading_box.set_visible(true);
        imp.waste_heading_label.set_label("Waste");
        imp.top_row_spacer_box.set_visible(true);
        imp.stock_waste_foundation_spacer_box.set_visible(true);
        imp.selected_freecell.set(None);
        imp.foundations_heading_box.set_visible(false);
        imp.foundations_heading_label.set_label("Foundations");
        imp.foundations_area_box.set_visible(false);
        let mode = self.active_game_mode();
        let caps = engine_for_mode(mode).capabilities();
        let spider = imp.game.borrow().spider().clone();
        if spider.is_won() && imp.timer_started.get() {
            imp.timer_started.set(false);
        }
        self.note_current_seed_win_if_needed();

        imp.stock_label
            .set_label(&render_plan::card_count_label(spider.stock_len()));
        imp.waste_label
            .set_label(&format!("{} runs", spider.completed_runs()));

        let selected = (*imp.selected_run.borrow()).and_then(|run| {
            let len = spider.tableau().get(run.col).map(Vec::len)?;
            if run.start >= len {
                return None;
            }
            spider
                .tableau_card(run.col, run.start)
                .filter(|card| card.face_up)
                .map(|_| run)
        });
        *imp.selected_run.borrow_mut() = selected;
        imp.waste_selected.set(false);

        self.render_card_images_spider(&spider);

        let controls =
            render_plan::plan_controls(caps, imp.history.borrow().len(), imp.future.borrow().len());
        imp.undo_button.set_sensitive(controls.undo_enabled);
        imp.redo_button.set_sensitive(controls.redo_enabled);
        imp.auto_hint_button
            .set_sensitive(controls.auto_hint_enabled);
        imp.cyclone_shuffle_button
            .set_sensitive(controls.cyclone_enabled);
        imp.peek_button.set_sensitive(controls.peek_enabled);
        imp.robot_button.set_sensitive(controls.robot_enabled);
        imp.seed_random_button
            .set_sensitive(controls.seed_random_enabled);
        imp.seed_rescue_button
            .set_sensitive(controls.seed_rescue_enabled);
        imp.seed_winnable_button
            .set_sensitive(controls.seed_winnable_enabled);
        imp.seed_repeat_button
            .set_sensitive(controls.seed_repeat_enabled);
        imp.seed_go_button.set_sensitive(controls.seed_go_enabled);
        imp.seed_combo.set_sensitive(controls.seed_combo_enabled);

        self.update_keyboard_focus_style();
        let show_controls_hint = imp.pending_deal_instructions.replace(false);
        let status = if let Some(err) = imp.deck_error.borrow().as_deref() {
            format!("Card deck load failed: {err}")
        } else if let Some(message) = imp.status_override.borrow().as_deref() {
            message.to_string()
        } else if spider.is_won() {
            "Spider won! All runs are complete.".to_string()
        } else if let Some(run) = selected {
            let amount = spider
                .tableau()
                .get(run.col)
                .map(Vec::len)
                .unwrap_or(0)
                .saturating_sub(run.start);
            if amount > 1 {
                format!(
                    "Selected {amount} cards from T{}. Click another tableau to move this run.",
                    run.col + 1
                )
            } else {
                format!(
                    "Selected tableau T{}. Click another tableau to move top card.",
                    run.col + 1
                )
            }
        } else if show_controls_hint {
            "Spider controls: build suited descending runs from King to Ace. Keyboard: arrows move focus, Enter activates."
                .to_string()
        } else {
            String::new()
        };
        if !status.is_empty() {
            self.append_status_line(&status);
        }

        self.apply_mobile_phone_mode_overrides();
        self.update_tableau_overflow_hints();
        self.update_stats_label();
        self.mark_session_dirty();
    }

    fn render_freecell(&self) {
        let imp = self.imp();
        imp.foundations_heading_box.set_visible(true);
        imp.foundations_area_box.set_visible(true);
        imp.top_row_spacer_box.set_visible(false);
        imp.stock_waste_foundation_spacer_box.set_visible(false);
        let mode = self.active_game_mode();
        let caps = engine_for_mode(mode).capabilities();
        let freecell = imp.game.borrow().freecell().clone();
        if freecell.is_won() && imp.timer_started.get() {
            imp.timer_started.set(false);
        }
        self.note_current_seed_win_if_needed();

        imp.stock_picture.set_visible(false);
        imp.stock_label.set_visible(false);
        imp.stock_heading_box.set_visible(false);
        imp.waste_overlay.set_visible(true);
        imp.waste_label.set_visible(true);
        imp.waste_heading_box.set_visible(true);
        imp.waste_heading_label.set_label("Free Cells");
        imp.foundations_heading_label.set_label("Foundations");

        let selected = (*imp.selected_run.borrow()).and_then(|run| {
            let len = freecell.tableau().get(run.col).map(Vec::len)?;
            if run.start >= len {
                return None;
            }
            Some(run)
        });
        *imp.selected_run.borrow_mut() = selected;
        imp.waste_selected.set(false);

        self.render_card_images_freecell(&freecell);

        let controls =
            render_plan::plan_controls(caps, imp.history.borrow().len(), imp.future.borrow().len());
        imp.undo_button.set_sensitive(controls.undo_enabled);
        imp.redo_button.set_sensitive(controls.redo_enabled);
        imp.auto_hint_button
            .set_sensitive(controls.auto_hint_enabled);
        imp.cyclone_shuffle_button
            .set_sensitive(controls.cyclone_enabled);
        imp.peek_button.set_sensitive(controls.peek_enabled);
        imp.robot_button.set_sensitive(controls.robot_enabled);
        imp.seed_random_button
            .set_sensitive(controls.seed_random_enabled);
        imp.seed_rescue_button
            .set_sensitive(controls.seed_rescue_enabled);
        imp.seed_winnable_button
            .set_sensitive(controls.seed_winnable_enabled);
        imp.seed_repeat_button
            .set_sensitive(controls.seed_repeat_enabled);
        imp.seed_go_button.set_sensitive(controls.seed_go_enabled);
        imp.seed_combo.set_sensitive(controls.seed_combo_enabled);

        self.update_keyboard_focus_style();
        let show_controls_hint = imp.pending_deal_instructions.replace(false);
        let status = if let Some(err) = imp.deck_error.borrow().as_deref() {
            format!("Card deck load failed: {err}")
        } else if let Some(message) = imp.status_override.borrow().as_deref() {
            message.to_string()
        } else if freecell.is_won() {
            "FreeCell won. all foundations complete.".to_string()
        } else if freecell.is_lost() {
            "FreeCell lost. no legal moves remain.".to_string()
        } else if let Some(slot) = imp.selected_freecell.get() {
            format!(
                "Selected free cell F{}. Click a tableau column or foundation to move it.",
                slot + 1
            )
        } else if let Some(run) = selected {
            let amount = freecell
                .tableau()
                .get(run.col)
                .map(Vec::len)
                .unwrap_or(0)
                .saturating_sub(run.start);
            if amount > 1 {
                format!(
                    "Selected {amount} cards from T{}. Click another tableau to move this run.",
                    run.col + 1
                )
            } else {
                format!(
                    "Selected tableau T{}. Click a tableau, free cell, or foundation to move it.",
                    run.col + 1
                )
            }
        } else if show_controls_hint {
            "FreeCell controls: build descending alternating cascades, clear to foundations, use free cells as buffers. Keyboard: arrows move focus, Enter activates."
                .to_string()
        } else {
            String::new()
        };
        if !status.is_empty() {
            self.append_status_line(&status);
        }

        self.apply_mobile_phone_mode_overrides();
        self.update_tableau_overflow_hints();
        self.update_stats_label();
        self.mark_session_dirty();
    }

    pub(super) fn flash_smart_move_fail_tableau_run(&self, col: usize, start: usize) {
        let imp = self.imp();
        let previous_selected = *imp.selected_run.borrow();
        let previous_waste_selected = imp.waste_selected.get();

        *imp.selected_run.borrow_mut() = Some(SelectedRun { col, start });
        imp.waste_selected.set(false);
        self.render();

        glib::timeout_add_local_once(
            Duration::from_millis(100),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move || {
                    let imp = window.imp();
                    let current = *imp.selected_run.borrow();
                    if current == Some(SelectedRun { col, start }) {
                        *imp.selected_run.borrow_mut() = previous_selected;
                        imp.waste_selected.set(previous_waste_selected);
                        window.render();
                    }
                }
            ),
        );
    }

    pub(super) fn flash_smart_move_fail_waste_top(&self) {
        let imp = self.imp();
        let game = imp.game.borrow();
        let show_count = render_plan::waste_visible_count(game.draw_mode(), game.waste_len());
        if show_count == 0 {
            return;
        }
        drop(game);

        let previous_selected = *imp.selected_run.borrow();
        let previous_waste_selected = imp.waste_selected.get();

        *imp.selected_run.borrow_mut() = None;
        imp.waste_selected.set(true);
        self.render();

        glib::timeout_add_local_once(
            Duration::from_millis(100),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move || {
                    let imp = window.imp();
                    if imp.waste_selected.get() {
                        *imp.selected_run.borrow_mut() = previous_selected;
                        imp.waste_selected.set(previous_waste_selected);
                        window.render();
                    }
                }
            ),
        );
    }

    pub(super) fn render_card_images(&self, game: &KlondikeGame) {
        let imp = self.imp();

        if !imp.deck_load_attempted.get() {
            imp.deck_load_attempted.set(true);
            let loaded = if let Some(settings) = Self::load_app_settings() {
                let custom_svg = settings.string(SETTINGS_KEY_CUSTOM_CARD_SVG).to_string();
                if custom_svg.trim().is_empty() {
                    AngloDeck::load()
                } else {
                    AngloDeck::load_with_custom_normal_svg(&custom_svg)
                }
            } else {
                AngloDeck::load()
            };

            match loaded {
                Ok(deck) => {
                    *imp.deck.borrow_mut() = Some(deck);
                    *imp.deck_error.borrow_mut() = None;
                }
                Err(err) => {
                    *imp.deck_error.borrow_mut() = Some(err);
                }
            }
        }

        let deck_slot = imp.deck.borrow();
        let Some(deck) = deck_slot.as_ref() else {
            return;
        };

        self.update_tableau_metrics();
        let card_width = imp.card_width.get();
        let card_height = imp.card_height.get();
        let face_up_step = imp.face_up_step.get();
        let face_down_step = imp.face_down_step.get();
        let peek_active = imp.peek_active.get();

        self.configure_stock_waste_foundation_widgets(card_width, card_height);
        self.render_stock_picture(game, deck, card_width, card_height);
        self.render_waste_fan(game, deck, card_width, card_height);
        self.render_foundations_area(game, deck, card_width, card_height);
        self.render_tableau_columns(
            game,
            deck,
            card_width,
            card_height,
            face_up_step,
            face_down_step,
            peek_active,
        );
    }

    fn render_card_images_spider(&self, game: &SpiderGame) {
        let imp = self.imp();

        if !imp.deck_load_attempted.get() {
            imp.deck_load_attempted.set(true);
            let loaded = if let Some(settings) = Self::load_app_settings() {
                let custom_svg = settings.string(SETTINGS_KEY_CUSTOM_CARD_SVG).to_string();
                if custom_svg.trim().is_empty() {
                    AngloDeck::load()
                } else {
                    AngloDeck::load_with_custom_normal_svg(&custom_svg)
                }
            } else {
                AngloDeck::load()
            };

            match loaded {
                Ok(deck) => {
                    *imp.deck.borrow_mut() = Some(deck);
                    *imp.deck_error.borrow_mut() = None;
                }
                Err(err) => {
                    *imp.deck_error.borrow_mut() = Some(err);
                }
            }
        }

        let deck_slot = imp.deck.borrow();
        let Some(deck) = deck_slot.as_ref() else {
            return;
        };

        self.update_tableau_metrics();
        let card_width = imp.card_width.get();
        let card_height = imp.card_height.get();
        let face_up_step = imp.face_up_step.get();
        let face_down_step = imp.face_down_step.get();
        let peek_active = imp.peek_active.get();

        self.configure_stock_waste_foundation_widgets(card_width, card_height);
        self.render_stock_picture_spider(game, deck, card_width, card_height);
        self.render_waste_fan_spider();
        self.render_foundations_area_spider(game, deck, card_width, card_height);
        self.render_tableau_columns_spider(
            game,
            deck,
            card_width,
            card_height,
            face_up_step,
            face_down_step,
            peek_active,
        );
    }

    fn render_card_images_freecell(&self, game: &FreecellGame) {
        let imp = self.imp();

        if !imp.deck_load_attempted.get() {
            imp.deck_load_attempted.set(true);
            let loaded = if let Some(settings) = Self::load_app_settings() {
                let custom_svg = settings.string(SETTINGS_KEY_CUSTOM_CARD_SVG).to_string();
                if custom_svg.trim().is_empty() {
                    AngloDeck::load()
                } else {
                    AngloDeck::load_with_custom_normal_svg(&custom_svg)
                }
            } else {
                AngloDeck::load()
            };

            match loaded {
                Ok(deck) => {
                    *imp.deck.borrow_mut() = Some(deck);
                    *imp.deck_error.borrow_mut() = None;
                }
                Err(err) => {
                    *imp.deck_error.borrow_mut() = Some(err);
                }
            }
        }

        let deck_slot = imp.deck.borrow();
        let Some(deck) = deck_slot.as_ref() else {
            return;
        };

        self.update_tableau_metrics();
        let card_width = imp.card_width.get();
        let card_height = imp.card_height.get();
        let face_up_step = imp.face_up_step.get();
        let face_down_step = imp.face_down_step.get();

        self.configure_stock_waste_foundation_widgets(card_width, card_height);
        self.render_freecell_slots(game, deck, card_width, card_height);
        self.render_foundations_area_freecell(game, deck, card_width, card_height);
        self.render_tableau_columns_freecell(
            game,
            deck,
            card_width,
            card_height,
            face_up_step,
            face_down_step,
        );
    }

    pub(super) fn set_picture_from_card(
        &self,
        picture: &gtk::Picture,
        card: Option<Card>,
        deck: &AngloDeck,
        width: i32,
        height: i32,
    ) {
        match card {
            Some(card) => {
                let texture = deck.texture_for_card_scaled(card, width, height);
                picture.set_paintable(Some(&texture));
            }
            None => picture.set_paintable(None::<&gdk::Paintable>),
        }
    }

    pub(super) fn blank_texture(width: i32, height: i32) -> gdk::Texture {
        let pixbuf = gdk_pixbuf::Pixbuf::new(
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            width.max(1),
            height.max(1),
        )
        .expect("failed to allocate blank pixbuf");
        pixbuf.fill(0x00000000);
        gdk::Texture::for_pixbuf(&pixbuf)
    }

    pub(super) fn foundation_pictures(&self) -> [gtk::Picture; 4] {
        let imp = self.imp();
        [
            imp.foundation_picture_1.get(),
            imp.foundation_picture_2.get(),
            imp.foundation_picture_3.get(),
            imp.foundation_picture_4.get(),
        ]
    }

    pub(super) fn foundation_placeholders(&self) -> [gtk::Label; 4] {
        let imp = self.imp();
        [
            imp.foundation_placeholder_1.get(),
            imp.foundation_placeholder_2.get(),
            imp.foundation_placeholder_3.get(),
            imp.foundation_placeholder_4.get(),
        ]
    }

    pub(super) fn waste_fan_slots(&self) -> [gtk::Picture; 5] {
        let imp = self.imp();
        [
            imp.waste_picture_1.get(),
            imp.waste_picture_2.get(),
            imp.waste_picture_3.get(),
            imp.waste_picture_4.get(),
            imp.waste_picture_5.get(),
        ]
    }

    pub(super) fn tableau_stacks(&self) -> [gtk::Fixed; 10] {
        let imp = self.imp();
        [
            imp.tableau_stack_1.get(),
            imp.tableau_stack_2.get(),
            imp.tableau_stack_3.get(),
            imp.tableau_stack_4.get(),
            imp.tableau_stack_5.get(),
            imp.tableau_stack_6.get(),
            imp.tableau_stack_7.get(),
            imp.tableau_stack_8.get(),
            imp.tableau_stack_9.get(),
            imp.tableau_stack_10.get(),
        ]
    }

    pub(super) fn invalidate_card_render_cache(&self) {
        for col in self
            .imp()
            .tableau_picture_state_cache
            .borrow_mut()
            .iter_mut()
        {
            col.clear();
        }
    }

    fn append_status_line(&self, status: &str) {
        let sanitized = self.sanitize_status_for_display(status);
        let imp = self.imp();
        if imp.status_last_appended.borrow().as_str() == sanitized {
            return;
        }

        *imp.status_last_appended.borrow_mut() = sanitized.clone();
        self.append_status_history_only(&sanitized);
        imp.status_label.set_label(&sanitized);
        imp.status_label.set_tooltip_text(Some(&sanitized));
    }

    pub(super) fn show_status_history_dialog(&self) {
        if let Some(existing) = self.imp().status_history_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let joined = self
            .imp()
            .status_history
            .borrow()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n");

        let dialog = gtk::Window::builder()
            .title("Status History")
            .modal(false)
            .default_width(760)
            .default_height(420)
            .build();
        if let Some(app) = self.application() {
            dialog.set_application(Some(&app));
        }
        dialog.set_resizable(true);
        dialog.set_deletable(true);
        dialog.set_hide_on_close(true);
        dialog.set_destroy_with_parent(true);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.set_margin_top(10);
        root.set_margin_bottom(10);
        root.set_margin_start(10);
        root.set_margin_end(10);

        let scroller = gtk::ScrolledWindow::new();
        scroller.set_hexpand(true);
        scroller.set_vexpand(true);
        scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        let text = gtk::TextView::new();
        text.set_editable(false);
        text.set_cursor_visible(false);
        text.set_monospace(true);
        text.set_wrap_mode(gtk::WrapMode::WordChar);
        text.buffer().set_text(&joined);
        *self.imp().status_history_buffer.borrow_mut() = Some(text.buffer());
        scroller.set_child(Some(&text));
        root.append(&scroller);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        actions.set_halign(gtk::Align::End);

        let clear = gtk::Button::with_label("Clear");
        clear.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            text,
            move |_| {
                let imp = window.imp();
                imp.status_history.borrow_mut().clear();
                imp.status_last_appended.borrow_mut().clear();
                imp.status_label.set_label("");
                imp.status_label.set_tooltip_text(None);
                let buffer = imp.status_history_buffer.borrow().as_ref().cloned();
                if let Some(buffer) = buffer {
                    buffer.set_text("");
                } else {
                    text.buffer().set_text("");
                }
            }
        ));
        actions.append(&clear);

        let copy = gtk::Button::with_label("Copy");
        copy.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                let payload = window
                    .imp()
                    .status_history
                    .borrow()
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join("\n");
                window.clipboard().set_text(&payload);
                *window.imp().status_override.borrow_mut() =
                    Some("Copied status history to clipboard.".to_string());
                window.render();
            }
        ));
        actions.append(&copy);

        let close = gtk::Button::with_label("Close");
        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        actions.append(&close);
        root.append(&actions);

        dialog.set_child(Some(&root));
        *self.imp().status_history_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }
}

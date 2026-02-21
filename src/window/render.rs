use super::*;
use crate::engine::boundary;
use crate::engine::render_plan;
use crate::engine::status_text;
use crate::engine::variant_engine::engine_for_mode;
use crate::game::{FreecellGame, SpiderGame};
use crate::startup_trace;
use sourceview5::prelude::*;
use std::time::Instant;

const LAYOUT_DEBUG_HISTORY_ENABLED: bool = false;
const SECONDARY_WINDOW_STATUS_HINT: &str =
    "Secondary window: session and seed history are not auto-saved.";
const STATUS_HISTORY_DEFAULT_RETENTION_LINES: usize = 1000;
const STATUS_HISTORY_RETENTION_CHOICES: &[(usize, &str)] = &[
    (120, "120"),
    (240, "240"),
    (500, "500"),
    (1000, "1,000 (Default)"),
    (2000, "2,000"),
    (5000, "5,000"),
    (10000, "10,000"),
    (25000, "25,000"),
];

impl CardthropicWindow {
    fn chess_robot_status_suffix(&self) -> Option<String> {
        let imp = self.imp();
        if !(imp.chess_mode_active.get() && imp.robot_mode_running.get()) {
            return None;
        }
        let ludicrous = if imp.robot_ludicrous_enabled.get() {
            "on"
        } else {
            "off"
        };
        let forever = if imp.robot_forever_enabled.get() {
            "on"
        } else {
            "off"
        };
        let robot_white =
            self.chess_robot_ai_strength_label_for_side(crate::game::ChessColor::White);
        let robot_black =
            self.chess_robot_ai_strength_label_for_side(crate::game::ChessColor::Black);
        Some(format!(
            "Chess Robot: White={robot_white} | Black={robot_black} | ludicrous={ludicrous} | forever={forever}"
        ))
    }

    fn current_variant_name(&self) -> String {
        if self.imp().chess_mode_active.get() {
            return self.imp().chess_variant.get().label().to_string();
        }
        match self.active_game_mode() {
            GameMode::Klondike => {
                format!(
                    "Klondike Deal {}",
                    self.current_klondike_draw_mode().count()
                )
            }
            GameMode::Spider => {
                format!(
                    "Spider Suit {}",
                    self.current_spider_suit_mode().suit_count()
                )
            }
            GameMode::Freecell => format!(
                "FreeCell {} Cards | {} Cells",
                self.current_freecell_card_count_mode().card_count(),
                self.current_freecell_cell_count()
            ),
        }
    }

    fn refresh_window_title(&self) {
        let title = format!(
            "Cardthropic \u{2014} {} ({})",
            self.current_variant_name(),
            self.imp().current_seed.get()
        );
        self.set_title(Some(&title));
    }

    fn decorate_status_for_window(&self, status: &str) -> String {
        let trimmed = status.trim();
        let with_chess_robot = if let Some(suffix) = self.chess_robot_status_suffix() {
            if trimmed.is_empty() {
                suffix
            } else if trimmed.contains(&suffix) {
                trimmed.to_string()
            } else {
                format!("{trimmed} | {suffix}")
            }
        } else {
            trimmed.to_string()
        };
        if self.should_persist_shared_state() {
            return with_chess_robot;
        }
        if with_chess_robot.is_empty() {
            return SECONDARY_WINDOW_STATUS_HINT.to_string();
        }
        format!("{with_chess_robot} | {SECONDARY_WINDOW_STATUS_HINT}")
    }

    fn status_performance_mode_active(&self) -> bool {
        let imp = self.imp();
        imp.robot_mode_running.get()
            && imp.robot_ludicrous_enabled.get()
            && imp.robot_forever_enabled.get()
            && imp.robot_auto_new_game_on_loss.get()
            && !imp.robot_debug_enabled.get()
    }

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

        if status.contains(" event=move_applied ") {
            return false;
        }

        let sample_mod = if status.contains(" event=planner_wait ") {
            16_u32
        } else if status.contains(" event=planner_ready ") {
            8_u32
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

    fn status_history_retention_limit(&self) -> usize {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return STATUS_HISTORY_DEFAULT_RETENTION_LINES;
        };
        let Some(schema) = settings.settings_schema() else {
            return STATUS_HISTORY_DEFAULT_RETENTION_LINES;
        };
        if !schema.has_key(SETTINGS_KEY_STATUS_HISTORY_RETENTION_LINES) {
            return STATUS_HISTORY_DEFAULT_RETENTION_LINES;
        }
        let Ok(raw) = usize::try_from(settings.int(SETTINGS_KEY_STATUS_HISTORY_RETENTION_LINES))
        else {
            return STATUS_HISTORY_DEFAULT_RETENTION_LINES;
        };
        if STATUS_HISTORY_RETENTION_CHOICES
            .iter()
            .any(|(value, _)| *value == raw)
        {
            raw
        } else {
            STATUS_HISTORY_DEFAULT_RETENTION_LINES
        }
    }

    fn persist_status_history_retention_limit(&self, limit: usize) {
        if !STATUS_HISTORY_RETENTION_CHOICES
            .iter()
            .any(|(value, _)| *value == limit)
        {
            return;
        }
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_STATUS_HISTORY_RETENTION_LINES) {
            return;
        }
        let Ok(limit_i32) = i32::try_from(limit) else {
            return;
        };
        if settings.int(SETTINGS_KEY_STATUS_HISTORY_RETENTION_LINES) != limit_i32 {
            let _ = settings.set_int(SETTINGS_KEY_STATUS_HISTORY_RETENTION_LINES, limit_i32);
        }
    }

    fn apply_status_history_retention_limit(&self) -> String {
        let limit = self.status_history_retention_limit();
        let imp = self.imp();
        let mut history = imp.status_history.borrow_mut();
        while history.len() > limit {
            let _ = history.pop_front();
        }
        history
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub(super) fn append_status_history_only(&self, status: &str) {
        if self.should_throttle_robot_history_line(status) {
            return;
        }
        if self.status_performance_mode_active()
            && (status.starts_with("robot_v=")
                || status.starts_with("bench_v=")
                || status.starts_with("W/L ")
                || status.starts_with("Robot "))
        {
            let is_move_line =
                status.contains(" event=move_applied ") || status.contains("Event: move applied.");
            if !is_move_line {
                return;
            }
        }

        let imp = self.imp();
        let history_limit = self.status_history_retention_limit();
        let mut history = imp.status_history.borrow_mut();
        history.push_back(status.to_string());
        while history.len() > history_limit {
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
        if !LAYOUT_DEBUG_HISTORY_ENABLED {
            return;
        }
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
        let spider_mode = self.active_game_mode() == GameMode::Spider;
        let chess_mode = imp.chess_mode_active.get();
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
            imp.foundations_area_box
                .set_spacing(if spider_mode { 0 } else { 2 });

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
            imp.seed_rescue_button.set_visible(!chess_mode);
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
            imp.cyclone_shuffle_button.set_visible(!chess_mode);
            imp.peek_button.set_visible(!chess_mode);
            imp.seed_rescue_button.set_visible(!chess_mode);
            imp.tableau_row.set_margin_start(0);
            imp.tableau_row.set_margin_end(0);
            imp.tableau_frame.set_label(Some("Tableau"));
            // Keep top-row groups left-flow aligned across modes.
            imp.stock_waste_foundation_spacer_box.set_visible(false);
            imp.foundations_area_box
                .set_spacing(if spider_mode { 0 } else { 8 });

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
        startup_trace::mark_once("render:first-enter");
        let render_started = Instant::now();
        self.refresh_window_title();
        if self.imp().chess_mode_active.get() {
            self.render_chess_board();
            self.record_render_timing(render_started.elapsed());
            startup_trace::mark_once("render:first-exit");
            return;
        }
        self.clear_chess_board_rotation_transform();
        self.imp().chess_drag_hover_row_from_top.set(None);
        let was_chess_surface = self
            .imp()
            .tableau_frame
            .has_css_class("chess-frame-no-label");
        if was_chess_surface {
            // Chess renders stack-specific GtkLabel widgets into tableau columns.
            // Remove those one time when switching back so card modes remain isolated.
            self.clear_tableau_render_state_for_chess();
            self.invalidate_card_render_cache();
        }
        self.imp()
            .tableau_frame
            .remove_css_class("chess-frame-no-label");
        self.imp()
            .tableau_frame
            .set_label_widget(None::<&gtk::Widget>);
        self.imp().tableau_frame.set_label(Some("Tableau"));
        for stack in self.tableau_stacks() {
            stack.remove_css_class("chess-tableau-drop-target");
            stack.add_css_class("tableau-drop-target");
        }
        self.imp().top_playfield_frame.set_visible(true);
        self.imp().top_heading_row_box.set_visible(true);
        self.imp().stock_waste_foundations_row_box.set_visible(true);
        match self.active_game_mode() {
            GameMode::Spider => {
                self.render_spider();
                self.record_render_timing(render_started.elapsed());
                startup_trace::mark_once("render:first-exit");
                return;
            }
            GameMode::Freecell => {
                self.render_freecell();
                self.record_render_timing(render_started.elapsed());
                startup_trace::mark_once("render:first-exit");
                return;
            }
            GameMode::Klondike => {}
        }

        let imp = self.imp();
        imp.stock_picture.set_visible(true);
        imp.stock_column_box.set_visible(true);
        imp.stock_label.set_visible(false);
        imp.stock_heading_box.set_visible(true);
        imp.stock_heading_label.set_label("Stock");
        imp.waste_overlay.set_visible(true);
        imp.waste_column_box.set_visible(true);
        imp.waste_label.set_visible(false);
        imp.waste_heading_box.set_visible(true);
        imp.waste_heading_label.set_label("Waste");
        imp.top_row_spacer_box.set_visible(false);
        imp.stock_waste_foundation_spacer_box.set_visible(false);
        imp.selected_freecell.set(None);
        imp.foundations_heading_box.set_visible(true);
        imp.foundations_heading_box.set_halign(gtk::Align::Start);
        imp.foundations_heading_box.set_margin_start(0);
        imp.foundations_heading_label.set_label("Foundations");
        imp.foundations_heading_label.set_xalign(0.0);
        imp.foundations_heading_label.set_halign(gtk::Align::Start);
        imp.foundations_area_box.set_halign(gtk::Align::Start);
        imp.foundations_area_box.set_margin_start(0);
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

        let selected_snapshot = imp.selected_run.try_borrow().ok().and_then(|run| *run);
        let selected_tuple = render_plan::sanitize_selected_run(
            game,
            selected_snapshot.map(|run| (run.col, run.start)),
        );
        let selected = selected_tuple.map(|(col, start)| SelectedRun { col, start });
        if let Ok(mut selected_run) = imp.selected_run.try_borrow_mut() {
            *selected_run = selected;
        }
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
            None,
            imp.status_override.borrow().as_deref(),
        );
        self.append_status_line(&status);

        self.apply_mobile_phone_mode_overrides();
        self.update_tableau_overflow_hints();
        self.update_stats_label();
        self.mark_session_dirty();
        self.record_render_timing(render_started.elapsed());
        startup_trace::mark_once("render:first-exit");
    }

    fn render_spider(&self) {
        let imp = self.imp();
        imp.stock_picture.set_visible(true);
        imp.stock_column_box.set_visible(true);
        imp.stock_label.set_visible(false);
        imp.stock_heading_box.set_visible(true);
        imp.stock_heading_label.set_label("Stock");
        imp.waste_overlay.set_visible(false);
        imp.waste_column_box.set_visible(false);
        imp.waste_label.set_visible(false);
        imp.waste_heading_box.set_visible(false);
        imp.waste_heading_label.set_label("Waste");
        imp.top_row_spacer_box.set_visible(false);
        imp.stock_waste_foundation_spacer_box.set_visible(false);
        imp.selected_freecell.set(None);
        imp.foundations_heading_box.set_visible(true);
        imp.foundations_heading_box.set_halign(gtk::Align::Start);
        imp.foundations_heading_box.set_margin_start(0);
        imp.foundations_heading_label.set_label("Completed Runs");
        imp.foundations_heading_label.set_xalign(0.0);
        imp.foundations_heading_label.set_halign(gtk::Align::Start);
        imp.foundations_area_box.set_halign(gtk::Align::Start);
        imp.foundations_area_box.set_margin_start(0);
        imp.foundations_area_box.set_visible(true);
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

        let selected_snapshot = imp.selected_run.try_borrow().ok().and_then(|run| *run);
        let selected = selected_snapshot.and_then(|run| {
            let len = spider.tableau().get(run.col).map(Vec::len)?;
            if run.start >= len {
                return None;
            }
            spider
                .tableau_card(run.col, run.start)
                .filter(|card| card.face_up)
                .map(|_| run)
        });
        if let Ok(mut selected_run) = imp.selected_run.try_borrow_mut() {
            *selected_run = selected;
        }
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
        let status = if let Some(message) = imp.status_override.borrow().as_deref() {
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
        self.append_status_line(&status);

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
        let freecell_cells = freecell.freecell_count();
        if freecell.is_won() && imp.timer_started.get() {
            imp.timer_started.set(false);
        }
        self.note_current_seed_win_if_needed();

        imp.stock_picture.set_visible(false);
        imp.stock_column_box.set_visible(false);
        imp.stock_label.set_visible(false);
        imp.stock_heading_box.set_visible(false);
        imp.waste_overlay.set_visible(true);
        imp.waste_column_box.set_visible(true);
        imp.waste_label.set_visible(true);
        imp.waste_heading_box.set_visible(true);
        imp.waste_heading_label.set_label("Free Cells");
        imp.foundations_heading_box.set_halign(gtk::Align::Start);
        imp.foundations_heading_box.set_margin_start(8);
        imp.foundations_heading_label.set_label("Foundations");
        imp.foundations_heading_label.set_xalign(0.0);
        imp.foundations_heading_label.set_halign(gtk::Align::Start);
        imp.foundations_area_box.set_halign(gtk::Align::Start);
        imp.foundations_area_box.set_margin_start(8);

        let selected_snapshot = imp.selected_run.try_borrow().ok().and_then(|run| *run);
        let selected = selected_snapshot.and_then(|run| {
            let len = freecell.tableau().get(run.col).map(Vec::len)?;
            if run.start >= len {
                return None;
            }
            Some(run)
        });
        if let Ok(mut selected_run) = imp.selected_run.try_borrow_mut() {
            *selected_run = selected;
        }
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
        let status = if let Some(message) = imp.status_override.borrow().as_deref() {
            message.to_string()
        } else if freecell.is_won() {
            format!("FreeCell won with {freecell_cells} free cells. All foundations complete.")
        } else if freecell.is_lost() {
            format!("FreeCell blocked ({freecell_cells} free cells). No legal moves remain.")
        } else if let Some(slot) = imp.selected_freecell.get() {
            format!(
                "Selected free cell F{}/{}. Click a tableau column or foundation to move it.",
                slot + 1,
                freecell_cells
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
                    "Selected tableau T{}. Click a tableau, free cell (1-{}), or foundation to move it.",
                    run.col + 1,
                    freecell_cells
                )
            }
        } else if show_controls_hint {
            format!(
                "FreeCell controls: build descending alternating cascades, clear to foundations, use {freecell_cells} free cells as buffers. Keyboard: arrows move focus, Enter activates."
            )
        } else {
            String::new()
        };
        self.append_status_line(&status);

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
        startup_trace::mark_once("render:first-images-enter");
        let imp = self.imp();
        let deck_slot = imp.deck.borrow();
        let deck = deck_slot.as_ref();

        startup_trace::mark_once("render:first-metrics-enter");
        self.update_tableau_metrics();
        let card_width = imp.card_width.get();
        let card_height = imp.card_height.get();
        let face_up_step = imp.face_up_step.get();
        let face_down_step = imp.face_down_step.get();
        let peek_active = imp.peek_active.get();
        startup_trace::mark_once("render:first-metrics-exit");

        startup_trace::mark_once("render:first-toprow-enter");
        self.configure_stock_waste_foundation_widgets(card_width, card_height);
        self.render_stock_picture(game, deck, card_width, card_height);
        self.render_waste_fan(game, deck, card_width, card_height);
        self.render_foundations_area(game, deck, card_width, card_height);
        startup_trace::mark_once("render:first-toprow-exit");
        startup_trace::mark_once("render:first-tableau-enter");
        self.render_tableau_columns(
            game,
            deck,
            card_width,
            card_height,
            face_up_step,
            face_down_step,
            peek_active,
        );
        startup_trace::mark_once("render:first-tableau-exit");
        startup_trace::mark_once("render:first-images-exit");
    }

    fn render_card_images_spider(&self, game: &SpiderGame) {
        startup_trace::mark_once("render:first-images-enter");
        let imp = self.imp();
        let deck_slot = imp.deck.borrow();
        let deck = deck_slot.as_ref();

        startup_trace::mark_once("render:first-metrics-enter");
        self.update_tableau_metrics();
        let card_width = imp.card_width.get();
        let card_height = imp.card_height.get();
        let face_up_step = imp.face_up_step.get();
        let face_down_step = imp.face_down_step.get();
        let peek_active = imp.peek_active.get();
        startup_trace::mark_once("render:first-metrics-exit");

        startup_trace::mark_once("render:first-toprow-enter");
        self.configure_stock_waste_foundation_widgets(card_width, card_height);
        self.render_stock_picture_spider(game, deck, card_width, card_height);
        self.render_waste_fan_spider(card_width, card_height);
        self.render_foundations_area_spider(game, deck, card_width, card_height);
        startup_trace::mark_once("render:first-toprow-exit");
        startup_trace::mark_once("render:first-tableau-enter");
        self.render_tableau_columns_spider(
            game,
            deck,
            card_width,
            card_height,
            face_up_step,
            face_down_step,
            peek_active,
        );
        startup_trace::mark_once("render:first-tableau-exit");
        startup_trace::mark_once("render:first-images-exit");
    }

    fn render_card_images_freecell(&self, game: &FreecellGame) {
        startup_trace::mark_once("render:first-images-enter");
        let imp = self.imp();
        let deck_slot = imp.deck.borrow();
        let deck = deck_slot.as_ref();

        startup_trace::mark_once("render:first-metrics-enter");
        self.update_tableau_metrics();
        let card_width = imp.card_width.get();
        let card_height = imp.card_height.get();
        let face_up_step = imp.face_up_step.get();
        let face_down_step = imp.face_down_step.get();
        startup_trace::mark_once("render:first-metrics-exit");

        startup_trace::mark_once("render:first-toprow-enter");
        self.configure_stock_waste_foundation_widgets(card_width, card_height);
        self.render_freecell_slots(game, deck, card_width, card_height);
        self.render_foundations_area_freecell(game, deck, card_width, card_height);
        startup_trace::mark_once("render:first-toprow-exit");
        startup_trace::mark_once("render:first-tableau-enter");
        self.render_tableau_columns_freecell(
            game,
            deck,
            card_width,
            card_height,
            face_up_step,
            face_down_step,
        );
        startup_trace::mark_once("render:first-tableau-exit");
        startup_trace::mark_once("render:first-images-exit");
    }

    pub(super) fn set_picture_from_card(
        &self,
        picture: &gtk::Picture,
        card: Option<Card>,
        deck: Option<&AngloDeck>,
        width: i32,
        height: i32,
    ) {
        match card {
            Some(card) => {
                if let Some(paintable) =
                    self.paintable_for_card_display(Some(card), true, deck, width, height)
                {
                    picture.set_paintable(Some(&paintable));
                } else {
                    picture.set_paintable(None::<&gdk::Paintable>);
                }
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

    pub(super) fn foundation_pictures(&self) -> [gtk::Picture; 8] {
        let imp = self.imp();
        [
            imp.foundation_picture_1.get(),
            imp.foundation_picture_2.get(),
            imp.foundation_picture_3.get(),
            imp.foundation_picture_4.get(),
            imp.foundation_picture_5.get(),
            imp.foundation_picture_6.get(),
            imp.foundation_picture_7.get(),
            imp.foundation_picture_8.get(),
        ]
    }

    pub(super) fn foundation_placeholders(&self) -> [gtk::Label; 8] {
        let imp = self.imp();
        [
            imp.foundation_placeholder_1.get(),
            imp.foundation_placeholder_2.get(),
            imp.foundation_placeholder_3.get(),
            imp.foundation_placeholder_4.get(),
            imp.foundation_placeholder_5.get(),
            imp.foundation_placeholder_6.get(),
            imp.foundation_placeholder_7.get(),
            imp.foundation_placeholder_8.get(),
        ]
    }

    pub(super) fn waste_fan_slots(&self) -> [gtk::Picture; 6] {
        let imp = self.imp();
        [
            imp.waste_picture_1.get(),
            imp.waste_picture_2.get(),
            imp.waste_picture_3.get(),
            imp.waste_picture_4.get(),
            imp.waste_picture_5.get(),
            imp.waste_picture_6.get(),
        ]
    }

    pub(super) fn freecell_slot_pictures(&self) -> [gtk::Picture; 6] {
        let imp = self.imp();
        [
            imp.waste_picture_1.get(),
            imp.waste_picture_2.get(),
            imp.waste_picture_3.get(),
            imp.waste_picture_4.get(),
            imp.waste_picture_5.get(),
            imp.waste_picture_6.get(),
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

    pub(super) fn append_status_line(&self, status: &str) {
        let decorated = self.decorate_status_for_window(status);
        if decorated.is_empty() {
            return;
        }
        let sanitized = self.sanitize_status_for_display(&decorated);
        let imp = self.imp();
        if imp.status_last_appended.borrow().as_str() == sanitized {
            return;
        }

        *imp.status_last_appended.borrow_mut() = sanitized.clone();
        let is_klondike_controls_hint = sanitized.starts_with("Klondike controls:");
        let should_append_history = if is_klondike_controls_hint {
            if imp.klondike_controls_history_logged.get() {
                false
            } else {
                imp.klondike_controls_history_logged.set(true);
                true
            }
        } else {
            true
        };
        if should_append_history {
            self.append_status_history_only(&sanitized);
        }
        imp.status_label.set_label(&sanitized);
        if self.status_performance_mode_active() {
            imp.status_label.set_tooltip_text(None);
        } else {
            imp.status_label.set_tooltip_text(Some(&sanitized));
        }
    }

    fn status_history_window_size(&self) -> (i32, i32) {
        const DEFAULT_WIDTH: i32 = 760;
        const DEFAULT_HEIGHT: i32 = 420;
        const MIN_WIDTH: i32 = 360;
        const MIN_HEIGHT: i32 = 240;

        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        };
        let Some(schema) = settings.settings_schema() else {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        };
        if !schema.has_key(SETTINGS_KEY_STATUS_HISTORY_WIDTH)
            || !schema.has_key(SETTINGS_KEY_STATUS_HISTORY_HEIGHT)
        {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        }

        (
            settings
                .int(SETTINGS_KEY_STATUS_HISTORY_WIDTH)
                .max(MIN_WIDTH),
            settings
                .int(SETTINGS_KEY_STATUS_HISTORY_HEIGHT)
                .max(MIN_HEIGHT),
        )
    }

    fn status_history_maximized(&self) -> bool {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return false;
        };
        let Some(schema) = settings.settings_schema() else {
            return false;
        };
        if !schema.has_key(SETTINGS_KEY_STATUS_HISTORY_MAXIMIZED) {
            return false;
        }
        settings.boolean(SETTINGS_KEY_STATUS_HISTORY_MAXIMIZED)
    }

    fn persist_status_history_maximized(&self, maximized: bool) {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_STATUS_HISTORY_MAXIMIZED) {
            return;
        }
        if settings.boolean(SETTINGS_KEY_STATUS_HISTORY_MAXIMIZED) != maximized {
            let _ = settings.set_boolean(SETTINGS_KEY_STATUS_HISTORY_MAXIMIZED, maximized);
        }
    }

    fn persist_status_history_window_size(&self, dialog: &gtk::Window) {
        const MIN_WIDTH: i32 = 360;
        const MIN_HEIGHT: i32 = 240;

        if dialog.is_maximized() {
            return;
        }

        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_STATUS_HISTORY_WIDTH)
            || !schema.has_key(SETTINGS_KEY_STATUS_HISTORY_HEIGHT)
        {
            return;
        }

        let width = dialog.width().max(MIN_WIDTH);
        let height = dialog.height().max(MIN_HEIGHT);
        if settings.int(SETTINGS_KEY_STATUS_HISTORY_WIDTH) != width {
            let _ = settings.set_int(SETTINGS_KEY_STATUS_HISTORY_WIDTH, width);
        }
        if settings.int(SETTINGS_KEY_STATUS_HISTORY_HEIGHT) != height {
            let _ = settings.set_int(SETTINGS_KEY_STATUS_HISTORY_HEIGHT, height);
        }
    }

    fn status_history_find_match_offsets(haystack: &str, query: &str) -> Vec<(i32, i32)> {
        let trimmed_query = query.trim();
        if trimmed_query.is_empty() {
            return Vec::new();
        }

        let haystack_lower = haystack.to_ascii_lowercase();
        let query_lower = trimmed_query.to_ascii_lowercase();
        if query_lower.is_empty() {
            return Vec::new();
        }

        let mut out = Vec::new();
        let mut search_start = 0usize;
        while search_start <= haystack_lower.len() {
            let Some(found_rel) = haystack_lower[search_start..].find(&query_lower) else {
                break;
            };
            let byte_start = search_start + found_rel;
            let byte_end = byte_start.saturating_add(query_lower.len());
            let start_chars = haystack_lower[..byte_start].chars().count() as i32;
            let end_chars = haystack_lower[..byte_end].chars().count() as i32;
            out.push((start_chars, end_chars));
            search_start = byte_start.saturating_add(query_lower.len().max(1));
        }

        out
    }

    fn apply_status_history_find_selection(
        buffer: &gtk::TextBuffer,
        text_view: &impl glib::object::IsA<gtk::TextView>,
        matches: &[(i32, i32)],
        current_index: Option<usize>,
        counter: &gtk::Label,
    ) {
        if matches.is_empty() {
            let start = buffer.start_iter();
            buffer.select_range(&start, &start);
            counter.set_label("0/0");
            return;
        }

        let index = current_index
            .unwrap_or(0)
            .min(matches.len().saturating_sub(1));
        let (start_offset, end_offset) = matches[index];
        let mut start_iter = buffer.iter_at_offset(start_offset);
        let end_iter = buffer.iter_at_offset(end_offset);
        buffer.select_range(&start_iter, &end_iter);
        let _ = text_view
            .as_ref()
            .scroll_to_iter(&mut start_iter, 0.2, false, 0.0, 0.0);
        counter.set_label(&format!("{}/{}", index.saturating_add(1), matches.len()));
    }

    fn status_history_text_counts(text: &str) -> (usize, usize) {
        let line_count = if text.is_empty() {
            0
        } else {
            text.chars()
                .filter(|ch| *ch == '\n')
                .count()
                .saturating_add(1)
        };
        let char_count = text.chars().count();
        (line_count, char_count)
    }

    fn update_status_history_counts_label(label: &gtk::Label, text: &str) {
        let (lines, chars) = Self::status_history_text_counts(text);
        label.set_label(&format!("Lines: {lines}  Chars: {chars}"));
    }

    pub(super) fn show_insert_note_dialog(&self, parent: Option<&gtk::Window>) {
        let transient_parent = parent
            .cloned()
            .unwrap_or_else(|| self.clone().upcast::<gtk::Window>());
        let note_dialog = gtk::Window::builder()
            .title("Insert Note")
            .modal(false)
            .default_width(420)
            .default_height(220)
            .transient_for(&transient_parent)
            .destroy_with_parent(true)
            .build();
        note_dialog.set_resizable(true);

        let note_root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        note_root.set_margin_top(10);
        note_root.set_margin_bottom(10);
        note_root.set_margin_start(10);
        note_root.set_margin_end(10);

        let note_hint = gtk::Label::new(Some("Type a note to append to status history."));
        note_hint.set_xalign(0.0);
        note_hint.add_css_class("dim-label");
        note_root.append(&note_hint);

        let note_scroller = gtk::ScrolledWindow::new();
        note_scroller.set_hexpand(true);
        note_scroller.set_vexpand(true);
        note_scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        let note_buffer = gtk::TextBuffer::new(None::<&gtk::TextTagTable>);
        let note_view = gtk::TextView::with_buffer(&note_buffer);
        note_buffer.set_text("Note: ");
        let end = note_buffer.end_iter();
        note_buffer.place_cursor(&end);
        note_view.set_wrap_mode(gtk::WrapMode::WordChar);
        note_scroller.set_child(Some(&note_view));
        note_root.append(&note_scroller);

        let note_actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        note_actions.set_halign(gtk::Align::End);
        let cancel = gtk::Button::with_label("Cancel");
        let save = gtk::Button::with_label("Save");
        note_actions.append(&cancel);
        note_actions.append(&save);
        note_root.append(&note_actions);

        cancel.connect_clicked(glib::clone!(
            #[weak]
            note_dialog,
            move |_| {
                note_dialog.close();
            }
        ));
        save.connect_clicked(glib::clone!(
            #[weak(rename_to = main_window)]
            self,
            #[weak]
            note_dialog,
            #[weak]
            note_buffer,
            move |_| {
                let raw = note_buffer
                    .text(&note_buffer.start_iter(), &note_buffer.end_iter(), true)
                    .to_string();
                let note = raw.trim();
                if note.is_empty() {
                    note_dialog.close();
                    return;
                }
                main_window.append_status_line(note);
                main_window.mark_session_dirty();
                note_dialog.close();
            }
        ));

        let note_keys = gtk::EventControllerKey::new();
        note_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        note_keys.connect_key_pressed(glib::clone!(
            #[weak]
            save,
            #[weak]
            cancel,
            #[weak]
            note_buffer,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, state| {
                if key == gdk::Key::Escape {
                    cancel.emit_clicked();
                    return glib::Propagation::Stop;
                }
                if key == gdk::Key::Return || key == gdk::Key::KP_Enter {
                    if state.contains(gdk::ModifierType::CONTROL_MASK) {
                        note_buffer.insert_at_cursor("\n");
                    } else {
                        save.emit_clicked();
                    }
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        note_dialog.add_controller(note_keys);

        note_dialog.set_child(Some(&note_root));
        note_dialog.present();
        note_view.grab_focus();
    }

    pub(super) fn show_status_history_dialog(&self) {
        if let Some(existing) = self.imp().status_history_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let joined = self.apply_status_history_retention_limit();
        let (saved_width, saved_height) = self.status_history_window_size();
        let saved_maximized = self.status_history_maximized();

        let dialog = gtk::Window::builder()
            .title("Status History")
            .modal(false)
            .default_width(saved_width)
            .default_height(saved_height)
            .transient_for(self)
            .build();
        dialog.set_resizable(true);
        dialog.set_deletable(true);
        dialog.set_hide_on_close(false);
        dialog.set_destroy_with_parent(true);
        dialog.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |dialog| {
                let maximized = dialog.is_maximized();
                window.persist_status_history_maximized(maximized);
                window.persist_status_history_window_size(dialog);
                let imp = window.imp();
                *imp.status_history_dialog.borrow_mut() = None;
                *imp.status_history_buffer.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));
        if saved_maximized {
            dialog.maximize();
        }
        let dialog_keys = gtk::EventControllerKey::new();
        dialog_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        dialog_keys.connect_key_pressed(glib::clone!(
            #[weak]
            dialog,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    dialog.close();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        dialog.add_controller(dialog_keys);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.set_margin_top(10);
        root.set_margin_bottom(10);
        root.set_margin_start(10);
        root.set_margin_end(10);

        let find_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        let find_entry = gtk::SearchEntry::new();
        find_entry.set_hexpand(true);
        find_entry.set_placeholder_text(Some("Find in history..."));
        let find_prev = gtk::Button::with_label("Prev");
        find_prev.add_css_class("flat");
        let find_next = gtk::Button::with_label("Next");
        find_next.add_css_class("flat");
        let find_result = gtk::Label::new(Some("0/0"));
        find_result.add_css_class("dim-label");
        find_result.set_xalign(1.0);
        find_result.set_width_chars(8);
        find_row.append(&find_entry);
        find_row.append(&find_prev);
        find_row.append(&find_next);
        find_row.append(&find_result);
        root.append(&find_row);

        let scroller = gtk::ScrolledWindow::new();
        scroller.set_hexpand(true);
        scroller.set_vexpand(true);
        scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        let source_buffer = sourceview5::Buffer::new(None::<&gtk::TextTagTable>);
        let text = sourceview5::View::with_buffer(&source_buffer);
        text.set_editable(false);
        text.set_cursor_visible(false);
        text.set_monospace(true);
        text.set_wrap_mode(gtk::WrapMode::WordChar);
        text.set_show_line_numbers(true);
        let scheme_manager = sourceview5::StyleSchemeManager::new();
        if let Some(scheme) = scheme_manager
            .scheme("Adwaita-dark")
            .or_else(|| scheme_manager.scheme("classic-dark"))
            .or_else(|| scheme_manager.scheme("oblivion"))
        {
            source_buffer.set_style_scheme(Some(&scheme));
        }
        source_buffer.set_text(&joined);
        let buffer: gtk::TextBuffer = source_buffer.upcast();
        *self.imp().status_history_buffer.borrow_mut() = Some(buffer.clone());
        scroller.set_child(Some(&text));
        root.append(&scroller);

        let find_matches = Rc::new(RefCell::new(Vec::<(i32, i32)>::new()));
        let find_match_index = Rc::new(Cell::new(None::<usize>));
        let history_counts = gtk::Label::new(None);
        history_counts.set_hexpand(false);
        history_counts.set_halign(gtk::Align::End);
        history_counts.set_xalign(1.0);
        history_counts.add_css_class("dim-label");
        Self::update_status_history_counts_label(&history_counts, &joined);

        find_entry.connect_search_changed(glib::clone!(
            #[weak]
            buffer,
            #[weak]
            text,
            #[weak]
            find_result,
            #[strong]
            find_matches,
            #[strong]
            find_match_index,
            move |entry| {
                let query = entry.text().to_string();
                let haystack = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string();
                let matches = Self::status_history_find_match_offsets(&haystack, &query);
                let index = if matches.is_empty() { None } else { Some(0) };
                *find_matches.borrow_mut() = matches;
                find_match_index.set(index);
                let matches_ref = find_matches.borrow();
                Self::apply_status_history_find_selection(
                    &buffer,
                    &text,
                    matches_ref.as_slice(),
                    find_match_index.get(),
                    &find_result,
                );
            }
        ));

        find_next.connect_clicked(glib::clone!(
            #[weak]
            buffer,
            #[weak]
            text,
            #[weak]
            find_result,
            #[strong]
            find_matches,
            #[strong]
            find_match_index,
            move |_| {
                let len = find_matches.borrow().len();
                let next = if len == 0 {
                    None
                } else {
                    Some(
                        find_match_index
                            .get()
                            .map(|index| (index + 1) % len)
                            .unwrap_or(0),
                    )
                };
                find_match_index.set(next);
                let matches_ref = find_matches.borrow();
                Self::apply_status_history_find_selection(
                    &buffer,
                    &text,
                    matches_ref.as_slice(),
                    find_match_index.get(),
                    &find_result,
                );
            }
        ));

        find_prev.connect_clicked(glib::clone!(
            #[weak]
            buffer,
            #[weak]
            text,
            #[weak]
            find_result,
            #[strong]
            find_matches,
            #[strong]
            find_match_index,
            move |_| {
                let len = find_matches.borrow().len();
                let previous = if len == 0 {
                    None
                } else {
                    Some(
                        find_match_index
                            .get()
                            .map(|index| {
                                if index == 0 {
                                    len.saturating_sub(1)
                                } else {
                                    index.saturating_sub(1)
                                }
                            })
                            .unwrap_or(0),
                    )
                };
                find_match_index.set(previous);
                let matches_ref = find_matches.borrow();
                Self::apply_status_history_find_selection(
                    &buffer,
                    &text,
                    matches_ref.as_slice(),
                    find_match_index.get(),
                    &find_result,
                );
            }
        ));

        find_entry.connect_activate(glib::clone!(
            #[weak]
            find_next,
            move |_| {
                find_next.emit_clicked();
            }
        ));

        buffer.connect_changed(glib::clone!(
            #[weak]
            find_entry,
            #[weak]
            text,
            #[weak]
            find_result,
            #[weak]
            history_counts,
            #[strong]
            find_matches,
            #[strong]
            find_match_index,
            move |buf| {
                let query = find_entry.text().to_string();
                let haystack = buf
                    .text(&buf.start_iter(), &buf.end_iter(), true)
                    .to_string();
                let matches = Self::status_history_find_match_offsets(&haystack, &query);
                let next_index = if matches.is_empty() {
                    None
                } else {
                    Some(
                        find_match_index
                            .get()
                            .unwrap_or(0)
                            .min(matches.len().saturating_sub(1)),
                    )
                };
                *find_matches.borrow_mut() = matches;
                find_match_index.set(next_index);
                let matches_ref = find_matches.borrow();
                Self::apply_status_history_find_selection(
                    buf,
                    &text,
                    matches_ref.as_slice(),
                    find_match_index.get(),
                    &find_result,
                );
                Self::update_status_history_counts_label(&history_counts, &haystack);
            }
        ));

        let actions_top = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        actions_top.set_hexpand(false);
        actions_top.set_halign(gtk::Align::End);
        actions_top.append(&history_counts);

        let retention_label = gtk::Label::new(Some("Keep"));
        retention_label.add_css_class("dim-label");
        actions_top.append(&retention_label);

        let retention_values = Rc::new(
            STATUS_HISTORY_RETENTION_CHOICES
                .iter()
                .map(|(value, _)| *value)
                .collect::<Vec<_>>(),
        );
        let retention_labels = STATUS_HISTORY_RETENTION_CHOICES
            .iter()
            .map(|(_, label)| format!("{label} lines"))
            .collect::<Vec<_>>();
        let retention_label_refs = retention_labels
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        let retention_dropdown = gtk::DropDown::from_strings(&retention_label_refs);
        let current_retention = self.status_history_retention_limit();
        let selected_index = retention_values
            .iter()
            .position(|value| *value == current_retention)
            .unwrap_or_else(|| {
                retention_values
                    .iter()
                    .position(|value| *value == STATUS_HISTORY_DEFAULT_RETENTION_LINES)
                    .unwrap_or(0)
            });
        retention_dropdown.set_selected(selected_index as u32);
        retention_dropdown.set_tooltip_text(Some("Maximum history lines kept in memory."));
        actions_top.append(&retention_dropdown);

        retention_dropdown.connect_selected_notify(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            #[weak]
            text,
            #[weak]
            find_entry,
            #[weak]
            find_result,
            #[weak]
            history_counts,
            #[strong]
            retention_values,
            #[strong]
            find_matches,
            #[strong]
            find_match_index,
            move |dropdown| {
                let index = dropdown.selected() as usize;
                let Some(limit) = retention_values.get(index).copied() else {
                    return;
                };

                window.persist_status_history_retention_limit(limit);
                let haystack = window.apply_status_history_retention_limit();
                buffer.set_text(&haystack);

                let query = find_entry.text().to_string();
                let matches = Self::status_history_find_match_offsets(&haystack, &query);
                let next_index = if matches.is_empty() {
                    None
                } else {
                    Some(
                        find_match_index
                            .get()
                            .unwrap_or(0)
                            .min(matches.len().saturating_sub(1)),
                    )
                };
                *find_matches.borrow_mut() = matches;
                find_match_index.set(next_index);

                let matches_ref = find_matches.borrow();
                Self::apply_status_history_find_selection(
                    &buffer,
                    &text,
                    matches_ref.as_slice(),
                    find_match_index.get(),
                    &find_result,
                );
                Self::update_status_history_counts_label(&history_counts, &haystack);
            }
        ));

        let actions_bottom = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        actions_bottom.set_hexpand(true);
        actions_bottom.set_halign(gtk::Align::End);
        const STATUS_HISTORY_ACTION_BUTTON_WIDTH: i32 = 84;

        let clear = gtk::Button::with_label("Clear");
        clear.set_width_request(STATUS_HISTORY_ACTION_BUTTON_WIDTH);
        clear.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            move |_| {
                let imp = window.imp();
                imp.status_history.borrow_mut().clear();
                imp.status_last_appended.borrow_mut().clear();
                imp.klondike_controls_history_logged.set(false);
                imp.status_label.set_label("");
                imp.status_label.set_tooltip_text(None);
                buffer.set_text("");
                window.mark_session_dirty();
            }
        ));
        actions_bottom.append(&clear);

        let copy = gtk::Button::with_label("Copy All");
        copy.set_width_request(STATUS_HISTORY_ACTION_BUTTON_WIDTH);
        copy.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            buffer,
            move |_| {
                let payload = buffer
                    .text(&buffer.start_iter(), &buffer.end_iter(), true)
                    .to_string();
                window.clipboard().set_text(&payload);
                *window.imp().status_override.borrow_mut() =
                    Some("Copied full status history to clipboard.".to_string());
                window.render();
            }
        ));
        actions_bottom.append(&copy);

        let insert_note = gtk::Button::with_label("Insert Note");
        insert_note.set_width_request(104);
        insert_note.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            move |_| {
                window.show_insert_note_dialog(Some(&dialog));
            }
        ));
        actions_bottom.append(&insert_note);

        let close = gtk::Button::with_label("Close");
        close.set_width_request(STATUS_HISTORY_ACTION_BUTTON_WIDTH);
        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        actions_bottom.append(&close);
        root.append(&actions_top);
        root.append(&actions_bottom);

        dialog.set_child(Some(&root));
        *self.imp().status_history_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }
}

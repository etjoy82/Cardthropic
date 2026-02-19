/* window.rs
 *
 * Copyright 2026 emviolet
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet, VecDeque};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gdk_pixbuf, gio, glib};

use crate::deck::AngloDeck;
use crate::engine::automation::AutomationProfile;
use crate::engine::freecell_planner::FreecellPlannerAction;
use crate::engine::hinting::{HintNode, HintSuggestion};
use crate::engine::keyboard_nav::KeyboardTarget;
use crate::engine::loss_analysis::LossVerdict;
use crate::engine::moves::{apply_hint_move_to_game, map_solver_line_to_hint_line, HintMove};
use crate::engine::robot::RobotPlayback;
use crate::engine::seed_history::SeedHistoryStore;
use crate::engine::variant::variant_for_mode;
use crate::engine::variant_engine::engine_for_mode;
use crate::engine::variant_state::VariantStateStore;
use crate::game::{
    Card, DrawMode, FreecellCardCountMode, GameMode, KlondikeGame, SolverMove, SpiderSuitMode, Suit,
};
use crate::startup_trace;
use crate::winnability;

#[path = "window/actions_history.rs"]
mod actions_history;
#[path = "window/actions_moves.rs"]
mod actions_moves;
#[path = "window/actions_selection.rs"]
mod actions_selection;
#[path = "window/ai.rs"]
mod ai;
#[path = "window/ai_winnability_check.rs"]
mod ai_winnability_check;
#[path = "window/dialogs_apm.rs"]
mod dialogs_apm;
#[path = "window/dialogs_command_search.rs"]
mod dialogs_command_search;
#[path = "window/dialogs_help.rs"]
mod dialogs_help;
#[path = "window/drag.rs"]
mod drag;
#[path = "window/drag_setup.rs"]
mod drag_setup;
#[path = "window/foundation_slots.rs"]
mod foundation_slots;
#[path = "window/handlers.rs"]
mod handlers;
#[path = "window/handlers_actions.rs"]
mod handlers_actions;
#[path = "window/handlers_geometry.rs"]
mod handlers_geometry;
#[path = "window/heuristics.rs"]
mod heuristics;
#[path = "window/hint_autoplay.rs"]
mod hint_autoplay;
#[path = "window/hint_core.rs"]
mod hint_core;
#[path = "window/hint_smart_move.rs"]
mod hint_smart_move;
#[path = "window/hint_suggestion.rs"]
mod hint_suggestion;
#[path = "window/hints.rs"]
mod hints;
#[path = "window/input.rs"]
mod input;
#[path = "window/layout.rs"]
mod layout;
#[path = "window/memory.rs"]
mod memory;
#[path = "window/menu.rs"]
mod menu;
#[path = "window/motion.rs"]
mod motion;
#[path = "window/overflow_hints.rs"]
mod overflow_hints;
#[path = "window/parsing.rs"]
mod parsing;
#[path = "window/render.rs"]
mod render;
#[path = "window/render_stock_waste_foundation.rs"]
mod render_stock_waste_foundation;
#[path = "window/render_tableau.rs"]
mod render_tableau;
#[path = "window/robot.rs"]
mod robot;
#[path = "window/seed.rs"]
mod seed;
#[path = "window/seed_history.rs"]
mod seed_history;
#[path = "window/seed_input.rs"]
mod seed_input;
#[path = "window/session.rs"]
mod session;
#[path = "window/state.rs"]
mod state;
#[path = "window/theme_color.rs"]
mod theme_color;
#[path = "window/theme_core.rs"]
mod theme_core;
#[path = "window/theme_menu.rs"]
mod theme_menu;
#[path = "window/theme_userstyle.rs"]
mod theme_userstyle;
#[path = "window/types.rs"]
mod types;
#[path = "window/variant_flow.rs"]
mod variant_flow;

use heuristics::*;
use parsing::*;
use types::*;

#[allow(deprecated)]
mod imp {
    use super::*;

    #[allow(deprecated)]
    #[derive(Debug, gtk::CompositeTemplate)]
    #[template(resource = "/io/codeberg/emviolet/cardthropic/window.ui")]
    pub struct CardthropicWindow {
        #[template_child]
        pub hud_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub undo_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub redo_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub auto_hint_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub cyclone_shuffle_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub peek_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub robot_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub stock_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub stock_column_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub waste_overlay: TemplateChild<gtk::Overlay>,
        #[template_child]
        pub waste_column_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub waste_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub waste_picture_1: TemplateChild<gtk::Picture>,
        #[template_child]
        pub waste_picture_2: TemplateChild<gtk::Picture>,
        #[template_child]
        pub waste_picture_3: TemplateChild<gtk::Picture>,
        #[template_child]
        pub waste_picture_4: TemplateChild<gtk::Picture>,
        #[template_child]
        pub waste_picture_5: TemplateChild<gtk::Picture>,
        #[template_child]
        pub waste_placeholder_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub waste_placeholder_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub stock_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub waste_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub stock_heading_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub stock_heading_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub waste_heading_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub top_row_spacer_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub waste_heading_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundations_heading_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub foundations_heading_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub stock_waste_foundation_spacer_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub foundations_area_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub foundation_picture_1: TemplateChild<gtk::Picture>,
        #[template_child]
        pub foundation_placeholder_1: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_picture_2: TemplateChild<gtk::Picture>,
        #[template_child]
        pub foundation_placeholder_2: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_picture_3: TemplateChild<gtk::Picture>,
        #[template_child]
        pub foundation_placeholder_3: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_picture_4: TemplateChild<gtk::Picture>,
        #[template_child]
        pub foundation_placeholder_4: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_picture_5: TemplateChild<gtk::Picture>,
        #[template_child]
        pub foundation_placeholder_5: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_picture_6: TemplateChild<gtk::Picture>,
        #[template_child]
        pub foundation_placeholder_6: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_picture_7: TemplateChild<gtk::Picture>,
        #[template_child]
        pub foundation_placeholder_7: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_picture_8: TemplateChild<gtk::Picture>,
        #[template_child]
        pub foundation_placeholder_8: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_label_1: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_label_2: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_label_3: TemplateChild<gtk::Label>,
        #[template_child]
        pub foundation_label_4: TemplateChild<gtk::Label>,
        #[template_child]
        pub status_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub status_history_button: TemplateChild<gtk::Button>,
        #[template_child]
        #[allow(deprecated)]
        pub seed_combo: TemplateChild<gtk::ComboBoxText>,
        #[template_child]
        pub seed_random_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub seed_rescue_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub seed_winnable_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub seed_repeat_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub seed_go_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub seed_controls_row: TemplateChild<gtk::Box>,
        #[template_child]
        pub stats_label: TemplateChild<gtk::Label>,
        #[template_child]
        pub status_block_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub tableau_stack_1: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_stack_2: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_stack_3: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_stack_4: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_stack_5: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_stack_6: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_stack_7: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_stack_8: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_stack_9: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_stack_10: TemplateChild<gtk::Fixed>,
        #[template_child]
        pub tableau_scroller: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub tableau_row: TemplateChild<gtk::Box>,
        #[template_child]
        pub main_menu_popover: TemplateChild<gtk::PopoverMenu>,
        #[template_child]
        pub board_color_menu_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub game_settings_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub main_menu_button: TemplateChild<gtk::MenuButton>,
        #[template_child]
        pub game_settings_popover: TemplateChild<gtk::Popover>,
        #[template_child]
        pub game_settings_content_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub top_playfield_frame: TemplateChild<gtk::Frame>,
        #[template_child]
        pub playfield_inner_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub top_heading_row_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub stock_waste_foundations_row_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub tableau_frame: TemplateChild<gtk::Frame>,
        #[template_child]
        pub board_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub toolbar_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub motion_layer: TemplateChild<gtk::Fixed>,
        pub game: RefCell<VariantStateStore>,
        pub current_seed: Cell<u64>,
        pub current_seed_win_recorded: Cell<bool>,
        pub(super) seed_history: RefCell<SeedHistoryStore>,
        pub seed_history_dirty: Cell<bool>,
        pub seed_history_dropdown_dirty: Cell<bool>,
        pub seed_history_flush_timer: RefCell<Option<glib::SourceId>>,
        pub(super) selected_run: RefCell<Option<SelectedRun>>,
        pub(super) selected_freecell: Cell<Option<usize>>,
        pub waste_selected: Cell<bool>,
        pub settings: RefCell<Option<gio::Settings>>,
        pub shared_state_persistence_owner: Cell<bool>,
        pub last_saved_session: RefCell<String>,
        pub session_dirty: Cell<bool>,
        pub session_flush_timer: RefCell<Option<glib::SourceId>>,
        pub board_color_hex: RefCell<String>,
        pub board_color_preview: RefCell<Option<gtk::DrawingArea>>,
        pub board_color_swatches: RefCell<Vec<gtk::DrawingArea>>,
        pub board_color_provider: RefCell<Option<gtk::CssProvider>>,
        pub interface_font_provider: RefCell<Option<gtk::CssProvider>>,
        pub custom_userstyle_css: RefCell<String>,
        pub saved_custom_userstyle_css: RefCell<String>,
        pub custom_userstyle_provider: RefCell<Option<gtk::CssProvider>>,
        pub custom_userstyle_dialog: RefCell<Option<gtk::Window>>,
        pub theme_presets_window: RefCell<Option<gtk::Window>>,
        pub status_history: RefCell<VecDeque<String>>,
        pub status_last_appended: RefCell<String>,
        pub layout_debug_last_appended: RefCell<String>,
        pub status_history_dialog: RefCell<Option<gtk::Window>>,
        pub status_history_buffer: RefCell<Option<gtk::TextBuffer>>,
        pub deck: RefCell<Option<AngloDeck>>,
        pub deck_load_attempted: Cell<bool>,
        pub deck_load_in_progress: Cell<bool>,
        pub deck_error: RefCell<Option<String>>,
        pub status_override: RefCell<Option<String>>,
        pub status_ephemeral_timer: RefCell<Option<glib::SourceId>>,
        pub history: RefCell<Vec<Snapshot>>,
        pub future: RefCell<Vec<Snapshot>>,
        pub(super) apm_samples: RefCell<Vec<ApmSample>>,
        pub apm_elapsed_offset_seconds: Cell<u32>,
        pub move_count: Cell<u32>,
        pub elapsed_seconds: Cell<u32>,
        pub timer_started: Cell<bool>,
        pub style_provider: RefCell<Option<gtk::CssProvider>>,
        pub card_width: Cell<i32>,
        pub card_height: Cell<i32>,
        pub face_up_step: Cell<i32>,
        pub face_down_step: Cell<i32>,
        pub foundation_slot_suits: RefCell<[Option<Suit>; 4]>,
        pub observed_window_width: Cell<i32>,
        pub observed_window_height: Cell<i32>,
        pub observed_scroller_width: Cell<i32>,
        pub observed_scroller_height: Cell<i32>,
        pub mobile_phone_mode: Cell<bool>,
        pub observed_maximized: Cell<bool>,
        pub geometry_render_pending: Cell<bool>,
        pub geometry_render_dirty: Cell<bool>,
        pub perf_resize_event_count: Cell<u64>,
        pub perf_resize_from_poll_count: Cell<u64>,
        pub perf_resize_from_notify_width_count: Cell<u64>,
        pub perf_resize_from_notify_height_count: Cell<u64>,
        pub perf_resize_from_notify_maximized_count: Cell<u64>,
        pub perf_geometry_render_count: Cell<u64>,
        pub perf_render_count: Cell<u64>,
        pub perf_render_total_us: Cell<u64>,
        pub perf_render_max_us: Cell<u64>,
        pub perf_last_report_mono_us: Cell<i64>,
        pub perf_last_report_resize_events: Cell<u64>,
        pub perf_last_report_resize_from_poll: Cell<u64>,
        pub perf_last_report_resize_from_notify_width: Cell<u64>,
        pub perf_last_report_resize_from_notify_height: Cell<u64>,
        pub perf_last_report_resize_from_notify_maximized: Cell<u64>,
        pub perf_last_report_geometry_renders: Cell<u64>,
        pub perf_last_report_render_count: Cell<u64>,
        pub perf_last_report_render_total_us: Cell<u64>,
        pub perf_last_report_deck_hits: Cell<u64>,
        pub perf_last_report_deck_misses: Cell<u64>,
        pub perf_last_report_deck_inserts: Cell<u64>,
        pub perf_last_report_deck_clears: Cell<u64>,
        pub pending_deal_instructions: Cell<bool>,
        pub last_metrics_key: Cell<u64>,
        pub tableau_card_pictures: RefCell<Vec<Vec<gtk::Picture>>>,
        pub(super) tableau_picture_state_cache:
            RefCell<Vec<Vec<Option<TableauPictureRenderState>>>>,
        pub last_stock_waste_foundation_size: Cell<(i32, i32, GameMode)>,
        pub hint_timeouts: RefCell<Vec<glib::SourceId>>,
        pub hint_widgets: RefCell<Vec<gtk::Widget>>,
        pub hint_recent_states: RefCell<VecDeque<u64>>,
        pub seed_check_running: Cell<bool>,
        pub seed_check_generation: Cell<u64>,
        pub seed_check_cancel: RefCell<Option<Arc<AtomicBool>>>,
        pub seed_check_timer: RefCell<Option<glib::SourceId>>,
        pub seed_check_seconds: Cell<u32>,
        pub seed_check_memory_guard_triggered: Cell<bool>,
        pub seed_check_memory_limit_mib: Cell<u64>,
        pub memory_guard_enabled: Cell<bool>,
        pub memory_guard_soft_limit_mib: Cell<u64>,
        pub memory_guard_hard_limit_mib: Cell<u64>,
        pub memory_guard_soft_triggered: Cell<bool>,
        pub memory_guard_hard_triggered: Cell<bool>,
        pub memory_guard_last_dialog_mono_us: Cell<i64>,
        pub auto_play_seen_states: RefCell<HashSet<u64>>,
        pub auto_playing_move: Cell<bool>,
        pub(super) hint_loss_cache: RefCell<HashMap<u64, LossVerdict>>,
        pub hint_loss_analysis_running: Cell<bool>,
        pub hint_loss_analysis_hash: Cell<u64>,
        pub hint_loss_analysis_cancel: RefCell<Option<Arc<AtomicBool>>>,
        pub rapid_wand_running: Cell<bool>,
        pub rapid_wand_timer: RefCell<Option<glib::SourceId>>,
        pub rapid_wand_nonproductive_streak: Cell<u32>,
        pub rapid_wand_foundation_drought_streak: Cell<u32>,
        pub rapid_wand_blocked_state_hash: RefCell<Option<u64>>,
        pub robot_mode_running: Cell<bool>,
        pub robot_mode_timer: RefCell<Option<glib::SourceId>>,
        pub robot_forever_enabled: Cell<bool>,
        pub robot_auto_new_game_on_loss: Cell<bool>,
        pub robot_ludicrous_enabled: Cell<bool>,
        pub robot_strict_debug_invariants: Cell<bool>,
        pub robot_wins: Cell<u32>,
        pub robot_losses: Cell<u32>,
        pub robot_last_benchmark_dump_total: Cell<u32>,
        pub robot_deals_tried: Cell<u32>,
        pub robot_moves_applied: Cell<u32>,
        pub robot_seen_states: RefCell<HashSet<u64>>,
        pub robot_recent_hashes: RefCell<VecDeque<u64>>,
        pub robot_recent_action_signatures: RefCell<VecDeque<String>>,
        pub(super) robot_freecell_recent_fallback_hashes: RefCell<VecDeque<u64>>,
        pub(super) robot_freecell_recent_fallback_signatures: RefCell<VecDeque<String>>,
        pub(super) robot_freecell_fallback_only_streak: Cell<u32>,
        pub robot_last_move_signature: RefCell<Option<String>>,
        pub robot_inverse_oscillation_streak: Cell<u32>,
        pub robot_hash_oscillation_streak: Cell<u32>,
        pub robot_hash_oscillation_period: Cell<u8>,
        pub robot_action_cycle_streak: Cell<u32>,
        pub robot_force_loss_now: Cell<bool>,
        pub robot_stall_streak: Cell<u32>,
        pub robot_moves_since_foundation_progress: Cell<u32>,
        pub robot_last_foundation_like: Cell<u32>,
        pub robot_last_empty_cols: Cell<u32>,
        pub robot_last_freecell_mobility: Cell<u32>,
        pub robot_last_freecell_capacity: Cell<u32>,
        pub robot_freecell_t2f_moves: Cell<u32>,
        pub robot_freecell_c2f_moves: Cell<u32>,
        pub robot_freecell_t2t_moves: Cell<u32>,
        pub robot_freecell_t2c_moves: Cell<u32>,
        pub robot_freecell_c2t_moves: Cell<u32>,
        pub robot_freecell_peak_used: Cell<u32>,
        pub(super) robot_freecell_plan: RefCell<VecDeque<FreecellPlannerAction>>,
        pub(super) robot_freecell_playback: RefCell<RobotPlayback<FreecellPlannerAction>>,
        pub(super) robot_freecell_planner_running: Cell<bool>,
        pub(super) robot_freecell_planner_rx:
            RefCell<Option<mpsc::Receiver<crate::engine::freecell_planner::FreecellPlannerResult>>>,
        pub(super) robot_freecell_planner_cancel: RefCell<Option<Arc<AtomicBool>>>,
        pub(super) robot_freecell_planner_anchor_hash: Cell<u64>,
        pub(super) robot_freecell_planner_wait_ticks: Cell<u32>,
        pub(super) robot_freecell_no_move_ticks: Cell<u32>,
        pub(super) robot_freecell_planner_empty_streak: Cell<u32>,
        pub(super) robot_freecell_planner_cooldown_ticks: Cell<u32>,
        pub(super) robot_freecell_planner_restart_debounce_ticks: Cell<u32>,
        pub(super) robot_freecell_planner_last_start_marker: Cell<u64>,
        pub(super) robot_cpu_last_exec_ns: Cell<u64>,
        pub(super) robot_cpu_last_mono_us: Cell<i64>,
        pub(super) robot_cpu_last_pct: Cell<f64>,
        pub robot_debug_enabled: Cell<bool>,
        pub(super) robot_playback: RefCell<RobotPlayback<HintMove>>,
        pub(super) drag_origin: RefCell<Option<DragOrigin>>,
        pub drag_widgets: RefCell<Vec<gtk::Widget>>,
        pub drag_timeouts: RefCell<Vec<glib::SourceId>>,
        pub suppress_waste_click_once: Cell<bool>,
        pub seed_search_in_progress: Cell<bool>,
        pub seed_search_cancel: RefCell<Option<Arc<AtomicBool>>>,
        pub seed_combo_updating: Cell<bool>,
        pub smart_move_mode: Cell<SmartMoveMode>,
        pub hud_enabled: Cell<bool>,
        pub hud_auto_hidden: Cell<bool>,
        pub peek_active: Cell<bool>,
        pub peek_generation: Cell<u64>,
        pub current_game_mode: Cell<GameMode>,
        pub klondike_draw_mode: Cell<DrawMode>,
        pub spider_suit_mode: Cell<SpiderSuitMode>,
        pub freecell_card_count_mode: Cell<FreecellCardCountMode>,
        pub game_mode_buttons: RefCell<HashMap<GameMode, gtk::Button>>,
        pub help_dialog: RefCell<Option<gtk::Window>>,
        pub command_search_dialog: RefCell<Option<gtk::Window>>,
        pub command_search_filter_entry: RefCell<Option<gtk::SearchEntry>>,
        pub memory_guard_dialog: RefCell<Option<gtk::Window>>,
        pub apm_graph_dialog: RefCell<Option<gtk::Window>>,
        pub apm_graph_area: RefCell<Option<gtk::DrawingArea>>,
        pub apm_peak_label: RefCell<Option<gtk::Label>>,
        pub apm_avg_label: RefCell<Option<gtk::Label>>,
        pub apm_tilt_label: RefCell<Option<gtk::Label>>,
        pub keyboard_target: Cell<KeyboardTarget>,
        pub startup_first_map_logged: Cell<bool>,
        pub startup_deck_logged: Cell<bool>,
    }

    impl Default for CardthropicWindow {
        fn default() -> Self {
            let seed = crate::engine::seed_ops::random_seed();
            Self {
                hud_button: TemplateChild::default(),
                undo_button: TemplateChild::default(),
                redo_button: TemplateChild::default(),
                auto_hint_button: TemplateChild::default(),
                cyclone_shuffle_button: TemplateChild::default(),
                peek_button: TemplateChild::default(),
                robot_button: TemplateChild::default(),
                stock_picture: TemplateChild::default(),
                stock_column_box: TemplateChild::default(),
                waste_overlay: TemplateChild::default(),
                waste_column_box: TemplateChild::default(),
                waste_picture: TemplateChild::default(),
                waste_picture_1: TemplateChild::default(),
                waste_picture_2: TemplateChild::default(),
                waste_picture_3: TemplateChild::default(),
                waste_picture_4: TemplateChild::default(),
                waste_picture_5: TemplateChild::default(),
                waste_placeholder_box: TemplateChild::default(),
                waste_placeholder_label: TemplateChild::default(),
                stock_label: TemplateChild::default(),
                waste_label: TemplateChild::default(),
                stock_heading_box: TemplateChild::default(),
                stock_heading_label: TemplateChild::default(),
                waste_heading_box: TemplateChild::default(),
                top_row_spacer_box: TemplateChild::default(),
                waste_heading_label: TemplateChild::default(),
                foundations_heading_box: TemplateChild::default(),
                foundations_heading_label: TemplateChild::default(),
                stock_waste_foundation_spacer_box: TemplateChild::default(),
                foundations_area_box: TemplateChild::default(),
                foundation_picture_1: TemplateChild::default(),
                foundation_placeholder_1: TemplateChild::default(),
                foundation_picture_2: TemplateChild::default(),
                foundation_placeholder_2: TemplateChild::default(),
                foundation_picture_3: TemplateChild::default(),
                foundation_placeholder_3: TemplateChild::default(),
                foundation_picture_4: TemplateChild::default(),
                foundation_placeholder_4: TemplateChild::default(),
                foundation_picture_5: TemplateChild::default(),
                foundation_placeholder_5: TemplateChild::default(),
                foundation_picture_6: TemplateChild::default(),
                foundation_placeholder_6: TemplateChild::default(),
                foundation_picture_7: TemplateChild::default(),
                foundation_placeholder_7: TemplateChild::default(),
                foundation_picture_8: TemplateChild::default(),
                foundation_placeholder_8: TemplateChild::default(),
                foundation_label_1: TemplateChild::default(),
                foundation_label_2: TemplateChild::default(),
                foundation_label_3: TemplateChild::default(),
                foundation_label_4: TemplateChild::default(),
                status_label: TemplateChild::default(),
                status_history_button: TemplateChild::default(),
                seed_combo: TemplateChild::default(),
                seed_random_button: TemplateChild::default(),
                seed_rescue_button: TemplateChild::default(),
                seed_winnable_button: TemplateChild::default(),
                seed_repeat_button: TemplateChild::default(),
                seed_go_button: TemplateChild::default(),
                seed_controls_row: TemplateChild::default(),
                stats_label: TemplateChild::default(),
                status_block_box: TemplateChild::default(),
                tableau_stack_1: TemplateChild::default(),
                tableau_stack_2: TemplateChild::default(),
                tableau_stack_3: TemplateChild::default(),
                tableau_stack_4: TemplateChild::default(),
                tableau_stack_5: TemplateChild::default(),
                tableau_stack_6: TemplateChild::default(),
                tableau_stack_7: TemplateChild::default(),
                tableau_stack_8: TemplateChild::default(),
                tableau_stack_9: TemplateChild::default(),
                tableau_stack_10: TemplateChild::default(),
                tableau_scroller: TemplateChild::default(),
                tableau_row: TemplateChild::default(),
                main_menu_popover: TemplateChild::default(),
                board_color_menu_button: TemplateChild::default(),
                game_settings_menu_button: TemplateChild::default(),
                main_menu_button: TemplateChild::default(),
                game_settings_popover: TemplateChild::default(),
                game_settings_content_box: TemplateChild::default(),
                top_playfield_frame: TemplateChild::default(),
                playfield_inner_box: TemplateChild::default(),
                top_heading_row_box: TemplateChild::default(),
                stock_waste_foundations_row_box: TemplateChild::default(),
                tableau_frame: TemplateChild::default(),
                board_box: TemplateChild::default(),
                toolbar_box: TemplateChild::default(),
                motion_layer: TemplateChild::default(),
                game: RefCell::new(VariantStateStore::new(seed)),
                current_seed: Cell::new(seed),
                current_seed_win_recorded: Cell::new(false),
                seed_history: RefCell::new(SeedHistoryStore::default()),
                seed_history_dirty: Cell::new(false),
                seed_history_dropdown_dirty: Cell::new(false),
                seed_history_flush_timer: RefCell::new(None),
                selected_run: RefCell::new(None),
                selected_freecell: Cell::new(None),
                waste_selected: Cell::new(false),
                settings: RefCell::new(None),
                shared_state_persistence_owner: Cell::new(false),
                last_saved_session: RefCell::new(String::new()),
                session_dirty: Cell::new(false),
                session_flush_timer: RefCell::new(None),
                board_color_hex: RefCell::new(DEFAULT_BOARD_COLOR.to_string()),
                board_color_preview: RefCell::new(None),
                board_color_swatches: RefCell::new(Vec::new()),
                board_color_provider: RefCell::new(None),
                interface_font_provider: RefCell::new(None),
                custom_userstyle_css: RefCell::new(String::new()),
                saved_custom_userstyle_css: RefCell::new(String::new()),
                custom_userstyle_provider: RefCell::new(None),
                custom_userstyle_dialog: RefCell::new(None),
                theme_presets_window: RefCell::new(None),
                status_history: RefCell::new(VecDeque::new()),
                status_last_appended: RefCell::new(String::new()),
                layout_debug_last_appended: RefCell::new(String::new()),
                status_history_dialog: RefCell::new(None),
                status_history_buffer: RefCell::new(None),
                deck: RefCell::new(None),
                deck_load_attempted: Cell::new(false),
                deck_load_in_progress: Cell::new(false),
                deck_error: RefCell::new(None),
                status_override: RefCell::new(None),
                status_ephemeral_timer: RefCell::new(None),
                history: RefCell::new(Vec::new()),
                future: RefCell::new(Vec::new()),
                apm_samples: RefCell::new(Vec::new()),
                apm_elapsed_offset_seconds: Cell::new(0),
                move_count: Cell::new(0),
                elapsed_seconds: Cell::new(0),
                timer_started: Cell::new(false),
                style_provider: RefCell::new(None),
                card_width: Cell::new(70),
                card_height: Cell::new(108),
                face_up_step: Cell::new(28),
                face_down_step: Cell::new(14),
                foundation_slot_suits: RefCell::new([None, None, None, None]),
                observed_window_width: Cell::new(0),
                observed_window_height: Cell::new(0),
                observed_scroller_width: Cell::new(0),
                observed_scroller_height: Cell::new(0),
                mobile_phone_mode: Cell::new(false),
                observed_maximized: Cell::new(false),
                geometry_render_pending: Cell::new(false),
                geometry_render_dirty: Cell::new(false),
                perf_resize_event_count: Cell::new(0),
                perf_resize_from_poll_count: Cell::new(0),
                perf_resize_from_notify_width_count: Cell::new(0),
                perf_resize_from_notify_height_count: Cell::new(0),
                perf_resize_from_notify_maximized_count: Cell::new(0),
                perf_geometry_render_count: Cell::new(0),
                perf_render_count: Cell::new(0),
                perf_render_total_us: Cell::new(0),
                perf_render_max_us: Cell::new(0),
                perf_last_report_mono_us: Cell::new(0),
                perf_last_report_resize_events: Cell::new(0),
                perf_last_report_resize_from_poll: Cell::new(0),
                perf_last_report_resize_from_notify_width: Cell::new(0),
                perf_last_report_resize_from_notify_height: Cell::new(0),
                perf_last_report_resize_from_notify_maximized: Cell::new(0),
                perf_last_report_geometry_renders: Cell::new(0),
                perf_last_report_render_count: Cell::new(0),
                perf_last_report_render_total_us: Cell::new(0),
                perf_last_report_deck_hits: Cell::new(0),
                perf_last_report_deck_misses: Cell::new(0),
                perf_last_report_deck_inserts: Cell::new(0),
                perf_last_report_deck_clears: Cell::new(0),
                pending_deal_instructions: Cell::new(true),
                last_metrics_key: Cell::new(0),
                tableau_card_pictures: RefCell::new(vec![Vec::new(); 10]),
                tableau_picture_state_cache: RefCell::new(vec![Vec::new(); 10]),
                last_stock_waste_foundation_size: Cell::new((0, 0, GameMode::Klondike)),
                hint_timeouts: RefCell::new(Vec::new()),
                hint_widgets: RefCell::new(Vec::new()),
                hint_recent_states: RefCell::new(VecDeque::new()),
                seed_check_running: Cell::new(false),
                seed_check_generation: Cell::new(0),
                seed_check_cancel: RefCell::new(None),
                seed_check_timer: RefCell::new(None),
                seed_check_seconds: Cell::new(0),
                seed_check_memory_guard_triggered: Cell::new(false),
                seed_check_memory_limit_mib: Cell::new(0),
                memory_guard_enabled: Cell::new(false),
                memory_guard_soft_limit_mib: Cell::new(1536),
                memory_guard_hard_limit_mib: Cell::new(2048),
                memory_guard_soft_triggered: Cell::new(false),
                memory_guard_hard_triggered: Cell::new(false),
                memory_guard_last_dialog_mono_us: Cell::new(0),
                auto_play_seen_states: RefCell::new(HashSet::new()),
                auto_playing_move: Cell::new(false),
                hint_loss_cache: RefCell::new(HashMap::new()),
                hint_loss_analysis_running: Cell::new(false),
                hint_loss_analysis_hash: Cell::new(0),
                hint_loss_analysis_cancel: RefCell::new(None),
                rapid_wand_running: Cell::new(false),
                rapid_wand_timer: RefCell::new(None),
                rapid_wand_nonproductive_streak: Cell::new(0),
                rapid_wand_foundation_drought_streak: Cell::new(0),
                rapid_wand_blocked_state_hash: RefCell::new(None),
                robot_mode_running: Cell::new(false),
                robot_mode_timer: RefCell::new(None),
                robot_forever_enabled: Cell::new(false),
                robot_auto_new_game_on_loss: Cell::new(true),
                robot_ludicrous_enabled: Cell::new(false),
                robot_strict_debug_invariants: Cell::new(true),
                robot_wins: Cell::new(0),
                robot_losses: Cell::new(0),
                robot_last_benchmark_dump_total: Cell::new(0),
                robot_deals_tried: Cell::new(0),
                robot_moves_applied: Cell::new(0),
                robot_seen_states: RefCell::new(HashSet::new()),
                robot_recent_hashes: RefCell::new(VecDeque::new()),
                robot_recent_action_signatures: RefCell::new(VecDeque::new()),
                robot_freecell_recent_fallback_hashes: RefCell::new(VecDeque::new()),
                robot_freecell_recent_fallback_signatures: RefCell::new(VecDeque::new()),
                robot_freecell_fallback_only_streak: Cell::new(0),
                robot_last_move_signature: RefCell::new(None),
                robot_inverse_oscillation_streak: Cell::new(0),
                robot_hash_oscillation_streak: Cell::new(0),
                robot_hash_oscillation_period: Cell::new(0),
                robot_action_cycle_streak: Cell::new(0),
                robot_force_loss_now: Cell::new(false),
                robot_stall_streak: Cell::new(0),
                robot_moves_since_foundation_progress: Cell::new(0),
                robot_last_foundation_like: Cell::new(0),
                robot_last_empty_cols: Cell::new(0),
                robot_last_freecell_mobility: Cell::new(0),
                robot_last_freecell_capacity: Cell::new(0),
                robot_freecell_t2f_moves: Cell::new(0),
                robot_freecell_c2f_moves: Cell::new(0),
                robot_freecell_t2t_moves: Cell::new(0),
                robot_freecell_t2c_moves: Cell::new(0),
                robot_freecell_c2t_moves: Cell::new(0),
                robot_freecell_peak_used: Cell::new(0),
                robot_freecell_plan: RefCell::new(VecDeque::new()),
                robot_freecell_playback: RefCell::new(RobotPlayback::default()),
                robot_freecell_planner_running: Cell::new(false),
                robot_freecell_planner_rx: RefCell::new(None),
                robot_freecell_planner_cancel: RefCell::new(None),
                robot_freecell_planner_anchor_hash: Cell::new(0),
                robot_freecell_planner_wait_ticks: Cell::new(0),
                robot_freecell_no_move_ticks: Cell::new(0),
                robot_freecell_planner_empty_streak: Cell::new(0),
                robot_freecell_planner_cooldown_ticks: Cell::new(0),
                robot_freecell_planner_restart_debounce_ticks: Cell::new(0),
                robot_freecell_planner_last_start_marker: Cell::new(0),
                robot_cpu_last_exec_ns: Cell::new(0),
                robot_cpu_last_mono_us: Cell::new(0),
                robot_cpu_last_pct: Cell::new(0.0),
                robot_debug_enabled: Cell::new(false),
                robot_playback: RefCell::new(RobotPlayback::default()),
                drag_origin: RefCell::new(None),
                drag_widgets: RefCell::new(Vec::new()),
                drag_timeouts: RefCell::new(Vec::new()),
                suppress_waste_click_once: Cell::new(false),
                seed_search_in_progress: Cell::new(false),
                seed_search_cancel: RefCell::new(None),
                seed_combo_updating: Cell::new(false),
                smart_move_mode: Cell::new(SmartMoveMode::DoubleClick),
                hud_enabled: Cell::new(true),
                hud_auto_hidden: Cell::new(false),
                peek_active: Cell::new(false),
                peek_generation: Cell::new(0),
                current_game_mode: Cell::new(GameMode::Klondike),
                klondike_draw_mode: Cell::new(DrawMode::One),
                spider_suit_mode: Cell::new(SpiderSuitMode::One),
                freecell_card_count_mode: Cell::new(FreecellCardCountMode::FiftyTwo),
                game_mode_buttons: RefCell::new(HashMap::new()),
                help_dialog: RefCell::new(None),
                command_search_dialog: RefCell::new(None),
                command_search_filter_entry: RefCell::new(None),
                memory_guard_dialog: RefCell::new(None),
                apm_graph_dialog: RefCell::new(None),
                apm_graph_area: RefCell::new(None),
                apm_peak_label: RefCell::new(None),
                apm_avg_label: RefCell::new(None),
                apm_tilt_label: RefCell::new(None),
                keyboard_target: Cell::new(KeyboardTarget::Stock),
                startup_first_map_logged: Cell::new(false),
                startup_deck_logged: Cell::new(false),
            }
        }
    }

    #[glib::object_subclass]
    impl ObjectSubclass for CardthropicWindow {
        const NAME: &'static str = "CardthropicWindow";
        type Type = super::CardthropicWindow;
        type ParentType = adw::ApplicationWindow;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.install_action("win.new-game", None, |window, _, _| {
                window.start_random_seed_game();
            });
            klass.install_action("win.random-seed", None, |window, _, _| {
                window.start_random_seed_game();
            });
            klass.install_action("win.winnable-seed", None, |window, _, _| {
                window.start_random_winnable_seed_game();
            });
            klass.install_action("win.seed-picker", None, |window, _, _| {
                window.show_seed_picker_dialog();
            });
            klass.install_action("win.repeat-seed", None, |window, _, _| {
                window.repeat_current_seed_game();
            });
            klass.install_action("win.check-seed-winnable", None, |window, _, _| {
                window.toggle_seed_winnable_check();
            });
            klass.install_action("win.draw", None, |window, _, _| {
                window.draw_card();
            });
            klass.install_action("win.undo", None, |window, _, _| {
                window.undo();
            });
            klass.install_action("win.redo", None, |window, _, _| {
                window.redo();
            });
            klass.install_action("win.play-hint-move", None, |window, _, _| {
                window.play_hint_for_player();
            });
            klass.install_action("win.rapid-wand", None, |window, _, _| {
                window.trigger_rapid_wand();
            });
            klass.install_action("win.cyclone-shuffle", None, |window, _, _| {
                window.cyclone_shuffle_tableau();
            });
            klass.install_action("win.peek", None, |window, _, _| {
                window.trigger_peek();
            });
            klass.install_action("win.robot-mode", None, |window, _, _| {
                window.toggle_robot_mode();
            });
            klass.install_action("win.copy-game-state", None, |window, _, _| {
                window.copy_game_state_to_clipboard();
            });
            klass.install_action("win.paste-game-state", None, |window, _, _| {
                window.paste_game_state_from_clipboard();
            });
            klass.install_action("win.copy-benchmark-snapshot", None, |window, _, _| {
                window.copy_benchmark_snapshot();
            });
            klass.install_action("win.command-search", None, |window, _, _| {
                window.show_command_search_dialog();
            });
            klass.install_action("win.help", None, |window, _, _| {
                window.show_help_dialog();
            });
            klass.install_action("win.toggle-fullscreen", None, |window, _, _| {
                window.toggle_fullscreen_mode();
            });
            klass.install_action("win.apm-graph", None, |window, _, _| {
                window.show_apm_graph_dialog();
            });
            klass.install_action("win.status-history", None, |window, _, _| {
                window.show_status_history_dialog();
            });
            klass.install_action("win.open-theme-presets", None, |window, _, _| {
                window.show_theme_presets_window();
            });
            klass.install_action("win.mode-klondike", None, |window, _, _| {
                window.select_game_mode("klondike");
            });
            klass.install_action("win.mode-spider", None, |window, _, _| {
                window.select_game_mode("spider");
            });
            klass.install_action("win.mode-freecell", None, |window, _, _| {
                window.select_game_mode("freecell");
            });
            klass.install_action("win.mode-klondike-deal-1", None, |window, _, _| {
                window.select_klondike_draw_mode(DrawMode::One);
            });
            klass.install_action("win.mode-klondike-deal-2", None, |window, _, _| {
                window.select_klondike_draw_mode(DrawMode::Two);
            });
            klass.install_action("win.mode-klondike-deal-3", None, |window, _, _| {
                window.select_klondike_draw_mode(DrawMode::Three);
            });
            klass.install_action("win.mode-klondike-deal-4", None, |window, _, _| {
                window.select_klondike_draw_mode(DrawMode::Four);
            });
            klass.install_action("win.mode-klondike-deal-5", None, |window, _, _| {
                window.select_klondike_draw_mode(DrawMode::Five);
            });
            klass.install_action("win.mode-spider-suit-1", None, |window, _, _| {
                window.select_spider_suit_mode(SpiderSuitMode::One);
            });
            klass.install_action("win.mode-spider-suit-2", None, |window, _, _| {
                window.select_spider_suit_mode(SpiderSuitMode::Two);
            });
            klass.install_action("win.mode-spider-suit-3", None, |window, _, _| {
                window.select_spider_suit_mode(SpiderSuitMode::Three);
            });
            klass.install_action("win.mode-spider-suit-4", None, |window, _, _| {
                window.select_spider_suit_mode(SpiderSuitMode::Four);
            });
            klass.install_action("win.mode-freecell-card-26", None, |window, _, _| {
                window.select_freecell_card_count_mode(FreecellCardCountMode::TwentySix);
            });
            klass.install_action("win.mode-freecell-card-39", None, |window, _, _| {
                window.select_freecell_card_count_mode(FreecellCardCountMode::ThirtyNine);
            });
            klass.install_action("win.mode-freecell-card-52", None, |window, _, _| {
                window.select_freecell_card_count_mode(FreecellCardCountMode::FiftyTwo);
            });
            klass.install_action("win.smart-move-double-click", None, |window, _, _| {
                window.set_smart_move_mode(SmartMoveMode::DoubleClick, true, true);
            });
            klass.install_action("win.smart-move-single-click", None, |window, _, _| {
                window.set_smart_move_mode(SmartMoveMode::SingleClick, true, true);
            });
            klass.install_action("win.smart-move-disabled", None, |window, _, _| {
                window.set_smart_move_mode(SmartMoveMode::Disabled, true, true);
            });
            klass.install_action("win.smart-move-right-click", None, |window, _, _| {
                window.set_smart_move_mode(SmartMoveMode::RightClick, true, true);
            });
            klass.install_action("win.mode-option-1", None, |window, _, _| {
                window.apply_mode_option_by_index(0);
            });
            klass.install_action("win.mode-option-2", None, |window, _, _| {
                window.apply_mode_option_by_index(1);
            });
            klass.install_action("win.mode-option-3", None, |window, _, _| {
                window.apply_mode_option_by_index(2);
            });
            klass.install_action("win.mode-option-4", None, |window, _, _| {
                window.apply_mode_option_by_index(3);
            });
            klass.install_action("win.mode-option-5", None, |window, _, _| {
                window.apply_mode_option_by_index(4);
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CardthropicWindow {
        fn constructed(&self) {
            self.parent_constructed();
            startup_trace::mark("window:constructed-enter");
            let obj = self.obj();
            obj.initialize_shared_state_persistence_owner();
            obj.connect_map(glib::clone!(
                #[weak(rename_to = window)]
                obj,
                move |_| {
                    if window.imp().startup_first_map_logged.replace(true) {
                        return;
                    }
                    startup_trace::mark("window:first-map");
                    startup_trace::print_summary_once();
                    window.append_startup_trace_history_if_robot_debug();
                }
            ));
            let icon_name = gdk::Display::default()
                .map(|display| {
                    let theme = gtk::IconTheme::for_display(&display);
                    // Flatpak app icons live under /app/share/icons.
                    theme.add_search_path("/app/share/icons");
                    theme.add_search_path("/app/share/pixmaps");
                    if theme.has_icon(APP_ICON_NAME) {
                        APP_ICON_NAME
                    } else if theme.has_icon(APP_ICON_FALLBACK_NAME) {
                        APP_ICON_FALLBACK_NAME
                    } else {
                        APP_ICON_NAME
                    }
                })
                .unwrap_or(APP_ICON_NAME);
            gtk::Window::set_default_icon_name(icon_name);
            obj.set_icon_name(Some(icon_name));
            obj.set_size_request(MIN_WINDOW_WIDTH, MIN_WINDOW_HEIGHT);
            obj.load_seed_history();
            obj.refresh_seed_history_dropdown();
            obj.setup_styles();
            obj.setup_hud_action();
            obj.setup_forever_mode_action();
            obj.setup_robot_auto_new_game_on_loss_action();
            obj.setup_ludicrous_speed_action();
            obj.setup_memory_guard_action();
            obj.setup_robot_debug_action();
            obj.setup_robot_strict_debug_invariants_action();
            obj.setup_board_color_preferences();
            startup_trace::mark("window:before-restore-session");
            let restored = obj.should_persist_shared_state() && obj.try_restore_saved_session();
            if !restored {
                obj.note_seed_play_started(self.current_seed.get());
                obj.set_seed_input_text(&self.current_seed.get().to_string());
            }
            startup_trace::mark("window:after-restore-session");
            obj.setup_handlers();
            startup_trace::mark("window:handlers-ready");
            obj.connect_close_request(glib::clone!(
                #[weak(rename_to = window)]
                obj,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_| {
                    window.close_auxiliary_windows();
                    window.flush_session_now();
                    window.flush_seed_history_now();
                    window.handoff_shared_state_persistence_owner_if_needed();
                    glib::Propagation::Proceed
                }
            ));
            obj.imp().tableau_row.set_homogeneous(true);
            obj.sync_mobile_phone_mode_to_size();
            obj.setup_timer();
            obj.render();
            obj.reset_hint_cycle_memory();
            obj.reset_auto_play_memory();
            let state_hash = obj.current_game_hash();
            obj.start_hint_loss_analysis_if_needed(state_hash);
            startup_trace::mark("window:constructed-exit");
        }
    }

    impl WidgetImpl for CardthropicWindow {}
    impl WindowImpl for CardthropicWindow {}
    impl ApplicationWindowImpl for CardthropicWindow {}
    impl AdwApplicationWindowImpl for CardthropicWindow {}
}

glib::wrapper! {
    pub struct CardthropicWindow(ObjectSubclass<imp::CardthropicWindow>)
        @extends gtk::Widget, gtk::Window, gtk::ApplicationWindow, adw::ApplicationWindow,
        @implements gio::ActionGroup, gio::ActionMap;
}

const APP_ICON_NAME: &str = "io.codeberg.emviolet.cardthropic";
const APP_ICON_FALLBACK_NAME: &str = "cardthropic";
const SETTINGS_SCHEMA_ID: &str = "io.codeberg.emviolet.cardthropic";
const SETTINGS_KEY_BOARD_COLOR: &str = "board-color";
const SETTINGS_KEY_SMART_MOVE_MODE: &str = "smart-move-mode";
const SETTINGS_KEY_SPIDER_SUIT_MODE: &str = "spider-suit-mode";
const SETTINGS_KEY_FREECELL_CARD_COUNT_MODE: &str = "freecell-card-count-mode";
const SETTINGS_KEY_SAVED_SESSION: &str = "saved-session";
const SETTINGS_KEY_CUSTOM_USERSTYLE_CSS: &str = "custom-userstyle-css";
const SETTINGS_KEY_SAVED_CUSTOM_USERSTYLE_CSS: &str = "saved-custom-userstyle-css";
const SETTINGS_KEY_CUSTOM_USERSTYLE_WORD_WRAP: &str = "custom-userstyle-word-wrap";
const SETTINGS_KEY_CUSTOM_CARD_SVG: &str = "custom-card-svg";
const SETTINGS_KEY_ENABLE_HUD: &str = "enable-hud";
const SETTINGS_KEY_FOREVER_MODE: &str = "forever-mode";
const SETTINGS_KEY_ROBOT_AUTO_NEW_GAME_ON_LOSS: &str = "robot-auto-new-game-on-loss";
const SETTINGS_KEY_LUDICROUS_SPEED: &str = "ludicrous-speed";
const SETTINGS_KEY_ROBOT_DEBUG_ENABLED: &str = "robot-debug-enabled";
const SETTINGS_KEY_ROBOT_STRICT_DEBUG_INVARIANTS: &str = "robot-strict-debug-invariants";
const SETTINGS_KEY_INTERFACE_EMOJI_FONT: &str = "interface-emoji-font";
const SETTINGS_KEY_SEED_HISTORY: &str = "seed-history";
const SETTINGS_KEY_CLOSE_PALETTE_ON_COMMAND: &str = "close-palette-on-command";
const SETTINGS_KEY_COMMAND_PALETTE_WIDTH: &str = "command-palette-width";
const SETTINGS_KEY_COMMAND_PALETTE_HEIGHT: &str = "command-palette-height";
const SETTINGS_KEY_COMMAND_PALETTE_MAXIMIZED: &str = "command-palette-maximized";
const SETTINGS_KEY_COMMAND_PALETTE_QUERY: &str = "command-palette-query";
const SETTINGS_KEY_STATUS_HISTORY_WIDTH: &str = "status-history-width";
const SETTINGS_KEY_STATUS_HISTORY_HEIGHT: &str = "status-history-height";
const SETTINGS_KEY_STATUS_HISTORY_MAXIMIZED: &str = "status-history-maximized";
const SETTINGS_KEY_THEME_PRESETS_WIDTH: &str = "theme-presets-width";
const SETTINGS_KEY_THEME_PRESETS_HEIGHT: &str = "theme-presets-height";
const SETTINGS_KEY_THEME_PRESETS_MAXIMIZED: &str = "theme-presets-maximized";
const SETTINGS_KEY_APM_GRAPH_WIDTH: &str = "apm-graph-width";
const SETTINGS_KEY_APM_GRAPH_HEIGHT: &str = "apm-graph-height";
const SETTINGS_KEY_APM_GRAPH_MAXIMIZED: &str = "apm-graph-maximized";
const SETTINGS_KEY_HELP_WIDTH: &str = "help-width";
const SETTINGS_KEY_HELP_HEIGHT: &str = "help-height";
const SETTINGS_KEY_HELP_MAXIMIZED: &str = "help-maximized";
const SETTINGS_KEY_MEMORY_GUARD_ENABLED: &str = "memory-guard-enabled";
const SETTINGS_KEY_MEMORY_GUARD_SOFT_LIMIT_MIB: &str = "memory-guard-soft-limit-mib";
const SETTINGS_KEY_MEMORY_GUARD_HARD_LIMIT_MIB: &str = "memory-guard-hard-limit-mib";
const MAX_SEED_HISTORY_ENTRIES: usize = 10_000;
const MAX_SEED_DROPDOWN_ENTRIES: usize = 250;
const SEED_HISTORY_FLUSH_INTERVAL_SECS: u32 = 15;
const SEED_WINNABLE_BUTTON_LABEL: &str = "W?";
const MIN_WINDOW_WIDTH: i32 = 250;
const MIN_WINDOW_HEIGHT: i32 = 250;
const MOBILE_PHONE_BREAKPOINT_PX: i32 = 450;
const MOBILE_PHONE_BREAKPOINT_HYSTERESIS_PX: i32 = 24;
const TABLEAU_FACE_UP_STEP_PX: i32 = 24;
const TABLEAU_FACE_DOWN_STEP_PX: i32 = 12;
const DEFAULT_BOARD_COLOR: &str = "#1f232b";
const ROBOT_BENCHMARK_DUMP_INTERVAL: u32 = 25;

impl CardthropicWindow {
    fn initialize_shared_state_persistence_owner(&self) {
        let owner = self
            .application()
            .map(|app| {
                !app.windows()
                    .iter()
                    .filter_map(|window| window.clone().downcast::<CardthropicWindow>().ok())
                    .any(|window| {
                        window.as_ptr() != self.as_ptr() && window.should_persist_shared_state()
                    })
            })
            .unwrap_or(true);
        self.set_shared_state_persistence_owner(owner);
    }

    pub(super) fn should_persist_shared_state(&self) -> bool {
        self.imp().shared_state_persistence_owner.get()
    }

    pub(super) fn set_shared_state_persistence_owner(&self, owner: bool) {
        self.imp().shared_state_persistence_owner.set(owner);
    }

    fn handoff_shared_state_persistence_owner_if_needed(&self) {
        if !self.should_persist_shared_state() {
            return;
        }
        self.set_shared_state_persistence_owner(false);

        let Some(app) = self.application() else {
            return;
        };
        if let Some(next_owner) = app
            .windows()
            .iter()
            .filter_map(|window| window.clone().downcast::<CardthropicWindow>().ok())
            .find(|window| window.as_ptr() != self.as_ptr())
        {
            next_owner.set_shared_state_persistence_owner(true);
        }
    }

    pub(super) fn append_startup_trace_history_if_robot_debug(&self) {
        if !self.imp().robot_debug_enabled.get() {
            return;
        }
        self.append_status_history_only("startup_trace_begin");
        for line in startup_trace::history_lines() {
            self.append_status_history_only(&line);
        }
    }

    pub(super) fn append_startup_deck_history_if_robot_debug(&self) {
        if self.imp().startup_deck_logged.replace(true) || !self.imp().robot_debug_enabled.get() {
            return;
        }
        self.append_status_history_only("startup_trace_deck_ready");
        for line in startup_trace::deck_history_lines() {
            self.append_status_history_only(&line);
        }
    }

    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        startup_trace::mark("window:new-enter");
        let window = glib::Object::builder()
            .property("application", application)
            .build();
        startup_trace::mark("window:new-exit");
        window
    }

    pub(super) fn close_auxiliary_windows(&self) {
        let imp = self.imp();
        let mut to_close: Vec<gtk::Window> = Vec::new();

        if let Some(w) = imp.custom_userstyle_dialog.borrow().as_ref() {
            to_close.push(w.clone());
        }
        if let Some(w) = imp.theme_presets_window.borrow().as_ref() {
            to_close.push(w.clone());
        }
        if let Some(w) = imp.status_history_dialog.borrow().as_ref() {
            to_close.push(w.clone());
        }
        if let Some(w) = imp.help_dialog.borrow().as_ref() {
            to_close.push(w.clone());
        }
        if let Some(w) = imp.command_search_dialog.borrow().as_ref() {
            to_close.push(w.clone());
        }
        if let Some(w) = imp.memory_guard_dialog.borrow().as_ref() {
            to_close.push(w.clone());
        }
        if let Some(w) = imp.apm_graph_dialog.borrow().as_ref() {
            to_close.push(w.clone());
        }

        *imp.custom_userstyle_dialog.borrow_mut() = None;
        *imp.theme_presets_window.borrow_mut() = None;
        *imp.status_history_dialog.borrow_mut() = None;
        *imp.help_dialog.borrow_mut() = None;
        *imp.command_search_dialog.borrow_mut() = None;
        *imp.command_search_filter_entry.borrow_mut() = None;
        *imp.memory_guard_dialog.borrow_mut() = None;
        *imp.apm_graph_dialog.borrow_mut() = None;
        *imp.status_history_buffer.borrow_mut() = None;
        *imp.apm_graph_area.borrow_mut() = None;
        *imp.apm_peak_label.borrow_mut() = None;
        *imp.apm_avg_label.borrow_mut() = None;
        *imp.apm_tilt_label.borrow_mut() = None;

        for window in to_close {
            window.close();
        }
    }

    pub(super) fn automation_profile(&self) -> AutomationProfile {
        engine_for_mode(self.imp().current_game_mode.get()).automation_profile()
    }

    pub(super) fn mode_spec(&self) -> crate::engine::variant::VariantSpec {
        let mode = self.imp().current_game_mode.get();
        variant_for_mode(mode).spec()
    }
}

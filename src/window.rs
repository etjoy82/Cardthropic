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
use crate::engine::hinting::{HintNode, HintSuggestion};
use crate::engine::keyboard_nav::KeyboardTarget;
use crate::engine::loss_analysis::LossVerdict;
use crate::engine::moves::{apply_hint_move_to_game, map_solver_line_to_hint_line, HintMove};
use crate::engine::robot::RobotPlayback;
use crate::engine::seed_history::SeedHistoryStore;
use crate::engine::variant::variant_for_mode;
use crate::engine::variant_engine::engine_for_mode;
use crate::engine::variant_state::VariantStateStore;
use crate::game::{Card, DrawMode, GameMode, KlondikeGame, SolverMove, SpiderSuitMode};
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
#[path = "window/dialogs_help.rs"]
mod dialogs_help;
#[path = "window/drag.rs"]
mod drag;
#[path = "window/drag_setup.rs"]
mod drag_setup;
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
        pub help_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub fullscreen_button: TemplateChild<gtk::Button>,
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
        pub copy_session_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub paste_session_button: TemplateChild<gtk::Button>,
        #[template_child]
        pub robot_debug_toggle_button: TemplateChild<gtk::ToggleButton>,
        #[template_child]
        pub stock_picture: TemplateChild<gtk::Picture>,
        #[template_child]
        pub waste_overlay: TemplateChild<gtk::Overlay>,
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
        pub waste_heading_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub foundations_heading_box: TemplateChild<gtk::Box>,
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
        pub stats_label: TemplateChild<gtk::Label>,
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
        pub board_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub toolbar_box: TemplateChild<gtk::Box>,
        #[template_child]
        pub motion_layer: TemplateChild<gtk::Fixed>,
        pub game: RefCell<VariantStateStore>,
        pub current_seed: Cell<u64>,
        pub current_seed_win_recorded: Cell<bool>,
        pub(super) seed_history: RefCell<SeedHistoryStore>,
        pub(super) selected_run: RefCell<Option<SelectedRun>>,
        pub waste_selected: Cell<bool>,
        pub settings: RefCell<Option<gio::Settings>>,
        pub last_saved_session: RefCell<String>,
        pub session_dirty: Cell<bool>,
        pub session_flush_timer: RefCell<Option<glib::SourceId>>,
        pub board_color_hex: RefCell<String>,
        pub board_color_preview: RefCell<Option<gtk::DrawingArea>>,
        pub board_color_swatches: RefCell<Vec<gtk::DrawingArea>>,
        pub board_color_provider: RefCell<Option<gtk::CssProvider>>,
        pub custom_userstyle_css: RefCell<String>,
        pub saved_custom_userstyle_css: RefCell<String>,
        pub custom_userstyle_provider: RefCell<Option<gtk::CssProvider>>,
        pub custom_userstyle_dialog: RefCell<Option<gtk::Window>>,
        pub theme_presets_window: RefCell<Option<gtk::Window>>,
        pub status_history: RefCell<VecDeque<String>>,
        pub status_last_appended: RefCell<String>,
        pub status_history_dialog: RefCell<Option<gtk::Window>>,
        pub status_history_buffer: RefCell<Option<gtk::TextBuffer>>,
        pub deck: RefCell<Option<AngloDeck>>,
        pub deck_load_attempted: Cell<bool>,
        pub deck_error: RefCell<Option<String>>,
        pub status_override: RefCell<Option<String>>,
        pub history: RefCell<Vec<Snapshot>>,
        pub future: RefCell<Vec<Snapshot>>,
        pub(super) apm_samples: RefCell<Vec<ApmSample>>,
        pub move_count: Cell<u32>,
        pub elapsed_seconds: Cell<u32>,
        pub timer_started: Cell<bool>,
        pub style_provider: RefCell<Option<gtk::CssProvider>>,
        pub card_width: Cell<i32>,
        pub card_height: Cell<i32>,
        pub face_up_step: Cell<i32>,
        pub face_down_step: Cell<i32>,
        pub observed_window_width: Cell<i32>,
        pub observed_window_height: Cell<i32>,
        pub observed_scroller_width: Cell<i32>,
        pub observed_scroller_height: Cell<i32>,
        pub observed_maximized: Cell<bool>,
        pub geometry_render_pending: Cell<bool>,
        pub geometry_render_dirty: Cell<bool>,
        pub pending_deal_instructions: Cell<bool>,
        pub last_metrics_key: Cell<u64>,
        pub tableau_card_pictures: RefCell<Vec<Vec<gtk::Picture>>>,
        pub(super) tableau_picture_state_cache: RefCell<Vec<Vec<Option<TableauPictureRenderState>>>>,
        pub last_stock_waste_foundation_size: Cell<(i32, i32)>,
        pub hint_timeouts: RefCell<Vec<glib::SourceId>>,
        pub hint_widgets: RefCell<Vec<gtk::Widget>>,
        pub hint_recent_states: RefCell<VecDeque<u64>>,
        pub seed_check_running: Cell<bool>,
        pub seed_check_generation: Cell<u64>,
        pub seed_check_cancel: RefCell<Option<Arc<AtomicBool>>>,
        pub seed_check_timer: RefCell<Option<glib::SourceId>>,
        pub seed_check_seconds: Cell<u32>,
        pub auto_play_seen_states: RefCell<HashSet<u64>>,
        pub auto_playing_move: Cell<bool>,
        pub(super) hint_loss_cache: RefCell<HashMap<u64, LossVerdict>>,
        pub hint_loss_analysis_running: Cell<bool>,
        pub hint_loss_analysis_hash: Cell<u64>,
        pub hint_loss_analysis_cancel: RefCell<Option<Arc<AtomicBool>>>,
        pub rapid_wand_running: Cell<bool>,
        pub rapid_wand_timer: RefCell<Option<glib::SourceId>>,
        pub robot_mode_running: Cell<bool>,
        pub robot_mode_timer: RefCell<Option<glib::SourceId>>,
        pub robot_forever_enabled: Cell<bool>,
        pub robot_wins: Cell<u32>,
        pub robot_losses: Cell<u32>,
        pub robot_last_benchmark_dump_total: Cell<u32>,
        pub robot_deals_tried: Cell<u32>,
        pub robot_moves_applied: Cell<u32>,
        pub robot_debug_enabled: Cell<bool>,
        pub(super) robot_playback: RefCell<RobotPlayback<HintMove>>,
        pub(super) drag_origin: RefCell<Option<DragOrigin>>,
        pub drag_widgets: RefCell<Vec<gtk::Widget>>,
        pub drag_timeouts: RefCell<Vec<glib::SourceId>>,
        pub suppress_waste_click_once: Cell<bool>,
        pub seed_search_in_progress: Cell<bool>,
        pub seed_combo_updating: Cell<bool>,
        pub smart_move_mode: Cell<SmartMoveMode>,
        pub robot_strategy: Cell<RobotStrategy>,
        pub hud_enabled: Cell<bool>,
        pub peek_active: Cell<bool>,
        pub peek_generation: Cell<u64>,
        pub current_game_mode: Cell<GameMode>,
        pub klondike_draw_mode: Cell<DrawMode>,
        pub spider_suit_mode: Cell<SpiderSuitMode>,
        pub game_mode_buttons: RefCell<HashMap<GameMode, gtk::Button>>,
        pub help_dialog: RefCell<Option<gtk::Window>>,
        pub apm_graph_dialog: RefCell<Option<gtk::Window>>,
        pub apm_graph_area: RefCell<Option<gtk::DrawingArea>>,
        pub apm_peak_label: RefCell<Option<gtk::Label>>,
        pub apm_avg_label: RefCell<Option<gtk::Label>>,
        pub apm_tilt_label: RefCell<Option<gtk::Label>>,
        pub keyboard_target: Cell<KeyboardTarget>,
    }

    impl Default for CardthropicWindow {
        fn default() -> Self {
            let seed = crate::engine::seed_ops::random_seed();
            Self {
                help_button: TemplateChild::default(),
                fullscreen_button: TemplateChild::default(),
                hud_button: TemplateChild::default(),
                undo_button: TemplateChild::default(),
                redo_button: TemplateChild::default(),
                auto_hint_button: TemplateChild::default(),
                cyclone_shuffle_button: TemplateChild::default(),
                peek_button: TemplateChild::default(),
                robot_button: TemplateChild::default(),
                copy_session_button: TemplateChild::default(),
                paste_session_button: TemplateChild::default(),
                robot_debug_toggle_button: TemplateChild::default(),
                stock_picture: TemplateChild::default(),
                waste_overlay: TemplateChild::default(),
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
                waste_heading_box: TemplateChild::default(),
                foundations_heading_box: TemplateChild::default(),
                foundations_area_box: TemplateChild::default(),
                foundation_picture_1: TemplateChild::default(),
                foundation_placeholder_1: TemplateChild::default(),
                foundation_picture_2: TemplateChild::default(),
                foundation_placeholder_2: TemplateChild::default(),
                foundation_picture_3: TemplateChild::default(),
                foundation_placeholder_3: TemplateChild::default(),
                foundation_picture_4: TemplateChild::default(),
                foundation_placeholder_4: TemplateChild::default(),
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
                stats_label: TemplateChild::default(),
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
                board_box: TemplateChild::default(),
                toolbar_box: TemplateChild::default(),
                motion_layer: TemplateChild::default(),
                game: RefCell::new(VariantStateStore::new(seed)),
                current_seed: Cell::new(seed),
                current_seed_win_recorded: Cell::new(false),
                seed_history: RefCell::new(SeedHistoryStore::default()),
                selected_run: RefCell::new(None),
                waste_selected: Cell::new(false),
                settings: RefCell::new(None),
                last_saved_session: RefCell::new(String::new()),
                session_dirty: Cell::new(false),
                session_flush_timer: RefCell::new(None),
                board_color_hex: RefCell::new(DEFAULT_BOARD_COLOR.to_string()),
                board_color_preview: RefCell::new(None),
                board_color_swatches: RefCell::new(Vec::new()),
                board_color_provider: RefCell::new(None),
                custom_userstyle_css: RefCell::new(String::new()),
                saved_custom_userstyle_css: RefCell::new(String::new()),
                custom_userstyle_provider: RefCell::new(None),
                custom_userstyle_dialog: RefCell::new(None),
                theme_presets_window: RefCell::new(None),
                status_history: RefCell::new(VecDeque::new()),
                status_last_appended: RefCell::new(String::new()),
                status_history_dialog: RefCell::new(None),
                status_history_buffer: RefCell::new(None),
                deck: RefCell::new(None),
                deck_load_attempted: Cell::new(false),
                deck_error: RefCell::new(None),
                status_override: RefCell::new(None),
                history: RefCell::new(Vec::new()),
                future: RefCell::new(Vec::new()),
                apm_samples: RefCell::new(Vec::new()),
                move_count: Cell::new(0),
                elapsed_seconds: Cell::new(0),
                timer_started: Cell::new(false),
                style_provider: RefCell::new(None),
                card_width: Cell::new(70),
                card_height: Cell::new(108),
                face_up_step: Cell::new(28),
                face_down_step: Cell::new(14),
                observed_window_width: Cell::new(0),
                observed_window_height: Cell::new(0),
                observed_scroller_width: Cell::new(0),
                observed_scroller_height: Cell::new(0),
                observed_maximized: Cell::new(false),
                geometry_render_pending: Cell::new(false),
                geometry_render_dirty: Cell::new(false),
                pending_deal_instructions: Cell::new(true),
                last_metrics_key: Cell::new(0),
                tableau_card_pictures: RefCell::new(vec![Vec::new(); 10]),
                tableau_picture_state_cache: RefCell::new(vec![Vec::new(); 10]),
                last_stock_waste_foundation_size: Cell::new((0, 0)),
                hint_timeouts: RefCell::new(Vec::new()),
                hint_widgets: RefCell::new(Vec::new()),
                hint_recent_states: RefCell::new(VecDeque::new()),
                seed_check_running: Cell::new(false),
                seed_check_generation: Cell::new(0),
                seed_check_cancel: RefCell::new(None),
                seed_check_timer: RefCell::new(None),
                seed_check_seconds: Cell::new(0),
                auto_play_seen_states: RefCell::new(HashSet::new()),
                auto_playing_move: Cell::new(false),
                hint_loss_cache: RefCell::new(HashMap::new()),
                hint_loss_analysis_running: Cell::new(false),
                hint_loss_analysis_hash: Cell::new(0),
                hint_loss_analysis_cancel: RefCell::new(None),
                rapid_wand_running: Cell::new(false),
                rapid_wand_timer: RefCell::new(None),
                robot_mode_running: Cell::new(false),
                robot_mode_timer: RefCell::new(None),
                robot_forever_enabled: Cell::new(false),
                robot_wins: Cell::new(0),
                robot_losses: Cell::new(0),
                robot_last_benchmark_dump_total: Cell::new(0),
                robot_deals_tried: Cell::new(0),
                robot_moves_applied: Cell::new(0),
                robot_debug_enabled: Cell::new(false),
                robot_playback: RefCell::new(RobotPlayback::default()),
                drag_origin: RefCell::new(None),
                drag_widgets: RefCell::new(Vec::new()),
                drag_timeouts: RefCell::new(Vec::new()),
                suppress_waste_click_once: Cell::new(false),
                seed_search_in_progress: Cell::new(false),
                seed_combo_updating: Cell::new(false),
                smart_move_mode: Cell::new(SmartMoveMode::DoubleClick),
                robot_strategy: Cell::new(RobotStrategy::Balanced),
                hud_enabled: Cell::new(true),
                peek_active: Cell::new(false),
                peek_generation: Cell::new(0),
                current_game_mode: Cell::new(GameMode::Klondike),
                klondike_draw_mode: Cell::new(DrawMode::One),
                spider_suit_mode: Cell::new(SpiderSuitMode::One),
                game_mode_buttons: RefCell::new(HashMap::new()),
                help_dialog: RefCell::new(None),
                apm_graph_dialog: RefCell::new(None),
                apm_graph_area: RefCell::new(None),
                apm_peak_label: RefCell::new(None),
                apm_avg_label: RefCell::new(None),
                apm_tilt_label: RefCell::new(None),
                keyboard_target: Cell::new(KeyboardTarget::Stock),
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
            klass.install_action("win.copy-benchmark-snapshot", None, |window, _, _| {
                window.copy_benchmark_snapshot();
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
            klass.install_action("win.mode-klondike", None, |window, _, _| {
                window.select_game_mode("klondike");
            });
            klass.install_action("win.mode-spider", None, |window, _, _| {
                window.select_game_mode("spider");
            });
            klass.install_action("win.mode-freecell", None, |window, _, _| {
                window.select_game_mode("freecell");
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
            klass.install_action("win.automation-strategy-fast", None, |window, _, _| {
                window.set_robot_strategy(RobotStrategy::Fast, true, true);
            });
            klass.install_action("win.automation-strategy-balanced", None, |window, _, _| {
                window.set_robot_strategy(RobotStrategy::Balanced, true, true);
            });
            klass.install_action("win.automation-strategy-deep", None, |window, _, _| {
                window.set_robot_strategy(RobotStrategy::Deep, true, true);
            });
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    impl ObjectImpl for CardthropicWindow {
        fn constructed(&self) {
            self.parent_constructed();
            let obj = self.obj();
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
            obj.setup_board_color_preferences();
            if !obj.try_restore_saved_session() {
                obj.note_seed_play_started(self.current_seed.get());
                obj.set_seed_input_text(&self.current_seed.get().to_string());
            }
            obj.setup_handlers();
            obj.connect_close_request(glib::clone!(
                #[weak(rename_to = window)]
                obj,
                #[upgrade_or]
                glib::Propagation::Proceed,
                move |_| {
                    window.flush_session_now();
                    glib::Propagation::Proceed
                }
            ));
            obj.imp().tableau_row.set_homogeneous(true);
            obj.setup_timer();
            obj.render();
            obj.reset_hint_cycle_memory();
            obj.reset_auto_play_memory();
            let state_hash = obj.current_game_hash();
            obj.start_hint_loss_analysis_if_needed(state_hash);
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
const SETTINGS_KEY_SAVED_SESSION: &str = "saved-session";
const SETTINGS_KEY_CUSTOM_USERSTYLE_CSS: &str = "custom-userstyle-css";
const SETTINGS_KEY_SAVED_CUSTOM_USERSTYLE_CSS: &str = "saved-custom-userstyle-css";
const SETTINGS_KEY_CUSTOM_USERSTYLE_WORD_WRAP: &str = "custom-userstyle-word-wrap";
const SETTINGS_KEY_CUSTOM_CARD_SVG: &str = "custom-card-svg";
const SETTINGS_KEY_ENABLE_HUD: &str = "enable-hud";
const SETTINGS_KEY_ROBOT_STRATEGY: &str = "robot-strategy";
const SEED_HISTORY_FILE_NAME: &str = "seed-history.txt";
const APP_DATA_DIR_NAME: &str = "io.codeberg.emviolet.cardthropic";
const MAX_SEED_HISTORY_ENTRIES: usize = 10_000;
const MAX_SEED_DROPDOWN_ENTRIES: usize = 250;
const SEED_WINNABLE_BUTTON_LABEL: &str = "W?";
const MIN_WINDOW_WIDTH: i32 = 700;
const MIN_WINDOW_HEIGHT: i32 = 800;
const TABLEAU_FACE_UP_STEP_PX: i32 = 24;
const TABLEAU_FACE_DOWN_STEP_PX: i32 = 12;
const DEFAULT_BOARD_COLOR: &str = "#1f232b";
const ROBOT_BENCHMARK_DUMP_INTERVAL: u32 = 25;

impl CardthropicWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }

    pub(super) fn automation_profile(&self) -> AutomationProfile {
        engine_for_mode(self.imp().current_game_mode.get()).automation_profile()
    }

    pub(super) fn mode_spec(&self) -> crate::engine::variant::VariantSpec {
        let mode = self.imp().current_game_mode.get();
        variant_for_mode(mode).spec()
    }
}

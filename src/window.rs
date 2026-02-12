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
use std::cmp::Reverse;
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
use crate::engine::moves::{apply_hint_move_to_game, map_solver_line_to_hint_line, HintMove};
use crate::engine::robot::RobotPlayback;
use crate::engine::seed_history::SeedHistoryStore;
use crate::game::{Card, DrawMode, DrawResult, GameMode, KlondikeGame, SolverMove, Suit};
use crate::winnability;

#[path = "window/actions_history.rs"]
mod actions_history;
#[path = "window/actions_moves.rs"]
mod actions_moves;
#[path = "window/actions_selection.rs"]
mod actions_selection;
#[path = "window/ai.rs"]
mod ai;
#[path = "window/dialogs.rs"]
mod dialogs;
#[path = "window/drag.rs"]
mod drag;
#[path = "window/handlers.rs"]
mod handlers;
#[path = "window/heuristics.rs"]
mod heuristics;
#[path = "window/hint_autoplay.rs"]
mod hint_autoplay;
#[path = "window/hint_core.rs"]
mod hint_core;
#[path = "window/hints.rs"]
mod hints;
#[path = "window/input.rs"]
mod input;
#[path = "window/layout.rs"]
mod layout;
#[path = "window/menu.rs"]
mod menu;
#[path = "window/parsing.rs"]
mod parsing;
#[path = "window/render.rs"]
mod render;
#[path = "window/robot.rs"]
mod robot;
#[path = "window/seed.rs"]
mod seed;
#[path = "window/session.rs"]
mod session;
#[path = "window/state.rs"]
mod state;
#[path = "window/theme.rs"]
mod theme;

use heuristics::*;
use parsing::*;

#[derive(Debug, Clone)]
pub struct Snapshot {
    game: KlondikeGame,
    selected_run: Option<SelectedRun>,
    selected_waste: bool,
    move_count: u32,
    elapsed_seconds: u32,
    timer_started: bool,
    apm_samples: Vec<ApmSample>,
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub(super) struct ApmSample {
    elapsed_seconds: u32,
    apm: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SelectedRun {
    col: usize,
    start: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmartMoveMode {
    Disabled,
    SingleClick,
    DoubleClick,
}

impl SmartMoveMode {
    fn as_setting(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::SingleClick => "single-click",
            Self::DoubleClick => "double-click",
        }
    }

    fn from_setting(value: &str) -> Self {
        match value {
            "disabled" => Self::Disabled,
            "single-click" => Self::SingleClick,
            "double-click" => Self::DoubleClick,
            _ => Self::DoubleClick,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardTarget {
    Stock,
    Waste,
    Foundation(usize),
    Tableau { col: usize, start: Option<usize> },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HintNode {
    Stock,
    Waste,
    Foundation(usize),
    Tableau { col: usize, index: Option<usize> },
}

#[derive(Debug, Clone, Copy)]
pub(super) enum DragOrigin {
    Waste,
    Tableau { col: usize, start: usize },
}

#[derive(Debug, Clone)]
struct HintSuggestion {
    message: String,
    source: Option<HintNode>,
    target: Option<HintNode>,
    hint_move: Option<HintMove>,
}

#[derive(Debug, Clone, Copy)]
enum LossVerdict {
    Lost { explored_states: usize },
    WinnableLikely,
    Inconclusive { explored_states: usize },
}

#[derive(Debug, Clone)]
struct PersistedSession {
    seed: u64,
    mode: GameMode,
    move_count: u32,
    elapsed_seconds: u32,
    timer_started: bool,
    game: KlondikeGame,
}

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
        pub tableau_scroller: TemplateChild<gtk::ScrolledWindow>,
        #[template_child]
        pub tableau_row: TemplateChild<gtk::Box>,
        #[template_child]
        pub main_menu_popover: TemplateChild<gtk::PopoverMenu>,
        #[template_child]
        pub board_color_menu_button: TemplateChild<gtk::MenuButton>,
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
        pub game: RefCell<KlondikeGame>,
        pub current_seed: Cell<u64>,
        pub current_seed_win_recorded: Cell<bool>,
        pub(super) seed_history: RefCell<SeedHistoryStore>,
        pub(super) selected_run: RefCell<Option<SelectedRun>>,
        pub waste_selected: Cell<bool>,
        pub settings: RefCell<Option<gio::Settings>>,
        pub last_saved_session: RefCell<String>,
        pub board_color_hex: RefCell<String>,
        pub board_color_preview: RefCell<Option<gtk::DrawingArea>>,
        pub board_color_provider: RefCell<Option<gtk::CssProvider>>,
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
        pub last_metrics_key: Cell<u64>,
        pub tableau_card_pictures: RefCell<Vec<Vec<gtk::Picture>>>,
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
        pub rapid_wand_running: Cell<bool>,
        pub rapid_wand_timer: RefCell<Option<glib::SourceId>>,
        pub robot_mode_running: Cell<bool>,
        pub robot_mode_timer: RefCell<Option<glib::SourceId>>,
        pub robot_deals_tried: Cell<u32>,
        pub(super) robot_playback: RefCell<RobotPlayback<HintMove>>,
        pub(super) drag_origin: RefCell<Option<DragOrigin>>,
        pub drag_widgets: RefCell<Vec<gtk::Widget>>,
        pub drag_timeouts: RefCell<Vec<glib::SourceId>>,
        pub suppress_waste_click_once: Cell<bool>,
        pub seed_search_in_progress: Cell<bool>,
        pub seed_combo_updating: Cell<bool>,
        pub smart_move_mode: Cell<SmartMoveMode>,
        pub peek_active: Cell<bool>,
        pub peek_generation: Cell<u64>,
        pub current_game_mode: Cell<GameMode>,
        pub klondike_draw_mode: Cell<DrawMode>,
        pub game_mode_klondike_button: RefCell<Option<gtk::Button>>,
        pub game_mode_spider_button: RefCell<Option<gtk::Button>>,
        pub game_mode_freecell_button: RefCell<Option<gtk::Button>>,
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
            let seed = random_seed();
            Self {
                help_button: TemplateChild::default(),
                fullscreen_button: TemplateChild::default(),
                undo_button: TemplateChild::default(),
                redo_button: TemplateChild::default(),
                auto_hint_button: TemplateChild::default(),
                cyclone_shuffle_button: TemplateChild::default(),
                peek_button: TemplateChild::default(),
                robot_button: TemplateChild::default(),
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
                tableau_scroller: TemplateChild::default(),
                tableau_row: TemplateChild::default(),
                main_menu_popover: TemplateChild::default(),
                board_color_menu_button: TemplateChild::default(),
                game_settings_menu_button: TemplateChild::default(),
                main_menu_button: TemplateChild::default(),
                game_settings_popover: TemplateChild::default(),
                game_settings_content_box: TemplateChild::default(),
                board_box: TemplateChild::default(),
                game: RefCell::new(KlondikeGame::new_with_seed(seed)),
                current_seed: Cell::new(seed),
                current_seed_win_recorded: Cell::new(false),
                seed_history: RefCell::new(SeedHistoryStore::default()),
                selected_run: RefCell::new(None),
                waste_selected: Cell::new(false),
                settings: RefCell::new(None),
                last_saved_session: RefCell::new(String::new()),
                board_color_hex: RefCell::new(DEFAULT_BOARD_COLOR.to_string()),
                board_color_preview: RefCell::new(None),
                board_color_provider: RefCell::new(None),
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
                last_metrics_key: Cell::new(0),
                tableau_card_pictures: RefCell::new(vec![Vec::new(); 7]),
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
                rapid_wand_running: Cell::new(false),
                rapid_wand_timer: RefCell::new(None),
                robot_mode_running: Cell::new(false),
                robot_mode_timer: RefCell::new(None),
                robot_deals_tried: Cell::new(0),
                robot_playback: RefCell::new(RobotPlayback::default()),
                drag_origin: RefCell::new(None),
                drag_widgets: RefCell::new(Vec::new()),
                drag_timeouts: RefCell::new(Vec::new()),
                suppress_waste_click_once: Cell::new(false),
                seed_search_in_progress: Cell::new(false),
                seed_combo_updating: Cell::new(false),
                smart_move_mode: Cell::new(SmartMoveMode::DoubleClick),
                peek_active: Cell::new(false),
                peek_generation: Cell::new(0),
                current_game_mode: Cell::new(GameMode::Klondike),
                klondike_draw_mode: Cell::new(DrawMode::One),
                game_mode_klondike_button: RefCell::new(None),
                game_mode_spider_button: RefCell::new(None),
                game_mode_freecell_button: RefCell::new(None),
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
            obj.setup_board_color_preferences();
            if !obj.try_restore_saved_session() {
                obj.note_seed_play_started(self.current_seed.get());
                obj.set_seed_input_text(&self.current_seed.get().to_string());
            }
            obj.setup_handlers();
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

const HINT_GUIDED_ANALYSIS_BUDGET: usize = 120_000;
const HINT_EXHAUSTIVE_ANALYSIS_BUDGET: usize = 220_000;
const AUTO_PLAY_LOOKAHEAD_DEPTH: u8 = 3;
const AUTO_PLAY_BEAM_WIDTH: usize = 10;
const AUTO_PLAY_NODE_BUDGET: usize = 3200;
const AUTO_PLAY_WIN_SCORE: i64 = 1_200_000;
const DIALOG_SEED_GUIDED_BUDGET: usize = 180_000;
const DIALOG_SEED_EXHAUSTIVE_BUDGET: usize = 300_000;
const DIALOG_FIND_WINNABLE_STATE_BUDGET: usize = 15_000;
const APP_ICON_NAME: &str = "io.codeberg.emviolet.cardthropic";
const APP_ICON_FALLBACK_NAME: &str = "cardthropic";
const SETTINGS_SCHEMA_ID: &str = "io.codeberg.emviolet.cardthropic";
const SETTINGS_KEY_BOARD_COLOR: &str = "board-color";
const SETTINGS_KEY_SMART_MOVE_MODE: &str = "smart-move-mode";
const SETTINGS_KEY_SAVED_SESSION: &str = "saved-session";
const SEED_HISTORY_FILE_NAME: &str = "seed-history.txt";
const APP_DATA_DIR_NAME: &str = "io.codeberg.emviolet.cardthropic";
const MAX_SEED_HISTORY_ENTRIES: usize = 10_000;
const MAX_SEED_DROPDOWN_ENTRIES: usize = 250;
const SEED_WINNABLE_BUTTON_LABEL: &str = "W?";
const MIN_WINDOW_WIDTH: i32 = 600;
const MIN_WINDOW_HEIGHT: i32 = 700;
const TABLEAU_FACE_UP_STEP_PX: i32 = 24;
const TABLEAU_FACE_DOWN_STEP_PX: i32 = 12;
const DEFAULT_BOARD_COLOR: &str = "#1f232b";
const BOARD_COLOR_THEMES: [(&str, &str); 4] = [
    ("Felt", "#1f3b2f"),
    ("Slate", "#2a2f45"),
    ("Sunset", "#5a3d24"),
    ("Ocean", "#1e3f53"),
];
const BOARD_COLOR_SWATCHES: [&str; 12] = [
    "#1f232b", "#1f3b2f", "#2a2f45", "#3a2a26", "#1e3f53", "#2d2d2d", "#3b4f24", "#47315c",
    "#5a3d24", "#0f5132", "#244a73", "#6b2f2f",
];

#[derive(Debug, Clone, Copy)]
enum WorkspacePreset {
    Compact600,
    Hd720,
    Fhd1080,
    Qhd1440,
}

#[derive(Debug, Clone, Copy)]
struct WorkspaceLayoutProfile {
    side_padding: i32,
    tableau_vertical_padding: i32,
    gap: i32,
    assumed_depth: i32,
    min_card_width: i32,
    max_card_width: i32,
    min_card_height: i32,
}

impl CardthropicWindow {
    pub fn new<P: IsA<gtk::Application>>(application: &P) -> Self {
        glib::Object::builder()
            .property("application", application)
            .build()
    }
}

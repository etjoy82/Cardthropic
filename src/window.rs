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
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

use adw::prelude::*;
use adw::subclass::prelude::*;
use gtk::{gdk, gdk_pixbuf, gio, glib};
use rand::Rng;

use crate::deck::AngloDeck;
use crate::game::{Card, DrawMode, DrawResult, GameMode, KlondikeGame, Suit};

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
enum HintMove {
    WasteToFoundation,
    TableauTopToFoundation {
        src: usize,
    },
    WasteToTableau {
        dst: usize,
    },
    TableauRunToTableau {
        src: usize,
        start: usize,
        dst: usize,
    },
    Draw,
}

#[derive(Debug, Clone, Copy)]
enum LossVerdict {
    Lost { explored_states: usize },
    WinnableLikely,
    Inconclusive { explored_states: usize },
}

#[derive(Debug, Clone, Copy)]
struct SeedWinnabilityCheckResult {
    winnable: bool,
    iterations: usize,
    moves_to_win: Option<u32>,
    hit_state_limit: bool,
}

#[derive(Debug, Clone, Default)]
struct SeedHistoryData {
    seeds: HashMap<u64, SeedHistoryStats>,
}

#[derive(Debug, Clone, Copy, Default)]
struct SeedHistoryStats {
    plays: u32,
    wins: u32,
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
        pub(super) seed_history: RefCell<SeedHistoryData>,
        pub(super) selected_run: RefCell<Option<SelectedRun>>,
        pub waste_selected: Cell<bool>,
        pub settings: RefCell<Option<gio::Settings>>,
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
        pub(super) drag_origin: RefCell<Option<DragOrigin>>,
        pub drag_widgets: RefCell<Vec<gtk::Widget>>,
        pub drag_timeouts: RefCell<Vec<glib::SourceId>>,
        pub suppress_waste_click_once: Cell<bool>,
        pub seed_search_in_progress: Cell<bool>,
        pub seed_combo_updating: Cell<bool>,
        pub smart_move_enabled: Cell<bool>,
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
    }

    impl Default for CardthropicWindow {
        fn default() -> Self {
            let seed = random_seed();
            Self {
                help_button: TemplateChild::default(),
                undo_button: TemplateChild::default(),
                redo_button: TemplateChild::default(),
                auto_hint_button: TemplateChild::default(),
                cyclone_shuffle_button: TemplateChild::default(),
                peek_button: TemplateChild::default(),
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
                seed_history: RefCell::new(SeedHistoryData::default()),
                selected_run: RefCell::new(None),
                waste_selected: Cell::new(false),
                settings: RefCell::new(None),
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
                drag_origin: RefCell::new(None),
                drag_widgets: RefCell::new(Vec::new()),
                drag_timeouts: RefCell::new(Vec::new()),
                suppress_waste_click_once: Cell::new(false),
                seed_search_in_progress: Cell::new(false),
                seed_combo_updating: Cell::new(false),
                smart_move_enabled: Cell::new(true),
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
            klass.install_action("win.help", None, |window, _, _| {
                window.show_help_dialog();
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
            obj.note_seed_play_started(self.current_seed.get());
            obj.set_seed_input_text(&self.current_seed.get().to_string());
            obj.setup_styles();
            obj.setup_board_color_preferences();
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
const SEED_HISTORY_FILE_NAME: &str = "seed-history.txt";
const APP_DATA_DIR_NAME: &str = "io.codeberg.emviolet.cardthropic";
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

    fn setup_timer(&self) {
        glib::timeout_add_seconds_local(
            1,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    window.on_timer_tick();
                    glib::ControlFlow::Continue
                }
            ),
        );
    }

    #[allow(deprecated)]
    fn seed_text_entry(&self) -> Option<gtk::Entry> {
        self.imp()
            .seed_combo
            .child()
            .and_then(|child| child.downcast::<gtk::Entry>().ok())
    }

    fn seed_input_text(&self) -> String {
        self.seed_text_entry()
            .map(|entry| entry.text().to_string())
            .unwrap_or_default()
    }

    fn set_seed_input_text(&self, text: &str) {
        let imp = self.imp();
        imp.seed_combo_updating.set(true);
        if let Some(entry) = self.seed_text_entry() {
            entry.set_text(text);
        }
        imp.seed_combo_updating.set(false);
    }

    fn seed_history_file_path() -> PathBuf {
        let mut path = glib::user_data_dir();
        path.push(APP_DATA_DIR_NAME);
        path.push(SEED_HISTORY_FILE_NAME);
        path
    }

    fn load_seed_history(&self) {
        let path = Self::seed_history_file_path();
        let mut data = SeedHistoryData::default();
        if let Ok(contents) = fs::read_to_string(path) {
            for line in contents.lines() {
                let mut parts = line.split_whitespace();
                let Some(seed_raw) = parts.next() else {
                    continue;
                };
                let Some(plays_raw) = parts.next() else {
                    continue;
                };
                let Some(wins_raw) = parts.next() else {
                    continue;
                };
                let Ok(seed) = seed_raw.parse::<u64>() else {
                    continue;
                };
                let Ok(plays) = plays_raw.parse::<u32>() else {
                    continue;
                };
                let Ok(wins) = wins_raw.parse::<u32>() else {
                    continue;
                };
                data.seeds.insert(
                    seed,
                    SeedHistoryStats {
                        plays,
                        wins: wins.min(plays),
                    },
                );
            }
        }

        *self.imp().seed_history.borrow_mut() = data;
    }

    fn save_seed_history(&self) {
        let path = Self::seed_history_file_path();
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }

        let mut rows: Vec<(u64, SeedHistoryStats)> = self
            .imp()
            .seed_history
            .borrow()
            .seeds
            .iter()
            .map(|(seed, stats)| (*seed, *stats))
            .collect();
        rows.sort_unstable_by(|a, b| a.0.cmp(&b.0));

        let mut serialized = String::new();
        for (seed, stats) in rows {
            let wins = stats.wins.min(stats.plays);
            serialized.push_str(&format!("{seed} {} {wins}\n", stats.plays));
        }
        let _ = fs::write(path, serialized);
    }

    fn note_seed_play_started(&self, seed: u64) {
        {
            let mut history = self.imp().seed_history.borrow_mut();
            let stats = history.seeds.entry(seed).or_default();
            stats.plays = stats.plays.saturating_add(1);
        }

        self.imp().current_seed_win_recorded.set(false);
        self.save_seed_history();
        self.refresh_seed_history_dropdown();
    }

    fn note_current_seed_win_if_needed(&self, game: &KlondikeGame) {
        if !game.is_won() || self.imp().current_seed_win_recorded.get() {
            return;
        }

        let seed = self.imp().current_seed.get();
        {
            let mut history = self.imp().seed_history.borrow_mut();
            let stats = history.seeds.entry(seed).or_default();
            if stats.plays == 0 {
                stats.plays = 1;
            }
            let next_wins = stats.wins.saturating_add(1);
            stats.wins = next_wins.min(stats.plays);
        }

        self.imp().current_seed_win_recorded.set(true);
        self.save_seed_history();
        self.refresh_seed_history_dropdown();
    }

    #[allow(deprecated)]
    fn refresh_seed_history_dropdown(&self) {
        let imp = self.imp();
        let current_text = self.seed_input_text();

        imp.seed_combo_updating.set(true);
        imp.seed_combo.remove_all();

        let mut seeds: Vec<(u64, SeedHistoryStats)> = imp
            .seed_history
            .borrow()
            .seeds
            .iter()
            .map(|(seed, stats)| (*seed, *stats))
            .collect();
        seeds.sort_unstable_by(|a, b| b.0.cmp(&a.0));

        for (seed, stats) in seeds {
            imp.seed_combo.append(
                Some(&seed.to_string()),
                &format!("{seed}: Plays {}, Wins {}", stats.plays, stats.wins),
            );
        }

        imp.seed_combo_updating.set(false);
        self.set_seed_input_text(&current_text);
    }

    fn setup_styles(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("style.css"));
        gtk::style_context_add_provider_for_display(
            &self.display(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        *self.imp().style_provider.borrow_mut() = Some(provider);
    }

    fn setup_board_color_preferences(&self) {
        let imp = self.imp();
        let settings = Self::load_app_settings();
        *imp.settings.borrow_mut() = settings;

        let initial_color = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.string(SETTINGS_KEY_BOARD_COLOR).to_string())
                .unwrap_or_else(|| DEFAULT_BOARD_COLOR.to_string())
        };
        self.set_board_color(&initial_color, false);
    }

    fn load_app_settings() -> Option<gio::Settings> {
        let source = gio::SettingsSchemaSource::default()?;
        let schema = source.lookup(SETTINGS_SCHEMA_ID, true)?;
        Some(gio::Settings::new_full(
            &schema,
            None::<&gio::SettingsBackend>,
            None::<&str>,
        ))
    }

    fn setup_board_color_dropdown(&self) {
        let imp = self.imp();
        let color_menu = imp.board_color_menu_button.get();
        color_menu.set_tooltip_text(Some("Board color"));
        color_menu.set_has_frame(true);
        color_menu.add_css_class("board-color-menu-button");

        let preview_frame = gtk::Frame::new(None);
        preview_frame.add_css_class("color-chip-frame");
        let preview = gtk::DrawingArea::new();
        preview.set_content_width(18);
        preview.set_content_height(18);
        preview.set_draw_func(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, cr, width, height| {
                let color = window.current_board_color_rgba();
                Self::draw_color_chip(cr, width, height, &color);
            }
        ));
        preview_frame.set_child(Some(&preview));

        let arrow = gtk::Image::from_icon_name("pan-down-symbolic");
        let color_menu_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        color_menu_content.append(&preview_frame);
        color_menu_content.append(&arrow);
        color_menu.set_child(Some(&color_menu_content));

        let palette_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        palette_box.set_margin_top(8);
        palette_box.set_margin_bottom(8);
        palette_box.set_margin_start(8);
        palette_box.set_margin_end(8);

        let theme_label = gtk::Label::new(Some("Themes"));
        theme_label.set_xalign(0.0);
        theme_label.add_css_class("dim-label");
        palette_box.append(&theme_label);

        let theme_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        theme_row.set_hexpand(true);
        theme_row.set_homogeneous(true);
        for (theme_name, color_hex) in BOARD_COLOR_THEMES {
            let theme_button = gtk::Button::with_label(theme_name);
            theme_button.add_css_class("flat");
            theme_button.connect_clicked(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.set_board_color(color_hex, true);
                }
            ));
            theme_row.append(&theme_button);
        }
        palette_box.append(&theme_row);

        let swatch_label = gtk::Label::new(Some("Swatches"));
        swatch_label.set_xalign(0.0);
        swatch_label.set_margin_top(2);
        swatch_label.add_css_class("dim-label");
        palette_box.append(&swatch_label);

        let palette_wrap = gtk::FlowBox::new();
        palette_wrap.set_selection_mode(gtk::SelectionMode::None);
        palette_wrap.set_max_children_per_line(6);
        palette_wrap.set_column_spacing(6);
        palette_wrap.set_row_spacing(6);
        palette_wrap.set_homogeneous(true);

        for color_hex in BOARD_COLOR_SWATCHES {
            let swatch_button = gtk::Button::new();
            swatch_button.set_has_frame(false);
            swatch_button.set_tooltip_text(Some(color_hex));

            let swatch_frame = gtk::Frame::new(None);
            swatch_frame.add_css_class("color-chip-frame");
            let swatch_chip = Self::build_color_chip(color_hex, 18);
            swatch_frame.set_child(Some(&swatch_chip));
            swatch_button.set_child(Some(&swatch_frame));

            swatch_button.connect_clicked(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.set_board_color(color_hex, true);
                }
            ));
            palette_wrap.insert(&swatch_button, -1);
        }
        palette_box.append(&palette_wrap);

        let reset_button = gtk::Button::with_label("Reset Default");
        reset_button.add_css_class("flat");
        reset_button.set_halign(gtk::Align::End);
        reset_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.set_board_color(DEFAULT_BOARD_COLOR, true);
            }
        ));
        palette_box.append(&reset_button);

        let palette_popover = gtk::Popover::new();
        palette_popover.set_child(Some(&palette_box));
        color_menu.set_popover(Some(&palette_popover));
        *imp.board_color_preview.borrow_mut() = Some(preview.clone());
        preview.queue_draw();
    }

    fn setup_game_mode_menu_item(&self) {
        let imp = self.imp();

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        row.set_margin_top(4);
        row.set_margin_bottom(4);
        row.set_margin_start(8);
        row.set_margin_end(8);

        let label = gtk::Label::new(Some("Game"));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        row.append(&label);

        let button_box = gtk::Box::new(gtk::Orientation::Horizontal, 6);

        let klondike_button = gtk::Button::with_label("🥇");
        klondike_button.add_css_class("flat");
        klondike_button.add_css_class("game-mode-emoji-button");
        klondike_button.set_tooltip_text(Some("Klondike"));
        klondike_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.select_game_mode("klondike");
            }
        ));
        button_box.append(&klondike_button);

        let spider_button = gtk::Button::with_label("🕷️");
        spider_button.add_css_class("flat");
        spider_button.add_css_class("game-mode-emoji-button");
        spider_button.set_tooltip_text(Some("Spider Solitaire"));
        spider_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.select_game_mode("spider");
            }
        ));
        button_box.append(&spider_button);

        let freecell_button = gtk::Button::with_label("🗽");
        freecell_button.add_css_class("flat");
        freecell_button.add_css_class("game-mode-emoji-button");
        freecell_button.set_tooltip_text(Some("FreeCell"));
        freecell_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.select_game_mode("freecell");
            }
        ));
        button_box.append(&freecell_button);

        row.append(&button_box);
        imp.main_menu_popover.add_child(&row, "game-mode-row");
        *imp.game_mode_klondike_button.borrow_mut() = Some(klondike_button);
        *imp.game_mode_spider_button.borrow_mut() = Some(spider_button);
        *imp.game_mode_freecell_button.borrow_mut() = Some(freecell_button);
        self.update_game_mode_menu_selection();
    }

    fn active_game_mode(&self) -> GameMode {
        self.imp().current_game_mode.get()
    }

    fn current_klondike_draw_mode(&self) -> DrawMode {
        self.imp().klondike_draw_mode.get()
    }

    fn set_klondike_draw_mode(&self, draw_mode: DrawMode) {
        let imp = self.imp();
        if imp.klondike_draw_mode.get() == draw_mode {
            return;
        }
        imp.klondike_draw_mode.set(draw_mode);
        imp.game.borrow_mut().set_draw_mode(draw_mode);
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        let state_hash = self.current_game_hash();
        self.start_hint_loss_analysis_if_needed(state_hash);
        *imp.status_override.borrow_mut() = Some(format!("Deal {} selected.", draw_mode.count()));
        self.render();
    }

    fn is_mode_engine_ready(&self) -> bool {
        self.active_game_mode().engine_ready()
    }

    fn guard_mode_engine(&self, action: &str) -> bool {
        let mode = self.active_game_mode();
        if mode.engine_ready() {
            return true;
        }

        *self.imp().status_override.borrow_mut() = Some(format!(
            "{action} is not available in {} yet. Engine refactor in progress.",
            mode.label()
        ));
        self.render();
        false
    }

    fn setup_game_settings_menu(&self) {
        self.update_game_settings_menu();
    }

    fn popdown_game_settings_later(&self) {
        glib::idle_add_local_once(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move || {
                window.imp().game_settings_popover.popdown();
            }
        ));
    }

    fn popdown_main_menu_later(&self) {
        glib::idle_add_local_once(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move || {
                window.imp().main_menu_popover.popdown();
            }
        ));
    }

    fn clear_game_settings_menu_content(&self) {
        let imp = self.imp();
        while let Some(child) = imp.game_settings_content_box.first_child() {
            imp.game_settings_content_box.remove(&child);
        }
    }

    fn update_game_settings_menu(&self) {
        let imp = self.imp();
        let mode = imp.current_game_mode.get();
        let mode_name = mode.label();
        imp.game_settings_menu_button.set_label(mode.emoji());
        imp.game_settings_menu_button
            .set_tooltip_text(Some(&format!("{mode_name} Settings")));

        self.clear_game_settings_menu_content();

        let heading = gtk::Label::new(Some(&format!("{mode_name} Settings")));
        heading.set_xalign(0.0);
        heading.add_css_class("heading");
        imp.game_settings_content_box.append(&heading);

        match mode {
            GameMode::Klondike => {
                let draw_label = gtk::Label::new(Some("Deal"));
                draw_label.set_xalign(0.0);
                draw_label.add_css_class("dim-label");
                imp.game_settings_content_box.append(&draw_label);

                let draw_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
                draw_row.set_hexpand(true);

                let modes = [
                    DrawMode::One,
                    DrawMode::Two,
                    DrawMode::Three,
                    DrawMode::Four,
                    DrawMode::Five,
                ];
                let current_draw_mode = self.current_klondike_draw_mode();
                let mut group_anchor: Option<gtk::CheckButton> = None;

                for mode in modes {
                    let label = format!("Deal {}", mode.count());
                    let button = gtk::CheckButton::with_label(&label);
                    if let Some(anchor) = group_anchor.as_ref() {
                        button.set_group(Some(anchor));
                    } else {
                        group_anchor = Some(button.clone());
                    }
                    if mode == current_draw_mode {
                        button.set_active(true);
                    }
                    button.connect_toggled(glib::clone!(
                        #[weak(rename_to = window)]
                        self,
                        move |btn| {
                            if btn.is_active() {
                                window.set_klondike_draw_mode(mode);
                            }
                        }
                    ));
                    draw_row.append(&button);
                }
                imp.game_settings_content_box.append(&draw_row);

                let random_button = gtk::Button::with_label("Start Random Deal");
                random_button.add_css_class("flat");
                random_button.set_halign(gtk::Align::Fill);
                random_button.set_hexpand(true);
                random_button.connect_clicked(glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |_| {
                        window.start_random_seed_game();
                        window.popdown_game_settings_later();
                    }
                ));
                imp.game_settings_content_box.append(&random_button);

                let winnable_button = gtk::Button::with_label("Winnable Deal");
                winnable_button.add_css_class("flat");
                winnable_button.set_halign(gtk::Align::Fill);
                winnable_button.set_hexpand(true);
                winnable_button.connect_clicked(glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |_| {
                        window.start_random_winnable_seed_game();
                        window.popdown_game_settings_later();
                    }
                ));
                imp.game_settings_content_box.append(&winnable_button);
            }
            GameMode::Spider => {
                let note =
                    gtk::Label::new(Some("Spider settings will appear once Spider is playable."));
                note.set_xalign(0.0);
                note.set_wrap(true);
                note.add_css_class("dim-label");
                imp.game_settings_content_box.append(&note);
            }
            GameMode::Freecell => {
                let note = gtk::Label::new(Some(
                    "FreeCell settings will appear once FreeCell is playable.",
                ));
                note.set_xalign(0.0);
                note.set_wrap(true);
                note.add_css_class("dim-label");
                imp.game_settings_content_box.append(&note);
            }
        }
    }

    fn update_game_mode_menu_selection(&self) {
        let imp = self.imp();
        let current = imp.current_game_mode.get();

        let klondike = imp.game_mode_klondike_button.borrow().clone();
        let spider = imp.game_mode_spider_button.borrow().clone();
        let freecell = imp.game_mode_freecell_button.borrow().clone();

        if let Some(button) = klondike.as_ref() {
            if current == GameMode::Klondike {
                button.add_css_class("game-mode-selected");
            } else {
                button.remove_css_class("game-mode-selected");
            }
        }
        if let Some(button) = spider.as_ref() {
            if current == GameMode::Spider {
                button.add_css_class("game-mode-selected");
            } else {
                button.remove_css_class("game-mode-selected");
            }
        }
        if let Some(button) = freecell.as_ref() {
            if current == GameMode::Freecell {
                button.add_css_class("game-mode-selected");
            } else {
                button.remove_css_class("game-mode-selected");
            }
        }
    }

    fn select_game_mode(&self, mode: &str) {
        let imp = self.imp();
        let status = match GameMode::from_id(mode) {
            Some(game_mode) => {
                imp.current_game_mode.set(game_mode);
                if game_mode.engine_ready() {
                    format!("{} selected.", game_mode.label())
                } else {
                    format!(
                        "{} selected. Gameplay engine is being refactored for this mode.",
                        game_mode.label()
                    )
                }
            }
            None => "Unknown game mode.".to_string(),
        };
        self.cancel_seed_winnable_check(None);
        *imp.selected_run.borrow_mut() = None;
        self.clear_hint_effects();
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        self.update_game_mode_menu_selection();
        self.update_game_settings_menu();
        *imp.status_override.borrow_mut() = Some(status);
        self.popdown_main_menu_later();
        self.render();
    }

    fn build_color_chip(color_hex: &str, size: i32) -> gtk::DrawingArea {
        let rgba = Self::normalize_board_color(color_hex);
        let chip = gtk::DrawingArea::new();
        chip.set_content_width(size);
        chip.set_content_height(size);
        chip.set_draw_func(move |_, cr, width, height| {
            Self::draw_color_chip(cr, width, height, &rgba);
        });
        chip
    }

    fn draw_color_chip(cr: &gtk::cairo::Context, width: i32, height: i32, color: &gdk::RGBA) {
        let w = f64::from(width.max(1));
        let h = f64::from(height.max(1));
        cr.set_source_rgba(
            f64::from(color.red()),
            f64::from(color.green()),
            f64::from(color.blue()),
            f64::from(color.alpha()),
        );
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();

        cr.set_source_rgba(0.0, 0.0, 0.0, 0.35);
        cr.rectangle(0.5, 0.5, (w - 1.0).max(0.0), (h - 1.0).max(0.0));
        let _ = cr.stroke();
    }

    fn normalize_board_color(color: &str) -> gdk::RGBA {
        gdk::RGBA::parse(color)
            .or_else(|_| gdk::RGBA::parse(DEFAULT_BOARD_COLOR))
            .unwrap_or_else(|_| gdk::RGBA::new(0.12, 0.14, 0.17, 1.0))
    }

    fn current_board_color_rgba(&self) -> gdk::RGBA {
        let color = self.imp().board_color_hex.borrow().clone();
        Self::normalize_board_color(&color)
    }

    fn set_board_color(&self, color: &str, persist: bool) {
        let imp = self.imp();
        let normalized = Self::normalize_board_color(color);
        let css_color = normalized.to_string();

        // Clone out of RefCell first to avoid overlapping immutable+mutable borrows.
        let existing_provider = imp.board_color_provider.borrow().clone();
        let provider = if let Some(provider) = existing_provider {
            provider
        } else {
            let provider = gtk::CssProvider::new();
            gtk::style_context_add_provider_for_display(
                &self.display(),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
            *imp.board_color_provider.borrow_mut() = Some(provider.clone());
            provider
        };

        provider.load_from_string(&format!(
            ".board-background {{ background-color: {}; border-radius: 12px; transition: background-color 180ms ease-in-out; }}",
            css_color
        ));

        *imp.board_color_hex.borrow_mut() = css_color.clone();
        let preview = imp.board_color_preview.borrow().clone();
        if let Some(preview) = preview.as_ref() {
            preview.queue_draw();
        }

        if persist {
            let settings = imp.settings.borrow().clone();
            if let Some(settings) = settings.as_ref() {
                let _ = settings.set_string(SETTINGS_KEY_BOARD_COLOR, &css_color);
            }
        }
    }

    fn on_timer_tick(&self) {
        let imp = self.imp();
        if imp.timer_started.get() {
            imp.elapsed_seconds.set(imp.elapsed_seconds.get() + 1);
            self.record_apm_sample_if_due();
            self.update_stats_label();
            if let Some(area) = imp.apm_graph_area.borrow().as_ref() {
                area.queue_draw();
            }
            self.update_apm_graph_chrome();
        }
    }

    fn current_apm(&self) -> f64 {
        let imp = self.imp();
        let elapsed = imp.elapsed_seconds.get();
        if elapsed == 0 {
            0.0
        } else {
            (imp.move_count.get() as f64 * 60.0) / elapsed as f64
        }
    }

    fn record_apm_sample_if_due(&self) {
        let imp = self.imp();
        let elapsed = imp.elapsed_seconds.get();
        if elapsed == 0 || elapsed % 5 != 0 {
            return;
        }
        let mut samples = imp.apm_samples.borrow_mut();
        if samples
            .last()
            .map(|sample| sample.elapsed_seconds == elapsed)
            .unwrap_or(false)
        {
            return;
        }
        samples.push(ApmSample {
            elapsed_seconds: elapsed,
            apm: self.current_apm(),
        });
    }

    #[allow(deprecated)]
    fn setup_handlers(&self) {
        let imp = self.imp();
        self.setup_smart_move_action();

        imp.help_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.show_help_dialog();
            }
        ));

        imp.undo_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.undo();
            }
        ));

        imp.redo_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.redo();
            }
        ));

        imp.auto_hint_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.play_hint_for_player();
            }
        ));
        let wand_middle_click = gtk::GestureClick::new();
        wand_middle_click.set_button(gdk::BUTTON_MIDDLE);
        wand_middle_click.connect_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _, _| {
                window.trigger_rapid_wand();
            }
        ));
        imp.auto_hint_button.add_controller(wand_middle_click);

        imp.cyclone_shuffle_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.cyclone_shuffle_tableau();
            }
        ));

        imp.peek_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.trigger_peek();
            }
        ));

        imp.seed_combo.connect_changed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |combo| {
                if window.imp().seed_combo_updating.get() {
                    return;
                }
                if let Some(seed) = combo.active_id() {
                    window.set_seed_input_text(seed.as_str());
                }
                window.clear_seed_entry_feedback();
                window.cancel_seed_winnable_check(None);
            }
        ));

        if let Some(seed_entry) = self.seed_text_entry() {
            seed_entry.set_placeholder_text(Some("Leave blank for random seed"));
            seed_entry.set_width_chars(1);
            seed_entry.connect_changed(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.clear_seed_entry_feedback();
                    window.cancel_seed_winnable_check(None);
                }
            ));
            seed_entry.connect_activate(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.start_new_game_from_seed_controls();
                }
            ));
        }

        imp.seed_random_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.start_random_seed_game();
            }
        ));

        imp.seed_rescue_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.start_random_winnable_seed_game();
            }
        ));

        imp.seed_winnable_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.toggle_seed_winnable_check();
            }
        ));

        imp.seed_repeat_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.repeat_current_seed_game();
            }
        ));

        imp.seed_go_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.start_new_game_from_seed_controls();
            }
        ));

        let stock_click = gtk::GestureClick::new();
        stock_click.connect_released(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _, _| {
                window.draw_card();
            }
        ));
        imp.stock_picture.add_controller(stock_click);

        let waste_click = gtk::GestureClick::new();
        waste_click.connect_released(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, n_press, _, _| {
                window.handle_waste_click(n_press);
            }
        ));
        imp.waste_overlay.add_controller(waste_click);

        for (index, stack) in self.tableau_stacks().into_iter().enumerate() {
            let click = gtk::GestureClick::new();
            click.connect_released(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, n_press, _, y| {
                    if n_press == 2 {
                        if window.smart_move_enabled() {
                            let start = {
                                let game = window.imp().game.borrow();
                                window.tableau_run_start_from_y(&game, index, y)
                            };
                            if let Some(start) = start {
                                window.try_smart_move_from_tableau(index, start);
                            }
                        }
                    } else {
                        let start = {
                            let game = window.imp().game.borrow();
                            window.tableau_run_start_from_y(&game, index, y)
                        };
                        window.select_or_move_tableau_with_start(index, start);
                    }
                }
            ));
            stack.add_controller(click);
        }

        for (index, foundation) in self.foundation_pictures().into_iter().enumerate() {
            let click = gtk::GestureClick::new();
            click.connect_released(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _, _, _| {
                    window.handle_click_on_foundation(index);
                }
            ));
            foundation.add_controller(click);
        }
        for (index, foundation) in self.foundation_placeholders().into_iter().enumerate() {
            let click = gtk::GestureClick::new();
            click.connect_released(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _, _, _| {
                    window.handle_click_on_foundation(index);
                }
            ));
            foundation.add_controller(click);
        }

        self.connect_notify_local(
            Some("width"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        self.connect_notify_local(
            Some("height"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        self.connect_notify_local(
            Some("maximized"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        imp.tableau_scroller.connect_notify_local(
            Some("width"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        imp.tableau_scroller.connect_notify_local(
            Some("height"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        self.add_tick_callback(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |widget, _| {
                let imp = window.imp();
                let width = widget.width();
                let height = widget.height();
                let scroller_width = imp.tableau_scroller.width();
                let scroller_height = imp.tableau_scroller.height();
                let maximized = window.is_maximized();
                if width > 0
                    && height > 0
                    && (width != imp.observed_window_width.get()
                        || height != imp.observed_window_height.get())
                {
                    imp.observed_window_width.set(width);
                    imp.observed_window_height.set(height);
                    imp.observed_scroller_width.set(scroller_width);
                    imp.observed_scroller_height.set(scroller_height);
                    imp.observed_maximized.set(maximized);
                    window.handle_window_geometry_change();
                } else if scroller_width > 0
                    && scroller_height > 0
                    && (scroller_width != imp.observed_scroller_width.get()
                        || scroller_height != imp.observed_scroller_height.get()
                        || maximized != imp.observed_maximized.get())
                {
                    imp.observed_scroller_width.set(scroller_width);
                    imp.observed_scroller_height.set(scroller_height);
                    imp.observed_maximized.set(maximized);
                    window.handle_window_geometry_change();
                }
                glib::ControlFlow::Continue
            }
        ));

        self.setup_board_color_dropdown();
        self.setup_game_mode_menu_item();
        self.setup_game_settings_menu();
        self.setup_drag_and_drop();
    }

    fn setup_smart_move_action(&self) {
        let initial_state = glib::Variant::from(self.imp().smart_move_enabled.get());
        let action = gio::SimpleAction::new_stateful("smart-move", None, &initial_state);
        action.connect_activate(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |action, _| {
                let next_enabled = !window.smart_move_enabled();
                action.change_state(&glib::Variant::from(next_enabled));
            }
        ));
        action.connect_change_state(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |action, value| {
                let Some(value) = value else {
                    return;
                };
                let Some(enabled) = value.get::<bool>() else {
                    return;
                };
                action.set_state(value);
                window.imp().smart_move_enabled.set(enabled);
                *window.imp().status_override.borrow_mut() = Some(if enabled {
                    "Smart Move enabled.".to_string()
                } else {
                    "Smart Move disabled.".to_string()
                });
                window.render();
            }
        ));
        self.add_action(&action);
    }

    fn smart_move_enabled(&self) -> bool {
        self.imp().smart_move_enabled.get()
    }

    fn accel_suffix_for_action(&self, action_name: &str) -> String {
        let Some(app) = self.application() else {
            return String::new();
        };
        let accels = app.accels_for_action(action_name);
        if accels.is_empty() {
            return String::new();
        }

        let labels: Vec<String> = accels
            .iter()
            .map(|accel| {
                if let Some((key, mods)) = gtk::accelerator_parse(accel) {
                    gtk::accelerator_get_label(key, mods).to_string()
                } else {
                    accel.to_string()
                }
            })
            .collect();
        format!(" ({})", labels.join(", "))
    }

    fn help_entries(&self) -> Vec<(String, String)> {
        let imp = self.imp();
        let mut rows = Vec::new();
        let mut push = |icon: &str, text: Option<String>, action: Option<&str>| {
            if let Some(text) = text {
                let suffix = action
                    .map(|name| self.accel_suffix_for_action(name))
                    .unwrap_or_default();
                rows.push((icon.to_string(), format!("{text}{suffix}")));
            }
        };

        push(
            "❓",
            imp.help_button.tooltip_text().map(|s| s.to_string()),
            Some("win.help"),
        );
        push(
            "↶",
            imp.undo_button.tooltip_text().map(|s| s.to_string()),
            Some("win.undo"),
        );
        push(
            "↷",
            imp.redo_button.tooltip_text().map(|s| s.to_string()),
            Some("win.redo"),
        );
        push(
            "🪄",
            imp.auto_hint_button.tooltip_text().map(|s| s.to_string()),
            Some("win.play-hint-move"),
        );
        push("⚡", Some("Rapid Wand".to_string()), Some("win.rapid-wand"));
        push(
            "🌀",
            imp.cyclone_shuffle_button
                .tooltip_text()
                .map(|s| s.to_string()),
            Some("win.cyclone-shuffle"),
        );
        push(
            "🫣",
            imp.peek_button.tooltip_text().map(|s| s.to_string()),
            Some("win.peek"),
        );
        push(
            "🎨",
            imp.board_color_menu_button
                .tooltip_text()
                .map(|s| s.to_string()),
            None,
        );
        push(
            imp.game_settings_menu_button
                .label()
                .as_deref()
                .unwrap_or("🎮"),
            imp.game_settings_menu_button
                .tooltip_text()
                .map(|s| s.to_string()),
            None,
        );
        push(
            "☰",
            imp.main_menu_button.tooltip_text().map(|s| s.to_string()),
            None,
        );
        push(
            "🎲",
            imp.seed_random_button.tooltip_text().map(|s| s.to_string()),
            Some("win.random-seed"),
        );
        push(
            "🛟",
            imp.seed_rescue_button.tooltip_text().map(|s| s.to_string()),
            Some("win.winnable-seed"),
        );
        push(
            "W?",
            imp.seed_winnable_button
                .tooltip_text()
                .map(|s| s.to_string()),
            None,
        );
        push(
            "🔁",
            imp.seed_repeat_button.tooltip_text().map(|s| s.to_string()),
            None,
        );
        push(
            "Go",
            imp.seed_go_button.tooltip_text().map(|s| s.to_string()),
            None,
        );
        push(
            "📈",
            Some("Show APM graph".to_string()),
            Some("win.apm-graph"),
        );

        rows
    }

    fn show_help_dialog(&self) {
        if let Some(existing) = self.imp().help_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let dialog = gtk::Window::builder()
            .title("Cardthropic Help")
            .transient_for(self)
            .modal(false)
            .default_width(520)
            .default_height(480)
            .build();
        dialog.set_hide_on_close(true);
        dialog.set_destroy_with_parent(true);

        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(glib::clone!(
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
        dialog.add_controller(key_controller);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(14);
        root.set_margin_bottom(14);
        root.set_margin_start(14);
        root.set_margin_end(14);

        let title = gtk::Label::new(Some("Controls"));
        title.set_xalign(0.0);
        title.add_css_class("title-4");
        root.append(&title);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_width(420)
            .min_content_height(300)
            .vexpand(true)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
        for (icon, text) in self.help_entries() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let icon_label = gtk::Label::new(Some(&icon));
            icon_label.set_width_chars(4);
            icon_label.set_xalign(0.0);
            let text_label = gtk::Label::new(Some(&text));
            text_label.set_xalign(0.0);
            text_label.set_wrap(true);
            text_label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            text_label.set_hexpand(true);
            row.append(&icon_label);
            row.append(&text_label);
            content.append(&row);
        }
        scrolled.set_child(Some(&content));
        root.append(&scrolled);

        let close = gtk::Button::with_label("Close");
        close.set_halign(gtk::Align::End);
        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        root.append(&close);

        dialog.set_child(Some(&root));
        *self.imp().help_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }

    fn apm_samples_for_graph(&self) -> Vec<ApmSample> {
        let imp = self.imp();
        let mut points = imp.apm_samples.borrow().clone();
        let elapsed = imp.elapsed_seconds.get();
        if elapsed > 0 {
            let current = ApmSample {
                elapsed_seconds: elapsed,
                apm: self.current_apm(),
            };
            if points
                .last()
                .map(|last| last.elapsed_seconds == current.elapsed_seconds)
                .unwrap_or(false)
            {
                if let Some(last) = points.last_mut() {
                    *last = current;
                }
            } else {
                points.push(current);
            }
        }
        points
    }

    fn draw_apm_graph(&self, cr: &gtk::cairo::Context, width: i32, height: i32) {
        let w = f64::from(width.max(1));
        let h = f64::from(height.max(1));
        cr.set_source_rgba(0.12, 0.14, 0.17, 1.0);
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();

        let left = 48.0;
        let right = 14.0;
        let top = 16.0;
        let bottom = 30.0;
        let plot_w = (w - left - right).max(1.0);
        let plot_h = (h - top - bottom).max(1.0);

        cr.set_source_rgba(1.0, 1.0, 1.0, 0.10);
        cr.rectangle(left, top, plot_w, plot_h);
        let _ = cr.stroke();

        let points = self.apm_samples_for_graph();
        if points.len() < 2 {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.75);
            cr.move_to(left + 8.0, top + 22.0);
            let _ = cr.show_text("Play for at least 10 seconds to plot APM.");
            return;
        }

        let max_t = points.last().map(|p| p.elapsed_seconds.max(1)).unwrap_or(1) as f64;
        let max_apm = points
            .iter()
            .fold(1.0_f64, |acc, p| acc.max(p.apm))
            .max(5.0)
            .ceil();

        cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
        for i in 1..=4 {
            let y = top + (plot_h * f64::from(i) / 4.0);
            cr.move_to(left, y);
            cr.line_to(left + plot_w, y);
            let _ = cr.stroke();
        }

        cr.set_source_rgba(0.35, 0.75, 1.0, 0.95);
        for (i, p) in points.iter().enumerate() {
            let x = left + ((p.elapsed_seconds as f64 / max_t) * plot_w);
            let y = top + (1.0 - (p.apm / max_apm).clamp(0.0, 1.0)) * plot_h;
            if i == 0 {
                cr.move_to(x, y);
            } else {
                cr.line_to(x, y);
            }
        }
        let _ = cr.stroke();

        if let Some(last) = points.last() {
            let x = left + ((last.elapsed_seconds as f64 / max_t) * plot_w);
            let y = top + (1.0 - (last.apm / max_apm).clamp(0.0, 1.0)) * plot_h;
            cr.arc(x, y, 3.5, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
        }

        cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
        cr.move_to(left, h - 10.0);
        let _ = cr.show_text("0s");
        cr.move_to(left + plot_w - 36.0, h - 10.0);
        let _ = cr.show_text(&format!("{max_t:.0}s"));
        cr.move_to(6.0, top + 4.0);
        let _ = cr.show_text(&format!("{max_apm:.0} APM"));
    }

    fn apm_summary(&self, points: &[ApmSample]) -> (f64, f64) {
        if points.is_empty() {
            return (0.0, 0.0);
        }
        let peak = points.iter().fold(0.0_f64, |acc, p| acc.max(p.apm));
        let avg = points.iter().map(|p| p.apm).sum::<f64>() / points.len() as f64;
        (peak, avg)
    }

    fn apm_tilt_badge(avg_apm: f64) -> &'static str {
        if avg_apm < 15.0 {
            "Calm"
        } else if avg_apm < 30.0 {
            "Focused"
        } else if avg_apm < 45.0 {
            "Turbo"
        } else {
            "Goblin Mode"
        }
    }

    fn apm_csv_string(&self) -> String {
        let points = self.apm_samples_for_graph();
        let mut rows = Vec::with_capacity(points.len() + 1);
        rows.push("elapsed_seconds,apm".to_string());
        rows.extend(
            points
                .iter()
                .map(|sample| format!("{},{}", sample.elapsed_seconds, sample.apm)),
        );
        rows.join("\n")
    }

    fn copy_apm_data_to_clipboard(&self) {
        if let Some(display) = gdk::Display::default() {
            let clipboard = display.clipboard();
            clipboard.set_text(&self.apm_csv_string());
            *self.imp().status_override.borrow_mut() =
                Some("Copied APM data to clipboard.".to_string());
            self.render();
        }
    }

    fn update_apm_graph_chrome(&self) {
        let imp = self.imp();
        let peak_label = imp.apm_peak_label.borrow().clone();
        let avg_label = imp.apm_avg_label.borrow().clone();
        let tilt_label = imp.apm_tilt_label.borrow().clone();
        if peak_label.is_none() && avg_label.is_none() && tilt_label.is_none() {
            return;
        }

        let points = self.apm_samples_for_graph();
        let (peak, avg) = self.apm_summary(&points);

        if let Some(label) = peak_label {
            label.set_label(&format!("Peak APM: {:.1}", peak));
        }
        if let Some(label) = avg_label {
            label.set_label(&format!("Average APM: {:.1}", avg));
        }
        if let Some(label) = tilt_label {
            label.set_label(&format!("Tilt: {}", Self::apm_tilt_badge(avg)));
        }
    }

    fn show_apm_graph_dialog(&self) {
        if let Some(existing) = self.imp().apm_graph_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let dialog = gtk::Window::builder()
            .title("APM Graph")
            .transient_for(self)
            .modal(false)
            .default_width(640)
            .default_height(360)
            .build();
        dialog.set_destroy_with_parent(true);
        dialog.set_hide_on_close(true);

        let stats_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let peak_label = gtk::Label::new(None);
        peak_label.set_xalign(0.0);
        let avg_label = gtk::Label::new(None);
        avg_label.set_xalign(0.0);
        let tilt_label = gtk::Label::new(None);
        tilt_label.set_xalign(0.0);
        tilt_label.add_css_class("accent");
        stats_row.append(&peak_label);
        stats_row.append(&avg_label);
        stats_row.append(&tilt_label);

        let graph = gtk::DrawingArea::new();
        graph.set_content_width(620);
        graph.set_content_height(320);
        graph.set_hexpand(true);
        graph.set_vexpand(true);
        graph.set_draw_func(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, cr, width, height| {
                window.draw_apm_graph(cr, width, height);
            }
        ));

        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.set_margin_top(10);
        root.set_margin_bottom(10);
        root.set_margin_start(10);
        root.set_margin_end(10);
        root.append(&stats_row);
        root.append(&graph);

        let actions_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions_row.set_halign(gtk::Align::End);
        let copy_button = gtk::Button::with_label("Copy Data");
        copy_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.copy_apm_data_to_clipboard();
            }
        ));
        actions_row.append(&copy_button);
        root.append(&actions_row);
        dialog.set_child(Some(&root));

        *self.imp().apm_peak_label.borrow_mut() = Some(peak_label);
        *self.imp().apm_avg_label.borrow_mut() = Some(avg_label);
        *self.imp().apm_tilt_label.borrow_mut() = Some(tilt_label);
        *self.imp().apm_graph_area.borrow_mut() = Some(graph);
        *self.imp().apm_graph_dialog.borrow_mut() = Some(dialog.clone());
        self.update_apm_graph_chrome();
        dialog.present();
    }

    fn setup_drag_and_drop(&self) {
        let imp = self.imp();

        let waste_hotspot = Rc::new(Cell::new((18_i32, 24_i32)));
        let waste_drag = gtk::DragSource::new();
        waste_drag.set_actions(gdk::DragAction::MOVE);
        waste_drag.connect_prepare(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[strong]
            waste_hotspot,
            #[upgrade_or]
            None,
            move |_, x, y| {
                if window.imp().game.borrow().waste_top().is_some() {
                    let imp = window.imp();
                    let max_x = (imp.card_width.get() - 1).max(0);
                    let max_y = (imp.card_height.get() - 1).max(0);
                    let hot_x = (x.round() as i32).clamp(0, max_x);
                    let hot_y = (y.round() as i32).clamp(0, max_y);
                    waste_hotspot.set((hot_x, hot_y));
                    Some(gdk::ContentProvider::for_value(&"waste".to_value()))
                } else {
                    None
                }
            }
        ));
        waste_drag.connect_drag_begin(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[strong]
            waste_hotspot,
            move |source, _| {
                let imp = window.imp();
                let game = imp.game.borrow();
                let deck_slot = imp.deck.borrow();
                let Some(deck) = deck_slot.as_ref() else {
                    return;
                };
                let Some(card) = game.waste_top() else {
                    return;
                };
                let card_width = imp.card_width.get().max(62);
                let card_height = imp.card_height.get().max(96);
                let texture = deck.texture_for_card_scaled(card, card_width, card_height);
                let (hot_x, hot_y) = waste_hotspot.get();
                source.set_icon(Some(&texture), hot_x, hot_y);
                window.start_drag(DragOrigin::Waste);
            }
        ));
        waste_drag.connect_drag_cancel(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            false,
            move |_, _, _| {
                window.finish_drag(false);
                false
            }
        ));
        waste_drag.connect_drag_end(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, delete_data| {
                window.finish_drag(delete_data);
            }
        ));
        imp.waste_overlay.add_controller(waste_drag);

        for (index, stack) in self.tableau_stacks().into_iter().enumerate() {
            let drag_start = Rc::new(Cell::new(None::<usize>));
            let drag_hotspot = Rc::new(Cell::new((18_i32, 24_i32)));
            let drag = gtk::DragSource::new();
            drag.set_actions(gdk::DragAction::MOVE);
            drag.connect_prepare(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                drag_start,
                #[strong]
                drag_hotspot,
                #[upgrade_or]
                None,
                move |_, x, y| {
                    let game = window.imp().game.borrow();
                    if let Some(start) = window.tableau_run_start_from_y(&game, index, y) {
                        let card_top = window.tableau_card_y_offset(&game, index, start);
                        let imp = window.imp();
                        let max_x = (imp.card_width.get() - 1).max(0);
                        let max_y = (imp.card_height.get() - 1).max(0);
                        let hot_x = (x.round() as i32).clamp(0, max_x);
                        let hot_y = ((y - f64::from(card_top)).round() as i32).clamp(0, max_y);
                        drag_hotspot.set((hot_x, hot_y));
                        drag_start.set(Some(start));
                        let payload = format!("tableau:{index}:{start}");
                        Some(gdk::ContentProvider::for_value(&payload.to_value()))
                    } else {
                        drag_start.set(None);
                        None
                    }
                }
            ));
            drag.connect_drag_begin(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                drag_start,
                #[strong]
                drag_hotspot,
                move |source, _| {
                    let Some(start) = drag_start.get() else {
                        return;
                    };
                    let imp = window.imp();
                    let game = imp.game.borrow();
                    let deck_slot = imp.deck.borrow();
                    let Some(deck) = deck_slot.as_ref() else {
                        return;
                    };
                    let Some(card) = game.tableau_card(index, start) else {
                        return;
                    };
                    let card_width = imp.card_width.get().max(62);
                    let card_height = imp.card_height.get().max(96);
                    let texture = window
                        .texture_for_tableau_drag_run(
                            &game,
                            deck,
                            index,
                            start,
                            card_width,
                            card_height,
                        )
                        .unwrap_or_else(|| {
                            if card.face_up {
                                deck.texture_for_card_scaled(card, card_width, card_height)
                            } else {
                                deck.back_texture_scaled(card_width, card_height)
                            }
                        });
                    let (hot_x, hot_y) = drag_hotspot.get();
                    source.set_icon(Some(&texture), hot_x, hot_y);
                    window.start_drag(DragOrigin::Tableau { col: index, start });
                }
            ));
            drag.connect_drag_cancel(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                false,
                move |_, _, _| {
                    window.finish_drag(false);
                    false
                }
            ));
            drag.connect_drag_end(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _, delete_data| {
                    window.finish_drag(delete_data);
                }
            ));
            stack.add_controller(drag);

            let tableau_drop = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
            tableau_drop.connect_drop(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    let Ok(payload) = value.get::<String>() else {
                        return false;
                    };
                    window.handle_drop_on_tableau(index, &payload)
                }
            ));
            stack.add_controller(tableau_drop);
        }

        for (index, foundation) in self.foundation_pictures().into_iter().enumerate() {
            let foundation_drop = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
            foundation_drop.connect_drop(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    let Ok(payload) = value.get::<String>() else {
                        return false;
                    };
                    window.handle_drop_on_foundation(index, &payload)
                }
            ));
            foundation.add_controller(foundation_drop);
        }
    }

    fn handle_window_geometry_change(&self) {
        let imp = self.imp();
        imp.geometry_render_dirty.set(true);
        if imp.geometry_render_pending.replace(true) {
            return;
        }

        glib::timeout_add_local(
            Duration::from_millis(16),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let imp = window.imp();
                    imp.geometry_render_pending.set(false);
                    if !imp.geometry_render_dirty.replace(false) {
                        return glib::ControlFlow::Break;
                    }
                    imp.last_metrics_key.set(0);
                    window.render();
                    if imp.geometry_render_dirty.get() {
                        window.handle_window_geometry_change();
                    }
                    glib::ControlFlow::Break
                }
            ),
        );
    }

    fn clear_seed_entry_feedback(&self) {
        if let Some(entry) = self.seed_text_entry() {
            entry.remove_css_class("error");
            entry.remove_css_class("seed-winnable");
            entry.remove_css_class("seed-unwinnable");
        }
    }

    fn seed_from_controls_or_random(&self) -> Result<u64, String> {
        let text = self.seed_input_text();
        let parsed = parse_seed_input(&text)?;
        let seed = parsed.unwrap_or_else(random_seed);
        if parsed.is_none() || text.trim() != seed.to_string() {
            self.set_seed_input_text(&seed.to_string());
        }
        Ok(seed)
    }

    fn start_new_game_from_seed_controls(&self) {
        if !self.guard_mode_engine("Starting a new deal") {
            return;
        }
        if self.imp().seed_search_in_progress.get() {
            *self.imp().status_override.borrow_mut() =
                Some("A winnable-seed search is still running. Please wait.".to_string());
            self.render();
            return;
        }

        self.cancel_seed_winnable_check(None);
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

        self.start_new_game_with_seed(seed, format!("Started a new game. Seed {seed}."));
    }

    fn start_random_seed_game(&self) {
        if !self.guard_mode_engine("Starting a random deal") {
            return;
        }
        if self.imp().seed_search_in_progress.get() {
            *self.imp().status_override.borrow_mut() =
                Some("A winnable-seed search is still running. Please wait.".to_string());
            self.render();
            return;
        }

        self.cancel_seed_winnable_check(None);
        self.clear_seed_entry_feedback();
        let seed = random_seed();
        self.start_new_game_with_seed(seed, format!("Started a new game. Seed {seed}."));
    }

    fn repeat_current_seed_game(&self) {
        if !self.guard_mode_engine("Repeating current seed") {
            return;
        }
        if self.imp().seed_search_in_progress.get() {
            *self.imp().status_override.borrow_mut() =
                Some("A winnable-seed search is still running. Please wait.".to_string());
            self.render();
            return;
        }

        self.cancel_seed_winnable_check(None);
        self.clear_seed_entry_feedback();
        let seed = self.imp().current_seed.get();
        self.set_seed_input_text(&seed.to_string());
        self.start_new_game_with_seed(seed, format!("Dealt again. Seed {seed}."));
    }

    fn start_random_winnable_seed_game(&self) {
        if !self.guard_mode_engine("Starting a winnable deal") {
            return;
        }
        if self.imp().seed_search_in_progress.get() {
            *self.imp().status_override.borrow_mut() =
                Some("A winnable-seed search is already running.".to_string());
            self.render();
            return;
        }

        self.cancel_seed_winnable_check(None);
        self.clear_seed_entry_feedback();
        let start_seed = random_seed();
        self.set_seed_input_text(&start_seed.to_string());

        self.begin_winnable_seed_search(
            start_seed,
            default_dialog_find_winnable_attempts(),
            DIALOG_FIND_WINNABLE_STATE_BUDGET,
        );
    }

    fn cancel_seed_winnable_check(&self, status: Option<&str>) {
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

    fn finish_seed_winnable_check(&self, generation: u64) -> bool {
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

    fn toggle_seed_winnable_check(&self) {
        if !self.guard_mode_engine("Winnability analysis") {
            return;
        }
        if self.imp().seed_check_running.get() {
            self.cancel_seed_winnable_check(Some("Winnability check canceled."));
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

        let (sender, receiver) = mpsc::channel::<Option<SeedWinnabilityCheckResult>>();
        let draw_mode = self.current_klondike_draw_mode();
        thread::spawn(move || {
            let result = is_seed_winnable_for_dialog(seed, draw_mode, &cancel_flag);
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
                                parse_seed_input(&window.seed_input_text()).ok().flatten();
                            if current_seed != Some(seed) {
                                return glib::ControlFlow::Break;
                            }

                            window.clear_seed_entry_feedback();
                            if result.winnable {
                                if let Some(entry) = window.seed_text_entry() {
                                    entry.add_css_class("seed-winnable");
                                }
                                let moves = result.moves_to_win.unwrap_or(0);
                                *window.imp().status_override.borrow_mut() = Some(format!(
                                    "Seed {seed} is winnable ({moves} moves, {} iterations).",
                                    result.iterations
                                ));
                            } else {
                                if let Some(entry) = window.seed_text_entry() {
                                    entry.add_css_class("seed-unwinnable");
                                }
                                let message = if result.hit_state_limit {
                                    format!(
                                        "Seed {seed} not proven winnable ({} iterations, limits hit).",
                                        result.iterations
                                    )
                                } else {
                                    format!(
                                        "Seed {seed} is not winnable ({} iterations).",
                                        result.iterations
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
                                    Some("Winnability check stopped unexpectedly.".to_string());
                                window.render();
                            }
                            glib::ControlFlow::Break
                        }
                    }
                }
            ),
        );
    }

    fn begin_winnable_seed_search(&self, start_seed: u64, attempts: u32, max_states: usize) {
        let imp = self.imp();
        if imp.seed_search_in_progress.replace(true) {
            *imp.status_override.borrow_mut() =
                Some("A winnable-seed search is already running.".to_string());
            self.render();
            return;
        }

        *imp.status_override.borrow_mut() = Some(format!(
            "Searching winnable seed from {start_seed} (attempts: {attempts}, state budget: {max_states})..."
        ));
        self.render();

        let (sender, receiver) = mpsc::channel::<Option<(u64, u32)>>();
        let draw_mode = self.current_klondike_draw_mode();
        thread::spawn(move || {
            let result = find_winnable_seed_parallel(start_seed, attempts, max_states, draw_mode);
            let _ = sender.send(result);
        });

        glib::timeout_add_local(
            Duration::from_millis(40),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || match receiver.try_recv() {
                    Ok(Some((seed, tested))) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        window.start_new_game_with_seed(
                            seed,
                            format!(
                                "Started winnable game. Seed {seed} (checked {tested} seed(s))."
                            ),
                        );
                        glib::ControlFlow::Break
                    }
                    Ok(None) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        *imp.status_override.borrow_mut() = Some(format!(
                            "No winnable seed found in {attempts} attempt(s) from seed {start_seed}."
                        ));
                        window.render();
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        *imp.status_override.borrow_mut() =
                            Some("Seed search stopped unexpectedly.".to_string());
                        window.render();
                        glib::ControlFlow::Break
                    }
                }
            ),
        );
    }

    fn start_new_game_with_seed(&self, seed: u64, status: String) {
        let imp = self.imp();
        self.cancel_seed_winnable_check(None);
        self.clear_hint_effects();
        *imp.game.borrow_mut() = KlondikeGame::new_with_seed(seed);
        imp.game
            .borrow_mut()
            .set_draw_mode(imp.klondike_draw_mode.get());
        imp.current_seed.set(seed);
        self.set_seed_input_text(&seed.to_string());
        self.clear_seed_entry_feedback();
        *imp.selected_run.borrow_mut() = None;
        imp.waste_selected.set(false);
        *imp.deck_error.borrow_mut() = None;
        *imp.status_override.borrow_mut() = Some(status);
        imp.history.borrow_mut().clear();
        imp.future.borrow_mut().clear();
        imp.apm_samples.borrow_mut().clear();
        imp.move_count.set(0);
        imp.elapsed_seconds.set(0);
        imp.timer_started.set(false);
        self.note_seed_play_started(seed);
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        let state_hash = self.current_game_hash();
        self.start_hint_loss_analysis_if_needed(state_hash);
        self.render();
    }

    fn snapshot(&self) -> Snapshot {
        let imp = self.imp();
        Snapshot {
            game: imp.game.borrow().clone(),
            selected_run: *imp.selected_run.borrow(),
            selected_waste: imp.waste_selected.get(),
            move_count: imp.move_count.get(),
            elapsed_seconds: imp.elapsed_seconds.get(),
            timer_started: imp.timer_started.get(),
            apm_samples: imp.apm_samples.borrow().clone(),
        }
    }

    fn apply_changed_move(&self, snapshot: Snapshot, changed: bool) -> bool {
        if changed {
            let imp = self.imp();
            self.clear_hint_effects();
            imp.waste_selected.set(false);
            imp.history.borrow_mut().push(snapshot);
            imp.future.borrow_mut().clear();
            imp.move_count.set(imp.move_count.get() + 1);
            imp.timer_started.set(true);
            *imp.status_override.borrow_mut() = None;
            self.note_current_state_for_hint_cycle();
            if imp.auto_playing_move.get() {
                self.note_current_state_for_auto_play();
            } else {
                self.reset_auto_play_memory();
            }
            let state_hash = self.current_game_hash();
            self.start_hint_loss_analysis_if_needed(state_hash);
            if imp.game.borrow().is_won() {
                imp.timer_started.set(false);
            }
        }
        changed
    }

    fn draw_card(&self) -> bool {
        if !self.guard_mode_engine("Draw") {
            return false;
        }
        let draw_mode = self.current_klondike_draw_mode();
        {
            let mut game = self.imp().game.borrow_mut();
            if game.draw_mode() != draw_mode {
                game.set_draw_mode(draw_mode);
            }
        }
        let snapshot = self.snapshot();
        let result = self
            .imp()
            .game
            .borrow_mut()
            .draw_or_recycle_with_count(draw_mode.count());
        let changed = match result {
            DrawResult::DrewFromStock => true,
            DrawResult::RecycledWaste => true,
            DrawResult::NoOp => false,
        };

        if !self.apply_changed_move(snapshot, changed) {
            *self.imp().status_override.borrow_mut() = Some("Nothing to draw.".to_string());
        }
        self.render();
        changed
    }

    fn cyclone_shuffle_tableau(&self) -> bool {
        if !self.guard_mode_engine("Cyclone shuffle") {
            return false;
        }

        let snapshot = self.snapshot();
        let changed = self.imp().game.borrow_mut().cyclone_shuffle_tableau();
        let changed = self.apply_changed_move(snapshot, changed);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            *self.imp().status_override.borrow_mut() = Some(
                "Cyclone shuffle complete: rerolled tableau while preserving each column's geometry."
                    .to_string(),
            );
        } else {
            *self.imp().status_override.borrow_mut() =
                Some("Cyclone shuffle had no effect.".to_string());
        }
        self.render();
        changed
    }

    fn trigger_peek(&self) {
        if !self.guard_mode_engine("Peek") {
            return;
        }
        let imp = self.imp();
        let generation = imp.peek_generation.get().wrapping_add(1);
        imp.peek_generation.set(generation);
        imp.peek_active.set(true);
        self.render();

        glib::timeout_add_local_once(
            Duration::from_secs(3),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move || {
                    let imp = window.imp();
                    if imp.peek_generation.get() != generation {
                        return;
                    }
                    imp.peek_active.set(false);
                    window.render();
                }
            ),
        );
    }

    fn move_waste_to_foundation(&self) -> bool {
        if !self.guard_mode_engine("Waste-to-foundation move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self.imp().game.borrow_mut().move_waste_to_foundation();
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    fn move_waste_to_tableau(&self, dst: usize) -> bool {
        if !self.guard_mode_engine("Waste-to-tableau move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self.imp().game.borrow_mut().move_waste_to_tableau(dst);
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    fn move_tableau_run_to_tableau(&self, src: usize, start: usize, dst: usize) -> bool {
        if !self.guard_mode_engine("Tableau move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self
            .imp()
            .game
            .borrow_mut()
            .move_tableau_run_to_tableau(src, start, dst);
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    fn move_tableau_to_foundation(&self, src: usize) -> bool {
        if !self.guard_mode_engine("Tableau-to-foundation move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self
            .imp()
            .game
            .borrow_mut()
            .move_tableau_top_to_foundation(src);
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    fn move_foundation_to_tableau(&self, foundation_idx: usize, dst: usize) -> bool {
        if !self.guard_mode_engine("Foundation-to-tableau move") {
            return false;
        }
        let snapshot = self.snapshot();
        let changed = self
            .imp()
            .game
            .borrow_mut()
            .move_foundation_top_to_tableau(foundation_idx, dst);
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    fn select_or_move_tableau_with_start(&self, clicked: usize, clicked_start: Option<usize>) {
        if !self.guard_mode_engine("Tableau selection") {
            return;
        }
        let imp = self.imp();
        if imp.waste_selected.get() {
            imp.waste_selected.set(false);
            let can_move = imp.game.borrow().can_move_waste_to_tableau(clicked);
            if can_move {
                self.move_waste_to_tableau(clicked);
            } else {
                *imp.status_override.borrow_mut() =
                    Some(format!("Waste card cannot move to T{}.", clicked + 1));
                self.render();
            }
            return;
        }
        let selected = *imp.selected_run.borrow();
        match selected {
            None => {
                if let Some(start) = clicked_start {
                    *imp.selected_run.borrow_mut() = Some(SelectedRun {
                        col: clicked,
                        start,
                    });
                }
            }
            Some(current) if current.col == clicked => {
                if clicked_start == Some(current.start) || clicked_start.is_none() {
                    *imp.selected_run.borrow_mut() = None;
                } else if let Some(start) = clicked_start {
                    *imp.selected_run.borrow_mut() = Some(SelectedRun {
                        col: clicked,
                        start,
                    });
                }
            }
            Some(current) => {
                self.move_tableau_run_to_tableau(current.col, current.start, clicked);
                *imp.selected_run.borrow_mut() = None;
            }
        }
        self.render();
    }

    fn handle_waste_click(&self, n_press: i32) {
        if !self.guard_mode_engine("Waste selection") {
            return;
        }
        let imp = self.imp();
        if imp.suppress_waste_click_once.replace(false) {
            return;
        }
        let has_waste = imp.game.borrow().waste_top().is_some();
        if !has_waste {
            imp.waste_selected.set(false);
            self.render();
            return;
        }

        if n_press == 2 {
            if self.smart_move_enabled() {
                imp.waste_selected.set(false);
                self.auto_play_waste();
            }
            return;
        }

        *imp.selected_run.borrow_mut() = None;
        imp.waste_selected.set(!imp.waste_selected.get());
        self.render();
    }

    fn handle_drop_on_tableau(&self, dst: usize, payload: &str) -> bool {
        let changed = if payload == "waste" {
            self.move_waste_to_tableau(dst)
        } else if let Some((src, start)) = parse_tableau_payload(payload) {
            self.move_tableau_run_to_tableau(src, start, dst)
        } else {
            false
        };

        if !changed {
            *self.imp().status_override.borrow_mut() =
                Some("That drag-and-drop move is not legal.".to_string());
            self.render();
        }
        changed
    }

    fn handle_drop_on_foundation(&self, foundation_idx: usize, payload: &str) -> bool {
        let changed = if payload == "waste" {
            let suit_ok = self
                .imp()
                .game
                .borrow()
                .waste_top()
                .map(|card| card.suit.foundation_index() == foundation_idx)
                .unwrap_or(false);
            suit_ok && self.move_waste_to_foundation()
        } else if let Some((src, _start)) = parse_tableau_payload(payload) {
            let suit_ok = self
                .imp()
                .game
                .borrow()
                .tableau_top(src)
                .map(|card| card.suit.foundation_index() == foundation_idx)
                .unwrap_or(false);
            suit_ok && self.move_tableau_to_foundation(src)
        } else {
            false
        };

        if !changed {
            *self.imp().status_override.borrow_mut() =
                Some("Drop to that foundation was not legal.".to_string());
            self.render();
        }
        changed
    }

    fn handle_click_on_foundation(&self, foundation_idx: usize) {
        if !self.guard_mode_engine("Foundation move") {
            return;
        }

        let imp = self.imp();
        let mut did_move = false;

        if imp.waste_selected.get() {
            let suit_ok = imp
                .game
                .borrow()
                .waste_top()
                .map(|card| card.suit.foundation_index() == foundation_idx)
                .unwrap_or(false);
            if suit_ok {
                did_move = self.move_waste_to_foundation();
                if did_move {
                    return;
                }
            }
        }

        let selected_run = { *imp.selected_run.borrow() };
        if let Some(selected) = selected_run {
            let selected_is_top = imp
                .game
                .borrow()
                .tableau_len(selected.col)
                .map(|len| selected.start + 1 == len)
                .unwrap_or(false);
            if !selected_is_top {
                *imp.status_override.borrow_mut() =
                    Some("Only the top card of a tableau can move to foundation.".to_string());
                self.render();
                return;
            }
            let suit_ok = imp
                .game
                .borrow()
                .tableau_top(selected.col)
                .map(|card| card.suit.foundation_index() == foundation_idx)
                .unwrap_or(false);
            if suit_ok {
                did_move = self.move_tableau_to_foundation(selected.col);
            }
            if did_move {
                *imp.selected_run.borrow_mut() = None;
                return;
            }
        }

        // Easy-mode foundation pull-back: click a foundation to move its top card back
        // to the first legal tableau column when nothing is selected.
        let foundation_top_exists = imp
            .game
            .borrow()
            .foundations()
            .get(foundation_idx)
            .map(|pile| !pile.is_empty())
            .unwrap_or(false);
        if foundation_top_exists {
            for dst in 0..7 {
                let can_move = imp
                    .game
                    .borrow()
                    .can_move_foundation_top_to_tableau(foundation_idx, dst);
                if can_move && self.move_foundation_to_tableau(foundation_idx, dst) {
                    *imp.status_override.borrow_mut() =
                        Some(format!("Moved foundation card to T{}.", dst + 1));
                    self.render();
                    return;
                }
            }
        }

        *imp.status_override.borrow_mut() =
            Some("That move to foundation is not legal.".to_string());
        self.render();
    }

    fn auto_play_waste(&self) {
        if !self.smart_move_enabled() {
            return;
        }
        if !self.guard_mode_engine("Waste auto-play") {
            return;
        }
        if self.try_auto_move_waste_to_foundation() {
            return;
        }
        for dst in 0..7 {
            if self.imp().game.borrow().can_move_waste_to_tableau(dst)
                && self.move_waste_to_tableau(dst)
            {
                return;
            }
        }
        *self.imp().status_override.borrow_mut() = Some("No automatic move for waste.".to_string());
        self.render();
    }

    fn undo(&self) {
        if !self.guard_mode_engine("Undo") {
            return;
        }
        let imp = self.imp();
        let Some(snapshot) = imp.history.borrow_mut().pop() else {
            *imp.status_override.borrow_mut() = Some("Nothing to undo yet.".to_string());
            self.render();
            return;
        };

        self.clear_hint_effects();
        imp.future.borrow_mut().push(self.snapshot());
        self.restore_snapshot(snapshot);
        *imp.status_override.borrow_mut() = Some("Undid last move.".to_string());
        self.render();
    }

    fn redo(&self) {
        if !self.guard_mode_engine("Redo") {
            return;
        }
        let imp = self.imp();
        let Some(snapshot) = imp.future.borrow_mut().pop() else {
            *imp.status_override.borrow_mut() = Some("Nothing to redo yet.".to_string());
            self.render();
            return;
        };

        self.clear_hint_effects();
        imp.history.borrow_mut().push(self.snapshot());
        self.restore_snapshot(snapshot);
        *imp.status_override.borrow_mut() = Some("Redid move.".to_string());
        self.render();
    }

    fn restore_snapshot(&self, snapshot: Snapshot) {
        let imp = self.imp();
        *imp.game.borrow_mut() = snapshot.game;
        imp.klondike_draw_mode.set(imp.game.borrow().draw_mode());
        *imp.selected_run.borrow_mut() = snapshot.selected_run;
        imp.waste_selected.set(snapshot.selected_waste);
        imp.move_count.set(snapshot.move_count);
        imp.elapsed_seconds.set(snapshot.elapsed_seconds);
        imp.timer_started.set(snapshot.timer_started);
        *imp.apm_samples.borrow_mut() = snapshot.apm_samples;
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        let state_hash = self.current_game_hash();
        self.start_hint_loss_analysis_if_needed(state_hash);
    }

    #[allow(dead_code)]
    fn show_hint(&self) {
        if !self.guard_mode_engine("Hint") {
            return;
        }
        let suggestion = self.compute_hint_suggestion();
        *self.imp().status_override.borrow_mut() = Some(suggestion.message);
        self.render();
        if let (Some(source), Some(target)) = (suggestion.source, suggestion.target) {
            self.play_hint_animation(source, target);
        }
    }

    fn play_hint_for_player(&self) {
        if !self.guard_mode_engine("Play hint move") {
            return;
        }
        let suggestion = self.compute_auto_play_suggestion();
        let Some(hint_move) = suggestion.hint_move else {
            *self.imp().status_override.borrow_mut() = Some(suggestion.message);
            self.render();
            return;
        };

        self.imp().auto_playing_move.set(true);
        let changed = self.apply_hint_move(hint_move);
        self.imp().auto_playing_move.set(false);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            *self.imp().status_override.borrow_mut() =
                Some(format!("Auto: {}", suggestion.message));
            self.render();
            if let (Some(source), Some(target)) = (suggestion.source, suggestion.target) {
                self.play_hint_animation(source, target);
            }
        } else {
            *self.imp().status_override.borrow_mut() =
                Some("Auto-hint move was not legal anymore.".to_string());
            self.render();
        }
    }

    fn trigger_rapid_wand(&self) {
        if !self.guard_mode_engine("Rapid Wand") {
            return;
        }
        if self.imp().rapid_wand_running.get() {
            return;
        }
        self.imp().rapid_wand_running.set(true);

        self.stop_rapid_wand();
        self.imp().rapid_wand_running.set(true);

        self.play_hint_for_player();
        let remaining_steps = Rc::new(Cell::new(4_u8));
        let timer = glib::timeout_add_local(
            Duration::from_millis(750),
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

    fn stop_rapid_wand(&self) {
        self.imp().rapid_wand_running.set(false);
        if let Some(source_id) = self.imp().rapid_wand_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
    }

    fn finish_rapid_wand(&self) {
        self.imp().rapid_wand_running.set(false);
        let _ = self.imp().rapid_wand_timer.borrow_mut().take();
    }

    fn apply_hint_move(&self, hint_move: HintMove) -> bool {
        match hint_move {
            HintMove::WasteToFoundation => self.move_waste_to_foundation(),
            HintMove::TableauTopToFoundation { src } => self.move_tableau_to_foundation(src),
            HintMove::WasteToTableau { dst } => self.move_waste_to_tableau(dst),
            HintMove::TableauRunToTableau { src, start, dst } => {
                self.move_tableau_run_to_tableau(src, start, dst)
            }
            HintMove::Draw => self.draw_card(),
        }
    }

    fn best_smart_move_for_source(&self, source: HintNode) -> Option<HintSuggestion> {
        let game = self.imp().game.borrow().clone();
        let state_hash = hash_game_state(&game);
        let seen_states: HashSet<u64> = self
            .imp()
            .hint_recent_states
            .borrow()
            .iter()
            .copied()
            .chain(std::iter::once(state_hash))
            .collect();

        let mut best: Option<(i64, HintSuggestion)> = None;
        for candidate in self.enumerate_hint_candidates(&game) {
            if candidate.source != Some(source) {
                continue;
            }
            let Some(hint_move) = candidate.hint_move else {
                continue;
            };

            let mut next_state = game.clone();
            if !apply_hint_move_to_game(&mut next_state, hint_move) {
                continue;
            }

            let next_hash = hash_game_state(&next_state);
            if seen_states.contains(&next_hash) {
                continue;
            }
            if self.is_functionally_useless_auto_move(&game, &next_state, hint_move, &seen_states) {
                continue;
            }

            let mut score =
                score_hint_candidate(&game, &next_state, hint_move, &seen_states, next_hash);
            if next_state.is_won() {
                score += AUTO_PLAY_WIN_SCORE;
            }
            if self.is_king_to_empty_without_unlock(&game, hint_move) {
                score -= 4_000;
            }

            match &best {
                None => best = Some((score, candidate)),
                Some((best_score, _)) if score > *best_score => best = Some((score, candidate)),
                _ => {}
            }
        }

        best.map(|(_, suggestion)| suggestion)
    }

    fn try_smart_move_from_tableau(&self, col: usize, start: usize) -> bool {
        if !self.guard_mode_engine("Smart Move") {
            return false;
        }
        let source = HintNode::Tableau {
            col,
            index: Some(start),
        };
        let Some(suggestion) = self.best_smart_move_for_source(source) else {
            *self.imp().status_override.borrow_mut() =
                Some("Smart Move: no legal move from that card.".to_string());
            self.render();
            return false;
        };
        let Some(hint_move) = suggestion.hint_move else {
            *self.imp().status_override.borrow_mut() =
                Some("Smart Move: no legal move from that card.".to_string());
            self.render();
            return false;
        };

        let changed = self.apply_hint_move(hint_move);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            let message = suggestion
                .message
                .strip_prefix("Hint: ")
                .unwrap_or(suggestion.message.as_str());
            *self.imp().status_override.borrow_mut() = Some(format!("Smart Move: {message}"));
            self.render();
        }
        changed
    }

    fn render(&self) {
        let imp = self.imp();
        let game = imp.game.borrow();
        let mode = self.active_game_mode();
        let engine_ready = self.is_mode_engine_ready();
        if engine_ready {
            self.note_current_seed_win_if_needed(&game);
            if game.is_won() && imp.timer_started.get() {
                imp.timer_started.set(false);
            }
        }

        imp.stock_label
            .set_label(&format!("{} cards", game.stock_len()));

        imp.waste_label
            .set_label(&format!("{} cards", game.waste_len()));

        let foundation_labels = [
            &imp.foundation_label_1,
            &imp.foundation_label_2,
            &imp.foundation_label_3,
            &imp.foundation_label_4,
        ];

        for label in foundation_labels {
            label.set_label("");
        }

        self.render_card_images(&game);

        imp.undo_button
            .set_sensitive(engine_ready && !imp.history.borrow().is_empty());
        imp.redo_button
            .set_sensitive(engine_ready && !imp.future.borrow().is_empty());
        imp.auto_hint_button.set_sensitive(engine_ready);
        imp.cyclone_shuffle_button.set_sensitive(engine_ready);
        imp.peek_button.set_sensitive(engine_ready);
        imp.seed_random_button.set_sensitive(engine_ready);
        imp.seed_rescue_button.set_sensitive(engine_ready);
        imp.seed_winnable_button.set_sensitive(engine_ready);
        imp.seed_repeat_button.set_sensitive(engine_ready);
        imp.seed_go_button.set_sensitive(engine_ready);
        imp.seed_combo.set_sensitive(engine_ready);

        let selected = sanitize_selected_run(&game, *imp.selected_run.borrow());
        *imp.selected_run.borrow_mut() = selected;
        self.update_tableau_selection_styles(selected);
        if imp.waste_selected.get() && game.waste_top().is_none() {
            imp.waste_selected.set(false);
        }
        self.update_waste_selection_style(imp.waste_selected.get() && game.waste_top().is_some());
        if let Some(err) = imp.deck_error.borrow().as_ref() {
            imp.status_label
                .set_label(&format!("Card deck load failed: {err}"));
        } else if let Some(message) = imp.status_override.borrow().as_ref() {
            imp.status_label.set_label(message);
        } else if game.is_won() {
            imp.status_label
                .set_label("You won! All foundations are complete.");
        } else if let Some(run) = selected {
            let amount = game
                .tableau_len(run.col)
                .unwrap_or(0)
                .saturating_sub(run.start);
            if amount > 1 {
                imp.status_label.set_label(&format!(
                    "Selected {amount} cards from T{}. Click another tableau to move this run.",
                    run.col + 1
                ));
            } else {
                imp.status_label.set_label(&format!(
                    "Selected tableau T{}. Click another tableau to move top card.",
                    run.col + 1
                ));
            }
        } else if imp.waste_selected.get() && game.waste_top().is_some() {
            imp.status_label.set_label(
                "Selected waste. Click a tableau to move it, or click waste again to cancel.",
            );
        } else if imp.peek_active.get() {
            imp.status_label.set_label(
                "Peek active: tableau face-up cards are hidden and face-down cards are revealed.",
            );
        } else if !engine_ready {
            imp.status_label.set_label(&format!(
                "{} mode scaffolded. Rules/engine are in progress.",
                mode.label()
            ));
        } else {
            imp.status_label
                .set_label(if self.smart_move_enabled() {
                    "Klondike controls: click columns to move, click waste to select, double-click cards/waste for Smart Move."
                } else {
                    "Klondike controls: click columns/waste to select and move manually. Smart Move is off (double-click disabled)."
                });
        }

        self.update_stats_label();
    }

    fn compute_auto_play_suggestion(&self) -> HintSuggestion {
        let mut game = self.imp().game.borrow().clone();
        game.set_draw_mode(self.current_klondike_draw_mode());
        if game.is_won() {
            return HintSuggestion {
                message: "Auto-play: game already won.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let state_hash = hash_game_state(&game);
        if let Some(LossVerdict::Lost { explored_states }) =
            self.cached_loss_verdict_for_hash(state_hash)
        {
            return HintSuggestion {
                message: format!(
                    "Auto-play: no winning path found from this position (explored {explored_states} states). Game is lost."
                ),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        self.imp()
            .auto_play_seen_states
            .borrow_mut()
            .insert(state_hash);
        let seen_states = self.imp().auto_play_seen_states.borrow().clone();

        let candidates = self.enumerate_hint_candidates(&game);
        if candidates.is_empty() {
            self.start_hint_loss_analysis_if_needed(state_hash);
            return HintSuggestion {
                message: "Auto-play: no legal moves remain. Game is lost.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let mut best: Option<(i64, HintSuggestion)> = None;
        let mut node_budget = AUTO_PLAY_NODE_BUDGET;
        for candidate in candidates {
            let Some(hint_move) = candidate.hint_move else {
                continue;
            };

            let mut next_state = game.clone();
            if !apply_hint_move_to_game(&mut next_state, hint_move) {
                continue;
            }

            let next_hash = hash_game_state(&next_state);
            if seen_states.contains(&next_hash) {
                continue;
            }

            if self.is_functionally_useless_auto_move(&game, &next_state, hint_move, &seen_states) {
                continue;
            }

            let mut score =
                score_hint_candidate(&game, &next_state, hint_move, &seen_states, next_hash);
            score += self.unseen_followup_count(&next_state, &seen_states) * 35;
            if next_state.is_won() {
                score += AUTO_PLAY_WIN_SCORE;
            }

            let mut path_seen = HashSet::new();
            path_seen.insert(state_hash);
            path_seen.insert(next_hash);
            let lookahead = self.auto_play_lookahead_score(
                &next_state,
                &seen_states,
                &mut path_seen,
                AUTO_PLAY_LOOKAHEAD_DEPTH.saturating_sub(1),
                &mut node_budget,
            );
            score += lookahead / 3;

            if self.is_king_to_empty_without_unlock(&game, hint_move) {
                score -= 4_000;
            }

            match &best {
                None => best = Some((score, candidate)),
                Some((best_score, _)) if score > *best_score => best = Some((score, candidate)),
                _ => {}
            }
        }

        if let Some((_, suggestion)) = best {
            suggestion
        } else {
            self.start_hint_loss_analysis_if_needed(state_hash);
            HintSuggestion {
                message: "Auto-play: no productive moves remain from this line. Game is lost."
                    .to_string(),
                source: None,
                target: None,
                hint_move: None,
            }
        }
    }

    #[allow(dead_code)]
    fn compute_hint_suggestion(&self) -> HintSuggestion {
        let mut game = self.imp().game.borrow().clone();
        game.set_draw_mode(self.current_klondike_draw_mode());
        if game.is_won() {
            return HintSuggestion {
                message: "Hint: game already won.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let state_hash = hash_game_state(&game);
        let seen_states: HashSet<u64> = self
            .imp()
            .hint_recent_states
            .borrow()
            .iter()
            .copied()
            .chain(std::iter::once(state_hash))
            .collect();

        if let Some(LossVerdict::Lost { explored_states }) =
            self.cached_loss_verdict_for_hash(state_hash)
        {
            return HintSuggestion {
                message: format!(
                    "Hint: no winning path found from this position (explored {explored_states} states)."
                ),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let candidates = self.enumerate_hint_candidates(&game);
        if candidates.is_empty() {
            self.start_hint_loss_analysis_if_needed(state_hash);
            return match self.cached_loss_verdict_for_hash(state_hash) {
                Some(LossVerdict::Lost { explored_states }) => HintSuggestion {
                    message: format!(
                        "Hint: no legal moves and no winning path found (explored {explored_states} states)."
                    ),
                    source: None,
                    target: None,
                    hint_move: None,
                },
                Some(LossVerdict::Inconclusive { explored_states }) => HintSuggestion {
                    message: format!(
                        "Hint: no legal moves. Analysis explored {explored_states} states but is inconclusive."
                    ),
                    source: None,
                    target: None,
                    hint_move: None,
                },
                Some(LossVerdict::WinnableLikely) => HintSuggestion {
                    message: "Hint: no legal move from here. Try undo/new game.".to_string(),
                    source: None,
                    target: None,
                    hint_move: None,
                },
                None => HintSuggestion {
                    message: "Hint: no legal moves. Running deeper loss analysis...".to_string(),
                    source: None,
                    target: None,
                    hint_move: None,
                },
            };
        }

        let mut best: Option<(i64, HintSuggestion)> = None;
        let mut node_budget = AUTO_PLAY_NODE_BUDGET;
        for candidate in candidates {
            let Some(hint_move) = candidate.hint_move else {
                continue;
            };

            let mut next_state = game.clone();
            if !apply_hint_move_to_game(&mut next_state, hint_move) {
                continue;
            }

            let next_hash = hash_game_state(&next_state);
            if seen_states.contains(&next_hash) {
                continue;
            }
            if self.is_functionally_useless_auto_move(&game, &next_state, hint_move, &seen_states) {
                continue;
            }

            let mut score =
                score_hint_candidate(&game, &next_state, hint_move, &seen_states, next_hash);
            score += self.unseen_followup_count(&next_state, &seen_states) * 35;
            if next_state.is_won() {
                score += AUTO_PLAY_WIN_SCORE;
            }
            let mut path_seen = HashSet::new();
            path_seen.insert(state_hash);
            path_seen.insert(next_hash);
            let lookahead = self.auto_play_lookahead_score(
                &next_state,
                &seen_states,
                &mut path_seen,
                AUTO_PLAY_LOOKAHEAD_DEPTH.saturating_sub(1),
                &mut node_budget,
            );
            score += lookahead / 3;
            if self.is_king_to_empty_without_unlock(&game, hint_move) {
                score -= 4_000;
            }

            match &best {
                None => best = Some((score, candidate)),
                Some((best_score, _)) if score > *best_score => best = Some((score, candidate)),
                _ => {}
            }
        }

        if let Some((_, suggestion)) = best {
            suggestion
        } else {
            self.start_hint_loss_analysis_if_needed(state_hash);
            HintSuggestion {
                message:
                    "Hint: no productive move found from this position. The line appears lost."
                        .to_string(),
                source: None,
                target: None,
                hint_move: None,
            }
        }
    }

    fn enumerate_hint_candidates(&self, game: &KlondikeGame) -> Vec<HintSuggestion> {
        let mut candidates = Vec::new();

        if self.can_auto_move_waste_to_foundation(game) {
            let foundation = game
                .waste_top()
                .map(|card| card.suit.foundation_index())
                .unwrap_or(0);
            candidates.push(HintSuggestion {
                message: "Hint: Move waste to foundation.".to_string(),
                source: Some(HintNode::Waste),
                target: Some(HintNode::Foundation(foundation)),
                hint_move: Some(HintMove::WasteToFoundation),
            });
        }

        for src in 0..7 {
            if !self.can_auto_move_tableau_to_foundation(game, src) {
                continue;
            }
            let foundation = game
                .tableau_top(src)
                .map(|card| card.suit.foundation_index())
                .unwrap_or(0);
            let len = game.tableau_len(src).unwrap_or(1);
            candidates.push(HintSuggestion {
                message: format!("Hint: Move T{} top card to foundation.", src + 1),
                source: Some(HintNode::Tableau {
                    col: src,
                    index: len.checked_sub(1),
                }),
                target: Some(HintNode::Foundation(foundation)),
                hint_move: Some(HintMove::TableauTopToFoundation { src }),
            });
        }

        for dst in 0..7 {
            if game.can_move_waste_to_tableau(dst) {
                candidates.push(HintSuggestion {
                    message: format!("Hint: Move waste card to T{}.", dst + 1),
                    source: Some(HintNode::Waste),
                    target: Some(HintNode::Tableau {
                        col: dst,
                        index: None,
                    }),
                    hint_move: Some(HintMove::WasteToTableau { dst }),
                });
            }
        }

        for src in 0..7 {
            let len = game.tableau_len(src).unwrap_or(0);
            for start in 0..len {
                for dst in 0..7 {
                    if !game.can_move_tableau_run_to_tableau(src, start, dst) {
                        continue;
                    }
                    let amount = len.saturating_sub(start);
                    candidates.push(HintSuggestion {
                        message: format!(
                            "Hint: Move {amount} card(s) T{} -> T{}.",
                            src + 1,
                            dst + 1
                        ),
                        source: Some(HintNode::Tableau {
                            col: src,
                            index: Some(start),
                        }),
                        target: Some(HintNode::Tableau {
                            col: dst,
                            index: None,
                        }),
                        hint_move: Some(HintMove::TableauRunToTableau { src, start, dst }),
                    });
                }
            }
        }

        if game.stock_len() > 0 || game.waste_top().is_some() {
            candidates.push(HintSuggestion {
                message: "Hint: Draw from stock.".to_string(),
                source: Some(HintNode::Stock),
                target: Some(HintNode::Stock),
                hint_move: Some(HintMove::Draw),
            });
        }

        candidates
    }

    fn play_hint_animation(&self, source: HintNode, target: HintNode) {
        self.clear_hint_effects();
        self.set_hint_node_active(source, true);

        let id_1 = glib::timeout_add_local(
            Duration::from_millis(1000),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    window.set_hint_node_active(source, false);
                    window.set_hint_node_active(target, true);
                    glib::ControlFlow::Break
                }
            ),
        );

        let id_2 = glib::timeout_add_local(
            Duration::from_millis(2000),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    window.set_hint_node_active(target, false);
                    glib::ControlFlow::Break
                }
            ),
        );

        let imp = self.imp();
        imp.hint_timeouts.borrow_mut().push(id_1);
        imp.hint_timeouts.borrow_mut().push(id_2);
    }

    fn clear_hint_effects(&self) {
        let imp = self.imp();
        for timeout_id in imp.hint_timeouts.borrow_mut().drain(..) {
            Self::remove_source_if_present(timeout_id);
        }
        for widget in imp.hint_widgets.borrow_mut().drain(..) {
            widget.remove_css_class("hint-invert");
        }
    }

    fn set_hint_node_active(&self, node: HintNode, active: bool) {
        if let Some(widget) = self.widget_for_hint_node(node) {
            if active {
                widget.add_css_class("hint-invert");
                self.imp().hint_widgets.borrow_mut().push(widget);
            } else {
                widget.remove_css_class("hint-invert");
            }
        }
    }

    fn widget_for_hint_node(&self, node: HintNode) -> Option<gtk::Widget> {
        let imp = self.imp();
        match node {
            HintNode::Stock => Some(imp.stock_picture.get().upcast()),
            HintNode::Waste => Some(imp.waste_picture.get().upcast()),
            HintNode::Foundation(index) => self
                .foundation_pictures()
                .get(index)
                .map(|picture| picture.clone().upcast()),
            HintNode::Tableau { col, index } => {
                if let Some(card_index) = index {
                    imp.tableau_card_pictures
                        .borrow()
                        .get(col)?
                        .get(card_index)
                        .map(|picture| picture.clone().upcast())
                } else {
                    self.tableau_stacks()
                        .get(col)
                        .map(|stack| stack.clone().upcast())
                }
            }
        }
    }

    fn tableau_card_y_offset(&self, game: &KlondikeGame, col: usize, index: usize) -> i32 {
        let mut y = 0_i32;
        let face_up_step = self.imp().face_up_step.get();
        let face_down_step = self.imp().face_down_step.get();
        for idx in 0..index {
            if let Some(card) = game.tableau_card(col, idx) {
                y += if card.face_up {
                    face_up_step
                } else {
                    face_down_step
                };
            }
        }
        y
    }

    fn texture_for_tableau_drag_run(
        &self,
        game: &KlondikeGame,
        deck: &AngloDeck,
        col: usize,
        start: usize,
        card_width: i32,
        card_height: i32,
    ) -> Option<gdk::Texture> {
        let len = game.tableau_len(col)?;
        if start >= len {
            return None;
        }

        let face_up_step = self.imp().face_up_step.get();
        let face_down_step = self.imp().face_down_step.get();
        let mut y = 0_i32;
        let mut layers: Vec<(gdk_pixbuf::Pixbuf, i32)> = Vec::new();

        for idx in start..len {
            let card = game.tableau_card(col, idx)?;
            let pixbuf = if card.face_up {
                deck.pixbuf_for_card_scaled(card, card_width, card_height)
            } else {
                deck.back_pixbuf_scaled(card_width, card_height)
            };
            layers.push((pixbuf, y));
            y += if card.face_up {
                face_up_step
            } else {
                face_down_step
            };
        }

        let run_height = layers
            .last()
            .map(|(_, pos_y)| pos_y + card_height)
            .unwrap_or(card_height)
            .max(card_height);
        let composed = gdk_pixbuf::Pixbuf::new(
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            card_width.max(1),
            run_height.max(1),
        )?;
        composed.fill(0x00000000);

        for (layer, pos_y) in layers {
            layer.copy_area(0, 0, card_width, card_height, &composed, 0, pos_y);
        }

        Some(gdk::Texture::for_pixbuf(&composed))
    }

    fn start_drag(&self, origin: DragOrigin) {
        self.cancel_drag_timeouts();
        let imp = self.imp();
        *imp.drag_origin.borrow_mut() = Some(origin);
        imp.drag_widgets.borrow_mut().clear();

        match origin {
            DragOrigin::Waste => {
                let game = imp.game.borrow();
                let visible_waste = game
                    .waste_len()
                    .min(usize::from(game.draw_mode().count().clamp(1, 5)));
                drop(game);
                if visible_waste > 0 {
                    let slots = self.waste_fan_slots();
                    let widget: gtk::Widget = slots[visible_waste - 1].clone().upcast();
                    widget.set_opacity(0.0);
                    imp.drag_widgets.borrow_mut().push(widget);
                }
            }
            DragOrigin::Tableau { col, start } => {
                if let Some(cards) = imp.tableau_card_pictures.borrow().get(col) {
                    let mut dragged = imp.drag_widgets.borrow_mut();
                    for picture in cards.iter().skip(start) {
                        let widget: gtk::Widget = picture.clone().upcast();
                        widget.set_opacity(0.0);
                        dragged.push(widget);
                    }
                }
            }
        }
    }

    fn finish_drag(&self, delete_data: bool) {
        let origin = self.imp().drag_origin.borrow_mut().take();
        if origin.is_none() {
            return;
        }
        if matches!(origin, Some(DragOrigin::Waste)) {
            self.imp().suppress_waste_click_once.set(true);
        }
        self.restore_drag_widgets(!delete_data);
    }

    fn restore_drag_widgets(&self, animate: bool) {
        let widgets: Vec<gtk::Widget> = self.imp().drag_widgets.borrow_mut().drain(..).collect();
        for widget in &widgets {
            widget.set_opacity(1.0);
        }
        if !animate || widgets.is_empty() {
            return;
        }

        for widget in &widgets {
            widget.add_css_class("drag-return");
        }

        let widgets_for_timeout = widgets;
        let timeout = glib::timeout_add_local(Duration::from_millis(16), move || {
            for widget in &widgets_for_timeout {
                widget.remove_css_class("drag-return");
            }
            glib::ControlFlow::Break
        });
        self.imp().drag_timeouts.borrow_mut().push(timeout);
    }

    fn cancel_drag_timeouts(&self) {
        for timeout_id in self.imp().drag_timeouts.borrow_mut().drain(..) {
            Self::remove_source_if_present(timeout_id);
        }
    }

    fn remove_source_if_present(source_id: glib::SourceId) {
        if glib::MainContext::default()
            .find_source_by_id(&source_id)
            .is_some()
        {
            source_id.remove();
        }
    }

    fn reset_hint_cycle_memory(&self) {
        let mut recent = self.imp().hint_recent_states.borrow_mut();
        recent.clear();
        drop(recent);
        self.note_current_state_for_hint_cycle();
    }

    fn note_current_state_for_hint_cycle(&self) {
        let hash = {
            let game = self.imp().game.borrow();
            hash_game_state(&game)
        };
        let mut recent = self.imp().hint_recent_states.borrow_mut();
        if recent.back().copied() == Some(hash) {
            return;
        }
        recent.push_back(hash);
        while recent.len() > 48 {
            recent.pop_front();
        }
    }

    fn reset_auto_play_memory(&self) {
        let current_hash = self.current_game_hash();
        let mut seen = self.imp().auto_play_seen_states.borrow_mut();
        seen.clear();
        seen.insert(current_hash);
    }

    fn note_current_state_for_auto_play(&self) {
        let current_hash = self.current_game_hash();
        self.imp()
            .auto_play_seen_states
            .borrow_mut()
            .insert(current_hash);
    }

    fn unseen_followup_count(&self, game: &KlondikeGame, seen_states: &HashSet<u64>) -> i64 {
        let mut followups: HashSet<u64> = HashSet::new();
        for candidate in self.enumerate_hint_candidates(game) {
            let Some(hint_move) = candidate.hint_move else {
                continue;
            };
            let mut next_state = game.clone();
            if !apply_hint_move_to_game(&mut next_state, hint_move) {
                continue;
            }
            let next_hash = hash_game_state(&next_state);
            if !seen_states.contains(&next_hash) {
                followups.insert(next_hash);
            }
        }
        followups.len() as i64
    }

    fn auto_play_lookahead_score(
        &self,
        current: &KlondikeGame,
        persistent_seen: &HashSet<u64>,
        path_seen: &mut HashSet<u64>,
        depth: u8,
        budget: &mut usize,
    ) -> i64 {
        if current.is_won() {
            return AUTO_PLAY_WIN_SCORE;
        }
        if depth == 0 || *budget == 0 {
            return auto_play_state_heuristic(current);
        }

        let mut scored_children: Vec<(i64, KlondikeGame, u64)> = Vec::new();
        for candidate in self.enumerate_hint_candidates(current) {
            let Some(hint_move) = candidate.hint_move else {
                continue;
            };
            let mut next = current.clone();
            if !apply_hint_move_to_game(&mut next, hint_move) {
                continue;
            }
            let next_hash = hash_game_state(&next);
            if persistent_seen.contains(&next_hash) || path_seen.contains(&next_hash) {
                continue;
            }
            if self.is_obviously_useless_auto_move(current, &next, hint_move) {
                continue;
            }

            let immediate =
                score_hint_candidate(current, &next, hint_move, persistent_seen, next_hash);
            scored_children.push((immediate, next, next_hash));
        }

        if scored_children.is_empty() {
            return -90_000 + auto_play_state_heuristic(current);
        }

        scored_children.sort_by_key(|(score, _, _)| Reverse(*score));
        let mut best = i64::MIN / 4;
        for (immediate, next, next_hash) in scored_children.into_iter().take(AUTO_PLAY_BEAM_WIDTH) {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            path_seen.insert(next_hash);
            let future = self.auto_play_lookahead_score(
                &next,
                persistent_seen,
                path_seen,
                depth - 1,
                budget,
            );
            path_seen.remove(&next_hash);

            let total = immediate + (future / 2);
            if total > best {
                best = total;
            }
        }

        if best == i64::MIN / 4 {
            auto_play_state_heuristic(current)
        } else {
            best
        }
    }

    fn is_obviously_useless_auto_move(
        &self,
        current: &KlondikeGame,
        next: &KlondikeGame,
        hint_move: HintMove,
    ) -> bool {
        let foundation_delta = foundation_count(next) - foundation_count(current);
        let hidden_delta = hidden_tableau_count(current) - hidden_tableau_count(next);
        let empty_delta = empty_tableau_count(next) - empty_tableau_count(current);
        let mobility_delta = non_draw_move_count(next) - non_draw_move_count(current);
        if foundation_delta > 0 || hidden_delta > 0 || empty_delta > 0 {
            return false;
        }

        match hint_move {
            HintMove::WasteToFoundation | HintMove::TableauTopToFoundation { .. } => false,
            HintMove::WasteToTableau { .. } => current.can_move_waste_to_foundation(),
            HintMove::TableauRunToTableau { src, start, dst } => {
                let run_len = current.tableau_len(src).unwrap_or(0).saturating_sub(start);
                if run_len == 0 {
                    return true;
                }
                if self.is_king_to_empty_without_unlock(current, hint_move) {
                    return true;
                }
                let reversible =
                    restores_current_by_inverse_tableau_move(current, next, src, dst, run_len);
                reversible && mobility_delta <= 0
            }
            HintMove::Draw => {
                // Deal-2/3/4/5 often requires cycling draws even when some non-draw moves exist.
                // Treating draw as "obviously useless" there makes wand/hint stall incorrectly.
                if current.draw_mode().count() > 1 {
                    false
                } else {
                    non_draw_move_count(current) > 0
                }
            }
        }
    }

    fn is_king_to_empty_without_unlock(&self, current: &KlondikeGame, hint_move: HintMove) -> bool {
        let HintMove::TableauRunToTableau { src, start, dst } = hint_move else {
            return false;
        };
        if current.tableau_len(dst) != Some(0) {
            return false;
        }
        let Some(card) = current.tableau_card(src, start) else {
            return false;
        };
        if card.rank != 13 {
            return false;
        }
        let reveals_hidden = start > 0
            && current
                .tableau_card(src, start - 1)
                .map(|below| !below.face_up)
                .unwrap_or(false);
        !reveals_hidden
    }

    fn is_functionally_useless_auto_move(
        &self,
        current: &KlondikeGame,
        next: &KlondikeGame,
        hint_move: HintMove,
        seen_states: &HashSet<u64>,
    ) -> bool {
        if next.is_won() {
            return false;
        }

        let foundation_delta = foundation_count(next) - foundation_count(current);
        let hidden_delta = hidden_tableau_count(current) - hidden_tableau_count(next);
        let empty_delta = empty_tableau_count(next) - empty_tableau_count(current);
        if foundation_delta > 0 || hidden_delta > 0 || empty_delta > 0 {
            return false;
        }

        let mobility_delta = non_draw_move_count(next) - non_draw_move_count(current);
        let unseen_followups = self.unseen_followup_count(next, seen_states);

        match hint_move {
            HintMove::WasteToFoundation | HintMove::TableauTopToFoundation { .. } => false,
            HintMove::WasteToTableau { .. } => {
                if current.can_move_waste_to_foundation() {
                    return true;
                }
                mobility_delta <= 0 && unseen_followups <= 0
            }
            HintMove::TableauRunToTableau { src, start, dst } => {
                let run_len = current.tableau_len(src).unwrap_or(0).saturating_sub(start);
                if run_len == 0 {
                    return true;
                }

                if self.is_king_to_empty_without_unlock(current, hint_move) && mobility_delta <= 0 {
                    return true;
                }

                let reversible =
                    restores_current_by_inverse_tableau_move(current, next, src, dst, run_len);

                if reversible && mobility_delta <= 0 {
                    return true;
                }

                run_len == 1 && mobility_delta <= 0 && unseen_followups <= 1
            }
            HintMove::Draw => {
                let drew_playable = next.can_move_waste_to_foundation()
                    || (0..7).any(|dst| next.can_move_waste_to_tableau(dst));
                if drew_playable {
                    return false;
                }
                if current.draw_mode().count() > 1 {
                    // In multi-draw modes, advancing waste/stock position itself can be useful.
                    return unseen_followups <= 0
                        && hash_game_state(current) == hash_game_state(next);
                }
                let current_non_draw_moves = non_draw_move_count(current);
                current_non_draw_moves > 0 && unseen_followups <= 0
            }
        }
    }

    fn current_game_hash(&self) -> u64 {
        let game = self.imp().game.borrow();
        hash_game_state(&game)
    }

    fn cached_loss_verdict_for_hash(&self, state_hash: u64) -> Option<LossVerdict> {
        self.imp()
            .hint_loss_cache
            .borrow()
            .get(&state_hash)
            .copied()
    }

    fn start_hint_loss_analysis_if_needed(&self, state_hash: u64) {
        if self.cached_loss_verdict_for_hash(state_hash).is_some() {
            return;
        }

        let imp = self.imp();
        if imp.hint_loss_analysis_running.get() {
            return;
        }
        imp.hint_loss_analysis_running.set(true);
        imp.hint_loss_analysis_hash.set(state_hash);

        let game = imp.game.borrow().clone();
        let (sender, receiver) = mpsc::channel::<LossVerdict>();

        thread::spawn(move || {
            let verdict = if game.is_winnable_guided(HINT_GUIDED_ANALYSIS_BUDGET) {
                LossVerdict::WinnableLikely
            } else {
                let result = game.analyze_winnability(HINT_EXHAUSTIVE_ANALYSIS_BUDGET);
                if result.winnable {
                    LossVerdict::WinnableLikely
                } else if result.hit_state_limit {
                    LossVerdict::Inconclusive {
                        explored_states: result.explored_states,
                    }
                } else {
                    LossVerdict::Lost {
                        explored_states: result.explored_states,
                    }
                }
            };
            let _ = sender.send(verdict);
        });

        glib::timeout_add_local(
            Duration::from_millis(40),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || match receiver.try_recv() {
                    Ok(verdict) => {
                        let imp = window.imp();
                        let analyzed_hash = imp.hint_loss_analysis_hash.get();
                        imp.hint_loss_analysis_running.set(false);
                        imp.hint_loss_cache
                            .borrow_mut()
                            .insert(analyzed_hash, verdict);

                        let current_hash = window.current_game_hash();
                        if current_hash == analyzed_hash {
                            if let LossVerdict::Lost { explored_states } = verdict {
                                *imp.status_override.borrow_mut() = Some(format!(
                                    "No winning path found from this position (explored {explored_states} states). Game is lost."
                                ));
                                window.render();
                            }
                        }
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        window.imp().hint_loss_analysis_running.set(false);
                        glib::ControlFlow::Break
                    }
                }
            ),
        );
    }

    fn update_stats_label(&self) {
        let imp = self.imp();
        let elapsed = imp.elapsed_seconds.get();
        let apm = self.current_apm();
        imp.stats_label.set_label(&format!(
            "Moves: {}   APM: {:.1}   Time: {}",
            imp.move_count.get(),
            apm,
            format_time(elapsed)
        ));
    }

    fn update_tableau_selection_styles(&self, selected: Option<SelectedRun>) {
        let stacks = self.tableau_stacks();
        let card_pictures = self.imp().tableau_card_pictures.borrow();

        for (index, stack) in stacks.into_iter().enumerate() {
            stack.remove_css_class("tableau-selected-empty");
            for picture in &card_pictures[index] {
                picture.remove_css_class("tableau-selected-card");
            }

            if let Some(run) = selected {
                if run.col != index {
                    continue;
                }
                if card_pictures[index].is_empty() {
                    stack.add_css_class("tableau-selected-empty");
                    continue;
                }
                let start = run.start.min(card_pictures[index].len().saturating_sub(1));
                for picture in card_pictures[index].iter().skip(start) {
                    picture.add_css_class("tableau-selected-card");
                }
            }
        }
    }

    fn update_waste_selection_style(&self, selected: bool) {
        for waste in self.waste_fan_slots() {
            if selected {
                waste.add_css_class("waste-selected-card");
            } else {
                waste.remove_css_class("waste-selected-card");
            }
        }
    }

    fn tableau_run_start_from_y(&self, game: &KlondikeGame, col: usize, y: f64) -> Option<usize> {
        let len = game.tableau_len(col)?;
        if len == 0 {
            return None;
        }

        let mut y_pos = 0.0_f64;
        let mut positions = Vec::with_capacity(len);
        let face_up_step = f64::from(self.imp().face_up_step.get());
        let face_down_step = f64::from(self.imp().face_down_step.get());
        for idx in 0..len {
            positions.push((idx, y_pos));
            let card = game.tableau_card(col, idx)?;
            y_pos += if card.face_up {
                face_up_step
            } else {
                face_down_step
            };
        }

        let mut start = positions.last().map(|(idx, _)| *idx)?;
        for (idx, pos) in positions {
            if y >= pos {
                start = idx;
            } else {
                break;
            }
        }

        if game.tableau_card(col, start)?.face_up {
            Some(start)
        } else {
            None
        }
    }

    fn can_auto_move_waste_to_foundation(&self, game: &KlondikeGame) -> bool {
        let Some(card) = game.waste_top() else {
            return false;
        };
        game.can_move_waste_to_foundation() && self.is_safe_auto_foundation(game, card)
    }

    fn can_auto_move_tableau_to_foundation(&self, game: &KlondikeGame, src: usize) -> bool {
        let Some(card) = game.tableau_top(src) else {
            return false;
        };
        game.can_move_tableau_top_to_foundation(src) && self.is_safe_auto_foundation(game, card)
    }

    fn try_auto_move_waste_to_foundation(&self) -> bool {
        let game = self.imp().game.borrow();
        let legal = self.can_auto_move_waste_to_foundation(&game);
        drop(game);
        legal && self.move_waste_to_foundation()
    }

    fn is_safe_auto_foundation(&self, game: &KlondikeGame, card: Card) -> bool {
        if card.rank <= 2 {
            return true;
        }

        match card.suit {
            Suit::Hearts | Suit::Diamonds => {
                game.foundation_top_rank(Suit::Clubs) >= card.rank - 1
                    && game.foundation_top_rank(Suit::Spades) >= card.rank - 1
            }
            Suit::Clubs | Suit::Spades => {
                game.foundation_top_rank(Suit::Hearts) >= card.rank - 1
                    && game.foundation_top_rank(Suit::Diamonds) >= card.rank - 1
            }
        }
    }

    fn render_card_images(&self, game: &KlondikeGame) {
        let imp = self.imp();

        if !imp.deck_load_attempted.get() {
            imp.deck_load_attempted.set(true);
            match AngloDeck::load() {
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

        imp.stock_picture.set_width_request(card_width);
        imp.stock_picture.set_height_request(card_height);
        imp.stock_picture.set_can_shrink(false);
        imp.waste_picture.set_width_request(card_width);
        imp.waste_picture.set_height_request(card_height);
        imp.waste_picture.set_can_shrink(false);
        imp.waste_picture.set_halign(gtk::Align::Start);
        imp.waste_picture.set_valign(gtk::Align::Start);
        imp.waste_picture.set_paintable(None::<&gdk::Paintable>);
        imp.waste_placeholder_box.set_width_request(card_width);
        imp.waste_placeholder_box.set_height_request(card_height);
        for picture in self.waste_fan_slots() {
            picture.set_width_request(card_width);
            picture.set_height_request(card_height);
            picture.set_can_shrink(false);
        }
        let waste_fan_step = (card_width / 6).clamp(8, 22);
        let foundation_group_width = (card_width * 4) + (8 * 3);
        imp.stock_heading_box.set_width_request(card_width);
        imp.waste_heading_box
            .set_width_request(card_width + (waste_fan_step * 4));
        imp.foundations_heading_box
            .set_width_request(foundation_group_width);
        imp.waste_overlay
            .set_width_request(card_width + (waste_fan_step * 4));
        imp.waste_overlay.set_height_request(card_height);
        for picture in self.foundation_pictures() {
            picture.set_width_request(card_width);
            picture.set_height_request(card_height);
            picture.set_can_shrink(false);
        }

        if game.stock_len() > 0 {
            let back = deck.back_texture_scaled(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&back));
        } else {
            let empty = Self::blank_texture(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&empty));
        }

        let waste_widgets = self.waste_fan_slots();
        let visible_waste_cards = usize::from(game.draw_mode().count().clamp(1, 5));
        let waste_cards = game.waste_top_n(visible_waste_cards);
        let show_count = waste_cards.len();

        for picture in waste_widgets.iter() {
            picture.set_visible(false);
            picture.set_margin_start(0);
            picture.set_paintable(None::<&gdk::Paintable>);
        }

        for (idx, card) in waste_cards.iter().copied().enumerate() {
            if let Some(picture) = waste_widgets.get(idx) {
                let texture = deck.texture_for_card_scaled(card, card_width, card_height);
                picture.set_paintable(Some(&texture));
                if idx > 0 {
                    picture.set_margin_start((idx as i32) * waste_fan_step);
                }
                picture.set_visible(true);
            }
        }
        imp.waste_placeholder_label.set_visible(show_count == 0);

        for (idx, picture) in self.foundation_pictures().into_iter().enumerate() {
            let top = game.foundations()[idx].last().copied();
            self.set_picture_from_card(&picture, top, deck, card_width, card_height);
        }
        imp.foundation_placeholder_1
            .set_visible(game.foundations()[0].is_empty());
        imp.foundation_placeholder_2
            .set_visible(game.foundations()[1].is_empty());
        imp.foundation_placeholder_3
            .set_visible(game.foundations()[2].is_empty());
        imp.foundation_placeholder_4
            .set_visible(game.foundations()[3].is_empty());

        let mut tableau_card_pictures = vec![Vec::new(); 7];

        for (idx, stack) in self.tableau_stacks().into_iter().enumerate() {
            while let Some(child) = stack.first_child() {
                stack.remove(&child);
            }

            stack.set_width_request(card_width);

            let column = &game.tableau()[idx];
            let mut y = 0;
            for (card_idx, card) in column.iter().enumerate() {
                let picture = gtk::Picture::new();
                picture.set_width_request(card_width);
                picture.set_height_request(card_height);
                picture.set_can_shrink(true);
                picture.set_content_fit(gtk::ContentFit::Contain);

                let show_face_up = if peek_active {
                    !card.face_up
                } else {
                    card.face_up
                };
                let texture = if show_face_up {
                    deck.texture_for_card(*card)
                } else {
                    deck.back_texture()
                };
                picture.set_paintable(Some(&texture));
                tableau_card_pictures[idx].push(picture.clone());

                stack.put(&picture, 0.0, f64::from(y));
                if card_idx + 1 < column.len() {
                    y += if card.face_up {
                        face_up_step
                    } else {
                        face_down_step
                    };
                }
            }

            // Keep stack widgets tight to their content so tableau columns stay clustered.
            let stack_height = (y + card_height).max(card_height);
            stack.set_height_request(stack_height);
        }

        *imp.tableau_card_pictures.borrow_mut() = tableau_card_pictures;
    }

    fn update_tableau_metrics(&self) {
        const COLUMNS: i32 = 7;
        let profile = self.workspace_layout_profile();

        let imp = self.imp();
        imp.tableau_row.set_spacing(profile.gap);
        let window_width = self.width().max(MIN_WINDOW_WIDTH);
        let window_height = self.height().max(MIN_WINDOW_HEIGHT);
        let column_gap = imp.tableau_row.spacing().max(0);
        let mut metrics_hasher = DefaultHasher::new();
        window_width.hash(&mut metrics_hasher);
        window_height.hash(&mut metrics_hasher);
        self.is_maximized().hash(&mut metrics_hasher);
        profile.side_padding.hash(&mut metrics_hasher);
        profile.tableau_vertical_padding.hash(&mut metrics_hasher);
        column_gap.hash(&mut metrics_hasher);
        profile.assumed_depth.hash(&mut metrics_hasher);
        profile.min_card_width.hash(&mut metrics_hasher);
        profile.max_card_width.hash(&mut metrics_hasher);
        profile.min_card_height.hash(&mut metrics_hasher);
        TABLEAU_FACE_UP_STEP_PX.hash(&mut metrics_hasher);
        TABLEAU_FACE_DOWN_STEP_PX.hash(&mut metrics_hasher);
        let metrics_key = metrics_hasher.finish();
        if metrics_key == imp.last_metrics_key.get() {
            return;
        }
        imp.last_metrics_key.set(metrics_key);

        let scroller_width = imp.tableau_scroller.width();
        let available_width = if scroller_width > 0 {
            (scroller_width - profile.side_padding).max(0)
        } else {
            (window_width - profile.side_padding * 2).max(0)
        };
        let slots = (available_width - column_gap * (COLUMNS - 1)).max(0);
        let width_limited_by_columns = if slots > 0 { slots / COLUMNS } else { 70 };
        let width_limited_by_top_row = self.max_card_width_for_top_row_fit(available_width);
        // Cap card size by window-height budget so tableau depth stays playable.
        let reserve = self.vertical_layout_reserve(window_height);
        let usable_window_height = (window_height - reserve).max(220);
        let tableau_overhead = profile.tableau_vertical_padding + 12;
        let width_limited_by_window_height = self.max_card_width_for_window_height_fit(
            usable_window_height,
            profile,
            tableau_overhead,
        );

        // Keep width stable by geometry, but never exceed vertical-fit cap.
        let card_width = width_limited_by_columns
            .min(width_limited_by_top_row)
            .min(width_limited_by_window_height)
            .clamp(profile.min_card_width, profile.max_card_width);
        let card_height = (card_width * 108 / 70).max(profile.min_card_height);

        imp.card_width.set(card_width);
        imp.card_height.set(card_height);
        imp.face_up_step.set(TABLEAU_FACE_UP_STEP_PX);
        imp.face_down_step.set(TABLEAU_FACE_DOWN_STEP_PX);
    }

    fn workspace_layout_profile(&self) -> WorkspaceLayoutProfile {
        let window_width = self.width().max(1);
        let window_height = self.height().max(1);
        let preset = if window_height <= 600 {
            WorkspacePreset::Compact600
        } else if window_height <= 720 {
            WorkspacePreset::Hd720
        } else if window_height <= 1080 {
            WorkspacePreset::Fhd1080
        } else {
            WorkspacePreset::Qhd1440
        };

        let mut profile = match preset {
            WorkspacePreset::Compact600 => WorkspaceLayoutProfile {
                side_padding: 8,
                tableau_vertical_padding: 6,
                gap: 2,
                assumed_depth: 11,
                min_card_width: 14,
                max_card_width: 96,
                min_card_height: 24,
            },
            WorkspacePreset::Hd720 => WorkspaceLayoutProfile {
                side_padding: 12,
                tableau_vertical_padding: 8,
                gap: 3,
                assumed_depth: 12,
                min_card_width: 16,
                max_card_width: 118,
                min_card_height: 28,
            },
            WorkspacePreset::Fhd1080 => WorkspaceLayoutProfile {
                side_padding: 18,
                tableau_vertical_padding: 10,
                gap: 4,
                assumed_depth: 14,
                min_card_width: 18,
                max_card_width: 164,
                min_card_height: 32,
            },
            WorkspacePreset::Qhd1440 => WorkspaceLayoutProfile {
                side_padding: 24,
                tableau_vertical_padding: 14,
                gap: 6,
                assumed_depth: 14,
                min_card_width: 20,
                max_card_width: 240,
                min_card_height: 36,
            },
        };

        if self.is_maximized() {
            if window_height > 1080 {
                profile.max_card_width = profile.max_card_width.saturating_add(18);
            } else {
                profile.max_card_width = profile.max_card_width.saturating_add(6);
            }
        }

        // Very wide 1080p layouts need extra tableau depth budget; otherwise cards get too tall
        // and stacks become hard to read/interact with.
        if window_height <= 1080 && window_width >= 1600 {
            profile.max_card_width = profile.max_card_width.saturating_sub(16);
            profile.assumed_depth = profile.assumed_depth.saturating_add(1);
        }

        if window_width < window_height {
            profile.side_padding = (profile.side_padding / 2).max(6);
            profile.gap = (profile.gap - 1).max(2);
            profile.max_card_width = profile.max_card_width.saturating_sub(6);
        }

        profile
    }

    fn vertical_layout_reserve(&self, window_height: i32) -> i32 {
        if window_height <= 600 {
            196
        } else if window_height <= 720 {
            208
        } else if window_height <= 1080 {
            228
        } else {
            258
        }
    }

    fn max_card_width_for_window_height_fit(
        &self,
        usable_window_height: i32,
        profile: WorkspaceLayoutProfile,
        tableau_overhead: i32,
    ) -> i32 {
        let mut best = profile.min_card_width;
        let mut lo = profile.min_card_width;
        let mut hi = profile.max_card_width;

        while lo <= hi {
            let mid = (lo + hi) / 2;
            let card_height = (mid * 108 / 70).max(profile.min_card_height);
            let available_tableau_height =
                usable_window_height.saturating_sub(card_height + tableau_overhead);
            let tallest = self.tallest_tableau_height_with_steps(
                profile.assumed_depth,
                card_height,
                TABLEAU_FACE_UP_STEP_PX,
            );

            if available_tableau_height > 0 && tallest <= available_tableau_height {
                best = mid;
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }

        best
    }

    fn max_card_width_for_top_row_fit(&self, available_width: i32) -> i32 {
        // Reserve some extra pixels for frame paddings/borders to avoid borderline overflow.
        let usable = available_width.saturating_sub(24).max(120);
        let mut best = 18;
        let mut lo = 18;
        let mut hi = 320;

        while lo <= hi {
            let mid = (lo + hi) / 2;
            let waste_step = (mid / 6).clamp(8, 22);
            let top_row_width = (6 * mid) + (4 * waste_step) + 56; // stock + waste + foundations + fixed gaps
            if top_row_width <= usable {
                best = mid;
                lo = mid + 1;
            } else {
                hi = mid - 1;
            }
        }

        best
    }

    fn tallest_tableau_height_with_steps(
        &self,
        assumed_depth: i32,
        card_height: i32,
        face_up_step: i32,
    ) -> i32 {
        let depth = assumed_depth.max(1);
        card_height + (depth - 1) * face_up_step.max(1)
    }

    fn set_picture_from_card(
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

    fn blank_texture(width: i32, height: i32) -> gdk::Texture {
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

    fn foundation_pictures(&self) -> [gtk::Picture; 4] {
        let imp = self.imp();
        [
            imp.foundation_picture_1.get(),
            imp.foundation_picture_2.get(),
            imp.foundation_picture_3.get(),
            imp.foundation_picture_4.get(),
        ]
    }

    fn foundation_placeholders(&self) -> [gtk::Label; 4] {
        let imp = self.imp();
        [
            imp.foundation_placeholder_1.get(),
            imp.foundation_placeholder_2.get(),
            imp.foundation_placeholder_3.get(),
            imp.foundation_placeholder_4.get(),
        ]
    }

    fn waste_fan_slots(&self) -> [gtk::Picture; 5] {
        let imp = self.imp();
        [
            imp.waste_picture_1.get(),
            imp.waste_picture_2.get(),
            imp.waste_picture_3.get(),
            imp.waste_picture_4.get(),
            imp.waste_picture_5.get(),
        ]
    }

    fn tableau_stacks(&self) -> [gtk::Fixed; 7] {
        let imp = self.imp();
        [
            imp.tableau_stack_1.get(),
            imp.tableau_stack_2.get(),
            imp.tableau_stack_3.get(),
            imp.tableau_stack_4.get(),
            imp.tableau_stack_5.get(),
            imp.tableau_stack_6.get(),
            imp.tableau_stack_7.get(),
        ]
    }
}

fn sanitize_selected_run(
    game: &KlondikeGame,
    selected: Option<SelectedRun>,
) -> Option<SelectedRun> {
    let run = selected?;
    let len = game.tableau_len(run.col)?;
    if run.start >= len {
        return None;
    }
    let card = game.tableau_card(run.col, run.start)?;
    if card.face_up {
        Some(run)
    } else {
        None
    }
}

fn apply_hint_move_to_game(game: &mut KlondikeGame, hint_move: HintMove) -> bool {
    match hint_move {
        HintMove::WasteToFoundation => game.move_waste_to_foundation(),
        HintMove::TableauTopToFoundation { src } => game.move_tableau_top_to_foundation(src),
        HintMove::WasteToTableau { dst } => game.move_waste_to_tableau(dst),
        HintMove::TableauRunToTableau { src, start, dst } => {
            game.move_tableau_run_to_tableau(src, start, dst)
        }
        HintMove::Draw => game.draw_or_recycle() != DrawResult::NoOp,
    }
}

fn hash_game_state(game: &KlondikeGame) -> u64 {
    let mut hasher = DefaultHasher::new();
    game.hash(&mut hasher);
    hasher.finish()
}

fn foundation_count(game: &KlondikeGame) -> i64 {
    game.foundations()
        .iter()
        .map(|pile| pile.len() as i64)
        .sum()
}

fn hidden_tableau_count(game: &KlondikeGame) -> i64 {
    game.tableau()
        .iter()
        .flat_map(|pile| pile.iter())
        .filter(|card| !card.face_up)
        .count() as i64
}

fn face_up_tableau_count(game: &KlondikeGame) -> i64 {
    game.tableau()
        .iter()
        .flat_map(|pile| pile.iter())
        .filter(|card| card.face_up)
        .count() as i64
}

fn empty_tableau_count(game: &KlondikeGame) -> i64 {
    game.tableau().iter().filter(|pile| pile.is_empty()).count() as i64
}

fn score_hint_candidate(
    current: &KlondikeGame,
    next: &KlondikeGame,
    hint_move: HintMove,
    recent_hashes: &HashSet<u64>,
    next_hash: u64,
) -> i64 {
    let foundation_delta = foundation_count(next) - foundation_count(current);
    let hidden_delta = hidden_tableau_count(current) - hidden_tableau_count(next);
    let face_up_delta = face_up_tableau_count(next) - face_up_tableau_count(current);
    let empty_delta = empty_tableau_count(next) - empty_tableau_count(current);

    let mut score =
        foundation_delta * 1400 + hidden_delta * 260 + face_up_delta * 32 + empty_delta * 70;

    match hint_move {
        HintMove::WasteToFoundation | HintMove::TableauTopToFoundation { .. } => {
            score += 420;
        }
        HintMove::WasteToTableau { dst } => {
            score += 60;
            if current.tableau_len(dst) == Some(0) {
                score += 150;
            }
        }
        HintMove::TableauRunToTableau { src, start, dst } => {
            let run_len = current.tableau_len(src).unwrap_or(0).saturating_sub(start) as i64;
            if run_len > 1 {
                score += run_len * 12;
            }
            if current.tableau_len(dst) == Some(0) {
                score += 180;
            }
            if start > 0
                && current
                    .tableau_card(src, start - 1)
                    .map(|card| !card.face_up)
                    .unwrap_or(false)
            {
                score += 260;
            }
            if run_len == 1 && hidden_delta <= 0 && foundation_delta <= 0 {
                score -= 160;
            }
        }
        HintMove::Draw => {
            score -= 140;
        }
    }

    if recent_hashes.contains(&next_hash) {
        score -= 2400;
    }

    score
}

fn auto_play_state_heuristic(game: &KlondikeGame) -> i64 {
    if game.is_won() {
        return AUTO_PLAY_WIN_SCORE;
    }

    let foundation = foundation_count(game);
    let hidden = hidden_tableau_count(game);
    let face_up = face_up_tableau_count(game);
    let empty = empty_tableau_count(game);
    let mobility = non_draw_move_count(game);
    let stock = game.stock_len() as i64;
    let waste_has_card = game.waste_top().is_some() as i64;

    foundation * 1900 + face_up * 22 + empty * 90 + mobility * 12 - hidden * 190 - stock * 6
        + waste_has_card * 8
}

fn non_draw_move_count(game: &KlondikeGame) -> i64 {
    let mut count = 0_i64;

    if game.can_move_waste_to_foundation() {
        count += 1;
    }

    for src in 0..7 {
        if game.can_move_tableau_top_to_foundation(src) {
            count += 1;
        }
    }

    for dst in 0..7 {
        if game.can_move_waste_to_tableau(dst) {
            count += 1;
        }
    }

    for src in 0..7 {
        let len = game.tableau_len(src).unwrap_or(0);
        for start in 0..len {
            for dst in 0..7 {
                if game.can_move_tableau_run_to_tableau(src, start, dst) {
                    count += 1;
                }
            }
        }
    }

    count
}

fn restores_current_by_inverse_tableau_move(
    current: &KlondikeGame,
    next: &KlondikeGame,
    src: usize,
    dst: usize,
    run_len: usize,
) -> bool {
    if run_len == 0 {
        return false;
    }
    let dst_len = next.tableau_len(dst).unwrap_or(0);
    if dst_len < run_len {
        return false;
    }

    let inverse_start = dst_len - run_len;
    if !next.can_move_tableau_run_to_tableau(dst, inverse_start, src) {
        return false;
    }

    let mut reverse = next.clone();
    if !reverse.move_tableau_run_to_tableau(dst, inverse_start, src) {
        return false;
    }

    &reverse == current
}

fn format_time(seconds: u32) -> String {
    let minutes = seconds / 60;
    let remainder = seconds % 60;
    format!("{minutes:02}:{remainder:02}")
}

fn parse_seed_input(input: &str) -> Result<Option<u64>, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }

    let normalized = trimmed.replace('_', "");
    normalized.parse::<u64>().map(Some).map_err(|_| {
        "Seed must be an unsigned whole number (0 to 18446744073709551615).".to_string()
    })
}

fn random_seed() -> u64 {
    let mut rng = rand::thread_rng();
    rng.gen()
}

fn default_dialog_find_winnable_attempts() -> u32 {
    thread::available_parallelism()
        .map(|n| (n.get() * 6).clamp(16, 128) as u32)
        .unwrap_or(48)
}

fn is_seed_winnable_for_dialog(
    seed: u64,
    draw_mode: DrawMode,
    cancel: &AtomicBool,
) -> Option<SeedWinnabilityCheckResult> {
    let mut game = KlondikeGame::new_with_seed(seed);
    game.set_draw_mode(draw_mode);
    let guided = game.guided_winnability_cancelable(DIALOG_SEED_GUIDED_BUDGET, cancel)?;
    if guided.winnable {
        return Some(SeedWinnabilityCheckResult {
            winnable: true,
            iterations: guided.explored_states,
            moves_to_win: guided.win_depth,
            hit_state_limit: guided.hit_state_limit,
        });
    }

    let exhaustive = game.analyze_winnability_cancelable(DIALOG_SEED_EXHAUSTIVE_BUDGET, cancel)?;
    Some(SeedWinnabilityCheckResult {
        winnable: exhaustive.winnable,
        iterations: guided
            .explored_states
            .saturating_add(exhaustive.explored_states),
        moves_to_win: exhaustive.win_depth,
        hit_state_limit: exhaustive.hit_state_limit,
    })
}

fn find_winnable_seed_parallel(
    start_seed: u64,
    attempts: u32,
    max_states: usize,
    draw_mode: DrawMode,
) -> Option<(u64, u32)> {
    if attempts == 0 {
        return None;
    }

    let worker_count = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
        .min(attempts as usize)
        .max(1);

    let next_index = Arc::new(AtomicU32::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel::<(u64, u32)>();

    for _ in 0..worker_count {
        let next_index = Arc::clone(&next_index);
        let stop = Arc::clone(&stop);
        let sender = sender.clone();
        thread::spawn(move || loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let index = next_index.fetch_add(1, Ordering::Relaxed);
            if index >= attempts {
                break;
            }

            let seed = start_seed.wrapping_add(u64::from(index));
            let mut game = KlondikeGame::new_with_seed(seed);
            game.set_draw_mode(draw_mode);
            if game.is_winnable_guided(max_states) {
                if !stop.swap(true, Ordering::Relaxed) {
                    let _ = sender.send((seed, index + 1));
                }
                break;
            }
        });
    }

    drop(sender);
    receiver.recv().ok()
}

fn parse_tableau_payload(payload: &str) -> Option<(usize, usize)> {
    let rest = payload.strip_prefix("tableau:")?;
    let (src, start) = rest.split_once(':')?;
    let src = src.parse::<usize>().ok()?;
    let start = start.parse::<usize>().ok()?;
    Some((src, start))
}

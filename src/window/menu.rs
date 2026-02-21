use super::*;
use crate::engine::variant::variant_for_mode;
use crate::engine::variant_engine::engine_for_mode;

impl CardthropicWindow {
    fn refresh_main_menu_model(&self) {
        let model = self.build_main_menu_model();
        self.imp().main_menu_popover.set_menu_model(Some(&model));
    }

    fn build_main_menu_model(&self) -> gio::Menu {
        let root = gio::Menu::new();
        let section = gio::Menu::new();

        let klondike = gio::Menu::new();
        for (draw_mode, action) in [
            (DrawMode::One, "win.mode-klondike-deal-1"),
            (DrawMode::Two, "win.mode-klondike-deal-2"),
            (DrawMode::Three, "win.mode-klondike-deal-3"),
            (DrawMode::Four, "win.mode-klondike-deal-4"),
            (DrawMode::Five, "win.mode-klondike-deal-5"),
        ] {
            klondike.append(Some(&format!("Deal {}", draw_mode.count())), Some(action));
        }
        section.append_submenu(Some("Klondike"), &klondike);

        let spider = gio::Menu::new();
        for (suit_mode, action) in [
            (SpiderSuitMode::One, "win.mode-spider-suit-1"),
            (SpiderSuitMode::Two, "win.mode-spider-suit-2"),
            (SpiderSuitMode::Three, "win.mode-spider-suit-3"),
            (SpiderSuitMode::Four, "win.mode-spider-suit-4"),
        ] {
            spider.append(
                Some(&format!("Suit {}", suit_mode.suit_count())),
                Some(action),
            );
        }
        section.append_submenu(Some("Spider"), &spider);

        let freecell = gio::Menu::new();
        for (card_count_mode, action) in [
            (
                FreecellCardCountMode::TwentySix,
                "win.mode-freecell-card-26",
            ),
            (
                FreecellCardCountMode::ThirtyNine,
                "win.mode-freecell-card-39",
            ),
            (FreecellCardCountMode::FiftyTwo, "win.mode-freecell-card-52"),
        ] {
            freecell.append(
                Some(&format!("Card Count {}", card_count_mode.card_count())),
                Some(action),
            );
        }
        freecell.append(
            Some("Number of Free Cells…"),
            Some("win.freecell-cell-count-dialog"),
        );
        section.append_submenu(Some("FreeCell"), &freecell);

        let chess = gio::Menu::new();
        chess.append(
            Some("Standard Chess (Preview)"),
            Some("win.mode-chess-standard"),
        );
        chess.append(Some("Chess960 (Preview)"), Some("win.mode-chess-960"));
        chess.append(
            Some("Atomic Chess (Preview)"),
            Some("win.mode-chess-atomic"),
        );
        chess.append(Some("Flip Board"), Some("win.chess-flip-board"));
        chess.append(
            Some("Auto-flip Board Each Move"),
            Some("win.chess-auto-flip-board-each-move"),
        );
        chess.append(
            Some("Show Board Coordinates"),
            Some("win.chess-show-board-coordinates"),
        );
        chess.append(
            Some("System Move Sounds"),
            Some("win.chess-system-sounds-enabled"),
        );
        chess.append(Some("Rotate Board…"), Some("win.chess-rotate-board-dialog"));
        chess.append(
            Some("Auto-Response AI Strength…"),
            Some("win.chess-ai-strength-dialog"),
        );
        chess.append(
            Some("W? AI Strength…"),
            Some("win.chess-w-question-ai-strength-dialog"),
        );
        chess.append(
            Some("Your Wand AI Strength…"),
            Some("win.chess-wand-ai-strength-dialog"),
        );
        chess.append(
            Some("Robot White AI Strength…"),
            Some("win.chess-robot-white-ai-strength-dialog"),
        );
        chess.append(
            Some("Robot Black AI Strength…"),
            Some("win.chess-robot-black-ai-strength-dialog"),
        );
        chess.append(
            Some("Wand AI Opponent Auto Response"),
            Some("win.chess-wand-ai-opponent-auto-response"),
        );
        chess.append(
            Some("Auto-Response Plays White"),
            Some("win.chess-auto-response-plays-white"),
        );
        section.append_submenu(Some("Chessthropic"), &chess);

        let deal = gio::Menu::new();
        deal.append(Some("Start Random Game"), Some("win.random-seed"));
        deal.append(Some("Find Winnable Game"), Some("win.winnable-seed"));
        deal.append(Some("Seed Picker"), Some("win.seed-picker"));
        deal.append(Some("Repeat Current Seed"), Some("win.repeat-seed"));
        deal.append(
            Some("Check Seed Winnability"),
            Some("win.check-seed-winnable"),
        );
        deal.append(Some("Copy Game State"), Some("win.copy-game-state"));
        deal.append(Some("Load Game State"), Some("win.paste-game-state"));
        deal.append(Some("Insert Note"), Some("win.insert-note"));
        deal.append(Some("Clear Seed History"), Some("win.clear-seed-history"));
        section.append_submenu(Some("Game State"), &deal);

        let play = gio::Menu::new();
        play.append(Some("Draw"), Some("win.draw"));
        play.append(Some("Undo"), Some("win.undo"));
        play.append(Some("Redo"), Some("win.redo"));
        play.append(Some("Wave Magic Wand"), Some("win.play-hint-move"));
        play.append(Some("Rapid Wand"), Some("win.rapid-wand"));
        play.append(Some("Cyclone Shuffle Tableau"), Some("win.cyclone-shuffle"));
        play.append(Some("Peek"), Some("win.peek"));
        section.append_submenu(Some("Play"), &play);

        let smart_move = gio::Menu::new();
        smart_move.append(Some("Double Click"), Some("win.smart-move-double-click"));
        smart_move.append(Some("Single Click"), Some("win.smart-move-single-click"));
        smart_move.append(Some("Right Click"), Some("win.smart-move-right-click"));
        smart_move.append(Some("Disabled"), Some("win.smart-move-disabled"));
        section.append_submenu(Some("Smart Move"), &smart_move);

        let automation = gio::Menu::new();
        automation.append(Some("Robot Mode"), Some("win.robot-mode"));
        automation.append(Some("Forever Mode"), Some("win.forever-mode"));
        automation.append(
            Some("Auto New Game On Loss"),
            Some("win.robot-auto-new-game-on-loss"),
        );
        automation.append(Some("Ludicrous Speed"), Some("win.ludicrous-speed"));
        automation.append(
            Some("Enable App Auto-Close on Runaway Memory State"),
            Some("win.memory-guard-toggle"),
        );
        automation.append(Some("Robot Debug Mode"), Some("win.robot-debug-toggle"));
        automation.append(
            Some("Strict Debug Invariants"),
            Some("win.robot-strict-debug-invariants"),
        );
        automation.append(
            Some("Copy Benchmark Snapshot"),
            Some("win.copy-benchmark-snapshot"),
        );
        automation.append(
            Some("Clear All Cardthropic Settings and History"),
            Some("win.clear-all-settings-history"),
        );
        automation.append(
            Some("Copy All Cardthropic GSettings Variables"),
            Some("win.copy-all-gsettings-variables"),
        );
        automation.append(
            Some("Load All Cardthropic GSettings Variables From Clipboard"),
            Some("win.load-all-gsettings-variables"),
        );
        section.append_submenu(Some("Automation"), &automation);

        let view_help = gio::Menu::new();
        view_help.append(Some("New Window"), Some("app.new-window"));
        view_help.append(Some("Enable HUD"), Some("win.enable-hud"));
        view_help.append(Some("Fullscreen"), Some("win.toggle-fullscreen"));
        view_help.append(Some("Theme Presets"), Some("win.open-theme-presets"));
        view_help.append(Some("Custom CSS"), Some("win.open-custom-css"));
        view_help.append(Some("Status History"), Some("win.status-history"));
        view_help.append(Some("APM Graph"), Some("win.apm-graph"));
        section.append_submenu(Some("View"), &view_help);

        section.append(Some("Command Palette"), Some("win.command-search"));
        section.append(Some("Help"), Some("win.help"));
        section.append(Some("About Cardthropic"), Some("app.about"));
        section.append(Some("Quit"), Some("app.quit"));
        root.append_section(None, &section);
        root
    }

    pub(super) fn setup_game_mode_menu_item(&self) {
        self.refresh_main_menu_model();
    }

    pub(super) fn setup_game_settings_menu(&self) {
        self.update_game_settings_menu();
    }

    pub(super) fn apply_mode_option_by_index(&self, idx: usize) {
        match self.active_game_mode() {
            GameMode::Klondike => {
                let modes = [
                    DrawMode::One,
                    DrawMode::Two,
                    DrawMode::Three,
                    DrawMode::Four,
                    DrawMode::Five,
                ];
                if let Some(mode) = modes.get(idx).copied() {
                    self.set_klondike_draw_mode(mode);
                }
            }
            GameMode::Spider => {
                let modes = [
                    SpiderSuitMode::One,
                    SpiderSuitMode::Two,
                    SpiderSuitMode::Three,
                    SpiderSuitMode::Four,
                ];
                if let Some(mode) = modes.get(idx).copied() {
                    self.set_spider_suit_mode(mode, true);
                }
            }
            GameMode::Freecell => {
                let modes = [
                    FreecellCardCountMode::TwentySix,
                    FreecellCardCountMode::ThirtyNine,
                    FreecellCardCountMode::FiftyTwo,
                ];
                if let Some(mode) = modes.get(idx).copied() {
                    self.set_freecell_card_count_mode(mode, true);
                }
            }
        }
        self.refresh_main_menu_model();
    }

    pub(super) fn popdown_main_menu_later(&self) {
        glib::idle_add_local_once(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move || {
                window.imp().main_menu_popover.popdown();
            }
        ));
    }

    fn clear_settings_content_box(content_box: &gtk::Box) {
        while let Some(child) = content_box.first_child() {
            content_box.remove(&child);
        }
    }

    fn populate_game_settings_content_for_mode(&self, mode: GameMode, content_box: &gtk::Box) {
        let spec = variant_for_mode(mode).spec();
        let caps = engine_for_mode(mode).capabilities();
        let mode_name = spec.label;

        Self::clear_settings_content_box(content_box);

        let heading = gtk::Label::new(Some(&format!("{mode_name} Settings")));
        heading.set_xalign(0.0);
        heading.add_css_class("heading");
        content_box.append(&heading);

        if mode == GameMode::Spider {
            let suit_label = gtk::Label::new(Some("Suits"));
            suit_label.set_xalign(0.0);
            suit_label.add_css_class("dim-label");
            content_box.append(&suit_label);

            let suit_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            suit_row.set_hexpand(true);
            let suits = [
                SpiderSuitMode::One,
                SpiderSuitMode::Two,
                SpiderSuitMode::Three,
                SpiderSuitMode::Four,
            ];
            let current_suit_mode = self.current_spider_suit_mode();
            let mut group_anchor: Option<gtk::CheckButton> = None;

            for suit_mode in suits {
                let label = format!("Suit {}", suit_mode.suit_count());
                let button = gtk::CheckButton::with_label(&label);
                if let Some(anchor) = group_anchor.as_ref() {
                    button.set_group(Some(anchor));
                } else {
                    group_anchor = Some(button.clone());
                }
                if suit_mode == current_suit_mode {
                    button.set_active(true);
                }
                button.connect_toggled(glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |btn| {
                        if btn.is_active() {
                            window.set_spider_suit_mode(suit_mode, true);
                        }
                    }
                ));
                suit_row.append(&button);
            }
            content_box.append(&suit_row);
        } else if mode == GameMode::Freecell {
            let card_count_label = gtk::Label::new(Some("Card Count"));
            card_count_label.set_xalign(0.0);
            card_count_label.add_css_class("dim-label");
            content_box.append(&card_count_label);

            let card_count_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
            card_count_row.set_hexpand(true);
            let modes = [
                FreecellCardCountMode::TwentySix,
                FreecellCardCountMode::ThirtyNine,
                FreecellCardCountMode::FiftyTwo,
            ];
            let current_card_count_mode = self.current_freecell_card_count_mode();
            let mut group_anchor: Option<gtk::CheckButton> = None;

            for card_count_mode in modes {
                let label = format!("{}", card_count_mode.card_count());
                let button = gtk::CheckButton::with_label(&label);
                if let Some(anchor) = group_anchor.as_ref() {
                    button.set_group(Some(anchor));
                } else {
                    group_anchor = Some(button.clone());
                }
                if card_count_mode == current_card_count_mode {
                    button.set_active(true);
                }
                button.connect_toggled(glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |btn| {
                        if btn.is_active() {
                            window.set_freecell_card_count_mode(card_count_mode, true);
                        }
                    }
                ));
                card_count_row.append(&button);
            }
            content_box.append(&card_count_row);
        } else if caps.draw_mode_selection {
            let draw_label = gtk::Label::new(Some("Deal"));
            draw_label.set_xalign(0.0);
            draw_label.add_css_class("dim-label");
            content_box.append(&draw_label);

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
            content_box.append(&draw_row);
        } else {
            let note = gtk::Label::new(Some(spec.settings_placeholder));
            note.set_xalign(0.0);
            note.set_wrap(true);
            note.add_css_class("dim-label");
            content_box.append(&note);
        }
    }

    pub(super) fn update_game_settings_menu(&self) {
        let imp = self.imp();
        let mode = imp.current_game_mode.get();
        self.populate_game_settings_content_for_mode(mode, &imp.game_settings_content_box);
        self.refresh_main_menu_model();
    }

    pub(super) fn update_game_mode_menu_selection(&self) {
        let imp = self.imp();
        let current = imp.current_game_mode.get();
        let buttons = imp.game_mode_buttons.borrow();
        for (mode, button) in buttons.iter() {
            if *mode == current {
                button.add_css_class("game-mode-selected");
            } else {
                button.remove_css_class("game-mode-selected");
            }
        }
    }

    pub(super) fn show_freecell_cell_count_dialog(&self) {
        self.popdown_main_menu_later();

        let dialog = gtk::Window::builder()
            .title("Number of Free Cells")
            .modal(true)
            .transient_for(self)
            .default_width(360)
            .default_height(200)
            .build();
        dialog.set_resizable(false);
        dialog.set_destroy_with_parent(true);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(14);
        root.set_margin_bottom(14);
        root.set_margin_start(14);
        root.set_margin_end(14);

        let heading = gtk::Label::new(Some("Choose FreeCell Cell Count"));
        heading.set_xalign(0.0);
        heading.add_css_class("title-4");
        root.append(&heading);

        let body = gtk::Label::new(Some(
            "Set how many free cells appear at the top. 6 is the maximum that currently fits the UI.",
        ));
        body.set_xalign(0.0);
        body.set_wrap(true);
        body.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        root.append(&body);

        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
        let label = gtk::Label::new(Some("Number of Free Cells"));
        label.set_xalign(0.0);
        label.set_hexpand(true);
        row.append(&label);

        let options: Vec<String> = (FREECELL_MIN_CELL_COUNT..=FREECELL_MAX_CELL_COUNT)
            .map(|count| count.to_string())
            .collect();
        let option_refs: Vec<&str> = options.iter().map(String::as_str).collect();
        let dropdown = gtk::DropDown::from_strings(&option_refs);
        let current = self
            .current_freecell_cell_count()
            .clamp(FREECELL_MIN_CELL_COUNT, FREECELL_MAX_CELL_COUNT);
        dropdown.set_selected(u32::from(current.saturating_sub(FREECELL_MIN_CELL_COUNT)));
        dropdown.connect_selected_notify(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |dd| {
                let selected = dd.selected();
                let count = FREECELL_MIN_CELL_COUNT.saturating_add(selected as u8);
                window.set_freecell_cell_count(count, true);
                let actual = window
                    .current_freecell_cell_count()
                    .clamp(FREECELL_MIN_CELL_COUNT, FREECELL_MAX_CELL_COUNT);
                let actual_selected = u32::from(actual.saturating_sub(FREECELL_MIN_CELL_COUNT));
                if dd.selected() != actual_selected {
                    dd.set_selected(actual_selected);
                }
            }
        ));
        row.append(&dropdown);
        root.append(&row);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);

        let close = gtk::Button::with_label("Close");
        close.add_css_class("suggested-action");
        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        actions.append(&close);
        root.append(&actions);

        dialog.set_default_widget(Some(&close));
        let _ = close.grab_focus();
        dialog.set_child(Some(&root));
        dialog.present();
    }
}

use super::*;

impl CardthropicWindow {
    pub(super) fn setup_game_mode_menu_item(&self) {
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

    pub(super) fn active_game_mode(&self) -> GameMode {
        self.imp().current_game_mode.get()
    }

    pub(super) fn current_klondike_draw_mode(&self) -> DrawMode {
        self.imp().klondike_draw_mode.get()
    }

    pub(super) fn set_klondike_draw_mode(&self, draw_mode: DrawMode) {
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

    pub(super) fn is_mode_engine_ready(&self) -> bool {
        self.active_game_mode().engine_ready()
    }

    pub(super) fn guard_mode_engine(&self, action: &str) -> bool {
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

    pub(super) fn setup_game_settings_menu(&self) {
        self.update_game_settings_menu();
    }

    pub(super) fn popdown_game_settings_later(&self) {
        glib::idle_add_local_once(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move || {
                window.imp().game_settings_popover.popdown();
            }
        ));
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

    fn clear_game_settings_menu_content(&self) {
        let imp = self.imp();
        while let Some(child) = imp.game_settings_content_box.first_child() {
            imp.game_settings_content_box.remove(&child);
        }
    }

    pub(super) fn update_game_settings_menu(&self) {
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

    pub(super) fn update_game_mode_menu_selection(&self) {
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

    pub(super) fn select_game_mode(&self, mode: &str) {
        let imp = self.imp();
        self.stop_robot_mode();
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
}

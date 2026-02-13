use super::*;
use crate::engine::variant::all_variants;
use crate::engine::variant_engine::engine_for_mode;

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
        let mut buttons = HashMap::new();
        for variant in all_variants() {
            let spec = variant.spec();
            let button = gtk::Button::with_label(spec.emoji);
            button.add_css_class("flat");
            button.add_css_class("game-mode-emoji-button");
            button.set_tooltip_text(Some(spec.label));
            button.connect_clicked(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                spec,
                move |_| {
                    window.select_game_mode(spec.id);
                }
            ));
            button_box.append(&button);
            buttons.insert(spec.mode, button);
        }

        row.append(&button_box);
        imp.main_menu_popover.add_child(&row, "game-mode-row");
        *imp.game_mode_buttons.borrow_mut() = buttons;
        self.update_game_mode_menu_selection();
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
        let spec = self.mode_settings_spec();
        let caps = engine_for_mode(mode).capabilities();
        let mode_name = spec.label;
        imp.game_settings_menu_button.set_label(spec.emoji);
        imp.game_settings_menu_button
            .set_tooltip_text(Some(&format!("{mode_name} Settings")));

        self.clear_game_settings_menu_content();

        let heading = gtk::Label::new(Some(&format!("{mode_name} Settings")));
        heading.set_xalign(0.0);
        heading.add_css_class("heading");
        imp.game_settings_content_box.append(&heading);

        if caps.draw_mode_selection {
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

            if caps.seeded_deals {
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
            }

            if caps.winnability {
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
        } else {
            let note = gtk::Label::new(Some(spec.settings_placeholder));
            note.set_xalign(0.0);
            note.set_wrap(true);
            note.add_css_class("dim-label");
            imp.game_settings_content_box.append(&note);
        }
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
}

use super::*;

impl CardthropicWindow {
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
            "⛶",
            imp.fullscreen_button.tooltip_text().map(|s| s.to_string()),
            Some("win.toggle-fullscreen"),
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
            "🤖",
            imp.robot_button.tooltip_text().map(|s| s.to_string()),
            Some("win.robot-mode"),
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

    pub(super) fn show_help_dialog(&self) {
        if let Some(existing) = self.imp().help_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let entries = self.help_entries();
        let row_count = entries.len() as i32;
        let estimated_height = 170 + (row_count * 34);
        let dialog_height = estimated_height.clamp(460, 820);

        let dialog = gtk::Window::builder()
            .title("Cardthropic Help")
            .transient_for(self)
            .modal(false)
            .default_width(620)
            .default_height(dialog_height)
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

        let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
        content.set_hexpand(true);
        content.set_vexpand(true);
        for (icon, text) in entries {
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
        root.append(&content);

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

    pub(super) fn toggle_fullscreen_mode(&self) {
        if self.is_fullscreen() {
            self.unfullscreen();
            *self.imp().status_override.borrow_mut() = Some("Exited fullscreen.".to_string());
        } else {
            self.fullscreen();
            *self.imp().status_override.borrow_mut() = Some("Entered fullscreen.".to_string());
        }
        self.render();
    }
}

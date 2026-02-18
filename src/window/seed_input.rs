use super::*;
use crate::engine::seed_ops;

impl CardthropicWindow {
    #[allow(deprecated)]
    pub(super) fn seed_text_entry(&self) -> Option<gtk::Entry> {
        self.imp()
            .seed_combo
            .child()
            .and_then(|child| child.downcast::<gtk::Entry>().ok())
    }

    pub(super) fn seed_input_text(&self) -> String {
        self.seed_text_entry()
            .map(|entry| entry.text().to_string())
            .unwrap_or_default()
    }

    pub(super) fn set_seed_input_text(&self, text: &str) {
        let imp = self.imp();
        imp.seed_combo_updating.set(true);
        if let Some(entry) = self.seed_text_entry() {
            entry.set_text(text);
        }
        imp.seed_combo_updating.set(false);
    }

    pub(super) fn clear_seed_entry_feedback(&self) {
        if let Some(entry) = self.seed_text_entry() {
            entry.remove_css_class("error");
            entry.remove_css_class("seed-winnable");
            entry.remove_css_class("seed-unwinnable");
        }
    }

    pub(super) fn seed_from_controls_or_random(&self) -> Result<u64, String> {
        let text = self.seed_input_text();
        let parsed = seed_ops::parse_seed_input(&text)?;
        let seed = seed_ops::seed_from_text_or_random(&text)?;
        if parsed.is_none() {
            self.set_seed_input_text(&seed.to_string());
        }
        Ok(seed)
    }

    pub(super) fn start_new_game_from_seed_controls(&self) {
        if !self.guard_mode_engine("Starting a new deal") {
            return;
        }
        if self.imp().seed_search_in_progress.get() {
            *self.imp().status_override.borrow_mut() =
                Some(seed_ops::msg_seed_search_still_running());
            self.render();
            return;
        }

        self.cancel_seed_winnable_check(None);
        self.clear_seed_entry_feedback();
        let original_seed_label = self.seed_input_text().trim().to_string();
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

        let status = if !original_seed_label.is_empty()
            && !original_seed_label.replace('_', "").is_empty()
            && original_seed_label
                .replace('_', "")
                .chars()
                .all(|ch| ch.is_ascii_alphabetic())
        {
            format!("Started a new game. Seed {seed}, [{original_seed_label}]")
        } else {
            seed_ops::msg_started_seed(seed)
        };
        self.start_new_game_with_seed(seed, status);
        if !original_seed_label.is_empty() {
            self.set_seed_input_text(&original_seed_label);
        }
    }

    pub(super) fn start_random_seed_game(&self) {
        if !self.guard_mode_engine("Starting a random deal") {
            return;
        }
        if self.imp().seed_search_in_progress.get() {
            *self.imp().status_override.borrow_mut() =
                Some(seed_ops::msg_seed_search_still_running());
            self.render();
            return;
        }

        self.cancel_seed_winnable_check(None);
        self.clear_seed_entry_feedback();
        let seed = seed_ops::random_seed();
        self.start_new_game_with_seed(seed, seed_ops::msg_started_seed(seed));
    }

    pub(super) fn repeat_current_seed_game(&self) {
        if !self.guard_mode_engine("Repeating current seed") {
            return;
        }
        if self.imp().seed_search_in_progress.get() {
            *self.imp().status_override.borrow_mut() =
                Some(seed_ops::msg_seed_search_still_running());
            self.render();
            return;
        }

        self.cancel_seed_winnable_check(None);
        self.clear_seed_entry_feedback();
        let seed = self.imp().current_seed.get();
        self.set_seed_input_text(&seed.to_string());
        self.start_new_game_with_seed(seed, seed_ops::msg_repeated_seed(seed));
    }

    pub(super) fn show_seed_picker_dialog(&self) {
        let dialog = gtk::Window::builder()
            .title("Seed Picker")
            .transient_for(self)
            .modal(true)
            .default_width(420)
            .default_height(140)
            .build();

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(12);
        root.set_margin_end(12);

        let seed_entry = gtk::Entry::new();
        let seed_text = self.seed_input_text();
        if seed_text.trim().is_empty() {
            seed_entry.set_text(&self.imp().current_seed.get().to_string());
        } else {
            seed_entry.set_text(&seed_text);
        }
        seed_entry.set_placeholder_text(Some("Seed number or word"));
        root.append(&seed_entry);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        actions.set_halign(gtk::Align::End);

        let random_button = gtk::Button::with_label("Random");
        random_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            move |_| {
                window.start_random_seed_game();
                dialog.close();
            }
        ));
        actions.append(&random_button);

        let winnable_button = gtk::Button::with_label("Winnable");
        winnable_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            move |_| {
                window.start_random_winnable_seed_game();
                dialog.close();
            }
        ));
        actions.append(&winnable_button);

        let repeat_button = gtk::Button::with_label("Repeat");
        repeat_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            move |_| {
                window.repeat_current_seed_game();
                dialog.close();
            }
        ));
        actions.append(&repeat_button);

        let check_button = gtk::Button::with_label("Check W?");
        check_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            seed_entry,
            move |_| {
                window.set_seed_input_text(seed_entry.text().as_str());
                window.toggle_seed_winnable_check();
            }
        ));
        actions.append(&check_button);

        let start_button = gtk::Button::with_label("Start");
        start_button.add_css_class("suggested-action");
        start_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            seed_entry,
            #[weak]
            dialog,
            move |_| {
                window.set_seed_input_text(seed_entry.text().as_str());
                window.start_new_game_from_seed_controls();
                dialog.close();
            }
        ));
        actions.append(&start_button);
        root.append(&actions);

        seed_entry.connect_activate(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            seed_entry,
            #[weak]
            dialog,
            move |_| {
                window.set_seed_input_text(seed_entry.text().as_str());
                window.start_new_game_from_seed_controls();
                dialog.close();
            }
        ));

        dialog.set_child(Some(&root));
        dialog.present();
        seed_entry.grab_focus();
    }
}

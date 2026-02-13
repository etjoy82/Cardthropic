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
        if parsed.is_none() || text.trim().replace('_', "") != seed.to_string() {
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

        self.start_new_game_with_seed(seed, seed_ops::msg_started_seed(seed));
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
}

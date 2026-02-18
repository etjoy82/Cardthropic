use super::*;
use crate::engine::boundary;
use crate::engine::seed_ops;

impl CardthropicWindow {
    pub(super) fn load_seed_history(&self) {
        let raw = self
            .imp()
            .settings
            .borrow()
            .as_ref()
            .map(|settings| settings.string(SETTINGS_KEY_SEED_HISTORY).to_string())
            .or_else(|| {
                Self::load_app_settings()
                    .map(|settings| settings.string(SETTINGS_KEY_SEED_HISTORY).to_string())
            })
            .unwrap_or_default();
        let data = SeedHistoryStore::load_from_string(&raw, MAX_SEED_HISTORY_ENTRIES);
        *self.imp().seed_history.borrow_mut() = data;
    }

    pub(super) fn save_seed_history(&self) {
        if let Some(settings) = self.imp().settings.borrow().as_ref() {
            let payload = self.imp().seed_history.borrow().serialize();
            let _ = settings.set_string(SETTINGS_KEY_SEED_HISTORY, &payload);
        }
    }

    pub(super) fn note_seed_play_started(&self, seed: u64) {
        self.imp()
            .seed_history
            .borrow_mut()
            .note_play_started(seed, MAX_SEED_HISTORY_ENTRIES);
        self.imp().current_seed_win_recorded.set(false);
        self.save_seed_history();
        self.refresh_seed_history_dropdown();
    }

    pub(super) fn note_current_seed_win_if_needed(&self) {
        let mode = self.active_game_mode();
        if !boundary::is_won(&self.imp().game.borrow(), mode)
            || self.imp().current_seed_win_recorded.get()
        {
            return;
        }

        let seed = self.imp().current_seed.get();
        self.imp().seed_history.borrow_mut().note_win(seed);

        self.imp().current_seed_win_recorded.set(true);
        self.save_seed_history();
        self.refresh_seed_history_dropdown();
    }

    #[allow(deprecated)]
    pub(super) fn refresh_seed_history_dropdown(&self) {
        let imp = self.imp();
        let current_text = self.seed_input_text();

        imp.seed_combo_updating.set(true);
        imp.seed_combo.remove_all();

        let (seeds, total_seed_count) = imp
            .seed_history
            .borrow()
            .dropdown_entries(MAX_SEED_DROPDOWN_ENTRIES);
        for (seed, stats) in seeds {
            imp.seed_combo.append(
                Some(&seed.to_string()),
                &format!("{seed}: Plays {}, Wins {}", stats.plays, stats.wins),
            );
        }

        if let Some(entry) = self.seed_text_entry() {
            let tooltip =
                seed_ops::seed_dropdown_tooltip(total_seed_count, MAX_SEED_DROPDOWN_ENTRIES);
            entry.set_tooltip_text(tooltip.as_deref());
        }

        imp.seed_combo_updating.set(false);
        self.set_seed_input_text(&current_text);
    }
}

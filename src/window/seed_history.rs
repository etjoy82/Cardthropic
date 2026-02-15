use super::*;
use crate::engine::boundary;
use crate::engine::seed_ops;

impl CardthropicWindow {
    pub(super) fn seed_history_file_path() -> std::path::PathBuf {
        let mut path = glib::user_data_dir();
        path.push(APP_DATA_DIR_NAME);
        path.push(SEED_HISTORY_FILE_NAME);
        path
    }

    pub(super) fn load_seed_history(&self) {
        let path = Self::seed_history_file_path();
        let data = SeedHistoryStore::load_from_path(&path, MAX_SEED_HISTORY_ENTRIES);
        *self.imp().seed_history.borrow_mut() = data;
    }

    pub(super) fn save_seed_history(&self) {
        let path = Self::seed_history_file_path();
        self.imp().seed_history.borrow().save_to_path(&path);
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

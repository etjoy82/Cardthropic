use super::*;

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
        let parsed = parse_seed_input(&text)?;
        let seed = parsed.unwrap_or_else(random_seed);
        if parsed.is_none() || text.trim() != seed.to_string() {
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

    pub(super) fn start_random_seed_game(&self) {
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

    pub(super) fn repeat_current_seed_game(&self) {
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

    pub(super) fn start_new_game_with_seed(&self, seed: u64, status: String) {
        self.start_new_game_with_seed_internal(seed, status, false);
    }

    pub(super) fn start_new_game_with_seed_internal(
        &self,
        seed: u64,
        status: String,
        preserve_robot: bool,
    ) {
        let imp = self.imp();
        if !preserve_robot {
            self.stop_robot_mode();
        }
        self.cancel_seed_winnable_check(None);
        self.clear_hint_effects();
        *imp.game.borrow_mut() = KlondikeGame::new_with_seed(seed);
        imp.robot_playback.borrow_mut().clear();
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

    pub(super) fn note_current_seed_win_if_needed(&self, game: &KlondikeGame) {
        if !game.is_won() || self.imp().current_seed_win_recorded.get() {
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
            if total_seed_count > MAX_SEED_DROPDOWN_ENTRIES {
                entry.set_tooltip_text(Some(&format!(
                    "Showing latest {} of {} seeds. Type any seed number to load.",
                    MAX_SEED_DROPDOWN_ENTRIES, total_seed_count
                )));
            } else {
                entry.set_tooltip_text(None);
            }
        }

        imp.seed_combo_updating.set(false);
        self.set_seed_input_text(&current_text);
    }
}

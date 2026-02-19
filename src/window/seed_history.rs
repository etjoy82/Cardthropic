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

    fn should_defer_seed_history_dropdown_refresh(&self) -> bool {
        let imp = self.imp();
        imp.robot_mode_running.get()
            && imp.robot_ludicrous_enabled.get()
            && imp.robot_forever_enabled.get()
            && imp.robot_auto_new_game_on_loss.get()
    }

    fn mark_seed_history_dirty(&self) {
        if !self.should_persist_shared_state() {
            return;
        }
        let imp = self.imp();
        imp.seed_history_dirty.set(true);
        if imp.seed_history_flush_timer.borrow().is_some() {
            return;
        }

        let timer = glib::timeout_add_seconds_local(
            SEED_HISTORY_FLUSH_INTERVAL_SECS,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    window.flush_seed_history_now();
                    glib::ControlFlow::Break
                }
            ),
        );
        *imp.seed_history_flush_timer.borrow_mut() = Some(timer);
    }

    pub(super) fn flush_seed_history_now(&self) {
        let imp = self.imp();
        if let Some(source_id) = imp.seed_history_flush_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }

        if imp.seed_history_dirty.replace(false) {
            if let Some(settings) = imp.settings.borrow().as_ref() {
                let payload = imp.seed_history.borrow().serialize();
                let _ = settings.set_string(SETTINGS_KEY_SEED_HISTORY, &payload);
            }
        }

        if imp.seed_history_dropdown_dirty.get()
            && !self.should_defer_seed_history_dropdown_refresh()
        {
            self.refresh_seed_history_dropdown_immediate();
            imp.seed_history_dropdown_dirty.set(false);
        }
    }

    pub(super) fn note_seed_play_started(&self, seed: u64) {
        self.imp()
            .seed_history
            .borrow_mut()
            .note_play_started(seed, MAX_SEED_HISTORY_ENTRIES);
        self.imp().current_seed_win_recorded.set(false);
        self.mark_seed_history_dirty();
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
        self.mark_seed_history_dirty();
        if self.should_defer_seed_history_dropdown_refresh() {
            self.imp().seed_history_dropdown_dirty.set(true);
            return;
        }
        self.refresh_seed_history_dropdown_immediate();
    }

    #[allow(deprecated)]
    pub(super) fn refresh_seed_history_dropdown(&self) {
        if self.should_defer_seed_history_dropdown_refresh() {
            self.imp().seed_history_dropdown_dirty.set(true);
            return;
        }
        self.refresh_seed_history_dropdown_immediate();
    }

    #[allow(deprecated)]
    fn refresh_seed_history_dropdown_immediate(&self) {
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
        imp.seed_history_dropdown_dirty.set(false);
    }
}

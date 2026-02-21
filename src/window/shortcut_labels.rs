use super::*;

impl CardthropicWindow {
    pub(super) fn shortcut_labels_for_action(&self, detailed_action: &str) -> Vec<String> {
        let Some(app) = self.application() else {
            return Vec::new();
        };

        app.accels_for_action(detailed_action)
            .into_iter()
            .filter_map(|accel| {
                let accel = accel.to_string();
                if accel.trim().is_empty() {
                    return None;
                }
                if let Some((key, mods)) = gtk::accelerator_parse(&accel) {
                    Some(gtk::accelerator_get_label(key, mods).to_string())
                } else {
                    Some(accel)
                }
            })
            .collect()
    }

    fn shortcut_text_for_action(&self, detailed_action: &str) -> Option<String> {
        let labels = self.shortcut_labels_for_action(detailed_action);
        if labels.is_empty() {
            None
        } else {
            Some(labels.join(" / "))
        }
    }

    fn with_shortcut_suffix(label: &str, shortcut: &str) -> String {
        let shortcut = shortcut.trim();
        if shortcut.is_empty() {
            return label.to_string();
        }
        format!("{label} ({shortcut})")
    }

    pub(super) fn main_button_tooltip_with_shortcut(
        &self,
        tooltip: &str,
        detailed_action: &str,
    ) -> String {
        self.shortcut_text_for_action(detailed_action)
            .map(|shortcut| Self::with_shortcut_suffix(tooltip, &shortcut))
            .unwrap_or_else(|| tooltip.to_string())
    }

    pub(super) fn main_button_label_with_shortcut(
        &self,
        label: &str,
        detailed_action: &str,
    ) -> String {
        self.shortcut_text_for_action(detailed_action)
            .map(|shortcut| Self::with_shortcut_suffix(label, &shortcut))
            .unwrap_or_else(|| label.to_string())
    }

    fn seed_shortcut_hint_text(&self) -> String {
        let mut parts = Vec::new();
        if let Some(shortcut) = self.shortcut_text_for_action("win.seed-picker") {
            parts.push(shortcut);
        }
        parts.push("Enter".to_string());
        parts.join(" / ")
    }

    fn seed_field_tooltip_label(&self) -> String {
        Self::with_shortcut_suffix("Game seed", &self.seed_shortcut_hint_text())
    }

    fn seed_entry_placeholder_label(&self) -> String {
        Self::with_shortcut_suffix(
            "Blank=random, or enter seed number/word",
            &self.seed_shortcut_hint_text(),
        )
    }

    pub(super) fn seed_go_button_shortcut_label(&self) -> String {
        "Enter".to_string()
    }

    fn seed_go_tooltip_label(&self) -> String {
        Self::with_shortcut_suffix("Start Game from Seed", "Enter")
    }

    pub(super) fn seed_winnable_idle_button_label(&self) -> String {
        self.main_button_label_with_shortcut(SEED_WINNABLE_BUTTON_LABEL, "win.check-seed-winnable")
    }

    pub(super) fn seed_winnable_progress_button_label(&self, phase: &str, seconds: u32) -> String {
        let base = format!("{phase} {seconds}s");
        self.shortcut_text_for_action("win.check-seed-winnable")
            .map(|shortcut| Self::with_shortcut_suffix(&base, &shortcut))
            .unwrap_or(base)
    }

    pub(super) fn seed_winnable_stopping_button_label(&self) -> String {
        self.shortcut_text_for_action("win.check-seed-winnable")
            .map(|shortcut| Self::with_shortcut_suffix("Stopping...", &shortcut))
            .unwrap_or_else(|| "Stopping...".to_string())
    }

    pub(super) fn robot_button_tooltip_label(&self, forever_enabled: bool) -> String {
        let base = if forever_enabled {
            "Robot Mode (Forever Mode enabled)"
        } else {
            "Robot Mode"
        };
        self.main_button_tooltip_with_shortcut(base, "win.robot-mode")
    }

    pub(super) fn apply_main_window_shortcut_labels(&self) {
        let imp = self.imp();
        imp.hud_button
            .set_label(&self.main_button_label_with_shortcut("HUD", "win.enable-hud"));
        imp.hud_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut("Toggle HUD", "win.enable-hud"),
        ));
        imp.board_color_menu_button
            .set_label(&self.main_button_label_with_shortcut("🎨", "win.open-theme-presets"));
        imp.board_color_menu_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut("Theme presets", "win.open-theme-presets"),
        ));
        imp.undo_button
            .set_label(&self.main_button_label_with_shortcut("↶", "win.undo"));
        imp.undo_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut("Undo", "win.undo"),
        ));
        imp.redo_button
            .set_label(&self.main_button_label_with_shortcut("↷", "win.redo"));
        imp.redo_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut("Redo", "win.redo"),
        ));
        imp.auto_hint_button
            .set_label(&self.main_button_label_with_shortcut("🪄", "win.play-hint-move"));
        imp.auto_hint_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut("Wave Magic Wand", "win.play-hint-move"),
        ));
        imp.cyclone_shuffle_button
            .set_label(&self.main_button_label_with_shortcut("🌀", "win.cyclone-shuffle"));
        imp.cyclone_shuffle_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut(
                "Cyclone Shuffle Tableau",
                "win.cyclone-shuffle",
            ),
        ));
        imp.peek_button
            .set_label(&self.main_button_label_with_shortcut("🫣", "win.peek"));
        imp.peek_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut("Peek", "win.peek"),
        ));
        imp.robot_button
            .set_label(&self.main_button_label_with_shortcut("🤖", "win.robot-mode"));
        imp.robot_button.set_tooltip_text(Some(
            &self.robot_button_tooltip_label(imp.robot_forever_enabled.get()),
        ));
        imp.command_search_button
            .set_label(&self.main_button_label_with_shortcut("/", "win.command-search"));
        imp.command_search_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut("Open Command Palette", "win.command-search"),
        ));
        imp.seed_random_button
            .set_label(&self.main_button_label_with_shortcut("🎲", "win.random-seed"));
        imp.seed_random_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut("Start Random Game", "win.random-seed"),
        ));
        imp.seed_rescue_button
            .set_label(&self.main_button_label_with_shortcut("🛟", "win.winnable-seed"));
        imp.seed_rescue_button
            .set_tooltip_text(Some(&self.main_button_tooltip_with_shortcut(
                "Find and Start Winnable Game",
                "win.winnable-seed",
            )));
        imp.seed_winnable_button
            .set_label(&self.seed_winnable_idle_button_label());
        imp.seed_winnable_button
            .set_tooltip_text(Some(&self.main_button_tooltip_with_shortcut(
                "Check if Seed is Winnable",
                "win.check-seed-winnable",
            )));
        imp.seed_repeat_button
            .set_label(&self.main_button_label_with_shortcut("🔁", "win.repeat-seed"));
        imp.seed_repeat_button
            .set_tooltip_text(Some(&self.main_button_tooltip_with_shortcut(
                "Start Game Again from Current Seed",
                "win.repeat-seed",
            )));
        imp.seed_go_button
            .set_label(&self.seed_go_button_shortcut_label());
        imp.seed_go_button
            .set_tooltip_text(Some(&self.seed_go_tooltip_label()));
        imp.status_history_button
            .set_label(&self.main_button_label_with_shortcut("History", "win.status-history"));
        imp.status_history_button.set_tooltip_text(Some(
            &self.main_button_tooltip_with_shortcut("Show status history", "win.status-history"),
        ));

        let seed_tooltip = self.seed_field_tooltip_label();
        imp.seed_combo.set_tooltip_text(Some(&seed_tooltip));
        if let Some(seed_entry) = self.seed_text_entry() {
            seed_entry.set_placeholder_text(Some(&self.seed_entry_placeholder_label()));
            seed_entry.set_tooltip_text(Some(&seed_tooltip));
        }
    }
}

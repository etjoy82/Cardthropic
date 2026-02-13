use super::*;

impl CardthropicWindow {
    fn wand_status_message(raw: &str) -> String {
        raw.trim_start()
            .strip_prefix("Hint:")
            .map(str::trim_start)
            .unwrap_or(raw)
            .to_string()
    }

    #[allow(dead_code)]
    pub(super) fn show_hint(&self) {
        if !self.guard_mode_engine("Hint") {
            return;
        }
        let suggestion = self.compute_hint_suggestion();
        *self.imp().status_override.borrow_mut() = Some(suggestion.message);
        self.render();
        if let (Some(source), Some(target)) = (suggestion.source, suggestion.target) {
            self.play_hint_animation(source, target);
        }
    }

    pub(super) fn play_hint_for_player(&self) -> bool {
        if !self.guard_mode_engine("Play hint move") {
            return false;
        }
        self.clear_hint_effects();
        let suggestion = self.compute_auto_play_suggestion();
        let wand_message = Self::wand_status_message(&suggestion.message);
        let Some(hint_move) = suggestion.hint_move else {
            *self.imp().status_override.borrow_mut() = Some(format!("Wand Wave: {wand_message}"));
            self.render();
            return false;
        };

        self.imp().auto_playing_move.set(true);
        let changed = self.apply_hint_move(hint_move);
        self.imp().auto_playing_move.set(false);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            *self.imp().status_override.borrow_mut() = Some(format!("Wand Wave: {wand_message}"));
            self.render();
        } else {
            *self.imp().status_override.borrow_mut() =
                Some("Wand Wave: move was not legal anymore.".to_string());
            self.render();
        }
        changed
    }

    pub(super) fn apply_hint_move(&self, hint_move: HintMove) -> bool {
        match hint_move {
            HintMove::WasteToFoundation => self.move_waste_to_foundation(),
            HintMove::TableauTopToFoundation { src } => self.move_tableau_to_foundation(src),
            HintMove::WasteToTableau { dst } => self.move_waste_to_tableau(dst),
            HintMove::TableauRunToTableau { src, start, dst } => {
                self.move_tableau_run_to_tableau(src, start, dst)
            }
            HintMove::Draw => self.draw_card(),
        }
    }
}

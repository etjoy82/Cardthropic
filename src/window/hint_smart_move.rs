use super::*;
use crate::engine::boundary;
use crate::engine::smart_move;

impl CardthropicWindow {
    pub(super) fn try_smart_move_from_tableau(&self, col: usize, start: usize) -> bool {
        if !self.guard_mode_engine("Smart Move") {
            return false;
        }
        let mode = self.active_game_mode();
        if smart_move::direct_tableau_to_foundation_move(
            &self.imp().game.borrow(),
            mode,
            col,
            start,
        )
        .is_some()
        {
            let changed = self.move_tableau_to_foundation(col);
            if changed {
                *self.imp().selected_run.borrow_mut() = None;
                *self.imp().status_override.borrow_mut() =
                    Some(format!("Smart Move: moved T{} to foundation.", col + 1));
                self.render();
            }
            return changed;
        }

        let primary_source = HintNode::Tableau {
            col,
            index: Some(start),
        };
        let top_index =
            boundary::tableau_len(&self.imp().game.borrow(), self.active_game_mode(), col)
                .and_then(|len| if len == 0 { None } else { Some(len - 1) });
        let fallback_source = top_index.and_then(|top| {
            if top != start {
                Some(HintNode::Tableau {
                    col,
                    index: Some(top),
                })
            } else {
                None
            }
        });

        let mut sources = vec![primary_source];
        if let Some(source) = fallback_source {
            sources.push(source);
        }
        let suggestion = self.compute_auto_play_suggestion_for_sources(
            &sources,
            "Smart Move: no legal move from that card.",
        );
        let (hint_move, used_fallback) = if let Some(hint_move) = suggestion.hint_move {
            (hint_move, false)
        } else {
            let fallback =
                smart_move::fallback_tableau_run_move(&self.imp().game.borrow(), mode, col, start);
            let Some(fallback_move) = fallback else {
                *self.imp().status_override.borrow_mut() =
                    Some("Smart Move: no legal move from that card.".to_string());
                self.render();
                return false;
            };
            (fallback_move, true)
        };

        let changed = self.apply_hint_move(hint_move);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            let message = if used_fallback {
                format!("moved T{} run to a legal tableau column.", col + 1)
            } else {
                suggestion
                    .message
                    .strip_prefix("Hint: ")
                    .unwrap_or(suggestion.message.as_str())
                    .to_string()
            };
            *self.imp().status_override.borrow_mut() = Some(format!("Smart Move: {message}"));
            self.render();
        }
        changed
    }

    pub(super) fn try_smart_move_from_waste(&self) -> bool {
        if !self.guard_mode_engine("Smart Move") {
            return false;
        }
        let mode = self.active_game_mode();
        if smart_move::direct_waste_to_foundation_move(&self.imp().game.borrow(), mode).is_some() {
            let changed = self.move_waste_to_foundation();
            if changed {
                *self.imp().selected_run.borrow_mut() = None;
                self.imp().waste_selected.set(false);
                *self.imp().status_override.borrow_mut() =
                    Some("Smart Move: moved waste to foundation.".to_string());
                self.render();
            }
            return changed;
        }

        let suggestion = self.compute_auto_play_suggestion_for_sources(
            &[HintNode::Waste],
            "Smart Move: no legal move from waste.",
        );
        let (hint_move, used_fallback) = if let Some(hint_move) = suggestion.hint_move {
            (hint_move, false)
        } else {
            let fallback =
                smart_move::fallback_waste_to_tableau_move(&self.imp().game.borrow(), mode);
            let Some(fallback_move) = fallback else {
                *self.imp().status_override.borrow_mut() =
                    Some("Smart Move: no legal move from waste.".to_string());
                self.render();
                return false;
            };
            (fallback_move, true)
        };

        let changed = self.apply_hint_move(hint_move);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            self.imp().waste_selected.set(false);
            let message = if used_fallback {
                "moved waste to a legal tableau column.".to_string()
            } else {
                suggestion
                    .message
                    .strip_prefix("Hint: ")
                    .unwrap_or(suggestion.message.as_str())
                    .to_string()
            };
            *self.imp().status_override.borrow_mut() = Some(format!("Smart Move: {message}"));
            self.render();
        }
        changed
    }
}

use super::*;
use crate::engine::boundary;
use crate::engine::smart_move;

impl CardthropicWindow {
    pub(super) fn try_smart_move_from_tableau(&self, col: usize, start: usize) -> bool {
        if !self.guard_mode_engine("Smart Move") {
            return false;
        }
        if self.active_game_mode() == GameMode::Freecell {
            return self.try_smart_move_from_tableau_freecell(col, start);
        }
        if !self.is_face_up_tableau_run(col, start) {
            self.flash_smart_move_fail_tableau_run(col, start);
            *self.imp().status_override.borrow_mut() =
                Some("Smart Move: no legal move from that card.".to_string());
            self.render();
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
            if mode == GameMode::Spider {
                self.flash_smart_move_fail_tableau_run(col, start);
                *self.imp().status_override.borrow_mut() =
                    Some("Smart Move: no legal move from that card.".to_string());
                self.render();
                return false;
            }
            let fallback =
                smart_move::fallback_tableau_run_move(&self.imp().game.borrow(), mode, col, start);
            let Some(fallback_move) = fallback else {
                self.flash_smart_move_fail_tableau_run(col, start);
                *self.imp().status_override.borrow_mut() =
                    Some("Smart Move: no legal move from that card.".to_string());
                self.render();
                return false;
            };
            (fallback_move, true)
        };

        if let HintMove::TableauRunToTableau { src, start, .. } = hint_move {
            if !self.is_face_up_tableau_run(src, start) {
                self.flash_smart_move_fail_tableau_run(col, start);
                *self.imp().status_override.borrow_mut() =
                    Some("Smart Move: no legal move from that card.".to_string());
                self.render();
                return false;
            }
        }

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
            if mode == GameMode::Spider {
                self.flash_smart_move_fail_waste_top();
                *self.imp().status_override.borrow_mut() =
                    Some("Smart Move: no legal move from waste.".to_string());
                self.render();
                return false;
            }
            let fallback =
                smart_move::fallback_waste_to_tableau_move(&self.imp().game.borrow(), mode);
            let Some(fallback_move) = fallback else {
                self.flash_smart_move_fail_waste_top();
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

    pub(super) fn try_smart_move_from_freecell(&self, cell: usize) -> bool {
        if !self.guard_mode_engine("Smart Move") {
            return false;
        }
        if self.active_game_mode() != GameMode::Freecell {
            return false;
        }
        let allowed = [HintNode::Freecell(cell)];
        let Some((message, _source, _target, action, score)) =
            self.compute_best_freecell_action(Some(&allowed))
        else {
            *self.imp().status_override.borrow_mut() =
                Some(format!("Smart Move: no legal move from F{}.", cell + 1));
            self.render();
            return false;
        };
        let changed = self.apply_freecell_hint_action(action);
        if changed {
            self.imp().selected_freecell.set(None);
            let debug_suffix = if self.imp().robot_debug_enabled.get() {
                format!(
                    " | fc_score={} fc_action={}",
                    score,
                    Self::freecell_action_tag(action)
                )
            } else {
                String::new()
            };
            *self.imp().status_override.borrow_mut() = Some(format!(
                "Smart Move: {}{}",
                message.strip_prefix("Hint: ").unwrap_or(message.as_str()),
                debug_suffix
            ));
            self.render();
            true
        } else {
            let detailed = self
                .imp()
                .status_override
                .borrow()
                .as_deref()
                .map(str::to_string);
            *self.imp().status_override.borrow_mut() = Some(if let Some(reason) = detailed {
                format!("Smart Move: {reason}")
            } else {
                "Smart Move: move was not legal anymore.".to_string()
            });
            self.render();
            false
        }
    }

    fn try_smart_move_from_tableau_freecell(&self, col: usize, start: usize) -> bool {
        if !self.is_face_up_tableau_run(col, start) {
            self.flash_smart_move_fail_tableau_run(col, start);
            *self.imp().status_override.borrow_mut() =
                Some("Smart Move: no legal move from that card.".to_string());
            self.render();
            return false;
        }
        let is_top = boundary::tableau_len(&self.imp().game.borrow(), GameMode::Freecell, col)
            .map(|len| start + 1 == len)
            .unwrap_or(false);
        let mut allowed = vec![HintNode::Tableau {
            col,
            index: Some(start),
        }];
        if is_top {
            allowed.push(HintNode::Tableau {
                col,
                index: boundary::tableau_len(&self.imp().game.borrow(), GameMode::Freecell, col)
                    .and_then(|len| len.checked_sub(1)),
            });
        }
        let Some((message, _source, _target, action, score)) =
            self.compute_best_freecell_action(Some(allowed.as_slice()))
        else {
            self.flash_smart_move_fail_tableau_run(col, start);
            *self.imp().status_override.borrow_mut() =
                Some("Smart Move: no legal move from that card.".to_string());
            self.render();
            return false;
        };
        let changed = self.apply_freecell_hint_action(action);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            let debug_suffix = if self.imp().robot_debug_enabled.get() {
                format!(
                    " | fc_score={} fc_action={}",
                    score,
                    Self::freecell_action_tag(action)
                )
            } else {
                String::new()
            };
            *self.imp().status_override.borrow_mut() = Some(format!(
                "Smart Move: {}{}",
                message.strip_prefix("Hint: ").unwrap_or(message.as_str()),
                debug_suffix
            ));
            self.render();
            return true;
        }
        self.flash_smart_move_fail_tableau_run(col, start);
        let detailed = self
            .imp()
            .status_override
            .borrow()
            .as_deref()
            .map(str::to_string);
        *self.imp().status_override.borrow_mut() = Some(if let Some(reason) = detailed {
            format!("Smart Move: {reason}")
        } else {
            "Smart Move: move was not legal anymore.".to_string()
        });
        self.render();
        false
    }
}

use super::*;

impl CardthropicWindow {
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
        let Some(hint_move) = suggestion.hint_move else {
            *self.imp().status_override.borrow_mut() = Some(suggestion.message);
            self.render();
            return false;
        };

        self.imp().auto_playing_move.set(true);
        let changed = self.apply_hint_move(hint_move);
        self.imp().auto_playing_move.set(false);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            *self.imp().status_override.borrow_mut() =
                Some(format!("Auto: {}", suggestion.message));
            self.render();
        } else {
            *self.imp().status_override.borrow_mut() =
                Some("Auto-hint move was not legal anymore.".to_string());
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

    pub(super) fn try_smart_move_from_tableau(&self, col: usize, start: usize) -> bool {
        if !self.guard_mode_engine("Smart Move") {
            return false;
        }
        let primary_source = HintNode::Tableau {
            col,
            index: Some(start),
        };
        let top_index = self.imp().game.borrow().tableau_len(col).and_then(|len| {
            if len == 0 {
                None
            } else {
                Some(len - 1)
            }
        });
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
        let Some(hint_move) = suggestion.hint_move else {
            *self.imp().status_override.borrow_mut() =
                Some("Smart Move: no legal move from that card.".to_string());
            self.render();
            return false;
        };

        let changed = self.apply_hint_move(hint_move);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            let message = suggestion
                .message
                .strip_prefix("Hint: ")
                .unwrap_or(suggestion.message.as_str());
            *self.imp().status_override.borrow_mut() = Some(format!("Smart Move: {message}"));
            self.render();
        }
        changed
    }

    pub(super) fn try_smart_move_from_waste(&self) -> bool {
        if !self.guard_mode_engine("Smart Move") {
            return false;
        }
        let suggestion = self.compute_auto_play_suggestion_for_sources(
            &[HintNode::Waste],
            "Smart Move: no legal move from waste.",
        );
        let Some(hint_move) = suggestion.hint_move else {
            *self.imp().status_override.borrow_mut() =
                Some("Smart Move: no legal move from waste.".to_string());
            self.render();
            return false;
        };

        let changed = self.apply_hint_move(hint_move);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            self.imp().waste_selected.set(false);
            let message = suggestion
                .message
                .strip_prefix("Hint: ")
                .unwrap_or(suggestion.message.as_str());
            *self.imp().status_override.borrow_mut() = Some(format!("Smart Move: {message}"));
            self.render();
        }
        changed
    }

    #[allow(dead_code)]
    pub(super) fn compute_hint_suggestion(&self) -> HintSuggestion {
        let mut game = self.imp().game.borrow().clone();
        game.set_draw_mode(self.current_klondike_draw_mode());
        if game.is_won() {
            return HintSuggestion {
                message: "Hint: game already won.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let state_hash = hash_game_state(&game);
        let seen_states: HashSet<u64> = self
            .imp()
            .hint_recent_states
            .borrow()
            .iter()
            .copied()
            .chain(std::iter::once(state_hash))
            .collect();

        if let Some(LossVerdict::Lost { explored_states }) =
            self.cached_loss_verdict_for_hash(state_hash)
        {
            return HintSuggestion {
                message: format!(
                    "Hint: no winning path found from this position (explored {explored_states} states)."
                ),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let candidates = self.enumerate_hint_candidates(&game);
        if candidates.is_empty() {
            self.start_hint_loss_analysis_if_needed(state_hash);
            return match self.cached_loss_verdict_for_hash(state_hash) {
                Some(LossVerdict::Lost { explored_states }) => HintSuggestion {
                    message: format!(
                        "Hint: no legal moves and no winning path found (explored {explored_states} states)."
                    ),
                    source: None,
                    target: None,
                    hint_move: None,
                },
                Some(LossVerdict::Inconclusive { explored_states }) => HintSuggestion {
                    message: format!(
                        "Hint: no legal moves. Analysis explored {explored_states} states but is inconclusive."
                    ),
                    source: None,
                    target: None,
                    hint_move: None,
                },
                Some(LossVerdict::WinnableLikely) => HintSuggestion {
                    message: "Hint: no legal move from here. Try undo/new game.".to_string(),
                    source: None,
                    target: None,
                    hint_move: None,
                },
                None => HintSuggestion {
                    message: "Hint: no legal moves. Running deeper loss analysis...".to_string(),
                    source: None,
                    target: None,
                    hint_move: None,
                },
            };
        }

        let mut best: Option<(i64, HintSuggestion)> = None;
        let mut node_budget = AUTO_PLAY_NODE_BUDGET;
        for candidate in candidates {
            let Some(hint_move) = candidate.hint_move else {
                continue;
            };

            let mut next_state = game.clone();
            if !apply_hint_move_to_game(&mut next_state, hint_move) {
                continue;
            }

            let next_hash = hash_game_state(&next_state);
            if seen_states.contains(&next_hash) {
                continue;
            }
            if self.is_functionally_useless_auto_move(&game, &next_state, hint_move, &seen_states) {
                continue;
            }

            let mut score =
                score_hint_candidate(&game, &next_state, hint_move, &seen_states, next_hash);
            score += self.unseen_followup_count(&next_state, &seen_states) * 35;
            if next_state.is_won() {
                score += AUTO_PLAY_WIN_SCORE;
            }
            let mut path_seen = HashSet::new();
            path_seen.insert(state_hash);
            path_seen.insert(next_hash);
            let lookahead = self.auto_play_lookahead_score(
                &next_state,
                &seen_states,
                &mut path_seen,
                AUTO_PLAY_LOOKAHEAD_DEPTH.saturating_sub(1),
                &mut node_budget,
            );
            score += lookahead / 3;
            if self.is_king_to_empty_without_unlock(&game, hint_move) {
                score -= 4_000;
            }

            match &best {
                None => best = Some((score, candidate)),
                Some((best_score, _)) if score > *best_score => best = Some((score, candidate)),
                _ => {}
            }
        }

        if let Some((_, suggestion)) = best {
            suggestion
        } else {
            self.start_hint_loss_analysis_if_needed(state_hash);
            HintSuggestion {
                message:
                    "Hint: no productive move found from this position. The line appears lost."
                        .to_string(),
                source: None,
                target: None,
                hint_move: None,
            }
        }
    }

    pub(super) fn enumerate_hint_candidates(&self, game: &KlondikeGame) -> Vec<HintSuggestion> {
        let mut candidates = Vec::new();

        if self.can_auto_move_waste_to_foundation(game) {
            let foundation = game
                .waste_top()
                .map(|card| card.suit.foundation_index())
                .unwrap_or(0);
            candidates.push(HintSuggestion {
                message: "Hint: Move waste to foundation.".to_string(),
                source: Some(HintNode::Waste),
                target: Some(HintNode::Foundation(foundation)),
                hint_move: Some(HintMove::WasteToFoundation),
            });
        }

        for src in 0..7 {
            if !self.can_auto_move_tableau_to_foundation(game, src) {
                continue;
            }
            let foundation = game
                .tableau_top(src)
                .map(|card| card.suit.foundation_index())
                .unwrap_or(0);
            let len = game.tableau_len(src).unwrap_or(1);
            candidates.push(HintSuggestion {
                message: format!("Hint: Move T{} top card to foundation.", src + 1),
                source: Some(HintNode::Tableau {
                    col: src,
                    index: len.checked_sub(1),
                }),
                target: Some(HintNode::Foundation(foundation)),
                hint_move: Some(HintMove::TableauTopToFoundation { src }),
            });
        }

        for dst in 0..7 {
            if game.can_move_waste_to_tableau(dst) {
                candidates.push(HintSuggestion {
                    message: format!("Hint: Move waste card to T{}.", dst + 1),
                    source: Some(HintNode::Waste),
                    target: Some(HintNode::Tableau {
                        col: dst,
                        index: None,
                    }),
                    hint_move: Some(HintMove::WasteToTableau { dst }),
                });
            }
        }

        for src in 0..7 {
            let len = game.tableau_len(src).unwrap_or(0);
            for start in 0..len {
                for dst in 0..7 {
                    if !game.can_move_tableau_run_to_tableau(src, start, dst) {
                        continue;
                    }
                    let amount = len.saturating_sub(start);
                    candidates.push(HintSuggestion {
                        message: format!(
                            "Hint: Move {amount} card(s) T{} -> T{}.",
                            src + 1,
                            dst + 1
                        ),
                        source: Some(HintNode::Tableau {
                            col: src,
                            index: Some(start),
                        }),
                        target: Some(HintNode::Tableau {
                            col: dst,
                            index: None,
                        }),
                        hint_move: Some(HintMove::TableauRunToTableau { src, start, dst }),
                    });
                }
            }
        }

        if game.stock_len() > 0 || game.waste_top().is_some() {
            candidates.push(HintSuggestion {
                message: "Hint: Draw from stock.".to_string(),
                source: Some(HintNode::Stock),
                target: Some(HintNode::Stock),
                hint_move: Some(HintMove::Draw),
            });
        }

        candidates
    }
}

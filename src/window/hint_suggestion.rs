use super::*;
use crate::engine::boundary;
use crate::engine::hinting;
use crate::engine::loss_analysis::LossVerdict;

impl CardthropicWindow {
    #[allow(dead_code)]
    pub(super) fn compute_hint_suggestion(&self) -> HintSuggestion {
        let Some(game) = boundary::clone_klondike_for_automation(
            &self.imp().game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        ) else {
            return HintSuggestion {
                message: "Hint: not available for this game mode yet.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        };
        let profile = self.automation_profile();
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

        let candidates = hinting::enumerate_hint_candidates(&game);
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
        let mut node_budget = profile.auto_play_node_budget;
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
                score += profile.auto_play_win_score;
            }
            let mut path_seen = HashSet::new();
            path_seen.insert(state_hash);
            path_seen.insert(next_hash);
            let lookahead = self.auto_play_lookahead_score(
                &next_state,
                &seen_states,
                &mut path_seen,
                profile.auto_play_lookahead_depth.saturating_sub(1),
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
}

use super::*;
use crate::engine::automation::AutomationProfile;
use crate::engine::autoplay;
use crate::engine::autoplay_search;
use crate::engine::boundary;
use crate::engine::hinting;
use crate::engine::loss_analysis::{self, LossVerdict};
use crate::winnability;
use std::collections::VecDeque;

const MAX_AUTO_PLAY_SEEN_STATES: usize = 50_000;
const MAX_HINT_LOSS_CACHE: usize = 4_096;
const MAX_AUTO_PLAY_SEEN_STATES_KLONDIKE_LUDICROUS: usize = 8_192;
const MAX_AUTO_PLAY_SEEN_SNAPSHOT_KLONDIKE_LUDICROUS: usize = 2_048;

impl CardthropicWindow {
    fn klondike_ludicrous_robot_mode_active(&self) -> bool {
        let imp = self.imp();
        self.active_game_mode() == GameMode::Klondike
            && imp.robot_mode_running.get()
            && imp.robot_ludicrous_enabled.get()
    }

    fn auto_play_seen_states_snapshot(&self) -> HashSet<u64> {
        let seen = self.imp().auto_play_seen_states.borrow();
        if self.klondike_ludicrous_robot_mode_active() {
            return seen
                .iter()
                .copied()
                .take(MAX_AUTO_PLAY_SEEN_SNAPSHOT_KLONDIKE_LUDICROUS)
                .collect();
        }
        seen.clone()
    }

    pub(super) fn automation_profile_with_strategy(&self) -> AutomationProfile {
        let mut profile = self.automation_profile();
        if self.klondike_ludicrous_robot_mode_active() {
            // In ludicrous robot mode for Klondike, favor frame-time stability
            // over deeper lookahead to avoid periodic long main-thread spikes.
            profile.auto_play_lookahead_depth = profile.auto_play_lookahead_depth.min(3);
            profile.auto_play_beam_width = profile.auto_play_beam_width.min(10);
            profile.auto_play_node_budget = profile.auto_play_node_budget.min(1_400);
            return profile;
        }
        profile.auto_play_lookahead_depth =
            profile.auto_play_lookahead_depth.saturating_add(2).min(8);
        profile.auto_play_beam_width = profile.auto_play_beam_width.saturating_add(8).min(40);
        profile.auto_play_node_budget = profile.auto_play_node_budget.saturating_mul(2).min(20_000);
        profile
    }

    pub(super) fn cancel_hint_loss_analysis(&self) {
        let imp = self.imp();
        if let Some(flag) = imp.hint_loss_analysis_cancel.borrow_mut().take() {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        imp.hint_loss_analysis_running.set(false);
    }

    pub(super) fn compute_auto_play_suggestion(&self) -> HintSuggestion {
        if self.active_game_mode() == GameMode::Spider {
            return self.compute_spider_auto_play_suggestion();
        }

        let Some(game) = boundary::clone_klondike_for_automation(
            &self.imp().game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        ) else {
            return HintSuggestion {
                message: "Move engine not available for this game mode yet.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        };
        if game.is_won() {
            return HintSuggestion {
                message: "Game already won.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let state_hash = hash_game_state(&game);
        if let Some(LossVerdict::Lost { explored_states }) =
            self.cached_loss_verdict_for_hash(state_hash)
        {
            return HintSuggestion {
                message: format!(
                    "Solver did not find a winning line from this position (explored {explored_states} states)."
                ),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        self.imp()
            .auto_play_seen_states
            .borrow_mut()
            .insert(state_hash);
        {
            let mut seen = self.imp().auto_play_seen_states.borrow_mut();
            let max_seen_states = if self.klondike_ludicrous_robot_mode_active() {
                MAX_AUTO_PLAY_SEEN_STATES_KLONDIKE_LUDICROUS
            } else {
                MAX_AUTO_PLAY_SEEN_STATES
            };
            if seen.len() > max_seen_states {
                seen.clear();
                seen.insert(state_hash);
            }
        }
        let seen_states = self.auto_play_seen_states_snapshot();

        let candidates = hinting::enumerate_hint_candidates(&game);
        if candidates.is_empty() {
            self.start_hint_loss_analysis_if_needed(state_hash);
            return HintSuggestion {
                message: "No legal moves remain from this position.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let profile = self.automation_profile_with_strategy();

        if let Some(suggestion) = self.best_auto_play_candidate_with_filter(
            &game,
            &seen_states,
            state_hash,
            profile,
            |candidate| candidate.hint_move.is_some(),
        ) {
            suggestion
        } else {
            self.start_hint_loss_analysis_if_needed(state_hash);
            HintSuggestion {
                message: "No productive moves remain from this line.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            }
        }
    }

    pub(super) fn compute_auto_play_suggestion_for_sources(
        &self,
        sources: &[HintNode],
        no_move_message: &str,
    ) -> HintSuggestion {
        if self.active_game_mode() == GameMode::Spider {
            return self.compute_spider_auto_play_suggestion_for_sources(sources, no_move_message);
        }

        let Some(game) = boundary::clone_klondike_for_automation(
            &self.imp().game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        ) else {
            return HintSuggestion {
                message: no_move_message.to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        };
        if game.is_won() {
            return HintSuggestion {
                message: "Game already won.".to_string(),
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

        if sources.is_empty() {
            return HintSuggestion {
                message: no_move_message.to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let profile = self.automation_profile_with_strategy();

        if let Some(suggestion) = self.best_auto_play_candidate_with_filter(
            &game,
            &seen_states,
            state_hash,
            profile,
            |candidate| {
                candidate
                    .source
                    .map(|source| sources.contains(&source))
                    .unwrap_or(false)
            },
        ) {
            suggestion
        } else {
            HintSuggestion {
                message: no_move_message.to_string(),
                source: None,
                target: None,
                hint_move: None,
            }
        }
    }

    fn best_auto_play_candidate_with_filter<F>(
        &self,
        game: &KlondikeGame,
        seen_states: &HashSet<u64>,
        state_hash: u64,
        profile: AutomationProfile,
        mut source_filter: F,
    ) -> Option<HintSuggestion>
    where
        F: FnMut(&HintSuggestion) -> bool,
    {
        let candidates = hinting::enumerate_hint_candidates(game);
        let candidate_slots: Vec<Option<HintMove>> =
            candidates.iter().map(|c| c.hint_move).collect();
        let best_index = autoplay_search::best_candidate_slot_index(
            game,
            &candidate_slots,
            seen_states,
            state_hash,
            profile,
            |index, _hint_move| source_filter(&candidates[index]),
        )?;
        let mut suggestion = candidates[best_index].clone();
        if self.klondike_ludicrous_robot_mode_active() {
            return Some(suggestion);
        }
        if let Some(hint_move) = suggestion.hint_move {
            let rationale =
                self.klondike_move_rationale(game, hint_move, seen_states, state_hash, profile);
            suggestion.message = format!("{} | {}", suggestion.message, rationale);
        }
        Some(suggestion)
    }

    pub(super) fn reset_hint_cycle_memory(&self) {
        let mut recent = self.imp().hint_recent_states.borrow_mut();
        recent.clear();
        drop(recent);
        self.imp().rapid_wand_nonproductive_streak.set(0);
        self.imp().rapid_wand_foundation_drought_streak.set(0);
        *self.imp().rapid_wand_blocked_state_hash.borrow_mut() = None;
        self.note_current_state_for_hint_cycle();
    }

    pub(super) fn note_current_state_for_hint_cycle(&self) {
        let hash = self.current_game_hash();
        let mut recent = self.imp().hint_recent_states.borrow_mut();
        if recent.back().copied() == Some(hash) {
            return;
        }
        let cap = match self.active_game_mode() {
            // Shared FreeCell wand policy needs a long memory window to avoid
            // re-entering medium/long oscillation cycles.
            GameMode::Freecell => 4096,
            GameMode::Spider => 128,
            GameMode::Klondike => 128,
        };
        recent.push_back(hash);
        while recent.len() > cap {
            recent.pop_front();
        }
    }

    pub(super) fn reset_auto_play_memory(&self) {
        let current_hash = self.current_game_hash();
        let mut seen = self.imp().auto_play_seen_states.borrow_mut();
        seen.clear();
        seen.insert(current_hash);
    }

    pub(super) fn note_current_state_for_auto_play(&self) {
        let current_hash = self.current_game_hash();
        let mut seen = self.imp().auto_play_seen_states.borrow_mut();
        seen.insert(current_hash);
        if seen.len() > MAX_AUTO_PLAY_SEEN_STATES {
            seen.clear();
            seen.insert(current_hash);
        }
    }

    pub(super) fn unseen_followup_count(
        &self,
        game: &KlondikeGame,
        seen_states: &HashSet<u64>,
    ) -> i64 {
        autoplay_search::unseen_followup_count(game, seen_states)
    }

    pub(super) fn auto_play_lookahead_score(
        &self,
        current: &KlondikeGame,
        persistent_seen: &HashSet<u64>,
        path_seen: &mut HashSet<u64>,
        depth: u8,
        budget: &mut usize,
    ) -> i64 {
        autoplay_search::auto_play_lookahead_score(
            current,
            persistent_seen,
            path_seen,
            depth,
            budget,
            self.automation_profile(),
        )
    }

    pub(super) fn is_king_to_empty_without_unlock(
        &self,
        current: &KlondikeGame,
        hint_move: HintMove,
    ) -> bool {
        autoplay_search::is_king_to_empty_without_unlock(current, hint_move)
    }

    pub(super) fn is_functionally_useless_auto_move(
        &self,
        current: &KlondikeGame,
        next: &KlondikeGame,
        hint_move: HintMove,
        seen_states: &HashSet<u64>,
    ) -> bool {
        autoplay_search::is_functionally_useless_auto_move(current, next, hint_move, seen_states)
    }

    pub(super) fn current_game_hash(&self) -> u64 {
        let game = self.imp().game.borrow();
        match self.active_game_mode() {
            GameMode::Spider => winnability::spider_solver_state_hash(game.spider()),
            GameMode::Freecell => Self::hash_freecell_game_state(game.freecell()),
            GameMode::Klondike => hash_game_state(&game),
        }
    }

    pub(super) fn hash_freecell_game_state(game: &crate::game::FreecellGame) -> u64 {
        crate::engine::freecell_planner::zobrist_hash(game)
    }

    pub(super) fn cached_loss_verdict_for_hash(&self, state_hash: u64) -> Option<LossVerdict> {
        self.imp()
            .hint_loss_cache
            .borrow()
            .get(&state_hash)
            .copied()
    }

    pub(super) fn start_hint_loss_analysis_if_needed(&self, state_hash: u64) {
        if self.cached_loss_verdict_for_hash(state_hash).is_some() {
            return;
        }

        let imp = self.imp();
        if imp.robot_mode_running.get() {
            return;
        }
        if boundary::is_won(&imp.game.borrow(), self.active_game_mode()) {
            return;
        }
        if imp.hint_loss_analysis_running.get() {
            return;
        }
        imp.hint_loss_analysis_running.set(true);
        imp.hint_loss_analysis_hash.set(state_hash);
        let cancel = Arc::new(AtomicBool::new(false));
        *imp.hint_loss_analysis_cancel.borrow_mut() = Some(Arc::clone(&cancel));

        let game = imp.game.borrow().clone();
        let profile = self.automation_profile();
        let (sender, receiver) = mpsc::channel::<Option<LossVerdict>>();

        thread::spawn(move || {
            let verdict = loss_analysis::analyze_klondike_loss_verdict_cancelable(
                &game,
                profile,
                cancel.as_ref(),
            );
            let _ = sender.send(verdict);
        });

        glib::timeout_add_local(
            Duration::from_millis(40),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || match receiver.try_recv() {
                    Ok(Some(verdict)) => {
                        let imp = window.imp();
                        let analyzed_hash = imp.hint_loss_analysis_hash.get();
                        imp.hint_loss_analysis_running.set(false);
                        imp.hint_loss_analysis_cancel.borrow_mut().take();
                        imp.hint_loss_cache
                            .borrow_mut()
                            .insert(analyzed_hash, verdict);
                        if imp.hint_loss_cache.borrow().len() > MAX_HINT_LOSS_CACHE {
                            let keep = imp.hint_loss_cache.borrow().get(&analyzed_hash).copied();
                            imp.hint_loss_cache.borrow_mut().clear();
                            if let Some(keep) = keep {
                                imp.hint_loss_cache.borrow_mut().insert(analyzed_hash, keep);
                            }
                        }

                        glib::ControlFlow::Break
                    }
                    Ok(None) => {
                        let imp = window.imp();
                        imp.hint_loss_analysis_running.set(false);
                        imp.hint_loss_analysis_cancel.borrow_mut().take();
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        let imp = window.imp();
                        imp.hint_loss_analysis_running.set(false);
                        imp.hint_loss_analysis_cancel.borrow_mut().take();
                        glib::ControlFlow::Break
                    }
                }
            ),
        );
    }

    fn compute_spider_auto_play_suggestion(&self) -> HintSuggestion {
        self.compute_spider_solver_suggestion(None, "No legal moves remain.")
    }

    fn compute_spider_auto_play_suggestion_for_sources(
        &self,
        sources: &[HintNode],
        no_move_message: &str,
    ) -> HintSuggestion {
        self.compute_spider_solver_suggestion(Some(sources), no_move_message)
    }

    fn compute_spider_solver_suggestion(
        &self,
        sources: Option<&[HintNode]>,
        no_move_message: &str,
    ) -> HintSuggestion {
        if sources.is_some_and(|list| list.is_empty()) {
            return HintSuggestion {
                message: no_move_message.to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let game = self.imp().game.borrow().spider().clone();
        if game.is_won() {
            return HintSuggestion {
                message: "Game already won.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let state_hash = winnability::spider_solver_state_hash(&game);
        let mut seen = self.imp().auto_play_seen_states.borrow_mut();
        seen.insert(state_hash);
        if seen.len() > MAX_AUTO_PLAY_SEEN_STATES {
            seen.clear();
            seen.insert(state_hash);
        }
        let seen_states = seen.clone();
        drop(seen);

        let mut recent_hashes = VecDeque::new();
        recent_hashes.push_back(state_hash);
        let selection_salt =
            state_hash ^ (game.stock_len() as u64) ^ (game.completed_runs() as u64);
        let source_filter = |hint_move: HintMove| -> bool {
            let Some(allowed_sources) = sources else {
                return true;
            };
            match hint_move {
                HintMove::Draw => allowed_sources.contains(&HintNode::Stock),
                HintMove::TableauRunToTableau { src, start, .. } => {
                    allowed_sources.contains(&HintNode::Tableau {
                        col: src,
                        index: Some(start),
                    })
                }
                _ => false,
            }
        };
        let decision = winnability::spider_solver_choose_move(
            &game,
            game.suit_mode(),
            None,
            &seen_states,
            &recent_hashes,
            selection_salt,
            winnability::SpiderSolverPolicy::hint_default(),
            source_filter,
        );

        match decision {
            Ok(choice) => match choice.hint_move {
                HintMove::Draw => HintSuggestion {
                    message: "Hint: Deal one card to each tableau column.".to_string(),
                    source: Some(HintNode::Stock),
                    target: Some(HintNode::Stock),
                    hint_move: Some(HintMove::Draw),
                },
                HintMove::TableauRunToTableau { src, start, dst } => {
                    let run_len = game
                        .tableau()
                        .get(src)
                        .map(Vec::len)
                        .unwrap_or(0)
                        .saturating_sub(start);
                    HintSuggestion {
                        message: format!(
                            "Hint: Move {run_len} card(s) T{} -> T{}.",
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
                        hint_move: Some(choice.hint_move),
                    }
                }
                _ => HintSuggestion {
                    message: no_move_message.to_string(),
                    source: None,
                    target: None,
                    hint_move: None,
                },
            },
            Err(_) => HintSuggestion {
                message: no_move_message.to_string(),
                source: None,
                target: None,
                hint_move: None,
            },
        }
    }

    fn klondike_move_rationale(
        &self,
        game: &KlondikeGame,
        hint_move: HintMove,
        seen_states: &HashSet<u64>,
        state_hash: u64,
        profile: crate::engine::automation::AutomationProfile,
    ) -> String {
        let mut next_state = game.clone();
        if !apply_hint_move_to_game(&mut next_state, hint_move) {
            return "rationale unavailable".to_string();
        }

        let next_hash = autoplay::hash_game_state(&next_state);
        let immediate =
            autoplay::score_hint_candidate(game, &next_state, hint_move, seen_states, next_hash);
        let followups = autoplay_search::unseen_followup_count(&next_state, seen_states);

        let mut node_budget = profile.auto_play_node_budget;
        let mut path_seen = HashSet::new();
        path_seen.insert(state_hash);
        path_seen.insert(next_hash);
        let lookahead = autoplay_search::auto_play_lookahead_score(
            &next_state,
            seen_states,
            &mut path_seen,
            profile.auto_play_lookahead_depth.saturating_sub(1),
            &mut node_budget,
            profile,
        ) / 3;

        let king_penalty = if autoplay_search::is_king_to_empty_without_unlock(game, hint_move) {
            -4_000
        } else {
            0
        };
        let win_bonus = if next_state.is_won() {
            profile.auto_play_win_score
        } else {
            0
        };
        let total = immediate + followups * 35 + lookahead + king_penalty + win_bonus;

        let foundation_delta =
            autoplay::foundation_count(&next_state) - autoplay::foundation_count(game);
        let hidden_delta =
            autoplay::hidden_tableau_count(game) - autoplay::hidden_tableau_count(&next_state);
        let mobility_delta =
            autoplay::non_draw_move_count(&next_state) - autoplay::non_draw_move_count(game);
        format!(
            "score={total}, immediate={immediate}, lookahead={lookahead}, followups={followups}, foundation_delta={foundation_delta}, hidden_delta={hidden_delta}, mobility_delta={mobility_delta}, king_penalty={king_penalty}"
        )
    }
}

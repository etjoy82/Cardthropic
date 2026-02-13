use super::*;
use crate::engine::autoplay_search;
use crate::engine::boundary;
use crate::engine::hinting;
use crate::engine::loss_analysis::{self, LossVerdict};

const MAX_AUTO_PLAY_SEEN_STATES: usize = 50_000;
const MAX_HINT_LOSS_CACHE: usize = 4_096;

impl CardthropicWindow {
    pub(super) fn cancel_hint_loss_analysis(&self) {
        let imp = self.imp();
        if let Some(flag) = imp.hint_loss_analysis_cancel.borrow_mut().take() {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        imp.hint_loss_analysis_running.set(false);
    }

    pub(super) fn compute_auto_play_suggestion(&self) -> HintSuggestion {
        let Some(game) = boundary::clone_klondike_for_automation(
            &self.imp().game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        ) else {
            return HintSuggestion {
                message: "Auto-play: not available for this game mode yet.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        };
        if game.is_won() {
            return HintSuggestion {
                message: "Auto-play: game already won.".to_string(),
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
                    "Auto-play: no winning path found from this position (explored {explored_states} states). Game is lost."
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
            if seen.len() > MAX_AUTO_PLAY_SEEN_STATES {
                seen.clear();
                seen.insert(state_hash);
            }
        }
        let seen_states = self.imp().auto_play_seen_states.borrow().clone();

        let candidates = hinting::enumerate_hint_candidates(&game);
        if candidates.is_empty() {
            self.start_hint_loss_analysis_if_needed(state_hash);
            return HintSuggestion {
                message: "Auto-play: no legal moves remain. Game is lost.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        if let Some(suggestion) = self.best_auto_play_candidate_with_filter(
            &game,
            &seen_states,
            state_hash,
            |candidate| candidate.hint_move.is_some(),
        ) {
            suggestion
        } else {
            self.start_hint_loss_analysis_if_needed(state_hash);
            HintSuggestion {
                message: "Auto-play: no productive moves remain from this line. Game is lost."
                    .to_string(),
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
                message: "Auto-play: game already won.".to_string(),
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

        if let Some(suggestion) = self.best_auto_play_candidate_with_filter(
            &game,
            &seen_states,
            state_hash,
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
        mut source_filter: F,
    ) -> Option<HintSuggestion>
    where
        F: FnMut(&HintSuggestion) -> bool,
    {
        let candidates = hinting::enumerate_hint_candidates(game);
        let candidate_slots: Vec<Option<HintMove>> =
            candidates.iter().map(|c| c.hint_move).collect();
        let profile = self.automation_profile();
        let best_index = autoplay_search::best_candidate_slot_index(
            game,
            &candidate_slots,
            seen_states,
            state_hash,
            profile,
            |index, _hint_move| source_filter(&candidates[index]),
        )?;
        Some(candidates[best_index].clone())
    }

    pub(super) fn reset_hint_cycle_memory(&self) {
        let mut recent = self.imp().hint_recent_states.borrow_mut();
        recent.clear();
        drop(recent);
        self.note_current_state_for_hint_cycle();
    }

    pub(super) fn note_current_state_for_hint_cycle(&self) {
        let hash = {
            let game = self.imp().game.borrow();
            hash_game_state(&game)
        };
        let mut recent = self.imp().hint_recent_states.borrow_mut();
        if recent.back().copied() == Some(hash) {
            return;
        }
        recent.push_back(hash);
        while recent.len() > 48 {
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
        hash_game_state(&game)
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

                        let current_hash = window.current_game_hash();
                        if current_hash == analyzed_hash {
                            if let LossVerdict::Lost { explored_states } = verdict {
                                *imp.status_override.borrow_mut() = Some(format!(
                                    "No winning path found from this position (explored {explored_states} states). Game is lost."
                                ));
                                window.render();
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
}

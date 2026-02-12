use super::*;

impl CardthropicWindow {
    pub(super) fn compute_auto_play_suggestion(&self) -> HintSuggestion {
        let mut game = self.imp().game.borrow().clone();
        game.set_draw_mode(self.current_klondike_draw_mode());
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
        let seen_states = self.imp().auto_play_seen_states.borrow().clone();

        let candidates = self.enumerate_hint_candidates(&game);
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
        let mut game = self.imp().game.borrow().clone();
        game.set_draw_mode(self.current_klondike_draw_mode());
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
        let mut best: Option<(i64, HintSuggestion)> = None;
        let mut node_budget = AUTO_PLAY_NODE_BUDGET;
        for candidate in self.enumerate_hint_candidates(game) {
            if !source_filter(&candidate) {
                continue;
            }
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

            if self.is_functionally_useless_auto_move(game, &next_state, hint_move, seen_states) {
                continue;
            }

            let mut score =
                score_hint_candidate(game, &next_state, hint_move, seen_states, next_hash);
            score += self.unseen_followup_count(&next_state, seen_states) * 35;
            if next_state.is_won() {
                score += AUTO_PLAY_WIN_SCORE;
            }

            let mut path_seen = HashSet::new();
            path_seen.insert(state_hash);
            path_seen.insert(next_hash);
            let lookahead = self.auto_play_lookahead_score(
                &next_state,
                seen_states,
                &mut path_seen,
                AUTO_PLAY_LOOKAHEAD_DEPTH.saturating_sub(1),
                &mut node_budget,
            );
            score += lookahead / 3;

            if self.is_king_to_empty_without_unlock(game, hint_move) {
                score -= 4_000;
            }

            match &best {
                None => best = Some((score, candidate)),
                Some((best_score, _)) if score > *best_score => best = Some((score, candidate)),
                _ => {}
            }
        }

        best.map(|(_, suggestion)| suggestion)
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
        self.imp()
            .auto_play_seen_states
            .borrow_mut()
            .insert(current_hash);
    }

    pub(super) fn unseen_followup_count(
        &self,
        game: &KlondikeGame,
        seen_states: &HashSet<u64>,
    ) -> i64 {
        let mut followups: HashSet<u64> = HashSet::new();
        for candidate in self.enumerate_hint_candidates(game) {
            let Some(hint_move) = candidate.hint_move else {
                continue;
            };
            let mut next_state = game.clone();
            if !apply_hint_move_to_game(&mut next_state, hint_move) {
                continue;
            }
            let next_hash = hash_game_state(&next_state);
            if !seen_states.contains(&next_hash) {
                followups.insert(next_hash);
            }
        }
        followups.len() as i64
    }

    pub(super) fn auto_play_lookahead_score(
        &self,
        current: &KlondikeGame,
        persistent_seen: &HashSet<u64>,
        path_seen: &mut HashSet<u64>,
        depth: u8,
        budget: &mut usize,
    ) -> i64 {
        if current.is_won() {
            return AUTO_PLAY_WIN_SCORE;
        }
        if depth == 0 || *budget == 0 {
            return auto_play_state_heuristic(current);
        }

        let mut scored_children: Vec<(i64, KlondikeGame, u64)> = Vec::new();
        for candidate in self.enumerate_hint_candidates(current) {
            let Some(hint_move) = candidate.hint_move else {
                continue;
            };
            let mut next = current.clone();
            if !apply_hint_move_to_game(&mut next, hint_move) {
                continue;
            }
            let next_hash = hash_game_state(&next);
            if persistent_seen.contains(&next_hash) || path_seen.contains(&next_hash) {
                continue;
            }
            if self.is_obviously_useless_auto_move(current, &next, hint_move) {
                continue;
            }

            let immediate =
                score_hint_candidate(current, &next, hint_move, persistent_seen, next_hash);
            scored_children.push((immediate, next, next_hash));
        }

        if scored_children.is_empty() {
            return -90_000 + auto_play_state_heuristic(current);
        }

        scored_children.sort_by_key(|(score, _, _)| Reverse(*score));
        let mut best = i64::MIN / 4;
        for (immediate, next, next_hash) in scored_children.into_iter().take(AUTO_PLAY_BEAM_WIDTH) {
            if *budget == 0 {
                break;
            }
            *budget -= 1;
            path_seen.insert(next_hash);
            let future = self.auto_play_lookahead_score(
                &next,
                persistent_seen,
                path_seen,
                depth - 1,
                budget,
            );
            path_seen.remove(&next_hash);

            let total = immediate + (future / 2);
            if total > best {
                best = total;
            }
        }

        if best == i64::MIN / 4 {
            auto_play_state_heuristic(current)
        } else {
            best
        }
    }

    pub(super) fn is_obviously_useless_auto_move(
        &self,
        current: &KlondikeGame,
        next: &KlondikeGame,
        hint_move: HintMove,
    ) -> bool {
        let foundation_delta = foundation_count(next) - foundation_count(current);
        let hidden_delta = hidden_tableau_count(current) - hidden_tableau_count(next);
        let empty_delta = empty_tableau_count(next) - empty_tableau_count(current);
        let mobility_delta = non_draw_move_count(next) - non_draw_move_count(current);
        if foundation_delta > 0 || hidden_delta > 0 || empty_delta > 0 {
            return false;
        }

        match hint_move {
            HintMove::WasteToFoundation | HintMove::TableauTopToFoundation { .. } => false,
            HintMove::WasteToTableau { .. } => current.can_move_waste_to_foundation(),
            HintMove::TableauRunToTableau { src, start, dst } => {
                let run_len = current.tableau_len(src).unwrap_or(0).saturating_sub(start);
                if run_len == 0 {
                    return true;
                }
                if self.is_king_to_empty_without_unlock(current, hint_move) {
                    return true;
                }
                let reversible =
                    restores_current_by_inverse_tableau_move(current, next, src, dst, run_len);
                reversible && mobility_delta <= 0
            }
            HintMove::Draw => {
                if current.draw_mode().count() > 1 {
                    false
                } else {
                    non_draw_move_count(current) > 0
                }
            }
        }
    }

    pub(super) fn is_king_to_empty_without_unlock(
        &self,
        current: &KlondikeGame,
        hint_move: HintMove,
    ) -> bool {
        let HintMove::TableauRunToTableau { src, start, dst } = hint_move else {
            return false;
        };
        if current.tableau_len(dst) != Some(0) {
            return false;
        }
        let Some(card) = current.tableau_card(src, start) else {
            return false;
        };
        if card.rank != 13 {
            return false;
        }
        let reveals_hidden = start > 0
            && current
                .tableau_card(src, start - 1)
                .map(|below| !below.face_up)
                .unwrap_or(false);
        !reveals_hidden
    }

    pub(super) fn is_functionally_useless_auto_move(
        &self,
        current: &KlondikeGame,
        next: &KlondikeGame,
        hint_move: HintMove,
        seen_states: &HashSet<u64>,
    ) -> bool {
        if next.is_won() {
            return false;
        }

        let foundation_delta = foundation_count(next) - foundation_count(current);
        let hidden_delta = hidden_tableau_count(current) - hidden_tableau_count(next);
        let empty_delta = empty_tableau_count(next) - empty_tableau_count(current);
        if foundation_delta > 0 || hidden_delta > 0 || empty_delta > 0 {
            return false;
        }

        let mobility_delta = non_draw_move_count(next) - non_draw_move_count(current);
        let unseen_followups = self.unseen_followup_count(next, seen_states);

        match hint_move {
            HintMove::WasteToFoundation | HintMove::TableauTopToFoundation { .. } => false,
            HintMove::WasteToTableau { .. } => {
                if current.can_move_waste_to_foundation() {
                    return true;
                }
                mobility_delta <= 0 && unseen_followups <= 0
            }
            HintMove::TableauRunToTableau { src, start, dst } => {
                let run_len = current.tableau_len(src).unwrap_or(0).saturating_sub(start);
                if run_len == 0 {
                    return true;
                }

                if self.is_king_to_empty_without_unlock(current, hint_move) && mobility_delta <= 0 {
                    return true;
                }

                let reversible =
                    restores_current_by_inverse_tableau_move(current, next, src, dst, run_len);

                if reversible && mobility_delta <= 0 {
                    return true;
                }

                run_len == 1 && mobility_delta <= 0 && unseen_followups <= 1
            }
            HintMove::Draw => {
                let drew_playable = next.can_move_waste_to_foundation()
                    || (0..7).any(|dst| next.can_move_waste_to_tableau(dst));
                if drew_playable {
                    return false;
                }
                if current.draw_mode().count() > 1 {
                    return unseen_followups <= 0
                        && hash_game_state(current) == hash_game_state(next);
                }
                let current_non_draw_moves = non_draw_move_count(current);
                current_non_draw_moves > 0 && unseen_followups <= 0
            }
        }
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
        if imp.hint_loss_analysis_running.get() {
            return;
        }
        imp.hint_loss_analysis_running.set(true);
        imp.hint_loss_analysis_hash.set(state_hash);

        let game = imp.game.borrow().clone();
        let (sender, receiver) = mpsc::channel::<LossVerdict>();

        thread::spawn(move || {
            let verdict = if game.is_winnable_guided(HINT_GUIDED_ANALYSIS_BUDGET) {
                LossVerdict::WinnableLikely
            } else {
                let result = game.analyze_winnability(HINT_EXHAUSTIVE_ANALYSIS_BUDGET);
                if result.winnable {
                    LossVerdict::WinnableLikely
                } else if result.hit_state_limit {
                    LossVerdict::Inconclusive {
                        explored_states: result.explored_states,
                    }
                } else {
                    LossVerdict::Lost {
                        explored_states: result.explored_states,
                    }
                }
            };
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
                    Ok(verdict) => {
                        let imp = window.imp();
                        let analyzed_hash = imp.hint_loss_analysis_hash.get();
                        imp.hint_loss_analysis_running.set(false);
                        imp.hint_loss_cache
                            .borrow_mut()
                            .insert(analyzed_hash, verdict);

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
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        window.imp().hint_loss_analysis_running.set(false);
                        glib::ControlFlow::Break
                    }
                }
            ),
        );
    }
}

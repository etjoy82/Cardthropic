use super::*;
use crate::engine::automation::AutomationProfile;
use crate::engine::autoplay;
use crate::engine::autoplay_search;
use crate::engine::boundary;
use crate::engine::hinting;
use crate::engine::loss_analysis::{self, LossVerdict};
use std::hash::{Hash, Hasher};

const MAX_AUTO_PLAY_SEEN_STATES: usize = 50_000;
const MAX_HINT_LOSS_CACHE: usize = 4_096;

impl CardthropicWindow {
    pub(super) fn automation_profile_with_strategy(&self) -> AutomationProfile {
        let mut profile = self.automation_profile();
        match self.robot_strategy() {
            RobotStrategy::Fast => {
                profile.auto_play_lookahead_depth = profile.auto_play_lookahead_depth.saturating_sub(1).max(1);
                profile.auto_play_beam_width = (profile.auto_play_beam_width / 2).max(4);
                profile.auto_play_node_budget = (profile.auto_play_node_budget / 2).max(800);
            }
            RobotStrategy::Balanced => {}
            RobotStrategy::Deep => {
                profile.auto_play_lookahead_depth = profile.auto_play_lookahead_depth.saturating_add(2).min(8);
                profile.auto_play_beam_width = profile.auto_play_beam_width.saturating_add(8).min(40);
                profile.auto_play_node_budget = profile.auto_play_node_budget.saturating_mul(2).min(20_000);
            }
        }
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
                    "No winning path found from this position (explored {explored_states} states). Game is lost."
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
                message: "No legal moves remain. Game is lost.".to_string(),
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
                message: "No productive moves remain from this line. Game is lost.".to_string(),
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
        match self.active_game_mode() {
            GameMode::Spider => Self::hash_spider_game_state(game.spider()),
            _ => hash_game_state(&game),
        }
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

    fn compute_spider_auto_play_suggestion(&self) -> HintSuggestion {
        let game = self.imp().game.borrow().spider().clone();
        if game.is_won() {
            return HintSuggestion {
                message: "Game already won.".to_string(),
                source: None,
                target: None,
                hint_move: None,
            };
        }

        let state_hash = Self::hash_spider_game_state(&game);
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

        if let Some(best) = self.best_spider_tableau_move(&game, None, &seen_states) {
            return best;
        }
        if let Some(unblock) = self.find_spider_deal_unblock_move(&game, None) {
            return unblock;
        }

        if game.can_deal_from_stock() {
            return HintSuggestion {
                message: "Hint: Deal one card to each tableau column.".to_string(),
                source: Some(HintNode::Stock),
                target: Some(HintNode::Stock),
                hint_move: Some(HintMove::Draw),
            };
        }

        HintSuggestion {
            message: "No legal moves remain.".to_string(),
            source: None,
            target: None,
            hint_move: None,
        }
    }

    fn compute_spider_auto_play_suggestion_for_sources(
        &self,
        sources: &[HintNode],
        no_move_message: &str,
    ) -> HintSuggestion {
        if sources.is_empty() {
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

        let seen_states = self.imp().auto_play_seen_states.borrow().clone();
        if let Some(best) = self.best_spider_tableau_move(&game, Some(sources), &seen_states) {
            return best;
        }
        if let Some(unblock) = self.find_spider_deal_unblock_move(&game, Some(sources)) {
            return unblock;
        }

        if sources.contains(&HintNode::Stock) && game.can_deal_from_stock() {
            return HintSuggestion {
                message: "Hint: Deal one card to each tableau column.".to_string(),
                source: Some(HintNode::Stock),
                target: Some(HintNode::Stock),
                hint_move: Some(HintMove::Draw),
            };
        }

        HintSuggestion {
            message: no_move_message.to_string(),
            source: None,
            target: None,
            hint_move: None,
        }
    }

    fn best_spider_tableau_move(
        &self,
        game: &crate::game::SpiderGame,
        sources: Option<&[HintNode]>,
        seen_states: &HashSet<u64>,
    ) -> Option<HintSuggestion> {
        let mut best: Option<(i64, HintSuggestion)> = None;
        let current_legal_moves = Self::count_spider_tableau_moves(game);
        let current_empty_cols = game.tableau().iter().filter(|pile| pile.is_empty()).count();
        let current_can_deal = game.can_deal_from_stock();
        for src in 0..10 {
            let Some(source_len) = game.tableau().get(src).map(Vec::len) else {
                continue;
            };
            for start in 0..source_len {
                let Some(card) = game.tableau_card(src, start) else {
                    continue;
                };
                if !card.face_up {
                    continue;
                }
                if let Some(allowed_sources) = sources {
                    let source_hint = HintNode::Tableau {
                        col: src,
                        index: Some(start),
                    };
                    if !allowed_sources.contains(&source_hint) {
                        continue;
                    }
                }

                for dst in 0..10 {
                    if !game.can_move_run(src, start, dst) {
                        continue;
                    }
                    let dst_len_before = game.tableau().get(dst).map(Vec::len).unwrap_or(0);
                    let mut next_state = game.clone();
                    if !next_state.move_run(src, start, dst) {
                        continue;
                    }
                    let next_hash = Self::hash_spider_game_state(&next_state);
                    if seen_states.contains(&next_hash) {
                        continue;
                    }

                    let run_len = source_len.saturating_sub(start);
                    let completed_delta = next_state
                        .completed_runs()
                        .saturating_sub(game.completed_runs());
                    let moving_between_same_parents = game.suit_mode()
                        == crate::game::SpiderSuitMode::One
                        && start > 0
                        && game
                            .tableau_card(src, start - 1)
                            .is_some_and(|parent| parent.face_up && parent.rank == card.rank + 1)
                        && game
                            .tableau()
                            .get(dst)
                            .and_then(|pile| pile.last().copied())
                            .is_some_and(|parent| parent.face_up && parent.rank == card.rank + 1);
                    if moving_between_same_parents && completed_delta == 0 {
                        continue;
                    }

                    let reveals_face_down = start > 0
                        && game
                            .tableau_card(src, start - 1)
                            .is_some_and(|prev| !prev.face_up);
                    let next_can_deal = next_state.can_deal_from_stock();
                    let deal_unlocked = !current_can_deal && next_can_deal;
                    let next_empty_cols = next_state
                        .tableau()
                        .iter()
                        .filter(|pile| pile.is_empty())
                        .count();
                    let empties_delta = i64::try_from(current_empty_cols).unwrap_or(0)
                        - i64::try_from(next_empty_cols).unwrap_or(0);
                    let next_legal_moves = Self::count_spider_tableau_moves(&next_state);
                    let mobility_delta = i64::try_from(next_legal_moves).unwrap_or(0)
                        - i64::try_from(current_legal_moves).unwrap_or(0);
                    let reverse_possible = next_state.can_move_run(dst, dst_len_before, src);
                    let is_low_value_reversible = reverse_possible
                        && completed_delta == 0
                        && !reveals_face_down
                        && !deal_unlocked
                        && mobility_delta <= 0;
                    if is_low_value_reversible {
                        continue;
                    }

                    let dst_is_empty = game.tableau().get(dst).is_some_and(Vec::is_empty);
                    let score = i64::from(reveals_face_down) * 900
                        + i64::try_from(completed_delta).unwrap_or(0) * 2_000
                        + i64::from(deal_unlocked) * 1_200
                        + empties_delta * 250
                        + mobility_delta * 60
                        + i64::try_from(run_len).unwrap_or(0) * 15
                        + if dst_is_empty { -80 } else { 30 };

                    let suggestion = HintSuggestion {
                        message: format!(
                            "Hint: Move {run_len} card(s) T{} -> T{} | score={score}, complete+{}, reveal={}, deal_unlock={}, mobility_delta={}, empties_delta={}, reversible={}",
                            src + 1,
                            dst + 1,
                            completed_delta,
                            reveals_face_down,
                            deal_unlocked,
                            mobility_delta,
                            empties_delta,
                            reverse_possible
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
                    };

                    match &best {
                        None => best = Some((score, suggestion)),
                        Some((best_score, _)) if score > *best_score => {
                            best = Some((score, suggestion))
                        }
                        _ => {}
                    }
                }
            }
        }

        best.map(|(_, suggestion)| suggestion)
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

    fn count_spider_tableau_moves(game: &crate::game::SpiderGame) -> usize {
        let mut count = 0usize;
        for src in 0..10 {
            let Some(len) = game.tableau().get(src).map(Vec::len) else {
                continue;
            };
            for start in 0..len {
                if !game
                    .tableau_card(src, start)
                    .is_some_and(|card| card.face_up)
                {
                    continue;
                }
                for dst in 0..10 {
                    if game.can_move_run(src, start, dst) {
                        count = count.saturating_add(1);
                    }
                }
            }
        }
        count
    }

    fn hash_spider_game_state(game: &crate::game::SpiderGame) -> u64 {
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        game.hash(&mut hasher);
        hasher.finish()
    }

    fn find_spider_deal_unblock_move(
        &self,
        game: &crate::game::SpiderGame,
        sources: Option<&[HintNode]>,
    ) -> Option<HintSuggestion> {
        if game.can_deal_from_stock() || game.stock_len() == 0 {
            return None;
        }
        let has_empty = game.tableau().iter().any(Vec::is_empty);
        if !has_empty {
            return None;
        }

        let empty_cols: Vec<usize> = game
            .tableau()
            .iter()
            .enumerate()
            .filter_map(|(idx, pile)| if pile.is_empty() { Some(idx) } else { None })
            .collect();
        if empty_cols.is_empty() {
            return None;
        }

        let mut best: Option<(i64, usize, usize, usize)> = None;
        for src in 0..10 {
            let Some(source_len) = game.tableau().get(src).map(Vec::len) else {
                continue;
            };
            for start in 0..source_len {
                let Some(card) = game.tableau_card(src, start) else {
                    continue;
                };
                if !card.face_up {
                    continue;
                }
                if let Some(allowed_sources) = sources {
                    let source_hint = HintNode::Tableau {
                        col: src,
                        index: Some(start),
                    };
                    if !allowed_sources.contains(&source_hint) {
                        continue;
                    }
                }

                for &dst in &empty_cols {
                    if !game.can_move_run(src, start, dst) {
                        continue;
                    }
                    let run_len = source_len.saturating_sub(start);
                    let reveals_face_down = start > 0
                        && game
                            .tableau_card(src, start - 1)
                            .is_some_and(|prev| !prev.face_up);
                    let score = i64::from(reveals_face_down) * 5_000
                        + i64::try_from(run_len).unwrap_or(0) * 100;
                    match best {
                        None => best = Some((score, src, start, dst)),
                        Some((best_score, ..)) if score > best_score => {
                            best = Some((score, src, start, dst))
                        }
                        _ => {}
                    }
                }
            }
        }

        let (_, src, start, dst) = best?;
        let run_len = game
            .tableau()
            .get(src)
            .map(Vec::len)
            .unwrap_or(0)
            .saturating_sub(start);
        Some(HintSuggestion {
            message: format!(
                "Hint: Move {run_len} card(s) T{} -> T{} to unblock stock dealing.",
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
        })
    }
}

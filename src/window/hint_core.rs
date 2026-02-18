use super::*;
use crate::engine::boundary;
use crate::engine::freecell_planner::FreecellPlannerAction;
use crate::game::{Card, FreecellGame};
use crate::winnability::{
    freecell_wand_best_action, freecell_wand_best_action_avoiding_seen, freecell_wand_state_hash,
};
use std::cmp::Ordering;
use std::collections::HashMap;
use std::collections::HashSet;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FreecellHintAction {
    TableauToFoundation {
        src: usize,
    },
    FreecellToFoundation {
        cell: usize,
    },
    TableauRunToTableau {
        src: usize,
        start: usize,
        dst: usize,
    },
    TableauToFreecell {
        src: usize,
        cell: usize,
    },
    FreecellToTableau {
        cell: usize,
        dst: usize,
    },
}

#[derive(Clone)]
struct FreecellSearchCandidate {
    message: String,
    source: HintNode,
    target: HintNode,
    action: FreecellHintAction,
    next: FreecellGame,
    immediate_score: i64,
}

impl CardthropicWindow {
    const WAND_PLANNER_WAIT_TICKS: u32 = 6;
    const WAND_ACTION_CYCLE_LIMIT: usize = 5;
    const WAND_ACTION_CYCLE_MAX_PERIOD: usize = 6;
    const WAND_ACTION_SIGNATURE_WINDOW: usize = 96;
    const WAND_FALLBACK_RECENT_SIGNATURE_BLOCK_WINDOW: usize = 24;
    const WAND_BLOCKED_STATE_RESEED_THRESHOLD: u32 = 4;

    fn planner_action_to_hint_action(action: FreecellPlannerAction) -> FreecellHintAction {
        match action {
            FreecellPlannerAction::TableauToFoundation { src } => {
                FreecellHintAction::TableauToFoundation { src }
            }
            FreecellPlannerAction::FreecellToFoundation { cell } => {
                FreecellHintAction::FreecellToFoundation { cell }
            }
            FreecellPlannerAction::TableauRunToTableau { src, start, dst } => {
                FreecellHintAction::TableauRunToTableau { src, start, dst }
            }
            FreecellPlannerAction::TableauToFreecell { src, cell } => {
                FreecellHintAction::TableauToFreecell { src, cell }
            }
            FreecellPlannerAction::FreecellToTableau { cell, dst } => {
                FreecellHintAction::FreecellToTableau { cell, dst }
            }
        }
    }

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
        if self.active_game_mode() == GameMode::Freecell {
            let (message, source, target, _action, _score) =
                self.compute_freecell_hint_action(false);
            *self.imp().status_override.borrow_mut() = Some(message);
            self.render();
            if let (Some(source), Some(target)) = (source, target) {
                self.play_hint_animation(source, target);
            }
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
        if self.active_game_mode() == GameMode::Freecell {
            return self.play_freecell_hint_for_player();
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

    fn play_freecell_hint_for_player(&self) -> bool {
        self.clear_hint_effects();
        self.note_current_state_for_hint_cycle();
        self.note_current_state_for_auto_play();
        let (message, source, target, action, _score) = self.compute_freecell_hint_action(true);
        let wand_message = Self::wand_status_message(&message);
        if let (Some(source), Some(target)) = (source, target) {
            self.play_hint_animation(source, target);
        }
        let Some(action) = action else {
            if self.imp().rapid_wand_running.get()
                && self.imp().robot_freecell_planner_running.get()
            {
                *self.imp().status_override.borrow_mut() =
                    Some("Wand Wave: planner thinking...".to_string());
                self.render();
                return true;
            }
            if self.imp().rapid_wand_running.get() {
                let hash = self.current_game_hash();
                let repeated = self
                    .imp()
                    .rapid_wand_blocked_state_hash
                    .borrow()
                    .is_some_and(|prev| prev == hash);
                if repeated {
                    let streak = self
                        .imp()
                        .rapid_wand_nonproductive_streak
                        .get()
                        .saturating_add(1);
                    self.imp().rapid_wand_nonproductive_streak.set(streak);
                    if streak >= Self::WAND_BLOCKED_STATE_RESEED_THRESHOLD {
                        *self.imp().status_override.borrow_mut() = Some(
                            "Wand Wave: repeated blocked state; starting a new game.".to_string(),
                        );
                        self.render();
                        self.start_random_seed_game();
                        self.imp().rapid_wand_nonproductive_streak.set(0);
                        *self.imp().rapid_wand_blocked_state_hash.borrow_mut() = None;
                        return false;
                    }
                } else {
                    self.imp().rapid_wand_nonproductive_streak.set(1);
                    *self.imp().rapid_wand_blocked_state_hash.borrow_mut() = Some(hash);
                }
            }
            *self.imp().status_override.borrow_mut() = Some(format!("Wand Wave: {wand_message}"));
            self.render();
            return false;
        };
        self.imp().auto_playing_move.set(true);
        let changed = self.apply_freecell_hint_action(action);
        self.imp().auto_playing_move.set(false);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            self.imp().selected_freecell.set(None);
            self.imp().rapid_wand_nonproductive_streak.set(0);
            *self.imp().rapid_wand_blocked_state_hash.borrow_mut() = None;
            {
                let mut recent = self.imp().robot_recent_action_signatures.borrow_mut();
                recent.push_back(Self::freecell_action_cycle_signature(action));
                while recent.len() > Self::WAND_ACTION_SIGNATURE_WINDOW {
                    recent.pop_front();
                }
            }
            self.note_current_state_for_hint_cycle();
            self.note_current_state_for_auto_play();
            *self.imp().status_override.borrow_mut() = Some(format!("Wand Wave: {wand_message}"));
        } else {
            let detailed = self
                .imp()
                .status_override
                .borrow()
                .as_deref()
                .map(str::to_string);
            *self.imp().status_override.borrow_mut() = Some(if let Some(reason) = detailed {
                format!("Wand Wave: {reason}")
            } else {
                "Wand Wave: move was not legal anymore.".to_string()
            });
        }
        self.render();
        changed
    }

    fn is_freecell_foundation_action(action: FreecellHintAction) -> bool {
        matches!(
            action,
            FreecellHintAction::TableauToFoundation { .. }
                | FreecellHintAction::FreecellToFoundation { .. }
        )
    }

    fn freecell_mobility_count_for_wand(game: &FreecellGame) -> u32 {
        let mut count = 0_u32;
        for cell in 0..4 {
            if game.can_move_freecell_to_foundation(cell) {
                count = count.saturating_add(1);
            }
            for dst in 0..8 {
                if game.can_move_freecell_to_tableau(cell, dst) {
                    count = count.saturating_add(1);
                }
            }
        }
        for src in 0..8 {
            if game.can_move_tableau_top_to_foundation(src) {
                count = count.saturating_add(1);
            }
            for cell in 0..4 {
                if game.can_move_tableau_top_to_freecell(src, cell) {
                    count = count.saturating_add(1);
                }
            }
            let len = game.tableau().get(src).map(Vec::len).unwrap_or(0);
            for start in 0..len {
                for dst in 0..8 {
                    if game.can_move_tableau_run_to_tableau(src, start, dst) {
                        count = count.saturating_add(1);
                    }
                }
            }
        }
        count
    }

    fn freecell_supermove_capacity_for_wand(game: &FreecellGame) -> u32 {
        let ef = game
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let et = game.tableau().iter().filter(|col| col.is_empty()).count();
        ((ef + 1) * (1usize << et)) as u32
    }

    fn wand_has_strong_progress_for_action(
        &self,
        game: &FreecellGame,
        action: FreecellHintAction,
    ) -> bool {
        if Self::is_freecell_foundation_action(action) {
            return true;
        }
        let mut next = game.clone();
        let changed = match action {
            FreecellHintAction::TableauToFoundation { src } => {
                next.move_tableau_top_to_foundation(src)
            }
            FreecellHintAction::FreecellToFoundation { cell } => {
                next.move_freecell_to_foundation(cell)
            }
            FreecellHintAction::TableauRunToTableau { src, start, dst } => {
                next.move_tableau_run_to_tableau(src, start, dst)
            }
            FreecellHintAction::TableauToFreecell { src, cell } => {
                next.move_tableau_top_to_freecell(src, cell)
            }
            FreecellHintAction::FreecellToTableau { cell, dst } => {
                next.move_freecell_to_tableau(cell, dst)
            }
        };
        if !changed {
            return false;
        }

        let foundation_before = game.foundations().iter().map(Vec::len).sum::<usize>() as u32;
        let foundation_after = next.foundations().iter().map(Vec::len).sum::<usize>() as u32;
        if foundation_after > foundation_before {
            return true;
        }

        let empty_before = game.tableau().iter().filter(|col| col.is_empty()).count() as u32;
        let empty_after = next.tableau().iter().filter(|col| col.is_empty()).count() as u32;
        if empty_after > empty_before {
            return true;
        }

        let mobility_before = Self::freecell_mobility_count_for_wand(game);
        let mobility_after = Self::freecell_mobility_count_for_wand(&next);
        if mobility_after > mobility_before {
            return true;
        }

        let capacity_before = Self::freecell_supermove_capacity_for_wand(game);
        let capacity_after = Self::freecell_supermove_capacity_for_wand(&next);
        capacity_after > capacity_before
    }

    fn wand_rejects_cyclic_fallback_action(
        &self,
        game: &FreecellGame,
        action: FreecellHintAction,
    ) -> bool {
        if Self::is_freecell_foundation_action(action) {
            return false;
        }
        let Some(next_hash) = Self::freecell_next_hash_for_action(game, action) else {
            return true;
        };
        let recent_hit = self
            .imp()
            .hint_recent_states
            .borrow()
            .iter()
            .any(|h| *h == next_hash);
        let seen_hit = self
            .imp()
            .auto_play_seen_states
            .borrow()
            .contains(&next_hash);
        recent_hit || seen_hit
    }

    fn parse_search_expanded(message: &str) -> Option<usize> {
        let marker = "expanded=";
        let start = message.find(marker)? + marker.len();
        let digits: String = message[start..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.is_empty() {
            return None;
        }
        digits.parse::<usize>().ok()
    }

    fn wand_rejects_action_cycle(&self, action: FreecellHintAction) -> bool {
        if Self::is_freecell_foundation_action(action) {
            return false;
        }
        let mut recent: Vec<String> = self
            .imp()
            .robot_recent_action_signatures
            .borrow()
            .iter()
            .cloned()
            .collect();
        recent.push(Self::freecell_action_cycle_signature(action));
        if recent.len() < 2 {
            return false;
        }

        let len = recent.len();
        let mut best_repeats = 0_usize;
        for period in 1..=Self::WAND_ACTION_CYCLE_MAX_PERIOD {
            if len < period * 2 {
                continue;
            }
            let mut repeats = 0_usize;
            let mut cursor = len;
            while cursor >= period * 2
                && recent[cursor - period..cursor] == recent[cursor - period * 2..cursor - period]
            {
                repeats = repeats.saturating_add(1);
                cursor -= period;
            }
            if repeats > best_repeats {
                best_repeats = repeats;
            }
        }
        best_repeats > Self::WAND_ACTION_CYCLE_LIMIT
    }

    fn wand_rejects_recent_repeat_signature(&self, action: FreecellHintAction) -> bool {
        if Self::is_freecell_foundation_action(action) {
            return false;
        }
        let sig = Self::freecell_action_cycle_signature(action);
        self.imp()
            .robot_recent_action_signatures
            .borrow()
            .iter()
            .rev()
            .take(Self::WAND_FALLBACK_RECENT_SIGNATURE_BLOCK_WINDOW)
            .any(|s| s == &sig)
    }

    pub(super) fn apply_freecell_hint_action(&self, action: FreecellHintAction) -> bool {
        match action {
            FreecellHintAction::TableauToFoundation { src } => self.move_tableau_to_foundation(src),
            FreecellHintAction::FreecellToFoundation { cell } => {
                self.move_freecell_to_foundation(cell)
            }
            FreecellHintAction::TableauRunToTableau { src, start, dst } => {
                self.move_tableau_run_to_tableau(src, start, dst)
            }
            FreecellHintAction::TableauToFreecell { src, cell } => {
                self.move_tableau_to_freecell(src, cell)
            }
            FreecellHintAction::FreecellToTableau { cell, dst } => {
                self.move_freecell_to_tableau(cell, dst)
            }
        }
    }

    pub(super) fn freecell_progress_analysis_for_action(
        &self,
        action: FreecellHintAction,
    ) -> Option<(i64, bool, String)> {
        let current = self.imp().game.borrow().freecell().clone();
        let mut next = current.clone();
        let changed = match action {
            FreecellHintAction::TableauToFoundation { src } => {
                next.move_tableau_top_to_foundation(src)
            }
            FreecellHintAction::FreecellToFoundation { cell } => {
                next.move_freecell_to_foundation(cell)
            }
            FreecellHintAction::TableauRunToTableau { src, start, dst } => {
                next.move_tableau_run_to_tableau(src, start, dst)
            }
            FreecellHintAction::TableauToFreecell { src, cell } => {
                next.move_tableau_top_to_freecell(src, cell)
            }
            FreecellHintAction::FreecellToTableau { cell, dst } => {
                next.move_freecell_to_tableau(cell, dst)
            }
        };
        if !changed {
            return None;
        }
        let current_hash = Self::freecell_game_hash(&current);
        let next_hash = Self::freecell_game_hash(&next);
        let recent_hashes: Vec<u64> = self
            .imp()
            .hint_recent_states
            .borrow()
            .iter()
            .copied()
            .collect();
        Some(Self::freecell_progress_analysis(
            &current,
            &next,
            current_hash,
            next_hash,
            &recent_hashes,
        ))
    }

    fn freecell_progress_analysis(
        current: &FreecellGame,
        next: &FreecellGame,
        current_hash: u64,
        next_hash: u64,
        recent_hashes: &[u64],
    ) -> (i64, bool, String) {
        let foundation_delta = Self::freecell_foundation_cards(next) as i64
            - Self::freecell_foundation_cards(current) as i64;
        let mobility_delta = Self::freecell_legal_move_count(next) as i64
            - Self::freecell_legal_move_count(current) as i64;
        let eval_delta = Self::freecell_state_eval(next) - Self::freecell_state_eval(current);
        let empty_cols_delta = next.tableau().iter().filter(|col| col.is_empty()).count() as i64
            - current
                .tableau()
                .iter()
                .filter(|col| col.is_empty())
                .count() as i64;
        let capacity_delta = Self::freecell_supermove_capacity(next) as i64
            - Self::freecell_supermove_capacity(current) as i64;
        let exposed_tops_delta = Self::freecell_exposed_top_delta(current, next);
        let foundation_push_delta = Self::freecell_foundation_push_count(next) as i64
            - Self::freecell_foundation_push_count(current) as i64;
        let tableau_order_delta =
            Self::freecell_tableau_order_score(next) - Self::freecell_tableau_order_score(current);
        let (loop_penalty, repeat_distance) =
            Self::freecell_recent_loop_details(next_hash, current_hash, recent_hashes);

        if next_hash == current_hash {
            return (-120_000, false, "state hash unchanged".to_string());
        }
        if repeat_distance.is_some_and(|d| d <= 2) || loop_penalty >= 40_000 {
            return (
                -30_000 - loop_penalty,
                false,
                format!(
                    "loop risk too high (penalty={}, repeat_distance={})",
                    loop_penalty,
                    repeat_distance
                        .map(|d| d.to_string())
                        .unwrap_or_else(|| "na".to_string())
                ),
            );
        }

        let novel_state = repeat_distance.is_none();
        let mut progressed = false;
        let reason: String;
        if foundation_delta > 0 {
            progressed = true;
            reason = format!("foundation_delta={}", foundation_delta);
        } else if foundation_push_delta > 0 {
            progressed = true;
            reason = format!("foundation_push_delta={}", foundation_push_delta);
        } else if empty_cols_delta > 0 {
            progressed = true;
            reason = format!("empty_cols_delta={}", empty_cols_delta);
        } else if tableau_order_delta > 0 && eval_delta >= 0 {
            progressed = true;
            reason = format!("tableau_order_delta={}", tableau_order_delta);
        } else if mobility_delta >= 2 && eval_delta > 0 {
            progressed = true;
            reason = format!(
                "mobility_delta={} eval_delta={}",
                mobility_delta, eval_delta
            );
        } else if exposed_tops_delta > 0 {
            progressed = true;
            reason = format!("exposed_tops_delta={}", exposed_tops_delta);
        } else if capacity_delta > 0 {
            progressed = true;
            reason = format!("supermove_capacity_delta={}", capacity_delta);
        } else if empty_cols_delta < 0
            && eval_delta >= 150
            && (foundation_push_delta > 0 || capacity_delta > 0 || tableau_order_delta > 0)
        {
            // Spending an empty tableau column as a temporary staging slot,
            // but only if it clearly improves downstream structure.
            progressed = true;
            reason = "main_area_cell_staging".to_string();
        } else {
            reason = format!(
                "no_progress foundation_delta={} mobility_delta={} foundation_push_delta={} exposed_tops_delta={} empty_cols_delta={} capacity_delta={} tableau_order_delta={} eval_delta={}",
                foundation_delta,
                mobility_delta,
                foundation_push_delta,
                exposed_tops_delta,
                empty_cols_delta,
                capacity_delta,
                tableau_order_delta,
                eval_delta
            );
        }

        let novelty_bonus = if novel_state { 2_200 } else { 200 };
        let score = foundation_delta * 60_000
            + mobility_delta * 260
            + foundation_push_delta * 700
            + exposed_tops_delta * 550
            + empty_cols_delta * 300
            + capacity_delta * 140
            + tableau_order_delta * 340
            + eval_delta
            + novelty_bonus
            - (loop_penalty / 7);
        (score, progressed, reason)
    }

    pub(super) fn freecell_action_tag(action: FreecellHintAction) -> &'static str {
        match action {
            FreecellHintAction::TableauToFoundation { .. } => "t2f",
            FreecellHintAction::FreecellToFoundation { .. } => "c2f",
            FreecellHintAction::TableauRunToTableau { .. } => "t2t",
            FreecellHintAction::TableauToFreecell { .. } => "t2c",
            FreecellHintAction::FreecellToTableau { .. } => "c2t",
        }
    }

    fn compute_freecell_hint_action(
        &self,
        consume_planner_step: bool,
    ) -> (
        String,
        Option<HintNode>,
        Option<HintNode>,
        Option<FreecellHintAction>,
        Option<i64>,
    ) {
        if let Some((message, source, target, action, score, solver_source)) =
            self.compute_freecell_hint_with_robot_logic(consume_planner_step)
        {
            let message = if self.imp().robot_debug_enabled.get() {
                format!(
                    "{message} | fc_score={} fc_action={} solver_source={}",
                    score,
                    Self::freecell_action_tag(action),
                    solver_source
                )
            } else {
                message
            };
            (
                message,
                Some(source),
                Some(target),
                Some(action),
                Some(score),
            )
        } else {
            (
                if self.imp().game.borrow().freecell().is_lost() {
                    "Hint: no legal moves. FreeCell game is lost.".to_string()
                } else {
                    "Hint: no legal moves. FreeCell line is blocked.".to_string()
                },
                None,
                None,
                None,
                None,
            )
        }
    }

    fn compute_freecell_hint_with_robot_logic(
        &self,
        consume_planner_step: bool,
    ) -> Option<(
        String,
        HintNode,
        HintNode,
        FreecellHintAction,
        i64,
        &'static str,
    )> {
        let game = self.imp().game.borrow().freecell().clone();
        let mut planner_failed_for_state = false;
        if let Some(result) = self.collect_freecell_planner_result() {
            let expected_anchor = self
                .projected_freecell_planner_state()
                .map(|(_, hash)| hash)
                .unwrap_or(0);
            if self.imp().robot_freecell_planner_anchor_hash.get() == expected_anchor {
                if result.stalled || result.actions.is_empty() {
                    planner_failed_for_state = self.imp().robot_freecell_plan.borrow().is_empty();
                } else {
                    let (_, high_watermark) = self.freecell_planner_queue_bounds();
                    let mut plan = self.imp().robot_freecell_plan.borrow_mut();
                    let available = high_watermark.saturating_sub(plan.len());
                    for action in result.actions.into_iter().take(available) {
                        plan.push_back(action);
                    }
                }
            }
        }

        if self.imp().robot_freecell_plan.borrow().is_empty() {
            self.start_freecell_planner_if_needed();
        }

        let planned_action = if consume_planner_step {
            self.imp().robot_freecell_plan.borrow_mut().pop_front()
        } else {
            self.imp().robot_freecell_plan.borrow().front().copied()
        };
        if let Some(planned_action) = planned_action {
            self.imp().robot_freecell_planner_wait_ticks.set(0);
            let action = Self::planner_action_to_hint_action(planned_action);
            let (source, target) = Self::freecell_action_nodes(&game, action);
            let remaining = if consume_planner_step {
                self.imp().robot_freecell_plan.borrow().len()
            } else {
                self.imp()
                    .robot_freecell_plan
                    .borrow()
                    .len()
                    .saturating_sub(1)
            };
            return Some((
                format!("planner step (remaining={remaining})"),
                source,
                target,
                action,
                0,
                "planner",
            ));
        }

        if self.imp().rapid_wand_running.get()
            && self.imp().robot_freecell_planner_running.get()
            && !planner_failed_for_state
        {
            return None;
        }

        if self.imp().robot_freecell_planner_running.get() {
            let wait_ticks = self
                .imp()
                .robot_freecell_planner_wait_ticks
                .get()
                .saturating_add(1);
            self.imp().robot_freecell_planner_wait_ticks.set(wait_ticks);
            if self.imp().rapid_wand_running.get()
                && wait_ticks <= Self::WAND_PLANNER_WAIT_TICKS
                && !planner_failed_for_state
            {
                return None;
            }
        }

        if let Some((message, source, target, action, score)) =
            self.compute_best_freecell_action(None)
        {
            let (progress_score, progressed, _) = self
                .freecell_progress_analysis_for_action(action)
                .unwrap_or((i64::MIN / 4, false, String::new()));
            let strong_progress = self.wand_has_strong_progress_for_action(&game, action);
            let expanded = Self::parse_search_expanded(&message).unwrap_or(1);
            let expanded_ok = Self::is_freecell_foundation_action(action) || expanded > 0;
            if (Self::is_freecell_foundation_action(action)
                || ((progressed && progress_score > 0) && strong_progress))
                && expanded_ok
                && !self.wand_rejects_cyclic_fallback_action(&game, action)
                && !self.wand_rejects_action_cycle(action)
                && !self.wand_rejects_recent_repeat_signature(action)
            {
                return Some((message, source, target, action, score, "fallback-search"));
            }
        }

        self.compute_unified_freecell_wand_action().and_then(
            |(message, source, target, action, score)| {
                let (progress_score, progressed, _) = self
                    .freecell_progress_analysis_for_action(action)
                    .unwrap_or((i64::MIN / 4, false, String::new()));
                let strong_progress = self.wand_has_strong_progress_for_action(&game, action);
                if Self::is_freecell_foundation_action(action)
                    || ((progressed && progress_score > 0) && strong_progress)
                {
                    if self.wand_rejects_cyclic_fallback_action(&game, action)
                        || self.wand_rejects_action_cycle(action)
                        || self.wand_rejects_recent_repeat_signature(action)
                    {
                        None
                    } else {
                        Some((message, source, target, action, score, "fallback"))
                    }
                } else {
                    None
                }
            },
        )
    }

    pub(super) fn compute_unified_freecell_wand_action(
        &self,
    ) -> Option<(String, HintNode, HintNode, FreecellHintAction, i64)> {
        let game = self.imp().game.borrow().freecell().clone();
        let current_hash = Self::freecell_game_hash(&game);
        let recent_hashes: Vec<u64> = self
            .imp()
            .hint_recent_states
            .borrow()
            .iter()
            .copied()
            .collect();
        let seen_hashes = self.imp().auto_play_seen_states.borrow().clone();
        let candidates = self.generate_freecell_candidates(&game, None, false);
        if candidates.is_empty() {
            return None;
        }

        let planner_action = if seen_hashes.is_empty() {
            freecell_wand_best_action(&game, current_hash, &recent_hashes)?
        } else {
            freecell_wand_best_action_avoiding_seen(
                &game,
                current_hash,
                &recent_hashes,
                &seen_hashes,
            )?
        };
        let selected_action = Self::planner_action_to_hint_action(planner_action);
        let chosen = candidates
            .into_iter()
            .find(|candidate| candidate.action == selected_action)?;
        let next_hash = freecell_wand_state_hash(&chosen.next);
        if next_hash == current_hash {
            return None;
        }
        let repeat_penalty = Self::freecell_recent_repeat_penalty(next_hash, &recent_hashes);
        if repeat_penalty > 20_000 {
            return None;
        }
        let score =
            chosen.immediate_score + Self::freecell_state_eval(&chosen.next) - (repeat_penalty / 2);
        Some((
            format!(
                "{} (shared wand policy)",
                chosen
                    .message
                    .strip_prefix("Hint: ")
                    .unwrap_or(chosen.message.as_str())
            ),
            chosen.source,
            chosen.target,
            chosen.action,
            score,
        ))
    }

    pub(super) fn compute_best_freecell_action(
        &self,
        allowed_sources: Option<&[HintNode]>,
    ) -> Option<(String, HintNode, HintNode, FreecellHintAction, i64)> {
        let decision_deadline =
            Some(Instant::now() + Duration::from_millis(self.freecell_decision_budget_ms()));
        let game = self.imp().game.borrow().freecell().clone();
        let current_hash = Self::freecell_game_hash(&game);
        let recent_hashes: Vec<u64> = self
            .imp()
            .hint_recent_states
            .borrow()
            .iter()
            .copied()
            .collect();
        let (depth, root_beam, branch_beam, node_budget) = self.freecell_search_profile();
        let root_candidates = self.generate_freecell_candidates(&game, allowed_sources, true);
        if root_candidates.is_empty() {
            return None;
        }

        #[derive(Clone)]
        struct RootOption {
            message: String,
            source: HintNode,
            target: HintNode,
            action: FreecellHintAction,
            next: FreecellGame,
            root_score: i64,
            progressed: bool,
            progress_score: i64,
        }

        #[derive(Clone)]
        struct SearchNode {
            game: FreecellGame,
            hash: u64,
            depth: u8,
            first_idx: usize,
            total_score: i64,
        }

        let mut roots = Vec::<RootOption>::new();
        for candidate in root_candidates {
            let next_hash = Self::freecell_game_hash(&candidate.next);
            let (progress_score, progressed, _reason) = Self::freecell_progress_analysis(
                &game,
                &candidate.next,
                current_hash,
                next_hash,
                &recent_hashes,
            );
            let loop_penalty =
                Self::freecell_recent_loop_penalty(next_hash, current_hash, &recent_hashes);
            let repeat_penalty = Self::freecell_recent_repeat_penalty(next_hash, &recent_hashes);
            let non_progress_penalty = if progressed {
                0
            } else {
                18_000 + (-progress_score).max(0)
            };
            let root_score = candidate.immediate_score
                + Self::freecell_state_eval(&candidate.next)
                + (progress_score / 3)
                - loop_penalty
                - repeat_penalty
                - non_progress_penalty;
            roots.push(RootOption {
                message: candidate.message,
                source: candidate.source,
                target: candidate.target,
                action: candidate.action,
                next: candidate.next,
                root_score,
                progressed,
                progress_score,
            });
        }

        roots.sort_by(|a, b| b.root_score.cmp(&a.root_score));
        roots.truncate(root_beam.max(4));
        if roots.is_empty() {
            return None;
        }

        // Never pass on an immediate win.
        if let Some((idx, _)) = roots
            .iter()
            .enumerate()
            .filter(|(_, root)| root.next.is_won())
            .max_by_key(|(_, root)| root.root_score)
        {
            let chosen = &roots[idx];
            let mut msg = chosen.message.clone();
            if self.imp().robot_debug_enabled.get() {
                msg.push_str(" | policy=immediate_win");
            }
            return Some((
                msg,
                chosen.source,
                chosen.target,
                chosen.action,
                chosen.root_score,
            ));
        }

        // Safe foundation moves are always highest priority in FreeCell.
        if let Some((idx, _)) = roots
            .iter()
            .enumerate()
            .find(|(_, root)| match root.action {
                FreecellHintAction::TableauToFoundation { src } => {
                    Self::freecell_safe_foundation_bias(&game, game.tableau_top(src)) > 0
                }
                FreecellHintAction::FreecellToFoundation { cell } => {
                    Self::freecell_safe_foundation_bias(&game, game.freecell_card(cell)) > 0
                }
                _ => false,
            })
        {
            let chosen = &roots[idx];
            let mut msg = chosen.message.clone();
            if self.imp().robot_debug_enabled.get() {
                msg.push_str(" | policy=safe_foundation");
            }
            return Some((
                msg,
                chosen.source,
                chosen.target,
                chosen.action,
                chosen.root_score,
            ));
        }

        let mut best_idx = 0usize;
        let mut best_score = roots[0].root_score;
        let mut best_progressed_score = i64::MIN / 4;
        for (idx, root) in roots.iter().enumerate() {
            if root.progressed && root.root_score > best_progressed_score {
                best_progressed_score = root.root_score;
                best_idx = idx;
                best_score = root.root_score;
            }
        }
        // If immediate move is non-progress, allow bounded tactical lookahead
        // before accepting it; this prevents local shuffles from dominating.
        if best_progressed_score <= i64::MIN / 8 {
            for (idx, root) in roots.iter_mut().enumerate() {
                let path_bonus = self.freecell_path_to_progress_bonus(
                    &root.next,
                    current_hash,
                    &recent_hashes,
                    2,
                    5,
                    42,
                    decision_deadline,
                );
                root.root_score = root.root_score + path_bonus;
                if root.root_score > best_score {
                    best_score = root.root_score;
                    best_idx = idx;
                }
            }
        }
        let mut frontier = roots
            .iter()
            .enumerate()
            .map(|(idx, root)| SearchNode {
                game: root.next.clone(),
                hash: Self::freecell_game_hash(&root.next),
                depth: 1,
                first_idx: idx,
                total_score: root.root_score,
            })
            .collect::<Vec<_>>();

        let mut best_seen_for_hash = HashMap::<u64, i64>::new();
        for node in &frontier {
            best_seen_for_hash.insert(node.hash, node.total_score);
        }

        let mut expanded = 0usize;
        while !frontier.is_empty() && expanded < node_budget {
            if decision_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
                break;
            }

            frontier.sort_by(|a, b| match b.total_score.cmp(&a.total_score) {
                Ordering::Equal => a.depth.cmp(&b.depth),
                other => other,
            });
            let node = frontier.remove(0);
            expanded = expanded.saturating_add(1);

            if node.total_score > best_score {
                best_score = node.total_score;
                best_idx = node.first_idx;
            }
            if node.depth >= depth {
                continue;
            }

            let mut children = self.generate_freecell_candidates(&node.game, None, false);
            if children.is_empty() {
                continue;
            }
            children.sort_by(|a, b| b.immediate_score.cmp(&a.immediate_score));
            children.truncate(branch_beam.max(3));

            for child in children {
                let child_hash = Self::freecell_game_hash(&child.next);
                let child_eval = Self::freecell_state_eval(&child.next);
                let repeat_penalty =
                    Self::freecell_recent_repeat_penalty(child_hash, &recent_hashes);
                let child_score =
                    node.total_score + child.immediate_score + child_eval - repeat_penalty;
                if best_seen_for_hash
                    .get(&child_hash)
                    .is_some_and(|prev| *prev >= child_score)
                {
                    continue;
                }
                best_seen_for_hash.insert(child_hash, child_score);
                frontier.push(SearchNode {
                    game: child.next,
                    hash: child_hash,
                    depth: node.depth.saturating_add(1),
                    first_idx: node.first_idx,
                    total_score: child_score,
                });
            }
        }

        let chosen = &roots[best_idx];
        let mut msg = chosen.message.clone();
        if self.imp().robot_debug_enabled.get() {
            msg.push_str(&format!(
                " | search=best_first depth={} root_beam={} branch_beam={} expanded={} score={} progressed={} progress_score={}",
                depth, root_beam, branch_beam, expanded, best_score, chosen.progressed, chosen.progress_score
            ));
        }
        Some((msg, chosen.source, chosen.target, chosen.action, best_score))
    }

    fn freecell_path_to_progress_bonus(
        &self,
        start: &FreecellGame,
        current_hash: u64,
        recent_hashes: &[u64],
        max_depth: u8,
        beam: usize,
        node_budget: usize,
        deadline: Option<Instant>,
    ) -> i64 {
        #[derive(Clone)]
        struct Node {
            game: FreecellGame,
            depth: u8,
            score: i64,
        }

        let mut frontier = vec![Node {
            game: start.clone(),
            depth: 0,
            score: 0,
        }];
        let mut best_seen = HashMap::<u64, i64>::new();
        let mut expanded = 0usize;
        let mut best_progress_score = i64::MIN / 4;

        while !frontier.is_empty() && expanded < node_budget {
            if deadline.is_some_and(|limit| Instant::now() >= limit) {
                break;
            }
            frontier.sort_by(|a, b| b.score.cmp(&a.score));
            let node = frontier.remove(0);
            expanded = expanded.saturating_add(1);
            if node.depth >= max_depth {
                continue;
            }

            let mut children = self.generate_freecell_candidates(&node.game, None, false);
            if children.is_empty() {
                continue;
            }
            children.sort_by(|a, b| b.immediate_score.cmp(&a.immediate_score));
            children.truncate(beam.max(3));

            for child in children {
                let from_hash = Self::freecell_game_hash(&node.game);
                let child_hash = Self::freecell_game_hash(&child.next);
                let (progress_score, progressed, _) = Self::freecell_progress_analysis(
                    &node.game,
                    &child.next,
                    if node.depth == 0 {
                        current_hash
                    } else {
                        from_hash
                    },
                    child_hash,
                    recent_hashes,
                );
                let aggregate_score = node.score
                    + child.immediate_score
                    + Self::freecell_state_eval(&child.next)
                    + progress_score / 3;
                if best_seen
                    .get(&child_hash)
                    .is_some_and(|prev| *prev >= aggregate_score)
                {
                    continue;
                }
                best_seen.insert(child_hash, aggregate_score);
                if progressed {
                    // Earlier progress is strongly preferred.
                    let depth_bonus = (max_depth.saturating_sub(node.depth) as i64) * 4_000;
                    best_progress_score = best_progress_score.max(progress_score + depth_bonus);
                }
                frontier.push(Node {
                    game: child.next,
                    depth: node.depth.saturating_add(1),
                    score: aggregate_score,
                });
            }
        }

        if best_progress_score > 0 {
            8_000 + best_progress_score.min(20_000)
        } else {
            -9_000
        }
    }

    fn freecell_search_profile(&self) -> (u8, usize, usize, usize) {
        if self.imp().robot_mode_running.get() {
            return (5, 18, 10, 1200);
        }
        (4, 12, 8, 420)
    }

    fn freecell_recent_repeat_penalty(next_hash: u64, recent: &[u64]) -> i64 {
        if let Some(distance) = recent.iter().rev().position(|h| *h == next_hash) {
            let proximity = (64_usize.saturating_sub(distance.min(64))) as i64;
            300 + proximity * 260
        } else {
            0
        }
    }

    fn freecell_decision_budget_ms(&self) -> u64 {
        if self.imp().robot_mode_running.get() {
            return 36;
        }
        // Keep all interactive FreeCell consumers responsive (smart move, wand, robot, hint).
        18
    }

    fn freecell_game_hash(game: &FreecellGame) -> u64 {
        Self::hash_freecell_game_state(game)
    }

    fn freecell_recent_loop_details(
        next_hash: u64,
        current_hash: u64,
        recent: &[u64],
    ) -> (i64, Option<usize>) {
        if next_hash == current_hash {
            // Direct no-op or inverse-undo churn.
            return (120_000, Some(0));
        }
        let mut penalty = 0_i64;
        let mut repeat_distance = None;
        if let Some(prev) = recent.last() {
            if next_hash == *prev {
                penalty += 24_000;
            }
        }
        if recent.len() >= 2 && next_hash == recent[recent.len() - 2] {
            // Explicit A -> B -> A cycle guard.
            penalty += 40_000;
        }
        if let Some(distance) = recent.iter().rev().position(|h| *h == next_hash) {
            // Penalize revisiting recent states; nearer repeats are worse.
            let proximity = (48_usize.saturating_sub(distance.min(48))) as i64;
            penalty += proximity * 220;
            repeat_distance = Some(distance);
        }
        (penalty, repeat_distance)
    }

    fn freecell_recent_loop_penalty(next_hash: u64, current_hash: u64, recent: &[u64]) -> i64 {
        Self::freecell_recent_loop_details(next_hash, current_hash, recent).0
    }

    #[allow(dead_code)]
    fn freecell_short_loop_scan_penalty(
        &self,
        start: &FreecellGame,
        current_hash: u64,
        root_hash: u64,
        depth: u8,
        beam_width: usize,
        max_nodes: usize,
    ) -> i64 {
        #[derive(Clone)]
        struct ScanNode {
            game: FreecellGame,
            hash: u64,
            parent_hash: u64,
            depth: u8,
        }

        let mut queue = VecDeque::new();
        queue.push_back(ScanNode {
            game: start.clone(),
            hash: root_hash,
            parent_hash: current_hash,
            depth: 0,
        });

        let mut best_seen_depth = HashMap::<u64, u8>::new();
        best_seen_depth.insert(root_hash, 0);

        let mut penalty = 0_i64;
        let mut processed = 0_usize;

        while let Some(node) = queue.pop_front() {
            if processed >= max_nodes {
                break;
            }
            processed += 1;

            if node.depth >= depth {
                continue;
            }

            let mut candidates = self.generate_freecell_candidates(&node.game, None, false);
            if candidates.is_empty() {
                continue;
            }
            candidates.sort_by(|a, b| b.immediate_score.cmp(&a.immediate_score));
            candidates.truncate(beam_width.max(2));

            for candidate in candidates {
                let next_hash = Self::freecell_game_hash(&candidate.next);
                let next_depth = node.depth.saturating_add(1);

                if next_hash == current_hash {
                    penalty += 16_000;
                }
                if next_hash == root_hash {
                    penalty += 10_000;
                }
                if next_hash == node.parent_hash {
                    penalty += 12_000;
                }
                if let Some(prev_depth) = best_seen_depth.get(&next_hash).copied() {
                    let depth_gap = next_depth.saturating_sub(prev_depth) as i64;
                    penalty += 1_800 + depth_gap * 350;
                }

                let should_enqueue = match best_seen_depth.get(&next_hash).copied() {
                    None => true,
                    Some(prev) => next_depth < prev,
                };
                if should_enqueue {
                    best_seen_depth.insert(next_hash, next_depth);
                    queue.push_back(ScanNode {
                        game: candidate.next,
                        hash: next_hash,
                        parent_hash: node.hash,
                        depth: next_depth,
                    });
                }
            }
        }

        penalty
    }

    fn freecell_state_eval(game: &FreecellGame) -> i64 {
        let foundation = Self::freecell_foundation_cards(game) as i64;
        let mobility = Self::freecell_legal_move_count(game) as i64;
        let empty_free = game
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count() as i64;
        let empty_cols = game.tableau().iter().filter(|col| col.is_empty()).count() as i64;
        let buried_starters_penalty = Self::freecell_buried_starter_depth_penalty(game);
        let deadlock_penalty = Self::freecell_same_suit_deadlock_penalty(game);

        // Weight empty columns above empty free cells (supermove capacity leverage).
        let mut score =
            foundation * 10_000 + empty_free * 500 + empty_cols * 2_000 + mobility * 160;
        // Strong anti-clog pressure as free cells fill up.
        let occupied = 4_i64.saturating_sub(empty_free);
        if occupied >= 3 {
            score -= 2_000;
        }
        if occupied == 4 {
            score -= 6_500;
        }
        score - buried_starters_penalty - deadlock_penalty
    }

    fn freecell_transition_score(current: &FreecellGame, next: &FreecellGame, bias: i64) -> i64 {
        let mut score = Self::freecell_state_eval(next) - Self::freecell_state_eval(current) + bias;
        let next_empty_free = next
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        if next_empty_free == 0 {
            score -= 300;
        }
        score
    }

    #[allow(dead_code)]
    fn freecell_projected_value(
        &self,
        game: &FreecellGame,
        depth: u8,
        beam_width: usize,
        visited: &mut HashSet<u64>,
    ) -> i64 {
        if game.is_won() || depth == 0 {
            return Self::freecell_state_eval(game);
        }

        let hash = Self::freecell_game_hash(game);
        if !visited.insert(hash) {
            return -100_000;
        }

        let mut candidates = self.generate_freecell_candidates(game, None, false);
        if candidates.is_empty() {
            visited.remove(&hash);
            return Self::freecell_state_eval(game) - 2_000;
        }
        candidates.sort_by(|a, b| b.immediate_score.cmp(&a.immediate_score));
        candidates.truncate(beam_width.max(4));

        let mut best = i64::MIN / 4;
        for candidate in candidates {
            let score = candidate.immediate_score
                + self.freecell_projected_value(
                    &candidate.next,
                    depth.saturating_sub(1),
                    beam_width,
                    visited,
                );
            if score > best {
                best = score;
            }
        }

        visited.remove(&hash);
        best.max(Self::freecell_state_eval(game))
    }

    fn generate_freecell_candidates(
        &self,
        game: &FreecellGame,
        allowed_sources: Option<&[HintNode]>,
        use_boundary_checks: bool,
    ) -> Vec<FreecellSearchCandidate> {
        let allow_source = |source: HintNode| {
            allowed_sources
                .map(|sources| sources.contains(&source))
                .unwrap_or(true)
        };
        let mut out = Vec::<FreecellSearchCandidate>::new();

        let state_guard = if use_boundary_checks {
            Some(self.imp().game.borrow())
        } else {
            None
        };
        let state_ref = state_guard.as_ref();

        for cell in 0..4 {
            let legal = if let Some(state) = state_ref {
                boundary::can_move_freecell_to_foundation(state, GameMode::Freecell, cell)
            } else {
                game.can_move_freecell_to_foundation(cell)
            };
            if !legal {
                continue;
            }
            let mut next = game.clone();
            if !next.move_freecell_to_foundation(cell) {
                continue;
            }
            let foundation_idx = game
                .freecell_card(cell)
                .map(|card| card.suit.foundation_index())
                .unwrap_or(0);
            let source = HintNode::Freecell(cell);
            if !allow_source(source) {
                continue;
            }
            out.push(FreecellSearchCandidate {
                message: format!("Hint: Move F{} to foundation.", cell + 1),
                source,
                target: HintNode::Foundation(foundation_idx),
                action: FreecellHintAction::FreecellToFoundation { cell },
                immediate_score: Self::freecell_transition_score(
                    game,
                    &next,
                    1_000 + Self::freecell_safe_foundation_bias(game, game.freecell_card(cell)),
                ),
                next,
            });
        }

        for src in 0..8 {
            if !game.can_move_tableau_top_to_foundation(src) {
                continue;
            }
            let Some(top) = game
                .tableau()
                .get(src)
                .map(Vec::len)
                .and_then(|len| len.checked_sub(1))
            else {
                continue;
            };
            let mut next = game.clone();
            if !next.move_tableau_top_to_foundation(src) {
                continue;
            }
            let foundation_idx = game
                .tableau_top(src)
                .map(|card| card.suit.foundation_index())
                .unwrap_or(0);
            let source = HintNode::Tableau {
                col: src,
                index: Some(top),
            };
            if !allow_source(source) {
                continue;
            }
            out.push(FreecellSearchCandidate {
                message: format!("Hint: Move T{} to foundation.", src + 1),
                source,
                target: HintNode::Foundation(foundation_idx),
                action: FreecellHintAction::TableauToFoundation { src },
                immediate_score: Self::freecell_transition_score(
                    game,
                    &next,
                    900 + Self::freecell_safe_foundation_bias(game, game.tableau_top(src)),
                ),
                next,
            });
        }

        for cell in 0..4 {
            for dst in 0..8 {
                let legal = if let Some(state) = state_ref {
                    boundary::can_move_freecell_to_tableau(state, GameMode::Freecell, cell, dst)
                } else {
                    game.can_move_freecell_to_tableau(cell, dst)
                };
                if !legal {
                    continue;
                }
                let mut next = game.clone();
                if !next.move_freecell_to_tableau(cell, dst) {
                    continue;
                }
                let source = HintNode::Freecell(cell);
                if !allow_source(source) {
                    continue;
                }
                out.push(FreecellSearchCandidate {
                    message: format!("Hint: Move F{} to T{}.", cell + 1, dst + 1),
                    source,
                    target: HintNode::Tableau {
                        col: dst,
                        index: None,
                    },
                    action: FreecellHintAction::FreecellToTableau { cell, dst },
                    immediate_score: Self::freecell_transition_score(
                        game,
                        &next,
                        Self::freecell_freecell_to_tableau_bias(game, &next, cell)
                            + Self::freecell_multi_cell_utilization_bias(game, &next),
                    ),
                    next,
                });
            }
        }

        for src in 0..8 {
            let len = game.tableau().get(src).map(Vec::len).unwrap_or(0);
            for start in 0..len {
                for dst in 0..8 {
                    if !game.can_move_tableau_run_to_tableau(src, start, dst) {
                        continue;
                    }
                    let mut next = game.clone();
                    if !next.move_tableau_run_to_tableau(src, start, dst) {
                        continue;
                    }
                    let source = HintNode::Tableau {
                        col: src,
                        index: Some(start),
                    };
                    if !allow_source(source) {
                        continue;
                    }
                    let amount = len.saturating_sub(start) as i64;
                    out.push(FreecellSearchCandidate {
                        message: format!(
                            "Hint: Move {} card(s) from T{} to T{}.",
                            len.saturating_sub(start),
                            src + 1,
                            dst + 1
                        ),
                        source,
                        target: HintNode::Tableau {
                            col: dst,
                            index: None,
                        },
                        action: FreecellHintAction::TableauRunToTableau { src, start, dst },
                        immediate_score: Self::freecell_transition_score(
                            game,
                            &next,
                            amount * 14
                                + Self::freecell_king_to_empty_column_bias(game, src, start, dst)
                                + Self::freecell_tableau_main_area_cell_bias(game, &next, src, dst),
                        ),
                        next,
                    });
                }
            }
        }

        for src in 0..8 {
            let Some(top) = game
                .tableau()
                .get(src)
                .map(Vec::len)
                .and_then(|len| len.checked_sub(1))
            else {
                continue;
            };
            let card = game.tableau_top(src);
            for cell in 0..4 {
                if !game.can_move_tableau_top_to_freecell(src, cell) {
                    continue;
                }
                let mut next = game.clone();
                if !next.move_tableau_top_to_freecell(src, cell) {
                    continue;
                }
                let staging_bias = Self::freecell_tableau_to_freecell_bias(game, src);
                let source = HintNode::Tableau {
                    col: src,
                    index: Some(top),
                };
                if !allow_source(source) {
                    continue;
                }
                out.push(FreecellSearchCandidate {
                    message: format!("Hint: Move T{} to free cell F{}.", src + 1, cell + 1),
                    source,
                    target: HintNode::Freecell(cell),
                    action: FreecellHintAction::TableauToFreecell { src, cell },
                    immediate_score: Self::freecell_transition_score(
                        game,
                        &next,
                        staging_bias
                            + Self::freecell_slot_preference_bias(card, cell)
                            + Self::freecell_tableau_to_freecell_unlock_bias(game, &next, src)
                            + Self::freecell_multi_cell_utilization_bias(game, &next),
                    ),
                    next,
                });
            }
        }

        out
    }

    fn freecell_slot_preference_bias(card: Option<crate::game::Card>, cell: usize) -> i64 {
        let Some(card) = card else {
            return 0;
        };
        let preferred = card.suit.foundation_index();
        if cell == preferred {
            70
        } else {
            let dist = preferred.abs_diff(cell);
            match dist {
                1 => 30,
                2 => 10,
                _ => 0,
            }
        }
    }

    fn freecell_tableau_to_freecell_bias(game: &FreecellGame, src: usize) -> i64 {
        let empty_before = game
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let src_len = game.tableau().get(src).map(Vec::len).unwrap_or(0);
        let no_foundation_progress = !Self::freecell_has_foundation_push(game);
        let mobility = Self::freecell_legal_move_count(game);
        let mut bias = 420_i64;

        // Encourage active use of free cells as staging buffers when available.
        if empty_before >= 1 {
            bias += 240;
        }
        if empty_before >= 2 {
            bias += 220;
        }
        if empty_before >= 3 {
            bias += 170;
        }

        // If no direct foundation progress exists, staging becomes much more important.
        if no_foundation_progress {
            bias += 420;
        }

        // Pulling from taller columns tends to unlock structure and route flexibility.
        if src_len >= 2 {
            bias += 120;
        }
        if src_len >= 4 {
            bias += 80;
        }
        if empty_before == 1 {
            // Spending the final free slot should require stronger downstream value.
            bias -= 140;
        }
        // If mobility is constrained and multiple free cells are unused,
        // strongly encourage tactical staging into free cells.
        if mobility < 22 {
            if empty_before >= 2 {
                bias += 220;
            }
            if empty_before >= 3 {
                bias += 180;
            }
        }
        bias
    }

    fn freecell_multi_cell_utilization_bias(current: &FreecellGame, next: &FreecellGame) -> i64 {
        let empty_before = current
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let empty_after = next
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let used_before = 4_usize.saturating_sub(empty_before);
        let used_after = 4_usize.saturating_sub(empty_after);
        let mobility_delta = Self::freecell_legal_move_count(next) as i64
            - Self::freecell_legal_move_count(current) as i64;
        let mut bias = 0_i64;

        if used_after > used_before {
            // Reward broadening free-cell usage when under-utilized.
            bias += 140;
            if used_before <= 1 {
                bias += 200;
            }
            if used_before == 0 {
                bias += 160;
            }
            if mobility_delta > 0 {
                bias += 120;
            }
        } else if used_after < used_before {
            // Small reward for freeing slots, especially when previously saturated.
            bias += 90;
            if used_before == 4 {
                bias += 110;
            }
        }
        bias
    }

    fn freecell_tableau_to_freecell_unlock_bias(
        current: &FreecellGame,
        next: &FreecellGame,
        src: usize,
    ) -> i64 {
        let mut bias = 0_i64;
        let prev_len = current.tableau().get(src).map(Vec::len).unwrap_or(0);
        let next_len = next.tableau().get(src).map(Vec::len).unwrap_or(0);
        if next_len < prev_len {
            let delta_moves = Self::freecell_legal_move_count(next) as i64
                - Self::freecell_legal_move_count(current) as i64;
            bias += delta_moves * 35;
        }

        if next_len == 0 {
            bias += 220;
            return bias;
        }

        if next.can_move_tableau_top_to_foundation(src) {
            bias += 360;
        }
        let can_new_top_move_to_tableau = (0..8)
            .filter(|&dst| dst != src)
            .any(|dst| next.can_move_tableau_run_to_tableau(src, next_len.saturating_sub(1), dst));
        if can_new_top_move_to_tableau {
            bias += 180;
        }
        bias
    }

    fn freecell_freecell_to_tableau_bias(
        current: &FreecellGame,
        next: &FreecellGame,
        cell: usize,
    ) -> i64 {
        let mut bias = 320_i64;
        let empty_before = current
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let empty_after = next
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();

        if empty_after > empty_before {
            bias += 160;
        }
        if empty_before == 0 {
            bias += 220;
        }
        if next.can_move_freecell_to_foundation(cell) {
            bias += 120;
        }
        let delta_moves = Self::freecell_legal_move_count(next) as i64
            - Self::freecell_legal_move_count(current) as i64;
        bias + delta_moves * 25
    }

    fn freecell_has_foundation_push(game: &FreecellGame) -> bool {
        (0..4).any(|cell| game.can_move_freecell_to_foundation(cell))
            || (0..8).any(|src| game.can_move_tableau_top_to_foundation(src))
    }

    fn freecell_safe_foundation_bias(game: &FreecellGame, card: Option<Card>) -> i64 {
        let Some(card) = card else {
            return 0;
        };
        if card.rank <= 1 {
            return 90_000;
        }
        let needed = usize::from(card.rank.saturating_sub(1));
        let opposite_ok = if card.color_red() {
            game.foundations()[0].len() >= needed && game.foundations()[3].len() >= needed
        } else {
            game.foundations()[1].len() >= needed && game.foundations()[2].len() >= needed
        };
        if opposite_ok {
            90_000
        } else {
            0
        }
    }

    fn freecell_king_to_empty_column_bias(
        game: &FreecellGame,
        src: usize,
        start: usize,
        dst: usize,
    ) -> i64 {
        if !game.tableau().get(dst).map(Vec::is_empty).unwrap_or(false) {
            return 0;
        }
        let Some(card) = game.tableau_card(src, start) else {
            return 0;
        };
        if card.rank == 13 {
            700
        } else {
            0
        }
    }

    fn freecell_foundation_cards(game: &FreecellGame) -> usize {
        game.foundations().iter().map(Vec::len).sum()
    }

    fn freecell_tableau_order_score(game: &FreecellGame) -> i64 {
        let mut total = 0_i64;
        for col in game.tableau() {
            if col.is_empty() {
                continue;
            }
            let mut run_len = 1_i64;
            for idx in (1..col.len()).rev() {
                let below = col[idx];
                let above = col[idx - 1];
                let ok_rank = above.rank == below.rank.saturating_add(1);
                let ok_color = above.color_red() != below.color_red();
                if ok_rank && ok_color {
                    run_len += 1;
                } else {
                    break;
                }
            }
            // Favor building longer ordered runs; quadratic weighting.
            total += run_len * run_len;
        }
        total
    }

    fn freecell_supermove_capacity(game: &FreecellGame) -> usize {
        let ef = game
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let et = game.tableau().iter().filter(|col| col.is_empty()).count();
        (ef + 1) * (1usize << et)
    }

    fn freecell_tableau_main_area_cell_bias(
        current: &FreecellGame,
        next: &FreecellGame,
        src: usize,
        dst: usize,
    ) -> i64 {
        let dst_was_empty = current
            .tableau()
            .get(dst)
            .map(Vec::is_empty)
            .unwrap_or(false);
        let src_now_empty = next.tableau().get(src).map(Vec::is_empty).unwrap_or(false);
        let capacity_delta = Self::freecell_supermove_capacity(next) as i64
            - Self::freecell_supermove_capacity(current) as i64;
        let mut bias = capacity_delta * 180;

        if dst_was_empty {
            // Encourage active use of tableau empty columns as temporary cells.
            bias += 320;
        }
        if src_now_empty {
            // Creating a new empty column is highly valuable.
            bias += 900;
        }
        bias
    }

    fn freecell_legal_move_count(game: &FreecellGame) -> usize {
        let mut count = 0_usize;
        for cell in 0..4 {
            if game.can_move_freecell_to_foundation(cell) {
                count += 1;
            }
            for dst in 0..8 {
                if game.can_move_freecell_to_tableau(cell, dst) {
                    count += 1;
                }
            }
        }
        for src in 0..8 {
            if game.can_move_tableau_top_to_foundation(src) {
                count += 1;
            }
            for cell in 0..4 {
                if game.can_move_tableau_top_to_freecell(src, cell) {
                    count += 1;
                }
            }
            let len = game.tableau().get(src).map(Vec::len).unwrap_or(0);
            for start in 0..len {
                for dst in 0..8 {
                    if game.can_move_tableau_run_to_tableau(src, start, dst) {
                        count += 1;
                    }
                }
            }
        }
        count
    }

    fn freecell_foundation_push_count(game: &FreecellGame) -> usize {
        let from_freecells = (0..4)
            .filter(|&cell| game.can_move_freecell_to_foundation(cell))
            .count();
        let from_tableau = (0..8)
            .filter(|&src| game.can_move_tableau_top_to_foundation(src))
            .count();
        from_freecells + from_tableau
    }

    fn freecell_buried_starter_depth_penalty(game: &FreecellGame) -> i64 {
        let mut penalty = 0_i64;
        for col in game.tableau() {
            for (idx, card) in col.iter().enumerate() {
                if card.rank <= 3 {
                    // Deeper buried A/2/3 cards slow foundation development.
                    let depth = (col.len().saturating_sub(idx + 1)) as i64;
                    penalty += depth * 120;
                }
            }
        }
        penalty
    }

    fn freecell_same_suit_deadlock_penalty(game: &FreecellGame) -> i64 {
        let mut penalty = 0_i64;
        for col in game.tableau() {
            for lower_idx in 0..col.len() {
                for upper_idx in (lower_idx + 1)..col.len() {
                    let lower = col[lower_idx];
                    let upper = col[upper_idx];
                    if lower.suit == upper.suit && lower.rank < upper.rank {
                        penalty += 90;
                    }
                }
            }
        }
        penalty
    }

    fn freecell_exposed_top_delta(current: &FreecellGame, next: &FreecellGame) -> i64 {
        let mut delta = 0_i64;
        for col in 0..8 {
            let c_col = &current.tableau()[col];
            let n_col = &next.tableau()[col];
            let c_len = c_col.len();
            let n_len = n_col.len();
            if n_len < c_len && c_len > 0 {
                // Removed cards from this column, potentially exposing a new actionable top.
                if n_len == 0 {
                    delta += 2;
                } else if c_col.last() != n_col.last() {
                    delta += 1;
                }
            }
        }
        delta
    }
}

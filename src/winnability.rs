/* winnability.rs
 *
 * Copyright 2026 emviolet
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 *
 * SPDX-License-Identifier: GPL-3.0-or-later
 */

use std::cmp::Ordering as CmpOrdering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicU8, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use crate::engine::freecell_planner::FreecellPlannerAction;
use crate::engine::moves::HintMove;
use crate::game::{
    Card, DrawMode, FreecellGame, KlondikeGame, SolverMove, SpiderGame, SpiderSuitMode,
};

#[derive(Debug, Clone)]
pub struct SeedWinnabilityCheckResult {
    pub winnable: bool,
    pub iterations: usize,
    pub moves_to_win: Option<u32>,
    pub hit_state_limit: bool,
    pub solver_line: Option<Vec<SolverMove>>,
    pub hint_line: Option<Vec<HintMove>>,
    pub freecell_line: Option<Vec<FreecellPlannerAction>>,
    pub canceled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum FreecellFindStopReason {
    None = 0,
    Won = 1,
    NoAction = 2,
    RepeatState = 3,
    StepLimit = 4,
    Canceled = 5,
    InvalidMove = 6,
}

impl FreecellFindStopReason {
    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Won,
            2 => Self::NoAction,
            3 => Self::RepeatState,
            4 => Self::StepLimit,
            5 => Self::Canceled,
            6 => Self::InvalidMove,
            _ => Self::None,
        }
    }

    fn as_label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Won => "won",
            Self::NoAction => "no_action",
            Self::RepeatState => "repeat_state",
            Self::StepLimit => "step_limit",
            Self::Canceled => "canceled",
            Self::InvalidMove => "invalid_move",
        }
    }
}

#[derive(Debug)]
pub struct FreecellFindProgress {
    pub checked: AtomicU32,
    pub last_seed: AtomicU64,
    pub last_expanded_states: AtomicUsize,
    pub last_generated_branches: AtomicUsize,
    pub last_elapsed_ms: AtomicU64,
    pub last_stop_reason: AtomicU8,
}

impl Default for FreecellFindProgress {
    fn default() -> Self {
        Self {
            checked: AtomicU32::new(0),
            last_seed: AtomicU64::new(0),
            last_expanded_states: AtomicUsize::new(0),
            last_generated_branches: AtomicUsize::new(0),
            last_elapsed_ms: AtomicU64::new(0),
            last_stop_reason: AtomicU8::new(FreecellFindStopReason::None as u8),
        }
    }
}

impl FreecellFindProgress {
    fn store_attempt(
        &self,
        seed: u64,
        checked: u32,
        expanded_states: usize,
        generated_branches: usize,
        elapsed_ms: u64,
        stop_reason: FreecellFindStopReason,
    ) {
        self.checked.fetch_max(checked, Ordering::Relaxed);
        self.last_seed.store(seed, Ordering::Relaxed);
        self.last_expanded_states
            .store(expanded_states, Ordering::Relaxed);
        self.last_generated_branches
            .store(generated_branches, Ordering::Relaxed);
        self.last_elapsed_ms.store(elapsed_ms, Ordering::Relaxed);
        self.last_stop_reason
            .store(stop_reason as u8, Ordering::Relaxed);
    }
}

pub fn freecell_find_stop_reason_label(code: u8) -> &'static str {
    FreecellFindStopReason::from_code(code).as_label()
}

pub fn default_find_winnable_attempts() -> u32 {
    thread::available_parallelism()
        .map(|n| (n.get() * 4).clamp(8, 64) as u32)
        .unwrap_or(24)
}

fn capped_seed_check_budgets(
    draw_mode: DrawMode,
    guided: usize,
    exhaustive: usize,
) -> (usize, usize) {
    let (guided_cap, exhaustive_cap) = match draw_mode {
        DrawMode::One => (80_000, 120_000),
        DrawMode::Two => (70_000, 100_000),
        DrawMode::Three => (60_000, 90_000),
        DrawMode::Four => (50_000, 80_000),
        DrawMode::Five => (45_000, 70_000),
    };
    (guided.min(guided_cap), exhaustive.min(exhaustive_cap))
}

pub fn is_seed_winnable(
    seed: u64,
    draw_mode: DrawMode,
    guided_budget: usize,
    exhaustive_budget: usize,
    cancel: &AtomicBool,
) -> Option<SeedWinnabilityCheckResult> {
    let (guided_budget, exhaustive_budget) =
        capped_seed_check_budgets(draw_mode, guided_budget, exhaustive_budget);
    let mut game = KlondikeGame::new_with_seed(seed);
    game.set_draw_mode(draw_mode);
    let guided_progress = AtomicUsize::new(0);
    let Some(guided) =
        game.guided_winnability_cancelable_with_progress(guided_budget, cancel, &guided_progress)
    else {
        return Some(SeedWinnabilityCheckResult {
            winnable: false,
            iterations: guided_progress.load(Ordering::Relaxed),
            moves_to_win: None,
            hit_state_limit: true,
            solver_line: None,
            hint_line: None,
            freecell_line: None,
            canceled: true,
        });
    };
    if guided.winnable {
        let Some(solver_line) = game.guided_winning_line_cancelable(guided_budget, cancel) else {
            return Some(SeedWinnabilityCheckResult {
                winnable: false,
                iterations: guided.explored_states,
                moves_to_win: None,
                hit_state_limit: true,
                solver_line: None,
                hint_line: None,
                freecell_line: None,
                canceled: true,
            });
        };
        return Some(SeedWinnabilityCheckResult {
            winnable: true,
            iterations: guided.explored_states,
            moves_to_win: guided.win_depth,
            hit_state_limit: guided.hit_state_limit,
            solver_line,
            hint_line: None,
            freecell_line: None,
            canceled: false,
        });
    }

    let exhaustive_progress = AtomicUsize::new(0);
    let Some(exhaustive) = game.analyze_winnability_cancelable_with_progress(
        exhaustive_budget,
        cancel,
        &exhaustive_progress,
    ) else {
        return Some(SeedWinnabilityCheckResult {
            winnable: false,
            iterations: guided
                .explored_states
                .saturating_add(exhaustive_progress.load(Ordering::Relaxed)),
            moves_to_win: None,
            hit_state_limit: true,
            solver_line: None,
            hint_line: None,
            freecell_line: None,
            canceled: true,
        });
    };
    Some(SeedWinnabilityCheckResult {
        winnable: exhaustive.winnable,
        iterations: guided
            .explored_states
            .saturating_add(exhaustive.explored_states),
        moves_to_win: exhaustive.win_depth,
        hit_state_limit: exhaustive.hit_state_limit,
        solver_line: None,
        hint_line: None,
        freecell_line: None,
        canceled: false,
    })
}

#[derive(Clone)]
struct SpiderGuidedNode {
    priority: i64,
    serial: u64,
    depth: u32,
    parent_hash: Option<u64>,
    state: SpiderGame,
}

impl PartialEq for SpiderGuidedNode {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.serial == other.serial
    }
}

impl Eq for SpiderGuidedNode {}

impl PartialOrd for SpiderGuidedNode {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for SpiderGuidedNode {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.serial.cmp(&self.serial))
    }
}

fn capped_spider_seed_check_budget(
    suit_mode: SpiderSuitMode,
    guided_budget: usize,
    exhaustive_budget: usize,
) -> usize {
    let budget = guided_budget.saturating_add(exhaustive_budget);
    let cap = match suit_mode {
        SpiderSuitMode::One => 180_000,
        SpiderSuitMode::Two => 130_000,
        SpiderSuitMode::Three => 80_000,
        SpiderSuitMode::Four => 60_000,
    };
    budget.min(cap)
}

fn spider_successor_limit(suit_mode: SpiderSuitMode) -> usize {
    match suit_mode {
        SpiderSuitMode::One => 160,
        SpiderSuitMode::Two => 112,
        SpiderSuitMode::Three => 72,
        SpiderSuitMode::Four => 56,
    }
}

fn spider_state_hash(game: &SpiderGame) -> u64 {
    let mut hasher = DefaultHasher::new();
    game.hash(&mut hasher);
    hasher.finish()
}

fn spider_face_up_count(game: &SpiderGame) -> usize {
    game.tableau()
        .iter()
        .map(|pile| pile.iter().filter(|card| card.face_up).count())
        .sum()
}

fn spider_empty_col_count(game: &SpiderGame) -> usize {
    game.tableau().iter().filter(|pile| pile.is_empty()).count()
}

fn is_descending_face_up_run(cards: &[Card]) -> bool {
    cards.windows(2).all(|pair| {
        let a = pair[0];
        let b = pair[1];
        a.face_up && b.face_up && a.rank == b.rank + 1
    })
}

fn spider_transition_score(
    current: &SpiderGame,
    next: &SpiderGame,
    hint_move: HintMove,
    parent_hash: Option<u64>,
) -> i64 {
    let completed_delta = next
        .completed_runs()
        .saturating_sub(current.completed_runs()) as i64;
    let face_up_delta = spider_face_up_count(next) as i64 - spider_face_up_count(current) as i64;
    let empty_delta = spider_empty_col_count(next) as i64 - spider_empty_col_count(current) as i64;

    let mut score = completed_delta * 120_000 + face_up_delta * 180 + empty_delta * 700;
    if matches!(hint_move, HintMove::Draw) {
        score -= 80;
    }
    if parent_hash.is_some_and(|hash| hash == spider_state_hash(next)) {
        score -= 9_000;
    }
    score
}

fn spider_successors_guided(
    game: &SpiderGame,
    parent_hash: Option<u64>,
) -> Vec<(SpiderGame, HintMove, i64)> {
    let mut successors = Vec::new();
    for src in 0..10 {
        let source = &game.tableau()[src];
        for start in 0..source.len() {
            if !source[start].face_up || !is_descending_face_up_run(&source[start..]) {
                continue;
            }
            for dst in 0..10 {
                if !game.can_move_run(src, start, dst) {
                    continue;
                }
                let mut next = game.clone();
                if !next.move_run(src, start, dst) {
                    continue;
                }
                let hint_move = HintMove::TableauRunToTableau { src, start, dst };
                let transition = spider_transition_score(game, &next, hint_move, parent_hash);
                successors.push((next, hint_move, transition));
            }
        }
    }

    if game.can_deal_from_stock() {
        let mut next = game.clone();
        if next.deal_from_stock() {
            let hint_move = HintMove::Draw;
            let transition = spider_transition_score(game, &next, hint_move, parent_hash);
            successors.push((next, hint_move, transition));
        }
    }

    successors
}

pub fn is_spider_seed_winnable(
    seed: u64,
    suit_mode: SpiderSuitMode,
    guided_budget: usize,
    exhaustive_budget: usize,
    cancel: &AtomicBool,
) -> Option<SeedWinnabilityCheckResult> {
    if cancel.load(Ordering::Relaxed) {
        return Some(SeedWinnabilityCheckResult {
            winnable: false,
            iterations: 0,
            moves_to_win: None,
            hit_state_limit: true,
            solver_line: None,
            hint_line: None,
            freecell_line: None,
            canceled: true,
        });
    }

    let start = SpiderGame::new_with_seed_and_mode(seed, suit_mode);
    if start.is_won() {
        return Some(SeedWinnabilityCheckResult {
            winnable: true,
            iterations: 1,
            moves_to_win: Some(0),
            hit_state_limit: false,
            solver_line: None,
            hint_line: Some(Vec::new()),
            freecell_line: None,
            canceled: false,
        });
    }

    let max_states = capped_spider_seed_check_budget(suit_mode, guided_budget, exhaustive_budget);
    if max_states == 0 {
        return Some(SeedWinnabilityCheckResult {
            winnable: false,
            iterations: 0,
            moves_to_win: None,
            hit_state_limit: true,
            solver_line: None,
            hint_line: None,
            freecell_line: None,
            canceled: false,
        });
    }

    let mut visited: HashSet<SpiderGame> = HashSet::new();
    let mut frontier: BinaryHeap<SpiderGuidedNode> = BinaryHeap::new();
    let mut serial = 0_u64;
    let mut generated_states = 1_usize;
    let max_generated_states = max_states.max(1);

    let start_hash = spider_state_hash(&start);
    let mut parent_by_hash: HashMap<u64, (Option<u64>, Option<HintMove>)> = HashMap::new();
    parent_by_hash.insert(start_hash, (None, None));

    frontier.push(SpiderGuidedNode {
        priority: (start.completed_runs() as i64) * 100_000,
        serial,
        depth: 0,
        parent_hash: None,
        state: start,
    });

    while let Some(node) = frontier.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Some(SeedWinnabilityCheckResult {
                winnable: false,
                iterations: visited.len(),
                moves_to_win: None,
                hit_state_limit: true,
                solver_line: None,
                hint_line: None,
                freecell_line: None,
                canceled: true,
            });
        }

        let state = node.state;
        if !visited.insert(state.clone()) {
            continue;
        }
        if state.is_won() {
            let mut line_rev: Vec<HintMove> = Vec::new();
            let mut cursor = Some(spider_state_hash(&state));
            while let Some(hash) = cursor {
                let Some((parent_hash, hint_move)) = parent_by_hash.get(&hash).copied() else {
                    break;
                };
                if let Some(mv) = hint_move {
                    line_rev.push(mv);
                }
                cursor = parent_hash;
            }
            line_rev.reverse();
            return Some(SeedWinnabilityCheckResult {
                winnable: true,
                iterations: visited.len(),
                moves_to_win: Some(node.depth),
                hit_state_limit: false,
                solver_line: None,
                hint_line: Some(line_rev),
                freecell_line: None,
                canceled: false,
            });
        }

        if visited.len() >= max_states {
            return Some(SeedWinnabilityCheckResult {
                winnable: false,
                iterations: visited.len(),
                moves_to_win: None,
                hit_state_limit: true,
                solver_line: None,
                hint_line: None,
                freecell_line: None,
                canceled: false,
            });
        }

        let current_hash = spider_state_hash(&state);
        let mut successors = spider_successors_guided(&state, node.parent_hash);
        successors.sort_by(|a, b| b.2.cmp(&a.2));
        if successors.len() > spider_successor_limit(suit_mode) {
            successors.truncate(spider_successor_limit(suit_mode));
        }

        for (next, hint_move, transition_score) in successors {
            if cancel.load(Ordering::Relaxed) {
                return Some(SeedWinnabilityCheckResult {
                    winnable: false,
                    iterations: visited.len(),
                    moves_to_win: None,
                    hit_state_limit: true,
                    solver_line: None,
                    hint_line: None,
                    freecell_line: None,
                    canceled: true,
                });
            }

            if visited.contains(&next) {
                continue;
            }
            if generated_states >= max_generated_states {
                return Some(SeedWinnabilityCheckResult {
                    winnable: false,
                    iterations: visited.len(),
                    moves_to_win: None,
                    hit_state_limit: true,
                    solver_line: None,
                    hint_line: None,
                    freecell_line: None,
                    canceled: false,
                });
            }

            serial = serial.wrapping_add(1);
            generated_states = generated_states.saturating_add(1);
            let next_hash = spider_state_hash(&next);
            parent_by_hash
                .entry(next_hash)
                .or_insert((Some(current_hash), Some(hint_move)));
            frontier.push(SpiderGuidedNode {
                priority: transition_score + (next.completed_runs() as i64) * 100_000
                    - i64::from(node.depth) * 2,
                serial,
                depth: node.depth + 1,
                parent_hash: Some(current_hash),
                state: next,
            });
        }
    }

    Some(SeedWinnabilityCheckResult {
        winnable: false,
        iterations: visited.len(),
        moves_to_win: None,
        hit_state_limit: false,
        solver_line: None,
        hint_line: None,
        freecell_line: None,
        canceled: false,
    })
}

pub(crate) fn freecell_wand_state_hash(game: &FreecellGame) -> u64 {
    crate::engine::freecell_planner::zobrist_hash(game)
}

#[derive(Clone)]
struct FreecellGuidedNode {
    priority: i64,
    serial: u64,
    depth: u32,
    state: FreecellGame,
}

impl PartialEq for FreecellGuidedNode {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.serial == other.serial
    }
}

impl Eq for FreecellGuidedNode {}

impl PartialOrd for FreecellGuidedNode {
    fn partial_cmp(&self, other: &Self) -> Option<CmpOrdering> {
        Some(self.cmp(other))
    }
}

impl Ord for FreecellGuidedNode {
    fn cmp(&self, other: &Self) -> CmpOrdering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.serial.cmp(&self.serial))
    }
}

fn capped_freecell_seed_check_budget(guided_budget: usize, exhaustive_budget: usize) -> usize {
    guided_budget
        .saturating_add(exhaustive_budget)
        .clamp(30_000, 180_000)
}

fn freecell_foundation_cards(game: &FreecellGame) -> usize {
    game.foundations().iter().map(Vec::len).sum()
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

fn freecell_buried_starter_depth_penalty(game: &FreecellGame) -> i64 {
    let mut penalty = 0_i64;
    for col in game.tableau() {
        for (idx, card) in col.iter().enumerate() {
            if card.rank <= 3 {
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

fn freecell_state_eval(game: &FreecellGame) -> i64 {
    let foundation = freecell_foundation_cards(game) as i64;
    let mobility = freecell_legal_move_count(game) as i64;
    let empty_free = game
        .freecells()
        .iter()
        .filter(|slot| slot.is_none())
        .count() as i64;
    let empty_cols = game.tableau().iter().filter(|col| col.is_empty()).count() as i64;
    let buried_starters_penalty = freecell_buried_starter_depth_penalty(game);
    let deadlock_penalty = freecell_same_suit_deadlock_penalty(game);

    let mut score = foundation * 10_000 + empty_free * 500 + empty_cols * 2_000 + mobility * 160;
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
    let mut score = freecell_state_eval(next) - freecell_state_eval(current) + bias;
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

fn freecell_slot_preference_bias(card: Option<crate::game::Card>, cell: usize) -> i64 {
    let Some(card) = card else {
        return 0;
    };
    let preferred = card.suit.foundation_index();
    if cell == preferred {
        70
    } else {
        match preferred.abs_diff(cell) {
            1 => 30,
            2 => 10,
            _ => 0,
        }
    }
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

fn freecell_supermove_capacity(game: &FreecellGame) -> usize {
    let ef = game
        .freecells()
        .iter()
        .filter(|slot| slot.is_none())
        .count();
    let et = game.tableau().iter().filter(|col| col.is_empty()).count();
    (ef + 1) * (1usize << et)
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
    let capacity_delta =
        freecell_supermove_capacity(next) as i64 - freecell_supermove_capacity(current) as i64;
    let mut bias = capacity_delta * 180;

    if dst_was_empty {
        bias += 320;
    }
    if src_now_empty {
        bias += 900;
    }
    bias
}

fn freecell_tableau_to_freecell_bias(game: &FreecellGame, src: usize) -> i64 {
    let empty_before = game
        .freecells()
        .iter()
        .filter(|slot| slot.is_none())
        .count();
    let src_len = game.tableau().get(src).map(Vec::len).unwrap_or(0);
    let no_foundation_progress = !freecell_has_foundation_push(game);
    let mobility = freecell_legal_move_count(game);
    let mut bias = 420_i64;

    if empty_before >= 1 {
        bias += 240;
    }
    if empty_before >= 2 {
        bias += 220;
    }
    if empty_before >= 3 {
        bias += 170;
    }
    if no_foundation_progress {
        bias += 420;
    }
    if src_len >= 2 {
        bias += 120;
    }
    if src_len >= 4 {
        bias += 80;
    }
    if empty_before == 1 {
        bias -= 140;
    }
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
    let mobility_delta =
        freecell_legal_move_count(next) as i64 - freecell_legal_move_count(current) as i64;
    let mut bias = 0_i64;

    if used_after > used_before {
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
        let delta_moves =
            freecell_legal_move_count(next) as i64 - freecell_legal_move_count(current) as i64;
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

fn freecell_freecell_to_tableau_bias(current: &FreecellGame, next: &FreecellGame) -> i64 {
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
    let delta_moves =
        freecell_legal_move_count(next) as i64 - freecell_legal_move_count(current) as i64;
    bias + delta_moves * 25
}

fn freecell_wand_candidates(
    game: &FreecellGame,
) -> Vec<(FreecellGame, FreecellPlannerAction, i64)> {
    let mut out = Vec::new();

    for cell in 0..4 {
        if !game.can_move_freecell_to_foundation(cell) {
            continue;
        }
        let mut next = game.clone();
        if !next.move_freecell_to_foundation(cell) {
            continue;
        }
        let score = freecell_transition_score(
            game,
            &next,
            1_000 + freecell_safe_foundation_bias(game, game.freecell_card(cell)),
        );
        out.push((
            next,
            FreecellPlannerAction::FreecellToFoundation { cell },
            score,
        ));
    }

    for src in 0..8 {
        if !game.can_move_tableau_top_to_foundation(src) {
            continue;
        }
        let mut next = game.clone();
        if !next.move_tableau_top_to_foundation(src) {
            continue;
        }
        let score = freecell_transition_score(
            game,
            &next,
            900 + freecell_safe_foundation_bias(game, game.tableau_top(src)),
        );
        out.push((
            next,
            FreecellPlannerAction::TableauToFoundation { src },
            score,
        ));
    }

    for cell in 0..4 {
        for dst in 0..8 {
            if !game.can_move_freecell_to_tableau(cell, dst) {
                continue;
            }
            let mut next = game.clone();
            if !next.move_freecell_to_tableau(cell, dst) {
                continue;
            }
            let score = freecell_transition_score(
                game,
                &next,
                freecell_freecell_to_tableau_bias(game, &next)
                    + freecell_multi_cell_utilization_bias(game, &next),
            );
            out.push((
                next,
                FreecellPlannerAction::FreecellToTableau { cell, dst },
                score,
            ));
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
                let amount = len.saturating_sub(start) as i64;
                let score = freecell_transition_score(
                    game,
                    &next,
                    amount * 14
                        + freecell_king_to_empty_column_bias(game, src, start, dst)
                        + freecell_tableau_main_area_cell_bias(game, &next, src, dst),
                );
                out.push((
                    next,
                    FreecellPlannerAction::TableauRunToTableau { src, start, dst },
                    score,
                ));
            }
        }
    }

    for src in 0..8 {
        let card = game.tableau_top(src);
        for cell in 0..4 {
            if !game.can_move_tableau_top_to_freecell(src, cell) {
                continue;
            }
            let mut next = game.clone();
            if !next.move_tableau_top_to_freecell(src, cell) {
                continue;
            }
            let score = freecell_transition_score(
                game,
                &next,
                freecell_tableau_to_freecell_bias(game, src)
                    + freecell_slot_preference_bias(card, cell)
                    + freecell_tableau_to_freecell_unlock_bias(game, &next, src)
                    + freecell_multi_cell_utilization_bias(game, &next),
            );
            out.push((
                next,
                FreecellPlannerAction::TableauToFreecell { src, cell },
                score,
            ));
        }
    }

    out
}

fn freecell_recent_repeat_penalty(next_hash: u64, recent: &[u64]) -> i64 {
    if let Some(distance) = recent.iter().rev().position(|h| *h == next_hash) {
        let proximity = (64_usize.saturating_sub(distance.min(64))) as i64;
        300 + proximity * 260
    } else {
        0
    }
}

fn freecell_recent_repeat_penalty_deque(next_hash: u64, recent: &VecDeque<u64>) -> i64 {
    if let Some(distance) = recent.iter().rev().position(|h| *h == next_hash) {
        let proximity = (64_usize.saturating_sub(distance.min(64))) as i64;
        300 + proximity * 260
    } else {
        0
    }
}

fn freecell_recent_repeat_distance(next_hash: u64, recent: &[u64]) -> Option<usize> {
    recent.iter().rev().position(|h| *h == next_hash)
}

fn freecell_recent_repeat_distance_deque(next_hash: u64, recent: &VecDeque<u64>) -> Option<usize> {
    recent.iter().rev().position(|h| *h == next_hash)
}

fn freecell_is_short_cycle(next_hash: u64, recent: &[u64]) -> bool {
    if recent.len() >= 2 && next_hash == recent[recent.len() - 2] {
        // Explicit A -> B -> A immediate reversal guard.
        return true;
    }
    false
}

fn freecell_is_short_cycle_deque(next_hash: u64, recent: &VecDeque<u64>) -> bool {
    if recent.len() >= 2 {
        let second_last_idx = recent.len() - 2;
        if recent.get(second_last_idx).copied() == Some(next_hash) {
            // Explicit A -> B -> A immediate reversal guard.
            return true;
        }
    }
    false
}

pub(crate) fn freecell_wand_best_action(
    game: &FreecellGame,
    current_hash: u64,
    recent_hashes: &[u64],
) -> Option<FreecellPlannerAction> {
    freecell_select_best_action_avoiding_seen(
        freecell_wand_candidates(game),
        current_hash,
        recent_hashes,
        &HashSet::new(),
    )
}

pub(crate) fn freecell_wand_best_action_avoiding_seen(
    game: &FreecellGame,
    current_hash: u64,
    recent_hashes: &[u64],
    seen_hashes: &HashSet<u64>,
) -> Option<FreecellPlannerAction> {
    freecell_select_best_action_avoiding_seen(
        freecell_wand_candidates(game),
        current_hash,
        recent_hashes,
        seen_hashes,
    )
}

fn freecell_select_best_action_avoiding_seen(
    candidates: Vec<(FreecellGame, FreecellPlannerAction, i64)>,
    current_hash: u64,
    recent_hashes: &[u64],
    seen_hashes: &HashSet<u64>,
) -> Option<FreecellPlannerAction> {
    let mut best: Option<(FreecellPlannerAction, i64, bool)> = None;
    for (next, action, immediate) in candidates {
        let next_hash = freecell_wand_state_hash(&next);
        if next_hash == current_hash || seen_hashes.contains(&next_hash) {
            continue;
        }
        if freecell_is_short_cycle(next_hash, recent_hashes) {
            continue;
        }
        if freecell_recent_repeat_distance(next_hash, recent_hashes).is_some() {
            continue;
        }
        let repeat_penalty = freecell_recent_repeat_penalty(next_hash, recent_hashes);
        let winning = next.is_won();
        let score = immediate + freecell_state_eval(&next) - (repeat_penalty / 2);
        let replace = match best {
            None => true,
            Some((_a, best_score, best_winning)) => {
                (winning && !best_winning) || (winning == best_winning && score > best_score)
            }
        };
        if replace {
            best = Some((action, score, winning));
            if winning {
                break;
            }
        }
    }
    best.map(|(action, _score, _winning)| action)
}

fn freecell_select_best_action_from_deque_avoiding_seen(
    candidates: Vec<(FreecellGame, FreecellPlannerAction, i64)>,
    current_hash: u64,
    recent_hashes: &VecDeque<u64>,
    seen_hashes: &HashSet<u64>,
) -> Option<FreecellPlannerAction> {
    let mut best: Option<(FreecellPlannerAction, i64, bool)> = None;
    for (next, action, immediate) in candidates {
        let next_hash = freecell_wand_state_hash(&next);
        if next_hash == current_hash || seen_hashes.contains(&next_hash) {
            continue;
        }
        if freecell_is_short_cycle_deque(next_hash, recent_hashes) {
            continue;
        }
        if freecell_recent_repeat_distance_deque(next_hash, recent_hashes).is_some() {
            continue;
        }
        let repeat_penalty = freecell_recent_repeat_penalty_deque(next_hash, recent_hashes);
        let winning = next.is_won();
        let score = immediate + freecell_state_eval(&next) - (repeat_penalty / 2);
        let replace = match best {
            None => true,
            Some((_a, best_score, best_winning)) => {
                (winning && !best_winning) || (winning == best_winning && score > best_score)
            }
        };
        if replace {
            best = Some((action, score, winning));
            if winning {
                break;
            }
        }
    }
    best.map(|(action, _score, _winning)| action)
}

fn apply_freecell_action(game: &mut FreecellGame, action: FreecellPlannerAction) -> bool {
    match action {
        FreecellPlannerAction::TableauToFoundation { src } => {
            game.move_tableau_top_to_foundation(src)
        }
        FreecellPlannerAction::FreecellToFoundation { cell } => {
            game.move_freecell_to_foundation(cell)
        }
        FreecellPlannerAction::TableauRunToTableau { src, start, dst } => {
            game.move_tableau_run_to_tableau(src, start, dst)
        }
        FreecellPlannerAction::TableauToFreecell { src, cell } => {
            game.move_tableau_top_to_freecell(src, cell)
        }
        FreecellPlannerAction::FreecellToTableau { cell, dst } => {
            game.move_freecell_to_tableau(cell, dst)
        }
    }
}

struct FreecellSinglePlaythroughResult {
    line: Option<Vec<FreecellPlannerAction>>,
    expanded_states: usize,
    generated_branches: usize,
    elapsed_ms: u64,
    stop_reason: FreecellFindStopReason,
}

fn freecell_single_playthrough_line(
    seed: u64,
    card_count_mode: crate::game::FreecellCardCountMode,
    cancel: &AtomicBool,
) -> FreecellSinglePlaythroughResult {
    const MAX_STEPS: usize = 1000;
    let started = std::time::Instant::now();
    let finish =
        |line: Option<Vec<FreecellPlannerAction>>,
         expanded_states: usize,
         generated_branches: usize,
         stop_reason: FreecellFindStopReason| FreecellSinglePlaythroughResult {
            line,
            expanded_states,
            generated_branches,
            elapsed_ms: started.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
            stop_reason,
        };

    let mut game = FreecellGame::new_with_seed_and_card_count(seed, card_count_mode);
    if game.is_won() {
        return finish(Some(Vec::new()), 0, 0, FreecellFindStopReason::Won);
    }

    let mut seen: HashSet<u64> = HashSet::new();
    let initial_hash = freecell_wand_state_hash(&game);
    seen.insert(initial_hash);
    let mut recent_hashes: VecDeque<u64> = VecDeque::new();
    recent_hashes.push_back(initial_hash);
    let mut line: Vec<FreecellPlannerAction> = Vec::new();
    let mut expanded_states = 0usize;
    let mut generated_branches = 0usize;

    for _ in 0..MAX_STEPS {
        if cancel.load(Ordering::Relaxed) {
            return finish(
                None,
                expanded_states,
                generated_branches,
                FreecellFindStopReason::Canceled,
            );
        }
        let current_hash = freecell_wand_state_hash(&game);
        let candidates = freecell_wand_candidates(&game);
        generated_branches = generated_branches.saturating_add(candidates.len());
        let Some(action) = freecell_select_best_action_from_deque_avoiding_seen(
            candidates,
            current_hash,
            &recent_hashes,
            &seen,
        ) else {
            return finish(
                None,
                expanded_states,
                generated_branches,
                FreecellFindStopReason::NoAction,
            );
        };
        if !apply_freecell_action(&mut game, action) {
            return finish(
                None,
                expanded_states,
                generated_branches,
                FreecellFindStopReason::InvalidMove,
            );
        }
        line.push(action);
        expanded_states = expanded_states.saturating_add(1);
        if game.is_won() {
            return finish(
                Some(line),
                expanded_states,
                generated_branches,
                FreecellFindStopReason::Won,
            );
        }
        let hash = freecell_wand_state_hash(&game);
        if !seen.insert(hash) {
            return finish(
                None,
                expanded_states,
                generated_branches,
                FreecellFindStopReason::RepeatState,
            );
        }
        recent_hashes.push_back(hash);
        if recent_hashes.len() > 96 {
            recent_hashes.pop_front();
        }
    }
    finish(
        None,
        expanded_states,
        generated_branches,
        FreecellFindStopReason::StepLimit,
    )
}

pub fn is_freecell_seed_winnable(
    seed: u64,
    card_count_mode: crate::game::FreecellCardCountMode,
    guided_budget: usize,
    exhaustive_budget: usize,
    cancel: &AtomicBool,
) -> Option<SeedWinnabilityCheckResult> {
    if cancel.load(Ordering::Relaxed) {
        return Some(SeedWinnabilityCheckResult {
            winnable: false,
            iterations: 0,
            moves_to_win: None,
            hit_state_limit: true,
            solver_line: None,
            hint_line: None,
            freecell_line: None,
            canceled: true,
        });
    }

    let start = FreecellGame::new_with_seed_and_card_count(seed, card_count_mode);
    if start.is_won() {
        return Some(SeedWinnabilityCheckResult {
            winnable: true,
            iterations: 1,
            moves_to_win: Some(0),
            hit_state_limit: false,
            solver_line: None,
            hint_line: None,
            freecell_line: Some(Vec::new()),
            canceled: false,
        });
    }

    let max_states = capped_freecell_seed_check_budget(guided_budget, exhaustive_budget);
    let mut visited: HashSet<FreecellGame> = HashSet::new();
    let mut frontier: BinaryHeap<FreecellGuidedNode> = BinaryHeap::new();
    let mut serial = 0_u64;
    let mut parent_by_hash: HashMap<u64, (Option<u64>, Option<FreecellPlannerAction>)> =
        HashMap::new();
    let start_hash = freecell_wand_state_hash(&start);
    parent_by_hash.insert(start_hash, (None, None));
    frontier.push(FreecellGuidedNode {
        priority: 0,
        serial,
        depth: 0,
        state: start,
    });

    while let Some(node) = frontier.pop() {
        if cancel.load(Ordering::Relaxed) {
            return Some(SeedWinnabilityCheckResult {
                winnable: false,
                iterations: visited.len(),
                moves_to_win: None,
                hit_state_limit: true,
                solver_line: None,
                hint_line: None,
                freecell_line: None,
                canceled: true,
            });
        }

        if !visited.insert(node.state.clone()) {
            continue;
        }
        if node.state.is_won() {
            let mut line_rev: Vec<FreecellPlannerAction> = Vec::new();
            let mut cursor = Some(freecell_wand_state_hash(&node.state));
            while let Some(hash) = cursor {
                let Some((parent_hash, action)) = parent_by_hash.get(&hash).copied() else {
                    break;
                };
                if let Some(action) = action {
                    line_rev.push(action);
                }
                cursor = parent_hash;
            }
            line_rev.reverse();
            return Some(SeedWinnabilityCheckResult {
                winnable: true,
                iterations: visited.len(),
                moves_to_win: Some(node.depth),
                hit_state_limit: false,
                solver_line: None,
                hint_line: None,
                freecell_line: Some(line_rev),
                canceled: false,
            });
        }
        if visited.len() >= max_states {
            return Some(SeedWinnabilityCheckResult {
                winnable: false,
                iterations: visited.len(),
                moves_to_win: None,
                hit_state_limit: true,
                solver_line: None,
                hint_line: None,
                freecell_line: None,
                canceled: false,
            });
        }

        let current_hash = freecell_wand_state_hash(&node.state);
        for (next, action, score) in freecell_wand_candidates(&node.state) {
            if visited.contains(&next) {
                continue;
            }
            serial = serial.wrapping_add(1);
            let next_hash = freecell_wand_state_hash(&next);
            parent_by_hash
                .entry(next_hash)
                .or_insert((Some(current_hash), Some(action)));
            frontier.push(FreecellGuidedNode {
                priority: score - i64::from(node.depth) * 2,
                serial,
                depth: node.depth + 1,
                state: next,
            });
        }
    }

    Some(SeedWinnabilityCheckResult {
        winnable: false,
        iterations: visited.len(),
        moves_to_win: None,
        hit_state_limit: false,
        solver_line: None,
        hint_line: None,
        freecell_line: None,
        canceled: false,
    })
}

pub fn find_winnable_freecell_seed_parallel(
    start_seed: u64,
    attempts: u32,
    guided_budget: usize,
    exhaustive_budget: usize,
    card_count_mode: crate::game::FreecellCardCountMode,
    cancel: Arc<AtomicBool>,
    progress_checked: Option<Arc<AtomicU32>>,
    progress_stats: Option<Arc<FreecellFindProgress>>,
) -> Option<(u64, u32, Vec<FreecellPlannerAction>)> {
    if attempts == 0 {
        return None;
    }

    const FREECELL_FIND_MAX_WORKERS: usize = 8;
    let worker_count = thread::available_parallelism()
        .map(|n| n.get())
        .map(|n| n.min(FREECELL_FIND_MAX_WORKERS))
        .unwrap_or(1)
        .min(attempts as usize)
        .max(1);

    let next_index = Arc::new(AtomicU32::new(0));
    let (sender, receiver) = mpsc::channel::<(u64, u32, Vec<FreecellPlannerAction>)>();
    let mut handles = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let next_index = Arc::clone(&next_index);
        let cancel = Arc::clone(&cancel);
        let sender = sender.clone();
        let progress_checked = progress_checked.clone();
        let progress_stats = progress_stats.clone();
        let handle = thread::spawn(move || loop {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let index = next_index.fetch_add(1, Ordering::Relaxed);
            if index >= attempts {
                break;
            }
            if let Some(progress) = &progress_checked {
                progress.fetch_max(index.saturating_add(1), Ordering::Relaxed);
            }

            let seed = start_seed.wrapping_add(u64::from(index));
            let _ = guided_budget;
            let _ = exhaustive_budget;
            let attempt = freecell_single_playthrough_line(seed, card_count_mode, cancel.as_ref());
            if let Some(progress) = &progress_stats {
                progress.store_attempt(
                    seed,
                    index.saturating_add(1),
                    attempt.expanded_states,
                    attempt.generated_branches,
                    attempt.elapsed_ms,
                    attempt.stop_reason,
                );
            }
            let Some(line) = attempt.line else {
                continue;
            };
            if !cancel.swap(true, Ordering::Relaxed) {
                let _ = sender.send((seed, index + 1, line));
            }
            break;
        });
        handles.push(handle);
    }

    drop(sender);
    let result = receiver.recv().ok();
    cancel.store(true, Ordering::Relaxed);
    for handle in handles {
        let _ = handle.join();
    }
    result
}

pub fn find_winnable_spider_seed_parallel(
    start_seed: u64,
    attempts: u32,
    guided_budget: usize,
    exhaustive_budget: usize,
    suit_mode: SpiderSuitMode,
    cancel: Arc<AtomicBool>,
) -> Option<(u64, u32, Vec<HintMove>)> {
    if attempts == 0 {
        return None;
    }

    let worker_count = thread::available_parallelism()
        .map(|n| n.get())
        .map(|n| n.min(4))
        .unwrap_or(1)
        .min(attempts as usize)
        .max(1);

    let next_index = Arc::new(AtomicU32::new(0));
    let (sender, receiver) = mpsc::channel::<(u64, u32, Vec<HintMove>)>();
    let mut handles = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let next_index = Arc::clone(&next_index);
        let cancel = Arc::clone(&cancel);
        let sender = sender.clone();
        let handle = thread::spawn(move || loop {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let index = next_index.fetch_add(1, Ordering::Relaxed);
            if index >= attempts {
                break;
            }

            let seed = start_seed.wrapping_add(u64::from(index));
            let Some(result) = is_spider_seed_winnable(
                seed,
                suit_mode,
                guided_budget,
                exhaustive_budget,
                cancel.as_ref(),
            ) else {
                break;
            };
            if result.canceled || !result.winnable {
                continue;
            }

            let line = result.hint_line.unwrap_or_default();
            if !cancel.swap(true, Ordering::Relaxed) {
                let _ = sender.send((seed, index + 1, line));
            }
            break;
        });
        handles.push(handle);
    }

    drop(sender);
    let result = receiver.recv().ok();
    cancel.store(true, Ordering::Relaxed);
    for handle in handles {
        let _ = handle.join();
    }
    result
}

pub fn find_winnable_seed_parallel(
    start_seed: u64,
    attempts: u32,
    max_states: usize,
    draw_mode: DrawMode,
    cancel: Arc<AtomicBool>,
) -> Option<(u64, u32, Vec<SolverMove>)> {
    if attempts == 0 {
        return None;
    }

    let worker_count = thread::available_parallelism()
        .map(|n| n.get())
        .map(|n| n.min(4))
        .unwrap_or(1)
        .min(attempts as usize)
        .max(1);

    let next_index = Arc::new(AtomicU32::new(0));
    let (sender, receiver) = mpsc::channel::<(u64, u32, Vec<SolverMove>)>();
    let mut handles = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let next_index = Arc::clone(&next_index);
        let cancel = Arc::clone(&cancel);
        let sender = sender.clone();
        let handle = thread::spawn(move || loop {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let index = next_index.fetch_add(1, Ordering::Relaxed);
            if index >= attempts {
                break;
            }

            let seed = start_seed.wrapping_add(u64::from(index));
            let mut game = KlondikeGame::new_with_seed(seed);
            game.set_draw_mode(draw_mode);
            let Some(winnable) = game.is_winnable_guided_cancelable(max_states, cancel.as_ref())
            else {
                break;
            };
            if !winnable {
                continue;
            }

            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let Some(line) = game.guided_winning_line_cancelable(max_states, cancel.as_ref())
            else {
                break;
            };
            let line = line.unwrap_or_default();
            if !cancel.swap(true, Ordering::Relaxed) {
                let _ = sender.send((seed, index + 1, line));
            }
            break;
        });
        handles.push(handle);
    }

    drop(sender);
    let result = receiver.recv().ok();
    cancel.store(true, Ordering::Relaxed);
    for handle in handles {
        let _ = handle.join();
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spider_seed_winnability_honors_cancel_flag() {
        let cancel = AtomicBool::new(true);
        let result = is_spider_seed_winnable(123, SpiderSuitMode::One, 100, 100, &cancel)
            .expect("result should always be returned");
        assert!(result.canceled);
        assert!(!result.winnable);
        assert!(result.hint_line.is_none());
        assert!(result.solver_line.is_none());
    }

    #[test]
    fn spider_seed_winnability_returns_spider_hint_line_shape() {
        let cancel = AtomicBool::new(false);
        let result = is_spider_seed_winnable(123, SpiderSuitMode::Four, 4, 4, &cancel)
            .expect("result should always be returned");
        assert!(result.solver_line.is_none());
        if let Some(line) = result.hint_line {
            assert_eq!(
                usize::try_from(result.moves_to_win.unwrap_or(0)).ok(),
                Some(line.len())
            );
        }
    }
}

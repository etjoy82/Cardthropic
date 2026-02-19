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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SpiderFindStopReason {
    None = 0,
    Won = 1,
    NoAction = 2,
    RepeatState = 3,
    StepLimit = 4,
    Canceled = 5,
    RepeatStateCycle = 6,
    RepeatStateStagnation = 7,
    PostStockStall = 8,
}

pub const SPIDER_SOLVER_STAGNATION_MOVE_LIMIT: usize = 100;
pub const SPIDER_SOLVER_REPEAT_SEQUENCE_MIN: usize = 3;
pub const SPIDER_SOLVER_REPEAT_SEQUENCE_MAX: usize = 5;
pub const SPIDER_SOLVER_REPEAT_SEQUENCE_LIMIT: usize = 3;
const SPIDER4_ENSEMBLE_VARIANTS: usize = 2;
const SPIDER1_ENSEMBLE_VARIANTS: usize = 3;

static SPIDER4_VARIANT_TRIALS: [AtomicU64; SPIDER4_ENSEMBLE_VARIANTS] =
    [const { AtomicU64::new(0) }; SPIDER4_ENSEMBLE_VARIANTS];
static SPIDER4_VARIANT_WINS: [AtomicU64; SPIDER4_ENSEMBLE_VARIANTS] =
    [const { AtomicU64::new(0) }; SPIDER4_ENSEMBLE_VARIANTS];
static SPIDER1_VARIANT_TRIALS: [AtomicU64; SPIDER1_ENSEMBLE_VARIANTS] =
    [const { AtomicU64::new(0) }; SPIDER1_ENSEMBLE_VARIANTS];
static SPIDER1_VARIANT_WINS: [AtomicU64; SPIDER1_ENSEMBLE_VARIANTS] =
    [const { AtomicU64::new(0) }; SPIDER1_ENSEMBLE_VARIANTS];

impl SpiderFindStopReason {
    fn from_code(code: u8) -> Self {
        match code {
            1 => Self::Won,
            2 => Self::NoAction,
            3 => Self::RepeatState,
            4 => Self::StepLimit,
            5 => Self::Canceled,
            6 => Self::RepeatStateCycle,
            7 => Self::RepeatStateStagnation,
            8 => Self::PostStockStall,
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
            Self::RepeatStateCycle => "repeat_state_cycle",
            Self::RepeatStateStagnation => "repeat_state_stagnation",
            Self::PostStockStall => "post_stock_stall",
        }
    }
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

#[derive(Debug)]
pub struct SpiderFindProgress {
    pub checked: AtomicU32,
    pub last_seed: AtomicU64,
    pub last_expanded_states: AtomicUsize,
    pub last_generated_branches: AtomicUsize,
    pub last_stop_reason: AtomicU8,
    pub last_completed_runs: AtomicUsize,
    pub last_stock_cards: AtomicUsize,
    pub last_face_down_cards: AtomicUsize,
    pub last_empty_cols: AtomicUsize,
    pub last_suited_edges: AtomicUsize,
    pub last_max_tail_run: AtomicUsize,
    pub last_cycle_period: AtomicUsize,
    pub last_cycle_blocks: AtomicUsize,
    pub last_draw_moves: AtomicUsize,
    pub last_tableau_moves: AtomicUsize,
    pub last_reveal_moves: AtomicUsize,
    pub last_peak_suited_edges: AtomicUsize,
    pub last_peak_tail_run: AtomicUsize,
    pub last_peak_empty_cols: AtomicUsize,
    pub last_empty_creates: AtomicUsize,
    pub last_adapt_events: AtomicUsize,
    pub best_peak_empty_cols: AtomicUsize,
    pub attempts_with_empty_create: AtomicU32,
}

impl Default for SpiderFindProgress {
    fn default() -> Self {
        Self {
            checked: AtomicU32::new(0),
            last_seed: AtomicU64::new(0),
            last_expanded_states: AtomicUsize::new(0),
            last_generated_branches: AtomicUsize::new(0),
            last_stop_reason: AtomicU8::new(SpiderFindStopReason::None as u8),
            last_completed_runs: AtomicUsize::new(0),
            last_stock_cards: AtomicUsize::new(0),
            last_face_down_cards: AtomicUsize::new(0),
            last_empty_cols: AtomicUsize::new(0),
            last_suited_edges: AtomicUsize::new(0),
            last_max_tail_run: AtomicUsize::new(0),
            last_cycle_period: AtomicUsize::new(0),
            last_cycle_blocks: AtomicUsize::new(0),
            last_draw_moves: AtomicUsize::new(0),
            last_tableau_moves: AtomicUsize::new(0),
            last_reveal_moves: AtomicUsize::new(0),
            last_peak_suited_edges: AtomicUsize::new(0),
            last_peak_tail_run: AtomicUsize::new(0),
            last_peak_empty_cols: AtomicUsize::new(0),
            last_empty_creates: AtomicUsize::new(0),
            last_adapt_events: AtomicUsize::new(0),
            best_peak_empty_cols: AtomicUsize::new(0),
            attempts_with_empty_create: AtomicU32::new(0),
        }
    }
}

impl SpiderFindProgress {
    fn store_attempt(
        &self,
        seed: u64,
        checked: u32,
        expanded_states: usize,
        generated_branches: usize,
        stop_reason: SpiderFindStopReason,
        completed_runs: usize,
        stock_cards: usize,
        face_down_cards: usize,
        empty_cols: usize,
        suited_edges: usize,
        max_tail_run: usize,
        cycle_period: usize,
        cycle_blocks: usize,
        draw_moves: usize,
        tableau_moves: usize,
        reveal_moves: usize,
        peak_suited_edges: usize,
        peak_tail_run: usize,
        peak_empty_cols: usize,
        empty_creates: usize,
        adapt_events: usize,
    ) {
        self.checked.fetch_max(checked, Ordering::Relaxed);
        self.last_seed.store(seed, Ordering::Relaxed);
        self.last_expanded_states
            .store(expanded_states, Ordering::Relaxed);
        self.last_generated_branches
            .store(generated_branches, Ordering::Relaxed);
        self.last_stop_reason
            .store(stop_reason as u8, Ordering::Relaxed);
        self.last_completed_runs
            .store(completed_runs, Ordering::Relaxed);
        self.last_stock_cards.store(stock_cards, Ordering::Relaxed);
        self.last_face_down_cards
            .store(face_down_cards, Ordering::Relaxed);
        self.last_empty_cols.store(empty_cols, Ordering::Relaxed);
        self.last_suited_edges
            .store(suited_edges, Ordering::Relaxed);
        self.last_max_tail_run
            .store(max_tail_run, Ordering::Relaxed);
        self.last_cycle_period
            .store(cycle_period, Ordering::Relaxed);
        self.last_cycle_blocks
            .store(cycle_blocks, Ordering::Relaxed);
        self.last_draw_moves.store(draw_moves, Ordering::Relaxed);
        self.last_tableau_moves
            .store(tableau_moves, Ordering::Relaxed);
        self.last_reveal_moves
            .store(reveal_moves, Ordering::Relaxed);
        self.last_peak_suited_edges
            .store(peak_suited_edges, Ordering::Relaxed);
        self.last_peak_tail_run
            .store(peak_tail_run, Ordering::Relaxed);
        self.last_peak_empty_cols
            .store(peak_empty_cols, Ordering::Relaxed);
        self.last_empty_creates
            .store(empty_creates, Ordering::Relaxed);
        self.last_adapt_events
            .store(adapt_events, Ordering::Relaxed);
        self.best_peak_empty_cols
            .fetch_max(peak_empty_cols, Ordering::Relaxed);
        if empty_creates > 0 {
            self.attempts_with_empty_create
                .fetch_add(1, Ordering::Relaxed);
        }
    }
}

pub fn freecell_find_stop_reason_label(code: u8) -> &'static str {
    FreecellFindStopReason::from_code(code).as_label()
}

pub fn spider_find_stop_reason_label(code: u8) -> &'static str {
    SpiderFindStopReason::from_code(code).as_label()
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

fn spider_empty_bridge_reserve(suit_mode: SpiderSuitMode) -> usize {
    match suit_mode {
        SpiderSuitMode::One => 4,
        SpiderSuitMode::Two => 6,
        SpiderSuitMode::Three => 8,
        SpiderSuitMode::Four => 12,
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

fn spider_hidden_count(game: &SpiderGame) -> usize {
    game.tableau()
        .iter()
        .map(|pile| pile.iter().filter(|card| !card.face_up).count())
        .sum()
}

fn spider_suited_desc_edges(game: &SpiderGame) -> usize {
    game.tableau()
        .iter()
        .map(|pile| {
            pile.windows(2)
                .filter(|pair| {
                    let a = pair[0];
                    let b = pair[1];
                    a.face_up && b.face_up && a.suit == b.suit && a.rank == b.rank + 1
                })
                .count()
        })
        .sum()
}

fn spider_max_face_up_run_len(game: &SpiderGame) -> usize {
    game.tableau()
        .iter()
        .map(|pile| {
            let mut best = 0usize;
            let mut cur = 0usize;
            for card in pile.iter().rev() {
                if card.face_up {
                    cur += 1;
                    best = best.max(cur);
                } else {
                    break;
                }
            }
            best
        })
        .max()
        .unwrap_or(0)
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
    has_tableau_moves: bool,
) -> i64 {
    let completed_delta = next
        .completed_runs()
        .saturating_sub(current.completed_runs()) as i64;
    let face_up_delta = spider_face_up_count(next) as i64 - spider_face_up_count(current) as i64;
    let empty_delta = spider_empty_col_count(next) as i64 - spider_empty_col_count(current) as i64;
    let hidden_reveal_delta =
        spider_hidden_count(current) as i64 - spider_hidden_count(next) as i64;
    let suited_edge_delta =
        spider_suited_desc_edges(next) as i64 - spider_suited_desc_edges(current) as i64;
    let run_len_delta =
        spider_max_face_up_run_len(next) as i64 - spider_max_face_up_run_len(current) as i64;

    let deal_unlock = !current.can_deal_from_stock() && next.can_deal_from_stock();
    let mut score = completed_delta * 120_000
        + face_up_delta * 220
        + empty_delta * 850
        + hidden_reveal_delta * 2_100
        + suited_edge_delta * 1_350
        + run_len_delta * 240
        + i64::from(deal_unlock) * 18_000;
    if let HintMove::TableauRunToTableau { src, start, dst } = hint_move {
        let source = &current.tableau()[src];
        if start > 0 {
            let above = source[start - 1];
            let moved = source[start];
            if above.face_up && moved.face_up && above.rank == moved.rank + 1 {
                score -= if above.suit == moved.suit {
                    2_600
                } else {
                    1_400
                };
            }
        }
        if let Some(dst_top) = current.tableau()[dst].last().copied() {
            let moved = source[start];
            if dst_top.face_up {
                if dst_top.rank == moved.rank + 1 {
                    score += if dst_top.suit == moved.suit {
                        1_800
                    } else {
                        700
                    };
                } else {
                    score -= 120;
                }
            }
        }
    }
    if matches!(hint_move, HintMove::Draw) {
        score -= if has_tableau_moves { 15_000 } else { 60 };
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
    let mut has_tableau_moves = false;
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
                has_tableau_moves = true;
                let hint_move = HintMove::TableauRunToTableau { src, start, dst };
                let transition =
                    spider_transition_score(game, &next, hint_move, parent_hash, has_tableau_moves);
                successors.push((next, hint_move, transition));
            }
        }
    }

    if game.can_deal_from_stock() {
        let mut next = game.clone();
        if next.deal_from_stock() {
            let hint_move = HintMove::Draw;
            let transition =
                spider_transition_score(game, &next, hint_move, parent_hash, has_tableau_moves);
            successors.push((next, hint_move, transition));
        }
    }

    successors
}

#[derive(Debug, Clone, Copy)]
pub struct SpiderSolverPolicy {
    pub recent_window: usize,
    pub top_width: usize,
    pub allow_revisit_fallback: bool,
    pub allow_draw: bool,
    pub empty_col_bonus: i64,
    pub empty_bridge_bonus: i64,
    pub empty_bridge_len_bonus: i64,
    pub empty_create_bonus: i64,
    pub pre_last_draw_empty_bonus: i64,
    pub pre_last_draw_clear_bonus: i64,
    pub no_empty_nonclear_penalty: i64,
    pub post_stock_flat_penalty: i64,
    pub require_post_stock_progress: bool,
    pub protect_last_empty_col: bool,
    pub last_empty_fill_penalty: i64,
}

impl SpiderSolverPolicy {
    pub fn winnability_default() -> Self {
        Self {
            recent_window: 12,
            top_width: 6,
            allow_revisit_fallback: true,
            allow_draw: true,
            empty_col_bonus: 0,
            empty_bridge_bonus: 0,
            empty_bridge_len_bonus: 0,
            empty_create_bonus: 0,
            pre_last_draw_empty_bonus: 0,
            pre_last_draw_clear_bonus: 0,
            no_empty_nonclear_penalty: 0,
            post_stock_flat_penalty: 0,
            require_post_stock_progress: false,
            protect_last_empty_col: false,
            last_empty_fill_penalty: 0,
        }
    }

    pub fn hint_default() -> Self {
        Self {
            recent_window: 12,
            top_width: 6,
            allow_revisit_fallback: true,
            allow_draw: true,
            empty_col_bonus: 0,
            empty_bridge_bonus: 0,
            empty_bridge_len_bonus: 0,
            empty_create_bonus: 0,
            pre_last_draw_empty_bonus: 0,
            pre_last_draw_clear_bonus: 0,
            no_empty_nonclear_penalty: 0,
            post_stock_flat_penalty: 0,
            require_post_stock_progress: false,
            protect_last_empty_col: false,
            last_empty_fill_penalty: 0,
        }
    }
}

#[derive(Debug)]
pub struct SpiderSolverDecision {
    pub next_state: SpiderGame,
    pub hint_move: HintMove,
    pub next_hash: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpiderSolverNoMoveReason {
    NoAction,
    LoopFiltered,
}

pub fn spider_solver_move_signature(hint_move: HintMove) -> String {
    match hint_move {
        HintMove::WasteToFoundation => "wf".to_string(),
        HintMove::TableauTopToFoundation { src } => format!("t{src}f"),
        HintMove::WasteToTableau { dst } => format!("w{dst}"),
        HintMove::TableauRunToTableau { src, start, dst } => format!("t{src}:{start}>{dst}"),
        HintMove::Draw => "d".to_string(),
    }
}

fn repeated_tail_block_count(signatures: &[String], period: usize) -> usize {
    if period == 0 || signatures.len() < period {
        return 0;
    }
    let mut blocks = 1usize;
    let mut cursor = signatures.len();
    while cursor >= period * 2
        && signatures[cursor - period..cursor] == signatures[cursor - period * 2..cursor - period]
    {
        blocks = blocks.saturating_add(1);
        cursor -= period;
    }
    blocks
}

fn spider_solver_detect_repeating_tail_cycle_with_limit(
    signatures: &[String],
    repeat_limit: usize,
) -> Option<(usize, usize)> {
    let mut best: Option<(usize, usize)> = None;
    for period in SPIDER_SOLVER_REPEAT_SEQUENCE_MIN..=SPIDER_SOLVER_REPEAT_SEQUENCE_MAX {
        let blocks = repeated_tail_block_count(signatures, period);
        if blocks > repeat_limit {
            match best {
                None => best = Some((period, blocks)),
                Some((_best_period, best_blocks)) if blocks > best_blocks => {
                    best = Some((period, blocks));
                }
                _ => {}
            }
        }
    }
    best
}

pub fn spider_solver_detect_repeating_tail_cycle(signatures: &[String]) -> Option<(usize, usize)> {
    spider_solver_detect_repeating_tail_cycle_with_limit(
        signatures,
        SPIDER_SOLVER_REPEAT_SEQUENCE_LIMIT,
    )
}

pub fn spider_solver_state_hash(game: &SpiderGame) -> u64 {
    spider_state_hash(game)
}

pub fn spider_solver_ranked_candidates(
    game: &SpiderGame,
    suit_mode: SpiderSuitMode,
    parent_hash: Option<u64>,
) -> Vec<(SpiderGame, HintMove, i64)> {
    let mut successors = spider_successors_guided(game, parent_hash);
    if successors.is_empty() {
        return successors;
    }
    let limit = spider_successor_limit(suit_mode);
    if successors.len() <= limit {
        successors.sort_by(|a, b| b.2.cmp(&a.2));
        return successors;
    }

    let mut empty_bridge: Vec<(SpiderGame, HintMove, i64)> = Vec::new();
    let mut regular: Vec<(SpiderGame, HintMove, i64)> = Vec::new();
    for candidate in successors {
        let is_empty_dst = match candidate.1 {
            HintMove::TableauRunToTableau { dst, .. } => game.tableau()[dst].is_empty(),
            _ => false,
        };
        if is_empty_dst {
            empty_bridge.push(candidate);
        } else {
            regular.push(candidate);
        }
    }
    empty_bridge.sort_by(|a, b| b.2.cmp(&a.2));
    regular.sort_by(|a, b| b.2.cmp(&a.2));

    let reserve = spider_empty_bridge_reserve(suit_mode)
        .min(limit)
        .min(empty_bridge.len());
    let mut selected: Vec<(SpiderGame, HintMove, i64)> = Vec::with_capacity(limit);
    selected.extend(empty_bridge.into_iter().take(reserve));

    let remaining = limit.saturating_sub(selected.len());
    selected.extend(regular.into_iter().take(remaining));
    selected.sort_by(|a, b| b.2.cmp(&a.2));
    selected.truncate(limit);
    selected
}

pub fn spider_solver_choose_move<F>(
    game: &SpiderGame,
    suit_mode: SpiderSuitMode,
    parent_hash: Option<u64>,
    seen_hashes: &HashSet<u64>,
    recent_hashes: &VecDeque<u64>,
    selection_salt: u64,
    policy: SpiderSolverPolicy,
    mut allow_move: F,
) -> Result<SpiderSolverDecision, SpiderSolverNoMoveReason>
where
    F: FnMut(HintMove) -> bool,
{
    #[inline]
    fn structural_tuple(game: &SpiderGame) -> (usize, usize, usize, usize) {
        (
            game.completed_runs(),
            spider_empty_col_count(game),
            spider_suited_desc_edges(game),
            spider_max_face_up_run_len(game),
        )
    }

    let current_hash = spider_state_hash(game);
    let (cur_runs, cur_empty, cur_edges, cur_tail) = structural_tuple(game);
    let mut primary: Vec<(SpiderGame, HintMove, u64, i64)> = Vec::new();
    let mut secondary: Vec<(SpiderGame, HintMove, u64, i64)> = Vec::new();
    let mut tertiary: Vec<(SpiderGame, HintMove, u64, i64)> = Vec::new();
    let mut had_successor = false;

    for (next, hint_move, base_score) in
        spider_solver_ranked_candidates(game, suit_mode, parent_hash)
    {
        if matches!(hint_move, HintMove::Draw) && !policy.allow_draw {
            continue;
        }
        if !allow_move(hint_move) {
            continue;
        }
        had_successor = true;
        let next_hash = spider_state_hash(&next);
        if next_hash == current_hash {
            continue;
        }
        if recent_hashes
            .iter()
            .rev()
            .take(policy.recent_window)
            .any(|hash| *hash == next_hash)
        {
            continue;
        }
        let (next_runs, next_empty, next_edges, next_tail) = structural_tuple(&next);
        let mut adjusted_score =
            base_score + ((next_empty as i64) - (cur_empty as i64)) * policy.empty_col_bonus;
        if let HintMove::TableauRunToTableau { src, start, dst } = hint_move {
            if cur_empty == 0 && start > 0 {
                adjusted_score -= policy.no_empty_nonclear_penalty;
            }
            if start == 0 {
                // Clearing a source column creates strategic capacity, especially before last deal.
                adjusted_score += policy.empty_create_bonus;
                if game.stock_len() == 10 {
                    adjusted_score += policy.pre_last_draw_clear_bonus;
                }
            }
            if game.tableau()[dst].is_empty() {
                let moved_len = game.tableau()[src].len().saturating_sub(start) as i64;
                adjusted_score += policy.empty_bridge_bonus;
                adjusted_score += moved_len * policy.empty_bridge_len_bonus;
                if game.stock_len() == 10 {
                    adjusted_score += policy.pre_last_draw_empty_bonus;
                }
            }
        }
        let post_stock = game.stock_len() == 0;
        let structural_progress = next_runs > cur_runs
            || next_empty > cur_empty
            || next_edges > cur_edges
            || next_tail > cur_tail;
        let last_empty_filled = policy.protect_last_empty_col
            && post_stock
            && cur_empty == 1
            && next_empty == 0
            && matches!(hint_move, HintMove::TableauRunToTableau { .. });
        if last_empty_filled {
            adjusted_score -= policy.last_empty_fill_penalty;
        }
        let gated_post_stock_flat = post_stock
            && !structural_progress
            && policy.post_stock_flat_penalty > 0
            && matches!(hint_move, HintMove::TableauRunToTableau { .. });
        if gated_post_stock_flat {
            adjusted_score -= policy.post_stock_flat_penalty;
        }
        let bucket =
            if policy.require_post_stock_progress && (gated_post_stock_flat || last_empty_filled) {
                &mut tertiary
            } else if !seen_hashes.contains(&next_hash) {
                &mut primary
            } else {
                &mut secondary
            };
        bucket.push((next, hint_move, next_hash, adjusted_score));
    }

    let sort_desc = |items: &mut Vec<(SpiderGame, HintMove, u64, i64)>| {
        items.sort_by(|a, b| b.3.cmp(&a.3));
    };
    sort_desc(&mut primary);
    sort_desc(&mut secondary);
    sort_desc(&mut tertiary);

    let pick = |items: &mut Vec<(SpiderGame, HintMove, u64, i64)>| -> Option<SpiderSolverDecision> {
        if items.is_empty() {
            return None;
        }
        let width = items.len().min(policy.top_width.max(1));
        let idx = usize::try_from(selection_salt % (width as u64)).unwrap_or(0);
        let (next_state, hint_move, next_hash, _score) = items.swap_remove(idx);
        Some(SpiderSolverDecision {
            next_state,
            hint_move,
            next_hash,
        })
    };

    if let Some(choice) = pick(&mut primary) {
        return Ok(choice);
    }
    if policy.allow_revisit_fallback {
        if let Some(choice) = pick(&mut secondary) {
            return Ok(choice);
        }
        if let Some(choice) = pick(&mut tertiary) {
            return Ok(choice);
        }
    }
    Err(if had_successor {
        SpiderSolverNoMoveReason::LoopFiltered
    } else {
        SpiderSolverNoMoveReason::NoAction
    })
}

fn capped_spider_find_step_budget(
    suit_mode: SpiderSuitMode,
    guided_budget: usize,
    exhaustive_budget: usize,
) -> usize {
    let requested = guided_budget.saturating_add(exhaustive_budget);
    let cap = match suit_mode {
        SpiderSuitMode::One => 512,
        SpiderSuitMode::Two => 256,
        SpiderSuitMode::Three => 224,
        SpiderSuitMode::Four => 640,
    };
    requested.clamp(64, cap)
}

fn spider_find_stagnation_move_limit(suit_mode: SpiderSuitMode) -> usize {
    match suit_mode {
        SpiderSuitMode::Four => 220,
        _ => SPIDER_SOLVER_STAGNATION_MOVE_LIMIT,
    }
}

fn spider_find_repeat_sequence_limit(suit_mode: SpiderSuitMode) -> usize {
    match suit_mode {
        SpiderSuitMode::Four => 5,
        _ => SPIDER_SOLVER_REPEAT_SEQUENCE_LIMIT,
    }
}

fn spider_find_policy(suit_mode: SpiderSuitMode) -> SpiderSolverPolicy {
    let mut policy = SpiderSolverPolicy::winnability_default();
    if suit_mode == SpiderSuitMode::One {
        policy.top_width = 12;
        policy.recent_window = 10;
        policy.empty_col_bonus = 800;
        policy.empty_bridge_bonus = 700;
        policy.empty_bridge_len_bonus = 80;
        policy.empty_create_bonus = 3_200;
        policy.pre_last_draw_empty_bonus = 1_200;
        policy.pre_last_draw_clear_bonus = 1_400;
        policy.no_empty_nonclear_penalty = 600;
        policy.post_stock_flat_penalty = 2_200;
        policy.require_post_stock_progress = false;
        policy.protect_last_empty_col = true;
        policy.last_empty_fill_penalty = 4_000;
    } else if suit_mode == SpiderSuitMode::Four {
        policy.top_width = 10;
        policy.recent_window = 16;
        policy.empty_col_bonus = 2_600;
        policy.empty_bridge_bonus = 1_800;
        policy.empty_bridge_len_bonus = 130;
        policy.empty_create_bonus = 9_000;
        policy.pre_last_draw_empty_bonus = 2_600;
        policy.pre_last_draw_clear_bonus = 2_800;
        policy.no_empty_nonclear_penalty = 2_000;
        policy.post_stock_flat_penalty = 12_000;
        policy.require_post_stock_progress = true;
        policy.protect_last_empty_col = true;
        policy.last_empty_fill_penalty = 14_000;
    }
    policy
}

fn spider_has_reveal_ready_move(game: &SpiderGame) -> bool {
    let hidden_before = spider_hidden_count(game);
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
                if next.move_run(src, start, dst) && spider_hidden_count(&next) < hidden_before {
                    return true;
                }
            }
        }
    }
    false
}

fn spider_find_post_stock_stall_limit(suit_mode: SpiderSuitMode) -> usize {
    match suit_mode {
        SpiderSuitMode::Four => 48,
        SpiderSuitMode::Three => 40,
        SpiderSuitMode::Two => 36,
        SpiderSuitMode::One => 32,
    }
}

pub fn spider_find_step_budget(
    suit_mode: SpiderSuitMode,
    guided_budget: usize,
    exhaustive_budget: usize,
) -> usize {
    capped_spider_find_step_budget(suit_mode, guided_budget, exhaustive_budget)
}

#[derive(Debug, Clone)]
struct SpiderSinglePlaythroughResult {
    line: Option<Vec<HintMove>>,
    expanded_states: usize,
    generated_branches: usize,
    stop_reason: SpiderFindStopReason,
    completed_runs: usize,
    stock_cards: usize,
    face_down_cards: usize,
    empty_cols: usize,
    suited_edges: usize,
    max_tail_run: usize,
    cycle_period: usize,
    cycle_blocks: usize,
    draw_moves: usize,
    tableau_moves: usize,
    reveal_moves: usize,
    peak_suited_edges: usize,
    peak_tail_run: usize,
    peak_empty_cols: usize,
    empty_creates: usize,
    adapt_events: usize,
}

fn spider_single_playthrough_line(
    seed: u64,
    suit_mode: SpiderSuitMode,
    guided_budget: usize,
    exhaustive_budget: usize,
    cancel: &AtomicBool,
) -> SpiderSinglePlaythroughResult {
    #[inline]
    fn next_u64(state: &mut u64) -> u64 {
        // xorshift64* for deterministic low-cost rollout diversification.
        *state ^= *state >> 12;
        *state ^= *state << 25;
        *state ^= *state >> 27;
        state.wrapping_mul(0x2545F4914F6CDD1D)
    }

    fn attempt_with_salt(
        seed: u64,
        suit_mode: SpiderSuitMode,
        max_steps: usize,
        stagnation_limit: usize,
        repeat_limit: usize,
        policy: SpiderSolverPolicy,
        salt: usize,
        cancel: &AtomicBool,
    ) -> SpiderSinglePlaythroughResult {
        let finish = |line: Option<Vec<HintMove>>,
                      expanded_states: usize,
                      generated_branches: usize,
                      stop_reason: SpiderFindStopReason,
                      game: &SpiderGame,
                      recent_move_signatures: &[String],
                      draw_moves: usize,
                      tableau_moves: usize,
                      reveal_moves: usize,
                      peak_suited_edges: usize,
                      peak_tail_run: usize,
                      peak_empty_cols: usize,
                      empty_creates: usize,
                      adapt_events: usize|
         -> SpiderSinglePlaythroughResult {
            let cycle = spider_solver_detect_repeating_tail_cycle_with_limit(
                recent_move_signatures,
                repeat_limit,
            );
            SpiderSinglePlaythroughResult {
                line,
                expanded_states,
                generated_branches,
                stop_reason,
                completed_runs: game.completed_runs(),
                stock_cards: game.stock_len(),
                face_down_cards: spider_hidden_count(game),
                empty_cols: spider_empty_col_count(game),
                suited_edges: spider_suited_desc_edges(game),
                max_tail_run: spider_max_face_up_run_len(game),
                cycle_period: cycle.map_or(0, |(p, _)| p),
                cycle_blocks: cycle.map_or(0, |(_, b)| b),
                draw_moves,
                tableau_moves,
                reveal_moves,
                peak_suited_edges,
                peak_tail_run,
                peak_empty_cols,
                empty_creates,
                adapt_events,
            }
        };

        let mut game = SpiderGame::new_with_seed_and_mode(seed, suit_mode);
        if game.is_won() {
            return finish(
                Some(Vec::new()),
                0,
                0,
                SpiderFindStopReason::Won,
                &game,
                &[],
                0,
                0,
                0,
                spider_suited_desc_edges(&game),
                spider_max_face_up_run_len(&game),
                spider_empty_col_count(&game),
                0,
                0,
            );
        }

        let mut seen_hashes: HashSet<u64> = HashSet::new();
        let mut recent_hashes: VecDeque<u64> = VecDeque::new();
        let start_hash = spider_state_hash(&game);
        seen_hashes.insert(start_hash);
        recent_hashes.push_back(start_hash);
        let mut parent_hash: Option<u64> = None;
        let mut line: Vec<HintMove> = Vec::new();
        let mut expanded_states = 0usize;
        let mut generated_branches = 0usize;
        let mut rng_state = seed ^ ((salt as u64) << 32) ^ 0x9E37_79B9_7F4A_7C15;
        let mut best_completed_runs = game.completed_runs();
        let mut best_hidden_count = spider_hidden_count(&game);
        let mut best_empty_cols = spider_empty_col_count(&game);
        let mut best_suited_edges = spider_suited_desc_edges(&game);
        let mut best_tail_run = spider_max_face_up_run_len(&game);
        let mut stagnation_steps = 0usize;
        let mut recent_move_signatures: Vec<String> = Vec::new();
        let mut draw_moves = 0usize;
        let mut tableau_moves = 0usize;
        let mut reveal_moves = 0usize;
        let mut peak_suited_edges = spider_suited_desc_edges(&game);
        let mut peak_tail_run = spider_max_face_up_run_len(&game);
        let mut peak_empty_cols = spider_empty_col_count(&game);
        let mut empty_creates = 0usize;
        let mut adapt_events = 0usize;
        let mut adaptive_policy = policy;
        let post_stock_stall_limit = spider_find_post_stock_stall_limit(suit_mode);
        let mut last_deal_blocked_steps = 0usize;
        let mut no_empty_drought_steps = 0usize;
        let mut post_stock_tracking_started = false;
        let mut post_stock_no_reveal_steps = 0usize;
        let mut post_stock_no_structure_steps = 0usize;
        let mut post_stock_best_runs = 0usize;
        let mut post_stock_best_suited_edges = 0usize;
        let mut post_stock_best_tail_run = 0usize;

        for _ in 0..max_steps {
            if cancel.load(Ordering::Relaxed) {
                return finish(
                    None,
                    expanded_states,
                    generated_branches,
                    SpiderFindStopReason::Canceled,
                    &game,
                    &recent_move_signatures,
                    draw_moves,
                    tableau_moves,
                    reveal_moves,
                    peak_suited_edges,
                    peak_tail_run,
                    peak_empty_cols,
                    empty_creates,
                    adapt_events,
                );
            }
            let current_hash = spider_state_hash(&game);
            let branch_count = spider_solver_ranked_candidates(&game, suit_mode, parent_hash).len();
            generated_branches = generated_branches.saturating_add(branch_count);
            let selection_salt = next_u64(&mut rng_state);
            let last_deal_candidate = game.stock_len() == 10;
            let last_deal_ready = spider_empty_col_count(&game) > 0
                || spider_has_reveal_ready_move(&game)
                || spider_suited_desc_edges(&game) >= 12
                || spider_max_face_up_run_len(&game) >= 14;
            if suit_mode == SpiderSuitMode::Four && spider_empty_col_count(&game) == 0 {
                no_empty_drought_steps = no_empty_drought_steps.saturating_add(1);
            } else {
                no_empty_drought_steps = 0;
            }
            if suit_mode == SpiderSuitMode::Four && last_deal_candidate && !last_deal_ready {
                last_deal_blocked_steps = last_deal_blocked_steps.saturating_add(1);
            } else {
                last_deal_blocked_steps = 0;
            }
            if suit_mode == SpiderSuitMode::Four
                && stagnation_steps > 0
                && stagnation_steps.is_multiple_of(16)
            {
                adaptive_policy.empty_col_bonus += 500;
                adaptive_policy.empty_bridge_bonus += 300;
                adaptive_policy.empty_create_bonus += 900;
                adaptive_policy.pre_last_draw_empty_bonus += 300;
                adaptive_policy.pre_last_draw_clear_bonus += 280;
                adaptive_policy.no_empty_nonclear_penalty += 250;
                adaptive_policy.top_width = adaptive_policy.top_width.saturating_add(1).min(14);
                adapt_events = adapt_events.saturating_add(1);
            }
            let no_empty_cols = spider_empty_col_count(&game) == 0;
            let post_stock_rescue = suit_mode == SpiderSuitMode::Four
                && game.stock_len() == 0
                && (post_stock_no_reveal_steps >= post_stock_stall_limit / 2
                    || post_stock_no_structure_steps >= post_stock_stall_limit / 2);
            let force_source_clear = suit_mode == SpiderSuitMode::Four
                && no_empty_cols
                && ((last_deal_candidate && last_deal_blocked_steps >= 6)
                    || no_empty_drought_steps >= 14
                    || (game.stock_len() == 0 && no_empty_drought_steps >= 8)
                    || post_stock_rescue);
            if force_source_clear {
                adapt_events = adapt_events.saturating_add(1);
            }
            let mut force_policy = adaptive_policy;
            if force_source_clear {
                force_policy.recent_window = force_policy.recent_window.min(6);
                force_policy.no_empty_nonclear_penalty =
                    force_policy.no_empty_nonclear_penalty.saturating_mul(2);
            }
            if post_stock_rescue {
                // Late-game recovery mode: prioritize creating workspace over local structure greed.
                force_policy.recent_window = force_policy.recent_window.min(4);
                force_policy.top_width = force_policy.top_width.max(12);
                force_policy.require_post_stock_progress = false;
                force_policy.post_stock_flat_penalty = force_policy.post_stock_flat_penalty / 2;
                force_policy.empty_create_bonus += 3_000;
                force_policy.empty_col_bonus += 1_000;
                force_policy.no_empty_nonclear_penalty += 3_000;
                adapt_events = adapt_events.saturating_add(1);
            }
            let standard_allow = |hint_move: HintMove| {
                !(suit_mode == SpiderSuitMode::Four
                    && last_deal_candidate
                    && matches!(hint_move, HintMove::Draw)
                    && !last_deal_ready
                    && last_deal_blocked_steps < 20)
            };
            let mut decision = if force_source_clear {
                spider_solver_choose_move(
                    &game,
                    suit_mode,
                    parent_hash,
                    &seen_hashes,
                    &recent_hashes,
                    selection_salt,
                    force_policy,
                    |hint_move| match hint_move {
                        HintMove::TableauRunToTableau { start, .. } => start == 0,
                        HintMove::Draw => standard_allow(hint_move),
                        _ => true,
                    },
                )
            } else {
                spider_solver_choose_move(
                    &game,
                    suit_mode,
                    parent_hash,
                    &seen_hashes,
                    &recent_hashes,
                    selection_salt,
                    force_policy,
                    standard_allow,
                )
            };
            if decision.is_err() && force_source_clear {
                decision = spider_solver_choose_move(
                    &game,
                    suit_mode,
                    parent_hash,
                    &seen_hashes,
                    &recent_hashes,
                    selection_salt,
                    adaptive_policy,
                    standard_allow,
                );
            }
            if decision.is_err()
                && suit_mode == SpiderSuitMode::Four
                && last_deal_candidate
                && !last_deal_ready
            {
                // Last fallback: if setup remains blocked, allow any move including draw.
                decision = spider_solver_choose_move(
                    &game,
                    suit_mode,
                    parent_hash,
                    &seen_hashes,
                    &recent_hashes,
                    selection_salt,
                    adaptive_policy,
                    |_| true,
                );
            }
            if decision.is_err() && post_stock_rescue {
                decision = spider_solver_choose_move(
                    &game,
                    suit_mode,
                    parent_hash,
                    &seen_hashes,
                    &recent_hashes,
                    selection_salt,
                    force_policy,
                    |_| true,
                );
            }

            let (next, hint_move, next_hash) = match decision {
                Ok(SpiderSolverDecision {
                    next_state,
                    hint_move,
                    next_hash,
                }) => (next_state, hint_move, next_hash),
                Err(reason) => {
                    return finish(
                        None,
                        expanded_states,
                        generated_branches,
                        if reason == SpiderSolverNoMoveReason::LoopFiltered {
                            SpiderFindStopReason::RepeatStateCycle
                        } else {
                            SpiderFindStopReason::NoAction
                        },
                        &game,
                        &recent_move_signatures,
                        draw_moves,
                        tableau_moves,
                        reveal_moves,
                        peak_suited_edges,
                        peak_tail_run,
                        peak_empty_cols,
                        empty_creates,
                        adapt_events,
                    );
                }
            };

            line.push(hint_move);
            if matches!(hint_move, HintMove::Draw) {
                draw_moves = draw_moves.saturating_add(1);
            } else {
                tableau_moves = tableau_moves.saturating_add(1);
            }
            let hidden_before = spider_hidden_count(&game);
            recent_move_signatures.push(spider_solver_move_signature(hint_move));
            if spider_solver_detect_repeating_tail_cycle_with_limit(
                &recent_move_signatures,
                repeat_limit,
            )
            .is_some()
            {
                return finish(
                    None,
                    expanded_states,
                    generated_branches,
                    SpiderFindStopReason::RepeatStateCycle,
                    &game,
                    &recent_move_signatures,
                    draw_moves,
                    tableau_moves,
                    reveal_moves,
                    peak_suited_edges,
                    peak_tail_run,
                    peak_empty_cols,
                    empty_creates,
                    adapt_events,
                );
            }
            expanded_states = expanded_states.saturating_add(1);
            let empty_before = spider_empty_col_count(&game);
            game = next;
            let hidden_after = spider_hidden_count(&game);
            let empty_after = spider_empty_col_count(&game);
            if empty_after > empty_before {
                empty_creates = empty_creates.saturating_add(1);
                no_empty_drought_steps = 0;
            }
            if hidden_after < hidden_before {
                reveal_moves = reveal_moves.saturating_add(1);
            }
            let suited_edges_now = spider_suited_desc_edges(&game);
            let tail_run_now = spider_max_face_up_run_len(&game);
            peak_suited_edges = peak_suited_edges.max(suited_edges_now);
            peak_tail_run = peak_tail_run.max(tail_run_now);
            peak_empty_cols = peak_empty_cols.max(empty_after);
            if game.is_won() {
                return finish(
                    Some(line),
                    expanded_states,
                    generated_branches,
                    SpiderFindStopReason::Won,
                    &game,
                    &recent_move_signatures,
                    draw_moves,
                    tableau_moves,
                    reveal_moves,
                    peak_suited_edges,
                    peak_tail_run,
                    peak_empty_cols,
                    empty_creates,
                    adapt_events,
                );
            }
            let completed_runs = game.completed_runs();
            let structural_progress = completed_runs > best_completed_runs
                || hidden_after < best_hidden_count
                || empty_after > best_empty_cols
                || suited_edges_now > best_suited_edges
                || tail_run_now > best_tail_run;
            if structural_progress {
                best_completed_runs = best_completed_runs.max(completed_runs);
                best_hidden_count = best_hidden_count.min(hidden_after);
                best_empty_cols = best_empty_cols.max(empty_after);
                best_suited_edges = best_suited_edges.max(suited_edges_now);
                best_tail_run = best_tail_run.max(tail_run_now);
                stagnation_steps = 0;
            } else {
                stagnation_steps = stagnation_steps.saturating_add(1);
                if stagnation_steps >= stagnation_limit {
                    return finish(
                        None,
                        expanded_states,
                        generated_branches,
                        SpiderFindStopReason::RepeatStateStagnation,
                        &game,
                        &recent_move_signatures,
                        draw_moves,
                        tableau_moves,
                        reveal_moves,
                        peak_suited_edges,
                        peak_tail_run,
                        peak_empty_cols,
                        empty_creates,
                        adapt_events,
                    );
                }
            }
            if game.stock_len() == 0 {
                if !post_stock_tracking_started {
                    post_stock_tracking_started = true;
                    post_stock_best_runs = completed_runs;
                    post_stock_best_suited_edges = suited_edges_now;
                    post_stock_best_tail_run = tail_run_now;
                }
                let revealed_now = hidden_after < hidden_before;
                if revealed_now {
                    post_stock_no_reveal_steps = 0;
                } else {
                    post_stock_no_reveal_steps = post_stock_no_reveal_steps.saturating_add(1);
                }
                let structure_improved = completed_runs > post_stock_best_runs
                    || suited_edges_now > post_stock_best_suited_edges
                    || tail_run_now > post_stock_best_tail_run;
                if structure_improved {
                    post_stock_best_runs = post_stock_best_runs.max(completed_runs);
                    post_stock_best_suited_edges =
                        post_stock_best_suited_edges.max(suited_edges_now);
                    post_stock_best_tail_run = post_stock_best_tail_run.max(tail_run_now);
                    post_stock_no_structure_steps = 0;
                } else {
                    post_stock_no_structure_steps = post_stock_no_structure_steps.saturating_add(1);
                }
                if post_stock_no_reveal_steps >= post_stock_stall_limit
                    && post_stock_no_structure_steps >= post_stock_stall_limit
                {
                    return finish(
                        None,
                        expanded_states,
                        generated_branches,
                        SpiderFindStopReason::PostStockStall,
                        &game,
                        &recent_move_signatures,
                        draw_moves,
                        tableau_moves,
                        reveal_moves,
                        peak_suited_edges,
                        peak_tail_run,
                        peak_empty_cols,
                        empty_creates,
                        adapt_events,
                    );
                }
            }

            seen_hashes.insert(next_hash);
            recent_hashes.push_back(next_hash);
            if recent_hashes.len() > 96 {
                recent_hashes.pop_front();
            }
            parent_hash = Some(current_hash);
        }
        finish(
            None,
            expanded_states,
            generated_branches,
            SpiderFindStopReason::StepLimit,
            &game,
            &recent_move_signatures,
            draw_moves,
            tableau_moves,
            reveal_moves,
            peak_suited_edges,
            peak_tail_run,
            peak_empty_cols,
            empty_creates,
            adapt_events,
        )
    }

    let finish = |line: Option<Vec<HintMove>>,
                  expanded_states: usize,
                  generated_branches: usize,
                  stop_reason: SpiderFindStopReason,
                  completed_runs: usize,
                  stock_cards: usize,
                  face_down_cards: usize,
                  empty_cols: usize,
                  suited_edges: usize,
                  max_tail_run: usize,
                  cycle_period: usize,
                  cycle_blocks: usize,
                  draw_moves: usize,
                  tableau_moves: usize,
                  reveal_moves: usize,
                  peak_suited_edges: usize,
                  peak_tail_run: usize,
                  peak_empty_cols: usize,
                  empty_creates: usize,
                  adapt_events: usize| SpiderSinglePlaythroughResult {
        line,
        expanded_states,
        generated_branches,
        stop_reason,
        completed_runs,
        stock_cards,
        face_down_cards,
        empty_cols,
        suited_edges,
        max_tail_run,
        cycle_period,
        cycle_blocks,
        draw_moves,
        tableau_moves,
        reveal_moves,
        peak_suited_edges,
        peak_tail_run,
        peak_empty_cols,
        empty_creates,
        adapt_events,
    };

    // One-pass by design: a single continuous wand-style playthrough per seed.
    let max_steps =
        capped_spider_find_step_budget(suit_mode, guided_budget, exhaustive_budget).max(1);
    let stagnation_limit = spider_find_stagnation_move_limit(suit_mode);
    let repeat_limit = spider_find_repeat_sequence_limit(suit_mode);
    let policy = spider_find_policy(suit_mode);
    let primary = attempt_with_salt(
        seed,
        suit_mode,
        max_steps,
        stagnation_limit,
        repeat_limit,
        policy,
        0,
        cancel,
    );
    if primary.stop_reason == SpiderFindStopReason::Canceled {
        return finish(
            None,
            primary.expanded_states,
            primary.generated_branches,
            SpiderFindStopReason::Canceled,
            primary.completed_runs,
            primary.stock_cards,
            primary.face_down_cards,
            primary.empty_cols,
            primary.suited_edges,
            primary.max_tail_run,
            primary.cycle_period,
            primary.cycle_blocks,
            primary.draw_moves,
            primary.tableau_moves,
            primary.reveal_moves,
            primary.peak_suited_edges,
            primary.peak_tail_run,
            primary.peak_empty_cols,
            primary.empty_creates,
            primary.adapt_events,
        );
    }
    if let Some(line) = primary.line.clone() {
        return finish(
            Some(line),
            primary.expanded_states,
            primary.generated_branches,
            SpiderFindStopReason::Won,
            primary.completed_runs,
            primary.stock_cards,
            primary.face_down_cards,
            primary.empty_cols,
            primary.suited_edges,
            primary.max_tail_run,
            primary.cycle_period,
            primary.cycle_blocks,
            primary.draw_moves,
            primary.tableau_moves,
            primary.reveal_moves,
            primary.peak_suited_edges,
            primary.peak_tail_run,
            primary.peak_empty_cols,
            primary.empty_creates,
            primary.adapt_events,
        );
    }

    let mut best_attempt = primary;
    if matches!(suit_mode, SpiderSuitMode::One | SpiderSuitMode::Four)
        && !cancel.load(Ordering::Relaxed)
    {
        let should_ensemble = matches!(
            best_attempt.stop_reason,
            SpiderFindStopReason::PostStockStall
                | SpiderFindStopReason::RepeatStateCycle
                | SpiderFindStopReason::RepeatStateStagnation
                | SpiderFindStopReason::NoAction
        );
        if should_ensemble {
            let rescue_steps = if suit_mode == SpiderSuitMode::One {
                max_steps
                    .saturating_mul(5)
                    .saturating_div(4)
                    .clamp(320, 960)
            } else {
                max_steps
                    .saturating_mul(3)
                    .saturating_div(4)
                    .clamp(256, 768)
            };
            let rescue_stagnation = if suit_mode == SpiderSuitMode::One {
                stagnation_limit.saturating_add(40)
            } else {
                stagnation_limit.saturating_add(80)
            };
            let rescue_repeat = repeat_limit.saturating_add(1).min(8);
            let quality =
                |a: &SpiderSinglePlaythroughResult| -> (usize, usize, usize, usize, usize, i32) {
                    let stop_rank = match a.stop_reason {
                        SpiderFindStopReason::Won => 5,
                        SpiderFindStopReason::PostStockStall => 4,
                        SpiderFindStopReason::RepeatStateStagnation => 3,
                        SpiderFindStopReason::RepeatStateCycle => 2,
                        SpiderFindStopReason::NoAction => 1,
                        _ => 0,
                    };
                    (
                        a.completed_runs,
                        a.peak_empty_cols,
                        a.empty_creates,
                        a.peak_suited_edges,
                        a.peak_tail_run,
                        stop_rank,
                    )
                };
            if suit_mode == SpiderSuitMode::One {
                let mut variant_defs: Vec<(usize, SpiderSolverPolicy)> = Vec::new();

                let mut p1 = policy;
                p1.top_width = p1.top_width.max(14);
                p1.recent_window = p1.recent_window.min(8);
                p1.empty_create_bonus = p1.empty_create_bonus.saturating_add(1_000);
                variant_defs.push((0, p1));

                let mut p2 = policy;
                p2.empty_create_bonus = p2.empty_create_bonus.saturating_add(2_400);
                p2.empty_col_bonus = p2.empty_col_bonus.saturating_add(1_000);
                p2.empty_bridge_bonus = p2.empty_bridge_bonus.saturating_add(800);
                p2.post_stock_flat_penalty /= 2;
                variant_defs.push((1, p2));

                let mut p3 = policy;
                p3.top_width = p3.top_width.max(16);
                p3.recent_window = p3.recent_window.min(6);
                p3.allow_revisit_fallback = true;
                p3.require_post_stock_progress = false;
                p3.post_stock_flat_penalty = 0;
                variant_defs.push((2, p3));

                variant_defs.sort_by(|(a, _), (b, _)| {
                    let ta = SPIDER1_VARIANT_TRIALS[*a].load(Ordering::Relaxed) as f64;
                    let wa = SPIDER1_VARIANT_WINS[*a].load(Ordering::Relaxed) as f64;
                    let tb = SPIDER1_VARIANT_TRIALS[*b].load(Ordering::Relaxed) as f64;
                    let wb = SPIDER1_VARIANT_WINS[*b].load(Ordering::Relaxed) as f64;
                    let score_a = (wa + 1.0) / (ta + 2.0) + 1.0 / (ta + 1.0).sqrt();
                    let score_b = (wb + 1.0) / (tb + 2.0) + 1.0 / (tb + 1.0).sqrt();
                    score_b.partial_cmp(&score_a).unwrap_or(CmpOrdering::Equal)
                });

                for (variant_id, vpolicy) in variant_defs {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    SPIDER1_VARIANT_TRIALS[variant_id].fetch_add(1, Ordering::Relaxed);
                    let trial = attempt_with_salt(
                        seed,
                        suit_mode,
                        rescue_steps,
                        rescue_stagnation,
                        rescue_repeat,
                        vpolicy,
                        variant_id + 1,
                        cancel,
                    );
                    if trial.stop_reason == SpiderFindStopReason::Canceled {
                        best_attempt = trial;
                        break;
                    }
                    if trial.line.is_some() {
                        SPIDER1_VARIANT_WINS[variant_id].fetch_add(1, Ordering::Relaxed);
                        best_attempt = trial;
                        break;
                    }
                    if quality(&trial) > quality(&best_attempt) {
                        SPIDER1_VARIANT_WINS[variant_id].fetch_add(1, Ordering::Relaxed);
                        best_attempt = trial;
                    }
                }
            } else {
                let mut variant_defs: Vec<(usize, SpiderSolverPolicy)> = Vec::new();

                let mut p1 = policy;
                p1.top_width = p1.top_width.max(12);
                p1.recent_window = p1.recent_window.min(8);
                variant_defs.push((0, p1));

                let mut p2 = policy;
                p2.empty_create_bonus = p2.empty_create_bonus.saturating_add(8_000);
                p2.empty_col_bonus = p2.empty_col_bonus.saturating_add(2_000);
                p2.no_empty_nonclear_penalty = p2.no_empty_nonclear_penalty.saturating_add(5_000);
                p2.require_post_stock_progress = false;
                p2.post_stock_flat_penalty /= 2;
                variant_defs.push((1, p2));

                variant_defs.sort_by(|(a, _), (b, _)| {
                    let ta = SPIDER4_VARIANT_TRIALS[*a].load(Ordering::Relaxed) as f64;
                    let wa = SPIDER4_VARIANT_WINS[*a].load(Ordering::Relaxed) as f64;
                    let tb = SPIDER4_VARIANT_TRIALS[*b].load(Ordering::Relaxed) as f64;
                    let wb = SPIDER4_VARIANT_WINS[*b].load(Ordering::Relaxed) as f64;
                    let score_a = (wa + 1.0) / (ta + 2.0) + 1.0 / (ta + 1.0).sqrt();
                    let score_b = (wb + 1.0) / (tb + 2.0) + 1.0 / (tb + 1.0).sqrt();
                    score_b.partial_cmp(&score_a).unwrap_or(CmpOrdering::Equal)
                });

                for (variant_id, vpolicy) in variant_defs {
                    if cancel.load(Ordering::Relaxed) {
                        break;
                    }
                    SPIDER4_VARIANT_TRIALS[variant_id].fetch_add(1, Ordering::Relaxed);
                    let trial = attempt_with_salt(
                        seed,
                        suit_mode,
                        rescue_steps,
                        rescue_stagnation,
                        rescue_repeat,
                        vpolicy,
                        variant_id + 1,
                        cancel,
                    );
                    if trial.stop_reason == SpiderFindStopReason::Canceled {
                        best_attempt = trial;
                        break;
                    }
                    if trial.line.is_some() {
                        SPIDER4_VARIANT_WINS[variant_id].fetch_add(1, Ordering::Relaxed);
                        best_attempt = trial;
                        break;
                    }
                    if quality(&trial) > quality(&best_attempt) {
                        SPIDER4_VARIANT_WINS[variant_id].fetch_add(1, Ordering::Relaxed);
                        best_attempt = trial;
                    }
                }
            }
        }
    }

    let attempt = best_attempt;
    finish(
        None,
        attempt.expanded_states,
        attempt.generated_branches,
        attempt.stop_reason,
        attempt.completed_runs,
        attempt.stock_cards,
        attempt.face_down_cards,
        attempt.empty_cols,
        attempt.suited_edges,
        attempt.max_tail_run,
        attempt.cycle_period,
        attempt.cycle_blocks,
        attempt.draw_moves,
        attempt.tableau_moves,
        attempt.reveal_moves,
        attempt.peak_suited_edges,
        attempt.peak_tail_run,
        attempt.peak_empty_cols,
        attempt.empty_creates,
        attempt.adapt_events,
    )
}

fn spider_rollout_candidate_score(attempt: &SpiderSinglePlaythroughResult) -> i64 {
    let stop_rank = match attempt.stop_reason {
        SpiderFindStopReason::Won => 10_000_000_i64,
        SpiderFindStopReason::PostStockStall => 80_000,
        SpiderFindStopReason::RepeatStateStagnation => 60_000,
        SpiderFindStopReason::RepeatStateCycle => 45_000,
        SpiderFindStopReason::NoAction => 30_000,
        _ => 0,
    };
    stop_rank
        + (attempt.completed_runs as i64) * 200_000
        + (attempt.peak_empty_cols as i64) * 45_000
        + (attempt.empty_creates as i64) * 8_000
        + (attempt.peak_suited_edges as i64) * 500
        + (attempt.peak_tail_run as i64) * 250
        + (attempt.reveal_moves as i64) * 120
        - (attempt.face_down_cards as i64) * 90
}

fn spider4_candidate_is_promising(attempt: &SpiderSinglePlaythroughResult) -> bool {
    attempt.completed_runs > 0
        || attempt.peak_empty_cols > 0
        || attempt.empty_creates > 0
        || attempt.peak_suited_edges >= 12
        || attempt.peak_tail_run >= 16
}

fn spider4_verifier_budgets(guided_budget: usize, exhaustive_budget: usize) -> (usize, usize) {
    let base = guided_budget.saturating_add(exhaustive_budget).max(240_000);
    let boosted = base.saturating_mul(2).clamp(240_000, 1_200_000);
    (boosted, boosted / 2)
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

    let mut visited_hashes: HashSet<u64> = HashSet::new();
    let mut seen_hashes: HashSet<u64> = HashSet::new();
    let mut frontier: BinaryHeap<SpiderGuidedNode> = BinaryHeap::new();
    let mut serial = 0_u64;
    let mut generated_states = 1_usize;
    let max_generated_states = max_states.max(1);

    let start_hash = spider_state_hash(&start);
    seen_hashes.insert(start_hash);
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
                iterations: visited_hashes.len(),
                moves_to_win: None,
                hit_state_limit: true,
                solver_line: None,
                hint_line: None,
                freecell_line: None,
                canceled: true,
            });
        }

        let state = node.state;
        let current_hash = spider_state_hash(&state);
        if !visited_hashes.insert(current_hash) {
            continue;
        }
        if state.is_won() {
            let mut line_rev: Vec<HintMove> = Vec::new();
            let mut cursor = Some(current_hash);
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
                iterations: visited_hashes.len(),
                moves_to_win: Some(node.depth),
                hit_state_limit: false,
                solver_line: None,
                hint_line: Some(line_rev),
                freecell_line: None,
                canceled: false,
            });
        }

        if visited_hashes.len() >= max_states {
            return Some(SeedWinnabilityCheckResult {
                winnable: false,
                iterations: visited_hashes.len(),
                moves_to_win: None,
                hit_state_limit: true,
                solver_line: None,
                hint_line: None,
                freecell_line: None,
                canceled: false,
            });
        }

        let successors = spider_solver_ranked_candidates(&state, suit_mode, node.parent_hash);

        for (next, hint_move, transition_score) in successors {
            if cancel.load(Ordering::Relaxed) {
                return Some(SeedWinnabilityCheckResult {
                    winnable: false,
                    iterations: visited_hashes.len(),
                    moves_to_win: None,
                    hit_state_limit: true,
                    solver_line: None,
                    hint_line: None,
                    freecell_line: None,
                    canceled: true,
                });
            }

            let next_hash = spider_state_hash(&next);
            if seen_hashes.contains(&next_hash) {
                continue;
            }
            if generated_states >= max_generated_states {
                return Some(SeedWinnabilityCheckResult {
                    winnable: false,
                    iterations: visited_hashes.len(),
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
            seen_hashes.insert(next_hash);
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
        iterations: visited_hashes.len(),
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
    progress_checked: Option<Arc<AtomicU32>>,
    progress_stats: Option<Arc<SpiderFindProgress>>,
) -> Option<(u64, u32, Vec<HintMove>)> {
    if attempts == 0 {
        return None;
    }

    const SPIDER_FIND_MAX_WORKERS: usize = 8;
    let worker_count = thread::available_parallelism()
        .map(|n| n.get())
        .map(|n| n.min(SPIDER_FIND_MAX_WORKERS))
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
        let progress_checked = progress_checked.clone();
        let progress_stats = progress_stats.clone();
        let handle = thread::spawn(move || loop {
            const SPIDER4_TOP_K: usize = 6;
            const SPIDER4_VERIFY_INTERVAL: u32 = 24;
            let mut candidate_pool: Vec<(i64, u64, u32)> = Vec::new();
            let mut verified_candidates: HashSet<u64> = HashSet::new();
            let mut since_verify = 0_u32;

            let mut verify_best_candidate = |candidate_pool: &mut Vec<(i64, u64, u32)>| -> bool {
                if suit_mode != SpiderSuitMode::Four || cancel.load(Ordering::Relaxed) {
                    return false;
                }
                candidate_pool.sort_by(|a, b| b.0.cmp(&a.0));
                while let Some((_, cseed, tested)) = candidate_pool.first().copied() {
                    candidate_pool.remove(0);
                    if !verified_candidates.insert(cseed) {
                        continue;
                    }
                    let (verify_guided, verify_exhaustive) =
                        spider4_verifier_budgets(guided_budget, exhaustive_budget);
                    let won_line = is_spider_seed_winnable(
                        cseed,
                        suit_mode,
                        verify_guided,
                        verify_exhaustive,
                        cancel.as_ref(),
                    )
                    .and_then(|r| if r.winnable { r.hint_line } else { None });
                    if let Some(line) = won_line {
                        if !cancel.swap(true, Ordering::Relaxed) {
                            let _ = sender.send((cseed, tested, line));
                        }
                        return true;
                    }
                }
                false
            };

            if cancel.load(Ordering::Relaxed) {
                break;
            }
            loop {
                let index = next_index.fetch_add(1, Ordering::Relaxed);
                if index >= attempts {
                    break;
                }

                let seed = start_seed.wrapping_add(u64::from(index));
                let attempt = spider_single_playthrough_line(
                    seed,
                    suit_mode,
                    guided_budget,
                    exhaustive_budget,
                    cancel.as_ref(),
                );
                if let Some(progress) = &progress_stats {
                    progress.store_attempt(
                        seed,
                        index.saturating_add(1),
                        attempt.expanded_states,
                        attempt.generated_branches,
                        attempt.stop_reason,
                        attempt.completed_runs,
                        attempt.stock_cards,
                        attempt.face_down_cards,
                        attempt.empty_cols,
                        attempt.suited_edges,
                        attempt.max_tail_run,
                        attempt.cycle_period,
                        attempt.cycle_blocks,
                        attempt.draw_moves,
                        attempt.tableau_moves,
                        attempt.reveal_moves,
                        attempt.peak_suited_edges,
                        attempt.peak_tail_run,
                        attempt.peak_empty_cols,
                        attempt.empty_creates,
                        attempt.adapt_events,
                    );
                }
                if let Some(progress) = &progress_checked {
                    progress.fetch_max(index.saturating_add(1), Ordering::Relaxed);
                }
                if let Some(line) = attempt.line {
                    if !cancel.swap(true, Ordering::Relaxed) {
                        let _ = sender.send((seed, index + 1, line));
                    }
                    return;
                }

                if suit_mode == SpiderSuitMode::Four && spider4_candidate_is_promising(&attempt) {
                    let score = spider_rollout_candidate_score(&attempt);
                    if candidate_pool.len() < SPIDER4_TOP_K {
                        candidate_pool.push((score, seed, index + 1));
                    } else if let Some((min_i, _)) = candidate_pool
                        .iter()
                        .enumerate()
                        .min_by(|(_, a), (_, b)| a.0.cmp(&b.0))
                    {
                        if score > candidate_pool[min_i].0 {
                            candidate_pool[min_i] = (score, seed, index + 1);
                        }
                    }
                }

                since_verify = since_verify.saturating_add(1);
                if since_verify >= SPIDER4_VERIFY_INTERVAL {
                    since_verify = 0;
                    if verify_best_candidate(&mut candidate_pool) {
                        return;
                    }
                }
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
            }

            while !cancel.load(Ordering::Relaxed) && !candidate_pool.is_empty() {
                if verify_best_candidate(&mut candidate_pool) {
                    return;
                }
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
    use crate::game::Suit;

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

    #[test]
    fn spider_find_winnable_honors_cancel_flag() {
        let cancel = Arc::new(AtomicBool::new(true));
        let result = find_winnable_spider_seed_parallel(
            123,
            16,
            15_000,
            0,
            SpiderSuitMode::Four,
            cancel,
            None,
            None,
        );
        assert!(result.is_none());
    }

    #[test]
    fn spider_repeating_tail_cycle_detector_trips_on_3_4_5_patterns() {
        let mut seq3 = Vec::new();
        for _ in 0..4 {
            seq3.extend(["a".to_string(), "b".to_string(), "c".to_string()]);
        }
        assert_eq!(
            spider_solver_detect_repeating_tail_cycle(&seq3),
            Some((3, 4))
        );

        let mut seq4 = Vec::new();
        for _ in 0..4 {
            seq4.extend([
                "m1".to_string(),
                "m2".to_string(),
                "m3".to_string(),
                "m4".to_string(),
            ]);
        }
        assert_eq!(
            spider_solver_detect_repeating_tail_cycle(&seq4),
            Some((4, 4))
        );

        let mut seq5 = Vec::new();
        for _ in 0..4 {
            seq5.extend([
                "p1".to_string(),
                "p2".to_string(),
                "p3".to_string(),
                "p4".to_string(),
                "p5".to_string(),
            ]);
        }
        assert_eq!(
            spider_solver_detect_repeating_tail_cycle(&seq5),
            Some((5, 4))
        );
    }

    #[test]
    fn spider_four_suit_prefers_source_clearing_move_when_no_empty_cols() {
        fn c(suit: Suit, rank: u8, face_up: bool) -> Card {
            Card {
                suit,
                rank,
                face_up,
            }
        }

        let stock = vec![c(Suit::Spades, 1, false); 10];
        let tableau = [
            vec![c(Suit::Spades, 6, true)], // clearing move: 6 -> 7
            vec![c(Suit::Hearts, 7, true)],
            vec![c(Suit::Clubs, 9, true), c(Suit::Clubs, 8, true)], // non-clearing move: 8 -> 9
            vec![c(Suit::Diamonds, 13, true)],
            vec![c(Suit::Diamonds, 9, true)],
            vec![c(Suit::Spades, 12, true)],
            vec![c(Suit::Hearts, 5, true)],
            vec![c(Suit::Clubs, 2, true)],
            vec![c(Suit::Diamonds, 4, true)],
            vec![c(Suit::Spades, 1, true)],
        ];
        let game = SpiderGame::debug_new(SpiderSuitMode::Four, stock, tableau, 0);
        assert_eq!(spider_empty_col_count(&game), 0);
        assert_eq!(game.stock_len(), 10);

        let ranked = spider_solver_ranked_candidates(&game, SpiderSuitMode::Four, None);
        assert!(ranked.iter().any(|(_, hm, _)| {
            matches!(
                hm,
                HintMove::TableauRunToTableau {
                    start,
                    ..
                } if *start > 0
            )
        }));
        assert!(ranked
            .iter()
            .any(|(_, hm, _)| { matches!(hm, HintMove::TableauRunToTableau { start: 0, .. }) }));

        let decision = spider_solver_choose_move(
            &game,
            SpiderSuitMode::Four,
            None,
            &HashSet::new(),
            &VecDeque::new(),
            0,
            spider_find_policy(SpiderSuitMode::Four),
            |_| true,
        )
        .expect("must have at least one legal move");

        match decision.hint_move {
            HintMove::TableauRunToTableau { start, .. } => {
                assert_eq!(
                    start, 0,
                    "expected clear-source move preference when no empty columns"
                );
            }
            other => panic!("expected tableau move, got {:?}", other),
        }
    }

    #[test]
    fn spider_four_suit_headless_search_emits_empty_metrics() {
        let cancel = Arc::new(AtomicBool::new(false));
        let progress_checked = Arc::new(AtomicU32::new(0));
        let progress_stats = Arc::new(SpiderFindProgress::default());

        let _ = find_winnable_spider_seed_parallel(
            11_000_411_495_875_974_310,
            128,
            15_000,
            0,
            SpiderSuitMode::Four,
            Arc::clone(&cancel),
            Some(Arc::clone(&progress_checked)),
            Some(Arc::clone(&progress_stats)),
        );

        let checked = progress_checked.load(Ordering::Relaxed);
        let peak_empty = progress_stats.last_peak_empty_cols.load(Ordering::Relaxed);
        let empty_creates = progress_stats.last_empty_creates.load(Ordering::Relaxed);
        let adapt_events = progress_stats.last_adapt_events.load(Ordering::Relaxed);
        let best_peak_empty = progress_stats.best_peak_empty_cols.load(Ordering::Relaxed);
        let attempts_with_empty = progress_stats
            .attempts_with_empty_create
            .load(Ordering::Relaxed);
        let stock_cards = progress_stats.last_stock_cards.load(Ordering::Relaxed);
        let stop_reason =
            spider_find_stop_reason_label(progress_stats.last_stop_reason.load(Ordering::Relaxed));

        println!(
            "headless_spider4 checked={} stock={} peak_empty={} empty_creates={} best_peak_empty={} attempts_with_empty={} adapt={} stop={}",
            checked,
            stock_cards,
            peak_empty,
            empty_creates,
            best_peak_empty,
            attempts_with_empty,
            adapt_events,
            stop_reason
        );

        assert!(checked > 0, "search should evaluate at least one seed");
    }

    #[test]
    fn spider_four_suit_headless_batch_diagnostics() {
        let starts = [
            11_000_411_495_875_974_310_u64,
            7_581_659_828_837_066_710_u64,
            14_845_754_829_415_625_642_u64,
            17_069_608_616_668_230_469_u64,
        ];
        let attempts = 128;
        let mut sum_peak_empty = 0usize;
        let mut sum_empty_creates = 0usize;
        let mut sum_adapt = 0usize;
        let mut post_stock_stall = 0usize;
        let mut repeat_cycle = 0usize;
        let mut repeat_stag = 0usize;
        let mut no_action = 0usize;

        for start_seed in starts {
            let cancel = Arc::new(AtomicBool::new(false));
            let progress_checked = Arc::new(AtomicU32::new(0));
            let progress_stats = Arc::new(SpiderFindProgress::default());

            let _ = find_winnable_spider_seed_parallel(
                start_seed,
                attempts,
                15_000,
                0,
                SpiderSuitMode::Four,
                Arc::clone(&cancel),
                Some(Arc::clone(&progress_checked)),
                Some(Arc::clone(&progress_stats)),
            );

            let checked = progress_checked.load(Ordering::Relaxed);
            let peak_empty = progress_stats.last_peak_empty_cols.load(Ordering::Relaxed);
            let empty_creates = progress_stats.last_empty_creates.load(Ordering::Relaxed);
            let adapt_events = progress_stats.last_adapt_events.load(Ordering::Relaxed);
            let best_peak_empty = progress_stats.best_peak_empty_cols.load(Ordering::Relaxed);
            let attempts_with_empty = progress_stats
                .attempts_with_empty_create
                .load(Ordering::Relaxed);
            let stop_code = progress_stats.last_stop_reason.load(Ordering::Relaxed);
            let stop_reason = spider_find_stop_reason_label(stop_code);

            sum_peak_empty = sum_peak_empty.saturating_add(peak_empty);
            sum_empty_creates = sum_empty_creates.saturating_add(empty_creates);
            sum_adapt = sum_adapt.saturating_add(adapt_events);
            match stop_reason {
                "post_stock_stall" => post_stock_stall = post_stock_stall.saturating_add(1),
                "repeat_state_cycle" => repeat_cycle = repeat_cycle.saturating_add(1),
                "repeat_state_stagnation" => repeat_stag = repeat_stag.saturating_add(1),
                "no_action" => no_action = no_action.saturating_add(1),
                _ => {}
            }

            println!(
                "spider4_diag seed={} checked={} peak_empty={} empty_creates={} best_peak_empty={} attempts_with_empty={} adapt={} stop={}",
                start_seed,
                checked,
                peak_empty,
                empty_creates,
                best_peak_empty,
                attempts_with_empty,
                adapt_events,
                stop_reason
            );
        }

        let runs = starts.len().max(1);
        println!(
            "spider4_diag_summary runs={} avg_peak_empty={:.2} avg_empty_creates={:.2} avg_adapt={:.2} stops(post_stock_stall/repeat_cycle/repeat_stag/no_action)={}/{}/{}/{}",
            runs,
            (sum_peak_empty as f64) / (runs as f64),
            (sum_empty_creates as f64) / (runs as f64),
            (sum_adapt as f64) / (runs as f64),
            post_stock_stall,
            repeat_cycle,
            repeat_stag,
            no_action
        );
    }

    #[test]
    fn spider_one_suit_headless_batch_winrate() {
        let starts = [
            3_286_964_361_610_284_302_u64,
            178_474_232_105_319_697_u64,
            9_252_126_266_307_643_378_u64,
            15_610_344_138_934_073_849_u64,
        ];
        let attempts = 128;
        let mut wins = 0usize;
        let mut checked_total = 0usize;

        for start_seed in starts {
            let cancel = Arc::new(AtomicBool::new(false));
            let progress_checked = Arc::new(AtomicU32::new(0));
            let progress_stats = Arc::new(SpiderFindProgress::default());

            let found = find_winnable_spider_seed_parallel(
                start_seed,
                attempts,
                15_000,
                0,
                SpiderSuitMode::One,
                Arc::clone(&cancel),
                Some(Arc::clone(&progress_checked)),
                Some(Arc::clone(&progress_stats)),
            )
            .is_some();

            if found {
                wins = wins.saturating_add(1);
            }
            let checked = progress_checked.load(Ordering::Relaxed) as usize;
            checked_total = checked_total.saturating_add(checked);
            println!(
                "spider1_diag seed={} found={} checked={} stop={}",
                start_seed,
                found,
                checked,
                spider_find_stop_reason_label(
                    progress_stats.last_stop_reason.load(Ordering::Relaxed)
                )
            );
        }

        let runs = starts.len().max(1);
        println!(
            "spider1_diag_summary runs={} wins={} per_run_win_rate={:.2}% avg_checked={:.2}",
            runs,
            wins,
            (wins as f64) * 100.0 / (runs as f64),
            (checked_total as f64) / (runs as f64)
        );
        assert!(
            checked_total > 0,
            "1-suit diagnostic should check some seeds"
        );
    }
}

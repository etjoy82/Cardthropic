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
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use crate::engine::moves::HintMove;
use crate::game::{Card, DrawMode, KlondikeGame, SolverMove, SpiderGame, SpiderSuitMode};

#[derive(Debug, Clone)]
pub struct SeedWinnabilityCheckResult {
    pub winnable: bool,
    pub iterations: usize,
    pub moves_to_win: Option<u32>,
    pub hit_state_limit: bool,
    pub solver_line: Option<Vec<SolverMove>>,
    pub hint_line: Option<Vec<HintMove>>,
    pub canceled: bool,
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
        canceled: false,
    })
}

pub fn find_winnable_seed_parallel(
    start_seed: u64,
    attempts: u32,
    max_states: usize,
    draw_mode: DrawMode,
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
    let stop = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel::<(u64, u32, Vec<SolverMove>)>();
    let mut handles = Vec::with_capacity(worker_count);

    for _ in 0..worker_count {
        let next_index = Arc::clone(&next_index);
        let stop = Arc::clone(&stop);
        let sender = sender.clone();
        let handle = thread::spawn(move || loop {
            if stop.load(Ordering::Relaxed) {
                break;
            }
            let index = next_index.fetch_add(1, Ordering::Relaxed);
            if index >= attempts {
                break;
            }

            let seed = start_seed.wrapping_add(u64::from(index));
            let mut game = KlondikeGame::new_with_seed(seed);
            game.set_draw_mode(draw_mode);
            let Some(winnable) = game.is_winnable_guided_cancelable(max_states, stop.as_ref())
            else {
                break;
            };
            if !winnable {
                continue;
            }

            if stop.load(Ordering::Relaxed) {
                break;
            }
            let Some(line) = game.guided_winning_line_cancelable(max_states, stop.as_ref()) else {
                break;
            };
            let line = line.unwrap_or_default();
            if !stop.swap(true, Ordering::Relaxed) {
                let _ = sender.send((seed, index + 1, line));
            }
            break;
        });
        handles.push(handle);
    }

    drop(sender);
    let result = receiver.recv().ok();
    stop.store(true, Ordering::Relaxed);
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

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

use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;

use crate::game::{DrawMode, KlondikeGame, SolverMove};

#[derive(Debug, Clone)]
pub struct SeedWinnabilityCheckResult {
    pub winnable: bool,
    pub iterations: usize,
    pub moves_to_win: Option<u32>,
    pub hit_state_limit: bool,
    pub solver_line: Option<Vec<SolverMove>>,
}

pub fn default_find_winnable_attempts() -> u32 {
    thread::available_parallelism()
        .map(|n| (n.get() * 6).clamp(16, 128) as u32)
        .unwrap_or(48)
}

pub fn is_seed_winnable(
    seed: u64,
    draw_mode: DrawMode,
    guided_budget: usize,
    exhaustive_budget: usize,
    cancel: &AtomicBool,
) -> Option<SeedWinnabilityCheckResult> {
    let mut game = KlondikeGame::new_with_seed(seed);
    game.set_draw_mode(draw_mode);
    let guided = game.guided_winnability_cancelable(guided_budget, cancel)?;
    if guided.winnable {
        let solver_line = game.guided_winning_line_cancelable(guided_budget, cancel)?;
        return Some(SeedWinnabilityCheckResult {
            winnable: true,
            iterations: guided.explored_states,
            moves_to_win: guided.win_depth,
            hit_state_limit: guided.hit_state_limit,
            solver_line,
        });
    }

    let exhaustive = game.analyze_winnability_cancelable(exhaustive_budget, cancel)?;
    Some(SeedWinnabilityCheckResult {
        winnable: exhaustive.winnable,
        iterations: guided
            .explored_states
            .saturating_add(exhaustive.explored_states),
        moves_to_win: exhaustive.win_depth,
        hit_state_limit: exhaustive.hit_state_limit,
        solver_line: None,
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
        .unwrap_or(1)
        .min(attempts as usize)
        .max(1);

    let next_index = Arc::new(AtomicU32::new(0));
    let stop = Arc::new(AtomicBool::new(false));
    let (sender, receiver) = mpsc::channel::<(u64, u32, Vec<SolverMove>)>();

    for _ in 0..worker_count {
        let next_index = Arc::clone(&next_index);
        let stop = Arc::clone(&stop);
        let sender = sender.clone();
        thread::spawn(move || loop {
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
            if game.is_winnable_guided(max_states) {
                let cancel = AtomicBool::new(false);
                let line = game
                    .guided_winning_line_cancelable(max_states, &cancel)
                    .unwrap_or(None)
                    .unwrap_or_default();
                if !stop.swap(true, Ordering::Relaxed) {
                    let _ = sender.send((seed, index + 1, line));
                }
                break;
            }
        });
    }

    drop(sender);
    receiver.recv().ok()
}

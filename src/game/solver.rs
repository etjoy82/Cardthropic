use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

use super::*;

const GUIDED_LOOP_PENALTY: i64 = 9_000;
const GUIDED_KING_EMPTY_PENALTY: i64 = 2_600;
const GUIDED_ZERO_PROGRESS_PENALTY: i64 = 900;

#[derive(Clone)]
struct GuidedNode {
    priority: i64,
    serial: u64,
    depth: u32,
    parent_hash: Option<u64>,
    state: KlondikeGame,
}

#[derive(Clone)]
struct ScoredSuccessor {
    state: KlondikeGame,
    solver_move: SolverMove,
    transition_score: i64,
}

impl PartialEq for GuidedNode {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.serial == other.serial
    }
}

impl Eq for GuidedNode {}

impl PartialOrd for GuidedNode {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for GuidedNode {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.serial.cmp(&self.serial))
    }
}

impl KlondikeGame {
    pub fn is_winnable_best_play(&self, max_states: usize) -> bool {
        self.analyze_winnability(max_states).winnable
    }

    pub fn guided_winnability(&self, max_states: usize) -> GuidedWinnabilityResult {
        let cancel = AtomicBool::new(false);
        self.guided_winnability_cancelable(max_states, &cancel)
            .unwrap_or(GuidedWinnabilityResult {
                winnable: false,
                explored_states: 0,
                generated_states: 0,
                win_depth: None,
                hit_state_limit: true,
            })
    }

    pub fn is_winnable_guided(&self, max_states: usize) -> bool {
        self.guided_winnability(max_states).winnable
    }

    pub fn is_winnable_guided_cancelable(
        &self,
        max_states: usize,
        cancel: &AtomicBool,
    ) -> Option<bool> {
        Some(
            self.guided_winnability_cancelable(max_states, cancel)?
                .winnable,
        )
    }

    pub fn guided_winnability_cancelable(
        &self,
        max_states: usize,
        cancel: &AtomicBool,
    ) -> Option<GuidedWinnabilityResult> {
        if cancel.load(AtomicOrdering::Relaxed) {
            return None;
        }
        if self.is_won() {
            return Some(GuidedWinnabilityResult {
                winnable: true,
                explored_states: 1,
                generated_states: 1,
                win_depth: Some(0),
                hit_state_limit: false,
            });
        }
        if max_states == 0 {
            return Some(GuidedWinnabilityResult {
                winnable: false,
                explored_states: 0,
                generated_states: 1,
                win_depth: None,
                hit_state_limit: true,
            });
        }

        let mut visited: HashSet<KlondikeGame> = HashSet::new();
        let mut frontier: BinaryHeap<GuidedNode> = BinaryHeap::new();
        let mut serial = 0_u64;
        let mut generated_states = 1_usize;
        frontier.push(GuidedNode {
            priority: self.state_priority(),
            serial,
            depth: 0,
            parent_hash: None,
            state: self.clone(),
        });

        while let Some(node) = frontier.pop() {
            if cancel.load(AtomicOrdering::Relaxed) {
                return None;
            }
            let state = node.state;
            if !visited.insert(state.clone()) {
                continue;
            }
            if state.is_won() {
                return Some(GuidedWinnabilityResult {
                    winnable: true,
                    explored_states: visited.len(),
                    generated_states,
                    win_depth: Some(node.depth),
                    hit_state_limit: false,
                });
            }
            if visited.len() >= max_states {
                return Some(GuidedWinnabilityResult {
                    winnable: false,
                    explored_states: visited.len(),
                    generated_states,
                    win_depth: None,
                    hit_state_limit: true,
                });
            }

            let current_hash = state.state_hash();
            for successor in state.successor_states_guided(node.parent_hash) {
                if cancel.load(AtomicOrdering::Relaxed) {
                    return None;
                }
                let next = successor.state;
                if visited.contains(&next) {
                    continue;
                }
                serial = serial.wrapping_add(1);
                generated_states = generated_states.saturating_add(1);
                frontier.push(GuidedNode {
                    priority: successor.transition_score + next.state_priority()
                        - i64::from(node.depth) * 2,
                    serial,
                    depth: node.depth + 1,
                    parent_hash: Some(current_hash),
                    state: next,
                });
            }
        }

        Some(GuidedWinnabilityResult {
            winnable: false,
            explored_states: visited.len(),
            generated_states,
            win_depth: None,
            hit_state_limit: false,
        })
    }

    pub fn guided_winning_line_cancelable(
        &self,
        max_states: usize,
        cancel: &AtomicBool,
    ) -> Option<Option<Vec<SolverMove>>> {
        if cancel.load(AtomicOrdering::Relaxed) {
            return None;
        }
        if self.is_won() {
            return Some(Some(Vec::new()));
        }
        if max_states == 0 {
            return Some(None);
        }

        let mut visited: HashSet<KlondikeGame> = HashSet::new();
        let mut frontier: BinaryHeap<GuidedNode> = BinaryHeap::new();
        let mut serial = 0_u64;
        let initial_hash = self.state_hash();
        let mut parent_by_hash: HashMap<u64, (Option<u64>, Option<SolverMove>)> = HashMap::new();
        parent_by_hash.insert(initial_hash, (None, None));

        frontier.push(GuidedNode {
            priority: self.state_priority(),
            serial,
            depth: 0,
            parent_hash: None,
            state: self.clone(),
        });

        while let Some(node) = frontier.pop() {
            if cancel.load(AtomicOrdering::Relaxed) {
                return None;
            }
            let state = node.state;
            if !visited.insert(state.clone()) {
                continue;
            }

            let current_hash = state.state_hash();
            if state.is_won() {
                let mut line_rev: Vec<SolverMove> = Vec::new();
                let mut cursor = Some(current_hash);
                while let Some(hash) = cursor {
                    let Some((parent_hash, solver_move)) = parent_by_hash.get(&hash).copied()
                    else {
                        break;
                    };
                    if let Some(mv) = solver_move {
                        line_rev.push(mv);
                    }
                    cursor = parent_hash;
                }
                line_rev.reverse();
                return Some(Some(line_rev));
            }

            if visited.len() >= max_states {
                return Some(None);
            }

            for successor in state.successor_states_guided(node.parent_hash) {
                if cancel.load(AtomicOrdering::Relaxed) {
                    return None;
                }
                let next = successor.state;
                if visited.contains(&next) {
                    continue;
                }
                let next_hash = next.state_hash();
                parent_by_hash
                    .entry(next_hash)
                    .or_insert((Some(current_hash), Some(successor.solver_move)));
                serial = serial.wrapping_add(1);
                frontier.push(GuidedNode {
                    priority: successor.transition_score + next.state_priority()
                        - i64::from(node.depth) * 2,
                    serial,
                    depth: node.depth + 1,
                    parent_hash: Some(current_hash),
                    state: next,
                });
            }
        }

        Some(None)
    }

    pub fn analyze_winnability(&self, max_states: usize) -> WinnabilityResult {
        let cancel = AtomicBool::new(false);
        self.analyze_winnability_cancelable(max_states, &cancel)
            .unwrap_or(WinnabilityResult {
                winnable: false,
                explored_states: 0,
                generated_states: 0,
                win_depth: None,
                hit_state_limit: true,
            })
    }

    pub fn analyze_winnability_cancelable(
        &self,
        max_states: usize,
        cancel: &AtomicBool,
    ) -> Option<WinnabilityResult> {
        if cancel.load(AtomicOrdering::Relaxed) {
            return None;
        }
        if self.is_won() {
            return Some(WinnabilityResult {
                winnable: true,
                explored_states: 1,
                generated_states: 1,
                win_depth: Some(0),
                hit_state_limit: false,
            });
        }

        if max_states == 0 {
            return Some(WinnabilityResult {
                winnable: false,
                explored_states: 0,
                generated_states: 1,
                win_depth: None,
                hit_state_limit: true,
            });
        }

        let mut visited: HashSet<KlondikeGame> = HashSet::new();
        let mut frontier: VecDeque<(KlondikeGame, u32)> = VecDeque::new();
        let mut generated_states = 1_usize;
        frontier.push_back((self.clone(), 0));

        while let Some((state, depth)) = frontier.pop_front() {
            if cancel.load(AtomicOrdering::Relaxed) {
                return None;
            }
            if !visited.insert(state.clone()) {
                continue;
            }

            if state.is_won() {
                return Some(WinnabilityResult {
                    winnable: true,
                    explored_states: visited.len(),
                    generated_states,
                    win_depth: Some(depth),
                    hit_state_limit: false,
                });
            }

            if visited.len() >= max_states {
                return Some(WinnabilityResult {
                    winnable: false,
                    explored_states: visited.len(),
                    generated_states,
                    win_depth: None,
                    hit_state_limit: true,
                });
            }

            for next in state
                .successor_states_guided(None)
                .into_iter()
                .map(|successor| successor.state)
            {
                if cancel.load(AtomicOrdering::Relaxed) {
                    return None;
                }
                if !visited.contains(&next) {
                    generated_states = generated_states.saturating_add(1);
                    frontier.push_back((next, depth + 1));
                }
            }
        }

        Some(WinnabilityResult {
            winnable: false,
            explored_states: visited.len(),
            generated_states,
            win_depth: None,
            hit_state_limit: false,
        })
    }

    fn successor_states_guided(&self, previous_hash: Option<u64>) -> Vec<ScoredSuccessor> {
        let mut raw: Vec<(SolverMove, KlondikeGame)> = Vec::new();

        if self.can_move_waste_to_foundation() {
            let mut state = self.clone();
            state.move_waste_to_foundation();
            raw.push((SolverMove::WasteToFoundation, state));
        }

        for src in 0..7 {
            if self.can_move_tableau_top_to_foundation(src) {
                let mut state = self.clone();
                state.move_tableau_top_to_foundation(src);
                raw.push((SolverMove::TableauTopToFoundation { src }, state));
            }
        }

        for dst in 0..7 {
            if self.can_move_waste_to_tableau(dst) {
                let mut state = self.clone();
                state.move_waste_to_tableau(dst);
                raw.push((SolverMove::WasteToTableau { dst }, state));
            }
        }

        for src in 0..7 {
            let len = self.tableau_len(src).unwrap_or(0);
            for start in 0..len {
                for dst in 0..7 {
                    if !self.can_move_tableau_run_to_tableau(src, start, dst) {
                        continue;
                    }
                    let mut state = self.clone();
                    state.move_tableau_run_to_tableau(src, start, dst);
                    raw.push((SolverMove::TableauRunToTableau { src, start, dst }, state));
                }
            }
        }

        let mut draw_state = self.clone();
        if draw_state.draw_or_recycle() != DrawResult::NoOp {
            raw.push((SolverMove::Draw, draw_state));
        }

        let current_non_draw_moves = self.non_draw_move_count();
        let mut best_by_hash = std::collections::HashMap::<u64, ScoredSuccessor>::new();
        for (solver_move, state) in raw {
            let state_hash = state.state_hash();
            let mut score =
                score_solver_transition(self, &state, solver_move, current_non_draw_moves);

            if previous_hash == Some(state_hash) {
                score -= GUIDED_LOOP_PENALTY;
            }
            if is_king_to_empty_without_reveal(self, solver_move) {
                score -= GUIDED_KING_EMPTY_PENALTY;
            }

            let successor = ScoredSuccessor {
                state,
                solver_move,
                transition_score: score,
            };

            match best_by_hash.get(&state_hash) {
                None => {
                    best_by_hash.insert(state_hash, successor);
                }
                Some(existing) if successor.transition_score > existing.transition_score => {
                    best_by_hash.insert(state_hash, successor);
                }
                _ => {}
            }
        }

        let mut scored: Vec<ScoredSuccessor> = best_by_hash.into_values().collect();
        scored.sort_by(|a, b| b.transition_score.cmp(&a.transition_score));
        scored
    }

    fn state_priority(&self) -> i64 {
        let foundation_cards = self
            .foundations
            .iter()
            .map(|pile| pile.len() as i64)
            .sum::<i64>();
        let face_up_cards = self
            .tableau
            .iter()
            .flat_map(|pile| pile.iter())
            .filter(|card| card.face_up)
            .count() as i64;
        let empty_tableau = self.tableau.iter().filter(|pile| pile.is_empty()).count() as i64;

        foundation_cards * 500 + face_up_cards * 15 + empty_tableau * 40
            - self.stock.len() as i64 * 2
            - self.waste.len() as i64
    }

    fn state_hash(&self) -> u64 {
        let mut hasher = DefaultHasher::new();
        self.hash(&mut hasher);
        hasher.finish()
    }

    fn non_draw_move_count(&self) -> i64 {
        let mut count = 0_i64;

        if self.can_move_waste_to_foundation() {
            count += 1;
        }
        for src in 0..7 {
            if self.can_move_tableau_top_to_foundation(src) {
                count += 1;
            }
        }
        for dst in 0..7 {
            if self.can_move_waste_to_tableau(dst) {
                count += 1;
            }
        }
        for src in 0..7 {
            let len = self.tableau_len(src).unwrap_or(0);
            for start in 0..len {
                for dst in 0..7 {
                    if self.can_move_tableau_run_to_tableau(src, start, dst) {
                        count += 1;
                    }
                }
            }
        }

        count
    }
}

fn foundation_count(game: &KlondikeGame) -> i64 {
    game.foundations()
        .iter()
        .map(|pile| pile.len() as i64)
        .sum()
}

fn hidden_tableau_count(game: &KlondikeGame) -> i64 {
    game.tableau()
        .iter()
        .flat_map(|pile| pile.iter())
        .filter(|card| !card.face_up)
        .count() as i64
}

fn face_up_tableau_count(game: &KlondikeGame) -> i64 {
    game.tableau()
        .iter()
        .flat_map(|pile| pile.iter())
        .filter(|card| card.face_up)
        .count() as i64
}

fn empty_tableau_count(game: &KlondikeGame) -> i64 {
    game.tableau().iter().filter(|pile| pile.is_empty()).count() as i64
}

fn is_king_to_empty_without_reveal(game: &KlondikeGame, solver_move: SolverMove) -> bool {
    let SolverMove::TableauRunToTableau { src, start, dst } = solver_move else {
        return false;
    };
    if game.tableau_len(dst) != Some(0) {
        return false;
    }
    let Some(card) = game.tableau_card(src, start) else {
        return false;
    };
    if card.rank != 13 {
        return false;
    }
    let reveals_hidden = start > 0
        && game
            .tableau_card(src, start - 1)
            .map(|below| !below.face_up)
            .unwrap_or(false);
    !reveals_hidden
}

fn score_solver_transition(
    current: &KlondikeGame,
    next: &KlondikeGame,
    solver_move: SolverMove,
    current_non_draw_moves: i64,
) -> i64 {
    let foundation_delta = foundation_count(next) - foundation_count(current);
    let hidden_delta = hidden_tableau_count(current) - hidden_tableau_count(next);
    let face_up_delta = face_up_tableau_count(next) - face_up_tableau_count(current);
    let empty_delta = empty_tableau_count(next) - empty_tableau_count(current);
    let mobility_delta = next.non_draw_move_count() - current_non_draw_moves;

    let mut score =
        foundation_delta * 2100 + hidden_delta * 480 + face_up_delta * 55 + empty_delta * 120;
    score += mobility_delta * 24;

    match solver_move {
        SolverMove::WasteToFoundation | SolverMove::TableauTopToFoundation { .. } => {
            score += 700;
        }
        SolverMove::WasteToTableau { dst } => {
            score += 120;
            if current.tableau_len(dst) == Some(0) {
                score += 250;
            }
            if current.can_move_waste_to_foundation() {
                score -= 450;
            }
        }
        SolverMove::TableauRunToTableau { src, start, dst } => {
            let run_len = current.tableau_len(src).unwrap_or(0).saturating_sub(start) as i64;
            if run_len > 1 {
                score += run_len * 20;
            }
            if start > 0
                && current
                    .tableau_card(src, start - 1)
                    .map(|card| !card.face_up)
                    .unwrap_or(false)
            {
                score += 420;
            }
            if current.tableau_len(dst) == Some(0) {
                score += 180;
            }
            if run_len == 1 && hidden_delta <= 0 && foundation_delta <= 0 {
                score -= 260;
            }
        }
        SolverMove::Draw => {
            if current_non_draw_moves > 0 {
                score -= 800;
            } else {
                score += 80;
            }
            let waste_playable = next.can_move_waste_to_foundation()
                || (0..7).any(|dst| next.can_move_waste_to_tableau(dst));
            if waste_playable {
                score += 180;
            } else {
                score -= 160;
            }
        }
    }

    if foundation_delta <= 0 && hidden_delta <= 0 && empty_delta <= 0 && mobility_delta <= 0 {
        score -= GUIDED_ZERO_PROGRESS_PENALTY;
    }

    score
}

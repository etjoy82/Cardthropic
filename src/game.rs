use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BinaryHeap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicBool, Ordering as AtomicOrdering};

use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};

const GUIDED_LOOP_PENALTY: i64 = 9_000;
const GUIDED_KING_EMPTY_PENALTY: i64 = 2_600;
const GUIDED_ZERO_PROGRESS_PENALTY: i64 = 900;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameMode {
    Klondike,
    Spider,
    Freecell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrawMode {
    One,
    Two,
    Three,
    Four,
    Five,
}

impl DrawMode {
    pub fn count(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::Five => 5,
        }
    }
}

impl GameMode {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "klondike" => Some(Self::Klondike),
            "spider" => Some(Self::Spider),
            "freecell" => Some(Self::Freecell),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Klondike => "klondike",
            Self::Spider => "spider",
            Self::Freecell => "freecell",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Klondike => "Klondike",
            Self::Spider => "Spider",
            Self::Freecell => "FreeCell",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            Self::Klondike => "🥇",
            Self::Spider => "🕷️",
            Self::Freecell => "🗽",
        }
    }

    pub fn engine_ready(self) -> bool {
        matches!(self, Self::Klondike)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

    pub fn is_red(self) -> bool {
        matches!(self, Suit::Diamonds | Suit::Hearts)
    }

    pub fn short(self) -> &'static str {
        match self {
            Suit::Clubs => "C",
            Suit::Diamonds => "D",
            Suit::Hearts => "H",
            Suit::Spades => "S",
        }
    }

    pub fn foundation_index(self) -> usize {
        match self {
            Suit::Clubs => 0,
            Suit::Diamonds => 1,
            Suit::Hearts => 2,
            Suit::Spades => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: u8,
    pub face_up: bool,
}

impl Card {
    pub fn label(&self) -> String {
        format!("{}{}", rank_label(self.rank), self.suit.short())
    }

    pub fn color_red(&self) -> bool {
        self.suit.is_red()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KlondikeGame {
    draw_mode: DrawMode,
    stock: Vec<Card>,
    waste: Vec<Card>,
    foundations: [Vec<Card>; 4],
    tableau: [Vec<Card>; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawResult {
    DrewFromStock,
    RecycledWaste,
    NoOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinnabilityResult {
    pub winnable: bool,
    pub explored_states: usize,
    pub generated_states: usize,
    pub win_depth: Option<u32>,
    pub hit_state_limit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuidedWinnabilityResult {
    pub winnable: bool,
    pub explored_states: usize,
    pub generated_states: usize,
    pub win_depth: Option<u32>,
    pub hit_state_limit: bool,
}

#[derive(Clone)]
struct GuidedNode {
    priority: i64,
    serial: u64,
    depth: u32,
    parent_hash: Option<u64>,
    state: KlondikeGame,
}

#[derive(Debug, Clone, Copy)]
enum SolverMove {
    Draw,
    WasteToFoundation,
    WasteToTableau {
        dst: usize,
    },
    TableauTopToFoundation,
    TableauRunToTableau {
        src: usize,
        start: usize,
        dst: usize,
    },
}

#[derive(Clone)]
struct ScoredSuccessor {
    state: KlondikeGame,
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
    pub fn new_shuffled() -> Self {
        let mut rng = rand::thread_rng();
        Self::new_with_seed(rng.gen())
    }

    pub fn new_with_seed(seed: u64) -> Self {
        let mut deck = full_deck();
        let mut rng = StdRng::seed_from_u64(seed);
        deck.shuffle(&mut rng);

        let mut game = Self {
            draw_mode: DrawMode::One,
            stock: Vec::new(),
            waste: Vec::new(),
            foundations: std::array::from_fn(|_| Vec::new()),
            tableau: std::array::from_fn(|_| Vec::new()),
        };

        let mut draw = deck.into_iter();
        for col in 0..7 {
            for row in 0..=col {
                let mut card = draw.next().expect("full deck has enough cards");
                card.face_up = row == col;
                game.tableau[col].push(card);
            }
        }

        for mut card in draw {
            card.face_up = false;
            game.stock.push(card);
        }

        game
    }

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

    pub fn draw_or_recycle_with_count(&mut self, draw_count: u8) -> DrawResult {
        if !self.stock.is_empty() {
            let draw_count = usize::from(draw_count.clamp(1, 5));
            for _ in 0..draw_count {
                let Some(mut card) = self.stock.pop() else {
                    break;
                };
                card.face_up = true;
                self.waste.push(card);
            }
            return DrawResult::DrewFromStock;
        }

        if self.waste.is_empty() {
            return DrawResult::NoOp;
        }

        while let Some(mut card) = self.waste.pop() {
            card.face_up = false;
            self.stock.push(card);
        }
        DrawResult::RecycledWaste
    }

    pub fn draw_or_recycle(&mut self) -> DrawResult {
        self.draw_or_recycle_with_count(self.draw_mode.count())
    }

    pub fn set_draw_mode(&mut self, mode: DrawMode) {
        self.draw_mode = mode;
    }

    pub fn draw_mode(&self) -> DrawMode {
        self.draw_mode
    }

    pub fn move_waste_to_foundation(&mut self) -> bool {
        let Some(card) = self.waste.last().copied() else {
            return false;
        };

        let idx = card.suit.foundation_index();
        if !can_stack_foundation(self.foundations[idx].last(), card) {
            return false;
        }

        self.waste.pop();
        self.foundations[idx].push(card);
        true
    }

    pub fn can_move_waste_to_foundation(&self) -> bool {
        let Some(card) = self.waste.last().copied() else {
            return false;
        };
        can_stack_foundation(self.foundations[card.suit.foundation_index()].last(), card)
    }

    pub fn move_waste_to_tableau(&mut self, dst: usize) -> bool {
        let Some(card) = self.waste.last().copied() else {
            return false;
        };

        if dst >= self.tableau.len() || !can_stack_tableau(self.tableau[dst].last(), card) {
            return false;
        }

        self.waste.pop();
        self.tableau[dst].push(card);
        true
    }

    pub fn can_move_waste_to_tableau(&self, dst: usize) -> bool {
        let Some(card) = self.waste.last().copied() else {
            return false;
        };
        if dst >= self.tableau.len() {
            return false;
        }
        can_stack_tableau(self.tableau[dst].last(), card)
    }

    pub fn move_tableau_top_to_foundation(&mut self, src: usize) -> bool {
        if src >= self.tableau.len() {
            return false;
        }

        let Some(card) = self.tableau[src].last().copied() else {
            return false;
        };

        if !card.face_up {
            return false;
        }

        let idx = card.suit.foundation_index();
        if !can_stack_foundation(self.foundations[idx].last(), card) {
            return false;
        }

        self.tableau[src].pop();
        self.foundations[idx].push(card);
        self.flip_top_tableau_if_needed(src);
        true
    }

    pub fn can_move_tableau_top_to_foundation(&self, src: usize) -> bool {
        if src >= self.tableau.len() {
            return false;
        }
        let Some(card) = self.tableau[src].last().copied() else {
            return false;
        };
        if !card.face_up {
            return false;
        }
        can_stack_foundation(self.foundations[card.suit.foundation_index()].last(), card)
    }

    pub fn can_move_foundation_top_to_tableau(&self, foundation_idx: usize, dst: usize) -> bool {
        if foundation_idx >= self.foundations.len() || dst >= self.tableau.len() {
            return false;
        }
        let Some(card) = self.foundations[foundation_idx].last().copied() else {
            return false;
        };
        can_stack_tableau(self.tableau[dst].last(), card)
    }

    pub fn move_foundation_top_to_tableau(&mut self, foundation_idx: usize, dst: usize) -> bool {
        if !self.can_move_foundation_top_to_tableau(foundation_idx, dst) {
            return false;
        }
        let Some(card) = self.foundations[foundation_idx].pop() else {
            return false;
        };
        self.tableau[dst].push(card);
        true
    }

    pub fn move_tableau_top_to_tableau(&mut self, src: usize, dst: usize) -> bool {
        if src == dst || src >= self.tableau.len() || dst >= self.tableau.len() {
            return false;
        }

        let Some(card) = self.tableau[src].last().copied() else {
            return false;
        };

        if !card.face_up || !can_stack_tableau(self.tableau[dst].last(), card) {
            return false;
        }

        self.tableau[src].pop();
        self.tableau[dst].push(card);
        self.flip_top_tableau_if_needed(src);
        true
    }

    pub fn can_move_tableau_run_to_tableau(&self, src: usize, start: usize, dst: usize) -> bool {
        if src == dst || src >= self.tableau.len() || dst >= self.tableau.len() {
            return false;
        }

        let source = &self.tableau[src];
        if start >= source.len() {
            return false;
        }

        let first = source[start];
        if !first.face_up {
            return false;
        }

        if !self.is_valid_face_up_run(source, start) {
            return false;
        }

        can_stack_tableau(self.tableau[dst].last(), first)
    }

    pub fn move_tableau_run_to_tableau(&mut self, src: usize, start: usize, dst: usize) -> bool {
        if !self.can_move_tableau_run_to_tableau(src, start, dst) {
            return false;
        }

        let moved = self.tableau[src].split_off(start);
        self.tableau[dst].extend(moved);
        self.flip_top_tableau_if_needed(src);
        true
    }

    pub fn can_move_tableau_top_to_tableau(&self, src: usize, dst: usize) -> bool {
        if src == dst || src >= self.tableau.len() || dst >= self.tableau.len() {
            return false;
        }
        let Some(card) = self.tableau[src].last().copied() else {
            return false;
        };
        card.face_up && can_stack_tableau(self.tableau[dst].last(), card)
    }

    pub fn tableau_top(&self, col: usize) -> Option<Card> {
        self.tableau.get(col).and_then(|pile| pile.last().copied())
    }

    pub fn tableau_len(&self, col: usize) -> Option<usize> {
        self.tableau.get(col).map(Vec::len)
    }

    pub fn tableau_card(&self, col: usize, index: usize) -> Option<Card> {
        self.tableau
            .get(col)
            .and_then(|pile| pile.get(index))
            .copied()
    }

    pub fn foundation_top_rank(&self, suit: Suit) -> u8 {
        self.foundations[suit.foundation_index()]
            .last()
            .map(|c| c.rank)
            .unwrap_or(0)
    }

    pub fn waste_top(&self) -> Option<Card> {
        self.waste.last().copied()
    }

    pub fn waste_top_n(&self, n: usize) -> Vec<Card> {
        if n == 0 {
            return Vec::new();
        }
        let mut cards: Vec<Card> = self.waste.iter().rev().take(n).copied().collect();
        cards.reverse();
        cards
    }

    pub fn stock_len(&self) -> usize {
        self.stock.len()
    }

    pub fn waste_len(&self) -> usize {
        self.waste.len()
    }

    pub fn foundations(&self) -> &[Vec<Card>; 4] {
        &self.foundations
    }

    pub fn tableau(&self) -> &[Vec<Card>; 7] {
        &self.tableau
    }

    pub fn cyclone_shuffle_tableau(&mut self) -> bool {
        let original = self.tableau.clone();
        let column_geometry: Vec<(usize, usize)> = self
            .tableau
            .iter()
            .map(|pile| {
                let face_down = pile.iter().filter(|card| !card.face_up).count();
                let face_up = pile.len().saturating_sub(face_down);
                (face_down, face_up)
            })
            .collect();

        let mut cards: Vec<Card> = self
            .tableau
            .iter()
            .flat_map(|pile| pile.iter().copied())
            .collect();
        if cards.len() < 2 {
            return false;
        }

        let mut rng = rand::thread_rng();
        for _ in 0..8 {
            cards.shuffle(&mut rng);

            let mut cursor = 0_usize;
            for (col, pile) in self.tableau.iter_mut().enumerate() {
                let (face_down, face_up) = column_geometry[col];
                let pile_len = face_down + face_up;
                pile.clear();
                for idx in 0..pile_len {
                    let mut card = cards[cursor];
                    cursor += 1;
                    card.face_up = idx >= face_down;
                    pile.push(card);
                }
            }

            if self.tableau != original {
                return true;
            }
        }

        false
    }

    pub fn is_won(&self) -> bool {
        self.foundations.iter().all(|pile| pile.len() == 13)
    }

    fn flip_top_tableau_if_needed(&mut self, col: usize) {
        if let Some(card) = self.tableau[col].last_mut() {
            card.face_up = true;
        }
    }

    fn is_valid_face_up_run(&self, source: &[Card], start: usize) -> bool {
        source[start..].windows(2).all(|pair| {
            let a = pair[0];
            let b = pair[1];
            a.face_up && b.face_up && a.color_red() != b.color_red() && a.rank == b.rank + 1
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
                raw.push((SolverMove::TableauTopToFoundation, state));
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
        SolverMove::WasteToFoundation | SolverMove::TableauTopToFoundation => {
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

fn full_deck() -> Vec<Card> {
    let mut deck = Vec::with_capacity(52);
    for suit in Suit::ALL {
        for rank in 1..=13 {
            deck.push(Card {
                suit,
                rank,
                face_up: false,
            });
        }
    }
    deck
}

fn can_stack_foundation(top: Option<&Card>, card: Card) -> bool {
    match top {
        None => card.rank == 1,
        Some(top_card) => top_card.suit == card.suit && card.rank == top_card.rank + 1,
    }
}

fn can_stack_tableau(top: Option<&Card>, card: Card) -> bool {
    match top {
        None => card.rank == 13,
        Some(top_card) => {
            top_card.face_up
                && top_card.color_red() != card.color_red()
                && top_card.rank == card.rank + 1
        }
    }
}

pub fn rank_label(rank: u8) -> &'static str {
    match rank {
        1 => "A",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "10",
        11 => "J",
        12 => "Q",
        13 => "K",
        _ => "?",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn card(suit: Suit, rank: u8, face_up: bool) -> Card {
        Card {
            suit,
            rank,
            face_up,
        }
    }

    fn empty_game() -> KlondikeGame {
        KlondikeGame {
            draw_mode: DrawMode::One,
            stock: Vec::new(),
            waste: Vec::new(),
            foundations: std::array::from_fn(|_| Vec::new()),
            tableau: std::array::from_fn(|_| Vec::new()),
        }
    }

    #[test]
    fn new_game_has_full_deck_accounted_for() {
        let game = KlondikeGame::new_shuffled();

        let tableau_count: usize = game.tableau.iter().map(Vec::len).sum();
        let foundations_count: usize = game.foundations.iter().map(Vec::len).sum();
        let total = game.stock.len() + game.waste.len() + foundations_count + tableau_count;

        assert_eq!(total, 52);
        assert_eq!(tableau_count, 28);
        assert_eq!(game.stock.len(), 24);
        assert_eq!(game.waste.len(), 0);
    }

    #[test]
    fn seeded_games_are_deterministic() {
        let game_a = KlondikeGame::new_with_seed(42);
        let game_b = KlondikeGame::new_with_seed(42);
        let game_c = KlondikeGame::new_with_seed(43);

        assert_eq!(game_a, game_b);
        assert_ne!(game_a, game_c);
    }

    #[test]
    fn draw_moves_one_card_from_stock_to_waste_face_up() {
        let mut game = empty_game();
        game.stock.push(card(Suit::Spades, 7, false));

        let result = game.draw_or_recycle();

        assert_eq!(result, DrawResult::DrewFromStock);
        assert_eq!(game.stock.len(), 0);
        assert_eq!(game.waste.len(), 1);
        assert!(game.waste[0].face_up);
        assert_eq!(game.waste[0].rank, 7);
    }

    #[test]
    fn draw_recycles_waste_back_to_stock_face_down() {
        let mut game = empty_game();
        game.waste.push(card(Suit::Hearts, 2, true));
        game.waste.push(card(Suit::Clubs, 9, true));

        let result = game.draw_or_recycle();

        assert_eq!(result, DrawResult::RecycledWaste);
        assert_eq!(game.waste.len(), 0);
        assert_eq!(game.stock.len(), 2);
        assert!(game.stock.iter().all(|c| !c.face_up));
    }

    #[test]
    fn draw_three_moves_up_to_three_cards_from_stock() {
        let mut game = empty_game();
        game.set_draw_mode(DrawMode::Three);
        game.stock.push(card(Suit::Clubs, 1, false));
        game.stock.push(card(Suit::Diamonds, 2, false));
        game.stock.push(card(Suit::Hearts, 3, false));
        game.stock.push(card(Suit::Spades, 4, false));

        let result = game.draw_or_recycle();

        assert_eq!(result, DrawResult::DrewFromStock);
        assert_eq!(game.stock.len(), 1);
        assert_eq!(game.waste.len(), 3);
        assert!(game.waste.iter().all(|card| card.face_up));
    }

    #[test]
    fn draw_three_with_low_stock_draws_remaining_cards_only() {
        let mut game = empty_game();
        game.set_draw_mode(DrawMode::Three);
        game.stock.push(card(Suit::Spades, 12, false));
        game.stock.push(card(Suit::Spades, 13, false));

        let result = game.draw_or_recycle();

        assert_eq!(result, DrawResult::DrewFromStock);
        assert_eq!(game.stock.len(), 0);
        assert_eq!(game.waste.len(), 2);
        assert!(game.waste.iter().all(|card| card.face_up));
    }

    #[test]
    fn waste_to_foundation_requires_ace_then_next_rank_same_suit() {
        let mut game = empty_game();
        game.waste.push(card(Suit::Clubs, 2, true));
        assert!(!game.move_waste_to_foundation());

        game.waste.clear();
        game.waste.push(card(Suit::Clubs, 1, true));
        assert!(game.move_waste_to_foundation());
        assert_eq!(game.foundations[Suit::Clubs.foundation_index()].len(), 1);

        game.waste.push(card(Suit::Clubs, 2, true));
        assert!(game.move_waste_to_foundation());
        assert_eq!(game.foundations[Suit::Clubs.foundation_index()].len(), 2);

        game.waste.push(card(Suit::Spades, 3, true));
        assert!(!game.move_waste_to_foundation());
    }

    #[test]
    fn waste_to_tableau_enforces_klondike_rules() {
        let mut game = empty_game();

        game.waste.push(card(Suit::Hearts, 13, true));
        assert!(game.move_waste_to_tableau(0));
        assert_eq!(game.tableau[0].len(), 1);

        game.waste.push(card(Suit::Diamonds, 12, true));
        assert!(!game.move_waste_to_tableau(0));

        game.waste.pop();
        game.waste.push(card(Suit::Spades, 12, true));
        assert!(game.move_waste_to_tableau(0));
        assert_eq!(game.tableau[0].len(), 2);
    }

    #[test]
    fn tableau_move_flips_new_top_card() {
        let mut game = empty_game();
        game.tableau[0].push(card(Suit::Clubs, 6, false));
        game.tableau[0].push(card(Suit::Hearts, 5, true));
        game.tableau[1].push(card(Suit::Spades, 6, true));

        assert!(game.move_tableau_top_to_tableau(0, 1));
        assert!(game.tableau[0][0].face_up);
        assert_eq!(game.tableau[1].last().map(|c| c.rank), Some(5));
    }

    #[test]
    fn tableau_to_foundation_rejects_face_down_cards() {
        let mut game = empty_game();
        game.tableau[0].push(card(Suit::Diamonds, 1, false));

        assert!(!game.move_tableau_top_to_foundation(0));
        assert!(game.foundations[Suit::Diamonds.foundation_index()].is_empty());
    }

    #[test]
    fn tableau_run_move_requires_valid_face_up_sequence() {
        let mut game = empty_game();
        game.tableau[0].push(card(Suit::Spades, 9, false));
        game.tableau[0].push(card(Suit::Hearts, 8, true));
        game.tableau[0].push(card(Suit::Clubs, 7, true));
        game.tableau[1].push(card(Suit::Clubs, 9, true));

        assert!(game.can_move_tableau_run_to_tableau(0, 1, 1));
        assert!(game.move_tableau_run_to_tableau(0, 1, 1));
        assert_eq!(game.tableau[0].len(), 1);
        assert!(game.tableau[0][0].face_up);
        assert_eq!(game.tableau[1].len(), 3);
        assert_eq!(game.tableau[1][1].rank, 8);
        assert_eq!(game.tableau[1][2].rank, 7);
    }

    #[test]
    fn tableau_run_move_rejects_invalid_start() {
        let mut game = empty_game();
        game.tableau[0].push(card(Suit::Spades, 9, true));
        game.tableau[0].push(card(Suit::Hearts, 8, true));
        game.tableau[0].push(card(Suit::Diamonds, 7, true)); // invalid color sequence
        game.tableau[1].push(card(Suit::Diamonds, 10, true));

        assert!(!game.can_move_tableau_run_to_tableau(0, 0, 1));
        assert!(!game.move_tableau_run_to_tableau(0, 0, 1));
    }

    #[test]
    fn rank_labels_are_correct() {
        assert_eq!(rank_label(1), "A");
        assert_eq!(rank_label(11), "J");
        assert_eq!(rank_label(12), "Q");
        assert_eq!(rank_label(13), "K");
        assert_eq!(rank_label(99), "?");
    }

    #[test]
    fn winnability_marks_completed_game_as_won() {
        let mut game = empty_game();
        for suit in Suit::ALL {
            let foundation = &mut game.foundations[suit.foundation_index()];
            for rank in 1..=13 {
                foundation.push(card(suit, rank, true));
            }
        }

        let result = game.analyze_winnability(10);
        assert!(result.winnable);
        assert!(!result.hit_state_limit);
        assert_eq!(result.explored_states, 1);
    }

    #[test]
    fn winnability_honors_state_limit() {
        let game = KlondikeGame::new_with_seed(7);
        let result = game.analyze_winnability(0);
        assert!(!result.winnable);
        assert!(result.hit_state_limit);
        assert_eq!(result.explored_states, 0);
    }

    #[test]
    fn cyclone_shuffle_preserves_tableau_geometry_and_card_set() {
        let mut game = empty_game();
        game.tableau[0] = vec![card(Suit::Clubs, 2, true)];
        game.tableau[1] = vec![card(Suit::Hearts, 10, false), card(Suit::Spades, 5, true)];
        game.tableau[2] = vec![
            card(Suit::Diamonds, 9, false),
            card(Suit::Clubs, 4, false),
            card(Suit::Hearts, 3, true),
        ];
        game.tableau[3] = vec![];
        game.tableau[4] = vec![
            card(Suit::Spades, 8, false),
            card(Suit::Diamonds, 7, true),
            card(Suit::Clubs, 6, true),
        ];
        game.tableau[5] = vec![card(Suit::Hearts, 1, true), card(Suit::Spades, 13, true)];
        game.tableau[6] = vec![card(Suit::Diamonds, 12, false)];

        let before_geometry: Vec<(usize, usize)> = game
            .tableau
            .iter()
            .map(|pile| {
                let down = pile.iter().filter(|card| !card.face_up).count();
                let up = pile.iter().filter(|card| card.face_up).count();
                (down, up)
            })
            .collect();

        let mut before_cards: std::collections::HashMap<(Suit, u8), usize> =
            std::collections::HashMap::new();
        for card in game.tableau.iter().flat_map(|pile| pile.iter()) {
            *before_cards.entry((card.suit, card.rank)).or_insert(0) += 1;
        }

        let _ = game.cyclone_shuffle_tableau();

        let after_geometry: Vec<(usize, usize)> = game
            .tableau
            .iter()
            .map(|pile| {
                let down = pile.iter().filter(|card| !card.face_up).count();
                let up = pile.iter().filter(|card| card.face_up).count();
                (down, up)
            })
            .collect();
        assert_eq!(after_geometry, before_geometry);

        let mut after_cards: std::collections::HashMap<(Suit, u8), usize> =
            std::collections::HashMap::new();
        for card in game.tableau.iter().flat_map(|pile| pile.iter()) {
            *after_cards.entry((card.suit, card.rank)).or_insert(0) += 1;
        }
        assert_eq!(after_cards, before_cards);
    }

    #[test]
    fn cyclone_shuffle_noops_for_tiny_tableau() {
        let mut game = empty_game();
        game.tableau[0].push(card(Suit::Clubs, 1, true));
        assert!(!game.cyclone_shuffle_tableau());
    }

    #[test]
    fn game_mode_metadata_is_stable() {
        assert_eq!(GameMode::from_id("klondike"), Some(GameMode::Klondike));
        assert_eq!(GameMode::from_id("spider"), Some(GameMode::Spider));
        assert_eq!(GameMode::from_id("freecell"), Some(GameMode::Freecell));
        assert_eq!(GameMode::from_id("unknown"), None);

        assert_eq!(GameMode::Klondike.label(), "Klondike");
        assert_eq!(GameMode::Spider.label(), "Spider");
        assert_eq!(GameMode::Freecell.label(), "FreeCell");
        assert_eq!(GameMode::Klondike.emoji(), "🥇");
        assert_eq!(GameMode::Spider.emoji(), "🕷️");
        assert_eq!(GameMode::Freecell.emoji(), "🗽");
        assert!(GameMode::Klondike.engine_ready());
        assert!(!GameMode::Spider.engine_ready());
        assert!(!GameMode::Freecell.engine_ready());
    }
}

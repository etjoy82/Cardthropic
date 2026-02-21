use crate::game::{Card, FreecellCardCountMode, FreecellGame, Suit, FREECELL_MAX_CELL_COUNT};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::sync::{
    atomic::{AtomicBool, Ordering as AtomicOrdering},
    OnceLock,
};
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FreecellPlannerAction {
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

#[derive(Debug, Clone, Copy)]
pub struct FreecellPlannerConfig {
    pub max_depth: u8,
    pub branch_beam: usize,
    pub node_budget: usize,
    pub time_budget_ms: u64,
}

#[derive(Debug, Clone)]
pub struct FreecellPlannerResult {
    pub actions: VecDeque<FreecellPlannerAction>,
    pub explored_states: usize,
    pub stalled: bool,
    pub stale_skips: usize,
    pub inverse_prunes: usize,
    pub inverse_checked: usize,
    pub branch_total: usize,
    pub expanded_nodes: usize,
    pub expanded_h_sum: u64,
    pub expanded_tb_sum: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannerLoc {
    Col(u8),
    Free(u8),
    Found(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlannerMove {
    from: PlannerLoc,
    to: PlannerLoc,
    count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PlannerIllegalMove {
    CountNotOne,
    SameLocation,
    EmptySource,
    NonEmptyFreecellDest,
    BadFoundationDestSuit,
    BadFoundationDestRank,
    BadTableauStack,
    FoundationMoveNotAllowed,
    FreeToFreeNotGenerated,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct PlannerUndo {
    mv: PlannerMove,
    card: Card,
    old_zhash: u64,
}

#[derive(Clone)]
struct PlannerState {
    foundations: [u8; 4],
    freecell_count: u8,
    freecells: [Option<Card>; FREECELL_SLOT_MAX],
    cols: [Vec<Card>; 8],
    zhash: u64,
}

struct PlannerZobrist {
    tab: Vec<u64>,
    fc_any: [u64; 52],
    found: [[u64; 14]; 4],
}

impl PlannerZobrist {
    fn new() -> Self {
        let mut tab = vec![0_u64; 8 * 52 * 52];
        for col in 0..8 {
            for depth in 0..52 {
                for card in 0..52 {
                    let idx = tab_index(col, depth, card);
                    tab[idx] = splitmix64(
                        0x5441_4200_0000_0000_u64
                            ^ ((col as u64) << 20)
                            ^ ((depth as u64) << 8)
                            ^ (card as u64),
                    );
                }
            }
        }
        let mut fc_any = [0_u64; 52];
        for (card, slot) in fc_any.iter_mut().enumerate() {
            *slot = splitmix64(0x4652_4545_0000_0000_u64 ^ card as u64);
        }
        let mut found = [[0_u64; 14]; 4];
        for (suit_idx, suit_vals) in found.iter_mut().enumerate() {
            for (rank, slot) in suit_vals.iter_mut().enumerate() {
                *slot =
                    splitmix64(0x464F_554E_0000_0000_u64 ^ ((suit_idx as u64) << 10) ^ rank as u64);
            }
        }
        Self { tab, fc_any, found }
    }
}

static PLANNER_ZOBRIST: OnceLock<PlannerZobrist> = OnceLock::new();

fn planner_zobrist() -> &'static PlannerZobrist {
    PLANNER_ZOBRIST.get_or_init(PlannerZobrist::new)
}

#[derive(Clone)]
struct Candidate {
    action: FreecellPlannerAction,
    next: FreecellGame,
    score: i64,
    class: MoveClass,
    ctx: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum MoveClass {
    SafeFoundation = 0,
    Foundation = 1,
    Relocation = 2,
    FreecellToTableau = 3,
    TableauRunToTableau = 4,
    TableauToFreecell = 5,
}

#[derive(Clone)]
struct Node {
    game: FreecellGame,
    hash: u64,
    g: u16,
    priority: i64,
    total_score: i64,
    foundation: usize,
    path: Vec<FreecellPlannerAction>,
}

const FREECELL_SLOT_MAX: usize = FREECELL_MAX_CELL_COUNT as usize;
const PLANNER_KEY_BYTES: usize = 4 + FREECELL_SLOT_MAX + 8 + 52;
const PLANNER_KEY_EMPTY: u8 = 0xFF;

#[derive(Clone, Copy)]
struct PlannerKey {
    bytes: [u8; PLANNER_KEY_BYTES],
    z: u64,
}

impl PartialEq for PlannerKey {
    fn eq(&self, other: &Self) -> bool {
        self.bytes == other.bytes
    }
}

impl Eq for PlannerKey {}

impl Hash for PlannerKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.z);
    }
}

impl Eq for Node {}
impl PartialEq for Node {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.g == other.g && self.hash == other.hash
    }
}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        self.priority
            .cmp(&other.priority)
            .then_with(|| other.g.cmp(&self.g))
            .then_with(|| self.hash.cmp(&other.hash))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

fn plan_line_impl(
    start: &FreecellGame,
    seen_states: &HashSet<u64>,
    config: FreecellPlannerConfig,
    cancel: Option<&AtomicBool>,
    enable_ida_fallback: bool,
) -> FreecellPlannerResult {
    let mut stale_skips = 0usize;
    let mut inverse_prunes = 0usize;
    let mut inverse_checked = 0usize;
    let mut branch_total = 0usize;
    let mut expanded_nodes = 0usize;

    if start.is_won() {
        return FreecellPlannerResult {
            actions: VecDeque::new(),
            explored_states: 0,
            stalled: false,
            stale_skips,
            inverse_prunes,
            inverse_checked,
            branch_total,
            expanded_nodes,
            expanded_h_sum: 0,
            expanded_tb_sum: 0,
        };
    }

    // Always take an immediate winning move when one exists.
    let mut immediate_wins = generate_atomic_candidates(start, None)
        .into_iter()
        .filter(|candidate| candidate.next.is_won())
        .collect::<Vec<_>>();
    if immediate_wins.is_empty() {
        immediate_wins = generate_candidates(start, None)
            .into_iter()
            .filter(|candidate| candidate.next.is_won())
            .collect::<Vec<_>>();
    }
    if let Some(chosen) = immediate_wins.into_iter().max_by(|a, b| {
        a.score
            .cmp(&b.score)
            .then_with(|| b.class.cmp(&a.class))
            .then_with(|| b.ctx.cmp(&a.ctx))
    }) {
        return FreecellPlannerResult {
            actions: VecDeque::from([chosen.action]),
            explored_states: 1,
            stalled: false,
            stale_skips,
            inverse_prunes,
            inverse_checked,
            branch_total,
            expanded_nodes,
            expanded_h_sum: 0,
            expanded_tb_sum: 0,
        };
    }

    if let Some(actions) = greedy_safe_foundation_pass(start, 6) {
        return FreecellPlannerResult {
            actions,
            explored_states: 1,
            stalled: false,
            stale_skips,
            inverse_prunes,
            inverse_checked,
            branch_total,
            expanded_nodes,
            expanded_h_sum: 0,
            expanded_tb_sum: 0,
        };
    }

    let start_key = make_planner_key(start);
    let start_hash = start_key.z;
    let start_foundation = foundation_cards(start);
    let mut frontier = BinaryHeap::new();
    frontier.push(Node {
        game: start.clone(),
        hash: start_hash,
        g: 0,
        priority: heuristic(start),
        total_score: heuristic(start),
        foundation: start_foundation,
        path: Vec::new(),
    });

    let mut best_g = HashMap::<PlannerKey, u16>::new();
    let mut best_score = HashMap::<PlannerKey, i64>::new();
    best_g.insert(start_key, 0);
    best_score.insert(start_key, heuristic(start));

    let started_at = Instant::now();
    let mut explored = 0usize;
    while let Some(node) = frontier.pop() {
        let node_key = make_planner_key(&node.game);
        if best_g.get(&node_key).is_some_and(|best| node.g > *best) {
            stale_skips = stale_skips.saturating_add(1);
            continue;
        }
        if cancel.is_some_and(|flag| flag.load(AtomicOrdering::Relaxed)) {
            return FreecellPlannerResult {
                actions: VecDeque::new(),
                explored_states: explored,
                stalled: true,
                stale_skips,
                inverse_prunes,
                inverse_checked,
                branch_total,
                expanded_nodes,
                expanded_h_sum: 0,
                expanded_tb_sum: 0,
            };
        }
        if explored >= config.node_budget {
            break;
        }
        if started_at.elapsed().as_millis() >= u128::from(config.time_budget_ms) {
            break;
        }
        explored = explored.saturating_add(1);

        let node = compress_node_by_safe_foundation(node, 12);

        if node.game.is_won() {
            return FreecellPlannerResult {
                actions: VecDeque::from(node.path),
                explored_states: explored,
                stalled: false,
                stale_skips,
                inverse_prunes,
                inverse_checked,
                branch_total,
                expanded_nodes,
                expanded_h_sum: 0,
                expanded_tb_sum: 0,
            };
        }
        if node.foundation > start_foundation && !node.path.is_empty() {
            return FreecellPlannerResult {
                actions: VecDeque::from(node.path),
                explored_states: explored,
                stalled: false,
                stale_skips,
                inverse_prunes,
                inverse_checked,
                branch_total,
                expanded_nodes,
                expanded_h_sum: 0,
                expanded_tb_sum: 0,
            };
        }
        if node.path.len() >= usize::from(config.max_depth) {
            continue;
        }

        let last_action = node.path.last().copied();
        let mut candidates = generate_atomic_candidates(&node.game, last_action);
        let relocation = generate_relocation_candidates(&node.game);
        if !relocation.is_empty() {
            candidates.extend(relocation);
        }
        if candidates.len() < config.branch_beam {
            candidates.extend(generate_candidates(&node.game, last_action));
        }
        candidates = dedup_candidates_by_action(candidates);
        let mut atomic = generate_atomic_candidates(&node.game, last_action);
        if atomic.len() < config.branch_beam {
            atomic.extend(candidates);
            candidates = dedup_candidates_by_action(atomic);
        }
        expanded_nodes = expanded_nodes.saturating_add(1);
        if candidates.is_empty() {
            continue;
        }
        candidates.sort_by(|a, b| {
            b.score
                .cmp(&a.score)
                .then_with(|| a.class.cmp(&b.class))
                .then_with(|| a.ctx.cmp(&b.ctx))
        });
        candidates = select_candidates_with_class_caps(candidates, config.branch_beam.max(4));
        branch_total = branch_total.saturating_add(candidates.len());

        for child in candidates {
            inverse_checked = inverse_checked.saturating_add(1);
            if node
                .path
                .last()
                .copied()
                .is_some_and(|prev| actions_are_inverse(prev, child.action))
            {
                inverse_prunes = inverse_prunes.saturating_add(1);
                continue;
            }
            let next_key = make_planner_key(&child.next);
            let next_hash = next_key.z;
            let next_g = node.g.saturating_add(1);
            let child_total = node.total_score + child.score + heuristic(&child.next);
            if seen_states.contains(&next_hash) {
                continue;
            }
            if best_g.get(&next_key).is_some_and(|best| *best <= next_g)
                && best_score
                    .get(&next_key)
                    .is_some_and(|best| *best >= child_total)
            {
                continue;
            }
            let mut path = node.path.clone();
            path.push(child.action);
            let foundation = foundation_cards(&child.next);
            let priority = child_total - i64::from(next_g) * 32;
            best_g.insert(next_key, next_g);
            best_score
                .entry(next_key)
                .and_modify(|best| {
                    if child_total > *best {
                        *best = child_total;
                    }
                })
                .or_insert(child_total);
            frontier.push(Node {
                game: child.next,
                hash: next_hash,
                g: next_g,
                priority,
                total_score: child_total,
                foundation,
                path,
            });
        }
    }

    if enable_ida_fallback {
        let fallback_config = FreecellPlannerConfig {
            max_depth: config.max_depth,
            branch_beam: config.branch_beam,
            node_budget: (config.node_budget / 2).max(1),
            time_budget_ms: (config.time_budget_ms / 2).max(1),
        };
        let mut ida = plan_line_ida(start, seen_states, fallback_config, cancel);
        ida.explored_states = ida.explored_states.saturating_add(explored);
        ida.stale_skips = ida.stale_skips.saturating_add(stale_skips);
        ida.inverse_prunes = ida.inverse_prunes.saturating_add(inverse_prunes);
        ida.inverse_checked = ida.inverse_checked.saturating_add(inverse_checked);
        ida.branch_total = ida.branch_total.saturating_add(branch_total);
        ida.expanded_nodes = ida.expanded_nodes.saturating_add(expanded_nodes);
        if !ida.stalled || !ida.actions.is_empty() {
            return ida;
        }
        return FreecellPlannerResult {
            actions: VecDeque::new(),
            explored_states: ida.explored_states,
            stalled: true,
            stale_skips: ida.stale_skips,
            inverse_prunes: ida.inverse_prunes,
            inverse_checked: ida.inverse_checked,
            branch_total: ida.branch_total,
            expanded_nodes: ida.expanded_nodes,
            expanded_h_sum: ida.expanded_h_sum,
            expanded_tb_sum: ida.expanded_tb_sum,
        };
    }

    FreecellPlannerResult {
        actions: VecDeque::new(),
        explored_states: explored,
        stalled: true,
        stale_skips,
        inverse_prunes,
        inverse_checked,
        branch_total,
        expanded_nodes,
        expanded_h_sum: 0,
        expanded_tb_sum: 0,
    }
}

pub fn plan_line_astar_only(
    start: &FreecellGame,
    seen_states: &HashSet<u64>,
    config: FreecellPlannerConfig,
    cancel: Option<&AtomicBool>,
) -> FreecellPlannerResult {
    plan_line_impl(start, seen_states, config, cancel, false)
}

pub fn plan_line_ida_with_astar_fallback(
    start: &FreecellGame,
    seen_states: &HashSet<u64>,
    config: FreecellPlannerConfig,
    cancel: Option<&AtomicBool>,
) -> FreecellPlannerResult {
    let ida = plan_line_ida(start, seen_states, config, cancel);
    if !ida.stalled || !ida.actions.is_empty() {
        return ida;
    }

    let fallback_config = FreecellPlannerConfig {
        max_depth: config.max_depth,
        branch_beam: config.branch_beam,
        node_budget: (config.node_budget / 2).max(1),
        time_budget_ms: (config.time_budget_ms / 2).max(1),
    };
    let mut astar = plan_line_astar_only(start, seen_states, fallback_config, cancel);
    astar.explored_states = astar.explored_states.saturating_add(ida.explored_states);
    astar.stale_skips = astar.stale_skips.saturating_add(ida.stale_skips);
    astar.inverse_prunes = astar.inverse_prunes.saturating_add(ida.inverse_prunes);
    astar.inverse_checked = astar.inverse_checked.saturating_add(ida.inverse_checked);
    astar.branch_total = astar.branch_total.saturating_add(ida.branch_total);
    astar.expanded_nodes = astar.expanded_nodes.saturating_add(ida.expanded_nodes);
    astar.expanded_h_sum = astar.expanded_h_sum.saturating_add(ida.expanded_h_sum);
    astar.expanded_tb_sum = astar.expanded_tb_sum.saturating_add(ida.expanded_tb_sum);
    astar
}

pub fn zobrist_hash(game: &FreecellGame) -> u64 {
    let mut hash = 0_u64;
    let tableau_base = 0usize; // 8 * 52 slots
    let freecell_base = tableau_base + (8 * 52);
    let foundation_base = freecell_base + FREECELL_SLOT_MAX;

    // Keep tableau column identity stable in the hash (no column sorting).
    for (col, pile) in game.tableau().iter().enumerate() {
        for (depth, card) in pile.iter().enumerate() {
            let card_idx = zobrist_card_index(*card);
            let pos_idx = tableau_base + (col * 52) + depth.min(51);
            hash ^= zobrist_value(card_idx, pos_idx);
        }
        hash ^= splitmix64(0xC011_EA00_u64 ^ ((col as u64) << 8) ^ (pile.len() as u64));
    }

    // Canonicalize freecells by treating them as an unordered multiset.
    let mut fc_cards: Vec<Card> = game.freecells().iter().copied().flatten().collect();
    fc_cards.sort_by_key(|card| (card.suit.foundation_index(), card.rank));
    for (idx, card) in fc_cards.iter().enumerate() {
        let card_idx = zobrist_card_index(*card);
        let pos_idx = freecell_base + idx.min(FREECELL_SLOT_MAX.saturating_sub(1));
        hash ^= zobrist_value(card_idx, pos_idx);
    }

    for (f_idx, pile) in game.foundations().iter().enumerate() {
        for (depth, card) in pile.iter().enumerate() {
            let card_idx = zobrist_card_index(*card);
            let pos_idx = foundation_base + f_idx * 13 + depth.min(12);
            hash ^= zobrist_value(card_idx, pos_idx);
        }
    }
    hash
}

fn make_planner_key(game: &FreecellGame) -> PlannerKey {
    let bytes = pack_state_key(game);
    PlannerKey {
        z: packed_key_hash64(&bytes),
        bytes,
    }
}

fn make_planner_key_from_state(st: &PlannerState) -> PlannerKey {
    let bytes = pack_planner_state_key(st);
    PlannerKey { z: st.zhash, bytes }
}

fn pack_state_key(game: &FreecellGame) -> [u8; PLANNER_KEY_BYTES] {
    let mut bytes = [PLANNER_KEY_EMPTY; PLANNER_KEY_BYTES];
    let freecell_offset = 4usize;
    let tableau_len_offset = freecell_offset + FREECELL_SLOT_MAX;
    let tableau_cards_offset = tableau_len_offset + 8;

    for suit in 0..4 {
        bytes[suit] = game
            .foundations()
            .get(suit)
            .map(Vec::len)
            .unwrap_or(0)
            .min(13) as u8;
    }

    let mut freecell_cards: Vec<u8> = game
        .freecells()
        .iter()
        .copied()
        .flatten()
        .map(card_to_u8)
        .collect();
    freecell_cards.sort_unstable();
    for (idx, card) in freecell_cards
        .into_iter()
        .take(FREECELL_SLOT_MAX)
        .enumerate()
    {
        bytes[freecell_offset + idx] = card;
    }

    // Canonicalize tableau columns in key view only (safe for replay since state is unchanged).
    let mut canonical_cols: Vec<Vec<u8>> = game
        .tableau()
        .iter()
        .map(|col| col.iter().copied().map(card_to_u8).collect::<Vec<u8>>())
        .collect();
    canonical_cols.sort_unstable();

    for col in 0..8 {
        let len = canonical_cols.get(col).map(Vec::len).unwrap_or(0).min(52);
        bytes[tableau_len_offset + col] = len as u8;
    }

    let mut out = tableau_cards_offset;
    for cards in canonical_cols.iter().take(8) {
        for &card in cards {
            if out >= PLANNER_KEY_BYTES {
                break;
            }
            bytes[out] = card;
            out += 1;
        }
        if out >= PLANNER_KEY_BYTES {
            break;
        }
    }

    bytes
}

fn pack_planner_state_key(st: &PlannerState) -> [u8; PLANNER_KEY_BYTES] {
    let mut bytes = [PLANNER_KEY_EMPTY; PLANNER_KEY_BYTES];
    let freecell_offset = 4usize;
    let tableau_len_offset = freecell_offset + FREECELL_SLOT_MAX;
    let tableau_cards_offset = tableau_len_offset + 8;

    bytes[..4].copy_from_slice(&st.foundations);

    let mut freecell_cards: Vec<u8> = st
        .freecells
        .iter()
        .take(st.freecell_count as usize)
        .filter_map(|slot| slot.map(card_to_u8))
        .collect();
    freecell_cards.sort_unstable();
    for (idx, card) in freecell_cards
        .into_iter()
        .take(FREECELL_SLOT_MAX)
        .enumerate()
    {
        bytes[freecell_offset + idx] = card;
    }

    let mut canonical_cols: Vec<Vec<u8>> = st
        .cols
        .iter()
        .map(|col| col.iter().copied().map(card_to_u8).collect::<Vec<u8>>())
        .collect();
    canonical_cols.sort_unstable();

    for col in 0..8 {
        let len = canonical_cols.get(col).map(Vec::len).unwrap_or(0).min(52);
        bytes[tableau_len_offset + col] = len as u8;
    }

    let mut out = tableau_cards_offset;
    for cards in canonical_cols.iter().take(8) {
        for &card in cards {
            if out >= PLANNER_KEY_BYTES {
                break;
            }
            bytes[out] = card;
            out += 1;
        }
        if out >= PLANNER_KEY_BYTES {
            break;
        }
    }

    bytes
}

fn packed_key_hash64(bytes: &[u8; PLANNER_KEY_BYTES]) -> u64 {
    let mut h = 0xD1A5_10B1_5EED_1234_u64;
    let mut idx = 0usize;
    while idx < bytes.len() {
        let mut chunk = [0u8; 8];
        let end = (idx + 8).min(bytes.len());
        let len = end - idx;
        chunk[..len].copy_from_slice(&bytes[idx..end]);
        let word = u64::from_le_bytes(chunk);
        h ^= splitmix64(word ^ ((idx as u64) << 1));
        idx = end;
    }
    splitmix64(h ^ (PLANNER_KEY_BYTES as u64))
}

fn tab_index(col: usize, depth: usize, card: usize) -> usize {
    (col * 52 + depth) * 52 + card
}

fn suit_from_index(idx: usize) -> Suit {
    match idx {
        0 => Suit::Clubs,
        1 => Suit::Diamonds,
        2 => Suit::Hearts,
        _ => Suit::Spades,
    }
}

fn card_id(card: Card) -> usize {
    card.suit.foundation_index() * 13 + usize::from(card.rank.saturating_sub(1))
}

fn card_from_suit_rank(suit_idx: usize, rank: u8) -> Card {
    Card {
        suit: suit_from_index(suit_idx),
        rank,
        face_up: true,
    }
}

fn can_stack_on_tableau(moving: Card, dest_top: Card) -> bool {
    moving.rank + 1 == dest_top.rank && moving.color_red() != dest_top.color_red()
}

fn can_move_to_foundation(card: Card, foundations: &[u8; 4]) -> bool {
    foundations[card.suit.foundation_index()] + 1 == card.rank
}

fn planner_state_from_game(game: &FreecellGame) -> PlannerState {
    let z = planner_zobrist();
    let mut freecells = [None; FREECELL_SLOT_MAX];
    for (idx, slot) in game.freecells().iter().copied().enumerate() {
        if idx >= freecells.len() {
            break;
        }
        freecells[idx] = slot;
    }
    let mut st = PlannerState {
        foundations: [0; 4],
        freecell_count: game.freecell_count() as u8,
        freecells,
        cols: game.tableau().clone(),
        zhash: 0,
    };
    for suit_idx in 0..4 {
        st.foundations[suit_idx] = game
            .foundations()
            .get(suit_idx)
            .map(Vec::len)
            .unwrap_or(0)
            .min(13) as u8;
        st.zhash ^= z.found[suit_idx][st.foundations[suit_idx] as usize];
    }
    for card in st
        .freecells
        .iter()
        .take(st.freecell_count as usize)
        .flatten()
    {
        st.zhash ^= z.fc_any[card_id(*card)];
    }
    for (col, pile) in st.cols.iter().enumerate() {
        for (depth, card) in pile.iter().enumerate() {
            st.zhash ^= z.tab[tab_index(col, depth, card_id(*card))];
        }
    }
    st
}

fn planner_state_to_game(
    st: &PlannerState,
    card_count_mode: FreecellCardCountMode,
) -> FreecellGame {
    let foundations = std::array::from_fn(|suit_idx| {
        let top_rank = st.foundations[suit_idx];
        let mut pile = Vec::with_capacity(top_rank as usize);
        for rank in 1..=top_rank {
            pile.push(card_from_suit_rank(suit_idx, rank));
        }
        pile
    });
    FreecellGame::from_parts_unchecked_with_cell_count(
        card_count_mode,
        foundations,
        st.freecell_count,
        st.freecells,
        st.cols.clone(),
    )
}

fn peek_source_card(
    st: &PlannerState,
    from: PlannerLoc,
    allow_found_as_source: bool,
) -> Result<Card, PlannerIllegalMove> {
    match from {
        PlannerLoc::Col(ci) => st.cols[ci as usize]
            .last()
            .copied()
            .ok_or(PlannerIllegalMove::EmptySource),
        PlannerLoc::Free(fi) => {
            let idx = fi as usize;
            if idx >= st.freecell_count as usize {
                return Err(PlannerIllegalMove::EmptySource);
            }
            st.freecells[idx].ok_or(PlannerIllegalMove::EmptySource)
        }
        PlannerLoc::Found(su) => {
            if !allow_found_as_source {
                return Err(PlannerIllegalMove::FoundationMoveNotAllowed);
            }
            let suit_idx = su as usize;
            let top_rank = st.foundations[suit_idx];
            if top_rank == 0 {
                return Err(PlannerIllegalMove::EmptySource);
            }
            Ok(card_from_suit_rank(suit_idx, top_rank))
        }
    }
}

fn check_dest_legal(
    st: &PlannerState,
    to: PlannerLoc,
    card: Card,
) -> Result<(), PlannerIllegalMove> {
    match to {
        PlannerLoc::Col(ci) => {
            if let Some(dest_top) = st.cols[ci as usize].last().copied() {
                if !can_stack_on_tableau(card, dest_top) {
                    return Err(PlannerIllegalMove::BadTableauStack);
                }
            }
            Ok(())
        }
        PlannerLoc::Free(fi) => {
            let idx = fi as usize;
            if idx >= st.freecell_count as usize {
                return Err(PlannerIllegalMove::NonEmptyFreecellDest);
            }
            if st.freecells[idx].is_some() {
                return Err(PlannerIllegalMove::NonEmptyFreecellDest);
            }
            Ok(())
        }
        PlannerLoc::Found(su) => {
            let suit_idx = su as usize;
            if card.suit.foundation_index() != suit_idx {
                return Err(PlannerIllegalMove::BadFoundationDestSuit);
            }
            if !can_move_to_foundation(card, &st.foundations) {
                return Err(PlannerIllegalMove::BadFoundationDestRank);
            }
            Ok(())
        }
    }
}

fn apply_move_in_place(
    st: &mut PlannerState,
    mv: PlannerMove,
    allow_found_as_source: bool,
) -> Result<PlannerUndo, PlannerIllegalMove> {
    if mv.count != 1 {
        return Err(PlannerIllegalMove::CountNotOne);
    }
    if mv.from == mv.to {
        return Err(PlannerIllegalMove::SameLocation);
    }
    if matches!(mv.from, PlannerLoc::Free(_)) && matches!(mv.to, PlannerLoc::Free(_)) {
        return Err(PlannerIllegalMove::FreeToFreeNotGenerated);
    }

    let card = peek_source_card(st, mv.from, allow_found_as_source)?;
    check_dest_legal(st, mv.to, card)?;

    let z = planner_zobrist();
    let old_zhash = st.zhash;

    match mv.from {
        PlannerLoc::Col(ci) => {
            let c = ci as usize;
            let depth = st.cols[c].len().saturating_sub(1);
            let popped = st.cols[c].pop().ok_or(PlannerIllegalMove::EmptySource)?;
            debug_assert_eq!(popped, card);
            st.zhash ^= z.tab[tab_index(c, depth, card_id(card))];
        }
        PlannerLoc::Free(fi) => {
            let f = fi as usize;
            let taken = st.freecells[f]
                .take()
                .ok_or(PlannerIllegalMove::EmptySource)?;
            debug_assert_eq!(taken, card);
            st.zhash ^= z.fc_any[card_id(card)];
        }
        PlannerLoc::Found(su) => {
            let s = su as usize;
            let r = st.foundations[s];
            if r == 0 {
                return Err(PlannerIllegalMove::EmptySource);
            }
            st.zhash ^= z.found[s][r as usize];
            st.foundations[s] = r - 1;
            st.zhash ^= z.found[s][st.foundations[s] as usize];
        }
    }

    match mv.to {
        PlannerLoc::Col(ci) => {
            let c = ci as usize;
            let depth = st.cols[c].len();
            st.cols[c].push(card);
            st.zhash ^= z.tab[tab_index(c, depth, card_id(card))];
        }
        PlannerLoc::Free(fi) => {
            let f = fi as usize;
            if st.freecells[f].is_some() {
                return Err(PlannerIllegalMove::NonEmptyFreecellDest);
            }
            st.freecells[f] = Some(card);
            st.zhash ^= z.fc_any[card_id(card)];
        }
        PlannerLoc::Found(su) => {
            let s = su as usize;
            let old_r = st.foundations[s] as usize;
            let new_r = old_r + 1;
            st.zhash ^= z.found[s][old_r];
            st.foundations[s] = new_r as u8;
            st.zhash ^= z.found[s][new_r];
        }
    }

    Ok(PlannerUndo {
        mv,
        card,
        old_zhash,
    })
}

fn undo_move_in_place(st: &mut PlannerState, undo: PlannerUndo) {
    let mv = undo.mv;
    let card = undo.card;

    match mv.to {
        PlannerLoc::Col(ci) => {
            let c = ci as usize;
            let popped = st.cols[c].pop();
            debug_assert_eq!(popped, Some(card));
        }
        PlannerLoc::Free(fi) => {
            let f = fi as usize;
            let taken = st.freecells[f].take();
            debug_assert_eq!(taken, Some(card));
        }
        PlannerLoc::Found(su) => {
            let s = su as usize;
            st.foundations[s] = st.foundations[s].saturating_sub(1);
        }
    }

    match mv.from {
        PlannerLoc::Col(ci) => st.cols[ci as usize].push(card),
        PlannerLoc::Free(fi) => {
            let f = fi as usize;
            debug_assert!(st.freecells[f].is_none());
            st.freecells[f] = Some(card);
        }
        PlannerLoc::Found(su) => {
            let s = su as usize;
            st.foundations[s] = st.foundations[s].saturating_add(1);
        }
    }

    st.zhash = undo.old_zhash;
}

fn touches_foundation(mv: PlannerMove) -> bool {
    matches!(mv.from, PlannerLoc::Found(_)) || matches!(mv.to, PlannerLoc::Found(_))
}

fn is_immediate_inverse(prev: PlannerMove, next: PlannerMove, allow_found_as_source: bool) -> bool {
    if !allow_found_as_source && (touches_foundation(prev) || touches_foundation(next)) {
        return false;
    }
    prev.from == next.to && prev.to == next.from && prev.count == 1 && next.count == 1
}

fn planner_move_from_action(action: FreecellPlannerAction) -> PlannerMove {
    match action {
        FreecellPlannerAction::TableauToFoundation { src } => PlannerMove {
            from: PlannerLoc::Col(src as u8),
            to: PlannerLoc::Found(0),
            count: 1,
        },
        FreecellPlannerAction::FreecellToFoundation { cell } => PlannerMove {
            from: PlannerLoc::Free(cell as u8),
            to: PlannerLoc::Found(0),
            count: 1,
        },
        FreecellPlannerAction::TableauRunToTableau { src, dst, .. } => PlannerMove {
            from: PlannerLoc::Col(src as u8),
            to: PlannerLoc::Col(dst as u8),
            count: 1,
        },
        FreecellPlannerAction::TableauToFreecell { src, cell } => PlannerMove {
            from: PlannerLoc::Col(src as u8),
            to: PlannerLoc::Free(cell as u8),
            count: 1,
        },
        FreecellPlannerAction::FreecellToTableau { cell, dst } => PlannerMove {
            from: PlannerLoc::Free(cell as u8),
            to: PlannerLoc::Col(dst as u8),
            count: 1,
        },
    }
}

fn generate_moves(
    st: &PlannerState,
    last_move: Option<PlannerMove>,
    allow_found_as_source: bool,
) -> Vec<PlannerMove> {
    let mut out = Vec::with_capacity(64);
    let first_empty_fc = st
        .freecells
        .iter()
        .take(st.freecell_count as usize)
        .position(|slot| slot.is_none())
        .map(|idx| idx as u8);
    let first_empty_col = (0..8)
        .find(|&col| st.cols[col].is_empty())
        .map(|col| col as u8);

    let mut push_move = |mv: PlannerMove| {
        if mv.count != 1 {
            return;
        }
        if mv.from == mv.to {
            return;
        }
        if matches!(mv.from, PlannerLoc::Free(_)) && matches!(mv.to, PlannerLoc::Free(_)) {
            return;
        }
        if let Some(prev) = last_move {
            if is_immediate_inverse(prev, mv, allow_found_as_source) {
                return;
            }
        }
        out.push(mv);
    };

    for src in 0..8_u8 {
        if let Some(card) = st.cols[src as usize].last().copied() {
            if can_move_to_foundation(card, &st.foundations) {
                let suit = card.suit.foundation_index() as u8;
                push_move(PlannerMove {
                    from: PlannerLoc::Col(src),
                    to: PlannerLoc::Found(suit),
                    count: 1,
                });
            }
        }
    }

    for free in 0..st.freecell_count {
        if let Some(card) = st.freecells[free as usize] {
            if can_move_to_foundation(card, &st.foundations) {
                let suit = card.suit.foundation_index() as u8;
                push_move(PlannerMove {
                    from: PlannerLoc::Free(free),
                    to: PlannerLoc::Found(suit),
                    count: 1,
                });
            }
        }
    }

    for free in 0..st.freecell_count {
        let Some(card) = st.freecells[free as usize] else {
            continue;
        };
        for dst in 0..8_u8 {
            if let Some(dest_top) = st.cols[dst as usize].last().copied() {
                if can_stack_on_tableau(card, dest_top) {
                    push_move(PlannerMove {
                        from: PlannerLoc::Free(free),
                        to: PlannerLoc::Col(dst),
                        count: 1,
                    });
                }
            } else if Some(dst) == first_empty_col {
                push_move(PlannerMove {
                    from: PlannerLoc::Free(free),
                    to: PlannerLoc::Col(dst),
                    count: 1,
                });
            }
        }
    }

    for src in 0..8_u8 {
        let Some(card) = st.cols[src as usize].last().copied() else {
            continue;
        };
        for dst in 0..8_u8 {
            if src == dst {
                continue;
            }
            if let Some(dest_top) = st.cols[dst as usize].last().copied() {
                if can_stack_on_tableau(card, dest_top) {
                    push_move(PlannerMove {
                        from: PlannerLoc::Col(src),
                        to: PlannerLoc::Col(dst),
                        count: 1,
                    });
                }
            } else if Some(dst) == first_empty_col {
                push_move(PlannerMove {
                    from: PlannerLoc::Col(src),
                    to: PlannerLoc::Col(dst),
                    count: 1,
                });
            }
        }
    }

    if let Some(free_dst) = first_empty_fc {
        for src in 0..8_u8 {
            if !st.cols[src as usize].is_empty() {
                push_move(PlannerMove {
                    from: PlannerLoc::Col(src),
                    to: PlannerLoc::Free(free_dst),
                    count: 1,
                });
            }
        }
    }

    if allow_found_as_source {
        for su in 0..4_u8 {
            let suit_idx = su as usize;
            let top_rank = st.foundations[suit_idx];
            if top_rank == 0 {
                continue;
            }
            let card = card_from_suit_rank(suit_idx, top_rank);
            for dst in 0..8_u8 {
                if let Some(dest_top) = st.cols[dst as usize].last().copied() {
                    if can_stack_on_tableau(card, dest_top) {
                        push_move(PlannerMove {
                            from: PlannerLoc::Found(su),
                            to: PlannerLoc::Col(dst),
                            count: 1,
                        });
                    }
                } else if Some(dst) == first_empty_col {
                    push_move(PlannerMove {
                        from: PlannerLoc::Found(su),
                        to: PlannerLoc::Col(dst),
                        count: 1,
                    });
                }
            }
            if let Some(free_dst) = first_empty_fc {
                push_move(PlannerMove {
                    from: PlannerLoc::Found(su),
                    to: PlannerLoc::Free(free_dst),
                    count: 1,
                });
            }
        }
    }

    out
}

fn build_candidates_from_generated_moves(
    game: &FreecellGame,
    atomic_profile: bool,
    last_action: Option<FreecellPlannerAction>,
) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut st = planner_state_from_game(game);
    let card_count_mode = game.card_count_mode();
    let last_move = last_action.map(planner_move_from_action);
    let generated = generate_moves(&st, last_move, false);

    for mv in generated {
        let action_and_meta = match (mv.from, mv.to) {
            (PlannerLoc::Col(src), PlannerLoc::Found(_)) => {
                let card = game.tableau_top(src as usize);
                let class = if safe_foundation_card(game, card) {
                    MoveClass::SafeFoundation
                } else {
                    MoveClass::Foundation
                };
                let bias = if atomic_profile { 2_800 } else { 2_600 };
                Some((
                    FreecellPlannerAction::TableauToFoundation { src: src as usize },
                    class,
                    bias,
                    src as i32,
                ))
            }
            (PlannerLoc::Free(cell), PlannerLoc::Found(_)) => {
                let card = game.freecell_card(cell as usize);
                let class = if safe_foundation_card(game, card) {
                    MoveClass::SafeFoundation
                } else {
                    MoveClass::Foundation
                };
                let bias = if atomic_profile { 3_200 } else { 3_000 };
                Some((
                    FreecellPlannerAction::FreecellToFoundation {
                        cell: cell as usize,
                    },
                    class,
                    bias,
                    cell as i32,
                ))
            }
            (PlannerLoc::Free(cell), PlannerLoc::Col(dst)) => Some((
                FreecellPlannerAction::FreecellToTableau {
                    cell: cell as usize,
                    dst: dst as usize,
                },
                MoveClass::FreecellToTableau,
                if atomic_profile { 320 } else { 260 },
                ((cell as i32) << 8) | dst as i32,
            )),
            (PlannerLoc::Col(src), PlannerLoc::Col(dst)) => {
                let start = game
                    .tableau()
                    .get(src as usize)
                    .map(Vec::len)
                    .unwrap_or(0)
                    .saturating_sub(1);
                let empty_bonus = if game.tableau().get(dst as usize).is_some_and(Vec::is_empty) {
                    if atomic_profile {
                        520
                    } else {
                        420
                    }
                } else if atomic_profile {
                    220
                } else {
                    180
                };
                let bias = if atomic_profile {
                    empty_bonus
                } else {
                    empty_bonus + 10
                };
                Some((
                    FreecellPlannerAction::TableauRunToTableau {
                        src: src as usize,
                        start,
                        dst: dst as usize,
                    },
                    MoveClass::TableauRunToTableau,
                    bias,
                    ((src as i32) << 16) | ((start as i32) << 8) | dst as i32,
                ))
            }
            (PlannerLoc::Col(src), PlannerLoc::Free(cell)) => Some((
                FreecellPlannerAction::TableauToFreecell {
                    src: src as usize,
                    cell: cell as usize,
                },
                MoveClass::TableauToFreecell,
                if atomic_profile { 280 } else { 340 },
                ((src as i32) << 8) | cell as i32,
            )),
            _ => None,
        };

        let Some((action, class, score_bias, ctx)) = action_and_meta else {
            continue;
        };
        if let Ok(undo) = apply_move_in_place(&mut st, mv, false) {
            let next = planner_state_to_game(&st, card_count_mode);
            out.push(Candidate {
                action,
                score: transition_score(game, &next, score_bias),
                class,
                ctx,
                next,
            });
            undo_move_in_place(&mut st, undo);
        }
    }

    out
}

fn generate_candidates(
    game: &FreecellGame,
    last_action: Option<FreecellPlannerAction>,
) -> Vec<Candidate> {
    build_candidates_from_generated_moves(game, false, last_action)
}

fn generate_atomic_candidates(
    game: &FreecellGame,
    last_action: Option<FreecellPlannerAction>,
) -> Vec<Candidate> {
    build_candidates_from_generated_moves(game, true, last_action)
}

fn planner_state_is_goal(st: &PlannerState, target_cards: u32) -> bool {
    st.foundations.iter().map(|&rank| rank as u32).sum::<u32>() == target_cards
}

fn heuristic_admissible_state(st: &PlannerState, target_cards: u32) -> u32 {
    let in_found = st.foundations.iter().map(|&rank| rank as u32).sum::<u32>();
    target_cards.saturating_sub(in_found)
}

fn count_foundation_cards_state(st: &PlannerState) -> u32 {
    st.foundations.iter().map(|&rank| rank as u32).sum::<u32>()
}

fn count_empty_freecells_state(st: &PlannerState) -> u32 {
    st.freecells
        .iter()
        .take(st.freecell_count as usize)
        .filter(|slot| slot.is_none())
        .count() as u32
}

fn count_empty_cols_state(st: &PlannerState) -> u32 {
    st.cols.iter().filter(|col| col.is_empty()).count() as u32
}

fn count_immediate_found_moves_state(st: &PlannerState) -> u32 {
    let mut n = 0u32;
    for col in 0..8 {
        if let Some(top) = st.cols[col].last().copied() {
            if can_move_to_foundation(top, &st.foundations) {
                n = n.saturating_add(1);
            }
        }
    }
    for free in 0..st.freecell_count as usize {
        if let Some(card) = st.freecells[free] {
            if can_move_to_foundation(card, &st.foundations) {
                n = n.saturating_add(1);
            }
        }
    }
    n
}

fn greedy_score_state(st: &PlannerState) -> i32 {
    let fnd = count_foundation_cards_state(st) as i32;
    let efc = count_empty_freecells_state(st) as i32;
    let eco = count_empty_cols_state(st) as i32;
    let imm = count_immediate_found_moves_state(st) as i32;
    1000 * fnd + 50 * efc + 30 * eco + 10 * imm
}

fn move_bucket_state(st: &PlannerState, mv: PlannerMove, allow_found_as_source: bool) -> u8 {
    if matches!(mv.to, PlannerLoc::Found(_)) {
        return 0;
    }
    if matches!(mv.from, PlannerLoc::Free(_)) && matches!(mv.to, PlannerLoc::Col(_)) {
        return 1;
    }
    if let (PlannerLoc::Col(_), PlannerLoc::Col(dst)) = (mv.from, mv.to) {
        return if st.cols[dst as usize].is_empty() {
            3
        } else {
            2
        };
    }
    if matches!(mv.from, PlannerLoc::Col(_)) && matches!(mv.to, PlannerLoc::Free(_)) {
        return 4;
    }
    if allow_found_as_source && matches!(mv.from, PlannerLoc::Found(_)) {
        return 5;
    }
    6
}

fn filter_and_order_moves_for_ida(
    st: &mut PlannerState,
    allow_found_as_source: bool,
    g: u32,
    bound: u32,
    target_cards: u32,
    next_bound: &mut u32,
    moves: &mut Vec<PlannerMove>,
) {
    let mut scored: Vec<((u8, u32, i32), PlannerMove)> = Vec::with_capacity(moves.len());
    for &mv in moves.iter() {
        let bucket = move_bucket_state(st, mv, allow_found_as_source);
        let Ok(undo) = apply_move_in_place(st, mv, allow_found_as_source) else {
            continue;
        };
        let h2 = heuristic_admissible_state(st, target_cards);
        let f2 = g.saturating_add(1).saturating_add(h2);
        if f2 > bound {
            if f2 < *next_bound {
                *next_bound = f2;
            }
            undo_move_in_place(st, undo);
            continue;
        }
        let gs = greedy_score_state(st);
        undo_move_in_place(st, undo);
        scored.push(((bucket, f2, -gs), mv));
    }
    scored.sort_unstable_by_key(|entry| entry.0);
    moves.clear();
    moves.extend(scored.into_iter().map(|(_, mv)| mv));
}

fn planner_action_from_move(st: &PlannerState, mv: PlannerMove) -> Option<FreecellPlannerAction> {
    match (mv.from, mv.to) {
        (PlannerLoc::Col(src), PlannerLoc::Found(_)) => {
            Some(FreecellPlannerAction::TableauToFoundation { src: src as usize })
        }
        (PlannerLoc::Free(cell), PlannerLoc::Found(_)) => {
            Some(FreecellPlannerAction::FreecellToFoundation {
                cell: cell as usize,
            })
        }
        (PlannerLoc::Col(src), PlannerLoc::Col(dst)) => {
            let start = st
                .cols
                .get(src as usize)
                .map(Vec::len)
                .unwrap_or(0)
                .saturating_sub(1);
            Some(FreecellPlannerAction::TableauRunToTableau {
                src: src as usize,
                start,
                dst: dst as usize,
            })
        }
        (PlannerLoc::Col(src), PlannerLoc::Free(cell)) => {
            Some(FreecellPlannerAction::TableauToFreecell {
                src: src as usize,
                cell: cell as usize,
            })
        }
        (PlannerLoc::Free(cell), PlannerLoc::Col(dst)) => {
            Some(FreecellPlannerAction::FreecellToTableau {
                cell: cell as usize,
                dst: dst as usize,
            })
        }
        _ => None,
    }
}

struct IdaStats {
    explored_states: usize,
    branch_total: usize,
    expanded_nodes: usize,
}

struct IdaParams<'a> {
    allow_found_as_source: bool,
    target_cards: u32,
    node_budget: usize,
    time_budget_ms: u64,
    started_at: Instant,
    seen_states: &'a HashSet<u64>,
    cancel: Option<&'a AtomicBool>,
}

struct IdaFrame {
    g: u32,
    key: PlannerKey,
    moves: Vec<PlannerMove>,
    next_i: usize,
    undo_to_parent: Option<PlannerUndo>,
    action_from_parent: Option<FreecellPlannerAction>,
}

enum IdaEnterResult {
    FoundGoal,
    Cutoff,
    Pruned,
    Abort,
    Expanded {
        key: PlannerKey,
        moves: Vec<PlannerMove>,
    },
}

fn ida_enter_node(
    st: &PlannerState,
    params: &IdaParams<'_>,
    g: u32,
    bound: u32,
    last_move: Option<PlannerMove>,
    on_path: &mut HashSet<PlannerKey>,
    tt: &mut HashMap<PlannerKey, u32>,
    next_bound: &mut u32,
    stats: &mut IdaStats,
) -> IdaEnterResult {
    if params
        .cancel
        .is_some_and(|flag| flag.load(AtomicOrdering::Relaxed))
    {
        return IdaEnterResult::Abort;
    }
    if stats.explored_states >= params.node_budget {
        return IdaEnterResult::Abort;
    }
    if params.started_at.elapsed().as_millis() >= u128::from(params.time_budget_ms) {
        return IdaEnterResult::Abort;
    }

    stats.explored_states = stats.explored_states.saturating_add(1);
    let h = heuristic_admissible_state(st, params.target_cards);
    let f = g.saturating_add(h);
    if f > bound {
        if f < *next_bound {
            *next_bound = f;
        }
        return IdaEnterResult::Cutoff;
    }
    if planner_state_is_goal(st, params.target_cards) {
        return IdaEnterResult::FoundGoal;
    }

    let key = make_planner_key_from_state(st);
    if on_path.contains(&key) {
        return IdaEnterResult::Pruned;
    }
    on_path.insert(key);

    if let Some(&best_g) = tt.get(&key) {
        if g >= best_g {
            on_path.remove(&key);
            return IdaEnterResult::Pruned;
        }
    }
    tt.insert(key, g);

    let moves = generate_moves(st, last_move, params.allow_found_as_source);
    stats.expanded_nodes = stats.expanded_nodes.saturating_add(1);
    IdaEnterResult::Expanded { key, moves }
}

enum IdaIterationResult {
    Found,
    Continue,
    Abort,
}

fn ida_iteration_iterative(
    st: &mut PlannerState,
    params: &IdaParams<'_>,
    bound: u32,
    path_actions: &mut Vec<FreecellPlannerAction>,
    on_path: &mut HashSet<PlannerKey>,
    tt: &mut HashMap<PlannerKey, u32>,
    next_bound: &mut u32,
    stats: &mut IdaStats,
) -> IdaIterationResult {
    path_actions.clear();
    on_path.clear();
    tt.clear();
    *next_bound = u32::MAX;

    let mut stack: Vec<IdaFrame> = Vec::new();
    match ida_enter_node(st, params, 0, bound, None, on_path, tt, next_bound, stats) {
        IdaEnterResult::FoundGoal => return IdaIterationResult::Found,
        IdaEnterResult::Abort => return IdaIterationResult::Abort,
        IdaEnterResult::Cutoff | IdaEnterResult::Pruned => return IdaIterationResult::Continue,
        IdaEnterResult::Expanded { key, mut moves } => {
            filter_and_order_moves_for_ida(
                st,
                params.allow_found_as_source,
                0,
                bound,
                params.target_cards,
                next_bound,
                &mut moves,
            );
            stats.branch_total = stats.branch_total.saturating_add(moves.len());
            stack.push(IdaFrame {
                g: 0,
                key,
                moves,
                next_i: 0,
                undo_to_parent: None,
                action_from_parent: None,
            });
        }
    }

    while !stack.is_empty() {
        let exhausted = {
            let top = stack
                .last()
                .expect("stack has at least one frame during iterative ida");
            top.next_i >= top.moves.len()
        };
        if exhausted {
            let frame = stack
                .pop()
                .expect("stack has at least one frame when unwinding");
            on_path.remove(&frame.key);
            if let Some(undo) = frame.undo_to_parent {
                undo_move_in_place(st, undo);
            }
            if frame.action_from_parent.is_some() {
                path_actions.pop();
            }
            continue;
        }

        let mv = {
            let top = stack
                .last_mut()
                .expect("stack has at least one frame while expanding");
            let mv = top.moves[top.next_i];
            top.next_i += 1;
            mv
        };
        let Some(action) = planner_action_from_move(st, mv) else {
            continue;
        };
        let Ok(undo) = apply_move_in_place(st, mv, params.allow_found_as_source) else {
            continue;
        };
        if params.seen_states.contains(&st.zhash) {
            undo_move_in_place(st, undo);
            continue;
        }
        let g_child = stack
            .last()
            .expect("parent frame exists when creating child")
            .g
            .saturating_add(1);
        path_actions.push(action);

        match ida_enter_node(
            st,
            params,
            g_child,
            bound,
            Some(mv),
            on_path,
            tt,
            next_bound,
            stats,
        ) {
            IdaEnterResult::FoundGoal => return IdaIterationResult::Found,
            IdaEnterResult::Abort => {
                path_actions.pop();
                undo_move_in_place(st, undo);
                return IdaIterationResult::Abort;
            }
            IdaEnterResult::Cutoff | IdaEnterResult::Pruned => {
                path_actions.pop();
                undo_move_in_place(st, undo);
            }
            IdaEnterResult::Expanded { key, mut moves } => {
                filter_and_order_moves_for_ida(
                    st,
                    params.allow_found_as_source,
                    g_child,
                    bound,
                    params.target_cards,
                    next_bound,
                    &mut moves,
                );
                stats.branch_total = stats.branch_total.saturating_add(moves.len());
                stack.push(IdaFrame {
                    g: g_child,
                    key,
                    moves,
                    next_i: 0,
                    undo_to_parent: Some(undo),
                    action_from_parent: Some(action),
                });
            }
        }
    }
    IdaIterationResult::Continue
}

pub fn plan_line_ida(
    start: &FreecellGame,
    seen_states: &HashSet<u64>,
    config: FreecellPlannerConfig,
    cancel: Option<&AtomicBool>,
) -> FreecellPlannerResult {
    let mut st = planner_state_from_game(start);
    let target_cards = u32::from(start.card_count_mode().card_count());
    let mut bound = heuristic_admissible_state(&st, target_cards);
    let params = IdaParams {
        allow_found_as_source: false,
        target_cards,
        node_budget: config.node_budget,
        time_budget_ms: config.time_budget_ms,
        started_at: Instant::now(),
        seen_states,
        cancel,
    };
    let mut stats = IdaStats {
        explored_states: 0,
        branch_total: 0,
        expanded_nodes: 0,
    };
    let mut path_actions: Vec<FreecellPlannerAction> = Vec::new();
    let mut on_path: HashSet<PlannerKey> = HashSet::new();
    let mut tt: HashMap<PlannerKey, u32> = HashMap::new();
    let mut next_bound = u32::MAX;

    loop {
        match ida_iteration_iterative(
            &mut st,
            &params,
            bound,
            &mut path_actions,
            &mut on_path,
            &mut tt,
            &mut next_bound,
            &mut stats,
        ) {
            IdaIterationResult::Found => {
                return FreecellPlannerResult {
                    actions: path_actions.iter().copied().collect(),
                    explored_states: stats.explored_states,
                    stalled: false,
                    stale_skips: 0,
                    inverse_prunes: 0,
                    inverse_checked: 0,
                    branch_total: stats.branch_total,
                    expanded_nodes: stats.expanded_nodes,
                    expanded_h_sum: 0,
                    expanded_tb_sum: 0,
                };
            }
            IdaIterationResult::Abort => {
                return FreecellPlannerResult {
                    actions: VecDeque::new(),
                    explored_states: stats.explored_states,
                    stalled: true,
                    stale_skips: 0,
                    inverse_prunes: 0,
                    inverse_checked: 0,
                    branch_total: stats.branch_total,
                    expanded_nodes: stats.expanded_nodes,
                    expanded_h_sum: 0,
                    expanded_tb_sum: 0,
                };
            }
            IdaIterationResult::Continue => {
                if next_bound == u32::MAX {
                    return FreecellPlannerResult {
                        actions: VecDeque::new(),
                        explored_states: stats.explored_states,
                        stalled: true,
                        stale_skips: 0,
                        inverse_prunes: 0,
                        inverse_checked: 0,
                        branch_total: stats.branch_total,
                        expanded_nodes: stats.expanded_nodes,
                        expanded_h_sum: 0,
                        expanded_tb_sum: 0,
                    };
                }
                bound = next_bound;
            }
        }
    }
}

fn generate_relocation_candidates(game: &FreecellGame) -> Vec<Candidate> {
    let mut out = Vec::new();
    let mut st = planner_state_from_game(game);
    let card_count_mode = game.card_count_mode();
    let first_empty_fc = first_empty_freecell(game);
    let first_empty_col = first_empty_tableau_col(game);
    let allow_found_as_source = false;

    let mut push_candidate =
        |mv: PlannerMove, action: FreecellPlannerAction, score_bias: i64, ctx: i32| {
            if let Ok(undo) = apply_move_in_place(&mut st, mv, allow_found_as_source) {
                let next = planner_state_to_game(&st, card_count_mode);
                let score = transition_score(game, &next, score_bias);
                out.push(Candidate {
                    action,
                    next,
                    score,
                    class: MoveClass::Relocation,
                    ctx,
                });
                undo_move_in_place(&mut st, undo);
            }
        };

    for (src, col) in game.tableau().iter().enumerate() {
        if col.len() < 2 {
            continue;
        }
        let len = col.len();
        let mut target_idx = None;
        for (idx, card) in col.iter().enumerate() {
            if should_relocate_toward(game, *card) {
                target_idx = Some(idx);
                break;
            }
        }
        let Some(target_idx) = target_idx else {
            continue;
        };
        let blockers = len.saturating_sub(target_idx + 1);
        if blockers == 0 {
            continue;
        }

        // Relocation attempt 1: move top blocker card to freecell.
        for cell in 0..game.freecell_count() {
            if is_empty_freecell_slot(game, cell) && Some(cell) != first_empty_fc {
                continue;
            }
            let mv = PlannerMove {
                from: PlannerLoc::Col(src as u8),
                to: PlannerLoc::Free(cell as u8),
                count: 1,
            };
            let bias = 680 + (blockers as i64 * 140);
            push_candidate(
                mv,
                FreecellPlannerAction::TableauToFreecell { src, cell },
                bias,
                ((src as i32) << 16) | ((target_idx as i32) << 8) | cell as i32,
            );
        }

        // Relocation attempt 2: move top blocker card to another tableau.
        let top = len - 1;
        for dst in 0..8 {
            if is_empty_tableau_col(game, dst) && Some(dst) != first_empty_col {
                continue;
            }
            let mv = PlannerMove {
                from: PlannerLoc::Col(src as u8),
                to: PlannerLoc::Col(dst as u8),
                count: 1,
            };
            let dst_empty_bonus = if game.tableau().get(dst).is_some_and(Vec::is_empty) {
                220
            } else {
                0
            };
            let bias = 620 + (blockers as i64 * 120) + dst_empty_bonus;
            push_candidate(
                mv,
                FreecellPlannerAction::TableauRunToTableau {
                    src,
                    start: top,
                    dst,
                },
                bias,
                ((src as i32) << 16) | ((target_idx as i32) << 8) | dst as i32,
            );
        }
    }

    out
}

fn should_relocate_toward(game: &FreecellGame, card: Card) -> bool {
    if card.rank <= 3 {
        return true;
    }
    // Also prioritize cards that are currently close to being foundation-playable.
    let foundation_rank = game.foundations()[card.suit.foundation_index()].len() as u8;
    card.rank <= foundation_rank.saturating_add(2)
}

fn dedup_candidates_by_action(candidates: Vec<Candidate>) -> Vec<Candidate> {
    let mut best = HashMap::<FreecellPlannerAction, Candidate>::new();
    for candidate in candidates {
        best.entry(candidate.action)
            .and_modify(|existing| {
                if candidate.score > existing.score {
                    *existing = candidate.clone();
                }
            })
            .or_insert(candidate);
    }
    best.into_values().collect()
}

fn select_candidates_with_class_caps(candidates: Vec<Candidate>, beam: usize) -> Vec<Candidate> {
    let mut selected = Vec::with_capacity(beam);
    let mut used = HashMap::<MoveClass, usize>::new();
    for candidate in candidates.iter() {
        if selected.len() >= beam {
            break;
        }
        let cap = class_cap(candidate.class, beam);
        let count = used.get(&candidate.class).copied().unwrap_or(0);
        if count >= cap {
            continue;
        }
        selected.push(candidate.clone());
        used.insert(candidate.class, count + 1);
    }
    if selected.len() < beam {
        for candidate in candidates {
            if selected.len() >= beam {
                break;
            }
            if selected
                .iter()
                .any(|picked| picked.action == candidate.action)
            {
                continue;
            }
            selected.push(candidate);
        }
    }
    selected
}

fn class_cap(class: MoveClass, beam: usize) -> usize {
    let half = (beam / 2).max(1);
    let third = (beam / 3).max(1);
    match class {
        MoveClass::SafeFoundation => beam,
        MoveClass::Foundation => beam,
        MoveClass::Relocation => half.max(2),
        MoveClass::TableauRunToTableau => half.max(3),
        MoveClass::FreecellToTableau => third.max(2),
        MoveClass::TableauToFreecell => third.max(2),
    }
}

fn transition_score(current: &FreecellGame, next: &FreecellGame, bias: i64) -> i64 {
    let foundation_delta = foundation_cards(next) as i64 - foundation_cards(current) as i64;
    let order_delta = tableau_order_score(next) - tableau_order_score(current);
    let deadlock_delta = deadlock_penalty(current) - deadlock_penalty(next);
    let mobility_delta = legal_move_count(next) as i64 - legal_move_count(current) as i64;
    let empty_col_delta = next.tableau().iter().filter(|c| c.is_empty()).count() as i64
        - current.tableau().iter().filter(|c| c.is_empty()).count() as i64;
    foundation_delta * 1_000
        + order_delta * 70
        + deadlock_delta * 20
        + mobility_delta * 18
        + empty_col_delta * 240
        + bias
}

fn heuristic(game: &FreecellGame) -> i64 {
    let foundation = foundation_cards(game) as i64;
    let empty_cols = game.tableau().iter().filter(|col| col.is_empty()).count() as i64;
    let free_used = game
        .freecells()
        .iter()
        .filter(|slot| slot.is_some())
        .count() as i64;
    let buried = buried_ace_deuce_depth(game) as i64;
    let deep_deadlock = deadlock_penalty(game);
    foundation * 100 + empty_cols * 50 - buried * 10 - free_used * 20 - deep_deadlock
}

fn first_safe_foundation_action(game: &FreecellGame) -> Option<FreecellPlannerAction> {
    for cell in 0..game.freecell_count() {
        if game.can_move_freecell_to_foundation(cell)
            && safe_foundation_card(game, game.freecell_card(cell))
        {
            return Some(FreecellPlannerAction::FreecellToFoundation { cell });
        }
    }
    for src in 0..8 {
        if game.can_move_tableau_top_to_foundation(src)
            && safe_foundation_card(game, game.tableau_top(src))
        {
            return Some(FreecellPlannerAction::TableauToFoundation { src });
        }
    }
    None
}

fn greedy_safe_foundation_pass(
    start: &FreecellGame,
    max_steps: usize,
) -> Option<VecDeque<FreecellPlannerAction>> {
    let mut game = start.clone();
    let mut actions = VecDeque::new();
    for _ in 0..max_steps {
        let Some(action) = first_safe_foundation_action(&game) else {
            break;
        };
        let changed = match action {
            FreecellPlannerAction::TableauToFoundation { src } => {
                game.move_tableau_top_to_foundation(src)
            }
            FreecellPlannerAction::FreecellToFoundation { cell } => {
                game.move_freecell_to_foundation(cell)
            }
            _ => false,
        };
        if !changed {
            break;
        }
        actions.push_back(action);
    }
    if actions.is_empty() {
        None
    } else {
        Some(actions)
    }
}

fn compress_node_by_safe_foundation(mut node: Node, max_steps: usize) -> Node {
    for _ in 0..max_steps {
        let Some(action) = first_safe_foundation_action(&node.game) else {
            break;
        };
        if !apply_action(&mut node.game, action) {
            break;
        }
        node.path.push(action);
        node.g = node.g.saturating_add(1);
        node.foundation = foundation_cards(&node.game);
        node.hash = zobrist_hash(&node.game);
        node.total_score += 1_800 + heuristic(&node.game);
        node.priority = node.total_score - i64::from(node.g) * 32;
        if node.game.is_won() {
            break;
        }
    }
    node
}

fn apply_action(game: &mut FreecellGame, action: FreecellPlannerAction) -> bool {
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

fn safe_foundation_card(game: &FreecellGame, card: Option<Card>) -> bool {
    let Some(card) = card else {
        return false;
    };
    if card.rank <= 1 {
        return true;
    }
    let needed = usize::from(card.rank.saturating_sub(1));
    if card.color_red() {
        game.foundations()[0].len() >= needed && game.foundations()[3].len() >= needed
    } else {
        game.foundations()[1].len() >= needed && game.foundations()[2].len() >= needed
    }
}

fn legal_move_count(game: &FreecellGame) -> usize {
    let mut count = 0usize;
    let first_empty_fc = first_empty_freecell(game);
    let first_empty_col = first_empty_tableau_col(game);
    for cell in 0..game.freecell_count() {
        if game.can_move_freecell_to_foundation(cell) {
            count += 1;
        }
        for dst in 0..8 {
            if is_empty_tableau_col(game, dst) && Some(dst) != first_empty_col {
                continue;
            }
            if game.can_move_freecell_to_tableau(cell, dst) {
                count += 1;
            }
        }
    }
    for src in 0..8 {
        if game.can_move_tableau_top_to_foundation(src) {
            count += 1;
        }
        for cell in 0..game.freecell_count() {
            if is_empty_freecell_slot(game, cell) && Some(cell) != first_empty_fc {
                continue;
            }
            if game.can_move_tableau_top_to_freecell(src, cell) {
                count += 1;
            }
        }
        let len = game.tableau().get(src).map(Vec::len).unwrap_or(0);
        for start in 0..len {
            for dst in 0..8 {
                if is_empty_tableau_col(game, dst) && Some(dst) != first_empty_col {
                    continue;
                }
                if game.can_move_tableau_run_to_tableau(src, start, dst) {
                    count += 1;
                }
            }
        }
    }
    count
}

fn foundation_cards(game: &FreecellGame) -> usize {
    game.foundations().iter().map(Vec::len).sum()
}

fn buried_ace_deuce_depth(game: &FreecellGame) -> usize {
    let mut depth = 0usize;
    for col in game.tableau() {
        for (idx, card) in col.iter().enumerate() {
            if card.rank <= 2 {
                depth = depth.saturating_add(col.len().saturating_sub(idx + 1));
            }
        }
    }
    depth
}

fn deadlock_penalty(game: &FreecellGame) -> i64 {
    let mut penalty = 0i64;
    for col in game.tableau() {
        for lower_idx in 0..col.len() {
            for upper_idx in (lower_idx + 1)..col.len() {
                let lower = col[lower_idx];
                let upper = col[upper_idx];
                if lower.suit == upper.suit && lower.rank < upper.rank {
                    penalty += 6;
                }
            }
        }
    }
    penalty
}

fn tableau_order_score(game: &FreecellGame) -> i64 {
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
        total += run_len * run_len;
    }
    total
}

fn actions_are_inverse(a: FreecellPlannerAction, b: FreecellPlannerAction) -> bool {
    let to_move = |action: FreecellPlannerAction| -> Option<PlannerMove> {
        match action {
            FreecellPlannerAction::TableauRunToTableau { src, dst, .. } => Some(PlannerMove {
                from: PlannerLoc::Col(src as u8),
                to: PlannerLoc::Col(dst as u8),
                count: 1,
            }),
            FreecellPlannerAction::TableauToFreecell { src, cell } => Some(PlannerMove {
                from: PlannerLoc::Col(src as u8),
                to: PlannerLoc::Free(cell as u8),
                count: 1,
            }),
            FreecellPlannerAction::FreecellToTableau { cell, dst } => Some(PlannerMove {
                from: PlannerLoc::Free(cell as u8),
                to: PlannerLoc::Col(dst as u8),
                count: 1,
            }),
            FreecellPlannerAction::TableauToFoundation { .. }
            | FreecellPlannerAction::FreecellToFoundation { .. } => None,
        }
    };
    to_move(a)
        .zip(to_move(b))
        .is_some_and(|(prev, next)| is_immediate_inverse(prev, next, false))
}

fn first_empty_freecell(game: &FreecellGame) -> Option<usize> {
    game.freecells().iter().position(Option::is_none)
}

fn first_empty_tableau_col(game: &FreecellGame) -> Option<usize> {
    game.tableau().iter().position(Vec::is_empty)
}

fn is_empty_freecell_slot(game: &FreecellGame, cell: usize) -> bool {
    game.freecells()
        .get(cell)
        .is_some_and(|slot| slot.is_none())
}

fn is_empty_tableau_col(game: &FreecellGame, col: usize) -> bool {
    game.tableau().get(col).is_some_and(Vec::is_empty)
}

fn zobrist_card_index(card: Card) -> usize {
    card.suit.foundation_index() * 13 + usize::from(card.rank.saturating_sub(1))
}

fn card_to_u8(card: Card) -> u8 {
    (card.suit.foundation_index() as u8)
        .saturating_mul(13)
        .saturating_add(card.rank.saturating_sub(1))
}

fn zobrist_value(card_idx: usize, pos_idx: usize) -> u64 {
    let mixed = ((card_idx as u64) << 10) ^ (pos_idx as u64) ^ 0x9E37_79B9_7F4A_7C15;
    splitmix64(mixed)
}

fn splitmix64(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = x;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn c(suit: Suit, rank: u8) -> Card {
        Card {
            suit,
            rank,
            face_up: true,
        }
    }

    #[test]
    fn apply_and_undo_restores_state_and_hash() {
        let game = FreecellGame::debug_new(
            [Vec::new(), Vec::new(), Vec::new(), Vec::new()],
            [None, None, None, None],
            [
                vec![c(Suit::Clubs, 7)],
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
                Vec::new(),
            ],
        );
        let mut st = planner_state_from_game(&game);
        let old_hash = st.zhash;
        let old_cols = st.cols.clone();
        let old_free = st.freecells;
        let undo = apply_move_in_place(
            &mut st,
            PlannerMove {
                from: PlannerLoc::Col(0),
                to: PlannerLoc::Free(0),
                count: 1,
            },
            false,
        )
        .expect("move should be legal");
        assert_ne!(st.zhash, old_hash);
        assert_eq!(st.cols[0].len(), 0);
        assert_eq!(st.freecells[0], Some(c(Suit::Clubs, 7)));
        undo_move_in_place(&mut st, undo);
        assert_eq!(st.zhash, old_hash);
        assert_eq!(st.cols, old_cols);
        assert_eq!(st.freecells, old_free);
    }

    #[test]
    fn immediate_inverse_pruning_ignores_foundation_when_source_forbidden() {
        let prev = PlannerMove {
            from: PlannerLoc::Col(0),
            to: PlannerLoc::Found(0),
            count: 1,
        };
        let next = PlannerMove {
            from: PlannerLoc::Found(0),
            to: PlannerLoc::Col(0),
            count: 1,
        };
        assert!(!is_immediate_inverse(prev, next, false));
    }
}

use crate::game::ChessMove;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchLimits {
    pub max_depth: u8,
    pub time_budget_ms: u64,
    pub node_budget: u64,
}

impl SearchLimits {
    pub const fn new(max_depth: u8, time_budget_ms: u64, node_budget: u64) -> Self {
        Self {
            max_depth,
            time_budget_ms,
            node_budget,
        }
    }

    pub const fn hint() -> Self {
        // Hint/Wand needs enough tactical depth to avoid immediate blunders.
        Self::new(4, 160, 200_000)
    }

    pub const fn robot() -> Self {
        Self::new(4, 160, 200_000)
    }
}

impl Default for SearchLimits {
    fn default() -> Self {
        Self::robot()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AiConfig {
    pub enable_transposition_table: bool,
    pub use_quiescence: bool,
    pub transposition_capacity: usize,
}

impl Default for AiConfig {
    fn default() -> Self {
        Self {
            enable_transposition_table: true,
            use_quiescence: true,
            transposition_capacity: 200_000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTermination {
    Completed,
    TimeBudget,
    NodeBudget,
    Canceled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchResult {
    pub best_move: Option<ChessMove>,
    pub best_score_cp: i32,
    pub depth_reached: u8,
    pub nodes: u64,
    pub pv: Vec<ChessMove>,
    pub termination: SearchTermination,
}

impl SearchResult {
    pub fn empty(termination: SearchTermination) -> Self {
        Self {
            best_move: None,
            best_score_cp: 0,
            depth_reached: 0,
            nodes: 0,
            pv: Vec::new(),
            termination,
        }
    }
}

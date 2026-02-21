use super::ai::{self, AiConfig, SearchLimits};
use crate::game::{ChessMove, ChessPosition};

pub fn best_move_hint(position: &ChessPosition) -> Option<ChessMove> {
    ai::search_best_move(position, SearchLimits::hint(), AiConfig::default()).best_move
}

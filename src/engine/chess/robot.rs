use super::ai::{self, AiConfig, SearchLimits};
use crate::game::{ChessMove, ChessPosition};

pub fn pick_robot_move(position: &ChessPosition) -> Option<ChessMove> {
    ai::search_best_move(position, SearchLimits::robot(), AiConfig::default()).best_move
}

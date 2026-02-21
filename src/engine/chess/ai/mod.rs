pub mod api;
pub mod eval;
pub mod search;
pub mod worker;

pub use api::{AiConfig, SearchLimits, SearchResult};
pub use worker::AsyncSearch;

use crate::game::ChessPosition;

pub fn search_best_move(
    position: &ChessPosition,
    limits: SearchLimits,
    config: AiConfig,
) -> SearchResult {
    search::iterative::search(position, limits, config, None)
}

pub fn spawn_search(
    position: ChessPosition,
    limits: SearchLimits,
    config: AiConfig,
) -> AsyncSearch {
    worker::spawn_search(position, limits, config)
}

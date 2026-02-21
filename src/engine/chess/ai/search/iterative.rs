use super::{alphabeta, move_order, no_legal_move_score, SearchContext};
use crate::engine::chess::ai::api::{AiConfig, SearchLimits, SearchResult, SearchTermination};
use crate::engine::chess::ai::eval;
use crate::game::{legal_moves, ChessMove, ChessPosition};
use std::sync::atomic::AtomicBool;

pub fn search(
    position: &ChessPosition,
    limits: SearchLimits,
    config: AiConfig,
    canceled: Option<&AtomicBool>,
) -> SearchResult {
    let mut ctx = SearchContext::new(limits, config, canceled);
    let legal = legal_moves(position);
    if legal.is_empty() {
        return SearchResult {
            best_move: None,
            best_score_cp: no_legal_move_score(position, 0),
            depth_reached: 0,
            nodes: 0,
            pv: Vec::new(),
            termination: SearchTermination::Completed,
        };
    }

    let mut best_move: Option<ChessMove> = legal.first().copied();
    let mut best_score = eval::evaluate(position);
    let mut depth_reached = 0_u8;

    for depth in 1..=ctx.limits.max_depth {
        if ctx.should_abort() {
            break;
        }
        let hash_move = best_move;
        let ordered_root = move_order::ordered_moves(position, hash_move);
        let root = alphabeta::search_root(position, depth, ordered_root, &mut ctx);
        if root.completed {
            if let Some(mv) = root.best_move {
                best_move = Some(mv);
                best_score = root.score;
                depth_reached = depth;
            }
        }
        if ctx.stop_reason.is_some() {
            break;
        }
    }

    let termination = ctx.stop_reason.unwrap_or(SearchTermination::Completed);
    SearchResult {
        best_move,
        best_score_cp: best_score,
        depth_reached,
        nodes: ctx.nodes,
        pv: best_move.into_iter().collect(),
        termination,
    }
}

#[cfg(test)]
mod tests {
    use super::search;
    use crate::engine::chess::ai::api::{AiConfig, SearchLimits, SearchTermination};
    use crate::game::standard_position;

    #[test]
    fn incomplete_iteration_does_not_replace_last_complete_depth() {
        let position = standard_position();
        let result = search(
            &position,
            SearchLimits::new(4, 0, 1),
            AiConfig::default(),
            None,
        );
        assert_eq!(result.termination, SearchTermination::NodeBudget);
        assert_eq!(result.depth_reached, 0);
        assert!(
            result.best_move.is_some(),
            "fallback move should still exist"
        );
    }
}

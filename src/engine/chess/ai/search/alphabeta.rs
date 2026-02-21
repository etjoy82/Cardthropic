use super::tt::{Bound, Entry};
use super::{move_order, no_legal_move_score, quiescence, SearchContext, SCORE_INF};
use crate::engine::chess::ai::eval;
use crate::game::{apply_move, ChessMove, ChessPosition};

#[derive(Debug, Clone, Copy)]
pub struct RootSearchResult {
    pub score: i32,
    pub best_move: Option<ChessMove>,
    pub completed: bool,
}

pub fn search_root(
    position: &ChessPosition,
    depth: u8,
    ordered_root_moves: Vec<ChessMove>,
    ctx: &mut SearchContext<'_>,
) -> RootSearchResult {
    let mut alpha = -SCORE_INF;
    let beta = SCORE_INF;
    let mut best_score = -SCORE_INF;
    let mut best_move = None;
    let mut completed = true;

    if ordered_root_moves.is_empty() {
        return RootSearchResult {
            score: no_legal_move_score(position, 0),
            best_move: None,
            completed: true,
        };
    }

    for mv in ordered_root_moves {
        if ctx.should_abort() {
            completed = false;
            break;
        }
        let mut next = position.clone();
        if !apply_move(&mut next, mv) {
            continue;
        }
        let score = -search(&next, depth.saturating_sub(1), -beta, -alpha, ctx, 1);
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
        }
    }

    if completed {
        if best_move.is_none() {
            best_score = eval::evaluate(position);
        }
    } else if best_move.is_none() {
        best_score = eval::evaluate(position);
    }

    if completed {
        if let Some(tt) = ctx.tt_mut() {
            tt.store(
                position,
                Entry {
                    depth,
                    score: best_score,
                    best_move,
                    bound: Bound::Exact,
                },
            );
        }
    }

    RootSearchResult {
        score: best_score,
        best_move,
        completed,
    }
}

fn search(
    position: &ChessPosition,
    depth: u8,
    mut alpha: i32,
    beta: i32,
    ctx: &mut SearchContext<'_>,
    ply: u8,
) -> i32 {
    if ctx.note_node() {
        return eval::evaluate(position);
    }

    let original_alpha = alpha;
    let mut hash_move = None;
    if let Some(tt) = ctx.tt_mut().and_then(|tt| tt.probe(position)) {
        hash_move = tt.best_move;
        if tt.depth >= depth {
            match tt.bound {
                Bound::Exact => return tt.score,
                Bound::Lower => alpha = alpha.max(tt.score),
                Bound::Upper => {}
            }
            if alpha >= beta {
                return tt.score;
            }
        }
    }

    if depth == 0 {
        if ctx.config.use_quiescence {
            return quiescence::search(position, alpha, beta, ctx, ply);
        }
        return eval::evaluate(position);
    }

    let ordered = move_order::ordered_moves(position, hash_move);
    if ordered.is_empty() {
        return no_legal_move_score(position, ply);
    }

    let mut best_score = -SCORE_INF;
    let mut best_move = None;
    let mut completed = true;

    for mv in ordered {
        if ctx.should_abort() {
            completed = false;
            break;
        }
        let mut next = position.clone();
        if !apply_move(&mut next, mv) {
            continue;
        }
        let score = -search(
            &next,
            depth.saturating_sub(1),
            -beta,
            -alpha,
            ctx,
            ply.saturating_add(1),
        );
        if score > best_score {
            best_score = score;
            best_move = Some(mv);
        }
        if score > alpha {
            alpha = score;
        }
        if alpha >= beta {
            break;
        }
    }

    if best_move.is_none() {
        return eval::evaluate(position);
    }

    if !completed {
        return best_score;
    }

    let bound = if best_score <= original_alpha {
        Bound::Upper
    } else if best_score >= beta {
        Bound::Lower
    } else {
        Bound::Exact
    };
    if let Some(tt) = ctx.tt_mut() {
        tt.store(
            position,
            Entry {
                depth,
                score: best_score,
                best_move,
                bound,
            },
        );
    }

    best_score
}

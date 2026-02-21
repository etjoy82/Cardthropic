use super::{move_order, SearchContext};
use crate::engine::chess::ai::eval;
use crate::game::{apply_move, ChessPosition};

const MAX_QUIESCENCE_PLY: u8 = 10;

pub fn search(
    position: &ChessPosition,
    mut alpha: i32,
    beta: i32,
    ctx: &mut SearchContext<'_>,
    ply: u8,
) -> i32 {
    if ctx.note_node() {
        return eval::evaluate(position);
    }
    if ply >= MAX_QUIESCENCE_PLY {
        return eval::evaluate(position);
    }

    let stand_pat = eval::evaluate(position);
    if stand_pat >= beta {
        return stand_pat;
    }
    if stand_pat > alpha {
        alpha = stand_pat;
    }

    let captures = move_order::ordered_capture_moves(position);
    if captures.is_empty() {
        return stand_pat;
    }

    let mut best = stand_pat;
    for mv in captures {
        if ctx.should_abort() {
            break;
        }
        let mut next = position.clone();
        if !apply_move(&mut next, mv) {
            continue;
        }
        let score = -search(&next, -beta, -alpha, ctx, ply.saturating_add(1));
        if score > best {
            best = score;
        }
        if score > alpha {
            alpha = score;
        }
        if alpha >= beta {
            break;
        }
    }

    best
}

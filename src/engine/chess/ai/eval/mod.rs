pub mod hanging;
pub mod king_safety;
pub mod material;
pub mod mobility;
pub mod pawn_structure;
pub mod pst;

use crate::game::{ChessColor, ChessPosition};

pub fn evaluate(position: &ChessPosition) -> i32 {
    let mut score = 0_i32;
    score += material::white_minus_black(position);
    score += pst::white_minus_black(position);
    score += pawn_structure::white_minus_black(position);
    score += king_safety::white_minus_black(position);
    score += hanging::white_minus_black(position);
    score += mobility::side_to_move_bonus(position);

    match position.side_to_move() {
        ChessColor::White => score,
        ChessColor::Black => -score,
    }
}

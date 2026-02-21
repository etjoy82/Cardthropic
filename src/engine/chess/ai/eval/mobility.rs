use crate::game::{legal_moves, ChessPosition};

pub fn side_to_move_bonus(position: &ChessPosition) -> i32 {
    let moves = legal_moves(position).len() as i32;
    moves * 2
}

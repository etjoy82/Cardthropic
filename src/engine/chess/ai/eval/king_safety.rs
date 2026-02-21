use crate::game::{is_in_check, ChessColor, ChessPosition};

pub fn white_minus_black(position: &ChessPosition) -> i32 {
    let white_in_check = is_in_check(position, ChessColor::White);
    let black_in_check = is_in_check(position, ChessColor::Black);
    let white_penalty = if white_in_check { 35 } else { 0 };
    let black_penalty = if black_in_check { 35 } else { 0 };
    black_penalty - white_penalty
}

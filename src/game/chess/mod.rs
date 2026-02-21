#![allow(dead_code)]
#![allow(unused_imports)]

mod apply;
mod atomic;
mod attacks;
mod castling;
mod fen;
mod legal;
mod movegen;
mod moves;
mod position;
mod rules;
mod setup;
mod types;

pub use apply::apply_move;
pub use fen::{decode_fen, encode_fen};
pub use legal::{is_in_check, legal_moves, square_attacked_by, terminal_state, ChessTerminalState};
pub use movegen::generate_pseudo_legal_moves;
pub use moves::ChessMove;
pub use position::{CastlingRights, ChessPosition};
pub use rules::ChessRuleset;
pub use setup::{
    atomic_position, chess960_back_rank_from_seed, chess960_position, is_valid_chess960_back_rank,
    standard_position, STANDARD_BACK_RANK,
};
pub use types::{
    file_of, parse_square, rank_of, square, square_name, ChessColor, ChessPiece, ChessPieceKind,
    ChessVariant, Square, BOARD_SQUARES,
};

#[cfg(test)]
mod tests;

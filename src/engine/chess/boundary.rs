use super::commands::{ChessCommand, ChessCommandResult, ChessStatus};
use crate::game::{
    apply_move, atomic_position, chess960_position, legal_moves, standard_position, ChessColor,
    ChessPosition,
};

pub fn execute(position: &mut ChessPosition, command: ChessCommand) -> ChessCommandResult {
    match command {
        ChessCommand::NewGame { seed, variant } => {
            *position = match variant {
                crate::game::ChessVariant::Standard => standard_position(),
                crate::game::ChessVariant::Chess960 => chess960_position(seed),
                crate::game::ChessVariant::Atomic => atomic_position(),
            };
            // New games always start with White to move.
            position.set_side_to_move(ChessColor::White);
            ChessCommandResult::changed(ChessStatus::Ready)
        }
        ChessCommand::TryMove(chess_move) => {
            if legal_moves(position).contains(&chess_move) && apply_move(position, chess_move) {
                ChessCommandResult::changed(ChessStatus::Ready)
            } else {
                ChessCommandResult::unchanged(ChessStatus::IllegalMove)
            }
        }
    }
}

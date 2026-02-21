use super::position::ChessPosition;
use super::types::{file_of, rank_of, square, ChessColor, ChessPiece, ChessPieceKind, Square};

pub(crate) fn is_in_check(position: &ChessPosition, color: ChessColor) -> bool {
    let Some(king_square) = find_king(position, color) else {
        return false;
    };
    square_attacked_by(position, king_square, color.opposite())
}

pub(crate) fn square_attacked_by(position: &ChessPosition, target: Square, by: ChessColor) -> bool {
    for from in 0_u8..64 {
        let Some(piece) = position.piece_at(from) else {
            continue;
        };
        if piece.color != by {
            continue;
        }
        if piece_attacks_square(position, from, piece, target) {
            return true;
        }
    }
    false
}

fn find_king(position: &ChessPosition, color: ChessColor) -> Option<Square> {
    for sq in 0_u8..64 {
        let Some(piece) = position.piece_at(sq) else {
            continue;
        };
        if piece.color == color && piece.kind == ChessPieceKind::King {
            return Some(sq);
        }
    }
    None
}

fn piece_attacks_square(
    position: &ChessPosition,
    from: Square,
    piece: ChessPiece,
    target: Square,
) -> bool {
    if from == target {
        return false;
    }

    let file_delta = file_of(target) as i8 - file_of(from) as i8;
    let rank_delta = rank_of(target) as i8 - rank_of(from) as i8;
    let abs_file = file_delta.unsigned_abs();
    let abs_rank = rank_delta.unsigned_abs();

    match piece.kind {
        ChessPieceKind::Pawn => {
            let forward = match piece.color {
                ChessColor::White => 1_i8,
                ChessColor::Black => -1_i8,
            };
            rank_delta == forward && abs_file == 1
        }
        ChessPieceKind::Knight => {
            (abs_file == 1 && abs_rank == 2) || (abs_file == 2 && abs_rank == 1)
        }
        ChessPieceKind::King => abs_file <= 1 && abs_rank <= 1,
        ChessPieceKind::Bishop => {
            abs_file == abs_rank && clear_line(position, from, target, file_delta, rank_delta)
        }
        ChessPieceKind::Rook => {
            (file_delta == 0 || rank_delta == 0)
                && clear_line(position, from, target, file_delta, rank_delta)
        }
        ChessPieceKind::Queen => {
            ((abs_file == abs_rank) || (file_delta == 0 || rank_delta == 0))
                && clear_line(position, from, target, file_delta, rank_delta)
        }
    }
}

fn clear_line(
    position: &ChessPosition,
    from: Square,
    target: Square,
    file_delta: i8,
    rank_delta: i8,
) -> bool {
    let step_file = file_delta.signum();
    let step_rank = rank_delta.signum();
    let mut file = file_of(from) as i8 + step_file;
    let mut rank = rank_of(from) as i8 + step_rank;

    while (0..8).contains(&file) && (0..8).contains(&rank) {
        let sq = square(file as u8, rank as u8).expect("validated board coordinates");
        if sq == target {
            return true;
        }
        if position.piece_at(sq).is_some() {
            return false;
        }
        file += step_file;
        rank += step_rank;
    }

    false
}

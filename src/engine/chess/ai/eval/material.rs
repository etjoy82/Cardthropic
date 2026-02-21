use crate::game::{ChessPieceKind, ChessPosition};

fn piece_value(kind: ChessPieceKind) -> i32 {
    match kind {
        ChessPieceKind::Pawn => 100,
        ChessPieceKind::Knight => 320,
        ChessPieceKind::Bishop => 330,
        ChessPieceKind::Rook => 500,
        ChessPieceKind::Queen => 900,
        ChessPieceKind::King => 0,
    }
}

pub fn white_minus_black(position: &ChessPosition) -> i32 {
    let mut white = 0_i32;
    let mut black = 0_i32;
    let mut white_bishops = 0_i32;
    let mut black_bishops = 0_i32;

    for sq in 0_u8..64 {
        let Some(piece) = position.piece_at(sq) else {
            continue;
        };
        let value = piece_value(piece.kind);
        match piece.color {
            crate::game::ChessColor::White => {
                white += value;
                if piece.kind == ChessPieceKind::Bishop {
                    white_bishops += 1;
                }
            }
            crate::game::ChessColor::Black => {
                black += value;
                if piece.kind == ChessPieceKind::Bishop {
                    black_bishops += 1;
                }
            }
        }
    }

    let white_pair_bonus = if white_bishops >= 2 { 20 } else { 0 };
    let black_pair_bonus = if black_bishops >= 2 { 20 } else { 0 };
    (white + white_pair_bonus) - (black + black_pair_bonus)
}

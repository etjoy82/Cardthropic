use crate::game::{file_of, rank_of, ChessColor, ChessPieceKind, ChessPosition};

fn center_bonus(file: u8, rank: u8) -> i32 {
    let df = (3_i32 - i32::from(file))
        .abs()
        .min((4_i32 - i32::from(file)).abs());
    let dr = (3_i32 - i32::from(rank))
        .abs()
        .min((4_i32 - i32::from(rank)).abs());
    let dist = df + dr;
    (6 - dist).max(0)
}

fn piece_square_bonus(kind: ChessPieceKind, file: u8, rank: u8, color: ChessColor) -> i32 {
    let center = center_bonus(file, rank);
    let forward_rank = match color {
        ChessColor::White => i32::from(rank),
        ChessColor::Black => i32::from(7_u8.saturating_sub(rank)),
    };

    match kind {
        ChessPieceKind::Pawn => center * 2 + forward_rank * 2,
        ChessPieceKind::Knight => center * 5,
        ChessPieceKind::Bishop => center * 4,
        ChessPieceKind::Rook => center * 2,
        ChessPieceKind::Queen => center * 2,
        ChessPieceKind::King => {
            // Mild centralization incentive kept intentionally small for now.
            center
        }
    }
}

pub fn white_minus_black(position: &ChessPosition) -> i32 {
    let mut score = 0_i32;
    for sq in 0_u8..64 {
        let Some(piece) = position.piece_at(sq) else {
            continue;
        };
        let file = file_of(sq);
        let rank = rank_of(sq);
        let bonus = piece_square_bonus(piece.kind, file, rank, piece.color);
        match piece.color {
            ChessColor::White => score += bonus,
            ChessColor::Black => score -= bonus,
        }
    }
    score
}

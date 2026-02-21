use crate::game::{
    file_of, rank_of, square, square_attacked_by, ChessColor, ChessPiece, ChessPieceKind,
    ChessPosition, Square,
};

const HANGING_QUEEN_PENALTY: i32 = 220;
const HANGING_ROOK_PENALTY: i32 = 140;
const CONTESTED_QUEEN_PENALTY: i32 = 32;
const CONTESTED_ROOK_PENALTY: i32 = 20;
const TRAPPED_QUEEN_PENALTY_UNDEFENDED: i32 = 170;
const TRAPPED_QUEEN_PENALTY_DEFENDED: i32 = 90;
const SEMI_TRAPPED_QUEEN_PENALTY_UNDEFENDED: i32 = 80;
const SEMI_TRAPPED_QUEEN_PENALTY_DEFENDED: i32 = 38;
const TRAPPED_ROOK_PENALTY_UNDEFENDED: i32 = 110;
const TRAPPED_ROOK_PENALTY_DEFENDED: i32 = 60;
const SEMI_TRAPPED_ROOK_PENALTY_UNDEFENDED: i32 = 48;
const SEMI_TRAPPED_ROOK_PENALTY_DEFENDED: i32 = 22;

fn major_piece_penalty(kind: ChessPieceKind, defended: bool) -> i32 {
    match (kind, defended) {
        (ChessPieceKind::Queen, false) => HANGING_QUEEN_PENALTY,
        (ChessPieceKind::Queen, true) => CONTESTED_QUEEN_PENALTY,
        (ChessPieceKind::Rook, false) => HANGING_ROOK_PENALTY,
        (ChessPieceKind::Rook, true) => CONTESTED_ROOK_PENALTY,
        _ => 0,
    }
}

fn major_piece_trapped_penalty(kind: ChessPieceKind, defended: bool, safe_escapes: u8) -> i32 {
    match (kind, defended, safe_escapes) {
        (ChessPieceKind::Queen, false, 0) => TRAPPED_QUEEN_PENALTY_UNDEFENDED,
        (ChessPieceKind::Queen, true, 0) => TRAPPED_QUEEN_PENALTY_DEFENDED,
        (ChessPieceKind::Queen, false, 1) => SEMI_TRAPPED_QUEEN_PENALTY_UNDEFENDED,
        (ChessPieceKind::Queen, true, 1) => SEMI_TRAPPED_QUEEN_PENALTY_DEFENDED,
        (ChessPieceKind::Rook, false, 0) => TRAPPED_ROOK_PENALTY_UNDEFENDED,
        (ChessPieceKind::Rook, true, 0) => TRAPPED_ROOK_PENALTY_DEFENDED,
        (ChessPieceKind::Rook, false, 1) => SEMI_TRAPPED_ROOK_PENALTY_UNDEFENDED,
        (ChessPieceKind::Rook, true, 1) => SEMI_TRAPPED_ROOK_PENALTY_DEFENDED,
        _ => 0,
    }
}

fn is_major_escape_square_safe(position: &ChessPosition, piece: ChessPiece, to: Square) -> bool {
    if let Some(blocker) = position.piece_at(to) {
        if blocker.color == piece.color {
            return false;
        }
    }
    !square_attacked_by(position, to, piece.color.opposite())
}

fn count_safe_sliding_escapes(
    position: &ChessPosition,
    from: Square,
    piece: ChessPiece,
    directions: &[(i8, i8)],
) -> u8 {
    let mut safe = 0_u8;
    for (df, dr) in directions {
        let mut f = file_of(from) as i8 + df;
        let mut r = rank_of(from) as i8 + dr;
        while (0..8).contains(&f) && (0..8).contains(&r) {
            let to = square(f as u8, r as u8).expect("in-bounds board square");
            if let Some(blocker) = position.piece_at(to) {
                if blocker.color != piece.color && is_major_escape_square_safe(position, piece, to)
                {
                    safe = safe.saturating_add(1);
                }
                break;
            }
            if is_major_escape_square_safe(position, piece, to) {
                safe = safe.saturating_add(1);
            }
            f += df;
            r += dr;
        }
    }
    safe
}

fn count_safe_major_escapes(position: &ChessPosition, from: Square, piece: ChessPiece) -> u8 {
    const ROOK_DIRS: [(i8, i8); 4] = [(1, 0), (-1, 0), (0, 1), (0, -1)];
    const BISHOP_DIRS: [(i8, i8); 4] = [(1, 1), (1, -1), (-1, 1), (-1, -1)];
    match piece.kind {
        ChessPieceKind::Rook => count_safe_sliding_escapes(position, from, piece, &ROOK_DIRS),
        ChessPieceKind::Queen => {
            count_safe_sliding_escapes(position, from, piece, &ROOK_DIRS).saturating_add(
                count_safe_sliding_escapes(position, from, piece, &BISHOP_DIRS),
            )
        }
        _ => 0,
    }
}

pub fn white_minus_black(position: &ChessPosition) -> i32 {
    let mut white_penalty = 0_i32;
    let mut black_penalty = 0_i32;

    for sq in 0_u8..64 {
        let Some(piece) = position.piece_at(sq) else {
            continue;
        };
        if !matches!(piece.kind, ChessPieceKind::Queen | ChessPieceKind::Rook) {
            continue;
        }

        let attacker_color = piece.color.opposite();
        if !square_attacked_by(position, sq, attacker_color) {
            continue;
        }
        let defended = square_attacked_by(position, sq, piece.color);
        let mut penalty = major_piece_penalty(piece.kind, defended);
        let safe_escapes = count_safe_major_escapes(position, sq, piece);
        penalty += major_piece_trapped_penalty(piece.kind, defended, safe_escapes);

        match piece.color {
            ChessColor::White => white_penalty += penalty,
            ChessColor::Black => black_penalty += penalty,
        }
    }

    black_penalty - white_penalty
}

#[cfg(test)]
mod tests {
    use super::white_minus_black;
    use crate::game::{decode_fen, ChessVariant};

    #[test]
    fn undefended_hanging_white_queen_gets_penalized() {
        let position = decode_fen("4k3/6b1/8/8/3Q4/8/8/4K3 w - - 0 1", ChessVariant::Standard)
            .expect("valid FEN");

        assert!(white_minus_black(&position) < 0);
    }

    #[test]
    fn defended_major_piece_is_less_bad_than_hanging_major_piece() {
        let hanging = decode_fen("4k3/6b1/8/8/3Q4/8/8/4K3 w - - 0 1", ChessVariant::Standard)
            .expect("valid FEN");
        let defended = decode_fen(
            "4k3/6b1/8/8/3Q4/2B5/8/4K3 w - - 0 1",
            ChessVariant::Standard,
        )
        .expect("valid FEN");

        assert!(white_minus_black(&defended) > white_minus_black(&hanging));
    }

    #[test]
    fn undefended_hanging_black_rook_benefits_white() {
        let position = decode_fen("4k3/8/8/3r4/8/8/6B1/4K3 w - - 0 1", ChessVariant::Standard)
            .expect("valid FEN");

        assert!(white_minus_black(&position) > 0);
    }

    #[test]
    fn trapped_queen_penalized_more_than_mobile_attacked_queen() {
        let trapped = decode_fen(
            "4k3/8/8/2PPPn2/2PQP3/2PPP3/8/4K3 w - - 0 1",
            ChessVariant::Standard,
        )
        .expect("valid FEN");
        let mobile = decode_fen(
            "4k3/8/8/2PPPn2/1P1QP3/2PPP3/8/4K3 w - - 0 1",
            ChessVariant::Standard,
        )
        .expect("valid FEN");

        assert!(white_minus_black(&trapped) < white_minus_black(&mobile));
    }
}

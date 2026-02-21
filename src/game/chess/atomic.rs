use super::moves::ChessMove;
use super::position::ChessPosition;
use super::types::{file_of, rank_of, square, ChessColor, ChessPieceKind, Square};

pub(super) fn blast_zone(center: Square) -> Vec<Square> {
    let mut out = Vec::with_capacity(9);
    let center_file = file_of(center) as i8;
    let center_rank = rank_of(center) as i8;
    for file_delta in -1_i8..=1 {
        for rank_delta in -1_i8..=1 {
            let file = center_file + file_delta;
            let rank = center_rank + rank_delta;
            if !(0..8).contains(&file) || !(0..8).contains(&rank) {
                continue;
            }
            if let Some(sq) = square(file as u8, rank as u8) {
                out.push(sq);
            }
        }
    }
    out
}

pub(super) fn capture_center_for_move(
    position: &ChessPosition,
    chess_move: ChessMove,
    side_to_move: ChessColor,
) -> Option<Square> {
    if chess_move.is_castle_kingside || chess_move.is_castle_queenside {
        return None;
    }
    if chess_move.is_en_passant {
        if position.en_passant() != Some(chess_move.to) {
            return None;
        }
        let capture_square = square(file_of(chess_move.to), rank_of(chess_move.from))?;
        let captured = position.piece_at(capture_square)?;
        if captured.kind == ChessPieceKind::Pawn && captured.color != side_to_move {
            return Some(capture_square);
        }
        return None;
    }
    let target = position.piece_at(chess_move.to)?;
    (target.color != side_to_move).then_some(chess_move.to)
}

pub(super) fn is_king_capture_move(
    position: &ChessPosition,
    chess_move: ChessMove,
    side_to_move: ChessColor,
) -> bool {
    position
        .piece_at(chess_move.from)
        .is_some_and(|piece| piece.color == side_to_move && piece.kind == ChessPieceKind::King)
        && capture_center_for_move(position, chess_move, side_to_move).is_some()
}

pub(super) fn find_king_square(position: &ChessPosition, color: ChessColor) -> Option<Square> {
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

pub(super) fn is_in_check(position: &ChessPosition, color: ChessColor) -> bool {
    let Some(king_square) = find_king_square(position, color) else {
        return false;
    };
    square_attacked_by(position, king_square, color.opposite())
}

pub(super) fn square_attacked_by(position: &ChessPosition, target: Square, by: ChessColor) -> bool {
    for from in 0_u8..64 {
        let Some(piece) = position.piece_at(from) else {
            continue;
        };
        if piece.color != by || piece.kind == ChessPieceKind::King {
            continue;
        }
        if piece_explodes_target(position, from, piece.kind, by, target) {
            return true;
        }
    }
    false
}

fn piece_explodes_target(
    position: &ChessPosition,
    from: Square,
    kind: ChessPieceKind,
    color: ChessColor,
    target: Square,
) -> bool {
    match kind {
        ChessPieceKind::Pawn => pawn_explodes_target(position, from, color, target),
        ChessPieceKind::Knight => leaper_explodes_target(
            position,
            from,
            color,
            target,
            &[
                (1, 2),
                (2, 1),
                (2, -1),
                (1, -2),
                (-1, -2),
                (-2, -1),
                (-2, 1),
                (-1, 2),
            ],
        ),
        ChessPieceKind::Bishop => slider_explodes_target(
            position,
            from,
            color,
            target,
            &[(-1, -1), (1, -1), (-1, 1), (1, 1)],
        ),
        ChessPieceKind::Rook => slider_explodes_target(
            position,
            from,
            color,
            target,
            &[(-1, 0), (1, 0), (0, -1), (0, 1)],
        ),
        ChessPieceKind::Queen => slider_explodes_target(
            position,
            from,
            color,
            target,
            &[
                (-1, -1),
                (1, -1),
                (-1, 1),
                (1, 1),
                (-1, 0),
                (1, 0),
                (0, -1),
                (0, 1),
            ],
        ),
        ChessPieceKind::King => false,
    }
}

fn pawn_explodes_target(
    position: &ChessPosition,
    from: Square,
    color: ChessColor,
    target: Square,
) -> bool {
    let step = match color {
        ChessColor::White => 1_i8,
        ChessColor::Black => -1_i8,
    };

    for file_delta in [-1_i8, 1_i8] {
        let Some(capture_square) = offset_square(from, file_delta, step) else {
            continue;
        };
        if position
            .piece_at(capture_square)
            .is_some_and(|piece| piece.color != color)
            && explosion_hits(capture_square, target)
        {
            return true;
        }
    }

    if position.side_to_move() == color {
        if let Some(ep_target) = position.en_passant() {
            let from_file = file_of(from) as i8;
            let from_rank = rank_of(from) as i8;
            let ep_file = file_of(ep_target) as i8;
            let ep_rank = rank_of(ep_target) as i8;
            if ep_rank == from_rank + step && (from_file - ep_file).unsigned_abs() == 1 {
                if let Some(capture_center) = square(ep_file as u8, from_rank as u8) {
                    if position.piece_at(capture_center).is_some_and(|piece| {
                        piece.kind == ChessPieceKind::Pawn && piece.color != color
                    }) && explosion_hits(capture_center, target)
                    {
                        return true;
                    }
                }
            }
        }
    }

    false
}

fn leaper_explodes_target(
    position: &ChessPosition,
    from: Square,
    color: ChessColor,
    target: Square,
    deltas: &[(i8, i8)],
) -> bool {
    for &(file_delta, rank_delta) in deltas {
        let Some(capture_square) = offset_square(from, file_delta, rank_delta) else {
            continue;
        };
        if position
            .piece_at(capture_square)
            .is_some_and(|piece| piece.color != color)
            && explosion_hits(capture_square, target)
        {
            return true;
        }
    }
    false
}

fn slider_explodes_target(
    position: &ChessPosition,
    from: Square,
    color: ChessColor,
    target: Square,
    directions: &[(i8, i8)],
) -> bool {
    for &(file_step, rank_step) in directions {
        let mut file = file_of(from) as i8;
        let mut rank = rank_of(from) as i8;
        loop {
            file += file_step;
            rank += rank_step;
            if !(0..8).contains(&file) || !(0..8).contains(&rank) {
                break;
            }
            let capture_square = square(file as u8, rank as u8).expect("checked in bounds");
            let Some(captured_piece) = position.piece_at(capture_square) else {
                continue;
            };
            if captured_piece.color != color && explosion_hits(capture_square, target) {
                return true;
            }
            break;
        }
    }
    false
}

fn offset_square(from: Square, file_delta: i8, rank_delta: i8) -> Option<Square> {
    let file = file_of(from) as i8 + file_delta;
    let rank = rank_of(from) as i8 + rank_delta;
    if !(0..8).contains(&file) || !(0..8).contains(&rank) {
        return None;
    }
    square(file as u8, rank as u8)
}

fn explosion_hits(center: Square, target: Square) -> bool {
    let file_diff = (file_of(center) as i16 - file_of(target) as i16).unsigned_abs();
    let rank_diff = (rank_of(center) as i16 - rank_of(target) as i16).unsigned_abs();
    file_diff <= 1 && rank_diff <= 1
}

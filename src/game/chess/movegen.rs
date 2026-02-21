use super::castling::can_castle;
use super::moves::ChessMove;
use super::position::ChessPosition;
use super::types::{file_of, rank_of, square, ChessColor, ChessPieceKind, Square};

pub fn generate_pseudo_legal_moves(position: &ChessPosition) -> Vec<ChessMove> {
    let side_to_move = position.side_to_move();
    let mut moves = Vec::with_capacity(64);

    for from in 0_u8..64 {
        let Some(piece) = position.piece_at(from) else {
            continue;
        };
        if piece.color != side_to_move {
            continue;
        }

        match piece.kind {
            ChessPieceKind::Pawn => generate_pawn_moves(position, from, side_to_move, &mut moves),
            ChessPieceKind::Knight => {
                generate_leaper_moves(
                    position,
                    from,
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
                    &mut moves,
                );
            }
            ChessPieceKind::King => {
                generate_leaper_moves(
                    position,
                    from,
                    &[
                        (-1, -1),
                        (0, -1),
                        (1, -1),
                        (-1, 0),
                        (1, 0),
                        (-1, 1),
                        (0, 1),
                        (1, 1),
                    ],
                    &mut moves,
                );
                generate_castling_moves(position, side_to_move, &mut moves);
            }
            ChessPieceKind::Bishop => {
                generate_slider_moves(
                    position,
                    from,
                    &[(-1, -1), (1, -1), (-1, 1), (1, 1)],
                    &mut moves,
                );
            }
            ChessPieceKind::Rook => {
                generate_slider_moves(
                    position,
                    from,
                    &[(-1, 0), (1, 0), (0, -1), (0, 1)],
                    &mut moves,
                );
            }
            ChessPieceKind::Queen => {
                generate_slider_moves(
                    position,
                    from,
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
                    &mut moves,
                );
            }
        }
    }

    moves
}

fn generate_pawn_moves(
    position: &ChessPosition,
    from: Square,
    color: ChessColor,
    moves: &mut Vec<ChessMove>,
) {
    let from_file = file_of(from) as i8;
    let from_rank = rank_of(from) as i8;
    let (step, start_rank, promotion_rank) = match color {
        ChessColor::White => (1_i8, 1_i8, 7_i8),
        ChessColor::Black => (-1_i8, 6_i8, 0_i8),
    };

    if let Some(one_step) = offset_square(from, 0, step) {
        if position.piece_at(one_step).is_none() {
            let to_rank = rank_of(one_step) as i8;
            if to_rank == promotion_rank {
                push_promotions(moves, from, one_step);
            } else {
                moves.push(ChessMove::new(from, one_step));
                if from_rank == start_rank {
                    if let Some(two_step) = offset_square(from, 0, step * 2) {
                        if position.piece_at(two_step).is_none() {
                            moves.push(ChessMove::new(from, two_step));
                        }
                    }
                }
            }
        }
    }

    for file_delta in [-1_i8, 1_i8] {
        let Some(target) = offset_square(from, file_delta, step) else {
            continue;
        };
        let target_file = from_file + file_delta;
        let to_rank = rank_of(target) as i8;

        if let Some(target_piece) = position.piece_at(target) {
            if target_piece.color != color {
                if to_rank == promotion_rank {
                    push_promotions(moves, from, target);
                } else {
                    moves.push(ChessMove::new(from, target));
                }
            }
            continue;
        }

        if position.en_passant() == Some(target) {
            let Some(captured_square) = square(target_file as u8, from_rank as u8) else {
                continue;
            };
            let Some(captured_piece) = position.piece_at(captured_square) else {
                continue;
            };
            if captured_piece.kind == ChessPieceKind::Pawn && captured_piece.color != color {
                moves.push(ChessMove::new(from, target).as_en_passant());
            }
        }
    }
}

fn push_promotions(moves: &mut Vec<ChessMove>, from: Square, to: Square) {
    for promotion in [
        ChessPieceKind::Queen,
        ChessPieceKind::Rook,
        ChessPieceKind::Bishop,
        ChessPieceKind::Knight,
    ] {
        moves.push(ChessMove::new(from, to).with_promotion(promotion));
    }
}

fn generate_leaper_moves(
    position: &ChessPosition,
    from: Square,
    deltas: &[(i8, i8)],
    moves: &mut Vec<ChessMove>,
) {
    let Some(piece) = position.piece_at(from) else {
        return;
    };
    for &(file_delta, rank_delta) in deltas {
        let Some(to) = offset_square(from, file_delta, rank_delta) else {
            continue;
        };
        if let Some(target) = position.piece_at(to) {
            if target.color == piece.color {
                continue;
            }
        }
        moves.push(ChessMove::new(from, to));
    }
}

fn generate_slider_moves(
    position: &ChessPosition,
    from: Square,
    directions: &[(i8, i8)],
    moves: &mut Vec<ChessMove>,
) {
    let Some(piece) = position.piece_at(from) else {
        return;
    };

    for &(file_step, rank_step) in directions {
        let mut file = file_of(from) as i8;
        let mut rank = rank_of(from) as i8;
        loop {
            file += file_step;
            rank += rank_step;
            if !(0..8).contains(&file) || !(0..8).contains(&rank) {
                break;
            }
            let to = square(file as u8, rank as u8).expect("checked in bounds");
            if let Some(target) = position.piece_at(to) {
                if target.color != piece.color {
                    moves.push(ChessMove::new(from, to));
                }
                break;
            }
            moves.push(ChessMove::new(from, to));
        }
    }
}

fn offset_square(from: Square, file_delta: i8, rank_delta: i8) -> Option<Square> {
    let file = file_of(from) as i8 + file_delta;
    let rank = rank_of(from) as i8 + rank_delta;
    if !(0..8).contains(&file) || !(0..8).contains(&rank) {
        return None;
    }
    square(file as u8, rank as u8)
}

fn generate_castling_moves(
    position: &ChessPosition,
    color: ChessColor,
    moves: &mut Vec<ChessMove>,
) {
    if let Some(layout) = can_castle(position, color, true) {
        moves.push(ChessMove::new(layout.king_from, layout.king_to).as_kingside_castle());
    }
    if let Some(layout) = can_castle(position, color, false) {
        moves.push(ChessMove::new(layout.king_from, layout.king_to).as_queenside_castle());
    }
}

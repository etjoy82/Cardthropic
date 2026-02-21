use super::atomic::blast_zone;
use super::castling::{can_castle, castling_triplet};
use super::moves::ChessMove;
use super::position::{CastlingRights, ChessPosition};
use super::rules::ChessRuleset;
use super::types::{file_of, rank_of, square, ChessColor, ChessPiece, ChessPieceKind, Square};

pub fn apply_move(position: &mut ChessPosition, chess_move: ChessMove) -> bool {
    match ChessRuleset::for_variant(position.variant()) {
        ChessRuleset::Classical => apply_move_classical(position, chess_move),
        ChessRuleset::Atomic => apply_move_atomic(position, chess_move),
    }
}

fn apply_move_atomic(position: &mut ChessPosition, chess_move: ChessMove) -> bool {
    if chess_move.from as usize >= 64 || chess_move.to as usize >= 64 {
        return false;
    }

    let side_to_move = position.side_to_move();
    if chess_move.is_castle_kingside || chess_move.is_castle_queenside {
        return apply_castle_move(position, chess_move, side_to_move);
    }

    let Some(mut moving_piece) = position.piece_at(chess_move.from) else {
        return false;
    };
    if moving_piece.color != side_to_move {
        return false;
    }

    if let Some(promoted_kind) = chess_move.promotion {
        if moving_piece.kind != ChessPieceKind::Pawn {
            return false;
        }
        if matches!(promoted_kind, ChessPieceKind::King | ChessPieceKind::Pawn) {
            return false;
        }
        let promotion_rank = match side_to_move {
            ChessColor::White => 7,
            ChessColor::Black => 0,
        };
        if rank_of(chess_move.to) != promotion_rank {
            return false;
        }
        moving_piece.kind = promoted_kind;
    } else if moving_piece.kind == ChessPieceKind::Pawn {
        let promotion_rank = match side_to_move {
            ChessColor::White => 7,
            ChessColor::Black => 0,
        };
        if rank_of(chess_move.to) == promotion_rank {
            return false;
        }
    }

    let mut capture_square = None;
    let mut captured_piece = None;
    if chess_move.is_en_passant {
        if moving_piece.kind != ChessPieceKind::Pawn {
            return false;
        }
        if position.en_passant() != Some(chess_move.to)
            || position.piece_at(chess_move.to).is_some()
        {
            return false;
        }
        let Some(ep_capture_square) = square(file_of(chess_move.to), rank_of(chess_move.from))
        else {
            return false;
        };
        let Some(ep_captured_piece) = position.piece_at(ep_capture_square) else {
            return false;
        };
        if ep_captured_piece.kind != ChessPieceKind::Pawn || ep_captured_piece.color == side_to_move
        {
            return false;
        }
        capture_square = Some(ep_capture_square);
        captured_piece = Some(ep_captured_piece);
    } else if let Some(target_piece) = position.piece_at(chess_move.to) {
        if target_piece.color == side_to_move {
            return false;
        }
        capture_square = Some(chess_move.to);
        captured_piece = Some(target_piece);
    }

    // Atomic chess forbids king captures because captures always explode.
    if moving_piece.kind == ChessPieceKind::King && captured_piece.is_some() {
        return false;
    }

    let mut castling_rights = position.castling_rights();
    update_castling_rights_for_move(
        position,
        &mut castling_rights,
        side_to_move,
        moving_piece.kind,
        chess_move.from,
    );
    if let (Some(captured), Some(captured_sq)) = (captured_piece, capture_square) {
        if captured.kind == ChessPieceKind::Rook {
            update_castling_rights_for_capture(
                position,
                &mut castling_rights,
                side_to_move.opposite(),
                captured_sq,
            );
        }
    }

    let _ = position.set_piece(chess_move.from, None);
    if let Some(captured_sq) = capture_square {
        let _ = position.set_piece(captured_sq, None);
    }
    let _ = position.set_piece(chess_move.to, Some(moving_piece));

    if let Some(center) = capture_square {
        // Captures explode the capture center and all adjacent non-pawns.
        let mut blast = blast_zone(center);
        if !blast.contains(&chess_move.to) {
            blast.push(chess_move.to);
        }
        for sq in blast {
            let Some(piece) = position.piece_at(sq) else {
                continue;
            };
            let remove_piece = sq == chess_move.to || piece.kind != ChessPieceKind::Pawn;
            if !remove_piece {
                continue;
            }
            let _ = position.set_piece(sq, None);
            update_castling_rights_for_piece_loss(position, &mut castling_rights, piece, sq);
        }
        position.set_en_passant(None);
    } else {
        let mut en_passant = None;
        if moving_piece.kind == ChessPieceKind::Pawn
            && file_of(chess_move.from) == file_of(chess_move.to)
        {
            let from_rank = rank_of(chess_move.from) as i16;
            let to_rank = rank_of(chess_move.to) as i16;
            if (from_rank - to_rank).unsigned_abs() == 2 {
                let middle_rank = ((from_rank + to_rank) / 2) as u8;
                en_passant = square(file_of(chess_move.from), middle_rank);
            }
        }
        position.set_en_passant(en_passant);
    }
    position.set_castling_rights(castling_rights);

    if moving_piece.kind == ChessPieceKind::Pawn || captured_piece.is_some() {
        position.set_halfmove_clock(0);
    } else {
        position.set_halfmove_clock(position.halfmove_clock().saturating_add(1));
    }

    if side_to_move == ChessColor::Black {
        position.set_fullmove_number(position.fullmove_number().saturating_add(1));
    }
    position.set_side_to_move(side_to_move.opposite());
    true
}

fn apply_move_classical(position: &mut ChessPosition, chess_move: ChessMove) -> bool {
    if chess_move.from as usize >= 64 || chess_move.to as usize >= 64 {
        return false;
    }

    let side_to_move = position.side_to_move();
    if chess_move.is_castle_kingside || chess_move.is_castle_queenside {
        return apply_castle_move(position, chess_move, side_to_move);
    }

    let Some(mut moving_piece) = position.piece_at(chess_move.from) else {
        return false;
    };
    if moving_piece.color != side_to_move {
        return false;
    }

    if let Some(promoted_kind) = chess_move.promotion {
        if moving_piece.kind != ChessPieceKind::Pawn {
            return false;
        }
        if matches!(promoted_kind, ChessPieceKind::King | ChessPieceKind::Pawn) {
            return false;
        }
        let promotion_rank = match side_to_move {
            ChessColor::White => 7,
            ChessColor::Black => 0,
        };
        if rank_of(chess_move.to) != promotion_rank {
            return false;
        }
        moving_piece.kind = promoted_kind;
    } else if moving_piece.kind == ChessPieceKind::Pawn {
        let promotion_rank = match side_to_move {
            ChessColor::White => 7,
            ChessColor::Black => 0,
        };
        if rank_of(chess_move.to) == promotion_rank {
            return false;
        }
    }

    let mut capture_square = None;
    let mut captured_piece = None;
    if chess_move.is_en_passant {
        if moving_piece.kind != ChessPieceKind::Pawn {
            return false;
        }
        if position.en_passant() != Some(chess_move.to)
            || position.piece_at(chess_move.to).is_some()
        {
            return false;
        }
        let Some(ep_capture_square) = square(file_of(chess_move.to), rank_of(chess_move.from))
        else {
            return false;
        };
        let Some(ep_captured_piece) = position.piece_at(ep_capture_square) else {
            return false;
        };
        if ep_captured_piece.kind != ChessPieceKind::Pawn || ep_captured_piece.color == side_to_move
        {
            return false;
        }
        capture_square = Some(ep_capture_square);
        captured_piece = Some(ep_captured_piece);
    } else if let Some(target_piece) = position.piece_at(chess_move.to) {
        if target_piece.color == side_to_move {
            return false;
        }
        capture_square = Some(chess_move.to);
        captured_piece = Some(target_piece);
    }

    let mut castling_rights = position.castling_rights();
    update_castling_rights_for_move(
        position,
        &mut castling_rights,
        side_to_move,
        moving_piece.kind,
        chess_move.from,
    );
    if let (Some(captured), Some(captured_sq)) = (captured_piece, capture_square) {
        if captured.kind == ChessPieceKind::Rook {
            update_castling_rights_for_capture(
                position,
                &mut castling_rights,
                side_to_move.opposite(),
                captured_sq,
            );
        }
    }

    let _ = position.set_piece(chess_move.from, None);
    if let Some(captured_sq) = capture_square {
        let _ = position.set_piece(captured_sq, None);
    }
    let _ = position.set_piece(chess_move.to, Some(moving_piece));

    let mut en_passant = None;
    if moving_piece.kind == ChessPieceKind::Pawn
        && file_of(chess_move.from) == file_of(chess_move.to)
    {
        let from_rank = rank_of(chess_move.from) as i16;
        let to_rank = rank_of(chess_move.to) as i16;
        if (from_rank - to_rank).unsigned_abs() == 2 {
            let middle_rank = ((from_rank + to_rank) / 2) as u8;
            en_passant = square(file_of(chess_move.from), middle_rank);
        }
    }
    position.set_en_passant(en_passant);
    position.set_castling_rights(castling_rights);

    if moving_piece.kind == ChessPieceKind::Pawn || captured_piece.is_some() {
        position.set_halfmove_clock(0);
    } else {
        position.set_halfmove_clock(position.halfmove_clock().saturating_add(1));
    }

    if side_to_move == ChessColor::Black {
        position.set_fullmove_number(position.fullmove_number().saturating_add(1));
    }
    position.set_side_to_move(side_to_move.opposite());
    true
}

fn apply_castle_move(
    position: &mut ChessPosition,
    chess_move: ChessMove,
    side_to_move: ChessColor,
) -> bool {
    if chess_move.promotion.is_some() || chess_move.is_en_passant {
        return false;
    }
    let kingside = match (
        chess_move.is_castle_kingside,
        chess_move.is_castle_queenside,
    ) {
        (true, false) => true,
        (false, true) => false,
        _ => return false,
    };

    let Some(layout) = can_castle(position, side_to_move, kingside) else {
        return false;
    };
    if chess_move.from != layout.king_from || chess_move.to != layout.king_to {
        return false;
    }

    let _ = position.set_piece(layout.king_from, None);
    let _ = position.set_piece(layout.rook_from, None);
    let _ = position.set_piece(
        layout.king_to,
        Some(ChessPiece {
            color: side_to_move,
            kind: ChessPieceKind::King,
        }),
    );
    let _ = position.set_piece(
        layout.rook_to,
        Some(ChessPiece {
            color: side_to_move,
            kind: ChessPieceKind::Rook,
        }),
    );

    let mut rights = position.castling_rights();
    disable_castling_for_color(&mut rights, side_to_move);
    position.set_castling_rights(rights);
    position.set_en_passant(None);
    position.set_halfmove_clock(position.halfmove_clock().saturating_add(1));
    if side_to_move == ChessColor::Black {
        position.set_fullmove_number(position.fullmove_number().saturating_add(1));
    }
    position.set_side_to_move(side_to_move.opposite());
    true
}

fn update_castling_rights_for_move(
    position: &ChessPosition,
    rights: &mut CastlingRights,
    color: ChessColor,
    kind: ChessPieceKind,
    from: Square,
) {
    if kind == ChessPieceKind::King {
        disable_castling_for_color(rights, color);
        return;
    }
    if kind != ChessPieceKind::Rook {
        return;
    }

    if let Some(home) = castling_triplet(position, color, false).map(|layout| layout.rook_from) {
        if from == home {
            set_castling_side(rights, color, false, false);
        }
    }
    if let Some(home) = castling_triplet(position, color, true).map(|layout| layout.rook_from) {
        if from == home {
            set_castling_side(rights, color, true, false);
        }
    }
}

fn update_castling_rights_for_capture(
    position: &ChessPosition,
    rights: &mut CastlingRights,
    color: ChessColor,
    captured_square: Square,
) {
    if let Some(home) = castling_triplet(position, color, false).map(|layout| layout.rook_from) {
        if captured_square == home {
            set_castling_side(rights, color, false, false);
        }
    }
    if let Some(home) = castling_triplet(position, color, true).map(|layout| layout.rook_from) {
        if captured_square == home {
            set_castling_side(rights, color, true, false);
        }
    }
}

fn disable_castling_for_color(rights: &mut CastlingRights, color: ChessColor) {
    match color {
        ChessColor::White => {
            rights.white_king_side = false;
            rights.white_queen_side = false;
        }
        ChessColor::Black => {
            rights.black_king_side = false;
            rights.black_queen_side = false;
        }
    }
}

fn set_castling_side(rights: &mut CastlingRights, color: ChessColor, kingside: bool, value: bool) {
    match (color, kingside) {
        (ChessColor::White, true) => rights.white_king_side = value,
        (ChessColor::White, false) => rights.white_queen_side = value,
        (ChessColor::Black, true) => rights.black_king_side = value,
        (ChessColor::Black, false) => rights.black_queen_side = value,
    }
}

fn update_castling_rights_for_piece_loss(
    position: &ChessPosition,
    rights: &mut CastlingRights,
    piece: ChessPiece,
    at: Square,
) {
    match piece.kind {
        ChessPieceKind::King => disable_castling_for_color(rights, piece.color),
        ChessPieceKind::Rook => {
            update_castling_rights_for_capture(position, rights, piece.color, at);
        }
        _ => {}
    }
}

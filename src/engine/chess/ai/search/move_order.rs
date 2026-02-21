use crate::game::{ChessMove, ChessPieceKind, ChessPosition};

fn piece_value(kind: ChessPieceKind) -> i32 {
    match kind {
        ChessPieceKind::Pawn => 100,
        ChessPieceKind::Knight => 320,
        ChessPieceKind::Bishop => 330,
        ChessPieceKind::Rook => 500,
        ChessPieceKind::Queen => 900,
        ChessPieceKind::King => 20_000,
    }
}

fn move_order_score(position: &ChessPosition, mv: ChessMove, hash_move: Option<ChessMove>) -> i32 {
    let mut score = 0_i32;

    if Some(mv) == hash_move {
        score += 50_000;
    }
    if mv.is_castle_kingside || mv.is_castle_queenside {
        score += 1_000;
    }
    if mv.is_en_passant {
        score += 4_000;
    }
    if let Some(promote) = mv.promotion {
        score += 8_000 + piece_value(promote);
    }
    if let Some(captured) = position.piece_at(mv.to) {
        let victim = piece_value(captured.kind);
        let attacker = position
            .piece_at(mv.from)
            .map(|p| piece_value(p.kind))
            .unwrap_or(0);
        score += 5_000 + victim - (attacker / 10);
    }

    score
}

pub fn ordered_moves(position: &ChessPosition, hash_move: Option<ChessMove>) -> Vec<ChessMove> {
    let mut moves = crate::game::legal_moves(position);
    moves.sort_unstable_by_key(|mv| -move_order_score(position, *mv, hash_move));
    moves
}

pub fn ordered_capture_moves(position: &ChessPosition) -> Vec<ChessMove> {
    let mut captures = crate::game::legal_moves(position)
        .into_iter()
        .filter(|mv| {
            mv.is_en_passant || mv.promotion.is_some() || position.piece_at(mv.to).is_some()
        })
        .collect::<Vec<_>>();
    captures.sort_unstable_by_key(|mv| -move_order_score(position, *mv, None));
    captures
}

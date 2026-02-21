use super::atomic;
use super::attacks;
use super::position::ChessPosition;
use super::rules::ChessRuleset;
use super::types::{file_of, rank_of, square, ChessColor, ChessPieceKind, Square};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct CastlingTriplet {
    pub(super) king_from: Square,
    pub(super) rook_from: Square,
    pub(super) king_to: Square,
    pub(super) rook_to: Square,
}

pub(super) fn castling_right_enabled(
    position: &ChessPosition,
    color: ChessColor,
    kingside: bool,
) -> bool {
    let rights = position.castling_rights();
    match (color, kingside) {
        (ChessColor::White, true) => rights.white_king_side,
        (ChessColor::White, false) => rights.white_queen_side,
        (ChessColor::Black, true) => rights.black_king_side,
        (ChessColor::Black, false) => rights.black_queen_side,
    }
}

pub(super) fn castling_triplet(
    position: &ChessPosition,
    color: ChessColor,
    kingside: bool,
) -> Option<CastlingTriplet> {
    let back_rank = position.back_rank(color);
    let king_file = back_rank
        .iter()
        .position(|piece| *piece == ChessPieceKind::King)? as u8;
    let mut rook_files = back_rank
        .iter()
        .enumerate()
        .filter_map(|(idx, piece)| (*piece == ChessPieceKind::Rook).then_some(idx as u8))
        .collect::<Vec<_>>();
    rook_files.sort_unstable();
    let rook_file = if kingside {
        rook_files
            .iter()
            .copied()
            .filter(|file| *file > king_file)
            .min()?
    } else {
        rook_files
            .iter()
            .copied()
            .filter(|file| *file < king_file)
            .max()?
    };

    let rank = match color {
        ChessColor::White => 0,
        ChessColor::Black => 7,
    };
    let king_target_file = if kingside { 6 } else { 2 };
    let rook_target_file = if kingside { 5 } else { 3 };

    Some(CastlingTriplet {
        king_from: square(king_file, rank)?,
        rook_from: square(rook_file, rank)?,
        king_to: square(king_target_file, rank)?,
        rook_to: square(rook_target_file, rank)?,
    })
}

pub(super) fn can_castle(
    position: &ChessPosition,
    color: ChessColor,
    kingside: bool,
) -> Option<CastlingTriplet> {
    let ruleset = ChessRuleset::for_variant(position.variant());
    if !castling_right_enabled(position, color, kingside) {
        return None;
    }
    let layout = castling_triplet(position, color, kingside)?;
    if !has_required_castling_pieces(position, color, layout) {
        return None;
    }
    if is_in_check_for_ruleset(position, color, ruleset) {
        return None;
    }

    let king_path = line_squares_excluding_start(layout.king_from, layout.king_to)?;
    let rook_path = line_squares_excluding_start(layout.rook_from, layout.rook_to)?;

    let mut temp = position.clone();
    let _ = temp.set_piece(layout.king_from, None);
    let _ = temp.set_piece(layout.rook_from, None);

    for sq in king_path.iter().chain(rook_path.iter()) {
        if temp.piece_at(*sq).is_some() {
            return None;
        }
    }

    let enemy = color.opposite();
    for sq in king_path {
        if square_attacked_by_for_ruleset(&temp, sq, enemy, ruleset) {
            return None;
        }
    }

    Some(layout)
}

fn has_required_castling_pieces(
    position: &ChessPosition,
    color: ChessColor,
    layout: CastlingTriplet,
) -> bool {
    let king_ok = position
        .piece_at(layout.king_from)
        .is_some_and(|piece| piece.color == color && piece.kind == ChessPieceKind::King);
    let rook_ok = position
        .piece_at(layout.rook_from)
        .is_some_and(|piece| piece.color == color && piece.kind == ChessPieceKind::Rook);
    king_ok && rook_ok
}

fn line_squares_excluding_start(from: Square, to: Square) -> Option<Vec<Square>> {
    if from == to {
        return Some(Vec::new());
    }

    let from_file = file_of(from) as i8;
    let from_rank = rank_of(from) as i8;
    let to_file = file_of(to) as i8;
    let to_rank = rank_of(to) as i8;

    let (step_file, step_rank) = if from_file == to_file {
        (0_i8, (to_rank - from_rank).signum())
    } else if from_rank == to_rank {
        ((to_file - from_file).signum(), 0_i8)
    } else {
        return None;
    };

    let mut file = from_file + step_file;
    let mut rank = from_rank + step_rank;
    let mut out = Vec::new();
    while (0..8).contains(&file) && (0..8).contains(&rank) {
        let sq = square(file as u8, rank as u8).expect("validated board coordinates");
        out.push(sq);
        if sq == to {
            return Some(out);
        }
        file += step_file;
        rank += step_rank;
    }

    None
}

fn is_in_check_for_ruleset(
    position: &ChessPosition,
    color: ChessColor,
    ruleset: ChessRuleset,
) -> bool {
    match ruleset {
        ChessRuleset::Classical => attacks::is_in_check(position, color),
        ChessRuleset::Atomic => atomic::is_in_check(position, color),
    }
}

fn square_attacked_by_for_ruleset(
    position: &ChessPosition,
    target: Square,
    by: ChessColor,
    ruleset: ChessRuleset,
) -> bool {
    match ruleset {
        ChessRuleset::Classical => attacks::square_attacked_by(position, target, by),
        ChessRuleset::Atomic => atomic::square_attacked_by(position, target, by),
    }
}

use super::atomic;
use super::attacks;
use super::movegen::generate_pseudo_legal_moves;
use super::moves::ChessMove;
use super::position::ChessPosition;
use super::rules::ChessRuleset;
use super::types::{ChessColor, ChessPieceKind, Square};
use crate::game::apply_move;

pub const FIFTY_MOVE_RULE_HALFMOVE_LIMIT: u16 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChessTerminalState {
    Checkmate { winner: ChessColor },
    DrawStalemate,
    DrawFiftyMoveRule,
    DrawInsufficientMaterial,
}

impl ChessTerminalState {
    pub const fn is_draw(self) -> bool {
        !matches!(self, Self::Checkmate { .. })
    }
}

pub fn legal_moves(position: &ChessPosition) -> Vec<ChessMove> {
    match ChessRuleset::for_variant(position.variant()) {
        ChessRuleset::Classical => legal_moves_classical(position),
        ChessRuleset::Atomic => legal_moves_atomic(position),
    }
}

fn legal_moves_classical(position: &ChessPosition) -> Vec<ChessMove> {
    let side = position.side_to_move();
    let mut legal = Vec::new();
    for chess_move in generate_pseudo_legal_moves(position) {
        let mut next = position.clone();
        if !apply_move(&mut next, chess_move) {
            continue;
        }
        if !attacks::is_in_check(&next, side) {
            legal.push(chess_move);
        }
    }
    legal
}

pub fn is_in_check(position: &ChessPosition, color: ChessColor) -> bool {
    match ChessRuleset::for_variant(position.variant()) {
        ChessRuleset::Classical => attacks::is_in_check(position, color),
        ChessRuleset::Atomic => atomic::is_in_check(position, color),
    }
}

fn legal_moves_atomic(position: &ChessPosition) -> Vec<ChessMove> {
    let side = position.side_to_move();

    let mut legal = Vec::new();
    for chess_move in generate_pseudo_legal_moves(position) {
        if atomic::is_king_capture_move(position, chess_move, side) {
            continue;
        }
        let mut next = position.clone();
        if !apply_move(&mut next, chess_move) {
            continue;
        }
        if atomic::find_king_square(&next, side).is_none() {
            continue;
        }
        if !atomic::is_in_check(&next, side) {
            legal.push(chess_move);
        }
    }
    legal
}

fn terminal_state_classical(position: &ChessPosition) -> Option<ChessTerminalState> {
    if is_draw_by_fifty_move_rule(position) {
        return Some(ChessTerminalState::DrawFiftyMoveRule);
    }
    if is_draw_by_insufficient_material(position) {
        return Some(ChessTerminalState::DrawInsufficientMaterial);
    }

    let next_moves = legal_moves_classical(position);
    if next_moves.is_empty() {
        let side_to_move = position.side_to_move();
        if attacks::is_in_check(position, side_to_move) {
            Some(ChessTerminalState::Checkmate {
                winner: side_to_move.opposite(),
            })
        } else {
            Some(ChessTerminalState::DrawStalemate)
        }
    } else {
        None
    }
}

fn terminal_state_atomic(position: &ChessPosition) -> Option<ChessTerminalState> {
    let white_king = atomic::find_king_square(position, ChessColor::White);
    let black_king = atomic::find_king_square(position, ChessColor::Black);
    match (white_king, black_king) {
        (Some(_), None) => {
            return Some(ChessTerminalState::Checkmate {
                winner: ChessColor::White,
            });
        }
        (None, Some(_)) => {
            return Some(ChessTerminalState::Checkmate {
                winner: ChessColor::Black,
            });
        }
        (None, None) => return Some(ChessTerminalState::DrawStalemate),
        (Some(_), Some(_)) => {}
    }

    if is_draw_by_fifty_move_rule(position) {
        return Some(ChessTerminalState::DrawFiftyMoveRule);
    }

    let next_moves = legal_moves_atomic(position);
    if next_moves.is_empty() {
        let side_to_move = position.side_to_move();
        if atomic::is_in_check(position, side_to_move) {
            Some(ChessTerminalState::Checkmate {
                winner: side_to_move.opposite(),
            })
        } else {
            Some(ChessTerminalState::DrawStalemate)
        }
    } else {
        None
    }
}

pub fn square_attacked_by(position: &ChessPosition, target: Square, by: ChessColor) -> bool {
    match ChessRuleset::for_variant(position.variant()) {
        ChessRuleset::Classical => attacks::square_attacked_by(position, target, by),
        ChessRuleset::Atomic => atomic::square_attacked_by(position, target, by),
    }
}

pub fn is_draw_by_fifty_move_rule(position: &ChessPosition) -> bool {
    position.halfmove_clock() >= FIFTY_MOVE_RULE_HALFMOVE_LIMIT
}

pub fn is_draw_by_insufficient_material(position: &ChessPosition) -> bool {
    #[derive(Default, Clone, Copy)]
    struct MaterialCounts {
        pawns: u8,
        rooks: u8,
        queens: u8,
        bishops: u8,
        knights: u8,
    }

    fn has_mating_material(c: MaterialCounts) -> bool {
        // Practical mating-material rule:
        // if neither side has enough material to mate without opponent blunders,
        // we treat the position as an immediate draw.
        c.pawns > 0
            || c.rooks > 0
            || c.queens > 0
            || c.bishops >= 2
            || (c.bishops >= 1 && c.knights >= 1)
            || c.knights >= 3
    }

    let mut white = MaterialCounts::default();
    let mut black = MaterialCounts::default();
    for piece in position.board().iter().flatten() {
        let counts = match piece.color {
            ChessColor::White => &mut white,
            ChessColor::Black => &mut black,
        };
        match piece.kind {
            ChessPieceKind::King => {}
            ChessPieceKind::Pawn => counts.pawns = counts.pawns.saturating_add(1),
            ChessPieceKind::Rook => counts.rooks = counts.rooks.saturating_add(1),
            ChessPieceKind::Queen => counts.queens = counts.queens.saturating_add(1),
            ChessPieceKind::Bishop => counts.bishops = counts.bishops.saturating_add(1),
            ChessPieceKind::Knight => counts.knights = counts.knights.saturating_add(1),
        }
    }

    !has_mating_material(white) && !has_mating_material(black)
}

pub fn terminal_state(position: &ChessPosition) -> Option<ChessTerminalState> {
    match ChessRuleset::for_variant(position.variant()) {
        ChessRuleset::Classical => terminal_state_classical(position),
        ChessRuleset::Atomic => terminal_state_atomic(position),
    }
}

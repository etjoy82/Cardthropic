use super::boundary::execute;
use super::commands::{ChessCommand, ChessStatus};
use super::hint::best_move_hint;
use super::robot::pick_robot_move;
use crate::game::{parse_square, ChessColor, ChessMove, ChessPosition, ChessVariant};

#[test]
fn new_game_standard_initializes_ready_position() {
    let mut position = ChessPosition::empty(ChessVariant::Standard);
    let result = execute(
        &mut position,
        ChessCommand::NewGame {
            seed: 0,
            variant: ChessVariant::Standard,
        },
    );
    assert!(result.changed);
    assert_eq!(result.status, ChessStatus::Ready);
    assert_eq!(position.variant(), ChessVariant::Standard);
    assert_eq!(position.side_to_move(), ChessColor::White);
    assert_eq!(position.piece_count(crate::game::ChessColor::White), 16);
    assert_eq!(position.piece_count(crate::game::ChessColor::Black), 16);
}

#[test]
fn new_game_chess960_initializes_ready_position_with_white_to_move() {
    let mut position = ChessPosition::empty(ChessVariant::Chess960);
    let result = execute(
        &mut position,
        ChessCommand::NewGame {
            seed: 42,
            variant: ChessVariant::Chess960,
        },
    );
    assert!(result.changed);
    assert_eq!(result.status, ChessStatus::Ready);
    assert_eq!(position.variant(), ChessVariant::Chess960);
    assert_eq!(position.side_to_move(), ChessColor::White);
    assert_eq!(position.piece_count(crate::game::ChessColor::White), 16);
    assert_eq!(position.piece_count(crate::game::ChessColor::Black), 16);
}

#[test]
fn new_game_atomic_initializes_ready_position_with_white_to_move() {
    let mut position = ChessPosition::empty(ChessVariant::Atomic);
    let result = execute(
        &mut position,
        ChessCommand::NewGame {
            seed: 7,
            variant: ChessVariant::Atomic,
        },
    );
    assert!(result.changed);
    assert_eq!(result.status, ChessStatus::Ready);
    assert_eq!(position.variant(), ChessVariant::Atomic);
    assert_eq!(position.side_to_move(), ChessColor::White);
    assert_eq!(position.piece_count(crate::game::ChessColor::White), 16);
    assert_eq!(position.piece_count(crate::game::ChessColor::Black), 16);
}

#[test]
fn try_move_accepts_legal_move() {
    let mut position = ChessPosition::empty(ChessVariant::Standard);
    let _ = execute(
        &mut position,
        ChessCommand::NewGame {
            seed: 0,
            variant: ChessVariant::Standard,
        },
    );
    let result = execute(
        &mut position,
        ChessCommand::TryMove(ChessMove::new(sq("e2"), sq("e4"))),
    );
    assert!(result.changed);
    assert_eq!(result.status, ChessStatus::Ready);
    assert!(position.piece_at(sq("e4")).is_some());
    assert!(position.piece_at(sq("e2")).is_none());
}

#[test]
fn try_move_rejects_illegal_move() {
    let mut position = ChessPosition::empty(ChessVariant::Standard);
    let _ = execute(
        &mut position,
        ChessCommand::NewGame {
            seed: 0,
            variant: ChessVariant::Standard,
        },
    );
    let before = position.clone();
    let result = execute(
        &mut position,
        ChessCommand::TryMove(ChessMove::new(sq("e2"), sq("e5"))),
    );
    assert!(!result.changed);
    assert_eq!(result.status, ChessStatus::IllegalMove);
    assert_eq!(position, before);
}

#[test]
fn hint_move_is_legal_when_available() {
    let mut position = ChessPosition::empty(ChessVariant::Standard);
    let _ = execute(
        &mut position,
        ChessCommand::NewGame {
            seed: 0,
            variant: ChessVariant::Standard,
        },
    );
    let hint = best_move_hint(&position);
    assert!(
        hint.is_some_and(|mv| crate::game::legal_moves(&position).contains(&mv)),
        "hint should return a legal move in initial standard position"
    );
}

#[test]
fn robot_move_is_legal_when_available() {
    let mut position = ChessPosition::empty(ChessVariant::Standard);
    let _ = execute(
        &mut position,
        ChessCommand::NewGame {
            seed: 0,
            variant: ChessVariant::Standard,
        },
    );
    let mv = pick_robot_move(&position);
    assert!(
        mv.is_some_and(|m| crate::game::legal_moves(&position).contains(&m)),
        "robot should return a legal move in initial standard position"
    );
}

fn sq(name: &str) -> u8 {
    parse_square(name).expect("valid square")
}

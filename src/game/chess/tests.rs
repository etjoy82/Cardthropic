use super::{
    apply_move, atomic_position, chess960_back_rank_from_seed, chess960_position, decode_fen,
    encode_fen, file_of, generate_pseudo_legal_moves, is_in_check, is_valid_chess960_back_rank,
    legal_moves, parse_square, rank_of, standard_position, terminal_state, ChessColor, ChessMove,
    ChessPieceKind, ChessTerminalState, ChessVariant,
};

#[test]
fn standard_position_has_32_pieces_and_correct_side_to_move() {
    let position = standard_position();
    assert_eq!(position.variant(), ChessVariant::Standard);
    assert_eq!(position.side_to_move(), ChessColor::White);
    assert_eq!(position.piece_count(ChessColor::White), 16);
    assert_eq!(position.piece_count(ChessColor::Black), 16);
}

#[test]
fn atomic_position_has_32_pieces_and_correct_side_to_move() {
    let position = atomic_position();
    assert_eq!(position.variant(), ChessVariant::Atomic);
    assert_eq!(position.side_to_move(), ChessColor::White);
    assert_eq!(position.piece_count(ChessColor::White), 16);
    assert_eq!(position.piece_count(ChessColor::Black), 16);
}

#[test]
fn atomic_capture_explodes_adjacent_non_pawns_but_keeps_adjacent_pawns() {
    let mut position =
        decode_fen("k7/8/8/3n4/2PRB3/8/8/7K w - - 0 1", ChessVariant::Atomic).expect("valid FEN");
    assert!(apply_move(
        &mut position,
        ChessMove::new(sq("d4"), sq("d5")),
    ));

    assert!(position.piece_at(sq("d5")).is_none());
    assert!(position.piece_at(sq("e4")).is_none());
    assert!(position.piece_at(sq("c4")).is_some());
}

#[test]
fn atomic_king_capture_moves_are_not_legal() {
    let position =
        decode_fen("4k3/8/8/8/8/8/4p3/4K3 w - - 0 1", ChessVariant::Atomic).expect("valid FEN");
    let legal = legal_moves(&position);
    assert!(!legal
        .iter()
        .any(|mv| mv.from == sq("e1") && mv.to == sq("e2")));
}

#[test]
fn atomic_terminal_state_reports_king_explosion_as_checkmate() {
    let mut position =
        decode_fen("4k3/4q3/8/8/8/8/8/K3R3 w - - 0 1", ChessVariant::Atomic).expect("valid FEN");
    assert!(apply_move(
        &mut position,
        ChessMove::new(sq("e1"), sq("e7")),
    ));

    assert_eq!(
        terminal_state(&position),
        Some(ChessTerminalState::Checkmate {
            winner: ChessColor::White
        })
    );
}

#[test]
fn atomic_castling_is_blocked_while_in_atomic_check() {
    let position =
        decode_fen("k2r4/8/8/8/8/8/3P4/4K2R w K - 0 1", ChessVariant::Atomic).expect("valid FEN");
    assert!(is_in_check(&position, ChessColor::White));
    let legal = legal_moves(&position);
    assert!(!legal.iter().any(|mv| {
        mv.from == sq("e1") && mv.to == sq("g1") && mv.is_castle_kingside && !mv.is_castle_queenside
    }));
}

#[test]
fn chess960_back_rank_generation_respects_fischer_rules() {
    for seed in 0_u64..512 {
        let back_rank = chess960_back_rank_from_seed(seed);
        assert!(is_valid_chess960_back_rank(&back_rank));
    }
}

#[test]
fn chess960_position_uses_generated_back_rank() {
    let seed = 960_u64;
    let position = chess960_position(seed);
    let expected = chess960_back_rank_from_seed(seed);
    assert_eq!(*position.back_rank(ChessColor::White), expected);
    assert_eq!(*position.back_rank(ChessColor::Black), expected);
    assert_eq!(position.piece_count(ChessColor::White), 16);
    assert_eq!(position.piece_count(ChessColor::Black), 16);
}

#[test]
fn fen_roundtrip_for_standard_start_is_stable() {
    let position = standard_position();
    let fen = encode_fen(&position);
    let decoded = decode_fen(&fen, ChessVariant::Standard).expect("decode FEN");
    assert_eq!(decoded, position);
}

#[test]
fn chess960_back_rank_has_expected_piece_inventory() {
    let rank = chess960_back_rank_from_seed(42);
    let mut kings = 0;
    let mut queens = 0;
    let mut rooks = 0;
    let mut bishops = 0;
    let mut knights = 0;
    for piece in rank {
        match piece {
            ChessPieceKind::King => kings += 1,
            ChessPieceKind::Queen => queens += 1,
            ChessPieceKind::Rook => rooks += 1,
            ChessPieceKind::Bishop => bishops += 1,
            ChessPieceKind::Knight => knights += 1,
            ChessPieceKind::Pawn => panic!("pawn not allowed in chess back rank"),
        }
    }
    assert_eq!(kings, 1);
    assert_eq!(queens, 1);
    assert_eq!(rooks, 2);
    assert_eq!(bishops, 2);
    assert_eq!(knights, 2);
}

#[test]
fn standard_start_has_20_pseudo_and_legal_moves() {
    let position = standard_position();
    assert_eq!(generate_pseudo_legal_moves(&position).len(), 20);
    assert_eq!(legal_moves(&position).len(), 20);
}

#[test]
fn standard_start_places_queens_on_home_colors() {
    let position = standard_position();

    let white_queen = position.piece_at(sq("d1")).expect("white queen on d1");
    assert_eq!(white_queen.color, ChessColor::White);
    assert_eq!(white_queen.kind, ChessPieceKind::Queen);

    let white_king = position.piece_at(sq("e1")).expect("white king on e1");
    assert_eq!(white_king.color, ChessColor::White);
    assert_eq!(white_king.kind, ChessPieceKind::King);

    let black_queen = position.piece_at(sq("d8")).expect("black queen on d8");
    assert_eq!(black_queen.color, ChessColor::Black);
    assert_eq!(black_queen.kind, ChessPieceKind::Queen);

    let black_king = position.piece_at(sq("e8")).expect("black king on e8");
    assert_eq!(black_king.color, ChessColor::Black);
    assert_eq!(black_king.kind, ChessPieceKind::King);

    let white_queen_square_is_light = (file_of(sq("d1")) + rank_of(sq("d1"))) % 2 == 1;
    let black_queen_square_is_dark = (file_of(sq("d8")) + rank_of(sq("d8"))) % 2 == 0;
    assert!(white_queen_square_is_light);
    assert!(black_queen_square_is_dark);
}

#[test]
fn pawn_double_push_sets_en_passant_target() {
    let mut position = standard_position();
    let from = sq("e2");
    let to = sq("e4");
    assert!(apply_move(&mut position, ChessMove::new(from, to)));
    assert_eq!(position.en_passant(), Some(sq("e3")));
    assert_eq!(position.side_to_move(), ChessColor::Black);
}

#[test]
fn check_detection_finds_simple_rook_check() {
    let position =
        decode_fen("4k3/8/8/8/8/8/4r3/4K3 w - - 0 1", ChessVariant::Standard).expect("valid FEN");
    assert!(is_in_check(&position, ChessColor::White));
    assert!(!is_in_check(&position, ChessColor::Black));
}

#[test]
fn en_passant_move_is_generated_and_applied() {
    let mut position =
        decode_fen("4k3/8/8/3pP3/8/8/8/4K3 w - d6 0 1", ChessVariant::Standard).expect("valid FEN");

    let ep_move = legal_moves(&position)
        .into_iter()
        .find(|mv| mv.from == sq("e5") && mv.to == sq("d6") && mv.is_en_passant)
        .expect("expected en-passant move");

    assert!(apply_move(&mut position, ep_move));
    assert!(position.piece_at(sq("d5")).is_none());
    let piece = position.piece_at(sq("d6")).expect("white pawn on d6");
    assert_eq!(piece.color, ChessColor::White);
    assert_eq!(piece.kind, ChessPieceKind::Pawn);
}

#[test]
fn perft_depth_2_matches_known_start_value() {
    let position = standard_position();
    assert_eq!(perft(&position, 2), 400);
}

#[test]
fn perft_depth_3_matches_known_start_value() {
    let position = standard_position();
    assert_eq!(perft(&position, 3), 8_902);
}

#[test]
fn perft_depth_4_matches_known_start_value() {
    let position = standard_position();
    assert_eq!(perft(&position, 4), 197_281);
}

#[test]
fn standard_castling_moves_are_generated_when_clear() {
    let position = decode_fen(
        "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
        ChessVariant::Standard,
    )
    .expect("valid FEN");
    let legal = legal_moves(&position);
    assert!(legal.iter().any(|mv| {
        mv.from == sq("e1") && mv.to == sq("g1") && mv.is_castle_kingside && !mv.is_castle_queenside
    }));
    assert!(legal.iter().any(|mv| {
        mv.from == sq("e1") && mv.to == sq("c1") && mv.is_castle_queenside && !mv.is_castle_kingside
    }));
}

#[test]
fn castling_through_check_is_rejected() {
    let position = decode_fen(
        "r3k2r/5r2/8/8/8/8/8/R3K2R w KQkq - 0 1",
        ChessVariant::Standard,
    )
    .expect("valid FEN");
    let legal = legal_moves(&position);
    assert!(!legal
        .iter()
        .any(|mv| { mv.from == sq("e1") && mv.to == sq("g1") && mv.is_castle_kingside }));
}

#[test]
fn standard_castling_move_updates_king_rook_and_rights() {
    let mut position = decode_fen(
        "r3k2r/8/8/8/8/8/8/R3K2R w KQkq - 0 1",
        ChessVariant::Standard,
    )
    .expect("valid FEN");
    let castle = legal_moves(&position)
        .into_iter()
        .find(|mv| mv.from == sq("e1") && mv.to == sq("g1") && mv.is_castle_kingside)
        .expect("kingside castling move");
    assert!(apply_move(&mut position, castle));
    let king = position.piece_at(sq("g1")).expect("king on g1");
    let rook = position.piece_at(sq("f1")).expect("rook on f1");
    assert_eq!(king.kind, ChessPieceKind::King);
    assert_eq!(rook.kind, ChessPieceKind::Rook);
    assert!(position.piece_at(sq("e1")).is_none());
    assert!(position.piece_at(sq("h1")).is_none());
    let rights = position.castling_rights();
    assert!(!rights.white_king_side);
    assert!(!rights.white_queen_side);
    assert_eq!(position.side_to_move(), ChessColor::Black);
}

#[test]
fn chess960_queenside_castle_allows_king_to_stay_put() {
    let mut position =
        decode_fen("4k3/8/8/8/8/8/8/R1K4R w KQ - 0 1", ChessVariant::Chess960).expect("valid FEN");
    let castle = legal_moves(&position)
        .into_iter()
        .find(|mv| mv.from == sq("c1") && mv.to == sq("c1") && mv.is_castle_queenside)
        .expect("queenside castling move");
    assert!(apply_move(&mut position, castle));
    let king = position.piece_at(sq("c1")).expect("king remains on c1");
    let rook = position.piece_at(sq("d1")).expect("rook moved to d1");
    assert_eq!(king.kind, ChessPieceKind::King);
    assert_eq!(rook.kind, ChessPieceKind::Rook);
    assert!(position.piece_at(sq("a1")).is_none());
}

fn sq(name: &str) -> u8 {
    parse_square(name).expect("valid square")
}

fn perft(position: &super::ChessPosition, depth: u8) -> u64 {
    if depth == 0 {
        return 1;
    }
    let moves = legal_moves(position);
    if depth == 1 {
        return moves.len() as u64;
    }
    let mut total = 0_u64;
    for mv in moves {
        let mut next = position.clone();
        assert!(apply_move(&mut next, mv));
        total += perft(&next, depth - 1);
    }
    total
}

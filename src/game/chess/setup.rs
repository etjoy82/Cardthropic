use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::SeedableRng;

use super::position::{CastlingRights, ChessPosition};
use super::types::{square, ChessColor, ChessPiece, ChessPieceKind, ChessVariant};

pub const STANDARD_BACK_RANK: [ChessPieceKind; 8] = [
    ChessPieceKind::Rook,
    ChessPieceKind::Knight,
    ChessPieceKind::Bishop,
    ChessPieceKind::Queen,
    ChessPieceKind::King,
    ChessPieceKind::Bishop,
    ChessPieceKind::Knight,
    ChessPieceKind::Rook,
];

pub fn standard_position() -> ChessPosition {
    position_from_back_rank(ChessVariant::Standard, STANDARD_BACK_RANK)
}

pub fn chess960_position(seed: u64) -> ChessPosition {
    let back_rank = chess960_back_rank_from_seed(seed);
    position_from_back_rank(ChessVariant::Chess960, back_rank)
}

pub fn atomic_position() -> ChessPosition {
    position_from_back_rank(ChessVariant::Atomic, STANDARD_BACK_RANK)
}

pub fn chess960_back_rank_from_seed(seed: u64) -> [ChessPieceKind; 8] {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut rank = [ChessPieceKind::Pawn; 8];
    let mut remaining: Vec<usize> = (0..8).collect();

    let light_square_files = [0_usize, 2, 4, 6];
    let dark_square_files = [1_usize, 3, 5, 7];

    let bishop_light = *light_square_files
        .choose(&mut rng)
        .expect("light-square bishop index");
    let bishop_dark = *dark_square_files
        .choose(&mut rng)
        .expect("dark-square bishop index");
    rank[bishop_light] = ChessPieceKind::Bishop;
    rank[bishop_dark] = ChessPieceKind::Bishop;
    remaining.retain(|&idx| idx != bishop_light && idx != bishop_dark);

    let queen_file = *remaining.choose(&mut rng).expect("queen index");
    rank[queen_file] = ChessPieceKind::Queen;
    remaining.retain(|&idx| idx != queen_file);

    remaining.shuffle(&mut rng);
    let knight_a = remaining.pop().expect("first knight index");
    let knight_b = remaining.pop().expect("second knight index");
    rank[knight_a] = ChessPieceKind::Knight;
    rank[knight_b] = ChessPieceKind::Knight;

    remaining.sort_unstable();
    let rook_left = remaining[0];
    let king = remaining[1];
    let rook_right = remaining[2];
    rank[rook_left] = ChessPieceKind::Rook;
    rank[king] = ChessPieceKind::King;
    rank[rook_right] = ChessPieceKind::Rook;

    debug_assert!(is_valid_chess960_back_rank(&rank));
    rank
}

pub fn is_valid_chess960_back_rank(back_rank: &[ChessPieceKind; 8]) -> bool {
    let mut king_idx = None;
    let mut rook_indices = Vec::new();
    let mut bishop_indices = Vec::new();
    let mut queens = 0_usize;
    let mut knights = 0_usize;
    let mut kings = 0_usize;
    let mut rooks = 0_usize;
    let mut bishops = 0_usize;

    for (idx, piece) in back_rank.iter().copied().enumerate() {
        match piece {
            ChessPieceKind::King => {
                kings += 1;
                king_idx = Some(idx);
            }
            ChessPieceKind::Queen => queens += 1,
            ChessPieceKind::Rook => {
                rooks += 1;
                rook_indices.push(idx);
            }
            ChessPieceKind::Bishop => {
                bishops += 1;
                bishop_indices.push(idx);
            }
            ChessPieceKind::Knight => knights += 1,
            ChessPieceKind::Pawn => return false,
        }
    }

    if kings != 1 || queens != 1 || rooks != 2 || bishops != 2 || knights != 2 {
        return false;
    }

    let Some(king_idx) = king_idx else {
        return false;
    };
    rook_indices.sort_unstable();
    if !(rook_indices[0] < king_idx && king_idx < rook_indices[1]) {
        return false;
    }

    bishop_indices.sort_unstable();
    (bishop_indices[0] + bishop_indices[1]) % 2 == 1
}

fn position_from_back_rank(variant: ChessVariant, back_rank: [ChessPieceKind; 8]) -> ChessPosition {
    let mut position = ChessPosition::empty(variant);
    position.clear_board();
    position.set_side_to_move(ChessColor::White);
    position.set_castling_rights(CastlingRights::all());
    position.set_en_passant(None);
    position.set_halfmove_clock(0);
    position.set_fullmove_number(1);
    position.set_back_ranks(back_rank, back_rank);

    for file in 0..8_u8 {
        let white_piece = ChessPiece {
            color: ChessColor::White,
            kind: back_rank[file as usize],
        };
        let black_piece = ChessPiece {
            color: ChessColor::Black,
            kind: back_rank[file as usize],
        };
        let white_back = square(file, 0).expect("valid white back-rank square");
        let white_pawn = square(file, 1).expect("valid white pawn square");
        let black_pawn = square(file, 6).expect("valid black pawn square");
        let black_back = square(file, 7).expect("valid black back-rank square");

        let _ = position.set_piece(white_back, Some(white_piece));
        let _ = position.set_piece(
            white_pawn,
            Some(ChessPiece {
                color: ChessColor::White,
                kind: ChessPieceKind::Pawn,
            }),
        );
        let _ = position.set_piece(
            black_pawn,
            Some(ChessPiece {
                color: ChessColor::Black,
                kind: ChessPieceKind::Pawn,
            }),
        );
        let _ = position.set_piece(black_back, Some(black_piece));
    }

    position
}

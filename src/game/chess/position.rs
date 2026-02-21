use super::setup::STANDARD_BACK_RANK;
use super::types::{ChessColor, ChessPiece, ChessPieceKind, ChessVariant, Square, BOARD_SQUARES};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct CastlingRights {
    pub white_king_side: bool,
    pub white_queen_side: bool,
    pub black_king_side: bool,
    pub black_queen_side: bool,
}

impl CastlingRights {
    pub const fn none() -> Self {
        Self {
            white_king_side: false,
            white_queen_side: false,
            black_king_side: false,
            black_queen_side: false,
        }
    }

    pub const fn all() -> Self {
        Self {
            white_king_side: true,
            white_queen_side: true,
            black_king_side: true,
            black_queen_side: true,
        }
    }

    pub fn has_any(self) -> bool {
        self.white_king_side
            || self.white_queen_side
            || self.black_king_side
            || self.black_queen_side
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ChessPosition {
    variant: ChessVariant,
    board: [Option<ChessPiece>; BOARD_SQUARES],
    side_to_move: ChessColor,
    castling_rights: CastlingRights,
    en_passant: Option<Square>,
    halfmove_clock: u16,
    fullmove_number: u16,
    white_back_rank: [ChessPieceKind; 8],
    black_back_rank: [ChessPieceKind; 8],
}

impl ChessPosition {
    pub fn empty(variant: ChessVariant) -> Self {
        Self {
            variant,
            board: [None; BOARD_SQUARES],
            side_to_move: ChessColor::White,
            castling_rights: CastlingRights::none(),
            en_passant: None,
            halfmove_clock: 0,
            fullmove_number: 1,
            white_back_rank: STANDARD_BACK_RANK,
            black_back_rank: STANDARD_BACK_RANK,
        }
    }

    pub fn variant(&self) -> ChessVariant {
        self.variant
    }

    pub fn board(&self) -> &[Option<ChessPiece>; BOARD_SQUARES] {
        &self.board
    }

    pub fn piece_at(&self, square: Square) -> Option<ChessPiece> {
        self.board.get(square as usize).copied().flatten()
    }

    pub fn set_piece(&mut self, square: Square, piece: Option<ChessPiece>) -> bool {
        if let Some(slot) = self.board.get_mut(square as usize) {
            *slot = piece;
            true
        } else {
            false
        }
    }

    pub fn clear_board(&mut self) {
        self.board = [None; BOARD_SQUARES];
    }

    pub fn side_to_move(&self) -> ChessColor {
        self.side_to_move
    }

    pub fn set_side_to_move(&mut self, side_to_move: ChessColor) {
        self.side_to_move = side_to_move;
    }

    pub fn castling_rights(&self) -> CastlingRights {
        self.castling_rights
    }

    pub fn set_castling_rights(&mut self, castling_rights: CastlingRights) {
        self.castling_rights = castling_rights;
    }

    pub fn en_passant(&self) -> Option<Square> {
        self.en_passant
    }

    pub fn set_en_passant(&mut self, en_passant: Option<Square>) {
        self.en_passant = en_passant;
    }

    pub fn halfmove_clock(&self) -> u16 {
        self.halfmove_clock
    }

    pub fn set_halfmove_clock(&mut self, halfmove_clock: u16) {
        self.halfmove_clock = halfmove_clock;
    }

    pub fn fullmove_number(&self) -> u16 {
        self.fullmove_number
    }

    pub fn set_fullmove_number(&mut self, fullmove_number: u16) {
        self.fullmove_number = fullmove_number.max(1);
    }

    pub fn back_rank(&self, color: ChessColor) -> &[ChessPieceKind; 8] {
        match color {
            ChessColor::White => &self.white_back_rank,
            ChessColor::Black => &self.black_back_rank,
        }
    }

    pub fn set_back_ranks(
        &mut self,
        white_back_rank: [ChessPieceKind; 8],
        black_back_rank: [ChessPieceKind; 8],
    ) {
        self.white_back_rank = white_back_rank;
        self.black_back_rank = black_back_rank;
    }

    pub fn piece_count(&self, color: ChessColor) -> usize {
        self.board
            .iter()
            .flatten()
            .filter(|piece| piece.color == color)
            .count()
    }
}

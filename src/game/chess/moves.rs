use super::types::{ChessPieceKind, Square};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChessMove {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<ChessPieceKind>,
    pub is_castle_kingside: bool,
    pub is_castle_queenside: bool,
    pub is_en_passant: bool,
}

impl ChessMove {
    pub fn new(from: Square, to: Square) -> Self {
        Self {
            from,
            to,
            promotion: None,
            is_castle_kingside: false,
            is_castle_queenside: false,
            is_en_passant: false,
        }
    }

    pub fn with_promotion(mut self, promotion: ChessPieceKind) -> Self {
        self.promotion = Some(promotion);
        self
    }

    pub fn as_kingside_castle(mut self) -> Self {
        self.is_castle_kingside = true;
        self
    }

    pub fn as_queenside_castle(mut self) -> Self {
        self.is_castle_queenside = true;
        self
    }

    pub fn as_en_passant(mut self) -> Self {
        self.is_en_passant = true;
        self
    }
}

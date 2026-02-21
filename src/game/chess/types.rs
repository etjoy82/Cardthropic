#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChessVariant {
    Standard,
    Chess960,
    Atomic,
}

impl ChessVariant {
    pub fn id(self) -> &'static str {
        match self {
            Self::Standard => "chess-standard",
            Self::Chess960 => "chess-960",
            Self::Atomic => "chess-atomic",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard Chess",
            Self::Chess960 => "Chess960",
            Self::Atomic => "Atomic Chess",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChessColor {
    White,
    Black,
}

impl ChessColor {
    pub fn opposite(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }

    pub fn fen_char(self) -> char {
        match self {
            Self::White => 'w',
            Self::Black => 'b',
        }
    }

    pub fn from_fen_char(value: char) -> Option<Self> {
        match value {
            'w' => Some(Self::White),
            'b' => Some(Self::Black),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChessPieceKind {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

impl ChessPieceKind {
    fn white_fen_char(self) -> char {
        match self {
            Self::King => 'K',
            Self::Queen => 'Q',
            Self::Rook => 'R',
            Self::Bishop => 'B',
            Self::Knight => 'N',
            Self::Pawn => 'P',
        }
    }

    pub fn fen_char(self, color: ChessColor) -> char {
        let white = self.white_fen_char();
        match color {
            ChessColor::White => white,
            ChessColor::Black => white.to_ascii_lowercase(),
        }
    }

    pub fn from_fen_char(value: char) -> Option<(Self, ChessColor)> {
        let color = if value.is_ascii_uppercase() {
            ChessColor::White
        } else {
            ChessColor::Black
        };
        let kind = match value.to_ascii_uppercase() {
            'K' => Self::King,
            'Q' => Self::Queen,
            'R' => Self::Rook,
            'B' => Self::Bishop,
            'N' => Self::Knight,
            'P' => Self::Pawn,
            _ => return None,
        };
        Some((kind, color))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChessPiece {
    pub color: ChessColor,
    pub kind: ChessPieceKind,
}

pub type Square = u8;

pub const BOARD_SQUARES: usize = 64;

pub fn square(file: u8, rank: u8) -> Option<Square> {
    if file < 8 && rank < 8 {
        Some((rank * 8) + file)
    } else {
        None
    }
}

pub fn file_of(square: Square) -> u8 {
    square % 8
}

pub fn rank_of(square: Square) -> u8 {
    square / 8
}

pub fn square_name(square: Square) -> String {
    let file = (b'a' + file_of(square)) as char;
    let rank = (b'1' + rank_of(square)) as char;
    format!("{file}{rank}")
}

pub fn parse_square(value: &str) -> Option<Square> {
    if value.len() != 2 {
        return None;
    }
    let mut chars = value.chars();
    let file_char = chars.next()?.to_ascii_lowercase();
    let rank_char = chars.next()?;
    if !('a'..='h').contains(&file_char) || !('1'..='8').contains(&rank_char) {
        return None;
    }
    let file = (file_char as u8).saturating_sub(b'a');
    let rank = (rank_char as u8).saturating_sub(b'1');
    square(file, rank)
}

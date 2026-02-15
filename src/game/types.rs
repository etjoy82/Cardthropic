use super::rank_label;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GameMode {
    Klondike,
    Spider,
    Freecell,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DrawMode {
    One,
    Two,
    Three,
    Four,
    Five,
}

impl DrawMode {
    pub fn count(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Three => 3,
            Self::Four => 4,
            Self::Five => 5,
        }
    }

    pub fn from_count(count: u8) -> Option<Self> {
        match count {
            1 => Some(Self::One),
            2 => Some(Self::Two),
            3 => Some(Self::Three),
            4 => Some(Self::Four),
            5 => Some(Self::Five),
            _ => None,
        }
    }
}

impl GameMode {
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "klondike" => Some(Self::Klondike),
            "spider" => Some(Self::Spider),
            "freecell" => Some(Self::Freecell),
            _ => None,
        }
    }

    pub fn id(self) -> &'static str {
        match self {
            Self::Klondike => "klondike",
            Self::Spider => "spider",
            Self::Freecell => "freecell",
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Klondike => "Klondike",
            Self::Spider => "Spider",
            Self::Freecell => "FreeCell",
        }
    }

    pub fn emoji(self) -> &'static str {
        match self {
            Self::Klondike => "🥇",
            Self::Spider => "🕷️",
            Self::Freecell => "🗽",
        }
    }

    pub fn engine_ready(self) -> bool {
        matches!(self, Self::Klondike | Self::Spider)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Clubs,
    Diamonds,
    Hearts,
    Spades,
}

impl Suit {
    pub const ALL: [Suit; 4] = [Suit::Clubs, Suit::Diamonds, Suit::Hearts, Suit::Spades];

    pub fn is_red(self) -> bool {
        matches!(self, Suit::Diamonds | Suit::Hearts)
    }

    pub fn short(self) -> &'static str {
        match self {
            Suit::Clubs => "C",
            Suit::Diamonds => "D",
            Suit::Hearts => "H",
            Suit::Spades => "S",
        }
    }

    pub fn foundation_index(self) -> usize {
        match self {
            Suit::Clubs => 0,
            Suit::Diamonds => 1,
            Suit::Hearts => 2,
            Suit::Spades => 3,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Card {
    pub suit: Suit,
    pub rank: u8,
    pub face_up: bool,
}

impl Card {
    pub fn label(&self) -> String {
        format!("{}{}", rank_label(self.rank), self.suit.short())
    }

    pub fn color_red(&self) -> bool {
        self.suit.is_red()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct KlondikeGame {
    pub(super) draw_mode: DrawMode,
    pub(super) stock: Vec<Card>,
    pub(super) waste: Vec<Card>,
    pub(super) foundations: [Vec<Card>; 4],
    pub(super) tableau: [Vec<Card>; 7],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DrawResult {
    DrewFromStock,
    RecycledWaste,
    NoOp,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WinnabilityResult {
    pub winnable: bool,
    pub explored_states: usize,
    pub generated_states: usize,
    pub win_depth: Option<u32>,
    pub hit_state_limit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GuidedWinnabilityResult {
    pub winnable: bool,
    pub explored_states: usize,
    pub generated_states: usize,
    pub win_depth: Option<u32>,
    pub hit_state_limit: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverMove {
    Draw,
    WasteToFoundation,
    WasteToTableau {
        dst: usize,
    },
    TableauTopToFoundation {
        src: usize,
    },
    TableauRunToTableau {
        src: usize,
        start: usize,
        dst: usize,
    },
}

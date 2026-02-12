#![allow(dead_code)]

use crate::game::{GameMode, KlondikeGame};

/// Runtime container for the active solitaire variant.
/// Klondike is fully implemented; other variants are explicit placeholders.
#[derive(Debug, Clone)]
pub enum VariantRuntime {
    Klondike(KlondikeGame),
    Spider,
    Freecell,
}

impl VariantRuntime {
    pub fn new(mode: GameMode, seed: u64) -> Self {
        match mode {
            GameMode::Klondike => Self::Klondike(KlondikeGame::new_with_seed(seed)),
            GameMode::Spider => Self::Spider,
            GameMode::Freecell => Self::Freecell,
        }
    }

    pub fn mode(&self) -> GameMode {
        match self {
            Self::Klondike(_) => GameMode::Klondike,
            Self::Spider => GameMode::Spider,
            Self::Freecell => GameMode::Freecell,
        }
    }

    pub fn supports_solver(&self) -> bool {
        matches!(self, Self::Klondike(_))
    }

    pub fn as_klondike(&self) -> Option<&KlondikeGame> {
        match self {
            Self::Klondike(game) => Some(game),
            Self::Spider | Self::Freecell => None,
        }
    }

    pub fn as_klondike_mut(&mut self) -> Option<&mut KlondikeGame> {
        match self {
            Self::Klondike(game) => Some(game),
            Self::Spider | Self::Freecell => None,
        }
    }
}

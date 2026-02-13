#![allow(dead_code)]

use std::ops::{Deref, DerefMut};

use crate::game::{GameMode, KlondikeGame, SpiderGame};

/// Runtime container for the active solitaire variant.
/// Klondike is fully implemented; other variants are explicit placeholders.
#[derive(Debug, Clone)]
pub enum VariantRuntime {
    Klondike(KlondikeGame),
    Spider(SpiderGame),
    Freecell,
}

impl VariantRuntime {
    pub fn new(mode: GameMode, seed: u64) -> Self {
        match mode {
            GameMode::Klondike => Self::Klondike(KlondikeGame::new_with_seed(seed)),
            GameMode::Spider => Self::Spider(SpiderGame::new_with_seed(seed)),
            GameMode::Freecell => Self::Freecell,
        }
    }

    pub fn mode(&self) -> GameMode {
        match self {
            Self::Klondike(_) => GameMode::Klondike,
            Self::Spider(_) => GameMode::Spider,
            Self::Freecell => GameMode::Freecell,
        }
    }

    pub fn supports_solver(&self) -> bool {
        matches!(self, Self::Klondike(_))
    }

    pub fn as_klondike(&self) -> Option<&KlondikeGame> {
        match self {
            Self::Klondike(game) => Some(game),
            Self::Spider(_) | Self::Freecell => None,
        }
    }

    pub fn as_klondike_mut(&mut self) -> Option<&mut KlondikeGame> {
        match self {
            Self::Klondike(game) => Some(game),
            Self::Spider(_) | Self::Freecell => None,
        }
    }

    pub fn into_klondike(self) -> Option<KlondikeGame> {
        match self {
            Self::Klondike(game) => Some(game),
            Self::Spider(_) | Self::Freecell => None,
        }
    }
}

impl Deref for VariantRuntime {
    type Target = KlondikeGame;

    fn deref(&self) -> &Self::Target {
        self.as_klondike()
            .expect("variant runtime expected Klondike state")
    }
}

impl DerefMut for VariantRuntime {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_klondike_mut()
            .expect("variant runtime expected Klondike state")
    }
}

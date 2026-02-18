use std::collections::HashMap;
use std::ops::{Deref, DerefMut};

use crate::engine::game_mode::VariantRuntime;
use crate::game::{FreecellGame, GameMode, KlondikeGame, SpiderGame};

#[derive(Debug, Clone)]
pub struct VariantStateStore {
    klondike: KlondikeGame,
    spider: SpiderGame,
    freecell: FreecellGame,
    parked: HashMap<GameMode, VariantRuntime>,
}

impl VariantStateStore {
    pub fn new(seed: u64) -> Self {
        Self {
            klondike: KlondikeGame::new_with_seed(seed),
            spider: SpiderGame::new_with_seed(seed),
            freecell: FreecellGame::new_with_seed(seed),
            parked: HashMap::new(),
        }
    }

    pub fn set_klondike(&mut self, game: KlondikeGame) {
        self.klondike = game;
    }

    pub fn klondike(&self) -> &KlondikeGame {
        &self.klondike
    }

    pub fn klondike_mut(&mut self) -> &mut KlondikeGame {
        &mut self.klondike
    }

    pub fn set_spider(&mut self, game: SpiderGame) {
        self.spider = game;
    }

    pub fn spider(&self) -> &SpiderGame {
        &self.spider
    }

    pub fn spider_mut(&mut self) -> &mut SpiderGame {
        &mut self.spider
    }

    pub fn set_freecell(&mut self, game: FreecellGame) {
        self.freecell = game;
    }

    pub fn freecell(&self) -> &FreecellGame {
        &self.freecell
    }

    pub fn freecell_mut(&mut self) -> &mut FreecellGame {
        &mut self.freecell
    }

    pub fn set_runtime(&mut self, runtime: VariantRuntime) {
        match runtime {
            VariantRuntime::Klondike(game) => self.klondike = game,
            VariantRuntime::Spider(game) => self.spider = game,
            VariantRuntime::Freecell(game) => self.freecell = game,
        }
    }

    pub fn runtime_for_mode(&self, mode: GameMode) -> VariantRuntime {
        match mode {
            GameMode::Klondike => VariantRuntime::Klondike(self.klondike.clone()),
            GameMode::Spider => VariantRuntime::Spider(self.spider.clone()),
            GameMode::Freecell => VariantRuntime::Freecell(self.freecell.clone()),
        }
    }

    pub fn park_runtime(&mut self, mode: GameMode, runtime: VariantRuntime) {
        self.parked.insert(mode, runtime);
    }

    pub fn parked_runtime(&self, mode: GameMode) -> Option<&VariantRuntime> {
        self.parked.get(&mode)
    }

    pub fn encode_runtime_for_session(&self, mode: GameMode) -> String {
        match mode {
            GameMode::Klondike => format!("k:{}", self.klondike.encode_for_session()),
            GameMode::Spider => format!("s:{}", self.spider.encode_for_session()),
            GameMode::Freecell => format!("f:{}", self.freecell.encode_for_session()),
        }
    }

    pub fn decode_runtime_for_session(mode: GameMode, encoded: &str) -> Option<VariantRuntime> {
        if let Some(rest) = encoded.strip_prefix("k:") {
            return KlondikeGame::decode_from_session(rest).map(VariantRuntime::Klondike);
        }
        if let Some(rest) = encoded.strip_prefix("s:") {
            return SpiderGame::decode_from_session(rest).map(VariantRuntime::Spider);
        }
        if let Some(rest) = encoded.strip_prefix("f:") {
            if rest.is_empty() {
                return Some(VariantRuntime::Freecell(FreecellGame::new_with_seed(0)));
            }
            return FreecellGame::decode_from_session(rest).map(VariantRuntime::Freecell);
        }
        if encoded == "f:" || encoded == "f" {
            return Some(VariantRuntime::Freecell(FreecellGame::new_with_seed(0)));
        }

        // Backward-compat fallback when no explicit runtime prefix exists.
        match mode {
            GameMode::Klondike => {
                KlondikeGame::decode_from_session(encoded).map(VariantRuntime::Klondike)
            }
            GameMode::Spider => {
                SpiderGame::decode_from_session(encoded).map(VariantRuntime::Spider)
            }
            GameMode::Freecell => {
                FreecellGame::decode_from_session(encoded).map(VariantRuntime::Freecell)
            }
        }
    }
}

impl Deref for VariantStateStore {
    type Target = KlondikeGame;

    fn deref(&self) -> &Self::Target {
        self.klondike()
    }
}

impl DerefMut for VariantStateStore {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.klondike_mut()
    }
}

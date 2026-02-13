use crate::game::{DrawMode, GameMode, KlondikeGame};

#[derive(Debug, Clone)]
pub struct GameViewModel {
    mode: GameMode,
    engine_ready: bool,
    klondike: KlondikeGame,
}

impl GameViewModel {
    pub fn new(
        mode: GameMode,
        engine_ready: bool,
        mut klondike: KlondikeGame,
        draw_mode: DrawMode,
    ) -> Self {
        if klondike.draw_mode() != draw_mode {
            klondike.set_draw_mode(draw_mode);
        }
        Self {
            mode,
            engine_ready,
            klondike,
        }
    }

    pub fn mode(&self) -> GameMode {
        self.mode
    }

    pub fn engine_ready(&self) -> bool {
        self.engine_ready
    }

    pub fn klondike(&self) -> &KlondikeGame {
        &self.klondike
    }
}

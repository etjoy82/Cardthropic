use crate::game::{DrawMode, DrawResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineCommand {
    DrawOrRecycle {
        draw_mode: DrawMode,
    },
    CycloneShuffleTableau,
    MoveWasteToFoundation,
    MoveWasteToTableau {
        dst: usize,
    },
    MoveTableauRunToTableau {
        src: usize,
        start: usize,
        dst: usize,
    },
    MoveTableauTopToFoundation {
        src: usize,
    },
    MoveFoundationTopToTableau {
        foundation_idx: usize,
        dst: usize,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EngineCommandResult {
    pub changed: bool,
    pub draw_result: Option<DrawResult>,
}

impl EngineCommandResult {
    pub const fn unchanged() -> Self {
        Self {
            changed: false,
            draw_result: None,
        }
    }

    pub const fn changed() -> Self {
        Self {
            changed: true,
            draw_result: None,
        }
    }

    pub const fn from_draw(result: DrawResult) -> Self {
        Self {
            changed: matches!(
                result,
                DrawResult::DrewFromStock | DrawResult::RecycledWaste
            ),
            draw_result: Some(result),
        }
    }
}

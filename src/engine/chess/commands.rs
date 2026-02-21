use crate::game::{ChessMove, ChessVariant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChessCommand {
    NewGame { seed: u64, variant: ChessVariant },
    TryMove(ChessMove),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChessStatus {
    Ready,
    IllegalMove,
    Unsupported,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChessCommandResult {
    pub changed: bool,
    pub status: ChessStatus,
}

impl ChessCommandResult {
    pub const fn changed(status: ChessStatus) -> Self {
        Self {
            changed: true,
            status,
        }
    }

    pub const fn unchanged(status: ChessStatus) -> Self {
        Self {
            changed: false,
            status,
        }
    }
}

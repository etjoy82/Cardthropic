#[path = "game/klondike_moves.rs"]
mod klondike_moves;
#[path = "game/session_codec.rs"]
mod session_codec;
#[path = "game/setup.rs"]
mod setup;
#[path = "game/solver.rs"]
mod solver;
#[path = "game/types.rs"]
mod types;
pub use types::*;

pub fn rank_label(rank: u8) -> &'static str {
    match rank {
        1 => "A",
        2 => "2",
        3 => "3",
        4 => "4",
        5 => "5",
        6 => "6",
        7 => "7",
        8 => "8",
        9 => "9",
        10 => "10",
        11 => "J",
        12 => "Q",
        13 => "K",
        _ => "?",
    }
}

#[cfg(test)]
mod tests;

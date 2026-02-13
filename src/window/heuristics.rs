use super::*;
use std::collections::HashSet;

pub(super) fn hash_game_state(game: &KlondikeGame) -> u64 {
    crate::engine::autoplay::hash_game_state(game)
}

pub(super) fn score_hint_candidate(
    current: &KlondikeGame,
    next: &KlondikeGame,
    hint_move: HintMove,
    recent_hashes: &HashSet<u64>,
    next_hash: u64,
) -> i64 {
    crate::engine::autoplay::score_hint_candidate(
        current,
        next,
        hint_move,
        recent_hashes,
        next_hash,
    )
}

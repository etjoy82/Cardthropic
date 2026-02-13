use std::collections::hash_map::DefaultHasher;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};

use crate::engine::automation::AutomationProfile;
use crate::engine::moves::HintMove;
use crate::game::KlondikeGame;

pub fn hash_game_state(game: &KlondikeGame) -> u64 {
    let mut hasher = DefaultHasher::new();
    game.hash(&mut hasher);
    hasher.finish()
}

pub fn foundation_count(game: &KlondikeGame) -> i64 {
    game.foundations()
        .iter()
        .map(|pile| pile.len() as i64)
        .sum()
}

pub fn hidden_tableau_count(game: &KlondikeGame) -> i64 {
    game.tableau()
        .iter()
        .flat_map(|pile| pile.iter())
        .filter(|card| !card.face_up)
        .count() as i64
}

pub fn face_up_tableau_count(game: &KlondikeGame) -> i64 {
    game.tableau()
        .iter()
        .flat_map(|pile| pile.iter())
        .filter(|card| card.face_up)
        .count() as i64
}

pub fn empty_tableau_count(game: &KlondikeGame) -> i64 {
    game.tableau().iter().filter(|pile| pile.is_empty()).count() as i64
}

pub fn score_hint_candidate(
    current: &KlondikeGame,
    next: &KlondikeGame,
    hint_move: HintMove,
    recent_hashes: &HashSet<u64>,
    next_hash: u64,
) -> i64 {
    let foundation_delta = foundation_count(next) - foundation_count(current);
    let hidden_delta = hidden_tableau_count(current) - hidden_tableau_count(next);
    let face_up_delta = face_up_tableau_count(next) - face_up_tableau_count(current);
    let empty_delta = empty_tableau_count(next) - empty_tableau_count(current);

    let mut score =
        foundation_delta * 1400 + hidden_delta * 260 + face_up_delta * 32 + empty_delta * 70;

    match hint_move {
        HintMove::WasteToFoundation | HintMove::TableauTopToFoundation { .. } => {
            score += 420;
        }
        HintMove::WasteToTableau { dst } => {
            score += 60;
            if current.tableau_len(dst) == Some(0) {
                score += 150;
            }
        }
        HintMove::TableauRunToTableau { src, start, dst } => {
            let run_len = current.tableau_len(src).unwrap_or(0).saturating_sub(start) as i64;
            if run_len > 1 {
                score += run_len * 12;
            }
            if current.tableau_len(dst) == Some(0) {
                score += 180;
            }
            if start > 0
                && current
                    .tableau_card(src, start - 1)
                    .map(|card| !card.face_up)
                    .unwrap_or(false)
            {
                score += 260;
            }
            if run_len == 1 && hidden_delta <= 0 && foundation_delta <= 0 {
                score -= 160;
            }
        }
        HintMove::Draw => {
            score -= 140;
        }
    }

    if recent_hashes.contains(&next_hash) {
        score -= 2400;
    }

    score
}

pub fn auto_play_state_heuristic(game: &KlondikeGame, profile: AutomationProfile) -> i64 {
    if game.is_won() {
        return profile.auto_play_win_score;
    }

    let foundation = foundation_count(game);
    let hidden = hidden_tableau_count(game);
    let face_up = face_up_tableau_count(game);
    let empty = empty_tableau_count(game);
    let mobility = non_draw_move_count(game);
    let stock = game.stock_len() as i64;
    let waste_has_card = game.waste_top().is_some() as i64;

    foundation * 1900 + face_up * 22 + empty * 90 + mobility * 12 - hidden * 190 - stock * 6
        + waste_has_card * 8
}

pub fn non_draw_move_count(game: &KlondikeGame) -> i64 {
    let mut count = 0_i64;

    if game.can_move_waste_to_foundation() {
        count += 1;
    }

    for src in 0..7 {
        if game.can_move_tableau_top_to_foundation(src) {
            count += 1;
        }
    }

    for dst in 0..7 {
        if game.can_move_waste_to_tableau(dst) {
            count += 1;
        }
    }

    for src in 0..7 {
        let len = game.tableau_len(src).unwrap_or(0);
        for start in 0..len {
            for dst in 0..7 {
                if game.can_move_tableau_run_to_tableau(src, start, dst) {
                    count += 1;
                }
            }
        }
    }

    count
}

pub fn restores_current_by_inverse_tableau_move(
    current: &KlondikeGame,
    next: &KlondikeGame,
    src: usize,
    dst: usize,
    run_len: usize,
) -> bool {
    if run_len == 0 {
        return false;
    }
    let dst_len = next.tableau_len(dst).unwrap_or(0);
    if dst_len < run_len {
        return false;
    }

    let inverse_start = dst_len - run_len;
    if !next.can_move_tableau_run_to_tableau(dst, inverse_start, src) {
        return false;
    }

    let mut reverse = next.clone();
    if !reverse.move_tableau_run_to_tableau(dst, inverse_start, src) {
        return false;
    }

    &reverse == current
}

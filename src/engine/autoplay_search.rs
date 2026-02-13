use std::cmp::Reverse;
use std::collections::HashSet;

use crate::engine::automation::AutomationProfile;
use crate::engine::autoplay;
use crate::engine::moves::{apply_hint_move_to_game, enumerate_hint_moves, HintMove};
use crate::game::KlondikeGame;

pub fn best_candidate_slot_index<F>(
    game: &KlondikeGame,
    candidate_slots: &[Option<HintMove>],
    seen_states: &HashSet<u64>,
    state_hash: u64,
    profile: AutomationProfile,
    mut include_slot: F,
) -> Option<usize>
where
    F: FnMut(usize, HintMove) -> bool,
{
    let mut best: Option<(i64, usize)> = None;
    let mut node_budget = profile.auto_play_node_budget;

    for (index, slot_move) in candidate_slots.iter().copied().enumerate() {
        let Some(hint_move) = slot_move else {
            continue;
        };
        if !include_slot(index, hint_move) {
            continue;
        }

        let mut next_state = game.clone();
        if !apply_hint_move_to_game(&mut next_state, hint_move) {
            continue;
        }

        let next_hash = autoplay::hash_game_state(&next_state);
        if seen_states.contains(&next_hash) {
            continue;
        }
        if is_functionally_useless_auto_move(game, &next_state, hint_move, seen_states) {
            continue;
        }

        let mut score =
            autoplay::score_hint_candidate(game, &next_state, hint_move, seen_states, next_hash);
        score += unseen_followup_count(&next_state, seen_states) * 35;
        if next_state.is_won() {
            score += profile.auto_play_win_score;
        }

        let mut path_seen = HashSet::new();
        path_seen.insert(state_hash);
        path_seen.insert(next_hash);
        let lookahead = auto_play_lookahead_score(
            &next_state,
            seen_states,
            &mut path_seen,
            profile.auto_play_lookahead_depth.saturating_sub(1),
            &mut node_budget,
            profile,
        );
        score += lookahead / 3;

        if is_king_to_empty_without_unlock(game, hint_move) {
            score -= 4_000;
        }

        match best {
            None => best = Some((score, index)),
            Some((best_score, _)) if score > best_score => best = Some((score, index)),
            _ => {}
        }
    }

    best.map(|(_, index)| index)
}

pub fn unseen_followup_count(game: &KlondikeGame, seen_states: &HashSet<u64>) -> i64 {
    let mut followups: HashSet<u64> = HashSet::new();
    for hint_move in enumerate_hint_moves(game) {
        let mut next_state = game.clone();
        if !apply_hint_move_to_game(&mut next_state, hint_move) {
            continue;
        }
        let next_hash = autoplay::hash_game_state(&next_state);
        if !seen_states.contains(&next_hash) {
            followups.insert(next_hash);
        }
    }
    followups.len() as i64
}

pub fn auto_play_lookahead_score(
    current: &KlondikeGame,
    persistent_seen: &HashSet<u64>,
    path_seen: &mut HashSet<u64>,
    depth: u8,
    budget: &mut usize,
    profile: AutomationProfile,
) -> i64 {
    if current.is_won() {
        return profile.auto_play_win_score;
    }
    if depth == 0 || *budget == 0 {
        return autoplay::auto_play_state_heuristic(current, profile);
    }

    let mut scored_children: Vec<(i64, KlondikeGame, u64)> = Vec::new();
    for hint_move in enumerate_hint_moves(current) {
        let mut next = current.clone();
        if !apply_hint_move_to_game(&mut next, hint_move) {
            continue;
        }
        let next_hash = autoplay::hash_game_state(&next);
        if persistent_seen.contains(&next_hash) || path_seen.contains(&next_hash) {
            continue;
        }
        if is_obviously_useless_auto_move(current, &next, hint_move) {
            continue;
        }

        let immediate =
            autoplay::score_hint_candidate(current, &next, hint_move, persistent_seen, next_hash);
        scored_children.push((immediate, next, next_hash));
    }

    if scored_children.is_empty() {
        return -90_000 + autoplay::auto_play_state_heuristic(current, profile);
    }

    scored_children.sort_by_key(|(score, _, _)| Reverse(*score));
    let mut best = i64::MIN / 4;
    for (immediate, next, next_hash) in scored_children
        .into_iter()
        .take(profile.auto_play_beam_width)
    {
        if *budget == 0 {
            break;
        }
        *budget -= 1;
        path_seen.insert(next_hash);
        let future = auto_play_lookahead_score(
            &next,
            persistent_seen,
            path_seen,
            depth - 1,
            budget,
            profile,
        );
        path_seen.remove(&next_hash);

        let total = immediate + (future / 2);
        if total > best {
            best = total;
        }
    }

    if best == i64::MIN / 4 {
        autoplay::auto_play_state_heuristic(current, profile)
    } else {
        best
    }
}

pub fn is_obviously_useless_auto_move(
    current: &KlondikeGame,
    next: &KlondikeGame,
    hint_move: HintMove,
) -> bool {
    let foundation_delta = autoplay::foundation_count(next) - autoplay::foundation_count(current);
    let hidden_delta =
        autoplay::hidden_tableau_count(current) - autoplay::hidden_tableau_count(next);
    let empty_delta = autoplay::empty_tableau_count(next) - autoplay::empty_tableau_count(current);
    let mobility_delta =
        autoplay::non_draw_move_count(next) - autoplay::non_draw_move_count(current);
    if foundation_delta > 0 || hidden_delta > 0 || empty_delta > 0 {
        return false;
    }

    match hint_move {
        HintMove::WasteToFoundation | HintMove::TableauTopToFoundation { .. } => false,
        HintMove::WasteToTableau { .. } => current.can_move_waste_to_foundation(),
        HintMove::TableauRunToTableau { src, start, dst } => {
            let run_len = current.tableau_len(src).unwrap_or(0).saturating_sub(start);
            if run_len == 0 {
                return true;
            }
            if is_king_to_empty_without_unlock(current, hint_move) {
                return true;
            }
            let reversible = autoplay::restores_current_by_inverse_tableau_move(
                current, next, src, dst, run_len,
            );
            reversible && mobility_delta <= 0
        }
        HintMove::Draw => {
            if current.draw_mode().count() > 1 {
                false
            } else {
                autoplay::non_draw_move_count(current) > 0
            }
        }
    }
}

pub fn is_king_to_empty_without_unlock(current: &KlondikeGame, hint_move: HintMove) -> bool {
    let HintMove::TableauRunToTableau { src, start, dst } = hint_move else {
        return false;
    };
    if current.tableau_len(dst) != Some(0) {
        return false;
    }
    let Some(card) = current.tableau_card(src, start) else {
        return false;
    };
    if card.rank != 13 {
        return false;
    }
    let reveals_hidden = start > 0
        && current
            .tableau_card(src, start - 1)
            .map(|below| !below.face_up)
            .unwrap_or(false);
    !reveals_hidden
}

pub fn is_functionally_useless_auto_move(
    current: &KlondikeGame,
    next: &KlondikeGame,
    hint_move: HintMove,
    seen_states: &HashSet<u64>,
) -> bool {
    if next.is_won() {
        return false;
    }

    let foundation_delta = autoplay::foundation_count(next) - autoplay::foundation_count(current);
    let hidden_delta =
        autoplay::hidden_tableau_count(current) - autoplay::hidden_tableau_count(next);
    let empty_delta = autoplay::empty_tableau_count(next) - autoplay::empty_tableau_count(current);
    if foundation_delta > 0 || hidden_delta > 0 || empty_delta > 0 {
        return false;
    }

    let mobility_delta =
        autoplay::non_draw_move_count(next) - autoplay::non_draw_move_count(current);
    let unseen_followups = unseen_followup_count(next, seen_states);

    match hint_move {
        HintMove::WasteToFoundation | HintMove::TableauTopToFoundation { .. } => false,
        HintMove::WasteToTableau { .. } => {
            if current.can_move_waste_to_foundation() {
                return true;
            }
            mobility_delta <= 0 && unseen_followups <= 0
        }
        HintMove::TableauRunToTableau { src, start, dst } => {
            let run_len = current.tableau_len(src).unwrap_or(0).saturating_sub(start);
            if run_len == 0 {
                return true;
            }

            if is_king_to_empty_without_unlock(current, hint_move) && mobility_delta <= 0 {
                return true;
            }

            let reversible = autoplay::restores_current_by_inverse_tableau_move(
                current, next, src, dst, run_len,
            );

            if reversible && mobility_delta <= 0 {
                return true;
            }

            run_len == 1 && mobility_delta <= 0 && unseen_followups <= 1
        }
        HintMove::Draw => {
            let drew_playable = next.can_move_waste_to_foundation()
                || (0..7).any(|dst| next.can_move_waste_to_tableau(dst));
            if drew_playable {
                return false;
            }
            if current.draw_mode().count() > 1 {
                return unseen_followups <= 0
                    && autoplay::hash_game_state(current) == autoplay::hash_game_state(next);
            }
            let current_non_draw_moves = autoplay::non_draw_move_count(current);
            current_non_draw_moves > 0 && unseen_followups <= 0
        }
    }
}

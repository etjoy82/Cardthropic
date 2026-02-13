use crate::engine::boundary;
use crate::engine::moves::HintMove;
use crate::engine::variant_state::VariantStateStore;
use crate::game::GameMode;

pub fn direct_tableau_to_foundation_move(
    state: &VariantStateStore,
    mode: GameMode,
    col: usize,
    start: usize,
) -> Option<HintMove> {
    let top = boundary::tableau_len(state, mode, col).and_then(|len| len.checked_sub(1));
    if top == Some(start) && boundary::can_move_tableau_top_to_foundation(state, mode, col) {
        Some(HintMove::TableauTopToFoundation { src: col })
    } else {
        None
    }
}

pub fn direct_waste_to_foundation_move(
    state: &VariantStateStore,
    mode: GameMode,
) -> Option<HintMove> {
    if boundary::can_move_waste_to_foundation(state, mode) {
        Some(HintMove::WasteToFoundation)
    } else {
        None
    }
}

pub fn fallback_tableau_run_move(
    state: &VariantStateStore,
    mode: GameMode,
    col: usize,
    start: usize,
) -> Option<HintMove> {
    (0..7)
        .find(|&dst| boundary::can_move_tableau_run_to_tableau(state, mode, col, start, dst))
        .map(|dst| HintMove::TableauRunToTableau {
            src: col,
            start,
            dst,
        })
}

pub fn fallback_waste_to_tableau_move(
    state: &VariantStateStore,
    mode: GameMode,
) -> Option<HintMove> {
    (0..7)
        .find(|&dst| boundary::can_move_waste_to_tableau(state, mode, dst))
        .map(|dst| HintMove::WasteToTableau { dst })
}

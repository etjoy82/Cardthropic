//! Thin façade over the active variant engine.
//!
//! UI code calls these helpers instead of touching concrete engines directly.
//! That keeps variant-specific behavior behind a stable API and makes it easier
//! to add new solitaire modes without rewriting window logic.

use crate::engine::commands::{EngineCommand, EngineCommandResult};
use crate::engine::variant_engine::{engine_for_mode, VariantEngine};
use crate::engine::variant_state::VariantStateStore;
use crate::engine::view_model::GameViewModel;
use crate::game::{Card, DrawMode, GameMode, KlondikeGame};

fn engine(mode: GameMode) -> &'static dyn VariantEngine {
    let selected = engine_for_mode(mode);
    debug_assert_eq!(selected.mode(), mode);
    selected
}

fn with_engine<R>(mode: GameMode, f: impl FnOnce(&dyn VariantEngine) -> R) -> R {
    f(engine(mode))
}

fn changed_or_unchanged(changed: bool) -> EngineCommandResult {
    if changed {
        EngineCommandResult::changed()
    } else {
        EngineCommandResult::unchanged()
    }
}

pub fn execute_command(
    state: &mut VariantStateStore,
    mode: GameMode,
    command: EngineCommand,
) -> EngineCommandResult {
    let engine = engine(mode);
    match command {
        EngineCommand::DrawOrRecycle { draw_mode } => engine
            .draw_or_recycle(state, draw_mode)
            .map(EngineCommandResult::from_draw)
            .unwrap_or_else(EngineCommandResult::unchanged),
        EngineCommand::CycloneShuffleTableau => {
            changed_or_unchanged(engine.cyclone_shuffle_tableau(state))
        }
        EngineCommand::MoveWasteToFoundation => {
            changed_or_unchanged(engine.move_waste_to_foundation(state))
        }
        EngineCommand::MoveWasteToTableau { dst } => {
            changed_or_unchanged(engine.move_waste_to_tableau(state, dst))
        }
        EngineCommand::MoveTableauRunToTableau { src, start, dst } => {
            changed_or_unchanged(engine.move_tableau_run_to_tableau(state, src, start, dst))
        }
        EngineCommand::MoveTableauTopToFoundation { src } => {
            changed_or_unchanged(engine.move_tableau_top_to_foundation(state, src))
        }
        EngineCommand::MoveTableauTopToFreecell { src, cell } => {
            changed_or_unchanged(engine.move_tableau_top_to_freecell(state, src, cell))
        }
        EngineCommand::MoveFreecellToFoundation { cell } => {
            changed_or_unchanged(engine.move_freecell_to_foundation(state, cell))
        }
        EngineCommand::MoveFreecellToTableau { cell, dst } => {
            changed_or_unchanged(engine.move_freecell_to_tableau(state, cell, dst))
        }
        EngineCommand::MoveFoundationTopToTableau {
            foundation_idx,
            dst,
        } => {
            changed_or_unchanged(engine.move_foundation_top_to_tableau(state, foundation_idx, dst))
        }
    }
}

pub fn set_draw_mode(state: &mut VariantStateStore, mode: GameMode, draw_mode: DrawMode) -> bool {
    with_engine(mode, |engine| engine.set_draw_mode(state, draw_mode))
}

pub fn initialize_seeded(
    state: &mut VariantStateStore,
    mode: GameMode,
    seed: u64,
    draw_mode: DrawMode,
) -> bool {
    with_engine(mode, |engine| {
        engine.initialize_seeded(state, seed, draw_mode)
    })
}

pub fn initialize_seeded_with_draw_mode(
    state: &mut VariantStateStore,
    mode: GameMode,
    seed: u64,
    draw_mode: DrawMode,
) -> bool {
    let initialized = initialize_seeded(state, mode, seed, draw_mode);
    let _ = set_draw_mode(state, mode, draw_mode);
    initialized
}

pub fn can_move_waste_to_tableau(state: &VariantStateStore, mode: GameMode, dst: usize) -> bool {
    with_engine(mode, |engine| engine.can_move_waste_to_tableau(state, dst))
}

pub fn can_move_waste_to_foundation(state: &VariantStateStore, mode: GameMode) -> bool {
    with_engine(mode, |engine| engine.can_move_waste_to_foundation(state))
}

pub fn can_move_tableau_top_to_foundation(
    state: &VariantStateStore,
    mode: GameMode,
    src: usize,
) -> bool {
    with_engine(mode, |engine| {
        engine.can_move_tableau_top_to_foundation(state, src)
    })
}

pub fn can_move_tableau_top_to_freecell(
    state: &VariantStateStore,
    mode: GameMode,
    src: usize,
    cell: usize,
) -> bool {
    with_engine(mode, |engine| {
        engine.can_move_tableau_top_to_freecell(state, src, cell)
    })
}

pub fn can_move_freecell_to_foundation(
    state: &VariantStateStore,
    mode: GameMode,
    cell: usize,
) -> bool {
    with_engine(mode, |engine| {
        engine.can_move_freecell_to_foundation(state, cell)
    })
}

pub fn can_move_freecell_to_tableau(
    state: &VariantStateStore,
    mode: GameMode,
    cell: usize,
    dst: usize,
) -> bool {
    with_engine(mode, |engine| {
        engine.can_move_freecell_to_tableau(state, cell, dst)
    })
}

pub fn can_move_tableau_run_to_tableau(
    state: &VariantStateStore,
    mode: GameMode,
    src: usize,
    start: usize,
    dst: usize,
) -> bool {
    with_engine(mode, |engine| {
        engine.can_move_tableau_run_to_tableau(state, src, start, dst)
    })
}

pub fn can_move_foundation_top_to_tableau(
    state: &VariantStateStore,
    mode: GameMode,
    foundation_idx: usize,
    dst: usize,
) -> bool {
    with_engine(mode, |engine| {
        engine.can_move_foundation_top_to_tableau(state, foundation_idx, dst)
    })
}

pub fn waste_top(state: &VariantStateStore, mode: GameMode) -> Option<Card> {
    with_engine(mode, |engine| engine.waste_top(state))
}

pub fn tableau_top(state: &VariantStateStore, mode: GameMode, col: usize) -> Option<Card> {
    with_engine(mode, |engine| engine.tableau_top(state, col))
}

pub fn tableau_len(state: &VariantStateStore, mode: GameMode, col: usize) -> Option<usize> {
    with_engine(mode, |engine| engine.tableau_len(state, col))
}

pub fn foundation_top_exists(
    state: &VariantStateStore,
    mode: GameMode,
    foundation_idx: usize,
) -> bool {
    with_engine(mode, |engine| {
        engine.foundation_top_exists(state, foundation_idx)
    })
}

pub fn clone_klondike_for_automation(
    state: &VariantStateStore,
    mode: GameMode,
    draw_mode: DrawMode,
) -> Option<KlondikeGame> {
    with_engine(mode, |engine| engine.clone_for_automation(state, draw_mode))
}

pub fn game_view_model(
    state: &VariantStateStore,
    mode: GameMode,
    draw_mode: DrawMode,
) -> GameViewModel {
    let fallback_engine = engine(GameMode::Klondike);
    let klondike = with_engine(mode, |engine| engine.clone_for_automation(state, draw_mode))
        .or_else(|| fallback_engine.clone_for_automation(state, draw_mode))
        .unwrap_or_else(|| state.klondike().clone());
    GameViewModel::new(
        mode,
        with_engine(mode, |engine| engine.engine_ready()),
        klondike,
        draw_mode,
    )
}

pub fn is_won(state: &VariantStateStore, mode: GameMode) -> bool {
    with_engine(mode, |engine| engine.is_won(state))
}

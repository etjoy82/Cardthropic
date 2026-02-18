//! Engine dispatch and capability surface for solitaire variants.
//!
//! Extension point for new variants:
//! 1. Add a concrete engine type (or stub) and implement [`VariantEngine`].
//! 2. Register it in [`ENGINE_REGISTRY`].
//! 3. Ensure [`crate::engine::variant`] has matching metadata/spec.

use crate::engine::automation::AutomationProfile;
use crate::engine::variant_state::VariantStateStore;
use crate::game::{Card, DrawMode, DrawResult, GameMode, KlondikeGame};

mod klondike;
mod stubs;

pub use klondike::KlondikeEngine;
pub use stubs::{FreecellEngine, SpiderEngine};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VariantCapabilities {
    pub draw: bool,
    pub undo_redo: bool,
    pub smart_move: bool,
    pub autoplay: bool,
    pub rapid_wand: bool,
    pub robot_mode: bool,
    pub winnability: bool,
    pub seeded_deals: bool,
    pub cyclone_shuffle: bool,
    pub peek: bool,
    pub draw_mode_selection: bool,
}

impl VariantCapabilities {
    pub const fn disabled() -> Self {
        Self {
            draw: false,
            undo_redo: false,
            smart_move: false,
            autoplay: false,
            rapid_wand: false,
            robot_mode: false,
            winnability: false,
            seeded_deals: false,
            cyclone_shuffle: false,
            peek: false,
            draw_mode_selection: false,
        }
    }

    pub const fn klondike_default() -> Self {
        Self {
            draw: true,
            undo_redo: true,
            smart_move: true,
            autoplay: true,
            rapid_wand: true,
            robot_mode: true,
            winnability: true,
            seeded_deals: true,
            cyclone_shuffle: true,
            peek: true,
            draw_mode_selection: true,
        }
    }
}

pub trait VariantEngine: Sync {
    fn mode(&self) -> GameMode;
    fn engine_ready(&self) -> bool;
    fn capabilities(&self) -> VariantCapabilities {
        if self.engine_ready() {
            VariantCapabilities::klondike_default()
        } else {
            VariantCapabilities::disabled()
        }
    }
    fn automation_profile(&self) -> AutomationProfile {
        AutomationProfile::for_mode(self.mode())
    }

    fn draw_or_recycle(
        &self,
        _state: &mut VariantStateStore,
        _draw_mode: DrawMode,
    ) -> Option<DrawResult> {
        None
    }

    fn set_draw_mode(&self, _state: &mut VariantStateStore, _draw_mode: DrawMode) -> bool {
        false
    }

    fn initialize_seeded(
        &self,
        _state: &mut VariantStateStore,
        _seed: u64,
        _draw_mode: DrawMode,
    ) -> bool {
        false
    }

    fn cyclone_shuffle_tableau(&self, _state: &mut VariantStateStore) -> bool {
        false
    }

    fn move_waste_to_foundation(&self, _state: &mut VariantStateStore) -> bool {
        false
    }

    fn move_waste_to_tableau(&self, _state: &mut VariantStateStore, _dst: usize) -> bool {
        false
    }

    fn move_tableau_run_to_tableau(
        &self,
        _state: &mut VariantStateStore,
        _src: usize,
        _start: usize,
        _dst: usize,
    ) -> bool {
        false
    }

    fn move_tableau_top_to_foundation(&self, _state: &mut VariantStateStore, _src: usize) -> bool {
        false
    }

    fn move_tableau_top_to_freecell(
        &self,
        _state: &mut VariantStateStore,
        _src: usize,
        _cell: usize,
    ) -> bool {
        false
    }

    fn move_freecell_to_foundation(&self, _state: &mut VariantStateStore, _cell: usize) -> bool {
        false
    }

    fn move_freecell_to_tableau(
        &self,
        _state: &mut VariantStateStore,
        _cell: usize,
        _dst: usize,
    ) -> bool {
        false
    }

    fn move_foundation_top_to_tableau(
        &self,
        _state: &mut VariantStateStore,
        _foundation_idx: usize,
        _dst: usize,
    ) -> bool {
        false
    }

    fn can_move_waste_to_tableau(&self, _state: &VariantStateStore, _dst: usize) -> bool {
        false
    }

    fn can_move_waste_to_foundation(&self, _state: &VariantStateStore) -> bool {
        false
    }

    fn can_move_tableau_top_to_foundation(&self, _state: &VariantStateStore, _src: usize) -> bool {
        false
    }

    fn can_move_tableau_top_to_freecell(
        &self,
        _state: &VariantStateStore,
        _src: usize,
        _cell: usize,
    ) -> bool {
        false
    }

    fn can_move_freecell_to_foundation(&self, _state: &VariantStateStore, _cell: usize) -> bool {
        false
    }

    fn can_move_freecell_to_tableau(
        &self,
        _state: &VariantStateStore,
        _cell: usize,
        _dst: usize,
    ) -> bool {
        false
    }

    fn can_move_tableau_run_to_tableau(
        &self,
        _state: &VariantStateStore,
        _src: usize,
        _start: usize,
        _dst: usize,
    ) -> bool {
        false
    }

    fn can_move_foundation_top_to_tableau(
        &self,
        _state: &VariantStateStore,
        _foundation_idx: usize,
        _dst: usize,
    ) -> bool {
        false
    }

    fn waste_top(&self, _state: &VariantStateStore) -> Option<Card> {
        None
    }

    fn tableau_top(&self, _state: &VariantStateStore, _col: usize) -> Option<Card> {
        None
    }

    fn tableau_len(&self, _state: &VariantStateStore, _col: usize) -> Option<usize> {
        None
    }

    fn foundation_top_exists(&self, _state: &VariantStateStore, _foundation_idx: usize) -> bool {
        false
    }

    fn clone_for_automation(
        &self,
        _state: &VariantStateStore,
        _draw_mode: DrawMode,
    ) -> Option<KlondikeGame> {
        None
    }

    fn is_won(&self, _state: &VariantStateStore) -> bool {
        false
    }
}

const KLONDIKE_ENGINE: KlondikeEngine = KlondikeEngine;
const SPIDER_ENGINE: SpiderEngine = SpiderEngine;
const FREECELL_ENGINE: FreecellEngine = FreecellEngine;

const ENGINE_REGISTRY: [&'static dyn VariantEngine; 3] =
    [&KLONDIKE_ENGINE, &SPIDER_ENGINE, &FREECELL_ENGINE];

pub fn all_engines() -> &'static [&'static dyn VariantEngine] {
    &ENGINE_REGISTRY
}

pub fn engine_for_mode(mode: GameMode) -> &'static dyn VariantEngine {
    all_engines()
        .iter()
        .copied()
        .find(|engine| engine.mode() == mode)
        .unwrap_or(&KLONDIKE_ENGINE)
}

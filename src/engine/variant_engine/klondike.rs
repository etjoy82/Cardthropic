use crate::engine::automation::{AutomationProfile, KLONDIKE_AUTOMATION_PROFILE};
use crate::engine::variant_engine::VariantEngine;
use crate::engine::variant_state::VariantStateStore;
use crate::game::{Card, DrawMode, DrawResult, GameMode, KlondikeGame};

#[derive(Debug, Clone, Copy)]
pub struct KlondikeEngine;

impl VariantEngine for KlondikeEngine {
    fn mode(&self) -> GameMode {
        GameMode::Klondike
    }

    fn engine_ready(&self) -> bool {
        true
    }

    fn automation_profile(&self) -> AutomationProfile {
        KLONDIKE_AUTOMATION_PROFILE
    }

    fn draw_or_recycle(
        &self,
        state: &mut VariantStateStore,
        draw_mode: DrawMode,
    ) -> Option<DrawResult> {
        let game = state.klondike_mut();
        if game.draw_mode() != draw_mode {
            game.set_draw_mode(draw_mode);
        }
        Some(game.draw_or_recycle_with_count(draw_mode.count()))
    }

    fn set_draw_mode(&self, state: &mut VariantStateStore, draw_mode: DrawMode) -> bool {
        let game = state.klondike_mut();
        if game.draw_mode() == draw_mode {
            false
        } else {
            game.set_draw_mode(draw_mode);
            true
        }
    }

    fn initialize_seeded(
        &self,
        state: &mut VariantStateStore,
        seed: u64,
        draw_mode: DrawMode,
    ) -> bool {
        let mut game = KlondikeGame::new_with_seed(seed);
        game.set_draw_mode(draw_mode);
        state.set_klondike(game);
        true
    }

    fn cyclone_shuffle_tableau(&self, state: &mut VariantStateStore) -> bool {
        state.klondike_mut().cyclone_shuffle_tableau()
    }

    fn move_waste_to_foundation(&self, state: &mut VariantStateStore) -> bool {
        state.klondike_mut().move_waste_to_foundation()
    }

    fn move_waste_to_tableau(&self, state: &mut VariantStateStore, dst: usize) -> bool {
        state.klondike_mut().move_waste_to_tableau(dst)
    }

    fn move_tableau_run_to_tableau(
        &self,
        state: &mut VariantStateStore,
        src: usize,
        start: usize,
        dst: usize,
    ) -> bool {
        state
            .klondike_mut()
            .move_tableau_run_to_tableau(src, start, dst)
    }

    fn move_tableau_top_to_foundation(&self, state: &mut VariantStateStore, src: usize) -> bool {
        state.klondike_mut().move_tableau_top_to_foundation(src)
    }

    fn move_foundation_top_to_tableau(
        &self,
        state: &mut VariantStateStore,
        foundation_idx: usize,
        dst: usize,
    ) -> bool {
        state
            .klondike_mut()
            .move_foundation_top_to_tableau(foundation_idx, dst)
    }

    fn can_move_waste_to_tableau(&self, state: &VariantStateStore, dst: usize) -> bool {
        state.klondike().can_move_waste_to_tableau(dst)
    }

    fn can_move_waste_to_foundation(&self, state: &VariantStateStore) -> bool {
        state.klondike().can_move_waste_to_foundation()
    }

    fn can_move_tableau_top_to_foundation(&self, state: &VariantStateStore, src: usize) -> bool {
        state.klondike().can_move_tableau_top_to_foundation(src)
    }

    fn can_move_tableau_run_to_tableau(
        &self,
        state: &VariantStateStore,
        src: usize,
        start: usize,
        dst: usize,
    ) -> bool {
        state
            .klondike()
            .can_move_tableau_run_to_tableau(src, start, dst)
    }

    fn can_move_foundation_top_to_tableau(
        &self,
        state: &VariantStateStore,
        foundation_idx: usize,
        dst: usize,
    ) -> bool {
        state
            .klondike()
            .can_move_foundation_top_to_tableau(foundation_idx, dst)
    }

    fn waste_top(&self, state: &VariantStateStore) -> Option<Card> {
        state.klondike().waste_top()
    }

    fn tableau_top(&self, state: &VariantStateStore, col: usize) -> Option<Card> {
        state.klondike().tableau_top(col)
    }

    fn tableau_len(&self, state: &VariantStateStore, col: usize) -> Option<usize> {
        state.klondike().tableau_len(col)
    }

    fn foundation_top_exists(&self, state: &VariantStateStore, foundation_idx: usize) -> bool {
        state
            .klondike()
            .foundations()
            .get(foundation_idx)
            .map(|pile| !pile.is_empty())
            .unwrap_or(false)
    }

    fn clone_for_automation(
        &self,
        state: &VariantStateStore,
        draw_mode: DrawMode,
    ) -> Option<KlondikeGame> {
        let mut game = state.klondike().clone();
        game.set_draw_mode(draw_mode);
        Some(game)
    }

    fn is_won(&self, state: &VariantStateStore) -> bool {
        state.klondike().is_won()
    }
}

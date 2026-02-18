use crate::engine::automation::{
    AutomationProfile, FREECELL_AUTOMATION_PROFILE, SPIDER_AUTOMATION_PROFILE,
};
use crate::engine::variant_engine::{VariantCapabilities, VariantEngine};
use crate::engine::variant_state::VariantStateStore;
use crate::game::{Card, DrawMode, DrawResult, FreecellGame, GameMode, SpiderGame};

#[derive(Debug, Clone, Copy)]
pub struct SpiderEngine;

impl VariantEngine for SpiderEngine {
    fn mode(&self) -> GameMode {
        GameMode::Spider
    }

    fn engine_ready(&self) -> bool {
        true
    }

    fn capabilities(&self) -> VariantCapabilities {
        VariantCapabilities {
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
            draw_mode_selection: false,
        }
    }

    fn automation_profile(&self) -> AutomationProfile {
        SPIDER_AUTOMATION_PROFILE
    }

    fn initialize_seeded(
        &self,
        state: &mut VariantStateStore,
        seed: u64,
        _draw_mode: DrawMode,
    ) -> bool {
        let suit_mode = state.spider().suit_mode();
        state.set_spider(SpiderGame::new_with_seed_and_mode(seed, suit_mode));
        true
    }

    fn draw_or_recycle(
        &self,
        state: &mut VariantStateStore,
        _draw_mode: DrawMode,
    ) -> Option<DrawResult> {
        if state.spider_mut().deal_from_stock() {
            Some(DrawResult::DrewFromStock)
        } else {
            Some(DrawResult::NoOp)
        }
    }

    fn move_tableau_run_to_tableau(
        &self,
        state: &mut VariantStateStore,
        src: usize,
        start: usize,
        dst: usize,
    ) -> bool {
        state.spider_mut().move_run(src, start, dst)
    }

    fn cyclone_shuffle_tableau(&self, state: &mut VariantStateStore) -> bool {
        state.spider_mut().cyclone_shuffle_tableau()
    }

    fn can_move_tableau_run_to_tableau(
        &self,
        state: &VariantStateStore,
        src: usize,
        start: usize,
        dst: usize,
    ) -> bool {
        state.spider().can_move_run(src, start, dst)
    }

    fn tableau_top(&self, state: &VariantStateStore, col: usize) -> Option<Card> {
        state
            .spider()
            .tableau()
            .get(col)
            .and_then(|pile| pile.last().copied())
    }

    fn tableau_len(&self, state: &VariantStateStore, col: usize) -> Option<usize> {
        state.spider().tableau().get(col).map(Vec::len)
    }

    fn is_won(&self, state: &VariantStateStore) -> bool {
        state.spider().is_won()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FreecellEngine;

impl VariantEngine for FreecellEngine {
    fn mode(&self) -> GameMode {
        GameMode::Freecell
    }

    fn engine_ready(&self) -> bool {
        true
    }

    fn automation_profile(&self) -> AutomationProfile {
        FREECELL_AUTOMATION_PROFILE
    }

    fn capabilities(&self) -> VariantCapabilities {
        VariantCapabilities {
            draw: false,
            undo_redo: true,
            smart_move: true,
            autoplay: true,
            rapid_wand: true,
            robot_mode: true,
            winnability: true,
            seeded_deals: true,
            cyclone_shuffle: true,
            peek: false,
            draw_mode_selection: false,
        }
    }

    fn initialize_seeded(
        &self,
        state: &mut VariantStateStore,
        seed: u64,
        _draw_mode: DrawMode,
    ) -> bool {
        let card_count_mode = state.freecell().card_count_mode();
        state.set_freecell(FreecellGame::new_with_seed_and_card_count(
            seed,
            card_count_mode,
        ));
        true
    }

    fn move_tableau_run_to_tableau(
        &self,
        state: &mut VariantStateStore,
        src: usize,
        start: usize,
        dst: usize,
    ) -> bool {
        state
            .freecell_mut()
            .move_tableau_run_to_tableau(src, start, dst)
    }

    fn cyclone_shuffle_tableau(&self, state: &mut VariantStateStore) -> bool {
        state.freecell_mut().cyclone_shuffle_tableau()
    }

    fn move_tableau_top_to_foundation(&self, state: &mut VariantStateStore, src: usize) -> bool {
        state.freecell_mut().move_tableau_top_to_foundation(src)
    }

    fn move_tableau_top_to_freecell(
        &self,
        state: &mut VariantStateStore,
        src: usize,
        cell: usize,
    ) -> bool {
        state.freecell_mut().move_tableau_top_to_freecell(src, cell)
    }

    fn move_freecell_to_foundation(&self, state: &mut VariantStateStore, cell: usize) -> bool {
        state.freecell_mut().move_freecell_to_foundation(cell)
    }

    fn move_freecell_to_tableau(
        &self,
        state: &mut VariantStateStore,
        cell: usize,
        dst: usize,
    ) -> bool {
        state.freecell_mut().move_freecell_to_tableau(cell, dst)
    }

    fn move_foundation_top_to_tableau(
        &self,
        state: &mut VariantStateStore,
        foundation_idx: usize,
        dst: usize,
    ) -> bool {
        state
            .freecell_mut()
            .move_foundation_top_to_tableau(foundation_idx, dst)
    }

    fn can_move_tableau_top_to_foundation(&self, state: &VariantStateStore, src: usize) -> bool {
        state.freecell().can_move_tableau_top_to_foundation(src)
    }

    fn can_move_tableau_top_to_freecell(
        &self,
        state: &VariantStateStore,
        src: usize,
        cell: usize,
    ) -> bool {
        state.freecell().can_move_tableau_top_to_freecell(src, cell)
    }

    fn can_move_freecell_to_foundation(&self, state: &VariantStateStore, cell: usize) -> bool {
        state.freecell().can_move_freecell_to_foundation(cell)
    }

    fn can_move_freecell_to_tableau(
        &self,
        state: &VariantStateStore,
        cell: usize,
        dst: usize,
    ) -> bool {
        state.freecell().can_move_freecell_to_tableau(cell, dst)
    }

    fn can_move_tableau_run_to_tableau(
        &self,
        state: &VariantStateStore,
        src: usize,
        start: usize,
        dst: usize,
    ) -> bool {
        state
            .freecell()
            .can_move_tableau_run_to_tableau(src, start, dst)
    }

    fn can_move_foundation_top_to_tableau(
        &self,
        state: &VariantStateStore,
        foundation_idx: usize,
        dst: usize,
    ) -> bool {
        state
            .freecell()
            .can_move_foundation_top_to_tableau(foundation_idx, dst)
    }

    fn tableau_top(&self, state: &VariantStateStore, col: usize) -> Option<Card> {
        state.freecell().tableau_top(col)
    }

    fn tableau_len(&self, state: &VariantStateStore, col: usize) -> Option<usize> {
        state.freecell().tableau().get(col).map(Vec::len)
    }

    fn foundation_top_exists(&self, state: &VariantStateStore, foundation_idx: usize) -> bool {
        state
            .freecell()
            .foundations()
            .get(foundation_idx)
            .map(|pile| !pile.is_empty())
            .unwrap_or(false)
    }

    fn is_won(&self, state: &VariantStateStore) -> bool {
        state.freecell().is_won()
    }
}

use crate::engine::automation::{
    AutomationProfile, FREECELL_AUTOMATION_PROFILE, SPIDER_AUTOMATION_PROFILE,
};
use crate::engine::variant_engine::VariantEngine;
use crate::engine::variant_state::VariantStateStore;
use crate::game::{DrawMode, GameMode, SpiderGame};

#[derive(Debug, Clone, Copy)]
pub struct SpiderEngine;

impl VariantEngine for SpiderEngine {
    fn mode(&self) -> GameMode {
        GameMode::Spider
    }

    fn engine_ready(&self) -> bool {
        false
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
        state.set_spider(SpiderGame::new_with_seed(seed));
        true
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FreecellEngine;

impl VariantEngine for FreecellEngine {
    fn mode(&self) -> GameMode {
        GameMode::Freecell
    }

    fn engine_ready(&self) -> bool {
        false
    }

    fn automation_profile(&self) -> AutomationProfile {
        FREECELL_AUTOMATION_PROFILE
    }

    fn initialize_seeded(
        &self,
        _state: &mut VariantStateStore,
        _seed: u64,
        _draw_mode: DrawMode,
    ) -> bool {
        true
    }
}

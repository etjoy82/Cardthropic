use crate::game::GameMode;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AutomationProfile {
    pub hint_guided_analysis_budget: usize,
    pub hint_exhaustive_analysis_budget: usize,
    pub auto_play_lookahead_depth: u8,
    pub auto_play_beam_width: usize,
    pub auto_play_node_budget: usize,
    pub auto_play_win_score: i64,
    pub dialog_seed_guided_budget: usize,
    pub dialog_seed_exhaustive_budget: usize,
    pub dialog_find_winnable_state_budget: usize,
    pub rapid_wand_interval_ms: u64,
    pub rapid_wand_total_steps: u8,
    pub robot_step_interval_ms: u64,
}

pub const KLONDIKE_AUTOMATION_PROFILE: AutomationProfile = AutomationProfile {
    hint_guided_analysis_budget: 120_000,
    hint_exhaustive_analysis_budget: 220_000,
    auto_play_lookahead_depth: 3,
    auto_play_beam_width: 10,
    auto_play_node_budget: 3_200,
    auto_play_win_score: 1_200_000,
    dialog_seed_guided_budget: 180_000,
    dialog_seed_exhaustive_budget: 300_000,
    dialog_find_winnable_state_budget: 15_000,
    rapid_wand_interval_ms: 750,
    rapid_wand_total_steps: 5,
    robot_step_interval_ms: 250,
};

pub const SPIDER_AUTOMATION_PROFILE: AutomationProfile = AutomationProfile {
    hint_guided_analysis_budget: 120_000,
    hint_exhaustive_analysis_budget: 220_000,
    auto_play_lookahead_depth: 3,
    auto_play_beam_width: 10,
    auto_play_node_budget: 3_200,
    auto_play_win_score: 1_200_000,
    dialog_seed_guided_budget: 180_000,
    dialog_seed_exhaustive_budget: 300_000,
    dialog_find_winnable_state_budget: 15_000,
    rapid_wand_interval_ms: 750,
    rapid_wand_total_steps: 5,
    robot_step_interval_ms: 250,
};

pub const FREECELL_AUTOMATION_PROFILE: AutomationProfile = AutomationProfile {
    hint_guided_analysis_budget: 120_000,
    hint_exhaustive_analysis_budget: 220_000,
    auto_play_lookahead_depth: 3,
    auto_play_beam_width: 10,
    auto_play_node_budget: 3_200,
    auto_play_win_score: 1_200_000,
    dialog_seed_guided_budget: 180_000,
    dialog_seed_exhaustive_budget: 300_000,
    dialog_find_winnable_state_budget: 15_000,
    rapid_wand_interval_ms: 750,
    rapid_wand_total_steps: 5,
    robot_step_interval_ms: 50,
};

impl AutomationProfile {
    pub fn for_mode(mode: GameMode) -> Self {
        // Future variants can customize solver/autoplay tuning independently.
        match mode {
            GameMode::Klondike => KLONDIKE_AUTOMATION_PROFILE,
            GameMode::Spider => SPIDER_AUTOMATION_PROFILE,
            GameMode::Freecell => FREECELL_AUTOMATION_PROFILE,
        }
    }
}

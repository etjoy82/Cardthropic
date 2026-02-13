use crate::engine::automation::AutomationProfile;
use crate::game::KlondikeGame;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LossVerdict {
    Lost { explored_states: usize },
    WinnableLikely,
    Inconclusive { explored_states: usize },
}

#[allow(dead_code)]
pub fn analyze_klondike_loss_verdict(
    game: &KlondikeGame,
    profile: AutomationProfile,
) -> LossVerdict {
    if game.is_winnable_guided(profile.hint_guided_analysis_budget) {
        return LossVerdict::WinnableLikely;
    }

    let result = game.analyze_winnability(profile.hint_exhaustive_analysis_budget);
    if result.winnable {
        LossVerdict::WinnableLikely
    } else if result.hit_state_limit {
        LossVerdict::Inconclusive {
            explored_states: result.explored_states,
        }
    } else {
        LossVerdict::Lost {
            explored_states: result.explored_states,
        }
    }
}

pub fn analyze_klondike_loss_verdict_cancelable(
    game: &KlondikeGame,
    profile: AutomationProfile,
    cancel: &AtomicBool,
) -> Option<LossVerdict> {
    let guided = game.is_winnable_guided_cancelable(profile.hint_guided_analysis_budget, cancel)?;
    if guided {
        return Some(LossVerdict::WinnableLikely);
    }

    let result =
        game.analyze_winnability_cancelable(profile.hint_exhaustive_analysis_budget, cancel)?;
    Some(if result.winnable {
        LossVerdict::WinnableLikely
    } else if result.hit_state_limit {
        LossVerdict::Inconclusive {
            explored_states: result.explored_states,
        }
    } else {
        LossVerdict::Lost {
            explored_states: result.explored_states,
        }
    })
}

pub mod alphabeta;
pub mod iterative;
pub mod move_order;
pub mod quiescence;
pub mod tt;

use super::api::{AiConfig, SearchLimits, SearchTermination};
use crate::game::{is_in_check, ChessPosition};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

pub(crate) const SCORE_INF: i32 = 32_000;
pub(crate) const SCORE_MATE: i32 = 30_000;

pub(crate) struct SearchContext<'a> {
    pub(crate) started: Instant,
    pub(crate) limits: SearchLimits,
    pub(crate) config: AiConfig,
    pub(crate) canceled: Option<&'a AtomicBool>,
    pub(crate) nodes: u64,
    pub(crate) stop_reason: Option<SearchTermination>,
    tt: Option<tt::TranspositionTable>,
}

impl<'a> SearchContext<'a> {
    pub(crate) fn new(
        limits: SearchLimits,
        config: AiConfig,
        canceled: Option<&'a AtomicBool>,
    ) -> Self {
        let limits = sanitize_limits(limits);
        let tt = if config.enable_transposition_table {
            Some(tt::TranspositionTable::new(
                config.transposition_capacity.max(1_024),
            ))
        } else {
            None
        };
        Self {
            started: Instant::now(),
            limits,
            config,
            canceled,
            nodes: 0,
            stop_reason: None,
            tt,
        }
    }

    pub(crate) fn note_node(&mut self) -> bool {
        self.nodes = self.nodes.saturating_add(1);
        self.should_abort()
    }

    pub(crate) fn should_abort(&mut self) -> bool {
        if self.stop_reason.is_some() {
            return true;
        }
        if let Some(cancel) = self.canceled {
            if cancel.load(Ordering::Relaxed) {
                self.stop_reason = Some(SearchTermination::Canceled);
                return true;
            }
        }
        if self.limits.time_budget_ms > 0
            && self.started.elapsed().as_millis() >= u128::from(self.limits.time_budget_ms)
        {
            self.stop_reason = Some(SearchTermination::TimeBudget);
            return true;
        }
        if self.limits.node_budget > 0 && self.nodes >= self.limits.node_budget {
            self.stop_reason = Some(SearchTermination::NodeBudget);
            return true;
        }
        false
    }

    pub(crate) fn tt_mut(&mut self) -> Option<&mut tt::TranspositionTable> {
        self.tt.as_mut()
    }
}

pub(crate) fn no_legal_move_score(position: &ChessPosition, ply: u8) -> i32 {
    if is_in_check(position, position.side_to_move()) {
        -SCORE_MATE + i32::from(ply)
    } else {
        0
    }
}

fn sanitize_limits(mut limits: SearchLimits) -> SearchLimits {
    if limits.max_depth == 0 {
        limits.max_depth = 1;
    }
    limits
}

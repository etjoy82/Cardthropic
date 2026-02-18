use super::*;
use crate::engine::boundary;
use crate::engine::freecell_planner::{
    self, FreecellPlannerAction, FreecellPlannerConfig, FreecellPlannerResult,
};
use crate::engine::seed_ops;
use crate::window::hint_core::FreecellHintAction;

impl CardthropicWindow {
    const ROBOT_STALL_LIMIT: u32 = 10;
    const ROBOT_FOUNDATION_DROUGHT_LIMIT: u32 = 50;
    const ROBOT_OSCILLATION_LIMIT: u32 = 5;
    const ROBOT_SEEN_STATES_CAP: usize = 50_000;
    const FREECELL_PLANNER_WAIT_TICK_LIMIT_NORMAL: u32 = 36;
    const FREECELL_PLANNER_WAIT_TICK_LIMIT_LUDICROUS: u32 = 90;
    const FREECELL_NO_MOVE_RECOVERY_TICKS_NORMAL: u32 = 120;
    const FREECELL_NO_MOVE_RECOVERY_TICKS_LUDICROUS: u32 = 300;
    const FREECELL_PLANNER_STALL_EXPLORED_MIN: usize = 300;
    const FREECELL_PLANNER_EMPTY_STREAK_COOLDOWN_THRESHOLD: u32 = 2;
    const FREECELL_PLANNER_COOLDOWN_MOVES: u32 = 10;
    const FREECELL_PLANNER_RESTART_DEBOUNCE_TICKS: u32 = 2;
    const FREECELL_PLANNER_WAIT_LOG_INTERVAL: u32 = 8;
    const FREECELL_FALLBACK_TABU_SIGNATURE_WINDOW: usize = 24;
    const FREECELL_FALLBACK_TABU_HASH_WINDOW: usize = 24;
    const FREECELL_FALLBACK_ONLY_REBUILD_THRESHOLD: u32 = 8;
    const FREECELL_FALLBACK_ONLY_RESEED_THRESHOLD: u32 = 20;
    const ROBOT_FIXED_TIME_BUDGET_MS: u64 = 500;

    fn read_process_exec_runtime_ns() -> Option<u64> {
        // /proc/self/schedstat format: "<exec_runtime_ns> <run_delay_ns> <timeslices>"
        let raw = std::fs::read_to_string("/proc/self/schedstat").ok()?;
        raw.split_whitespace().next()?.parse::<u64>().ok()
    }

    fn robot_cpu_pct_sample(&self) -> Option<f64> {
        let now_us = glib::monotonic_time();
        let exec_ns = Self::read_process_exec_runtime_ns()?;
        let last_us = self.imp().robot_cpu_last_mono_us.get();
        let last_exec_ns = self.imp().robot_cpu_last_exec_ns.get();

        self.imp().robot_cpu_last_mono_us.set(now_us);
        self.imp().robot_cpu_last_exec_ns.set(exec_ns);

        if last_us <= 0 || now_us <= last_us || exec_ns < last_exec_ns {
            return Some(self.imp().robot_cpu_last_pct.get());
        }

        let wall_ns = (now_us - last_us) as f64 * 1000.0;
        if wall_ns <= 0.0 {
            return Some(self.imp().robot_cpu_last_pct.get());
        }
        let cpu_ns = (exec_ns - last_exec_ns) as f64;
        let pct = ((cpu_ns * 100.0) / wall_ns).max(0.0);
        self.imp().robot_cpu_last_pct.set(pct);
        Some(pct)
    }

    fn robot_step_interval_ms_current(&self) -> u64 {
        if self.imp().robot_ludicrous_enabled.get() {
            40
        } else {
            self.automation_profile().robot_step_interval_ms
        }
    }

    fn freecell_planner_wait_tick_limit(&self) -> u32 {
        if self.imp().robot_ludicrous_enabled.get() {
            Self::FREECELL_PLANNER_WAIT_TICK_LIMIT_LUDICROUS
        } else {
            Self::FREECELL_PLANNER_WAIT_TICK_LIMIT_NORMAL
        }
    }

    fn freecell_no_move_recovery_ticks(&self) -> u32 {
        if self.imp().robot_ludicrous_enabled.get() {
            Self::FREECELL_NO_MOVE_RECOVERY_TICKS_LUDICROUS
        } else {
            Self::FREECELL_NO_MOVE_RECOVERY_TICKS_NORMAL
        }
    }

    fn freecell_planner_progress_marker(game: &crate::game::FreecellGame) -> u64 {
        // Strong progress marker only: planner cooldown may break early
        // when foundation cards or empty columns improve.
        let foundation_like = game.foundations().iter().map(Vec::len).sum::<usize>() as u64;
        let empty_cols = game.tableau().iter().filter(|col| col.is_empty()).count() as u64;
        foundation_like
            .saturating_mul(1_000_000)
            .saturating_add(empty_cols.saturating_mul(10_000))
    }

    pub(super) fn freecell_planner_queue_bounds(&self) -> (usize, usize) {
        (8, 32)
    }

    fn freecell_mobility_count(game: &crate::game::FreecellGame) -> u32 {
        let mut count = 0_u32;
        for cell in 0..4 {
            if game.can_move_freecell_to_foundation(cell) {
                count = count.saturating_add(1);
            }
            for dst in 0..8 {
                if game.can_move_freecell_to_tableau(cell, dst) {
                    count = count.saturating_add(1);
                }
            }
        }
        for src in 0..8 {
            if game.can_move_tableau_top_to_foundation(src) {
                count = count.saturating_add(1);
            }
            for cell in 0..4 {
                if game.can_move_tableau_top_to_freecell(src, cell) {
                    count = count.saturating_add(1);
                }
            }
            let len = game.tableau().get(src).map(Vec::len).unwrap_or(0);
            for start in 0..len {
                for dst in 0..8 {
                    if game.can_move_tableau_run_to_tableau(src, start, dst) {
                        count = count.saturating_add(1);
                    }
                }
            }
        }
        count
    }

    fn robot_freecell_supermove_capacity(game: &crate::game::FreecellGame) -> u32 {
        let ef = game
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let et = game.tableau().iter().filter(|col| col.is_empty()).count();
        ((ef + 1) * (1usize << et)) as u32
    }

    fn robot_progress_snapshot(&self) -> (u32, u32) {
        let imp = self.imp();
        match self.active_game_mode() {
            GameMode::Spider => {
                let game = imp.game.borrow();
                let spider = game.spider();
                let foundation_like = (spider.completed_runs() as u32).saturating_mul(13);
                let empty_cols =
                    spider.tableau().iter().filter(|col| col.is_empty()).count() as u32;
                (foundation_like, empty_cols)
            }
            GameMode::Klondike => {
                let game = imp.game.borrow();
                let klondike = game.klondike();
                let foundation_like =
                    klondike.foundations().iter().map(Vec::len).sum::<usize>() as u32;
                let empty_cols = klondike
                    .tableau()
                    .iter()
                    .filter(|col| col.is_empty())
                    .count() as u32;
                (foundation_like, empty_cols)
            }
            GameMode::Freecell => {
                let game = imp.game.borrow();
                let freecell = game.freecell();
                let foundation_like =
                    freecell.foundations().iter().map(Vec::len).sum::<usize>() as u32;
                let empty_cols = freecell
                    .tableau()
                    .iter()
                    .filter(|col| col.is_empty())
                    .count() as u32;
                (foundation_like, empty_cols)
            }
        }
    }

    #[cfg(debug_assertions)]
    fn tableau_has_face_down_after_face_up(pile: &[crate::game::Card]) -> bool {
        let mut saw_face_up = false;
        for card in pile {
            if card.face_up {
                saw_face_up = true;
            } else if saw_face_up {
                return true;
            }
        }
        false
    }

    #[cfg(debug_assertions)]
    fn robot_debug_invariant_violation_detail(&self, mode: GameMode) -> Option<String> {
        use std::collections::HashSet;

        let game = self.imp().game.borrow();
        match mode {
            GameMode::Klondike => {
                let k = game.klondike();
                let mut seen = HashSet::new();
                let mut visible_total = 0_usize;
                for (fidx, pile) in k.foundations().iter().enumerate() {
                    for (idx, card) in pile.iter().enumerate() {
                        if !card.face_up {
                            return Some("klondike foundation contains face-down card".to_string());
                        }
                        if card.suit.foundation_index() != fidx {
                            return Some("klondike foundation suit mismatch".to_string());
                        }
                        if usize::from(card.rank) != idx + 1 {
                            return Some("klondike foundation rank ordering invalid".to_string());
                        }
                        if !seen.insert((card.suit, card.rank)) {
                            return Some("klondike duplicate visible card detected".to_string());
                        }
                        visible_total += 1;
                    }
                }
                for pile in k.tableau() {
                    if Self::tableau_has_face_down_after_face_up(pile) {
                        return Some(
                            "klondike tableau has face-down card above face-up".to_string(),
                        );
                    }
                    for card in pile {
                        if !(1..=13).contains(&card.rank) {
                            return Some("klondike tableau contains invalid rank".to_string());
                        }
                        if !seen.insert((card.suit, card.rank)) {
                            return Some("klondike duplicate visible card detected".to_string());
                        }
                        visible_total += 1;
                    }
                }

                let total = k.stock_len()
                    + k.waste_len()
                    + k.foundations().iter().map(Vec::len).sum::<usize>()
                    + k.tableau().iter().map(Vec::len).sum::<usize>();
                if total != 52 {
                    return Some(format!(
                        "klondike card accounting mismatch (total={total}, expected=52)"
                    ));
                }
                let _ = visible_total;
                None
            }
            GameMode::Freecell => {
                let f = game.freecell();
                let expected_total = usize::from(f.card_count_mode().card_count());
                let mut seen = HashSet::new();
                let mut total = 0_usize;

                for (fidx, pile) in f.foundations().iter().enumerate() {
                    for (idx, card) in pile.iter().enumerate() {
                        if !card.face_up {
                            return Some("freecell foundation contains face-down card".to_string());
                        }
                        if card.suit.foundation_index() != fidx {
                            return Some("freecell foundation suit mismatch".to_string());
                        }
                        if usize::from(card.rank) != idx + 1 {
                            return Some("freecell foundation rank ordering invalid".to_string());
                        }
                        if !seen.insert((card.suit, card.rank)) {
                            return Some("freecell duplicate card detected".to_string());
                        }
                        total += 1;
                    }
                }

                for slot in f.freecells().iter().flatten() {
                    if !slot.face_up {
                        return Some("freecell freecell-slot contains face-down card".to_string());
                    }
                    if !seen.insert((slot.suit, slot.rank)) {
                        return Some("freecell duplicate card detected".to_string());
                    }
                    total += 1;
                }

                for pile in f.tableau() {
                    for card in pile {
                        if !card.face_up {
                            return Some("freecell tableau contains face-down card".to_string());
                        }
                        if !seen.insert((card.suit, card.rank)) {
                            return Some("freecell duplicate card detected".to_string());
                        }
                        total += 1;
                    }
                }

                if total != expected_total || seen.len() != expected_total {
                    return Some(format!(
                        "freecell card accounting mismatch (total={total}, unique={}, expected={expected_total})",
                        seen.len()
                    ));
                }
                None
            }
            GameMode::Spider => {
                let s = game.spider();
                if s.completed_runs() > 8 {
                    return Some("spider completed_runs exceeds 8".to_string());
                }

                let mut total = s.stock_len() + s.completed_runs() * 13;
                for pile in s.tableau() {
                    if Self::tableau_has_face_down_after_face_up(pile) {
                        return Some("spider tableau has face-down card above face-up".to_string());
                    }
                    total += pile.len();
                    for card in pile {
                        if !(1..=13).contains(&card.rank) {
                            return Some("spider tableau contains invalid rank".to_string());
                        }
                    }
                }

                if total != 104 {
                    return Some(format!(
                        "spider card accounting mismatch (total={total}, expected=104)"
                    ));
                }
                None
            }
        }
    }

    #[cfg(not(debug_assertions))]
    fn robot_debug_invariant_violation_detail(&self, _mode: GameMode) -> Option<String> {
        None
    }

    pub(super) fn reset_robot_search_tracking_for_current_deal(&self) {
        let imp = self.imp();
        imp.robot_freecell_plan.borrow_mut().clear();
        imp.robot_recent_hashes.borrow_mut().clear();
        imp.robot_recent_action_signatures.borrow_mut().clear();
        imp.robot_freecell_recent_fallback_hashes
            .borrow_mut()
            .clear();
        imp.robot_freecell_recent_fallback_signatures
            .borrow_mut()
            .clear();
        imp.robot_freecell_fallback_only_streak.set(0);
        imp.robot_seen_states.borrow_mut().clear();
        *imp.robot_last_move_signature.borrow_mut() = None;
        imp.robot_inverse_oscillation_streak.set(0);
        imp.robot_hash_oscillation_streak.set(0);
        imp.robot_hash_oscillation_period.set(0);
        imp.robot_action_cycle_streak.set(0);
        imp.robot_force_loss_now.set(false);
        imp.robot_stall_streak.set(0);
        imp.robot_moves_since_foundation_progress.set(0);
        imp.robot_freecell_planner_wait_ticks.set(0);
        imp.robot_freecell_no_move_ticks.set(0);
        imp.robot_freecell_planner_empty_streak.set(0);
        imp.robot_freecell_planner_cooldown_ticks.set(0);
        imp.robot_freecell_planner_restart_debounce_ticks.set(0);
        imp.robot_freecell_planner_last_start_marker.set(0);

        if self.active_game_mode() == GameMode::Freecell {
            let state_hash = self.current_game_hash();
            self.robot_mark_seen_state(state_hash);
            imp.robot_recent_hashes.borrow_mut().push_back(state_hash);
            let (foundation_like, empty_cols) = self.robot_progress_snapshot();
            imp.robot_last_foundation_like.set(foundation_like);
            imp.robot_last_empty_cols.set(empty_cols);
            let game = imp.game.borrow();
            let freecell = game.freecell();
            imp.robot_last_freecell_mobility
                .set(Self::freecell_mobility_count(freecell));
            imp.robot_last_freecell_capacity
                .set(Self::robot_freecell_supermove_capacity(freecell));
            imp.robot_freecell_planner_last_start_marker
                .set(Self::freecell_planner_progress_marker(freecell));
            self.reset_hint_cycle_memory();
        } else {
            imp.robot_last_foundation_like.set(0);
            imp.robot_last_empty_cols.set(0);
            imp.robot_last_freecell_mobility.set(0);
            imp.robot_last_freecell_capacity.set(0);
        }
    }

    fn robot_mark_seen_state(&self, state_hash: u64) {
        let mut seen = self.imp().robot_seen_states.borrow_mut();
        seen.insert(state_hash);
        if seen.len() > Self::ROBOT_SEEN_STATES_CAP {
            seen.clear();
            seen.insert(state_hash);
        }
    }

    fn robot_track_hash_oscillation_and_mark_loss(&self, solver_source: &str) -> bool {
        if self.active_game_mode() != GameMode::Freecell {
            return false;
        }
        let hash = self.current_game_hash();
        let mut two_cycle = false;
        let mut three_cycle = false;
        {
            let mut recent = self.imp().robot_recent_hashes.borrow_mut();
            recent.push_back(hash);
            while recent.len() > 14 {
                recent.pop_front();
            }
            let len = recent.len();
            if len >= 4 {
                two_cycle = recent[len - 1] == recent[len - 3]
                    && recent[len - 2] == recent[len - 4]
                    && recent[len - 1] != recent[len - 2];
            }
            if len >= 6 {
                three_cycle = recent[len - 1] == recent[len - 4]
                    && recent[len - 2] == recent[len - 5]
                    && recent[len - 3] == recent[len - 6]
                    && recent[len - 1] != recent[len - 2]
                    && recent[len - 2] != recent[len - 3]
                    && recent[len - 1] != recent[len - 3];
            }
        }
        let detected_period = if three_cycle {
            Some(3_u8)
        } else if two_cycle {
            Some(2_u8)
        } else {
            None
        };
        let streak = if let Some(period) = detected_period {
            let prev_period = self.imp().robot_hash_oscillation_period.get();
            if prev_period == period {
                let next = self
                    .imp()
                    .robot_hash_oscillation_streak
                    .get()
                    .saturating_add(1);
                self.imp().robot_hash_oscillation_streak.set(next);
                next
            } else {
                self.imp().robot_hash_oscillation_period.set(period);
                self.imp().robot_hash_oscillation_streak.set(1);
                1
            }
        } else {
            self.imp().robot_hash_oscillation_period.set(0);
            self.imp().robot_hash_oscillation_streak.set(0);
            0
        };
        if streak > Self::ROBOT_OSCILLATION_LIMIT {
            self.cancel_freecell_planner();
            self.imp().robot_freecell_plan.borrow_mut().clear();
            self.emit_robot_status(
                "running",
                "search_reset",
                "hash oscillation threshold reached; resetting planner search",
                Some(&format!(
                    "period-{} state cycle repeated more than 5 times (streak={})",
                    self.imp().robot_hash_oscillation_period.get(),
                    streak,
                )),
                None,
                Some(false),
                solver_source,
            );
            self.render();
            return true;
        }
        false
    }

    fn fallback_hash_cycle_detected_with(
        recent: &std::collections::VecDeque<u64>,
        next_hash: u64,
    ) -> bool {
        let mut seq: Vec<u64> = recent.iter().copied().collect();
        seq.push(next_hash);
        let len = seq.len();
        if len >= 4 {
            let two_cycle = seq[len - 1] == seq[len - 3]
                && seq[len - 2] == seq[len - 4]
                && seq[len - 1] != seq[len - 2];
            if two_cycle {
                return true;
            }
        }
        if len >= 6 {
            let three_cycle = seq[len - 1] == seq[len - 4]
                && seq[len - 2] == seq[len - 5]
                && seq[len - 3] == seq[len - 6]
                && seq[len - 1] != seq[len - 2]
                && seq[len - 2] != seq[len - 3]
                && seq[len - 1] != seq[len - 3];
            if three_cycle {
                return true;
            }
        }
        false
    }

    fn robot_rejects_fallback_action(
        &self,
        game: &crate::game::FreecellGame,
        action: FreecellHintAction,
    ) -> bool {
        let sig = Self::freecell_action_cycle_signature(action);
        if self
            .imp()
            .robot_freecell_recent_fallback_signatures
            .borrow()
            .iter()
            .any(|s| s == &sig)
        {
            return true;
        }
        let Some(next_hash) = Self::freecell_next_hash_for_action(game, action) else {
            return true;
        };
        let recent_hashes = self.imp().robot_freecell_recent_fallback_hashes.borrow();
        if recent_hashes.iter().any(|h| *h == next_hash) {
            return true;
        }
        Self::fallback_hash_cycle_detected_with(&recent_hashes, next_hash)
    }

    fn robot_note_fallback_action_and_hash(&self, action: FreecellHintAction, state_hash: u64) {
        {
            let mut sigs = self
                .imp()
                .robot_freecell_recent_fallback_signatures
                .borrow_mut();
            sigs.push_back(Self::freecell_action_cycle_signature(action));
            while sigs.len() > Self::FREECELL_FALLBACK_TABU_SIGNATURE_WINDOW {
                sigs.pop_front();
            }
        }
        {
            let mut hashes = self
                .imp()
                .robot_freecell_recent_fallback_hashes
                .borrow_mut();
            hashes.push_back(state_hash);
            while hashes.len() > Self::FREECELL_FALLBACK_TABU_HASH_WINDOW {
                hashes.pop_front();
            }
        }
    }

    fn robot_track_action_cycle_and_mark_loss(
        &self,
        action_signature: Option<String>,
        solver_source: &str,
    ) -> bool {
        if self.active_game_mode() != GameMode::Freecell {
            return false;
        }
        let Some(sig) = action_signature else {
            return false;
        };

        let mut best_period: usize = 0;
        let mut best_repeats: usize = 0;
        {
            let mut recent = self.imp().robot_recent_action_signatures.borrow_mut();
            recent.push_back(sig);
            while recent.len() > 96 {
                recent.pop_front();
            }
            let recent_slice = recent.make_contiguous();
            let len = recent_slice.len();
            for period in 1..=6 {
                if len < period * 2 {
                    continue;
                }
                let mut repeats = 0_usize;
                let mut cursor = len;
                while cursor >= period * 2
                    && recent_slice[cursor - period..cursor]
                        == recent_slice[cursor - period * 2..cursor - period]
                {
                    repeats = repeats.saturating_add(1);
                    cursor -= period;
                }
                if repeats > best_repeats {
                    best_repeats = repeats;
                    best_period = period;
                }
            }
        }

        let streak = best_repeats as u32;
        self.imp().robot_action_cycle_streak.set(streak);

        if streak > Self::ROBOT_OSCILLATION_LIMIT {
            self.cancel_freecell_planner();
            self.imp().robot_freecell_plan.borrow_mut().clear();
            self.emit_robot_status(
                "running",
                "search_reset",
                "action-cycle threshold reached; resetting planner search",
                Some(&format!(
                    "same {}-move cycle repeated more than {} times (streak={})",
                    best_period,
                    Self::ROBOT_OSCILLATION_LIMIT,
                    streak
                )),
                None,
                Some(false),
                solver_source,
            );
            self.render();
            return true;
        }
        false
    }

    pub(super) fn freecell_action_cycle_signature(action: FreecellHintAction) -> String {
        match action {
            FreecellHintAction::TableauToFoundation { src } => {
                format!("t2f:T{}->F", src)
            }
            FreecellHintAction::FreecellToFoundation { cell } => {
                format!("c2f:C{}->F", cell)
            }
            FreecellHintAction::TableauRunToTableau { src, start, dst } => {
                format!("t2t:T{}:{}->T{}", src, start, dst)
            }
            FreecellHintAction::TableauToFreecell { src, cell } => {
                format!("t2c:T{}->C{}", src, cell)
            }
            FreecellHintAction::FreecellToTableau { cell, dst } => {
                format!("c2t:C{}->T{}", cell, dst)
            }
        }
    }

    fn freecell_action_signature(
        game: &crate::game::FreecellGame,
        action: FreecellHintAction,
    ) -> Option<String> {
        match action {
            FreecellHintAction::TableauToFoundation { src } => {
                let card = game.tableau_top(src)?;
                Some(format!("{}:T{}->F", card.label(), src))
            }
            FreecellHintAction::FreecellToFoundation { cell } => {
                let card = game.freecell_card(cell)?;
                Some(format!("{}:C{}->F", card.label(), cell))
            }
            FreecellHintAction::TableauRunToTableau { src, start, dst } => {
                let card = game.tableau_card(src, start)?;
                Some(format!("{}:T{}:{}->T{}", card.label(), src, start, dst))
            }
            FreecellHintAction::TableauToFreecell { src, cell } => {
                let card = game.tableau_top(src)?;
                Some(format!("{}:T{}->C{}", card.label(), src, cell))
            }
            FreecellHintAction::FreecellToTableau { cell, dst } => {
                let card = game.freecell_card(cell)?;
                Some(format!("{}:C{}->T{}", card.label(), cell, dst))
            }
        }
    }

    fn freecell_inverse_action_signature(
        game: &crate::game::FreecellGame,
        action: FreecellHintAction,
    ) -> Option<String> {
        match action {
            FreecellHintAction::TableauToFoundation { src } => {
                let card = game.tableau_top(src)?;
                Some(format!("{}:F->T{}", card.label(), src))
            }
            FreecellHintAction::FreecellToFoundation { cell } => {
                let card = game.freecell_card(cell)?;
                Some(format!("{}:F->C{}", card.label(), cell))
            }
            FreecellHintAction::TableauRunToTableau { src, start, dst } => {
                let card = game.tableau_card(src, start)?;
                Some(format!("{}:T{}->T{}:{}", card.label(), dst, src, start))
            }
            FreecellHintAction::TableauToFreecell { src, cell } => {
                let card = game.tableau_top(src)?;
                Some(format!("{}:C{}->T{}", card.label(), cell, src))
            }
            FreecellHintAction::FreecellToTableau { cell, dst } => {
                let card = game.freecell_card(cell)?;
                Some(format!("{}:T{}->C{}", card.label(), dst, cell))
            }
        }
    }

    pub(super) fn freecell_next_hash_for_action(
        game: &crate::game::FreecellGame,
        action: FreecellHintAction,
    ) -> Option<u64> {
        let mut next = game.clone();
        let changed = match action {
            FreecellHintAction::TableauToFoundation { src } => {
                next.move_tableau_top_to_foundation(src)
            }
            FreecellHintAction::FreecellToFoundation { cell } => {
                next.move_freecell_to_foundation(cell)
            }
            FreecellHintAction::TableauRunToTableau { src, start, dst } => {
                next.move_tableau_run_to_tableau(src, start, dst)
            }
            FreecellHintAction::TableauToFreecell { src, cell } => {
                next.move_tableau_top_to_freecell(src, cell)
            }
            FreecellHintAction::FreecellToTableau { cell, dst } => {
                next.move_freecell_to_tableau(cell, dst)
            }
        };
        if !changed {
            return None;
        }
        Some(Self::hash_freecell_game_state(&next))
    }

    pub(super) fn freecell_action_nodes(
        game: &crate::game::FreecellGame,
        action: FreecellHintAction,
    ) -> (HintNode, HintNode) {
        match action {
            FreecellHintAction::TableauToFoundation { src } => {
                let src_idx = game
                    .tableau()
                    .get(src)
                    .and_then(|col| col.len().checked_sub(1));
                let dst = game
                    .tableau_top(src)
                    .map(|card| HintNode::Foundation(card.suit.foundation_index()))
                    .unwrap_or(HintNode::Foundation(0));
                (
                    HintNode::Tableau {
                        col: src,
                        index: src_idx,
                    },
                    dst,
                )
            }
            FreecellHintAction::FreecellToFoundation { cell } => {
                let dst = game
                    .freecell_card(cell)
                    .map(|card| HintNode::Foundation(card.suit.foundation_index()))
                    .unwrap_or(HintNode::Foundation(0));
                (HintNode::Freecell(cell), dst)
            }
            FreecellHintAction::TableauRunToTableau { src, start, dst } => (
                HintNode::Tableau {
                    col: src,
                    index: Some(start),
                },
                HintNode::Tableau {
                    col: dst,
                    index: None,
                },
            ),
            FreecellHintAction::TableauToFreecell { src, cell } => {
                let src_idx = game
                    .tableau()
                    .get(src)
                    .and_then(|col| col.len().checked_sub(1));
                (
                    HintNode::Tableau {
                        col: src,
                        index: src_idx,
                    },
                    HintNode::Freecell(cell),
                )
            }
            FreecellHintAction::FreecellToTableau { cell, dst } => (
                HintNode::Freecell(cell),
                HintNode::Tableau {
                    col: dst,
                    index: None,
                },
            ),
        }
    }

    fn planner_action_to_hint(action: FreecellPlannerAction) -> FreecellHintAction {
        match action {
            FreecellPlannerAction::TableauToFoundation { src } => {
                FreecellHintAction::TableauToFoundation { src }
            }
            FreecellPlannerAction::FreecellToFoundation { cell } => {
                FreecellHintAction::FreecellToFoundation { cell }
            }
            FreecellPlannerAction::TableauRunToTableau { src, start, dst } => {
                FreecellHintAction::TableauRunToTableau { src, start, dst }
            }
            FreecellPlannerAction::TableauToFreecell { src, cell } => {
                FreecellHintAction::TableauToFreecell { src, cell }
            }
            FreecellPlannerAction::FreecellToTableau { cell, dst } => {
                FreecellHintAction::FreecellToTableau { cell, dst }
            }
        }
    }

    fn freecell_planner_config(&self) -> FreecellPlannerConfig {
        let time_budget_ms = self.automation_time_budget_ms();
        FreecellPlannerConfig {
            max_depth: 9,
            branch_beam: 16,
            node_budget: 3_200,
            time_budget_ms,
        }
    }

    fn automation_time_budget_ms(&self) -> u64 {
        Self::ROBOT_FIXED_TIME_BUDGET_MS
    }

    fn freecell_action_is_inverse_tableau_of_last_move(
        &self,
        game: &crate::game::FreecellGame,
        action: FreecellHintAction,
    ) -> bool {
        if !matches!(action, FreecellHintAction::TableauRunToTableau { .. }) {
            return false;
        }
        Self::freecell_inverse_action_signature(game, action)
            .as_ref()
            .zip(self.imp().robot_last_move_signature.borrow().as_ref())
            .is_some_and(|(next, last)| next == last)
    }

    fn apply_planner_action_preview(
        game: &mut crate::game::FreecellGame,
        action: FreecellPlannerAction,
    ) -> bool {
        match action {
            FreecellPlannerAction::TableauToFoundation { src } => {
                game.move_tableau_top_to_foundation(src)
            }
            FreecellPlannerAction::FreecellToFoundation { cell } => {
                game.move_freecell_to_foundation(cell)
            }
            FreecellPlannerAction::TableauRunToTableau { src, start, dst } => {
                game.move_tableau_run_to_tableau(src, start, dst)
            }
            FreecellPlannerAction::TableauToFreecell { src, cell } => {
                game.move_tableau_top_to_freecell(src, cell)
            }
            FreecellPlannerAction::FreecellToTableau { cell, dst } => {
                game.move_freecell_to_tableau(cell, dst)
            }
        }
    }

    pub(super) fn projected_freecell_planner_state(
        &self,
    ) -> Option<(crate::game::FreecellGame, u64)> {
        if self.active_game_mode() != GameMode::Freecell {
            return None;
        }
        let mut projected = self.imp().game.borrow().freecell().clone();
        for action in self.imp().robot_freecell_plan.borrow().iter().copied() {
            if !Self::apply_planner_action_preview(&mut projected, action) {
                return None;
            }
        }
        let anchor = freecell_planner::zobrist_hash(&projected);
        Some((projected, anchor))
    }

    fn cancel_freecell_planner(&self) {
        if let Some(cancel) = self.imp().robot_freecell_planner_cancel.borrow_mut().take() {
            cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        }
        self.imp().robot_freecell_planner_rx.borrow_mut().take();
        self.imp().robot_freecell_planner_running.set(false);
        self.imp().robot_freecell_planner_wait_ticks.set(0);
    }

    pub(super) fn start_freecell_planner_if_needed(&self) {
        if self.active_game_mode() != GameMode::Freecell {
            return;
        }
        if self.imp().robot_freecell_planner_running.get() {
            return;
        }
        let queue_len = self.imp().robot_freecell_plan.borrow().len();
        let (low_watermark, high_watermark) = self.freecell_planner_queue_bounds();
        if queue_len >= low_watermark || queue_len >= high_watermark {
            return;
        }
        let restart_debounce = self
            .imp()
            .robot_freecell_planner_restart_debounce_ticks
            .get();
        if restart_debounce > 0 {
            self.imp()
                .robot_freecell_planner_restart_debounce_ticks
                .set(restart_debounce.saturating_sub(1));
            return;
        }
        let Some((game, anchor_hash)) = self.projected_freecell_planner_state() else {
            return;
        };
        let progress_marker = Self::freecell_planner_progress_marker(&game);
        let last_marker = self.imp().robot_freecell_planner_last_start_marker.get();
        let cooldown = self.imp().robot_freecell_planner_cooldown_ticks.get();
        if cooldown > 0 {
            if progress_marker == last_marker {
                return;
            }
            // A meaningful state change happened; release planner cooldown early.
            self.imp().robot_freecell_planner_cooldown_ticks.set(0);
            self.imp().robot_freecell_planner_empty_streak.set(0);
        }
        // Avoid cloning the full long-lived seen set on the UI thread.
        // Planner anti-loop behavior is primarily handled by local hash guards and action-cycle checks.
        let seen = HashSet::new();
        let config = self.freecell_planner_config();
        let cancel = Arc::new(AtomicBool::new(false));
        let (tx, rx) = mpsc::channel::<FreecellPlannerResult>();
        self.imp().robot_freecell_planner_running.set(true);
        self.imp()
            .robot_freecell_planner_anchor_hash
            .set(anchor_hash);
        self.imp().robot_freecell_planner_wait_ticks.set(0);
        self.imp()
            .robot_freecell_planner_last_start_marker
            .set(progress_marker);
        *self.imp().robot_freecell_planner_cancel.borrow_mut() = Some(Arc::clone(&cancel));
        *self.imp().robot_freecell_planner_rx.borrow_mut() = Some(rx);

        thread::spawn(move || {
            let result = freecell_planner::plan_line_ida_with_astar_fallback(
                &game,
                &seen,
                config,
                Some(cancel.as_ref()),
            );
            let _ = tx.send(result);
        });
    }

    pub(super) fn collect_freecell_planner_result(&self) -> Option<FreecellPlannerResult> {
        if !self.imp().robot_freecell_planner_running.get() {
            return None;
        }
        let recv_state = {
            let rx_borrow = self.imp().robot_freecell_planner_rx.borrow();
            let Some(rx) = rx_borrow.as_ref() else {
                return Some(FreecellPlannerResult {
                    actions: VecDeque::new(),
                    explored_states: 0,
                    stalled: true,
                    stale_skips: 0,
                    inverse_prunes: 0,
                    inverse_checked: 0,
                    branch_total: 0,
                    expanded_nodes: 0,
                    expanded_h_sum: 0,
                    expanded_tb_sum: 0,
                });
            };
            rx.try_recv()
        };

        match recv_state {
            Ok(result) => {
                self.imp().robot_freecell_planner_running.set(false);
                self.imp().robot_freecell_planner_rx.borrow_mut().take();
                self.imp().robot_freecell_planner_cancel.borrow_mut().take();
                Some(result)
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.imp().robot_freecell_planner_running.set(false);
                self.imp().robot_freecell_planner_rx.borrow_mut().take();
                self.imp().robot_freecell_planner_cancel.borrow_mut().take();
                Some(FreecellPlannerResult {
                    actions: VecDeque::new(),
                    explored_states: 0,
                    stalled: true,
                    stale_skips: 0,
                    inverse_prunes: 0,
                    inverse_checked: 0,
                    branch_total: 0,
                    expanded_nodes: 0,
                    expanded_h_sum: 0,
                    expanded_tb_sum: 0,
                })
            }
        }
    }

    fn robot_update_stall_after_move_and_mark_loss(&self) -> bool {
        let (foundation_like, empty_cols) = self.robot_progress_snapshot();
        let imp = self.imp();
        let foundation_progressed = foundation_like > imp.robot_last_foundation_like.get();
        let mut progressed = foundation_progressed || empty_cols > imp.robot_last_empty_cols.get();

        if self.active_game_mode() == GameMode::Freecell {
            let game = imp.game.borrow();
            let freecell = game.freecell();
            let mobility_now = Self::freecell_mobility_count(freecell);
            let capacity_now = Self::robot_freecell_supermove_capacity(freecell);
            if mobility_now > imp.robot_last_freecell_mobility.get()
                || capacity_now > imp.robot_last_freecell_capacity.get()
            {
                progressed = true;
            }
            imp.robot_last_freecell_mobility.set(mobility_now);
            imp.robot_last_freecell_capacity.set(capacity_now);
        }

        imp.robot_last_foundation_like.set(foundation_like);
        imp.robot_last_empty_cols.set(empty_cols);
        if foundation_progressed {
            imp.robot_moves_since_foundation_progress.set(0);
        } else {
            let drought = imp
                .robot_moves_since_foundation_progress
                .get()
                .saturating_add(1);
            imp.robot_moves_since_foundation_progress.set(drought);
            if self.active_game_mode() == GameMode::Freecell
                && drought >= Self::ROBOT_FOUNDATION_DROUGHT_LIMIT
            {
                self.cancel_freecell_planner();
                self.imp().robot_freecell_plan.borrow_mut().clear();
                imp.robot_stall_streak.set(0);
                self.emit_robot_status(
                    "running",
                    "search_reset",
                    "foundation drought threshold reached; resetting planner search",
                    Some(&format!(
                        "no new foundation card in {} moves (drought={}/{})",
                        Self::ROBOT_FOUNDATION_DROUGHT_LIMIT,
                        drought,
                        Self::ROBOT_FOUNDATION_DROUGHT_LIMIT
                    )),
                    None,
                    Some(false),
                    "search",
                );
                self.render();
                return true;
            }
        }
        if progressed {
            imp.robot_stall_streak.set(0);
            return false;
        }

        let streak = imp.robot_stall_streak.get().saturating_add(1);
        imp.robot_stall_streak.set(streak);
        if self.active_game_mode() == GameMode::Freecell && streak >= Self::ROBOT_STALL_LIMIT {
            self.emit_robot_status(
                "running",
                "search_reset",
                "stall threshold reached; resetting planner search",
                Some("no foundation/empty-column/mobility/capacity progress in 10 moves"),
                None,
                Some(false),
                "search",
            );
            self.cancel_freecell_planner();
            self.imp().robot_freecell_plan.borrow_mut().clear();
            imp.robot_stall_streak.set(0);
            self.render();
            return false;
        }
        if streak >= Self::ROBOT_STALL_LIMIT {
            self.emit_robot_status(
                "running",
                "lost",
                "stall threshold reached; treating state as lost",
                Some("no foundation/empty-column/mobility/capacity progress in 10 moves"),
                None,
                Some(false),
                "search",
            );
            self.render();
            return true;
        }
        false
    }

    pub(super) fn rebind_robot_mode_timer_for_current_speed(&self) {
        if !self.imp().robot_mode_running.get() {
            return;
        }
        if let Some(source_id) = self.imp().robot_mode_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }

        let step_interval_ms = self.robot_step_interval_ms_current();
        // Ludicrous mode needs tighter tick scheduling, not idle-priority callbacks.
        let priority = glib::Priority::DEFAULT;

        let timer = glib::timeout_add_local_full(
            Duration::from_millis(step_interval_ms),
            priority,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    if !window.imp().robot_mode_running.get() {
                        return glib::ControlFlow::Break;
                    }
                    window.robot_mode_step();
                    glib::ControlFlow::Continue
                }
            ),
        );
        *self.imp().robot_mode_timer.borrow_mut() = Some(timer);
    }

    fn robot_freecell_metrics_suffix(&self) -> String {
        if self.active_game_mode() != GameMode::Freecell {
            return String::new();
        }
        let freecell_moves = self
            .imp()
            .robot_freecell_t2f_moves
            .get()
            .saturating_add(self.imp().robot_freecell_c2f_moves.get())
            .saturating_add(self.imp().robot_freecell_t2t_moves.get())
            .saturating_add(self.imp().robot_freecell_t2c_moves.get())
            .saturating_add(self.imp().robot_freecell_c2t_moves.get());
        let total_moves = self.imp().robot_moves_applied.get();
        let touch_pct = if total_moves == 0 {
            0.0
        } else {
            ((f64::from(freecell_moves) / f64::from(total_moves)) * 100.0).min(100.0)
        };
        format!(
            " fc_t2f={} fc_c2f={} fc_t2t={} fc_t2c={} fc_c2t={} fc_peak_used={} fc_moves={} fc_touch_pct={:.1} fc_drought={}/{}",
            self.imp().robot_freecell_t2f_moves.get(),
            self.imp().robot_freecell_c2f_moves.get(),
            self.imp().robot_freecell_t2t_moves.get(),
            self.imp().robot_freecell_t2c_moves.get(),
            self.imp().robot_freecell_c2t_moves.get(),
            self.imp().robot_freecell_peak_used.get(),
            freecell_moves,
            touch_pct,
            self.imp().robot_moves_since_foundation_progress.get(),
            Self::ROBOT_FOUNDATION_DROUGHT_LIMIT
        )
    }

    fn robot_note_freecell_action(&self, action: FreecellHintAction) {
        let imp = self.imp();
        match action {
            FreecellHintAction::TableauToFoundation { .. } => imp
                .robot_freecell_t2f_moves
                .set(imp.robot_freecell_t2f_moves.get().saturating_add(1)),
            FreecellHintAction::FreecellToFoundation { .. } => imp
                .robot_freecell_c2f_moves
                .set(imp.robot_freecell_c2f_moves.get().saturating_add(1)),
            FreecellHintAction::TableauRunToTableau { .. } => imp
                .robot_freecell_t2t_moves
                .set(imp.robot_freecell_t2t_moves.get().saturating_add(1)),
            FreecellHintAction::TableauToFreecell { .. } => imp
                .robot_freecell_t2c_moves
                .set(imp.robot_freecell_t2c_moves.get().saturating_add(1)),
            FreecellHintAction::FreecellToTableau { .. } => imp
                .robot_freecell_c2t_moves
                .set(imp.robot_freecell_c2t_moves.get().saturating_add(1)),
        }

        let used = imp
            .game
            .borrow()
            .freecell()
            .freecells()
            .iter()
            .filter(|slot| slot.is_some())
            .count() as u32;
        if used > imp.robot_freecell_peak_used.get() {
            imp.robot_freecell_peak_used.set(used);
        }
    }

    fn robot_total_runs(&self) -> u32 {
        self.imp()
            .robot_wins
            .get()
            .saturating_add(self.imp().robot_losses.get())
    }

    fn robot_benchmark_summary_line(&self, trigger: &str) -> String {
        let wins = self.imp().robot_wins.get();
        let losses = self.imp().robot_losses.get();
        let total = self.robot_total_runs();
        let win_rate_pct = if total == 0 {
            0.0
        } else {
            (f64::from(wins) / f64::from(total)) * 100.0
        };
        let memory = self.current_memory_mib_text().replace(' ', "");
        format!(
            "bench_v=1 trigger={} runs={} wins={} losses={} win_rate_pct={:.1} strategy={} mode={} draw={} forever={} robot_moves={} deals={} elapsed_s={} mem={}{}",
            trigger,
            total,
            wins,
            losses,
            win_rate_pct,
            self.robot_strategy().as_setting(),
            self.active_game_mode().id(),
            self.current_klondike_draw_mode().count(),
            self.imp().robot_forever_enabled.get(),
            self.imp().robot_moves_applied.get(),
            self.imp().robot_deals_tried.get(),
            self.imp().elapsed_seconds.get(),
            memory,
            self.robot_freecell_metrics_suffix()
        )
    }

    fn maybe_emit_periodic_benchmark_dump(&self, trigger: &str) {
        let total = self.robot_total_runs();
        if total == 0 || total % ROBOT_BENCHMARK_DUMP_INTERVAL != 0 {
            return;
        }
        if self.imp().robot_last_benchmark_dump_total.get() == total {
            return;
        }
        self.imp().robot_last_benchmark_dump_total.set(total);
        let status = if self.imp().robot_debug_enabled.get() {
            self.robot_benchmark_summary_line(trigger)
        } else {
            let wins = self.imp().robot_wins.get();
            let losses = self.imp().robot_losses.get();
            let win_rate_pct = if total == 0 {
                0.0
            } else {
                (f64::from(wins) / f64::from(total)) * 100.0
            };
            format!(
                "Robot benchmark: {total} runs, {wins} wins, {losses} losses ({win_rate_pct:.1}% win rate)."
            )
        };
        *self.imp().status_override.borrow_mut() = Some(status);
        self.render();
    }

    pub(super) fn copy_benchmark_snapshot(&self) {
        let line = self.robot_benchmark_summary_line("manual_copy");
        self.clipboard().set_text(&line);
        let status = if self.imp().robot_debug_enabled.get() {
            line
        } else {
            "Benchmark snapshot copied to clipboard.".to_string()
        };
        *self.imp().status_override.borrow_mut() = Some(status);
        self.render();
    }

    fn robot_outcome_fields(&self) -> String {
        let wins = self.imp().robot_wins.get();
        let losses = self.imp().robot_losses.get();
        let total = wins.saturating_add(losses);
        let win_rate = if total == 0 {
            0.0
        } else {
            (f64::from(wins) / f64::from(total)) * 100.0
        };
        format!(
            " wins={} losses={} win_rate_pct={:.1}",
            wins, losses, win_rate
        )
    }

    fn robot_compact_outcome_prefix(&self) -> String {
        let wins = self.imp().robot_wins.get();
        let losses = self.imp().robot_losses.get();
        let total = wins.saturating_add(losses);
        let win_rate = if total == 0 {
            0.0
        } else {
            (f64::from(wins) / f64::from(total)) * 100.0
        };
        format!("W/L {wins}/{losses} ({win_rate:.1}% win)")
    }

    fn handle_robot_win(&self) -> bool {
        if self.imp().robot_forever_enabled.get() && self.imp().seed_search_in_progress.get() {
            return true;
        }
        let wins = self.imp().robot_wins.get().saturating_add(1);
        self.imp().robot_wins.set(wins);
        self.maybe_emit_periodic_benchmark_dump("win");
        if !self.imp().robot_forever_enabled.get() {
            self.stop_robot_mode_with_message("Robot Mode stopped: game won.");
            return true;
        }

        self.imp().robot_freecell_plan.borrow_mut().clear();
        self.imp()
            .robot_playback
            .borrow_mut()
            .set_use_scripted_line(false);
        self.imp()
            .robot_freecell_playback
            .borrow_mut()
            .set_use_scripted_line(false);
        self.begin_robot_forever_random_reseed("won");
        true
    }

    fn begin_robot_forever_random_reseed(&self, detail: &str) {
        let start_seed = seed_ops::random_seed();
        let status = if self.imp().robot_debug_enabled.get() {
            format!("Robot forever reseed ({detail}).")
        } else {
            "Robot forever reseeded random game.".to_string()
        };
        self.start_new_game_with_seed_internal(start_seed, status, true);
    }

    pub(super) fn start_robot_mode_forever(&self) {
        self.set_robot_forever_enabled(true, true, true);
        if !self.imp().robot_mode_running.get() {
            self.toggle_robot_mode();
        }
    }

    fn robot_quote(value: &str) -> String {
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
    }

    fn robot_debug_tail(&self) -> String {
        if !self.imp().robot_debug_enabled.get() {
            return String::new();
        }

        let state_hash = self.current_game_hash();
        let mode = self.active_game_mode();
        let (scripted_enabled, scripted_remaining, scripted_ready) = if mode == GameMode::Freecell {
            let playback = self.imp().robot_freecell_playback.borrow();
            (
                playback.use_scripted_line(),
                playback.scripted_line_len(),
                playback.has_scripted_line(),
            )
        } else {
            let playback = self.imp().robot_playback.borrow();
            (
                playback.use_scripted_line(),
                playback.scripted_line_len(),
                playback.has_scripted_line(),
            )
        };
        let cpu_pct = self.robot_cpu_pct_sample().unwrap_or(0.0);
        format!(
            " state_hash={} scripted_enabled={} scripted_ready={} scripted_remaining={} cpu_pct={:.1}",
            state_hash, scripted_enabled, scripted_ready, scripted_remaining, cpu_pct
        )
    }

    fn robot_progress_fields(&self) -> String {
        let imp = self.imp();
        match self.active_game_mode() {
            GameMode::Spider => {
                let runs = imp.game.borrow().spider().completed_runs();
                format!(
                    " progress_kind=completed_runs progress_value={} progress_score={}",
                    runs,
                    runs * 100
                )
            }
            GameMode::Klondike => {
                let foundation_cards: usize = imp
                    .game
                    .borrow()
                    .klondike()
                    .foundations()
                    .iter()
                    .map(Vec::len)
                    .sum();
                format!(
                    " progress_kind=foundation_cards progress_value={} progress_score={}",
                    foundation_cards,
                    foundation_cards * 10
                )
            }
            GameMode::Freecell => {
                let foundation_cards: usize = imp
                    .game
                    .borrow()
                    .freecell()
                    .foundations()
                    .iter()
                    .map(Vec::len)
                    .sum();
                format!(
                    " progress_kind=foundation_cards progress_value={} progress_score={}",
                    foundation_cards,
                    foundation_cards * 10
                )
            }
        }
    }

    fn robot_board_fields(&self) -> String {
        let imp = self.imp();
        match self.active_game_mode() {
            GameMode::Spider => {
                let game = imp.game.borrow();
                let spider = game.spider();
                let stock_cards = spider.stock_len();
                let completed_runs = spider.completed_runs();
                let tableau_empty_cols =
                    spider.tableau().iter().filter(|col| col.is_empty()).count();
                let tableau_nonempty_cols =
                    spider.tableau().len().saturating_sub(tableau_empty_cols);
                let tableau_face_up_cards = spider
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| card.face_up)
                    .count();
                let tableau_face_down_cards = spider
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| !card.face_up)
                    .count();
                format!(
                    " stock_cards={} waste_cards=na foundation_cards=na completed_runs={} tableau_empty_cols={} tableau_nonempty_cols={} tableau_face_up_cards={} tableau_face_down_cards={}",
                    stock_cards,
                    completed_runs,
                    tableau_empty_cols,
                    tableau_nonempty_cols,
                    tableau_face_up_cards,
                    tableau_face_down_cards
                )
            }
            GameMode::Klondike => {
                let game = imp.game.borrow();
                let klondike = game.klondike();
                let stock_cards = klondike.stock_len();
                let waste_cards = klondike.waste_len();
                let foundation_cards: usize = klondike.foundations().iter().map(Vec::len).sum();
                let tableau_empty_cols = klondike
                    .tableau()
                    .iter()
                    .filter(|col| col.is_empty())
                    .count();
                let tableau_nonempty_cols =
                    klondike.tableau().len().saturating_sub(tableau_empty_cols);
                let tableau_face_up_cards = klondike
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| card.face_up)
                    .count();
                let tableau_face_down_cards = klondike
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| !card.face_up)
                    .count();
                format!(
                    " stock_cards={} waste_cards={} foundation_cards={} completed_runs=na tableau_empty_cols={} tableau_nonempty_cols={} tableau_face_up_cards={} tableau_face_down_cards={}",
                    stock_cards,
                    waste_cards,
                    foundation_cards,
                    tableau_empty_cols,
                    tableau_nonempty_cols,
                    tableau_face_up_cards,
                    tableau_face_down_cards
                )
            }
            GameMode::Freecell => {
                let game = imp.game.borrow();
                let freecell = game.freecell();
                let freecell_occupied = freecell
                    .freecells()
                    .iter()
                    .filter(|slot| slot.is_some())
                    .count();
                let foundation_cards: usize = freecell.foundations().iter().map(Vec::len).sum();
                let tableau_empty_cols = freecell
                    .tableau()
                    .iter()
                    .filter(|col| col.is_empty())
                    .count();
                let tableau_nonempty_cols =
                    freecell.tableau().len().saturating_sub(tableau_empty_cols);
                let tableau_face_up_cards = freecell
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| card.face_up)
                    .count();
                let tableau_face_down_cards = freecell
                    .tableau()
                    .iter()
                    .flat_map(|col| col.iter())
                    .filter(|card| !card.face_up)
                    .count();
                format!(
                    " stock_cards=na waste_cards={} foundation_cards={} completed_runs=na tableau_empty_cols={} tableau_nonempty_cols={} tableau_face_up_cards={} tableau_face_down_cards={}",
                    freecell_occupied,
                    foundation_cards,
                    tableau_empty_cols,
                    tableau_nonempty_cols,
                    tableau_face_up_cards,
                    tableau_face_down_cards
                )
            }
        }
    }

    fn robot_move_description(hint_move: HintMove) -> &'static str {
        match hint_move {
            HintMove::WasteToFoundation => "waste -> foundation",
            HintMove::TableauTopToFoundation { .. } => "tableau top -> foundation",
            HintMove::WasteToTableau { .. } => "waste -> tableau",
            HintMove::TableauRunToTableau { .. } => "tableau run -> tableau",
            HintMove::Draw => "draw from stock",
        }
    }

    fn robot_move_fields(&self, hint_move: Option<HintMove>) -> String {
        let Some(hint_move) = hint_move else {
            return " move_kind=na src_col=na src_start=na dst_col=na cards_moved_total=na draw_from_stock_cards=na recycle_cards=na".to_string();
        };

        match hint_move {
            HintMove::WasteToFoundation => {
                " move_kind=waste_to_foundation src_col=na src_start=na dst_col=foundation cards_moved_total=1 draw_from_stock_cards=na recycle_cards=na".to_string()
            }
            HintMove::TableauTopToFoundation { src } => {
                format!(
                    " move_kind=tableau_top_to_foundation src_col={} src_start=top dst_col=foundation cards_moved_total=1 draw_from_stock_cards=na recycle_cards=na",
                    src
                )
            }
            HintMove::WasteToTableau { dst } => {
                format!(
                    " move_kind=waste_to_tableau src_col=waste src_start=top dst_col={} cards_moved_total=1 draw_from_stock_cards=na recycle_cards=na",
                    dst
                )
            }
            HintMove::TableauRunToTableau { src, start, dst } => {
                let cards = match self.active_game_mode() {
                    GameMode::Spider => self
                        .imp()
                        .game
                        .borrow()
                        .spider()
                        .tableau()
                        .get(src)
                        .map(Vec::len)
                        .unwrap_or(0)
                        .saturating_sub(start),
                    GameMode::Klondike => self
                        .imp()
                        .game
                        .borrow()
                        .klondike()
                        .tableau_len(src)
                        .unwrap_or(0)
                        .saturating_sub(start),
                    GameMode::Freecell => 0,
                };
                format!(
                    " move_kind=tableau_run_to_tableau src_col={} src_start={} dst_col={} cards_moved_total={} draw_from_stock_cards=na recycle_cards=na",
                    src, start, dst, cards
                )
            }
            HintMove::Draw => {
                match self.active_game_mode() {
                    GameMode::Spider => {
                        let draw_from_stock_cards = self.imp().game.borrow().spider().stock_len().min(10);
                        format!(
                            " move_kind=draw src_col=stock src_start=top dst_col=tableau cards_moved_total={} draw_from_stock_cards={} recycle_cards=0",
                            draw_from_stock_cards,
                            draw_from_stock_cards
                        )
                    }
                    GameMode::Klondike => {
                        let game = self.imp().game.borrow();
                        let klondike = game.klondike();
                        let stock_before = klondike.stock_len();
                        let waste_before = klondike.waste_len();
                        let draw_n = self.current_klondike_draw_mode().count() as usize;
                        let draw_from_stock_cards = if stock_before > 0 {
                            stock_before.min(draw_n)
                        } else {
                            0
                        };
                        let recycle_cards = if stock_before == 0 { waste_before } else { 0 };
                        let cards_moved_total = draw_from_stock_cards + recycle_cards;
                        format!(
                            " move_kind=draw src_col=stock src_start=top dst_col=waste cards_moved_total={} draw_from_stock_cards={} recycle_cards={}",
                            cards_moved_total,
                            draw_from_stock_cards,
                            recycle_cards
                        )
                    }
                    GameMode::Freecell => {
                        " move_kind=draw src_col=na src_start=na dst_col=na cards_moved_total=0 draw_from_stock_cards=0 recycle_cards=0".to_string()
                    }
                }
            }
        }
    }

    fn emit_robot_status(
        &self,
        state: &str,
        event: &str,
        detail: &str,
        reason: Option<&str>,
        move_fields_override: Option<&str>,
        move_changed: Option<bool>,
        solver_source: &str,
    ) {
        let deals_tried = self.imp().robot_deals_tried.get();
        let moves_applied = self.imp().robot_moves_applied.get();
        let outcome = self.robot_outcome_fields();
        let app_moves = self.imp().move_count.get();
        let elapsed = self.imp().elapsed_seconds.get();
        let timer = if self.imp().timer_started.get() { 1 } else { 0 };
        let mode = self.active_game_mode();
        let draw = self.current_klondike_draw_mode().count();
        let seed = self.imp().current_seed.get();
        let reason_field = reason
            .map(|value| format!(" reason=\"{}\"", Self::robot_quote(value)))
            .unwrap_or_default();
        let strategy = self.robot_strategy().as_setting();
        let unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let move_changed_field = move_changed
            .map(|value| value.to_string())
            .unwrap_or_else(|| "na".to_string());
        let move_fields = move_fields_override
            .map(str::to_string)
            .unwrap_or_else(|| self.robot_move_fields(None));
        let status = if self.imp().robot_debug_enabled.get() {
            format!(
                "robot_v=1{} unix={} strategy={} state={} event={} mode={} seed={} draw={} app_moves={} robot_moves={} deals_tried={} elapsed={} timer={} solver_source={} move_changed={} detail=\"{}\"{}{}{}{}{}{}",
                outcome,
                unix,
                strategy,
                state,
                event,
                mode.id(),
                seed,
                draw,
                app_moves,
                moves_applied,
                deals_tried,
                elapsed,
                timer,
                solver_source,
                move_changed_field,
                Self::robot_quote(detail),
                reason_field,
                self.robot_progress_fields(),
                self.robot_board_fields(),
                move_fields,
                self.robot_debug_tail(),
                self.robot_freecell_metrics_suffix()
            )
        } else {
            let mut parts = vec![self.robot_compact_outcome_prefix()];
            parts.push(format!("Robot {state}: {detail}"));
            if let Some(reason_text) = reason {
                parts.push(format!("Reason: {reason_text}."));
            }
            parts.push(format!("Moves: {moves_applied}."));
            parts.push(format!("Deals: {deals_tried}."));
            if event == "move_applied" {
                parts.push("Event: move applied.".to_string());
            }
            parts.join(" | ")
        };
        *self.imp().status_override.borrow_mut() = Some(status);
    }

    fn set_robot_status_running(&self, detail: &str, solver_source: &str) {
        self.emit_robot_status("running", "status", detail, None, None, None, solver_source);
    }

    fn record_robot_move_and_maybe_pulse(
        &self,
        move_fields: &str,
        detail: &str,
        solver_source: &str,
    ) {
        let next_moves = self.imp().robot_moves_applied.get().saturating_add(1);
        self.imp().robot_moves_applied.set(next_moves);
        self.emit_robot_status(
            "running",
            "move_applied",
            &format!("move {next_moves}: {detail}"),
            None,
            Some(move_fields),
            Some(true),
            solver_source,
        );
    }

    pub(super) fn trigger_rapid_wand(&self) {
        if !self.guard_mode_engine("Rapid Wand") {
            return;
        }
        if self.imp().rapid_wand_running.get() {
            return;
        }
        self.imp().rapid_wand_running.set(true);
        self.imp().rapid_wand_nonproductive_streak.set(0);
        self.imp().rapid_wand_foundation_drought_streak.set(0);
        *self.imp().rapid_wand_blocked_state_hash.borrow_mut() = None;

        self.stop_rapid_wand();
        self.imp().rapid_wand_running.set(true);
        self.cancel_freecell_planner();
        self.imp().robot_freecell_plan.borrow_mut().clear();
        self.imp()
            .robot_recent_action_signatures
            .borrow_mut()
            .clear();
        self.imp().robot_freecell_planner_wait_ticks.set(0);
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();

        let profile = self.automation_profile();
        self.play_hint_for_player();
        let remaining_steps = Rc::new(Cell::new(profile.rapid_wand_total_steps.saturating_sub(1)));
        let timer = glib::timeout_add_local(
            Duration::from_millis(profile.rapid_wand_interval_ms),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                remaining_steps,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    if remaining_steps.get() == 0 {
                        window.finish_rapid_wand();
                        return glib::ControlFlow::Break;
                    }

                    window.play_hint_for_player();
                    remaining_steps.set(remaining_steps.get().saturating_sub(1));
                    if remaining_steps.get() == 0 {
                        window.finish_rapid_wand();
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                }
            ),
        );
        *self.imp().rapid_wand_timer.borrow_mut() = Some(timer);
    }

    pub(super) fn stop_rapid_wand(&self) {
        self.imp().rapid_wand_running.set(false);
        self.imp().rapid_wand_nonproductive_streak.set(0);
        self.imp().rapid_wand_foundation_drought_streak.set(0);
        *self.imp().rapid_wand_blocked_state_hash.borrow_mut() = None;
        if let Some(source_id) = self.imp().rapid_wand_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
    }

    pub(super) fn finish_rapid_wand(&self) {
        self.imp().rapid_wand_running.set(false);
        self.imp().rapid_wand_nonproductive_streak.set(0);
        self.imp().rapid_wand_foundation_drought_streak.set(0);
        *self.imp().rapid_wand_blocked_state_hash.borrow_mut() = None;
        let _ = self.imp().rapid_wand_timer.borrow_mut().take();
    }

    pub(super) fn toggle_robot_mode(&self) {
        if self.imp().robot_mode_running.get() {
            self.stop_robot_mode();
        } else {
            let mode = self.active_game_mode();
            if boundary::is_won(&self.imp().game.borrow(), mode) {
                self.start_random_seed_game();
                if boundary::is_won(&self.imp().game.borrow(), mode) {
                    return;
                }
            }
            if mode == GameMode::Freecell {
                let use_solver_line = self.robot_freecell_solver_anchor_matches_current_state()
                    && self
                        .imp()
                        .robot_freecell_playback
                        .borrow()
                        .has_scripted_line();
                self.imp()
                    .robot_freecell_playback
                    .borrow_mut()
                    .set_use_scripted_line(use_solver_line);
            } else {
                let use_solver_line = self.robot_solver_anchor_matches_current_state()
                    && self.imp().robot_playback.borrow().has_scripted_line();
                self.imp()
                    .robot_playback
                    .borrow_mut()
                    .set_use_scripted_line(use_solver_line);
            }
            self.start_robot_mode();
        }
    }

    pub(super) fn start_robot_mode(&self) {
        if self.imp().robot_mode_running.get() {
            return;
        }
        if !self.guard_mode_engine("Robot Mode") {
            return;
        }

        self.stop_rapid_wand();
        self.cancel_hint_loss_analysis();
        self.cancel_freecell_planner();
        self.imp().robot_mode_running.set(true);
        self.imp().robot_deals_tried.set(0);
        self.imp().robot_moves_applied.set(0);
        self.imp().robot_wins.set(0);
        self.imp().robot_losses.set(0);
        self.imp().robot_freecell_t2f_moves.set(0);
        self.imp().robot_freecell_c2f_moves.set(0);
        self.imp().robot_freecell_t2t_moves.set(0);
        self.imp().robot_freecell_t2c_moves.set(0);
        self.imp().robot_freecell_c2t_moves.set(0);
        self.imp().robot_freecell_peak_used.set(0);
        self.imp().robot_freecell_plan.borrow_mut().clear();
        self.imp().robot_freecell_planner_wait_ticks.set(0);
        self.imp().robot_cpu_last_exec_ns.set(0);
        self.imp().robot_cpu_last_mono_us.set(0);
        self.imp().robot_cpu_last_pct.set(0.0);
        self.imp().robot_last_benchmark_dump_total.set(0);
        self.reset_robot_search_tracking_for_current_deal();
        let using_solver = if self.active_game_mode() == GameMode::Freecell {
            self.imp()
                .robot_freecell_playback
                .borrow()
                .use_scripted_line()
        } else {
            self.imp().robot_playback.borrow().use_scripted_line()
        };
        self.set_robot_status_running(
            if using_solver {
                "solver armed"
            } else {
                "searching"
            },
            if using_solver { "scripted" } else { "search" },
        );
        self.render();

        self.robot_mode_step();

        self.rebind_robot_mode_timer_for_current_speed();
    }

    pub(super) fn robot_mode_step(&self) {
        if !self.imp().robot_mode_running.get() {
            return;
        }

        let mode = self.active_game_mode();
        let was_won = boundary::is_won(&self.imp().game.borrow(), mode);
        if was_won {
            let _ = self.handle_robot_win();
            return;
        }

        let mut moved = if self.imp().robot_playback.borrow().use_scripted_line() {
            let scripted_move = {
                let mut playback = self.imp().robot_playback.borrow_mut();
                playback.pop_scripted_move()
            };
            if let Some(hint_move) = scripted_move {
                let desc = Self::robot_move_description(hint_move);
                let move_fields = self.robot_move_fields(Some(hint_move));
                self.imp().auto_playing_move.set(true);
                let changed = self.apply_hint_move(hint_move);
                self.imp().auto_playing_move.set(false);
                if changed {
                    *self.imp().selected_run.borrow_mut() = None;
                    self.record_robot_move_and_maybe_pulse(
                        &move_fields,
                        &format!("solver {desc}"),
                        "scripted",
                    );
                    self.render();
                    true
                } else {
                    self.imp().robot_playback.borrow_mut().clear_scripted_line();
                    self.stop_robot_mode_with_message(
                        "Robot Mode stopped: stored solver line became invalid.",
                    );
                    return;
                }
            } else {
                self.stop_robot_mode_with_message(
                    "Robot Mode stopped: stored solver line unavailable.",
                );
                return;
            }
        } else if mode == GameMode::Freecell {
            let scripted_active = self
                .imp()
                .robot_freecell_playback
                .borrow()
                .use_scripted_line();
            if scripted_active && self.imp().robot_freecell_plan.borrow().is_empty() {
                let scripted_step = self
                    .imp()
                    .robot_freecell_playback
                    .borrow_mut()
                    .pop_scripted_move();
                if let Some(step) = scripted_step {
                    self.imp().robot_freecell_plan.borrow_mut().push_back(step);
                } else {
                    self.stop_robot_mode_with_message(
                        "Robot Mode stopped: stored FreeCell solver line unavailable.",
                    );
                    return;
                }
            }
            let planner_result = if scripted_active {
                None
            } else {
                self.collect_freecell_planner_result()
            };
            let mut planner_failed_for_state = false;
            if let Some(result) = planner_result {
                let expected_anchor = self
                    .projected_freecell_planner_state()
                    .map(|(_, hash)| hash)
                    .unwrap_or(0);
                if self.imp().robot_freecell_planner_anchor_hash.get() == expected_anchor {
                    let queued = result.actions.len();
                    let planner_exhausted = result.stalled || queued == 0;
                    let (_, high_watermark) = self.freecell_planner_queue_bounds();
                    let mut plan = self.imp().robot_freecell_plan.borrow_mut();
                    if planner_exhausted {
                        planner_failed_for_state = plan.is_empty();
                        if planner_failed_for_state {
                            let next_streak = self
                                .imp()
                                .robot_freecell_planner_empty_streak
                                .get()
                                .saturating_add(1);
                            self.imp()
                                .robot_freecell_planner_empty_streak
                                .set(next_streak);
                            if next_streak >= Self::FREECELL_PLANNER_EMPTY_STREAK_COOLDOWN_THRESHOLD
                            {
                                self.imp()
                                    .robot_freecell_planner_cooldown_ticks
                                    .set(Self::FREECELL_PLANNER_COOLDOWN_MOVES);
                            }
                        }
                    } else {
                        self.imp().robot_freecell_planner_empty_streak.set(0);
                        self.imp().robot_freecell_planner_cooldown_ticks.set(0);
                        let available = high_watermark.saturating_sub(plan.len());
                        for action in result.actions.into_iter().take(available) {
                            plan.push_back(action);
                        }
                    }
                    if self.imp().robot_debug_enabled.get() {
                        let inverse_prune_pct = if result.inverse_checked == 0 {
                            0.0
                        } else {
                            (result.inverse_prunes as f64 * 100.0) / result.inverse_checked as f64
                        };
                        let avg_branch = if result.expanded_nodes == 0 {
                            0.0
                        } else {
                            result.branch_total as f64 / result.expanded_nodes as f64
                        };
                        let avg_h = if result.expanded_nodes == 0 {
                            0.0
                        } else {
                            result.expanded_h_sum as f64 / result.expanded_nodes as f64
                        };
                        let avg_tb = if result.expanded_nodes == 0 {
                            0.0
                        } else {
                            result.expanded_tb_sum as f64 / result.expanded_nodes as f64
                        };
                        self.emit_robot_status(
                            "running",
                            if planner_failed_for_state {
                                "planner_stalled"
                            } else {
                                "planner_ready"
                            },
                            &format!(
                                "planner finished: explored_states={} queued_moves={} stall_budget={} stale_skips={} inverse_prune_pct={:.1} avg_branch={:.2} avg_h={:.2} avg_tb={:.2}",
                                result.explored_states,
                                queued,
                                Self::FREECELL_PLANNER_STALL_EXPLORED_MIN,
                                result.stale_skips,
                                inverse_prune_pct,
                                avg_branch,
                                avg_h,
                                avg_tb
                            ),
                            Some(if planner_failed_for_state {
                                "planner exhausted or empty line"
                            } else {
                                "planner produced line"
                            }),
                            None,
                            Some(false),
                            "planner",
                        );
                        self.render();
                    }
                } else if self.imp().robot_debug_enabled.get() {
                    self.emit_robot_status(
                        "running",
                        "planner_stale",
                        "dropped stale planner line from old anchor",
                        Some("state advanced before planner completed"),
                        None,
                        Some(false),
                        "planner",
                    );
                    self.render();
                }
            }
            if self.imp().robot_freecell_plan.borrow().is_empty() {
                if scripted_active {
                    self.stop_robot_mode_with_message(
                        "Robot Mode stopped: stored FreeCell solver line became unavailable.",
                    );
                    return;
                }
                if planner_failed_for_state {
                    // Planner produced no usable line for the current state.
                    // Force this tick down the fallback path instead of re-entering planner wait/restart.
                    self.cancel_freecell_planner();
                    self.imp().robot_freecell_planner_wait_ticks.set(0);
                    if self.imp().robot_debug_enabled.get() {
                        let empty_streak = self.imp().robot_freecell_planner_empty_streak.get();
                        let cooldown = self.imp().robot_freecell_planner_cooldown_ticks.get();
                        self.emit_robot_status(
                            "running",
                            "planner_stalled",
                            "planner failed for current state; attempting fallback",
                            Some(&format!(
                                "no productive line from planner (empty_streak={empty_streak}, cooldown_moves={cooldown})"
                            )),
                            None,
                            Some(false),
                            "planner",
                        );
                        self.render();
                    }
                } else {
                    self.start_freecell_planner_if_needed();
                }
                if planner_failed_for_state {
                    false
                } else if self.imp().robot_freecell_planner_running.get() {
                    let wait_ticks = self
                        .imp()
                        .robot_freecell_planner_wait_ticks
                        .get()
                        .saturating_add(1);
                    self.imp().robot_freecell_planner_wait_ticks.set(wait_ticks);
                    let no_move_ticks = self
                        .imp()
                        .robot_freecell_no_move_ticks
                        .get()
                        .saturating_add(1);
                    self.imp().robot_freecell_no_move_ticks.set(no_move_ticks);
                    let wait_limit = self.freecell_planner_wait_tick_limit();
                    let no_move_limit = self.freecell_no_move_recovery_ticks();
                    if no_move_ticks >= no_move_limit {
                        self.cancel_freecell_planner();
                        self.imp().robot_freecell_plan.borrow_mut().clear();
                        if self.imp().robot_debug_enabled.get() {
                            self.emit_robot_status(
                                "running",
                                "planner_recover",
                                "no-move watchdog triggered; forcing fallback recovery",
                                Some("no robot move applied for prolonged planner wait"),
                                None,
                                Some(false),
                                "planner",
                            );
                            self.render();
                        }
                        false
                    } else if wait_ticks >= wait_limit {
                        self.cancel_freecell_planner();
                        self.start_freecell_planner_if_needed();
                        if self.imp().robot_debug_enabled.get() {
                            self.emit_robot_status(
                                "running",
                                "planner_restart",
                                "planner watchdog triggered; restarting background planner",
                                Some("planner wait exceeded threshold"),
                                None,
                                Some(false),
                                "planner",
                            );
                            self.render();
                        }
                        return;
                    } else {
                        let should_emit_wait_log = wait_ticks == 1
                            || wait_ticks == wait_limit
                            || (wait_ticks % Self::FREECELL_PLANNER_WAIT_LOG_INTERVAL == 0);
                        if self.imp().robot_debug_enabled.get() && should_emit_wait_log {
                            self.emit_robot_status(
                                "running",
                                "planner_wait",
                                &format!(
                                    "planner running; waiting for next line (ticks={wait_ticks}/{wait_limit}, no_move={no_move_ticks}/{no_move_limit})"
                                ),
                                Some("background planning"),
                                None,
                                Some(false),
                                "planner",
                            );
                            self.render();
                        }
                        return;
                    }
                } else {
                    false
                }
            } else {
                if !scripted_active {
                    self.start_freecell_planner_if_needed();
                }
                let Some(planned_action) = self.imp().robot_freecell_plan.borrow_mut().pop_front()
                else {
                    return;
                };
                let action = Self::planner_action_to_hint(planned_action);
                let remaining = if scripted_active {
                    self.imp()
                        .robot_freecell_playback
                        .borrow()
                        .scripted_line_len()
                } else {
                    self.imp().robot_freecell_plan.borrow().len()
                };
                let (source, target) = Self::freecell_action_nodes(
                    &self.imp().game.borrow().freecell().clone(),
                    action,
                );
                self.imp().robot_freecell_planner_wait_ticks.set(0);
                self.imp().robot_freecell_no_move_ticks.set(0);
                let selected = Some((
                    if scripted_active {
                        format!("scripted step (remaining={remaining})")
                    } else {
                        format!("planner step (remaining={remaining})")
                    },
                    source,
                    target,
                    action,
                    0_i64,
                    if scripted_active {
                        "scripted".to_string()
                    } else {
                        "planner".to_string()
                    },
                ));
                match selected {
                    Some((message, source, target, action, score, solver_source)) => {
                        let freecell_before = self.imp().game.borrow().freecell().clone();
                        let inverse_signature =
                            Self::freecell_inverse_action_signature(&freecell_before, action);
                        if inverse_signature
                            .as_ref()
                            .zip(self.imp().robot_last_move_signature.borrow().as_ref())
                            .is_some_and(|(next, last)| next == last)
                        {
                            let streak = self
                                .imp()
                                .robot_inverse_oscillation_streak
                                .get()
                                .saturating_add(1);
                            self.imp().robot_inverse_oscillation_streak.set(streak);
                            if streak > Self::ROBOT_OSCILLATION_LIMIT {
                                self.cancel_freecell_planner();
                                self.imp().robot_freecell_plan.borrow_mut().clear();
                                self.emit_robot_status(
                                    "running",
                                    "search_reset",
                                    "oscillation threshold reached; resetting planner search",
                                    Some("same inverse move pair repeated more than 5 times"),
                                    None,
                                    Some(false),
                                    &solver_source,
                                );
                                self.render();
                                false
                            } else {
                                self.emit_robot_status(
                                    "running",
                                    "move_skipped",
                                    "anti-oscillation: inverse move blocked",
                                    Some(&format!(
                                        "inverse of previous move (oscillation_streak={})",
                                        streak
                                    )),
                                    None,
                                    Some(false),
                                    &solver_source,
                                );
                                self.render();
                                false
                            }
                        } else if let Some(_next_hash) =
                            Self::freecell_next_hash_for_action(&freecell_before, action)
                        {
                            self.imp().robot_inverse_oscillation_streak.set(0);
                            let progress = self
                                .freecell_progress_analysis_for_action(action)
                                .unwrap_or((
                                    i64::MIN / 4,
                                    false,
                                    "action invalid during analysis".to_string(),
                                ));
                            let progress_score = progress.0;
                            let progressed = progress.1;
                            let progress_reason = progress.2;
                            self.imp().auto_playing_move.set(true);
                            let before_hash = freecell_planner::zobrist_hash(&freecell_before);
                            let changed = self.apply_freecell_hint_action(action);
                            self.imp().auto_playing_move.set(false);
                            if changed {
                                *self.imp().selected_run.borrow_mut() = None;
                                self.imp().selected_freecell.set(None);
                                self.robot_note_freecell_action(action);
                                self.imp().robot_inverse_oscillation_streak.set(0);
                                let after_hash = self.current_game_hash();
                                self.robot_mark_seen_state(after_hash);
                                let action_signature =
                                    Self::freecell_action_signature(&freecell_before, action);
                                let action_cycle_signature =
                                    Self::freecell_action_cycle_signature(action);
                                *self.imp().robot_last_move_signature.borrow_mut() =
                                    action_signature.clone();
                                if self.robot_track_action_cycle_and_mark_loss(
                                    Some(action_cycle_signature),
                                    &solver_source,
                                ) {
                                    false
                                } else {
                                    let move_fields = format!(
                                    " move_kind=freecell_hint src_col={:?} src_start=na dst_col={:?} cards_moved_total=na draw_from_stock_cards=na recycle_cards=na hash_before={} hash_after={}",
                                    source,
                                    target,
                                    before_hash,
                                    after_hash
                                );
                                    let mut detail = message
                                        .strip_prefix("Hint: ")
                                        .unwrap_or(message.as_str())
                                        .to_string();
                                    if self.imp().robot_debug_enabled.get() {
                                        detail.push_str(&format!(
                                        " | fc_score={} fc_action={} fc_progress={} fc_progress_score={} fc_progress_reason=\"{}\"",
                                        score,
                                        Self::freecell_action_tag(action),
                                        progressed,
                                        progress_score,
                                        progress_reason.replace('"', "\\\"")
                                    ));
                                    }
                                    self.record_robot_move_and_maybe_pulse(
                                        &move_fields,
                                        &detail,
                                        &solver_source,
                                    );
                                    self.imp().robot_freecell_fallback_only_streak.set(0);
                                    self.imp()
                                        .robot_freecell_recent_fallback_hashes
                                        .borrow_mut()
                                        .clear();
                                    self.imp()
                                        .robot_freecell_recent_fallback_signatures
                                        .borrow_mut()
                                        .clear();
                                    true
                                }
                            } else {
                                if scripted_active {
                                    self.imp()
                                        .robot_freecell_playback
                                        .borrow_mut()
                                        .clear_scripted_line();
                                    self.stop_robot_mode_with_message(
                                        "Robot Mode stopped: stored FreeCell solver line became invalid.",
                                    );
                                    return;
                                } else {
                                    self.imp().robot_freecell_plan.borrow_mut().clear();
                                    self.emit_robot_status(
                                        "running",
                                        "move_invalid",
                                        "freecell move invalid; recalculating",
                                        Some("apply_freecell_hint_action returned false"),
                                        None,
                                        Some(false),
                                        &solver_source,
                                    );
                                    self.render();
                                    false
                                }
                            }
                        } else {
                            self.emit_robot_status(
                                "running",
                                "move_skipped",
                                "candidate move simulation invalid",
                                Some("could not derive next state"),
                                None,
                                Some(false),
                                &solver_source,
                            );
                            self.render();
                            false
                        }
                    }
                    None => {
                        let freecell_lost = self.imp().game.borrow().freecell().is_lost();
                        self.emit_robot_status(
                            "running",
                            if freecell_lost { "lost" } else { "no_move" },
                            if freecell_lost {
                                "no legal moves remain (loss detected)"
                            } else {
                                "no productive freecell hint move"
                            },
                            Some(if freecell_lost {
                                "freecell loss detected"
                            } else {
                                "freecell hint action unavailable"
                            }),
                            None,
                            Some(false),
                            "planner",
                        );
                        self.render();
                        false
                    }
                }
            }
        } else {
            let suggestion = self.compute_auto_play_suggestion();
            match suggestion.hint_move {
                Some(hint_move) => {
                    let desc = suggestion
                        .message
                        .strip_prefix("Hint: ")
                        .unwrap_or(suggestion.message.as_str())
                        .to_string();
                    let move_fields = self.robot_move_fields(Some(hint_move));
                    self.imp().auto_playing_move.set(true);
                    let changed = self.apply_hint_move(hint_move);
                    self.imp().auto_playing_move.set(false);
                    if changed {
                        *self.imp().selected_run.borrow_mut() = None;
                        self.record_robot_move_and_maybe_pulse(&move_fields, &desc, "search");
                        self.render();
                        true
                    } else {
                        self.emit_robot_status(
                            "running",
                            "move_invalid",
                            "move invalid; recalculating",
                            Some("apply_hint_move returned false"),
                            Some(&move_fields),
                            Some(false),
                            "search",
                        );
                        self.render();
                        false
                    }
                }
                None => {
                    self.emit_robot_status(
                        "running",
                        "no_move",
                        &suggestion.message,
                        Some("no candidate move from suggestion engine"),
                        None,
                        Some(false),
                        "search",
                    );
                    self.render();
                    false
                }
            }
        };
        if !moved && mode == GameMode::Spider {
            let extracted = {
                let mut game = self.imp().game.borrow_mut();
                game.spider_mut().extract_completed_runs()
            };
            if extracted > 0 {
                moved = true;
                self.emit_robot_status(
                    "running",
                    "move_applied",
                    &format!("auto-extracted {} completed Spider run(s).", extracted),
                    Some("completed suited K-to-A run detected and cleared"),
                    None,
                    Some(true),
                    "engine",
                );
                self.render();
            }
        }
        if self.imp().robot_strict_debug_invariants.get() {
            if let Some(detail) = self.robot_debug_invariant_violation_detail(mode) {
                self.emit_robot_status(
                    "stopped",
                    "invariant_violation",
                    &format!("debug invariant violation: {detail}"),
                    Some("robot halted to prevent state corruption"),
                    None,
                    None,
                    "engine",
                );
                self.stop_robot_mode_with_message(&format!(
                    "Robot Mode stopped: debug invariant failed ({detail})."
                ));
                return;
            }
        }

        let now_won = boundary::is_won(&self.imp().game.borrow(), mode);
        if now_won {
            let _ = self.handle_robot_win();
            return;
        }
        if moved && mode == GameMode::Freecell {
            let cooldown = self.imp().robot_freecell_planner_cooldown_ticks.get();
            if cooldown > 0 {
                self.imp()
                    .robot_freecell_planner_cooldown_ticks
                    .set(cooldown.saturating_sub(1));
            }
            self.note_current_state_for_hint_cycle();
            self.robot_mark_seen_state(self.current_game_hash());
            if self.robot_track_hash_oscillation_and_mark_loss("search") {
                moved = false;
            }
            if self.robot_update_stall_after_move_and_mark_loss() {
                moved = false;
            }
        }

        if !moved {
            if !self.imp().robot_mode_running.get() {
                return;
            }
            if self.imp().robot_forever_enabled.get() && self.imp().seed_search_in_progress.get() {
                return;
            }
            if mode == GameMode::Freecell {
                let force_loss_now = self.imp().robot_force_loss_now.replace(false);
                let has_legal = self.imp().game.borrow().freecell().has_legal_moves();
                if has_legal && !force_loss_now {
                    self.cancel_freecell_planner();
                    let freecell_before = self.imp().game.borrow().freecell().clone();
                    let mut selected = self.compute_unified_freecell_wand_action();
                    if let Some(action) = selected.as_ref().map(|(_, _, _, action, _)| *action) {
                        if self.freecell_action_is_inverse_tableau_of_last_move(
                            &freecell_before,
                            action,
                        ) {
                            selected = None;
                        }
                    }
                    if let Some(action) = selected.as_ref().map(|(_, _, _, action, _)| *action) {
                        let progress = self
                            .freecell_progress_analysis_for_action(action)
                            .unwrap_or((i64::MIN / 4, false, String::new()));
                        if !progress.1 && progress.0 <= 0 {
                            selected = None;
                        }
                    }
                    if let Some(action) = selected.as_ref().map(|(_, _, _, action, _)| *action) {
                        if self.robot_rejects_fallback_action(&freecell_before, action) {
                            selected = None;
                        }
                    }
                    if let Some((message, source, target, action, score)) = selected {
                        self.imp().robot_freecell_plan.borrow_mut().clear();
                        let before_hash = self.current_game_hash();
                        let fallback_solver_source = "fallback";
                        self.imp().auto_playing_move.set(true);
                        let changed = self.apply_freecell_hint_action(action);
                        self.imp().auto_playing_move.set(false);
                        if changed {
                            *self.imp().selected_run.borrow_mut() = None;
                            self.imp().selected_freecell.set(None);
                            self.robot_note_freecell_action(action);
                            self.imp().robot_inverse_oscillation_streak.set(0);
                            let after_hash = self.current_game_hash();
                            self.robot_mark_seen_state(after_hash);
                            let action_signature =
                                Self::freecell_action_signature(&freecell_before, action);
                            let action_cycle_signature =
                                Self::freecell_action_cycle_signature(action);
                            *self.imp().robot_last_move_signature.borrow_mut() =
                                action_signature.clone();
                            if self.robot_track_action_cycle_and_mark_loss(
                                Some(action_cycle_signature),
                                fallback_solver_source,
                            ) {}
                            if !self.imp().robot_force_loss_now.get()
                                && !self.robot_track_hash_oscillation_and_mark_loss(
                                    fallback_solver_source,
                                )
                                && !self.robot_update_stall_after_move_and_mark_loss()
                            {
                                let move_fields = format!(
                                    " move_kind=freecell_hint_fallback src_col={:?} src_start=na dst_col={:?} cards_moved_total=na draw_from_stock_cards=na recycle_cards=na hash_before={} hash_after={}",
                                    source, target, before_hash, after_hash
                                );
                                let mut detail = message
                                    .strip_prefix("Hint: ")
                                    .unwrap_or(message.as_str())
                                    .to_string();
                                if self.imp().robot_debug_enabled.get() {
                                    detail.push_str(&format!(
                                        " | fc_score={} fc_action={} fallback=true",
                                        score,
                                        Self::freecell_action_tag(action)
                                    ));
                                }
                                self.record_robot_move_and_maybe_pulse(
                                    &move_fields,
                                    &detail,
                                    fallback_solver_source,
                                );
                                self.robot_note_fallback_action_and_hash(action, after_hash);
                                let fallback_only_streak = if self
                                    .imp()
                                    .robot_moves_since_foundation_progress
                                    .get()
                                    == 0
                                {
                                    self.imp().robot_freecell_fallback_only_streak.set(0);
                                    0
                                } else {
                                    let next = self
                                        .imp()
                                        .robot_freecell_fallback_only_streak
                                        .get()
                                        .saturating_add(1);
                                    self.imp().robot_freecell_fallback_only_streak.set(next);
                                    next
                                };
                                self.imp()
                                    .robot_freecell_planner_restart_debounce_ticks
                                    .set(Self::FREECELL_PLANNER_RESTART_DEBOUNCE_TICKS);
                                let cooldown =
                                    self.imp().robot_freecell_planner_cooldown_ticks.get();
                                if cooldown > 0 {
                                    self.imp()
                                        .robot_freecell_planner_cooldown_ticks
                                        .set(cooldown.saturating_sub(1));
                                }
                                if self.imp().robot_moves_since_foundation_progress.get() > 0
                                    && fallback_only_streak
                                        >= Self::FREECELL_FALLBACK_ONLY_REBUILD_THRESHOLD
                                {
                                    self.cancel_freecell_planner();
                                    self.imp().robot_freecell_plan.borrow_mut().clear();
                                    if self.imp().robot_debug_enabled.get() {
                                        self.emit_robot_status(
                                            "running",
                                            "search_reset",
                                            "fallback-only streak threshold reached; rebuilding planner",
                                            Some(&format!(
                                                "fallback_only_streak={}/{} (no foundation progress)",
                                                fallback_only_streak,
                                                Self::FREECELL_FALLBACK_ONLY_REBUILD_THRESHOLD
                                            )),
                                            None,
                                            Some(false),
                                            "fallback",
                                        );
                                        self.render();
                                    }
                                }
                                let force_reseed_now =
                                    self.imp().robot_moves_since_foundation_progress.get() > 0
                                        && fallback_only_streak
                                            >= Self::FREECELL_FALLBACK_ONLY_RESEED_THRESHOLD;
                                if force_reseed_now {
                                    self.imp().robot_force_loss_now.set(true);
                                    self.emit_robot_status(
                                        "running",
                                        "search_reset",
                                        "fallback-only loop threshold reached; forcing reseed",
                                        Some(&format!(
                                            "fallback_only_streak={}/{} (forcing reseed)",
                                            fallback_only_streak,
                                            Self::FREECELL_FALLBACK_ONLY_RESEED_THRESHOLD
                                        )),
                                        None,
                                        Some(false),
                                        "fallback",
                                    );
                                    self.render();
                                }
                                self.imp().robot_freecell_no_move_ticks.set(0);
                                if !force_reseed_now {
                                    self.render();
                                    return;
                                }
                            }
                        }
                    }
                }
            }
            self.imp().robot_freecell_plan.borrow_mut().clear();
            self.imp().robot_recent_hashes.borrow_mut().clear();
            self.imp()
                .robot_recent_action_signatures
                .borrow_mut()
                .clear();
            self.imp()
                .robot_freecell_recent_fallback_hashes
                .borrow_mut()
                .clear();
            self.imp()
                .robot_freecell_recent_fallback_signatures
                .borrow_mut()
                .clear();
            self.imp().robot_freecell_fallback_only_streak.set(0);
            self.imp().robot_hash_oscillation_streak.set(0);
            self.imp().robot_hash_oscillation_period.set(0);
            self.imp().robot_action_cycle_streak.set(0);
            self.imp().robot_inverse_oscillation_streak.set(0);
            self.imp().robot_force_loss_now.set(false);
            let losses = self.imp().robot_losses.get().saturating_add(1);
            self.imp().robot_losses.set(losses);
            self.maybe_emit_periodic_benchmark_dump("loss");
            self.imp()
                .robot_playback
                .borrow_mut()
                .set_use_scripted_line(false);
            self.imp()
                .robot_freecell_playback
                .borrow_mut()
                .set_use_scripted_line(false);
            let next_deals_tried = self.imp().robot_deals_tried.get().saturating_add(1);
            self.imp().robot_deals_tried.set(next_deals_tried);
            if !self.imp().robot_auto_new_game_on_loss.get() {
                self.stop_robot_mode_with_message(
                    "Robot Mode stopped: game lost (auto new game on loss disabled).",
                );
            } else if self.imp().robot_forever_enabled.get() {
                self.begin_robot_forever_random_reseed("stuck_or_lost");
            } else {
                let seed = seed_ops::random_seed();
                let status = if self.imp().robot_debug_enabled.get() {
                    let unix = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    format!(
                        "robot_v=1{} unix={} strategy={} state=running event=reseed mode={} seed={} draw={} app_moves={} robot_moves={} deals_tried={} elapsed={} timer={} solver_source=search move_changed=false detail=\"stuck_or_lost\" reason=\"no productive move\"{}{} move_kind=na src_col=na src_start=na dst_col=na cards_moved_total=na draw_from_stock_cards=na recycle_cards=na{}",
                        self.robot_outcome_fields(),
                        unix,
                        self.robot_strategy().as_setting(),
                        self.active_game_mode().id(),
                        seed,
                        self.current_klondike_draw_mode().count(),
                        self.imp().move_count.get(),
                        self.imp().robot_moves_applied.get(),
                        next_deals_tried,
                        self.imp().elapsed_seconds.get(),
                        if self.imp().timer_started.get() { 1 } else { 0 },
                        self.robot_progress_fields(),
                        self.robot_board_fields(),
                        self.robot_debug_tail()
                    )
                } else {
                    "Robot got stuck and redealt a new game.".to_string()
                };
                self.start_new_game_with_seed_internal(seed, status, true);
            }
        }
    }

    pub(super) fn stop_robot_mode(&self) {
        if !self.imp().robot_mode_running.replace(false) {
            return;
        }
        self.cancel_hint_loss_analysis();
        self.cancel_freecell_planner();
        self.imp().robot_freecell_plan.borrow_mut().clear();
        self.imp().robot_playback.borrow_mut().clear_scripted_line();
        self.imp()
            .robot_freecell_playback
            .borrow_mut()
            .clear_scripted_line();
        if let Some(source_id) = self.imp().robot_mode_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        self.trim_process_memory_if_supported();
        self.emit_robot_status(
            "stopped",
            "stop",
            "robot stop requested",
            None,
            None,
            None,
            "na",
        );
        self.render();
    }

    pub(super) fn stop_robot_mode_with_message(&self, message: &str) {
        if !self.imp().robot_mode_running.replace(false) {
            return;
        }
        self.cancel_hint_loss_analysis();
        self.cancel_freecell_planner();
        self.imp().robot_freecell_plan.borrow_mut().clear();
        self.imp().robot_playback.borrow_mut().clear_scripted_line();
        self.imp()
            .robot_freecell_playback
            .borrow_mut()
            .clear_scripted_line();
        if let Some(source_id) = self.imp().robot_mode_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        self.trim_process_memory_if_supported();
        self.emit_robot_status("stopped", "stop", message, Some(message), None, None, "na");
        self.render();
    }

    pub(super) fn arm_robot_solver_anchor_for_current_state(&self, line: Vec<HintMove>) {
        self.imp().robot_playback.borrow_mut().arm(
            self.imp().current_seed.get(),
            self.current_klondike_draw_mode(),
            self.imp().move_count.get(),
            self.current_game_hash(),
            line,
        );
    }

    pub(super) fn arm_robot_freecell_solver_anchor_for_current_state(
        &self,
        line: Vec<FreecellPlannerAction>,
    ) {
        self.imp().robot_freecell_playback.borrow_mut().arm(
            self.imp().current_seed.get(),
            self.current_klondike_draw_mode(),
            self.imp().move_count.get(),
            self.current_game_hash(),
            line,
        );
    }

    pub(super) fn robot_solver_anchor_matches_current_state(&self) -> bool {
        self.imp().robot_playback.borrow().matches_current(
            self.imp().current_seed.get(),
            self.current_klondike_draw_mode(),
            self.imp().move_count.get(),
            self.current_game_hash(),
        )
    }

    pub(super) fn robot_freecell_solver_anchor_matches_current_state(&self) -> bool {
        self.imp().robot_freecell_playback.borrow().matches_current(
            self.imp().current_seed.get(),
            self.current_klondike_draw_mode(),
            self.imp().move_count.get(),
            self.current_game_hash(),
        )
    }
}

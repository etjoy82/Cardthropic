use super::*;
use crate::engine::boundary;
use crate::game::FreecellGame;
use crate::game::SpiderGame;

impl CardthropicWindow {
    pub(super) fn new_game_started_timestamp_status(&self) -> String {
        let formatted = glib::DateTime::now_local()
            .ok()
            .and_then(|now| now.format("%Y-%m-%d %H:%M:%S %Z (UTC%z)").ok())
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown local time".to_string());
        format!("New game started at {formatted}.")
    }

    pub(super) fn start_new_game_with_seed(&self, seed: u64, status: String) {
        self.start_new_game_with_seed_internal(seed, status, false);
    }

    pub(super) fn start_new_game_with_seed_internal(
        &self,
        seed: u64,
        status: String,
        preserve_robot: bool,
    ) {
        let imp = self.imp();
        if !preserve_robot {
            self.stop_robot_mode();
        }
        self.cancel_hint_loss_analysis();
        self.cancel_seed_winnable_check(None);
        self.clear_hint_effects();
        let mode = self.active_game_mode();
        if mode == GameMode::Spider {
            let suit_mode = imp.spider_suit_mode.get();
            imp.game
                .borrow_mut()
                .set_spider(SpiderGame::new_with_seed_and_mode(seed, suit_mode));
        } else if mode == GameMode::Freecell {
            let card_count_mode = imp.freecell_card_count_mode.get();
            let freecell_count = imp.freecell_cell_count.get();
            imp.game.borrow_mut().set_freecell(
                FreecellGame::new_with_seed_and_card_count_and_cells(
                    seed,
                    card_count_mode,
                    freecell_count,
                ),
            );
        }
        let _ = boundary::initialize_seeded_with_draw_mode(
            &mut imp.game.borrow_mut(),
            mode,
            seed,
            imp.klondike_draw_mode.get(),
        );
        imp.robot_playback.borrow_mut().clear();
        imp.robot_freecell_playback.borrow_mut().clear();
        imp.current_seed.set(seed);
        self.set_seed_input_text(&seed.to_string());
        self.clear_seed_entry_feedback();
        *imp.selected_run.borrow_mut() = None;
        imp.selected_freecell.set(None);
        imp.waste_selected.set(false);
        imp.pending_deal_instructions.set(true);
        imp.history.borrow_mut().clear();
        imp.future.borrow_mut().clear();
        self.roll_apm_timeline_forward();
        imp.move_count.set(0);
        imp.elapsed_seconds.set(0);
        imp.timer_started.set(false);
        self.note_seed_play_started(seed);
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        if preserve_robot {
            self.reset_robot_search_tracking_for_current_deal();
        }
        let state_hash = self.current_game_hash();
        self.start_hint_loss_analysis_if_needed(state_hash);
        let status = if mode == GameMode::Freecell {
            let already_mentions_cells = status.to_ascii_lowercase().contains("free cell");
            if already_mentions_cells {
                status
            } else {
                format!("{status} | Free Cells: {}", imp.freecell_cell_count.get())
            }
        } else {
            status
        };

        if imp.robot_debug_enabled.get() {
            let snapshot = self.build_saved_session();
            self.append_status_history_only(&format!("game_state_begin\n{snapshot}"));
            *imp.status_override.borrow_mut() =
                Some(format!("{status} | {}", snapshot.replace('\n', " | ")));
        } else {
            *imp.status_override.borrow_mut() = Some(status);
        }
        self.append_status_history_only(&self.new_game_started_timestamp_status());
        self.render();
        self.grab_focus();
    }
}

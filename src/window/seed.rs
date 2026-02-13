use super::*;
use crate::engine::boundary;

impl CardthropicWindow {
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
        let _ = boundary::initialize_seeded_with_draw_mode(
            &mut imp.game.borrow_mut(),
            mode,
            seed,
            imp.klondike_draw_mode.get(),
        );
        imp.robot_playback.borrow_mut().clear();
        imp.current_seed.set(seed);
        self.set_seed_input_text(&seed.to_string());
        self.clear_seed_entry_feedback();
        *imp.selected_run.borrow_mut() = None;
        imp.waste_selected.set(false);
        *imp.deck_error.borrow_mut() = None;
        *imp.status_override.borrow_mut() = Some(status);
        imp.history.borrow_mut().clear();
        imp.future.borrow_mut().clear();
        imp.apm_samples.borrow_mut().clear();
        imp.move_count.set(0);
        imp.elapsed_seconds.set(0);
        imp.timer_started.set(false);
        self.note_seed_play_started(seed);
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        let state_hash = self.current_game_hash();
        self.start_hint_loss_analysis_if_needed(state_hash);
        self.render();
    }
}

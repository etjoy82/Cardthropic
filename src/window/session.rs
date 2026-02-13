use super::*;
use crate::engine::boundary;
use crate::engine::session::{decode_persisted_session, encode_persisted_session};

impl CardthropicWindow {
    pub(super) fn setup_timer(&self) {
        glib::timeout_add_seconds_local(
            1,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    window.on_timer_tick();
                    glib::ControlFlow::Continue
                }
            ),
        );
    }

    fn on_timer_tick(&self) {
        let imp = self.imp();
        if imp.timer_started.get() {
            imp.elapsed_seconds.set(imp.elapsed_seconds.get() + 1);
            self.record_apm_sample_if_due();
            self.update_stats_label();
            self.persist_session_if_changed();
            if let Some(area) = imp.apm_graph_area.borrow().as_ref() {
                area.queue_draw();
            }
            self.update_apm_graph_chrome();
        }
    }

    pub(super) fn current_apm(&self) -> f64 {
        let imp = self.imp();
        let elapsed = imp.elapsed_seconds.get();
        if elapsed == 0 {
            0.0
        } else {
            (imp.move_count.get() as f64 * 60.0) / elapsed as f64
        }
    }

    fn record_apm_sample_if_due(&self) {
        let imp = self.imp();
        let elapsed = imp.elapsed_seconds.get();
        if elapsed == 0 || elapsed % 5 != 0 {
            return;
        }
        let mut samples = imp.apm_samples.borrow_mut();
        if samples
            .last()
            .map(|sample| sample.elapsed_seconds == elapsed)
            .unwrap_or(false)
        {
            return;
        }
        samples.push(ApmSample {
            elapsed_seconds: elapsed,
            apm: self.current_apm(),
        });
    }

    pub(super) fn build_saved_session(&self) -> String {
        let imp = self.imp();
        let mode = self.active_game_mode();
        let draw_mode = imp.klondike_draw_mode.get();
        let game = imp.game.borrow();
        let timer_started = imp.timer_started.get() && !boundary::is_won(&game, mode);
        encode_persisted_session(
            &game,
            imp.current_seed.get(),
            mode,
            imp.move_count.get(),
            imp.elapsed_seconds.get(),
            timer_started,
            draw_mode,
        )
    }

    pub(super) fn persist_session_if_changed(&self) {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings else {
            return;
        };
        let payload = self.build_saved_session();
        if *self.imp().last_saved_session.borrow() == payload {
            return;
        }
        let _ = settings.set_string(SETTINGS_KEY_SAVED_SESSION, &payload);
        *self.imp().last_saved_session.borrow_mut() = payload;
    }

    pub(super) fn try_restore_saved_session(&self) -> bool {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings else {
            return false;
        };
        let raw = settings.string(SETTINGS_KEY_SAVED_SESSION).to_string();
        if raw.trim().is_empty() {
            return false;
        }
        let Some(session) = decode_persisted_session(&raw) else {
            let _ = settings.set_string(SETTINGS_KEY_SAVED_SESSION, "");
            return false;
        };

        let imp = self.imp();
        imp.game.borrow_mut().set_runtime(session.runtime.clone());
        imp.current_seed.set(session.seed);
        imp.move_count.set(session.move_count);
        imp.elapsed_seconds.set(session.elapsed_seconds);
        imp.timer_started.set(session.timer_started);
        *imp.selected_run.borrow_mut() = None;
        imp.waste_selected.set(false);
        imp.history.borrow_mut().clear();
        imp.future.borrow_mut().clear();
        imp.apm_samples.borrow_mut().clear();
        imp.current_game_mode.set(session.mode);
        imp.klondike_draw_mode.set(session.klondike_draw_mode);
        let _ = boundary::set_draw_mode(
            &mut imp.game.borrow_mut(),
            session.mode,
            session.klondike_draw_mode,
        );
        imp.timer_started
            .set(imp.timer_started.get() && !boundary::is_won(&imp.game.borrow(), session.mode));
        self.set_seed_input_text(&session.seed.to_string());
        *imp.status_override.borrow_mut() = Some("Resumed previous game.".to_string());
        *imp.last_saved_session.borrow_mut() = raw;
        true
    }

    pub(super) fn update_stats_label(&self) {
        let imp = self.imp();
        let elapsed = imp.elapsed_seconds.get();
        let apm = self.current_apm();
        imp.stats_label.set_label(&format!(
            "Moves: {}   APM: {:.1}   Time: {}",
            imp.move_count.get(),
            apm,
            format_time(elapsed)
        ));
    }
}

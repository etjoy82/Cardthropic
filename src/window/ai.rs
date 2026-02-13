use super::*;
use crate::engine::seed_ops;

impl CardthropicWindow {
    pub(super) fn start_random_winnable_seed_game(&self) {
        if !self.guard_mode_engine("Starting a winnable deal") {
            return;
        }
        if self.imp().seed_search_in_progress.get() {
            *self.imp().status_override.borrow_mut() = Some(seed_ops::msg_seed_search_running());
            self.render();
            return;
        }

        self.cancel_seed_winnable_check(None);
        self.clear_seed_entry_feedback();
        let start_seed = seed_ops::random_seed();
        self.set_seed_input_text(&start_seed.to_string());

        self.begin_winnable_seed_search(
            start_seed,
            winnability::default_find_winnable_attempts(),
            self.automation_profile().dialog_find_winnable_state_budget,
        );
    }

    pub(super) fn begin_winnable_seed_search(
        &self,
        start_seed: u64,
        attempts: u32,
        max_states: usize,
    ) {
        let imp = self.imp();
        if imp.seed_search_in_progress.replace(true) {
            *imp.status_override.borrow_mut() = Some(seed_ops::msg_seed_search_running());
            self.render();
            return;
        }

        let draw_mode = self.current_klondike_draw_mode();
        let deal_count = draw_mode.count();
        *imp.status_override.borrow_mut() = Some(seed_ops::msg_searching_winnable_seed(
            start_seed, deal_count, attempts, max_states,
        ));
        self.render();

        let (sender, receiver) = mpsc::channel::<Option<(u64, u32, Vec<SolverMove>)>>();
        thread::spawn(move || {
            let result = winnability::find_winnable_seed_parallel(
                start_seed, attempts, max_states, draw_mode,
            );
            let _ = sender.send(result);
        });

        glib::timeout_add_local(
            Duration::from_millis(40),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || match receiver.try_recv() {
                    Ok(Some((seed, tested, solver_line))) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        window.start_new_game_with_seed(
                            seed,
                            seed_ops::msg_started_winnable_seed(seed, deal_count, tested),
                        );
                        if let Some(line) = map_solver_line_to_hint_line(
                            &window.imp().game.borrow().clone(),
                            solver_line.as_slice(),
                        ) {
                            window.arm_robot_solver_anchor_for_current_state(line);
                        }
                        glib::ControlFlow::Break
                    }
                    Ok(None) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        *imp.status_override.borrow_mut() = Some(seed_ops::msg_no_winnable_seed(
                            start_seed, deal_count, attempts,
                        ));
                        window.render();
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        let imp = window.imp();
                        imp.seed_search_in_progress.set(false);
                        *imp.status_override.borrow_mut() =
                            Some(seed_ops::msg_seed_search_stopped_unexpectedly(deal_count));
                        window.render();
                        glib::ControlFlow::Break
                    }
                }
            ),
        );
    }
}

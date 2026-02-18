use super::*;

impl CardthropicWindow {
    fn is_mobile_phone_breakpoint(&self) -> bool {
        let imp = self.imp();
        let width = self.width();
        let height = self.height();
        let enter = MOBILE_PHONE_BREAKPOINT_PX;
        let exit = MOBILE_PHONE_BREAKPOINT_PX + MOBILE_PHONE_BREAKPOINT_HYSTERESIS_PX;

        if imp.mobile_phone_mode.get() {
            // Hysteresis: once in mobile mode, require a larger margin before exiting,
            // to prevent resize flapping around the breakpoint.
            width <= exit || height <= exit
        } else {
            width <= enter || height <= enter
        }
    }

    fn set_mobile_phone_mode(&self, enabled: bool) {
        let imp = self.imp();
        if imp.mobile_phone_mode.get() == enabled {
            return;
        }
        imp.mobile_phone_mode.set(enabled);

        if enabled {
            imp.board_box.add_css_class("mobile-phone-mode");
            imp.tableau_row.set_homogeneous(false);
            imp.foundations_area_box.set_halign(gtk::Align::Start);
            // Mobile mode always hides HUD-heavy rows for gameplay-first focus.
            if imp.hud_enabled.get() {
                imp.hud_auto_hidden.set(true);
                self.set_hud_enabled(false, false);
            }
        } else {
            imp.board_box.remove_css_class("mobile-phone-mode");
            imp.tableau_row.set_homogeneous(true);
            imp.foundations_area_box.set_halign(gtk::Align::End);
        }
        self.apply_mobile_phone_mode_overrides();
    }

    pub(super) fn sync_mobile_phone_mode_to_size(&self) {
        self.set_mobile_phone_mode(self.is_mobile_phone_breakpoint());
    }

    pub(super) fn setup_hud_action(&self) {
        let action = gio::SimpleAction::new_stateful("enable-hud", None, &true.to_variant());
        action.connect_change_state(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, value| {
                let enabled = value
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(true);
                window.set_hud_enabled(enabled, true);
            }
        ));
        self.add_action(&action);
    }

    pub(super) fn set_hud_enabled(&self, hud_enabled: bool, persist: bool) {
        let imp = self.imp();
        imp.hud_enabled.set(hud_enabled);
        imp.seed_controls_row.set_visible(hud_enabled);
        imp.status_block_box.set_visible(hud_enabled);

        if persist {
            // Explicit user toggle clears the auto-hide flag so it doesn't
            // immediately fight back against the user's choice.
            imp.hud_auto_hidden.set(false);
            if let Some(settings) = imp.settings.borrow().clone() {
                let _ = settings.set_boolean(SETTINGS_KEY_ENABLE_HUD, hud_enabled);
            }
        }

        if let Some(action) = self.lookup_action("enable-hud") {
            if let Ok(action) = action.downcast::<gio::SimpleAction>() {
                let current = action
                    .state()
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(true);
                if current != hud_enabled {
                    action.set_state(&hud_enabled.to_variant());
                }
            }
        }
    }

    /// Auto-hide the HUD when the window shrinks below 600×600, and restore it
    /// when the window grows back above the threshold — but only if the HUD was
    /// hidden automatically (not by an explicit user action).
    pub(super) fn sync_hud_visibility_to_size(&self) {
        let imp = self.imp();
        if imp.mobile_phone_mode.get() {
            return;
        }
        let compact = self.width() < 600 || self.height() < 600;
        if compact && imp.hud_enabled.get() && !imp.hud_auto_hidden.get() {
            imp.hud_auto_hidden.set(true);
            self.set_hud_enabled(false, false);
        } else if !compact && imp.hud_auto_hidden.get() {
            imp.hud_auto_hidden.set(false);
            self.set_hud_enabled(true, false);
        }
    }

    pub(super) fn setup_forever_mode_action(&self) {
        let action = gio::SimpleAction::new_stateful("forever-mode", None, &false.to_variant());
        action.connect_change_state(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, value| {
                let enabled = value
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(false);
                window.set_robot_forever_enabled(enabled, true, true);
            }
        ));
        self.add_action(&action);
    }

    pub(super) fn setup_ludicrous_speed_action(&self) {
        let action = gio::SimpleAction::new_stateful("ludicrous-speed", None, &false.to_variant());
        action.connect_change_state(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, value| {
                let enabled = value
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(false);
                window.set_robot_ludicrous_enabled(enabled, true, true);
            }
        ));
        self.add_action(&action);
    }

    pub(super) fn setup_robot_auto_new_game_on_loss_action(&self) {
        let action = gio::SimpleAction::new_stateful(
            "robot-auto-new-game-on-loss",
            None,
            &true.to_variant(),
        );
        action.connect_change_state(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, value| {
                let enabled = value
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(true);
                window.set_robot_auto_new_game_on_loss_enabled(enabled, true, true);
            }
        ));
        self.add_action(&action);
    }

    pub(super) fn setup_robot_debug_action(&self) {
        let action =
            gio::SimpleAction::new_stateful("robot-debug-toggle", None, &false.to_variant());
        action.connect_change_state(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, value| {
                let enabled = value
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(false);
                window.set_robot_debug_enabled(enabled, true, true);
            }
        ));
        self.add_action(&action);
    }

    pub(super) fn setup_robot_strict_debug_invariants_action(&self) {
        let action = gio::SimpleAction::new_stateful(
            "robot-strict-debug-invariants",
            None,
            &true.to_variant(),
        );
        action.connect_change_state(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, value| {
                let enabled = value
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(true);
                window.set_robot_strict_debug_invariants_enabled(enabled, true, true);
            }
        ));
        self.add_action(&action);
    }

    pub(super) fn set_robot_forever_enabled(&self, enabled: bool, persist: bool, announce: bool) {
        let imp = self.imp();
        imp.robot_forever_enabled.set(enabled);
        if enabled {
            imp.robot_button.add_css_class("suggested-action");
            imp.robot_button
                .set_tooltip_text(Some("Robot Mode (Forever Mode enabled)"));
        } else {
            imp.robot_button.remove_css_class("suggested-action");
            imp.robot_button.set_tooltip_text(Some("Robot Mode"));
        }

        if let Some(action) = self.lookup_action("forever-mode") {
            if let Ok(action) = action.downcast::<gio::SimpleAction>() {
                let current = action
                    .state()
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(false);
                if current != enabled {
                    action.set_state(&enabled.to_variant());
                }
            }
        }

        if persist {
            if let Some(settings) = imp.settings.borrow().clone() {
                let _ = settings.set_boolean(SETTINGS_KEY_FOREVER_MODE, enabled);
            }
        }

        if announce {
            *imp.status_override.borrow_mut() = Some(if enabled {
                "Forever Mode enabled.".to_string()
            } else {
                "Forever Mode disabled.".to_string()
            });
            self.render();
        }
    }

    pub(super) fn set_robot_ludicrous_enabled(&self, enabled: bool, persist: bool, announce: bool) {
        let imp = self.imp();
        imp.robot_ludicrous_enabled.set(enabled);

        if persist {
            if let Some(settings) = imp.settings.borrow().clone() {
                let _ = settings.set_boolean(SETTINGS_KEY_LUDICROUS_SPEED, enabled);
            }
        }

        if let Some(action) = self.lookup_action("ludicrous-speed") {
            if let Ok(action) = action.downcast::<gio::SimpleAction>() {
                let current = action
                    .state()
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(false);
                if current != enabled {
                    action.set_state(&enabled.to_variant());
                }
            }
        }

        if imp.robot_mode_running.get() {
            self.rebind_robot_mode_timer_for_current_speed();
        }

        if announce {
            *imp.status_override.borrow_mut() = Some(if enabled {
                "Ludicrous Speed enabled (Robot = 40ms/move).".to_string()
            } else {
                "Ludicrous Speed disabled.".to_string()
            });
            self.render();
        } else if persist {
            self.render();
        }
    }

    pub(super) fn set_robot_auto_new_game_on_loss_enabled(
        &self,
        enabled: bool,
        persist: bool,
        announce: bool,
    ) {
        let imp = self.imp();
        imp.robot_auto_new_game_on_loss.set(enabled);

        if persist {
            if let Some(settings) = imp.settings.borrow().clone() {
                let _ = settings.set_boolean(SETTINGS_KEY_ROBOT_AUTO_NEW_GAME_ON_LOSS, enabled);
            }
        }

        if let Some(action) = self.lookup_action("robot-auto-new-game-on-loss") {
            if let Ok(action) = action.downcast::<gio::SimpleAction>() {
                let current = action
                    .state()
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(true);
                if current != enabled {
                    action.set_state(&enabled.to_variant());
                }
            }
        }

        if announce {
            *imp.status_override.borrow_mut() = Some(if enabled {
                "Robot auto new game on loss enabled.".to_string()
            } else {
                "Robot auto new game on loss disabled.".to_string()
            });
            self.render();
        } else if persist {
            self.render();
        }
    }

    pub(super) fn set_robot_debug_enabled(&self, enabled: bool, persist: bool, announce: bool) {
        let imp = self.imp();
        imp.robot_debug_enabled.set(enabled);

        if persist {
            if let Some(settings) = imp.settings.borrow().clone() {
                let _ = settings.set_boolean(SETTINGS_KEY_ROBOT_DEBUG_ENABLED, enabled);
            }
        }

        if let Some(action) = self.lookup_action("robot-debug-toggle") {
            if let Ok(action) = action.downcast::<gio::SimpleAction>() {
                let current = action
                    .state()
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(false);
                if current != enabled {
                    action.set_state(&enabled.to_variant());
                }
            }
        }

        if announce {
            *self.imp().status_override.borrow_mut() = Some(if enabled {
                "robot_debug=on".to_string()
            } else {
                "robot_debug=off".to_string()
            });
            self.render();
        } else if persist {
            self.render();
        }
    }

    pub(super) fn set_robot_strict_debug_invariants_enabled(
        &self,
        enabled: bool,
        persist: bool,
        announce: bool,
    ) {
        let imp = self.imp();
        imp.robot_strict_debug_invariants.set(enabled);

        if persist {
            if let Some(settings) = imp.settings.borrow().clone() {
                let _ = settings.set_boolean(SETTINGS_KEY_ROBOT_STRICT_DEBUG_INVARIANTS, enabled);
            }
        }

        if let Some(action) = self.lookup_action("robot-strict-debug-invariants") {
            if let Ok(action) = action.downcast::<gio::SimpleAction>() {
                let current = action
                    .state()
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(true);
                if current != enabled {
                    action.set_state(&enabled.to_variant());
                }
            }
        }

        if announce {
            *imp.status_override.borrow_mut() = Some(if enabled {
                "Strict debug invariants enabled.".to_string()
            } else {
                "Strict debug invariants disabled.".to_string()
            });
            self.render();
        } else if persist {
            self.render();
        }
    }

    pub(super) fn smart_move_mode(&self) -> SmartMoveMode {
        self.imp().smart_move_mode.get()
    }

    pub(super) fn set_smart_move_mode(&self, mode: SmartMoveMode, persist: bool, announce: bool) {
        self.imp().smart_move_mode.set(mode);
        if persist {
            if let Some(settings) = self.imp().settings.borrow().clone() {
                let _ = settings.set_string(SETTINGS_KEY_SMART_MOVE_MODE, mode.as_setting());
            }
        }
        if announce {
            *self.imp().status_override.borrow_mut() = Some(match mode {
                SmartMoveMode::Disabled => "Smart Move disabled.".to_string(),
                SmartMoveMode::SingleClick => "Smart Move set to single click.".to_string(),
                SmartMoveMode::DoubleClick => "Smart Move set to double click.".to_string(),
                SmartMoveMode::RightClick => "Smart Move set to right click.".to_string(),
            });
        }
        if persist || announce {
            self.render();
        }
    }

    pub(super) fn robot_strategy(&self) -> RobotStrategy {
        RobotStrategy::Deep
    }

    pub(super) fn set_ephemeral_status(&self, message: impl Into<String>, duration: Duration) {
        let imp = self.imp();
        if let Some(source_id) = imp.status_ephemeral_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }

        let message = message.into();
        *imp.status_override.borrow_mut() = Some(message.clone());
        self.render();

        let source_id = glib::timeout_add_local(
            duration,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let imp = window.imp();
                    if imp.status_override.borrow().as_deref() == Some(message.as_str()) {
                        *imp.status_override.borrow_mut() = None;
                        window.render();
                    }
                    *imp.status_ephemeral_timer.borrow_mut() = None;
                    glib::ControlFlow::Break
                }
            ),
        );
        *imp.status_ephemeral_timer.borrow_mut() = Some(source_id);
    }

    pub(super) fn handle_window_geometry_change(&self) {
        let imp = self.imp();
        imp.perf_resize_event_count
            .set(imp.perf_resize_event_count.get().saturating_add(1));
        imp.geometry_render_dirty.set(true);
        if imp.geometry_render_pending.replace(true) {
            return;
        }

        glib::timeout_add_local(
            Duration::from_millis(16),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let imp = window.imp();
                    if !imp.geometry_render_dirty.replace(false) {
                        imp.geometry_render_pending.set(false);
                        return glib::ControlFlow::Break;
                    }
                    let prev_mobile = imp.mobile_phone_mode.get();
                    let prev_card_w = imp.card_width.get();
                    let prev_card_h = imp.card_height.get();
                    let prev_face_up = imp.face_up_step.get();
                    let prev_face_down = imp.face_down_step.get();
                    let prev_metrics_key = imp.last_metrics_key.get();

                    window.sync_mobile_phone_mode_to_size();
                    window.sync_hud_visibility_to_size();
                    window.update_tableau_metrics();

                    let metrics_changed = imp.last_metrics_key.get() != prev_metrics_key
                        || imp.card_width.get() != prev_card_w
                        || imp.card_height.get() != prev_card_h
                        || imp.face_up_step.get() != prev_face_up
                        || imp.face_down_step.get() != prev_face_down;
                    let mobile_changed = imp.mobile_phone_mode.get() != prev_mobile;

                    if metrics_changed || mobile_changed {
                        imp.perf_geometry_render_count
                            .set(imp.perf_geometry_render_count.get().saturating_add(1));
                        window.render();
                    }
                    if imp.geometry_render_dirty.get() {
                        return glib::ControlFlow::Continue;
                    }
                    imp.geometry_render_pending.set(false);
                    glib::ControlFlow::Break
                }
            ),
        );
    }
}

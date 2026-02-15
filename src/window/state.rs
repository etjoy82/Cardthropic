use super::*;

impl CardthropicWindow {
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
        imp.toolbar_box.set_visible(hud_enabled);

        if persist {
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
            });
        }
        if persist || announce {
            self.render();
        }
    }

    pub(super) fn handle_window_geometry_change(&self) {
        let imp = self.imp();
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
                    imp.geometry_render_pending.set(false);
                    if !imp.geometry_render_dirty.replace(false) {
                        return glib::ControlFlow::Break;
                    }
                    imp.last_metrics_key.set(0);
                    window.render();
                    if imp.geometry_render_dirty.get() {
                        window.handle_window_geometry_change();
                    }
                    glib::ControlFlow::Break
                }
            ),
        );
    }
}

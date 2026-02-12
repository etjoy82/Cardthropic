use super::*;

impl CardthropicWindow {
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

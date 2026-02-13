use super::*;

impl CardthropicWindow {
    pub(super) fn setup_geometry_handlers(&self) {
        let imp = self.imp();

        self.connect_notify_local(
            Some("width"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        self.connect_notify_local(
            Some("height"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        self.connect_notify_local(
            Some("maximized"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        imp.tableau_scroller.connect_notify_local(
            Some("width"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        imp.tableau_scroller.connect_notify_local(
            Some("height"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    window.handle_window_geometry_change();
                }
            ),
        );
        self.add_tick_callback(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |widget, _| {
                let imp = window.imp();
                let width = widget.width();
                let height = widget.height();
                let scroller_width = imp.tableau_scroller.width();
                let scroller_height = imp.tableau_scroller.height();
                let maximized = window.is_maximized();
                if width > 0
                    && height > 0
                    && (width != imp.observed_window_width.get()
                        || height != imp.observed_window_height.get())
                {
                    imp.observed_window_width.set(width);
                    imp.observed_window_height.set(height);
                    imp.observed_scroller_width.set(scroller_width);
                    imp.observed_scroller_height.set(scroller_height);
                    imp.observed_maximized.set(maximized);
                    window.handle_window_geometry_change();
                } else if scroller_width > 0
                    && scroller_height > 0
                    && (scroller_width != imp.observed_scroller_width.get()
                        || scroller_height != imp.observed_scroller_height.get()
                        || maximized != imp.observed_maximized.get())
                {
                    imp.observed_scroller_width.set(scroller_width);
                    imp.observed_scroller_height.set(scroller_height);
                    imp.observed_maximized.set(maximized);
                    window.handle_window_geometry_change();
                }
                glib::ControlFlow::Continue
            }
        ));
    }
}

use super::*;

impl CardthropicWindow {
    fn poll_window_geometry_change(&self) {
        let imp = self.imp();
        let width = self.width();
        let height = self.height();
        let maximized = self.is_maximized();

        if width > 0
            && height > 0
            && (width != imp.observed_window_width.get()
                || height != imp.observed_window_height.get())
        {
            imp.observed_window_width.set(width);
            imp.observed_window_height.set(height);
            imp.observed_maximized.set(maximized);
            imp.perf_resize_from_poll_count
                .set(imp.perf_resize_from_poll_count.get().saturating_add(1));
            self.handle_window_geometry_change();
        } else if maximized != imp.observed_maximized.get() {
            imp.observed_maximized.set(maximized);
            imp.perf_resize_from_poll_count
                .set(imp.perf_resize_from_poll_count.get().saturating_add(1));
            self.handle_window_geometry_change();
        }
    }

    pub(super) fn setup_geometry_handlers(&self) {
        self.connect_notify_local(
            Some("width"),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _| {
                    let imp = window.imp();
                    imp.perf_resize_from_notify_width_count.set(
                        imp.perf_resize_from_notify_width_count
                            .get()
                            .saturating_add(1),
                    );
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
                    let imp = window.imp();
                    imp.perf_resize_from_notify_height_count.set(
                        imp.perf_resize_from_notify_height_count
                            .get()
                            .saturating_add(1),
                    );
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
                    let imp = window.imp();
                    imp.perf_resize_from_notify_maximized_count.set(
                        imp.perf_resize_from_notify_maximized_count
                            .get()
                            .saturating_add(1),
                    );
                    window.handle_window_geometry_change();
                }
            ),
        );
        // Use frame-clock ticks for geometry polling to avoid feedback loops
        // from child-widget allocation notifications during horizontal resize.
        self.add_tick_callback(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::ControlFlow::Break,
            move |_, _| {
                window.poll_window_geometry_change();
                glib::ControlFlow::Continue
            }
        ));
    }
}

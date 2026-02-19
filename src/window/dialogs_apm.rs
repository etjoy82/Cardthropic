use super::*;

impl CardthropicWindow {
    fn apm_samples_for_graph(&self) -> Vec<ApmSample> {
        let imp = self.imp();
        let mut points = imp.apm_samples.borrow().clone();
        let elapsed = imp.elapsed_seconds.get();
        if elapsed > 0 {
            let current = ApmSample {
                elapsed_seconds: self.current_apm_timeline_seconds(),
                apm: self.current_apm(),
            };
            if points
                .last()
                .map(|last| last.elapsed_seconds == current.elapsed_seconds)
                .unwrap_or(false)
            {
                if let Some(last) = points.last_mut() {
                    *last = current;
                }
            } else {
                points.push(current);
            }
        }
        points
    }

    fn draw_apm_graph(&self, cr: &gtk::cairo::Context, width: i32, height: i32) {
        let w = f64::from(width.max(1));
        let h = f64::from(height.max(1));
        cr.set_source_rgba(0.12, 0.14, 0.17, 1.0);
        cr.set_source_rgba(0.12, 0.14, 0.17, 1.0);
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();

        let left = 48.0;
        let right = 14.0;
        let top = 16.0;
        let bottom = 30.0;
        let plot_w = (w - left - right).max(1.0);
        let plot_h = (h - top - bottom).max(1.0);

        cr.set_source_rgba(1.0, 1.0, 1.0, 0.10);
        cr.rectangle(left, top, plot_w, plot_h);
        let _ = cr.stroke();

        let points = self.apm_samples_for_graph();
        let max_t = points.last().map(|p| p.elapsed_seconds.max(1)).unwrap_or(1) as f64;
        let max_apm = points
            .iter()
            .fold(1.0_f64, |acc, p| acc.max(p.apm))
            .max(5.0)
            .ceil();

        cr.set_source_rgba(1.0, 1.0, 1.0, 0.22);
        for i in 1..=4 {
            let y = top + (plot_h * f64::from(i) / 4.0);
            cr.move_to(left, y);
            cr.line_to(left + plot_w, y);
            let _ = cr.stroke();
        }
        for i in 1..=3 {
            let x = left + (plot_w * f64::from(i) / 4.0);
            cr.move_to(x, top);
            cr.line_to(x, top + plot_h);
            let _ = cr.stroke();
        }

        if !points.is_empty() {
            cr.set_source_rgba(0.35, 0.75, 1.0, 0.95);
            for (i, p) in points.iter().enumerate() {
                let x = left + ((p.elapsed_seconds as f64 / max_t) * plot_w);
                let y = top + (1.0 - (p.apm / max_apm).clamp(0.0, 1.0)) * plot_h;
                if i == 0 {
                    cr.move_to(x, y);
                } else {
                    cr.line_to(x, y);
                }
            }
            if points.len() >= 2 {
                let _ = cr.stroke();
            }

            if let Some(last) = points.last() {
                let x = left + ((last.elapsed_seconds as f64 / max_t) * plot_w);
                let y = top + (1.0 - (last.apm / max_apm).clamp(0.0, 1.0)) * plot_h;
                cr.arc(x, y, 3.5, 0.0, std::f64::consts::TAU);
                let _ = cr.fill();
            }
        } else {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.75);
            cr.move_to(left + 8.0, top + 22.0);
            let _ = cr.show_text("APM graph starts plotting immediately after your first move.");
        }

        cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
        for i in 0..=4 {
            let x = left + (plot_w * f64::from(i) / 4.0);
            let t = (max_t * f64::from(i) / 4.0).round() as u32;
            cr.move_to(x - 10.0, h - 10.0);
            let _ = cr.show_text(&format!("{t}s"));
        }
        cr.move_to(6.0, top + 4.0);
        let _ = cr.show_text(&format!("{max_apm:.0} APM"));
    }

    fn apm_summary(&self, points: &[ApmSample]) -> (f64, f64) {
        if points.is_empty() {
            return (0.0, 0.0);
        }
        let peak = points.iter().fold(0.0_f64, |acc, p| acc.max(p.apm));
        let avg = points.iter().map(|p| p.apm).sum::<f64>() / points.len() as f64;
        (peak, avg)
    }

    fn apm_tilt_badge(avg_apm: f64) -> &'static str {
        if avg_apm < 15.0 {
            "Calm"
        } else if avg_apm < 30.0 {
            "Focused"
        } else if avg_apm < 45.0 {
            "Turbo"
        } else {
            "Goblin Mode"
        }
    }

    fn apm_csv_string(&self) -> String {
        let points = self.apm_samples_for_graph();
        let mut rows = Vec::with_capacity(points.len() + 1);
        rows.push("elapsed_seconds,apm".to_string());
        rows.extend(
            points
                .iter()
                .map(|sample| format!("{},{}", sample.elapsed_seconds, sample.apm)),
        );
        rows.join("\n")
    }

    fn copy_apm_data_to_clipboard(&self) {
        if let Some(display) = gdk::Display::default() {
            let clipboard = display.clipboard();
            clipboard.set_text(&self.apm_csv_string());
            *self.imp().status_override.borrow_mut() =
                Some("Copied APM data to clipboard.".to_string());
            self.render();
        }
    }

    pub(super) fn update_apm_graph_chrome(&self) {
        let imp = self.imp();
        let peak_label = imp.apm_peak_label.borrow().clone();
        let avg_label = imp.apm_avg_label.borrow().clone();
        let tilt_label = imp.apm_tilt_label.borrow().clone();
        if peak_label.is_none() && avg_label.is_none() && tilt_label.is_none() {
            return;
        }

        let points = self.apm_samples_for_graph();
        let (peak, avg) = self.apm_summary(&points);

        if let Some(label) = peak_label {
            label.set_label(&format!("Peak APM: {:.1}", peak));
        }
        if let Some(label) = avg_label {
            label.set_label(&format!("Average APM: {:.1}", avg));
        }
        if let Some(label) = tilt_label {
            label.set_label(&format!("Tilt: {}", Self::apm_tilt_badge(avg)));
        }
    }

    fn apm_graph_window_size(&self) -> (i32, i32) {
        const DEFAULT_WIDTH: i32 = 640;
        const DEFAULT_HEIGHT: i32 = 360;
        const MIN_WIDTH: i32 = 360;
        const MIN_HEIGHT: i32 = 220;

        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        };
        let Some(schema) = settings.settings_schema() else {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        };
        if !schema.has_key(SETTINGS_KEY_APM_GRAPH_WIDTH)
            || !schema.has_key(SETTINGS_KEY_APM_GRAPH_HEIGHT)
        {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        }

        (
            settings.int(SETTINGS_KEY_APM_GRAPH_WIDTH).max(MIN_WIDTH),
            settings.int(SETTINGS_KEY_APM_GRAPH_HEIGHT).max(MIN_HEIGHT),
        )
    }

    fn apm_graph_maximized(&self) -> bool {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return false;
        };
        let Some(schema) = settings.settings_schema() else {
            return false;
        };
        if !schema.has_key(SETTINGS_KEY_APM_GRAPH_MAXIMIZED) {
            return false;
        }
        settings.boolean(SETTINGS_KEY_APM_GRAPH_MAXIMIZED)
    }

    fn persist_apm_graph_maximized(&self, maximized: bool) {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_APM_GRAPH_MAXIMIZED) {
            return;
        }
        if settings.boolean(SETTINGS_KEY_APM_GRAPH_MAXIMIZED) != maximized {
            let _ = settings.set_boolean(SETTINGS_KEY_APM_GRAPH_MAXIMIZED, maximized);
        }
    }

    fn persist_apm_graph_window_size(&self, dialog: &gtk::Window) {
        const MIN_WIDTH: i32 = 360;
        const MIN_HEIGHT: i32 = 220;

        if dialog.is_maximized() {
            return;
        }

        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_APM_GRAPH_WIDTH)
            || !schema.has_key(SETTINGS_KEY_APM_GRAPH_HEIGHT)
        {
            return;
        }

        let width = dialog.width().max(MIN_WIDTH);
        let height = dialog.height().max(MIN_HEIGHT);
        if settings.int(SETTINGS_KEY_APM_GRAPH_WIDTH) != width {
            let _ = settings.set_int(SETTINGS_KEY_APM_GRAPH_WIDTH, width);
        }
        if settings.int(SETTINGS_KEY_APM_GRAPH_HEIGHT) != height {
            let _ = settings.set_int(SETTINGS_KEY_APM_GRAPH_HEIGHT, height);
        }
    }

    pub(super) fn show_apm_graph_dialog(&self) {
        if let Some(existing) = self.imp().apm_graph_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let (saved_width, saved_height) = self.apm_graph_window_size();
        let saved_maximized = self.apm_graph_maximized();
        let dialog = gtk::Window::builder()
            .title("APM Graph")
            .transient_for(self)
            .modal(false)
            .default_width(saved_width)
            .default_height(saved_height)
            .build();
        dialog.set_destroy_with_parent(true);
        dialog.set_hide_on_close(false);
        dialog.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |dialog| {
                let maximized = dialog.is_maximized();
                window.persist_apm_graph_maximized(maximized);
                window.persist_apm_graph_window_size(dialog);
                *window.imp().apm_graph_dialog.borrow_mut() = None;
                *window.imp().apm_graph_area.borrow_mut() = None;
                *window.imp().apm_peak_label.borrow_mut() = None;
                *window.imp().apm_avg_label.borrow_mut() = None;
                *window.imp().apm_tilt_label.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));
        if saved_maximized {
            dialog.maximize();
        }
        let dialog_keys = gtk::EventControllerKey::new();
        dialog_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        dialog_keys.connect_key_pressed(glib::clone!(
            #[weak]
            dialog,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    dialog.close();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        dialog.add_controller(dialog_keys);

        let stats_row = gtk::Box::new(gtk::Orientation::Horizontal, 12);
        let peak_label = gtk::Label::new(None);
        peak_label.set_xalign(0.0);
        let avg_label = gtk::Label::new(None);
        avg_label.set_xalign(0.0);
        let tilt_label = gtk::Label::new(None);
        tilt_label.set_xalign(0.0);
        tilt_label.add_css_class("accent");
        stats_row.append(&peak_label);
        stats_row.append(&avg_label);
        stats_row.append(&tilt_label);

        let graph = gtk::DrawingArea::new();
        graph.set_content_width(620);
        graph.set_content_height(320);
        graph.set_hexpand(true);
        graph.set_vexpand(true);
        graph.set_draw_func(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, cr, width, height| {
                window.draw_apm_graph(cr, width, height);
            }
        ));

        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.set_margin_top(10);
        root.set_margin_bottom(10);
        root.set_margin_start(10);
        root.set_margin_end(10);
        root.append(&stats_row);
        root.append(&graph);

        let actions_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions_row.set_halign(gtk::Align::End);
        let copy_button = gtk::Button::with_label("Copy Data");
        copy_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.copy_apm_data_to_clipboard();
            }
        ));
        actions_row.append(&copy_button);
        root.append(&actions_row);
        dialog.set_child(Some(&root));

        *self.imp().apm_peak_label.borrow_mut() = Some(peak_label);
        *self.imp().apm_avg_label.borrow_mut() = Some(avg_label);
        *self.imp().apm_tilt_label.borrow_mut() = Some(tilt_label);
        *self.imp().apm_graph_area.borrow_mut() = Some(graph);
        *self.imp().apm_graph_dialog.borrow_mut() = Some(dialog.clone());
        self.update_apm_graph_chrome();
        dialog.present();
    }
}

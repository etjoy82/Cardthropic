use super::*;

impl CardthropicWindow {
    fn accel_suffix_for_action(&self, action_name: &str) -> String {
        let Some(app) = self.application() else {
            return String::new();
        };
        let accels = app.accels_for_action(action_name);
        if accels.is_empty() {
            return String::new();
        }

        let labels: Vec<String> = accels
            .iter()
            .map(|accel| {
                if let Some((key, mods)) = gtk::accelerator_parse(accel) {
                    gtk::accelerator_get_label(key, mods).to_string()
                } else {
                    accel.to_string()
                }
            })
            .collect();
        format!(" ({})", labels.join(", "))
    }

    fn help_entries(&self) -> Vec<(String, String)> {
        let imp = self.imp();
        let mut rows = Vec::new();
        let mut push = |icon: &str, text: Option<String>, action: Option<&str>| {
            if let Some(text) = text {
                let suffix = action
                    .map(|name| self.accel_suffix_for_action(name))
                    .unwrap_or_default();
                rows.push((icon.to_string(), format!("{text}{suffix}")));
            }
        };

        push(
            "❓",
            imp.help_button.tooltip_text().map(|s| s.to_string()),
            Some("win.help"),
        );
        push(
            "⛶",
            imp.fullscreen_button.tooltip_text().map(|s| s.to_string()),
            Some("win.toggle-fullscreen"),
        );
        push(
            "↶",
            imp.undo_button.tooltip_text().map(|s| s.to_string()),
            Some("win.undo"),
        );
        push(
            "↷",
            imp.redo_button.tooltip_text().map(|s| s.to_string()),
            Some("win.redo"),
        );
        push(
            "🪄",
            imp.auto_hint_button.tooltip_text().map(|s| s.to_string()),
            Some("win.play-hint-move"),
        );
        push("⚡", Some("Rapid Wand".to_string()), Some("win.rapid-wand"));
        push(
            "🌀",
            imp.cyclone_shuffle_button
                .tooltip_text()
                .map(|s| s.to_string()),
            Some("win.cyclone-shuffle"),
        );
        push(
            "🫣",
            imp.peek_button.tooltip_text().map(|s| s.to_string()),
            Some("win.peek"),
        );
        push(
            "🤖",
            imp.robot_button.tooltip_text().map(|s| s.to_string()),
            Some("win.robot-mode"),
        );
        push(
            "🎨",
            imp.board_color_menu_button
                .tooltip_text()
                .map(|s| s.to_string()),
            None,
        );
        push(
            imp.game_settings_menu_button
                .label()
                .as_deref()
                .unwrap_or("🎮"),
            imp.game_settings_menu_button
                .tooltip_text()
                .map(|s| s.to_string()),
            None,
        );
        push(
            "☰",
            imp.main_menu_button.tooltip_text().map(|s| s.to_string()),
            None,
        );
        push(
            "🎲",
            imp.seed_random_button.tooltip_text().map(|s| s.to_string()),
            Some("win.random-seed"),
        );
        push(
            "🛟",
            imp.seed_rescue_button.tooltip_text().map(|s| s.to_string()),
            Some("win.winnable-seed"),
        );
        push(
            "W?",
            imp.seed_winnable_button
                .tooltip_text()
                .map(|s| s.to_string()),
            None,
        );
        push(
            "🔁",
            imp.seed_repeat_button.tooltip_text().map(|s| s.to_string()),
            None,
        );
        push(
            "Go",
            imp.seed_go_button.tooltip_text().map(|s| s.to_string()),
            None,
        );
        push(
            "📈",
            Some("Show APM graph".to_string()),
            Some("win.apm-graph"),
        );

        rows
    }

    pub(super) fn show_help_dialog(&self) {
        if let Some(existing) = self.imp().help_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let dialog = gtk::Window::builder()
            .title("Cardthropic Help")
            .transient_for(self)
            .modal(false)
            .default_width(520)
            .default_height(480)
            .build();
        dialog.set_hide_on_close(true);
        dialog.set_destroy_with_parent(true);

        let key_controller = gtk::EventControllerKey::new();
        key_controller.connect_key_pressed(glib::clone!(
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
        dialog.add_controller(key_controller);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(14);
        root.set_margin_bottom(14);
        root.set_margin_start(14);
        root.set_margin_end(14);

        let title = gtk::Label::new(Some("Controls"));
        title.set_xalign(0.0);
        title.add_css_class("title-4");
        root.append(&title);

        let scrolled = gtk::ScrolledWindow::builder()
            .hscrollbar_policy(gtk::PolicyType::Never)
            .min_content_width(420)
            .min_content_height(300)
            .vexpand(true)
            .build();
        let content = gtk::Box::new(gtk::Orientation::Vertical, 6);
        for (icon, text) in self.help_entries() {
            let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
            let icon_label = gtk::Label::new(Some(&icon));
            icon_label.set_width_chars(4);
            icon_label.set_xalign(0.0);
            let text_label = gtk::Label::new(Some(&text));
            text_label.set_xalign(0.0);
            text_label.set_wrap(true);
            text_label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
            text_label.set_hexpand(true);
            row.append(&icon_label);
            row.append(&text_label);
            content.append(&row);
        }
        scrolled.set_child(Some(&content));
        root.append(&scrolled);

        let close = gtk::Button::with_label("Close");
        close.set_halign(gtk::Align::End);
        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        root.append(&close);

        dialog.set_child(Some(&root));
        *self.imp().help_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }

    pub(super) fn toggle_fullscreen_mode(&self) {
        if self.is_fullscreen() {
            self.unfullscreen();
            *self.imp().status_override.borrow_mut() = Some("Exited fullscreen.".to_string());
        } else {
            self.fullscreen();
            *self.imp().status_override.borrow_mut() = Some("Entered fullscreen.".to_string());
        }
        self.render();
    }

    fn apm_samples_for_graph(&self) -> Vec<ApmSample> {
        let imp = self.imp();
        let mut points = imp.apm_samples.borrow().clone();
        let elapsed = imp.elapsed_seconds.get();
        if elapsed > 0 {
            let current = ApmSample {
                elapsed_seconds: elapsed,
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
        if points.len() < 2 {
            cr.set_source_rgba(1.0, 1.0, 1.0, 0.75);
            cr.move_to(left + 8.0, top + 22.0);
            let _ = cr.show_text("Play for at least 10 seconds to plot APM.");
            return;
        }

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
        let _ = cr.stroke();

        if let Some(last) = points.last() {
            let x = left + ((last.elapsed_seconds as f64 / max_t) * plot_w);
            let y = top + (1.0 - (last.apm / max_apm).clamp(0.0, 1.0)) * plot_h;
            cr.arc(x, y, 3.5, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
        }

        cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
        cr.move_to(left, h - 10.0);
        let _ = cr.show_text("0s");
        cr.move_to(left + plot_w - 36.0, h - 10.0);
        let _ = cr.show_text(&format!("{max_t:.0}s"));
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

    pub(super) fn show_apm_graph_dialog(&self) {
        if let Some(existing) = self.imp().apm_graph_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let dialog = gtk::Window::builder()
            .title("APM Graph")
            .transient_for(self)
            .modal(false)
            .default_width(640)
            .default_height(360)
            .build();
        dialog.set_destroy_with_parent(true);
        dialog.set_hide_on_close(true);

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

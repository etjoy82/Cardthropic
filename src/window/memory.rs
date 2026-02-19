use super::*;
use std::fs;

fn read_status_kib(field: &str) -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix(field))
        .and_then(parse_kib_line)
}

fn read_smaps_rollup_kib(field: &str) -> Option<u64> {
    let smaps = fs::read_to_string("/proc/self/smaps_rollup").ok()?;
    smaps
        .lines()
        .find_map(|line| line.strip_prefix(field))
        .and_then(parse_kib_line)
}

fn parse_kib_line(line: &str) -> Option<u64> {
    line.split_whitespace().next()?.parse::<u64>().ok()
}

impl CardthropicWindow {
    const MEMORY_GUARD_DIALOG_COOLDOWN_US: i64 = 120_000_000;

    fn should_present_memory_guard_dialog(&self) -> bool {
        let imp = self.imp();
        let now = glib::monotonic_time();
        let last = imp.memory_guard_last_dialog_mono_us.get();
        if last > 0 && now.saturating_sub(last) < Self::MEMORY_GUARD_DIALOG_COOLDOWN_US {
            return false;
        }
        imp.memory_guard_last_dialog_mono_us.set(now);
        true
    }

    fn show_memory_guard_warning_dialog(&self) {
        if !self.should_present_memory_guard_dialog() {
            return;
        }
        if let Some(existing) = self.imp().memory_guard_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let dialog = gtk::Window::builder()
            .title("High Memory Use")
            .modal(true)
            .transient_for(self)
            .default_width(460)
            .default_height(140)
            .build();
        dialog.set_resizable(false);
        dialog.set_destroy_with_parent(true);

        dialog.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                *window.imp().memory_guard_dialog.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(14);
        root.set_margin_bottom(14);
        root.set_margin_start(14);
        root.set_margin_end(14);

        let label = gtk::Label::new(Some(
            "Carthropic's memory use in its current state is high. Please consider restarting Carthropic to reset memory use. Your game session is still saved.",
        ));
        label.set_wrap(true);
        label.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        label.set_xalign(0.0);
        root.append(&label);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);
        let ok = gtk::Button::with_label("OK");
        ok.add_css_class("suggested-action");
        ok.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        actions.append(&ok);
        root.append(&actions);

        dialog.set_child(Some(&root));
        *self.imp().memory_guard_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }

    pub(super) fn configure_memory_guard(
        &self,
        enabled: bool,
        soft_limit_mib: u64,
        hard_limit_mib: u64,
    ) {
        let imp = self.imp();
        imp.memory_guard_enabled.set(enabled);
        imp.memory_guard_soft_limit_mib.set(soft_limit_mib);
        imp.memory_guard_hard_limit_mib.set(hard_limit_mib);
        imp.memory_guard_soft_triggered.set(false);
        imp.memory_guard_hard_triggered.set(false);
    }

    pub(super) fn enforce_memory_guard_if_needed(&self) {
        let imp = self.imp();
        if !imp.memory_guard_enabled.get() || imp.memory_guard_hard_triggered.get() {
            return;
        }
        let Some(mem_mib) = self.current_memory_mib() else {
            return;
        };

        let hard = imp.memory_guard_hard_limit_mib.get();
        if hard > 0 && mem_mib >= hard {
            imp.memory_guard_hard_triggered.set(true);
            self.stop_rapid_wand();
            self.stop_robot_mode();
            self.cancel_seed_winnable_check(None);
            self.append_status_history_only(&format!(
                "memory_guard event=hard rss_mib={} hard_limit_mib={} action=quit",
                mem_mib, hard
            ));
            *imp.status_override.borrow_mut() = Some(format!(
                "Memory guard: {} MiB >= hard limit {} MiB. Saving session and exiting.",
                mem_mib, hard
            ));
            self.render();
            self.flush_session_now();
            if let Some(app) = self.application() {
                app.quit();
            } else {
                self.close();
            }
            return;
        }

        let soft = imp.memory_guard_soft_limit_mib.get();
        if soft > 0 && mem_mib >= soft {
            if !imp.memory_guard_soft_triggered.replace(true) {
                self.stop_rapid_wand();
                if imp.robot_mode_running.get() {
                    self.stop_robot_mode_with_message(
                        "Robot Mode stopped: memory guard soft limit reached.",
                    );
                }
                self.cancel_seed_winnable_check(None);
                self.trim_process_memory_if_supported();
                self.append_status_history_only(&format!(
                    "memory_guard event=soft rss_mib={} soft_limit_mib={} action=stop_automation",
                    mem_mib, soft
                ));
                *imp.status_override.borrow_mut() = Some(format!(
                    "Memory guard: {} MiB >= soft limit {} MiB. Automation stopped.",
                    mem_mib, soft
                ));
                self.render();
                self.show_memory_guard_warning_dialog();
            }
        } else if imp.memory_guard_soft_triggered.get() {
            // Reset after memory has recovered to avoid permanent latch.
            let hysteresis = soft.saturating_sub((soft / 10).max(1));
            if mem_mib < hysteresis {
                imp.memory_guard_soft_triggered.set(false);
            }
        }
    }

    pub(super) fn current_memory_mib(&self) -> Option<u64> {
        let kib = read_status_kib("RssAnon:")
            .or_else(|| read_smaps_rollup_kib("RssAnon:"))
            .or_else(|| read_status_kib("VmRSS:"))?;
        Some((kib as f64 / 1024.0).round() as u64)
    }

    pub(super) fn current_memory_mib_text(&self) -> String {
        // GNOME System Monitor's "Memory" for processes maps closer to private
        // resident usage than full RSS. RssAnon tracks that best for us.
        match self.current_memory_mib() {
            Some(mib) => format!("{mib} MiB"),
            None => "n/a".to_string(),
        }
    }

    pub(super) fn trim_process_memory_if_supported(&self) {
        #[cfg(all(target_os = "linux", target_env = "gnu"))]
        unsafe {
            unsafe extern "C" {
                fn malloc_trim(pad: usize) -> i32;
            }
            let _ = malloc_trim(0);
        }
    }
}

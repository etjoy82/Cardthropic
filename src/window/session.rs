use super::*;
use crate::engine::boundary;
use crate::engine::game_mode::VariantRuntime;
use crate::engine::session::{decode_persisted_session, encode_persisted_session};
use crate::engine::variant_state::VariantStateStore;
use crate::startup_trace;

impl CardthropicWindow {
    const SESSION_FLUSH_INTERVAL_SECS: u32 = 3;
    const SESSION_FLUSH_INTERVAL_AUTOMATION_SECS: u32 = 30;
    const MAX_PERSISTED_SNAPSHOTS: usize = 200;

    fn session_flush_interval_secs(&self) -> u32 {
        let imp = self.imp();
        if imp.robot_mode_running.get()
            && imp.robot_ludicrous_enabled.get()
            && imp.robot_forever_enabled.get()
            && imp.robot_auto_new_game_on_loss.get()
        {
            return Self::SESSION_FLUSH_INTERVAL_AUTOMATION_SECS;
        }
        Self::SESSION_FLUSH_INTERVAL_SECS
    }

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
            self.mark_session_dirty();
            if let Some(area) = imp.apm_graph_area.borrow().as_ref() {
                area.queue_draw();
            }
            self.update_apm_graph_chrome();
        }
        // Keep Mem in the stats row live even when gameplay timer is stopped.
        self.update_stats_label();
        self.enforce_memory_guard_if_needed();
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

    pub(super) fn current_apm_timeline_seconds(&self) -> u32 {
        let imp = self.imp();
        imp.apm_elapsed_offset_seconds
            .get()
            .saturating_add(imp.elapsed_seconds.get())
    }

    pub(super) fn roll_apm_timeline_forward(&self) {
        let imp = self.imp();
        let elapsed = imp.elapsed_seconds.get();
        if elapsed == 0 {
            return;
        }
        imp.apm_elapsed_offset_seconds
            .set(imp.apm_elapsed_offset_seconds.get().saturating_add(elapsed));
    }

    fn record_apm_sample_if_due(&self) {
        let imp = self.imp();
        let elapsed = imp.elapsed_seconds.get();
        if elapsed == 0 || elapsed % 5 != 0 {
            return;
        }
        let elapsed_absolute = self.current_apm_timeline_seconds();
        let mut samples = imp.apm_samples.borrow_mut();
        if samples
            .last()
            .map(|sample| sample.elapsed_seconds == elapsed_absolute)
            .unwrap_or(false)
        {
            return;
        }
        samples.push(ApmSample {
            elapsed_seconds: elapsed_absolute,
            apm: self.current_apm(),
        });
    }

    fn encode_snapshot_runtime(runtime: &VariantRuntime) -> String {
        match runtime {
            VariantRuntime::Klondike(game) => format!("k:{}", game.encode_for_session()),
            VariantRuntime::Spider(game) => format!("s:{}", game.encode_for_session()),
            VariantRuntime::Freecell(game) => format!("f:{}", game.encode_for_session()),
        }
    }

    fn encode_apm_samples(samples: &[ApmSample]) -> String {
        if samples.is_empty() {
            return "-".to_string();
        }
        samples
            .iter()
            .map(|sample| format!("{}:{:.3}", sample.elapsed_seconds, sample.apm))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn decode_apm_samples(raw: &str) -> Option<Vec<ApmSample>> {
        if raw == "-" || raw.is_empty() {
            return Some(Vec::new());
        }
        let mut out = Vec::new();
        for token in raw.split(',') {
            let (elapsed, apm) = token.split_once(':')?;
            out.push(ApmSample {
                elapsed_seconds: elapsed.parse::<u32>().ok()?,
                apm: apm.parse::<f64>().ok()?,
            });
        }
        Some(out)
    }

    fn encode_selected_run(selected: Option<SelectedRun>) -> String {
        match selected {
            Some(run) => format!("{},{}", run.col, run.start),
            None => "-".to_string(),
        }
    }

    fn decode_selected_run(raw: &str) -> Option<Option<SelectedRun>> {
        if raw == "-" || raw.is_empty() {
            return Some(None);
        }
        let (col, start) = raw.split_once(',')?;
        Some(Some(SelectedRun {
            col: col.parse::<usize>().ok()?,
            start: start.parse::<usize>().ok()?,
        }))
    }

    fn hex_encode(input: &str) -> String {
        let mut out = String::with_capacity(input.len() * 2);
        for b in input.as_bytes() {
            use std::fmt::Write as _;
            let _ = write!(&mut out, "{:02x}", b);
        }
        out
    }

    fn hex_decode(input: &str) -> Option<String> {
        if input.len() % 2 != 0 {
            return None;
        }
        let mut bytes = Vec::with_capacity(input.len() / 2);
        let mut i = 0;
        while i < input.len() {
            let byte = u8::from_str_radix(&input[i..i + 2], 16).ok()?;
            bytes.push(byte);
            i += 2;
        }
        String::from_utf8(bytes).ok()
    }

    fn encode_snapshot(snapshot: &Snapshot) -> String {
        let mode = snapshot.mode.id();
        let draw = snapshot.draw_mode.count();
        let selected = Self::encode_selected_run(snapshot.selected_run);
        let waste = if snapshot.selected_waste { 1 } else { 0 };
        let timer = if snapshot.timer_started { 1 } else { 0 };
        let runtime = Self::hex_encode(&Self::encode_snapshot_runtime(&snapshot.runtime));
        let apm = Self::encode_apm_samples(&snapshot.apm_samples);
        let foundation_slots = snapshot
            .foundation_slot_suits
            .iter()
            .map(|slot| match slot {
                Some(Suit::Clubs) => 'C',
                Some(Suit::Diamonds) => 'D',
                Some(Suit::Hearts) => 'H',
                Some(Suit::Spades) => 'S',
                None => '-',
            })
            .collect::<String>();
        format!(
            "mode={mode};draw={draw};selected={selected};waste={waste};moves={};elapsed={};timer={timer};apm_offset={};runtime_hex={runtime};apm={apm};fslots={foundation_slots}",
            snapshot.move_count, snapshot.elapsed_seconds, snapshot.apm_elapsed_offset_seconds
        )
    }

    fn decode_snapshot(encoded: &str) -> Option<Snapshot> {
        let mut fields = std::collections::HashMap::<&str, &str>::new();
        for part in encoded.split(';') {
            let (key, value) = part.split_once('=')?;
            fields.insert(key.trim(), value.trim());
        }

        let mode = GameMode::from_id(fields.get("mode")?)?;
        let draw_mode = DrawMode::from_count(fields.get("draw")?.parse::<u8>().ok()?)?;
        let selected_run = Self::decode_selected_run(fields.get("selected")?)?;
        let selected_waste = match *fields.get("waste")? {
            "1" => true,
            "0" => false,
            _ => return None,
        };
        let move_count = fields.get("moves")?.parse::<u32>().ok()?;
        let elapsed_seconds = fields.get("elapsed")?.parse::<u32>().ok()?;
        let timer_started = match *fields.get("timer")? {
            "1" => true,
            "0" => false,
            _ => return None,
        };
        let apm_elapsed_offset_seconds = fields
            .get("apm_offset")
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(0);
        let runtime_encoded = Self::hex_decode(fields.get("runtime_hex")?)?;
        let runtime = VariantStateStore::decode_runtime_for_session(mode, &runtime_encoded)?;
        let apm_samples = Self::decode_apm_samples(fields.get("apm")?)?;
        let foundation_slot_suits = fields
            .get("fslots")
            .and_then(|raw| {
                if raw.len() != 4 {
                    return None;
                }
                let mut out = [None, None, None, None];
                for (idx, ch) in raw.chars().enumerate() {
                    out[idx] = match ch {
                        'C' => Some(Suit::Clubs),
                        'D' => Some(Suit::Diamonds),
                        'H' => Some(Suit::Hearts),
                        'S' => Some(Suit::Spades),
                        '-' => None,
                        _ => return None,
                    };
                }
                Some(out)
            })
            .unwrap_or([None, None, None, None]);

        Some(Snapshot {
            mode,
            runtime,
            draw_mode,
            selected_run,
            selected_waste,
            move_count,
            elapsed_seconds,
            timer_started,
            apm_elapsed_offset_seconds,
            apm_samples,
            foundation_slot_suits,
        })
    }

    fn encode_snapshot_stack(stack: &[Snapshot]) -> String {
        if stack.is_empty() {
            return "-".to_string();
        }
        let len = stack.len();
        let start = len.saturating_sub(Self::MAX_PERSISTED_SNAPSHOTS);
        stack[start..]
            .iter()
            .map(|snapshot| Self::hex_encode(&Self::encode_snapshot(snapshot)))
            .collect::<Vec<_>>()
            .join(",")
    }

    fn decode_snapshot_stack(raw: &str) -> Option<Vec<Snapshot>> {
        if raw.is_empty() || raw == "-" {
            return Some(Vec::new());
        }
        let mut out = Vec::new();
        for token in raw.split(',') {
            let decoded = Self::hex_decode(token)?;
            out.push(Self::decode_snapshot(&decoded)?);
        }
        Some(out)
    }

    fn payload_field<'a>(raw: &'a str, key: &str) -> Option<&'a str> {
        for line in raw.lines() {
            let (k, v) = line.split_once('=')?;
            if k.trim() == key {
                return Some(v.trim());
            }
        }
        None
    }

    pub(super) fn build_saved_session(&self) -> String {
        let imp = self.imp();
        let mode = self.active_game_mode();
        let draw_mode = imp.klondike_draw_mode.get();
        let game = imp.game.borrow();
        let timer_started = imp.timer_started.get() && !boundary::is_won(&game, mode);
        let mut payload = encode_persisted_session(
            &game,
            imp.current_seed.get(),
            mode,
            imp.move_count.get(),
            imp.elapsed_seconds.get(),
            timer_started,
            draw_mode,
        );
        let history_encoded = Self::encode_snapshot_stack(&imp.history.borrow());
        let future_encoded = Self::encode_snapshot_stack(&imp.future.borrow());
        payload.push_str("\nhistory=");
        payload.push_str(&history_encoded);
        payload.push_str("\nfuture=");
        payload.push_str(&future_encoded);
        payload
    }

    fn persist_session_if_changed(&self) {
        if !self.should_persist_shared_state() {
            return;
        }
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

    pub(super) fn mark_session_dirty(&self) {
        let imp = self.imp();
        imp.session_dirty.set(true);
        if imp.session_flush_timer.borrow().is_some() {
            return;
        }

        let timer = glib::timeout_add_seconds_local(
            self.session_flush_interval_secs(),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let imp = window.imp();
                    if !imp.session_dirty.get() {
                        imp.session_flush_timer.borrow_mut().take();
                        return glib::ControlFlow::Break;
                    }

                    window.persist_session_if_changed();
                    imp.session_dirty.set(false);
                    imp.session_flush_timer.borrow_mut().take();
                    glib::ControlFlow::Break
                }
            ),
        );
        *imp.session_flush_timer.borrow_mut() = Some(timer);
    }

    pub(super) fn flush_session_now(&self) {
        let imp = self.imp();
        if let Some(source_id) = imp.session_flush_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        self.persist_session_if_changed();
        imp.session_dirty.set(false);
    }

    pub(super) fn try_restore_saved_session(&self) -> bool {
        if !self.should_persist_shared_state() {
            return false;
        }
        startup_trace::mark("session:restore-start");
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings else {
            startup_trace::mark("session:restore-end");
            return false;
        };
        let raw = settings.string(SETTINGS_KEY_SAVED_SESSION).to_string();
        if raw.trim().is_empty() {
            startup_trace::mark("session:restore-end");
            return false;
        }
        let restored = if self
            .restore_session_from_payload(&raw, "Resumed previous game.", false)
            .is_ok()
        {
            true
        } else {
            let _ = settings.set_string(SETTINGS_KEY_SAVED_SESSION, "");
            false
        };
        startup_trace::mark("session:restore-end");
        restored
    }

    pub(super) fn restore_session_from_payload(
        &self,
        raw: &str,
        status_message: &str,
        persist_payload: bool,
    ) -> Result<(), String> {
        let Some(session) = decode_persisted_session(raw) else {
            return Err("payload is not a valid Cardthropic game state".to_string());
        };

        let imp = self.imp();
        self.stop_rapid_wand();
        self.stop_robot_mode();
        self.cancel_seed_winnable_check(None);
        imp.game.borrow_mut().set_runtime(session.runtime.clone());
        imp.current_seed.set(session.seed);
        self.roll_apm_timeline_forward();
        imp.move_count.set(session.move_count);
        imp.elapsed_seconds.set(session.elapsed_seconds);
        imp.timer_started.set(session.timer_started);
        *imp.selected_run.borrow_mut() = None;
        imp.selected_freecell.set(None);
        imp.waste_selected.set(false);
        imp.history.borrow_mut().clear();
        imp.future.borrow_mut().clear();
        imp.current_game_mode.set(session.mode);
        imp.klondike_draw_mode.set(session.klondike_draw_mode);
        imp.freecell_card_count_mode
            .set(session.freecell_card_count_mode);
        let _ = boundary::set_draw_mode(
            &mut imp.game.borrow_mut(),
            session.mode,
            session.klondike_draw_mode,
        );
        imp.timer_started
            .set(imp.timer_started.get() && !boundary::is_won(&imp.game.borrow(), session.mode));
        imp.spider_suit_mode
            .set(imp.game.borrow().spider().suit_mode());
        self.set_seed_input_text(&session.seed.to_string());
        self.update_game_mode_menu_selection();
        self.invalidate_card_render_cache();
        imp.pending_deal_instructions.set(false);
        *imp.status_override.borrow_mut() = Some(status_message.to_string());
        let history = Self::payload_field(raw, "history")
            .and_then(Self::decode_snapshot_stack)
            .unwrap_or_default();
        let future = Self::payload_field(raw, "future")
            .and_then(Self::decode_snapshot_stack)
            .unwrap_or_default();
        *imp.history.borrow_mut() = history;
        *imp.future.borrow_mut() = future;
        *imp.last_saved_session.borrow_mut() = raw.to_string();
        if persist_payload && self.should_persist_shared_state() {
            if let Some(settings) = imp.settings.borrow().as_ref() {
                let _ = settings.set_string(SETTINGS_KEY_SAVED_SESSION, raw);
            }
        }
        imp.session_dirty.set(false);
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();
        let state_hash = self.current_game_hash();
        self.start_hint_loss_analysis_if_needed(state_hash);
        Ok(())
    }

    pub(super) fn update_stats_label(&self) {
        let imp = self.imp();
        let elapsed = imp.elapsed_seconds.get();
        let apm = self.current_apm();
        let mem = self.current_memory_mib_text();
        imp.stats_label.set_label(&format!(
            "Moves: {}   APM: {:.1}   Time: {}   Mem: {}",
            imp.move_count.get(),
            apm,
            format_time(elapsed),
            mem
        ));
    }
}

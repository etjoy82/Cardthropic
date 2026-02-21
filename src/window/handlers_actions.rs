use super::*;
use crate::engine::boundary;
use std::collections::BTreeSet;

impl CardthropicWindow {
    fn append_game_state_payload_history(&self, prefix: &str, payload: &str) {
        self.append_status_history_only(&format!("{prefix}_begin"));
        for line in payload.lines() {
            self.append_status_history_only(line);
        }
        self.append_status_history_only(&format!("{prefix}_end"));
    }

    pub(super) fn copy_game_state_to_clipboard(&self) {
        let (payload, status_message) = self.build_clipboard_game_state_payload();
        self.clipboard().set_text(&payload);
        self.append_game_state_payload_history("copy_game_state_payload", &payload);
        *self.imp().status_override.borrow_mut() = Some(status_message.to_string());
        self.render();
    }

    pub(super) fn copy_all_cardthropic_gsettings_variables_to_clipboard(&self) {
        let settings = self
            .imp()
            .settings
            .borrow()
            .clone()
            .or_else(Self::load_app_settings);
        let Some(settings) = settings else {
            *self.imp().status_override.borrow_mut() =
                Some("Copy failed: unable to load Cardthropic settings schema.".to_string());
            self.render();
            return;
        };
        if self.imp().settings.borrow().is_none() {
            *self.imp().settings.borrow_mut() = Some(settings.clone());
        }
        let Some(schema) = settings.settings_schema() else {
            *self.imp().status_override.borrow_mut() =
                Some("Copy failed: settings schema is unavailable.".to_string());
            self.render();
            return;
        };

        let mut keys = schema.list_keys();
        keys.sort();

        let mut payload_lines = Vec::with_capacity(keys.len() + 1);
        payload_lines.push(format!("# schema={SETTINGS_SCHEMA_ID}"));
        for key in &keys {
            let value = settings.value(key.as_str());
            payload_lines.push(format!("{}={}", key, value.print(false)));
        }
        let payload = payload_lines.join("\n");

        self.clipboard().set_text(&payload);
        self.append_game_state_payload_history("copy_gsettings_payload", &payload);
        *self.imp().status_override.borrow_mut() = Some(format!(
            "Copied {} Cardthropic GSettings variables to clipboard.",
            keys.len()
        ));
        self.render();
    }

    pub(super) fn load_all_cardthropic_gsettings_variables_from_clipboard(&self) {
        let clipboard = self.clipboard();
        clipboard.read_text_async(
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |result| match result {
                    Ok(Some(text)) => {
                        window.append_game_state_payload_history("load_gsettings_payload", &text);
                        match window.apply_cardthropic_gsettings_payload(&text) {
                            Ok(applied) => {
                                *window.imp().status_override.borrow_mut() = Some(format!(
                                    "Loaded {} Cardthropic GSettings variables from clipboard.",
                                    applied
                                ));
                            }
                            Err(err) => {
                                *window.imp().status_override.borrow_mut() =
                                    Some(format!("Load failed: {err}"));
                            }
                        }
                        window.render();
                    }
                    Ok(None) => {
                        *window.imp().status_override.borrow_mut() =
                            Some("Load failed: clipboard is empty.".to_string());
                        window.render();
                    }
                    Err(err) => {
                        *window.imp().status_override.borrow_mut() =
                            Some(format!("Load failed: {err}."));
                        window.render();
                    }
                }
            ),
        );
    }

    fn apply_cardthropic_gsettings_payload(&self, payload: &str) -> Result<usize, String> {
        let settings = self
            .imp()
            .settings
            .borrow()
            .clone()
            .or_else(Self::load_app_settings)
            .ok_or_else(|| "unable to load Cardthropic settings schema".to_string())?;
        if self.imp().settings.borrow().is_none() {
            *self.imp().settings.borrow_mut() = Some(settings.clone());
        }
        let schema = settings
            .settings_schema()
            .ok_or_else(|| "settings schema is unavailable".to_string())?;
        let mut schema_keys = schema
            .list_keys()
            .into_iter()
            .map(|k| k.to_string())
            .collect::<BTreeSet<_>>();
        if schema_keys.is_empty() {
            return Err("settings schema has no keys".to_string());
        }

        let mut seen_keys = BTreeSet::new();
        let mut parsed = Vec::<(String, glib::Variant)>::new();
        let mut saw_schema_header = false;

        for (line_no, raw_line) in payload.lines().enumerate() {
            let line = raw_line.trim();
            if line.is_empty()
                || line == "copy_gsettings_payload_begin"
                || line == "copy_gsettings_payload_end"
            {
                continue;
            }
            if let Some(found_schema) = line.strip_prefix("# schema=") {
                if found_schema != SETTINGS_SCHEMA_ID {
                    return Err(format!(
                        "schema mismatch at line {}: expected '{}', found '{}'",
                        line_no + 1,
                        SETTINGS_SCHEMA_ID,
                        found_schema
                    ));
                }
                saw_schema_header = true;
                continue;
            }
            if line.starts_with('#') {
                return Err(format!("unsupported metadata at line {}", line_no + 1));
            }

            let (key, value_text) = line
                .split_once('=')
                .ok_or_else(|| format!("invalid line {} (expected key=value)", line_no + 1))?;
            let key = key.trim();
            if key.is_empty() {
                return Err(format!("empty key at line {}", line_no + 1));
            }
            if !schema_keys.contains(key) {
                return Err(format!(
                    "unsupported setting '{}' at line {}",
                    key,
                    line_no + 1
                ));
            }
            if !seen_keys.insert(key.to_string()) {
                return Err(format!(
                    "duplicate setting '{}' at line {}",
                    key,
                    line_no + 1
                ));
            }

            let expected_type = settings.value(key).type_().to_owned();
            let parsed_value =
                glib::Variant::parse(Some(&expected_type), value_text).map_err(|err| {
                    format!(
                        "invalid value for '{}' at line {}: {}",
                        key,
                        line_no + 1,
                        err
                    )
                })?;
            if parsed_value.type_() != expected_type {
                return Err(format!(
                    "type mismatch for '{}' at line {}",
                    key,
                    line_no + 1
                ));
            }
            parsed.push((key.to_string(), parsed_value));
        }

        if !saw_schema_header {
            return Err(format!(
                "missing schema header '# schema={SETTINGS_SCHEMA_ID}'"
            ));
        }
        if parsed.is_empty() {
            return Err("no GSettings variables found in payload".to_string());
        }

        schema_keys.retain(|key| !seen_keys.contains(key));
        if !schema_keys.is_empty() {
            return Err(format!(
                "payload is missing {} setting(s): {}",
                schema_keys.len(),
                schema_keys.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }

        let backups = parsed
            .iter()
            .map(|(key, _)| (key.clone(), settings.value(key)))
            .collect::<Vec<_>>();

        let mut applied = 0usize;
        for (key, value) in &parsed {
            if let Err(err) = settings.set_value(key, value) {
                for (rollback_key, rollback_value) in &backups {
                    let _ = settings.set_value(rollback_key, rollback_value);
                }
                gio::Settings::sync();
                return Err(format!("failed to apply '{}': {}", key, err));
            }
            applied = applied.saturating_add(1);
        }

        gio::Settings::sync();
        Ok(applied)
    }

    pub(super) fn paste_game_state_from_clipboard(&self) {
        let clipboard = self.clipboard();
        clipboard.read_text_async(
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |result| match result {
                    Ok(Some(text)) => {
                        window.append_game_state_payload_history(
                            "load_game_state_payload",
                            text.as_str(),
                        );
                        match window.restore_game_state_from_clipboard_payload(&text, true) {
                            Ok(()) => {
                                window.render();
                            }
                            Err(err) => {
                                *window.imp().status_override.borrow_mut() = Some(err);
                                window.render();
                            }
                        }
                    }
                    Ok(None) => {
                        *window.imp().status_override.borrow_mut() =
                            Some("Paste failed: clipboard is empty.".to_string());
                        window.render();
                    }
                    Err(err) => {
                        *window.imp().status_override.borrow_mut() =
                            Some(format!("Paste failed: {err}."));
                        window.render();
                    }
                }
            ),
        );
    }

    pub(super) fn clear_seed_history_from_menu(&self) {
        self.clear_seed_history();
        *self.imp().status_override.borrow_mut() =
            Some("Cleared seed history from dropdown and settings.".to_string());
        self.render();
    }

    pub(super) fn show_clear_all_settings_and_history_confirmation(&self) {
        let confirmed = std::rc::Rc::new(std::cell::Cell::new(false));
        let dialog = gtk::Window::builder()
            .title("Clear Settings and History")
            .modal(true)
            .transient_for(self)
            .default_width(560)
            .default_height(260)
            .build();
        dialog.set_resizable(false);
        dialog.set_destroy_with_parent(true);
        dialog.add_css_class("clear-reset-dialog");
        dialog.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[strong]
            confirmed,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                if !confirmed.get() {
                    *window.imp().status_override.borrow_mut() =
                        Some("Clear settings/history canceled.".to_string());
                    window.render();
                }
                glib::Propagation::Proceed
            }
        ));

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(14);
        root.set_margin_bottom(14);
        root.set_margin_start(14);
        root.set_margin_end(14);

        let heading = gtk::Label::new(Some("Clear all settings and history?"));
        heading.set_xalign(0.0);
        heading.add_css_class("title-4");
        root.append(&heading);

        let body = gtk::Label::new(Some(
            "What this does:\n- Resets all Cardthropic settings for this profile.\n- Clears game history (seeds, status, undo/redo) and recent session data.\n- Resets Cardthropic appearance overrides (custom CSS/userstyle).\n\nWhy this is here:\n- Use it for a clean slate when testing, troubleshooting, or starting over.\n\nWarning:\n- Any in-progress games will be forgotten.\n- This action cannot be undone.\n- Some changes may require restarting Cardthropic.",
        ));
        body.set_wrap(true);
        body.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        body.set_xalign(0.0);
        root.append(&body);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);
        let cancel_button = gtk::Button::with_label("Cancel");
        cancel_button.add_css_class("clear-reset-cancel");
        cancel_button.add_css_class("suggested-action");
        let confirm_button = gtk::Button::with_label("Clear Everything");
        confirm_button.add_css_class("destructive-action");
        cancel_button.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        confirm_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            #[strong]
            confirmed,
            move |_| {
                confirmed.set(true);
                let result = window.clear_all_cardthropic_settings_and_history_now();
                match result {
                    Ok(key_count) => {
                        *window.imp().status_override.borrow_mut() = Some(format!(
                            "Cleared all Cardthropic settings and history ({} keys reset). Restart recommended.",
                            key_count
                        ));
                    }
                    Err(err) => {
                        *window.imp().status_override.borrow_mut() =
                            Some(format!("Failed to clear settings/history: {err}"));
                    }
                }
                window.render();
                dialog.close();
            }
        ));
        actions.append(&cancel_button);
        actions.append(&confirm_button);
        root.append(&actions);

        dialog.set_default_widget(Some(&cancel_button));
        let _ = cancel_button.grab_focus();
        dialog.set_child(Some(&root));
        dialog.present();
    }

    fn clear_all_cardthropic_settings_and_history_now(&self) -> Result<usize, String> {
        self.stop_rapid_wand();
        self.stop_robot_mode();
        self.cancel_seed_winnable_check(None);

        let settings = self
            .imp()
            .settings
            .borrow()
            .clone()
            .or_else(Self::load_app_settings)
            .ok_or_else(|| "unable to load Cardthropic settings schema".to_string())?;
        if self.imp().settings.borrow().is_none() {
            *self.imp().settings.borrow_mut() = Some(settings.clone());
        }
        let schema = settings
            .settings_schema()
            .ok_or_else(|| "settings schema is unavailable".to_string())?;
        let keys = schema.list_keys();
        for key in &keys {
            settings.reset(key.as_str());
        }
        gio::Settings::sync();

        let imp = self.imp();

        if let Some(source_id) = imp.session_flush_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        if let Some(source_id) = imp.seed_history_flush_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        if let Some(source_id) = imp.status_ephemeral_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }

        imp.session_dirty.set(false);
        imp.seed_history_dirty.set(false);
        imp.seed_history_dropdown_dirty.set(false);
        imp.history.borrow_mut().clear();
        imp.future.borrow_mut().clear();
        imp.chess_history.borrow_mut().clear();
        imp.chess_future.borrow_mut().clear();
        imp.status_history.borrow_mut().clear();
        *imp.status_last_appended.borrow_mut() = String::new();
        imp.klondike_controls_history_logged.set(false);
        *imp.layout_debug_last_appended.borrow_mut() = String::new();
        *imp.last_saved_session.borrow_mut() = String::new();
        if let Some(buffer) = imp.status_history_buffer.borrow().as_ref() {
            buffer.set_text("");
        }

        self.load_seed_history();
        self.refresh_seed_history_dropdown();

        let board_color = settings.string(SETTINGS_KEY_BOARD_COLOR).to_string();
        self.set_board_color(&board_color, false);
        let userstyle = settings
            .string(SETTINGS_KEY_CUSTOM_USERSTYLE_CSS)
            .to_string();
        if userstyle.trim().is_empty() {
            self.apply_custom_userstyle(Self::default_userstyle_css(), false);
        } else if let Some(migrated) = Self::migrate_legacy_userstyle_css(&userstyle) {
            self.apply_custom_userstyle(migrated, false);
        } else {
            self.apply_custom_userstyle(&userstyle, false);
        }
        *imp.saved_custom_userstyle_css.borrow_mut() = settings
            .string(SETTINGS_KEY_SAVED_CUSTOM_USERSTYLE_CSS)
            .to_string();
        let interface_font = settings
            .string(SETTINGS_KEY_INTERFACE_EMOJI_FONT)
            .to_string();
        if interface_font.trim().is_empty() {
            self.apply_interface_emoji_font(None, false);
        } else {
            self.apply_interface_emoji_font(Some(interface_font.as_str()), false);
        }
        self.set_card_render_mode(CardRenderMode::Unicode, false, false);
        let chess_rotation_degrees = if Self::settings_has_chess_rotation_key(&settings) {
            settings.int(SETTINGS_KEY_CHESS_BOARD_ROTATION_DEGREES)
        } else {
            0
        };
        self.set_chess_board_rotation_degrees(chess_rotation_degrees, false, false);

        let spider_suit_mode = u8::try_from(settings.int(SETTINGS_KEY_SPIDER_SUIT_MODE))
            .ok()
            .and_then(SpiderSuitMode::from_suit_count)
            .unwrap_or(SpiderSuitMode::One);
        let freecell_card_count_mode =
            u8::try_from(settings.int(SETTINGS_KEY_FREECELL_CARD_COUNT_MODE))
                .ok()
                .and_then(FreecellCardCountMode::from_card_count)
                .unwrap_or(FreecellCardCountMode::FiftyTwo);
        let freecell_cell_count = if Self::settings_has_freecell_cell_count_key(&settings) {
            u8::try_from(settings.int(SETTINGS_KEY_FREECELL_CELL_COUNT))
                .ok()
                .filter(|count| (FREECELL_MIN_CELL_COUNT..=FREECELL_MAX_CELL_COUNT).contains(count))
                .unwrap_or(FREECELL_DEFAULT_CELL_COUNT)
        } else {
            FREECELL_DEFAULT_CELL_COUNT
        };
        imp.spider_suit_mode.set(spider_suit_mode);
        imp.freecell_card_count_mode.set(freecell_card_count_mode);
        imp.freecell_cell_count.set(freecell_cell_count);

        let smart_move_mode =
            SmartMoveMode::from_setting(settings.string(SETTINGS_KEY_SMART_MOVE_MODE).as_ref());
        self.set_smart_move_mode(smart_move_mode, false, false);
        self.set_hud_enabled(settings.boolean(SETTINGS_KEY_ENABLE_HUD), false);
        self.set_robot_forever_enabled(settings.boolean(SETTINGS_KEY_FOREVER_MODE), false, false);
        self.set_robot_auto_new_game_on_loss_enabled(
            settings.boolean(SETTINGS_KEY_ROBOT_AUTO_NEW_GAME_ON_LOSS),
            false,
            false,
        );
        self.set_robot_ludicrous_enabled(
            settings.boolean(SETTINGS_KEY_LUDICROUS_SPEED),
            false,
            false,
        );
        self.set_chess_wand_ai_opponent_auto_response_enabled(
            if Self::settings_has_chess_wand_ai_opponent_auto_response_key(&settings) {
                settings.boolean(SETTINGS_KEY_CHESS_WAND_AI_OPPONENT_AUTO_RESPONSE)
            } else {
                true
            },
            false,
            false,
        );
        self.set_chess_auto_response_plays_white_enabled(
            if Self::settings_has_chess_auto_response_plays_white_key(&settings) {
                settings.boolean(SETTINGS_KEY_CHESS_AUTO_RESPONSE_PLAYS_WHITE)
            } else {
                false
            },
            false,
            false,
        );
        self.set_chess_auto_flip_board_each_move_enabled(
            if Self::settings_has_chess_auto_flip_board_each_move_key(&settings) {
                settings.boolean(SETTINGS_KEY_CHESS_AUTO_FLIP_BOARD_EACH_MOVE)
            } else {
                false
            },
            false,
            false,
        );
        self.set_chess_show_board_coordinates_enabled(
            if Self::settings_has_chess_show_board_coordinates_key(&settings) {
                settings.boolean(SETTINGS_KEY_CHESS_SHOW_BOARD_COORDINATES)
            } else {
                true
            },
            false,
            false,
        );
        self.set_chess_system_sounds_enabled(
            if Self::settings_has_chess_system_sounds_enabled_key(&settings) {
                settings.boolean(SETTINGS_KEY_CHESS_SYSTEM_SOUNDS_ENABLED)
            } else {
                false
            },
            false,
            false,
        );
        self.set_robot_debug_enabled(
            settings.boolean(SETTINGS_KEY_ROBOT_DEBUG_ENABLED),
            false,
            false,
        );
        self.set_robot_strict_debug_invariants_enabled(
            settings.boolean(SETTINGS_KEY_ROBOT_STRICT_DEBUG_INVARIANTS),
            false,
            false,
        );
        let memory_guard_enabled = settings.boolean(SETTINGS_KEY_MEMORY_GUARD_ENABLED);
        let memory_guard_soft_limit_mib =
            u64::try_from(settings.int(SETTINGS_KEY_MEMORY_GUARD_SOFT_LIMIT_MIB))
                .ok()
                .unwrap_or(1536);
        let memory_guard_hard_limit_mib =
            u64::try_from(settings.int(SETTINGS_KEY_MEMORY_GUARD_HARD_LIMIT_MIB))
                .ok()
                .unwrap_or(2048);
        self.configure_memory_guard(
            memory_guard_enabled,
            memory_guard_soft_limit_mib,
            memory_guard_hard_limit_mib,
        );
        self.set_memory_guard_enabled(memory_guard_enabled, false, false);

        self.update_game_mode_menu_selection();
        self.update_game_settings_menu();
        self.clear_hint_effects();
        self.reset_hint_cycle_memory();
        self.reset_auto_play_memory();

        Ok(keys.len())
    }

    pub(super) fn setup_primary_action_handlers(&self) {
        let imp = self.imp();

        imp.undo_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.undo();
            }
        ));
        imp.redo_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.redo();
            }
        ));

        imp.auto_hint_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.play_hint_for_player();
            }
        ));
        let wand_middle_click = gtk::GestureClick::new();
        wand_middle_click.set_button(gdk::BUTTON_MIDDLE);
        wand_middle_click.connect_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _, _| {
                window.trigger_rapid_wand();
            }
        ));
        imp.auto_hint_button.add_controller(wand_middle_click);

        imp.cyclone_shuffle_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.cyclone_shuffle_tableau();
            }
        ));
        imp.peek_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.trigger_peek();
            }
        ));
        imp.board_color_menu_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.show_theme_presets_window();
            }
        ));
        imp.robot_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.toggle_robot_mode();
            }
        ));
        let robot_middle_click = gtk::GestureClick::new();
        robot_middle_click.set_button(gdk::BUTTON_MIDDLE);
        robot_middle_click.connect_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _, _| {
                window.start_robot_mode_forever();
            }
        ));
        imp.robot_button.add_controller(robot_middle_click);
        imp.status_history_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.show_status_history_dialog();
            }
        ));
    }

    pub(super) fn setup_robot_stop_capture_handler(&self) {
        let robot_stop_click = gtk::GestureClick::new();
        robot_stop_click.set_button(0);
        robot_stop_click.set_propagation_phase(gtk::PropagationPhase::Capture);
        robot_stop_click.connect_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, x, y| {
                if !window.imp().robot_mode_running.get() {
                    return;
                }
                let imp = window.imp();
                let robot_button = imp.robot_button.get();
                if let Some(picked) = window.pick(x, y, gtk::PickFlags::DEFAULT) {
                    let robot_widget: gtk::Widget = robot_button.clone().upcast();
                    if picked == robot_widget || picked.is_ancestor(&robot_button) {
                        return;
                    }
                    let in_cards_zone = picked.is_ancestor(&imp.stock_picture.get())
                        || picked.is_ancestor(&imp.waste_overlay.get())
                        || picked.is_ancestor(&imp.foundation_picture_1.get())
                        || picked.is_ancestor(&imp.foundation_picture_2.get())
                        || picked.is_ancestor(&imp.foundation_picture_3.get())
                        || picked.is_ancestor(&imp.foundation_picture_4.get())
                        || picked.is_ancestor(&imp.tableau_scroller.get())
                        || picked.is_ancestor(&imp.tableau_row.get());
                    if !in_cards_zone {
                        return;
                    }
                } else {
                    return;
                }
                window.stop_robot_mode();
            }
        ));
        self.add_controller(robot_stop_click);
    }

    pub(super) fn setup_keyboard_navigation_handler(&self) {
        let keyboard_nav = gtk::EventControllerKey::new();
        keyboard_nav.set_propagation_phase(gtk::PropagationPhase::Capture);
        keyboard_nav.connect_key_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, state| {
                if window.handle_variant_shortcut_key(key, state) {
                    return glib::Propagation::Stop;
                }
                let navigation_key = matches!(
                    key,
                    gdk::Key::Left
                        | gdk::Key::Right
                        | gdk::Key::Up
                        | gdk::Key::Down
                        | gdk::Key::KP_Left
                        | gdk::Key::KP_Right
                        | gdk::Key::KP_Up
                        | gdk::Key::KP_Down
                        | gdk::Key::a
                        | gdk::Key::A
                        | gdk::Key::d
                        | gdk::Key::D
                        | gdk::Key::w
                        | gdk::Key::W
                        | gdk::Key::s
                        | gdk::Key::S
                        | gdk::Key::h
                        | gdk::Key::H
                        | gdk::Key::j
                        | gdk::Key::J
                        | gdk::Key::k
                        | gdk::Key::K
                        | gdk::Key::l
                        | gdk::Key::L
                );
                let activation_key =
                    matches!(key, gdk::Key::space | gdk::Key::Return | gdk::Key::KP_Enter);
                let numpad_solitaire_key = matches!(
                    key,
                    gdk::Key::KP_1
                        | gdk::Key::KP_3
                        | gdk::Key::KP_5
                        | gdk::Key::KP_7
                        | gdk::Key::KP_9
                        | gdk::Key::KP_Decimal
                        | gdk::Key::KP_Delete
                        | gdk::Key::KP_Multiply
                        | gdk::Key::KP_Subtract
                        | gdk::Key::KP_Add
                );
                let command_palette_key = key == gdk::Key::KP_Divide;
                if navigation_key || activation_key || numpad_solitaire_key || command_palette_key {
                    if window.is_seed_box_focused() {
                        return glib::Propagation::Proceed;
                    }
                }
                if navigation_key {
                    // Arrow navigation should move board focus, not stay trapped on a focused button.
                    window.grab_focus();
                }
                if state.intersects(
                    gdk::ModifierType::ALT_MASK
                        | gdk::ModifierType::CONTROL_MASK
                        | gdk::ModifierType::SUPER_MASK
                        | gdk::ModifierType::META_MASK,
                ) {
                    return glib::Propagation::Proceed;
                }
                if command_palette_key {
                    window.show_command_search_dialog();
                    return glib::Propagation::Stop;
                }
                if window.imp().chess_mode_active.get() {
                    if window.handle_chess_keyboard_key(key) {
                        return glib::Propagation::Stop;
                    }
                    return glib::Propagation::Proceed;
                }
                if window.handle_numpad_solitaire_shortcut_key(key, state) {
                    return glib::Propagation::Stop;
                }
                if window.handle_keyboard_navigation_key(key) {
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));
        self.add_controller(keyboard_nav);
    }

    #[allow(deprecated)]
    pub(super) fn setup_seed_handlers(&self) {
        let imp = self.imp();
        imp.seed_combo.connect_changed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |combo| {
                if window.imp().seed_combo_updating.get() {
                    return;
                }
                if let Some(seed) = combo.active_id() {
                    window.set_seed_input_text(seed.as_str());
                    window.start_new_game_from_seed_controls();
                    return;
                }
                window.clear_seed_entry_feedback();
                window.cancel_seed_winnable_check(None);
            }
        ));

        if let Some(seed_entry) = self.seed_text_entry() {
            seed_entry.set_placeholder_text(Some("Blank=random, or enter seed number/word"));
            seed_entry.set_width_chars(1);
            seed_entry.connect_changed(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.clear_seed_entry_feedback();
                    window.cancel_seed_winnable_check(None);
                }
            ));
            seed_entry.connect_activate(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.start_new_game_from_seed_controls();
                }
            ));
        }

        imp.seed_random_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.start_random_seed_game();
            }
        ));
        imp.seed_rescue_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.start_random_winnable_seed_game();
            }
        ));
        imp.seed_winnable_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.toggle_seed_winnable_check();
            }
        ));
        imp.seed_repeat_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.repeat_current_seed_game();
            }
        ));
        imp.seed_go_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.start_new_game_from_seed_controls();
            }
        ));
    }

    pub(super) fn setup_board_click_handlers(&self) {
        let imp = self.imp();

        let stock_click = gtk::GestureClick::new();
        stock_click.connect_released(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _, _| {
                if window.imp().chess_mode_active.get() {
                    return;
                }
                window.draw_card();
            }
        ));
        imp.stock_picture.add_controller(stock_click);

        let waste_click = gtk::GestureClick::new();
        waste_click.set_button(0);
        waste_click.connect_released(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |gesture, n_press, x, _| {
                if window.imp().chess_mode_active.get() {
                    return;
                }
                let current_button = gesture.current_button();
                if window.active_game_mode() == GameMode::Freecell {
                    if window.smart_move_mode() == SmartMoveMode::RightClick
                        && current_button == gdk::BUTTON_SECONDARY
                    {
                        let idx = window.freecell_slot_index_from_waste_x(x);
                        let _ = window.try_smart_move_from_freecell(idx);
                        return;
                    }
                    window.handle_freecell_click_x(n_press, Some(x));
                } else {
                    if window.smart_move_mode() == SmartMoveMode::RightClick
                        && current_button == gdk::BUTTON_SECONDARY
                    {
                        let _ = window.try_smart_move_from_waste();
                        return;
                    }
                    window.handle_waste_click(n_press);
                }
            }
        ));
        imp.waste_overlay.add_controller(waste_click);

        for (index, stack) in self.tableau_stacks().into_iter().enumerate() {
            let click = gtk::GestureClick::new();
            click.set_button(0);
            click.connect_released(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |gesture, n_press, _, y| {
                    let current_button = gesture.current_button();
                    if window.imp().chess_mode_active.get() {
                        window.handle_chess_board_stack_click(index, y);
                        return;
                    }
                    if window.active_game_mode() == GameMode::Spider {
                        let game = window.imp().game.borrow().spider().clone();
                        let start = window.tableau_run_start_from_y_spider(&game, index, y);
                        match window.smart_move_mode() {
                            SmartMoveMode::DoubleClick if n_press == 2 => {
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::SingleClick if n_press == 1 => {
                                *window.imp().selected_run.borrow_mut() = None;
                                window.imp().waste_selected.set(false);
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::RightClick
                                if current_button == gdk::BUTTON_SECONDARY =>
                            {
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::Disabled | SmartMoveMode::DoubleClick
                                if n_press == 1 =>
                            {
                                window.select_or_move_tableau_with_start(index, start);
                            }
                            _ => {}
                        }
                        return;
                    }
                    if window.active_game_mode() == GameMode::Freecell {
                        let game = window.imp().game.borrow().freecell().clone();
                        let start = window.tableau_run_start_from_y_freecell(&game, index, y);
                        match window.smart_move_mode() {
                            SmartMoveMode::DoubleClick if n_press == 2 => {
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::SingleClick if n_press == 1 => {
                                *window.imp().selected_run.borrow_mut() = None;
                                window.imp().selected_freecell.set(None);
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::RightClick
                                if current_button == gdk::BUTTON_SECONDARY =>
                            {
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            _ if n_press == 1 => {
                                window.select_or_move_tableau_with_start(index, start);
                            }
                            _ => {}
                        }
                        return;
                    }

                    match window.smart_move_mode() {
                        SmartMoveMode::DoubleClick if n_press == 2 => {
                            let start = boundary::clone_klondike_for_automation(
                                &window.imp().game.borrow(),
                                window.active_game_mode(),
                                window.current_klondike_draw_mode(),
                            )
                            .and_then(|game| window.tableau_run_start_from_y(&game, index, y));
                            if let Some(start) = start {
                                window.try_smart_move_from_tableau(index, start);
                            }
                        }
                        SmartMoveMode::SingleClick if n_press == 1 => {
                            *window.imp().selected_run.borrow_mut() = None;
                            window.imp().waste_selected.set(false);
                            let start = boundary::clone_klondike_for_automation(
                                &window.imp().game.borrow(),
                                window.active_game_mode(),
                                window.current_klondike_draw_mode(),
                            )
                            .and_then(|game| window.tableau_run_start_from_y(&game, index, y));
                            if let Some(start) = start {
                                window.try_smart_move_from_tableau(index, start);
                            }
                        }
                        SmartMoveMode::RightClick if current_button == gdk::BUTTON_SECONDARY => {
                            let start = boundary::clone_klondike_for_automation(
                                &window.imp().game.borrow(),
                                window.active_game_mode(),
                                window.current_klondike_draw_mode(),
                            )
                            .and_then(|game| window.tableau_run_start_from_y(&game, index, y));
                            if let Some(start) = start {
                                window.try_smart_move_from_tableau(index, start);
                            }
                        }
                        SmartMoveMode::Disabled | SmartMoveMode::DoubleClick if n_press == 1 => {
                            let start = boundary::clone_klondike_for_automation(
                                &window.imp().game.borrow(),
                                window.active_game_mode(),
                                window.current_klondike_draw_mode(),
                            )
                            .and_then(|game| window.tableau_run_start_from_y(&game, index, y));
                            window.select_or_move_tableau_with_start(index, start);
                        }
                        _ => {}
                    }
                }
            ));
            stack.add_controller(click);
        }

        for (index, foundation) in self.foundation_pictures().into_iter().enumerate() {
            let click = gtk::GestureClick::new();
            click.connect_released(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _, _, _| {
                    if window.imp().chess_mode_active.get() {
                        return;
                    }
                    window.handle_click_on_foundation(index);
                }
            ));
            foundation.add_controller(click);
        }
        for (index, foundation) in self.foundation_placeholders().into_iter().enumerate() {
            let click = gtk::GestureClick::new();
            click.connect_released(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _, _, _| {
                    if window.imp().chess_mode_active.get() {
                        return;
                    }
                    window.handle_click_on_foundation(index);
                }
            ));
            foundation.add_controller(click);
        }
    }
}

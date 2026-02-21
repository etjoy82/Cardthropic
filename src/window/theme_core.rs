use super::*;
use crate::game::{FreecellCardCountMode, FreecellGame, SpiderGame};

impl CardthropicWindow {
    pub(super) fn default_interface_font_family(&self) -> String {
        let fallback = "Sans".to_string();
        let Some(settings) = gtk::Settings::default() else {
            return fallback;
        };
        let font_name: String = settings.property("gtk-font-name");
        let trimmed = font_name.trim();
        if trimmed.is_empty() {
            return fallback;
        }
        // gtk-font-name is usually "Family Size" (e.g. "Cantarell 11").
        let family = trimmed
            .trim_end_matches(|c: char| c.is_ascii_digit() || c.is_ascii_whitespace())
            .trim();
        if family.is_empty() {
            fallback
        } else {
            family.to_string()
        }
    }

    fn css_string_literal(value: &str) -> String {
        value.replace('\\', "\\\\").replace('"', "\\\"")
    }

    pub(super) fn apply_interface_emoji_font(&self, family: Option<&str>, persist: bool) {
        let selected = family
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .unwrap_or_default();

        if persist {
            if let Some(settings) = self.imp().settings.borrow().as_ref() {
                let _ = settings.set_string(SETTINGS_KEY_INTERFACE_EMOJI_FONT, &selected);
            }
        }

        let existing_provider = {
            let borrow = self.imp().interface_font_provider.borrow();
            borrow.as_ref().cloned()
        };
        let provider = if let Some(existing) = existing_provider {
            existing
        } else {
            let provider = gtk::CssProvider::new();
            gtk::style_context_add_provider_for_display(
                &self.display(),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
            {
                let mut borrow_mut = self.imp().interface_font_provider.borrow_mut();
                *borrow_mut = Some(provider.clone());
            }
            provider
        };

        if selected.is_empty() {
            provider.load_from_string("");
            return;
        }

        let family_css = Self::css_string_literal(&selected);
        provider.load_from_string(&format!(
            "label, button, entry, checkbutton, menubutton, dropdown, combobox, spinbutton, textview, treeview {{
  font-family: \"{family}\";
}}",
            family = family_css
        ));
    }

    pub(super) fn setup_styles(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("../style.css"));
        gtk::style_context_add_provider_for_display(
            &self.display(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        *self.imp().style_provider.borrow_mut() = Some(provider);
        adw::StyleManager::default().set_color_scheme(adw::ColorScheme::Default);
    }

    pub(super) fn setup_board_color_preferences(&self) {
        let imp = self.imp();
        let settings = Self::load_app_settings();
        *imp.settings.borrow_mut() = settings;

        let initial_color = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.string(SETTINGS_KEY_BOARD_COLOR).to_string())
                .unwrap_or_else(|| DEFAULT_BOARD_COLOR.to_string())
        };
        self.set_board_color(&initial_color, false);
        let initial_userstyle = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    settings
                        .string(SETTINGS_KEY_CUSTOM_USERSTYLE_CSS)
                        .to_string()
                })
                .unwrap_or_default()
        };
        let saved_custom_userstyle = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    settings
                        .string(SETTINGS_KEY_SAVED_CUSTOM_USERSTYLE_CSS)
                        .to_string()
                })
                .unwrap_or_default()
        };
        if !saved_custom_userstyle.trim().is_empty() {
            *imp.saved_custom_userstyle_css.borrow_mut() = saved_custom_userstyle;
        } else if !initial_userstyle.trim().is_empty()
            && Self::userstyle_preset_for_css(&initial_userstyle) == 0
        {
            *imp.saved_custom_userstyle_css.borrow_mut() = initial_userstyle.clone();
            if let Some(settings) = imp.settings.borrow().as_ref() {
                let _ = settings
                    .set_string(SETTINGS_KEY_SAVED_CUSTOM_USERSTYLE_CSS, &initial_userstyle);
            }
        }
        if initial_userstyle.trim().is_empty() {
            self.apply_custom_userstyle(Self::default_userstyle_css(), false);
        } else if let Some(migrated) = Self::migrate_legacy_userstyle_css(&initial_userstyle) {
            self.apply_custom_userstyle(migrated, true);
        } else {
            self.apply_custom_userstyle(&initial_userstyle, false);
        }
        imp.card_render_mode.set(CardRenderMode::Unicode);

        let smart_move_mode = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    SmartMoveMode::from_setting(
                        settings.string(SETTINGS_KEY_SMART_MOVE_MODE).as_ref(),
                    )
                })
                .unwrap_or(SmartMoveMode::DoubleClick)
        };
        self.set_smart_move_mode(smart_move_mode, false, false);

        let hud_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.boolean(SETTINGS_KEY_ENABLE_HUD))
                .unwrap_or(true)
        };
        self.set_hud_enabled(hud_enabled, false);

        let forever_mode_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.boolean(SETTINGS_KEY_FOREVER_MODE))
                .unwrap_or(false)
        };
        self.set_robot_forever_enabled(forever_mode_enabled, false, false);

        let robot_auto_new_game_on_loss_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.boolean(SETTINGS_KEY_ROBOT_AUTO_NEW_GAME_ON_LOSS))
                .unwrap_or(true)
        };
        self.set_robot_auto_new_game_on_loss_enabled(
            robot_auto_new_game_on_loss_enabled,
            false,
            false,
        );

        let ludicrous_speed_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.boolean(SETTINGS_KEY_LUDICROUS_SPEED))
                .unwrap_or(false)
        };
        self.set_robot_ludicrous_enabled(ludicrous_speed_enabled, false, false);

        let chess_wand_ai_opponent_auto_response_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    if Self::settings_has_chess_wand_ai_opponent_auto_response_key(settings) {
                        settings.boolean(SETTINGS_KEY_CHESS_WAND_AI_OPPONENT_AUTO_RESPONSE)
                    } else {
                        true
                    }
                })
                .unwrap_or(true)
        };
        self.set_chess_wand_ai_opponent_auto_response_enabled(
            chess_wand_ai_opponent_auto_response_enabled,
            false,
            false,
        );

        let chess_auto_response_plays_white_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    if Self::settings_has_chess_auto_response_plays_white_key(settings) {
                        settings.boolean(SETTINGS_KEY_CHESS_AUTO_RESPONSE_PLAYS_WHITE)
                    } else {
                        false
                    }
                })
                .unwrap_or(false)
        };
        self.set_chess_auto_response_plays_white_enabled(
            chess_auto_response_plays_white_enabled,
            false,
            false,
        );

        let chess_auto_flip_board_each_move_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    if Self::settings_has_chess_auto_flip_board_each_move_key(settings) {
                        settings.boolean(SETTINGS_KEY_CHESS_AUTO_FLIP_BOARD_EACH_MOVE)
                    } else {
                        false
                    }
                })
                .unwrap_or(false)
        };
        self.set_chess_auto_flip_board_each_move_enabled(
            chess_auto_flip_board_each_move_enabled,
            false,
            false,
        );

        let chess_show_board_coordinates_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    if Self::settings_has_chess_show_board_coordinates_key(settings) {
                        settings.boolean(SETTINGS_KEY_CHESS_SHOW_BOARD_COORDINATES)
                    } else {
                        true
                    }
                })
                .unwrap_or(true)
        };
        self.set_chess_show_board_coordinates_enabled(
            chess_show_board_coordinates_enabled,
            false,
            false,
        );

        let chess_system_sounds_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    if Self::settings_has_chess_system_sounds_enabled_key(settings) {
                        settings.boolean(SETTINGS_KEY_CHESS_SYSTEM_SOUNDS_ENABLED)
                    } else {
                        false
                    }
                })
                .unwrap_or(false)
        };
        self.set_chess_system_sounds_enabled(chess_system_sounds_enabled, false, false);

        let robot_debug_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.boolean(SETTINGS_KEY_ROBOT_DEBUG_ENABLED))
                .unwrap_or(false)
        };
        self.set_robot_debug_enabled(robot_debug_enabled, false, false);

        let robot_strict_debug_invariants_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.boolean(SETTINGS_KEY_ROBOT_STRICT_DEBUG_INVARIANTS))
                .unwrap_or(true)
        };
        self.set_robot_strict_debug_invariants_enabled(
            robot_strict_debug_invariants_enabled,
            false,
            false,
        );

        let interface_font = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    settings
                        .string(SETTINGS_KEY_INTERFACE_EMOJI_FONT)
                        .to_string()
                })
                .unwrap_or_default()
        };
        if interface_font.trim().is_empty() {
            self.apply_interface_emoji_font(None, false);
        } else {
            self.apply_interface_emoji_font(Some(interface_font.as_str()), false);
        }

        let memory_guard_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.boolean(SETTINGS_KEY_MEMORY_GUARD_ENABLED))
                .unwrap_or(false)
        };
        let memory_guard_soft_limit_mib = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .and_then(|settings| {
                    u64::try_from(settings.int(SETTINGS_KEY_MEMORY_GUARD_SOFT_LIMIT_MIB)).ok()
                })
                .unwrap_or(1536)
        };
        let memory_guard_hard_limit_mib = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .and_then(|settings| {
                    u64::try_from(settings.int(SETTINGS_KEY_MEMORY_GUARD_HARD_LIMIT_MIB)).ok()
                })
                .unwrap_or(2048)
        };
        self.configure_memory_guard(
            memory_guard_enabled,
            memory_guard_soft_limit_mib,
            memory_guard_hard_limit_mib,
        );
        self.set_memory_guard_enabled(memory_guard_enabled, false, false);

        let spider_suit_mode = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .and_then(|settings| {
                    let raw = settings.int(SETTINGS_KEY_SPIDER_SUIT_MODE);
                    u8::try_from(raw)
                        .ok()
                        .and_then(SpiderSuitMode::from_suit_count)
                })
                .unwrap_or(SpiderSuitMode::One)
        };
        imp.spider_suit_mode.set(spider_suit_mode);
        let freecell_card_count_mode = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .and_then(|settings| {
                    let raw = settings.int(SETTINGS_KEY_FREECELL_CARD_COUNT_MODE);
                    u8::try_from(raw)
                        .ok()
                        .and_then(FreecellCardCountMode::from_card_count)
                })
                .unwrap_or(FreecellCardCountMode::FiftyTwo)
        };
        let freecell_cell_count = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    if Self::settings_has_freecell_cell_count_key(settings) {
                        settings.int(SETTINGS_KEY_FREECELL_CELL_COUNT)
                    } else {
                        i32::from(FREECELL_DEFAULT_CELL_COUNT)
                    }
                })
                .and_then(|raw| u8::try_from(raw).ok())
                .filter(|count| (FREECELL_MIN_CELL_COUNT..=FREECELL_MAX_CELL_COUNT).contains(count))
                .unwrap_or(FREECELL_DEFAULT_CELL_COUNT)
        };
        imp.freecell_card_count_mode.set(freecell_card_count_mode);
        imp.freecell_cell_count.set(freecell_cell_count);
        let seed = imp.current_seed.get();
        imp.game
            .borrow_mut()
            .set_spider(SpiderGame::new_with_seed_and_mode(seed, spider_suit_mode));
        imp.game
            .borrow_mut()
            .set_freecell(FreecellGame::new_with_seed_and_card_count_and_cells(
                seed,
                freecell_card_count_mode,
                freecell_cell_count,
            ));

        let chess_rotation_degrees = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .and_then(|settings| {
                    if Self::settings_has_chess_rotation_key(settings) {
                        Some(settings.int(SETTINGS_KEY_CHESS_BOARD_ROTATION_DEGREES))
                    } else {
                        None
                    }
                })
                .unwrap_or(0)
        };
        self.set_chess_board_rotation_degrees(chess_rotation_degrees, false, false);

        self.setup_theme_settings_sync();
    }

    fn setup_theme_settings_sync(&self) {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };

        settings.connect_changed(
            Some(SETTINGS_KEY_BOARD_COLOR),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |settings, _| {
                    let color = settings.string(SETTINGS_KEY_BOARD_COLOR).to_string();
                    window.set_board_color(&color, false);
                    window.render();
                }
            ),
        );

        settings.connect_changed(
            Some(SETTINGS_KEY_CUSTOM_USERSTYLE_CSS),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |settings, _| {
                    let css = settings
                        .string(SETTINGS_KEY_CUSTOM_USERSTYLE_CSS)
                        .to_string();
                    if css.trim().is_empty() {
                        window.apply_custom_userstyle(Self::default_userstyle_css(), false);
                    } else if let Some(migrated) = Self::migrate_legacy_userstyle_css(&css) {
                        window.apply_custom_userstyle(migrated, false);
                    } else {
                        window.apply_custom_userstyle(&css, false);
                    }
                    window.render();
                }
            ),
        );

        settings.connect_changed(
            Some(SETTINGS_KEY_SAVED_CUSTOM_USERSTYLE_CSS),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |settings, _| {
                    let css = settings
                        .string(SETTINGS_KEY_SAVED_CUSTOM_USERSTYLE_CSS)
                        .to_string();
                    *window.imp().saved_custom_userstyle_css.borrow_mut() = css;
                }
            ),
        );

        settings.connect_changed(
            Some(SETTINGS_KEY_INTERFACE_EMOJI_FONT),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |settings, _| {
                    let family = settings
                        .string(SETTINGS_KEY_INTERFACE_EMOJI_FONT)
                        .to_string();
                    if family.trim().is_empty() {
                        window.apply_interface_emoji_font(None, false);
                    } else {
                        window.apply_interface_emoji_font(Some(family.as_str()), false);
                    }
                    window.render();
                }
            ),
        );

        if Self::settings_has_freecell_cell_count_key(settings) {
            settings.connect_changed(
                Some(SETTINGS_KEY_FREECELL_CELL_COUNT),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |settings, _| {
                        let count = u8::try_from(settings.int(SETTINGS_KEY_FREECELL_CELL_COUNT))
                            .ok()
                            .filter(|count| {
                                (FREECELL_MIN_CELL_COUNT..=FREECELL_MAX_CELL_COUNT).contains(count)
                            })
                            .unwrap_or(FREECELL_DEFAULT_CELL_COUNT);
                        window.set_freecell_cell_count(count, false);
                    }
                ),
            );
        }

        if Self::settings_has_chess_rotation_key(settings) {
            settings.connect_changed(
                Some(SETTINGS_KEY_CHESS_BOARD_ROTATION_DEGREES),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |settings, _| {
                        window.set_chess_board_rotation_degrees(
                            settings.int(SETTINGS_KEY_CHESS_BOARD_ROTATION_DEGREES),
                            false,
                            false,
                        );
                    }
                ),
            );
        }

        if Self::settings_has_chess_wand_ai_opponent_auto_response_key(settings) {
            settings.connect_changed(
                Some(SETTINGS_KEY_CHESS_WAND_AI_OPPONENT_AUTO_RESPONSE),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |settings, _| {
                        window.set_chess_wand_ai_opponent_auto_response_enabled(
                            settings.boolean(SETTINGS_KEY_CHESS_WAND_AI_OPPONENT_AUTO_RESPONSE),
                            false,
                            false,
                        );
                    }
                ),
            );
        }

        if Self::settings_has_chess_auto_response_plays_white_key(settings) {
            settings.connect_changed(
                Some(SETTINGS_KEY_CHESS_AUTO_RESPONSE_PLAYS_WHITE),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |settings, _| {
                        window.set_chess_auto_response_plays_white_enabled(
                            settings.boolean(SETTINGS_KEY_CHESS_AUTO_RESPONSE_PLAYS_WHITE),
                            false,
                            false,
                        );
                    }
                ),
            );
        }

        if Self::settings_has_chess_auto_flip_board_each_move_key(settings) {
            settings.connect_changed(
                Some(SETTINGS_KEY_CHESS_AUTO_FLIP_BOARD_EACH_MOVE),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |settings, _| {
                        window.set_chess_auto_flip_board_each_move_enabled(
                            settings.boolean(SETTINGS_KEY_CHESS_AUTO_FLIP_BOARD_EACH_MOVE),
                            false,
                            false,
                        );
                    }
                ),
            );
        }

        if Self::settings_has_chess_show_board_coordinates_key(settings) {
            settings.connect_changed(
                Some(SETTINGS_KEY_CHESS_SHOW_BOARD_COORDINATES),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |settings, _| {
                        window.set_chess_show_board_coordinates_enabled(
                            settings.boolean(SETTINGS_KEY_CHESS_SHOW_BOARD_COORDINATES),
                            false,
                            false,
                        );
                    }
                ),
            );
        }

        if Self::settings_has_chess_system_sounds_enabled_key(settings) {
            settings.connect_changed(
                Some(SETTINGS_KEY_CHESS_SYSTEM_SOUNDS_ENABLED),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    move |settings, _| {
                        window.set_chess_system_sounds_enabled(
                            settings.boolean(SETTINGS_KEY_CHESS_SYSTEM_SOUNDS_ENABLED),
                            false,
                            false,
                        );
                    }
                ),
            );
        }
    }

    pub(super) fn load_app_settings() -> Option<gio::Settings> {
        let source = gio::SettingsSchemaSource::default()?;
        let schema = source.lookup(SETTINGS_SCHEMA_ID, true)?;
        Some(gio::Settings::new_full(
            &schema,
            None::<&gio::SettingsBackend>,
            None::<&str>,
        ))
    }
}

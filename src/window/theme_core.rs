use super::*;
use crate::game::SpiderGame;

impl CardthropicWindow {
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
                let _ = settings.set_string(SETTINGS_KEY_SAVED_CUSTOM_USERSTYLE_CSS, &initial_userstyle);
            }
        }
        if initial_userstyle.trim().is_empty() {
            self.apply_custom_userstyle(Self::default_userstyle_css(), false);
        } else if let Some(migrated) = Self::migrate_legacy_userstyle_css(&initial_userstyle) {
            self.apply_custom_userstyle(migrated, true);
        } else {
            self.apply_custom_userstyle(&initial_userstyle, false);
        }

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

        let robot_strategy = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    RobotStrategy::from_setting(
                        settings.string(SETTINGS_KEY_ROBOT_STRATEGY).as_ref(),
                    )
                })
                .unwrap_or(RobotStrategy::Balanced)
        };
        self.set_robot_strategy(robot_strategy, false, false);

        let hud_enabled = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.boolean(SETTINGS_KEY_ENABLE_HUD))
                .unwrap_or(true)
        };
        self.set_hud_enabled(hud_enabled, false);

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
        let seed = imp.current_seed.get();
        imp.game
            .borrow_mut()
            .set_spider(SpiderGame::new_with_seed_and_mode(seed, spider_suit_mode));
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

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppearanceMode {
    System,
    Light,
    Dark,
}

impl AppearanceMode {
    fn as_setting(self) -> &'static str {
        match self {
            Self::System => "system",
            Self::Light => "light",
            Self::Dark => "dark",
        }
    }

    fn from_setting(value: &str) -> Self {
        match value {
            "light" => Self::Light,
            "dark" => Self::Dark,
            _ => Self::System,
        }
    }
}

impl CardthropicWindow {
    const LIGHT_MODE_BOARD_VARIANTS: [(&'static str, &'static str); 12] = [
        ("#1f232b", "#d9dde6"),
        ("#1f3b2f", "#d4e8dc"),
        ("#2a2f45", "#d8dcef"),
        ("#3a2a26", "#ead9d2"),
        ("#1e3f53", "#d3e4ee"),
        ("#2d2d2d", "#e2e2e2"),
        ("#3b4f24", "#dce7c9"),
        ("#47315c", "#e3d7ef"),
        ("#5a3d24", "#efdcc9"),
        ("#0f5132", "#cbe8da"),
        ("#244a73", "#d1e1f1"),
        ("#6b2f2f", "#f0d6d6"),
    ];

    pub(super) fn setup_styles(&self) {
        let provider = gtk::CssProvider::new();
        provider.load_from_string(include_str!("../style.css"));
        gtk::style_context_add_provider_for_display(
            &self.display(),
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
        *self.imp().style_provider.borrow_mut() = Some(provider);
    }

    pub(super) fn setup_board_color_preferences(&self) {
        let imp = self.imp();
        let settings = Self::load_app_settings();
        *imp.settings.borrow_mut() = settings;

        let appearance_mode = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| {
                    AppearanceMode::from_setting(settings.string(SETTINGS_KEY_APPEARANCE_MODE).as_ref())
                })
                .unwrap_or(AppearanceMode::System)
        };
        self.set_appearance_mode(appearance_mode, false);

        let initial_color = {
            let settings = imp.settings.borrow().clone();
            settings
                .as_ref()
                .map(|settings| settings.string(SETTINGS_KEY_BOARD_COLOR).to_string())
                .unwrap_or_else(|| DEFAULT_BOARD_COLOR.to_string())
        };
        self.set_board_color(&initial_color, false);
        adw::StyleManager::default().connect_dark_notify(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.refresh_board_color_for_style();
                window.refresh_board_color_chips();
            }
        ));

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
    }

    fn load_app_settings() -> Option<gio::Settings> {
        let source = gio::SettingsSchemaSource::default()?;
        let schema = source.lookup(SETTINGS_SCHEMA_ID, true)?;
        Some(gio::Settings::new_full(
            &schema,
            None::<&gio::SettingsBackend>,
            None::<&str>,
        ))
    }

    #[allow(deprecated)]
    pub(super) fn setup_board_color_dropdown(&self) {
        let imp = self.imp();
        let color_menu = imp.board_color_menu_button.get();
        color_menu.set_tooltip_text(Some("Board color"));
        color_menu.set_has_frame(true);
        color_menu.add_css_class("board-color-menu-button");

        let preview_frame = gtk::Frame::new(None);
        preview_frame.add_css_class("color-chip-frame");
        let preview = gtk::DrawingArea::new();
        preview.set_content_width(18);
        preview.set_content_height(18);
        preview.set_draw_func(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, cr, width, height| {
                let color = window.current_board_color_rgba();
                Self::draw_color_chip(cr, width, height, &color);
            }
        ));
        preview_frame.set_child(Some(&preview));

        let arrow = gtk::Image::from_icon_name("pan-down-symbolic");
        let color_menu_content = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        color_menu_content.append(&preview_frame);
        color_menu_content.append(&arrow);
        color_menu.set_child(Some(&color_menu_content));

        let palette_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        palette_box.set_margin_top(8);
        palette_box.set_margin_bottom(8);
        palette_box.set_margin_start(8);
        palette_box.set_margin_end(8);
        let palette_popover = gtk::Popover::new();

        let theme_label = gtk::Label::new(Some("Themes"));
        theme_label.set_xalign(0.0);
        theme_label.add_css_class("dim-label");
        palette_box.append(&theme_label);

        let theme_row = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        theme_row.set_hexpand(true);
        theme_row.set_homogeneous(true);
        for (theme_name, color_hex) in BOARD_COLOR_THEMES {
            let theme_button = gtk::Button::with_label(theme_name);
            theme_button.add_css_class("flat");
            theme_button.connect_clicked(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.set_board_color(color_hex, true);
                }
            ));
            theme_row.append(&theme_button);
        }
        palette_box.append(&theme_row);

        let appearance_label = gtk::Label::new(Some("Appearance"));
        appearance_label.set_xalign(0.0);
        appearance_label.set_margin_top(2);
        appearance_label.add_css_class("dim-label");
        palette_box.append(&appearance_label);

        let appearance_dropdown = gtk::DropDown::from_strings(&["System Default", "Light", "Dark"]);
        appearance_dropdown.set_hexpand(true);
        appearance_dropdown.set_selected(match self.current_appearance_mode() {
            AppearanceMode::System => 0,
            AppearanceMode::Light => 1,
            AppearanceMode::Dark => 2,
        });
        let palette_popover_for_dropdown = palette_popover.clone();
        appearance_dropdown.connect_selected_notify(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |dropdown| {
                let mode = match dropdown.selected() {
                    1 => AppearanceMode::Light,
                    2 => AppearanceMode::Dark,
                    _ => AppearanceMode::System,
                };
                if mode == window.current_appearance_mode() {
                    return;
                }

                // Defer style changes until after the dropdown click cycle completes,
                // otherwise pointer grab/focus can get stuck in the popover.
                glib::idle_add_local_once(glib::clone!(
                    #[weak]
                    window,
                    #[weak]
                    palette_popover_for_dropdown,
                    move || {
                        window.set_appearance_mode(mode, true);
                        palette_popover_for_dropdown.popdown();
                    }
                ));
            }
        ));
        palette_box.append(&appearance_dropdown);

        let swatch_label = gtk::Label::new(Some("Swatches"));
        swatch_label.set_xalign(0.0);
        swatch_label.set_margin_top(2);
        swatch_label.add_css_class("dim-label");
        palette_box.append(&swatch_label);

        let palette_wrap = gtk::FlowBox::new();
        palette_wrap.set_selection_mode(gtk::SelectionMode::None);
        palette_wrap.set_max_children_per_line(6);
        palette_wrap.set_column_spacing(6);
        palette_wrap.set_row_spacing(6);
        palette_wrap.set_homogeneous(true);

        imp.board_color_swatches.borrow_mut().clear();
        for color_hex in BOARD_COLOR_SWATCHES {
            let swatch_button = gtk::Button::new();
            swatch_button.set_has_frame(false);
            swatch_button.set_tooltip_text(Some(color_hex));

            let swatch_frame = gtk::Frame::new(None);
            swatch_frame.add_css_class("color-chip-frame");
            let swatch_chip = self.build_color_chip(color_hex, 18);
            swatch_frame.set_child(Some(&swatch_chip));
            swatch_button.set_child(Some(&swatch_frame));
            imp.board_color_swatches
                .borrow_mut()
                .push(swatch_chip.clone());

            swatch_button.connect_clicked(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.set_board_color(color_hex, true);
                }
            ));
            palette_wrap.insert(&swatch_button, -1);
        }
        palette_box.append(&palette_wrap);

        let reset_button = gtk::Button::with_label("Reset Default");
        reset_button.add_css_class("flat");
        reset_button.set_halign(gtk::Align::End);
        reset_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.set_board_color(DEFAULT_BOARD_COLOR, true);
            }
        ));
        palette_box.append(&reset_button);

        palette_popover.set_child(Some(&palette_box));
        color_menu.set_popover(Some(&palette_popover));
        *imp.board_color_preview.borrow_mut() = Some(preview.clone());
        preview.queue_draw();
    }

    fn build_color_chip(&self, color_hex: &str, size: i32) -> gtk::DrawingArea {
        let color_hex = color_hex.to_string();
        let chip = gtk::DrawingArea::new();
        chip.set_content_width(size);
        chip.set_content_height(size);
        chip.set_draw_func(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, cr, width, height| {
                let rgba = Self::normalize_board_color(&color_hex);
                let shifted = window.shift_board_color_for_style(&rgba);
                Self::draw_color_chip(cr, width, height, &shifted);
            }
        ));
        chip
    }

    fn draw_color_chip(cr: &gtk::cairo::Context, width: i32, height: i32, color: &gdk::RGBA) {
        let w = f64::from(width.max(1));
        let h = f64::from(height.max(1));
        cr.set_source_rgba(
            f64::from(color.red()),
            f64::from(color.green()),
            f64::from(color.blue()),
            f64::from(color.alpha()),
        );
        cr.rectangle(0.0, 0.0, w, h);
        let _ = cr.fill();

        cr.set_source_rgba(0.0, 0.0, 0.0, 0.35);
        cr.rectangle(0.5, 0.5, (w - 1.0).max(0.0), (h - 1.0).max(0.0));
        let _ = cr.stroke();
    }

    fn normalize_board_color(color: &str) -> gdk::RGBA {
        gdk::RGBA::parse(color)
            .or_else(|_| gdk::RGBA::parse(DEFAULT_BOARD_COLOR))
            .unwrap_or_else(|_| gdk::RGBA::new(0.12, 0.14, 0.17, 1.0))
    }

    fn current_board_color_rgba(&self) -> gdk::RGBA {
        let color = self.imp().board_color_hex.borrow().clone();
        let normalized = Self::normalize_board_color(&color);
        self.shift_board_color_for_style(&normalized)
    }

    fn refresh_board_color_for_style(&self) {
        let color = self.imp().board_color_hex.borrow().clone();
        self.set_board_color(&color, false);
    }

    fn current_appearance_mode(&self) -> AppearanceMode {
        let settings = self.imp().settings.borrow().clone();
        settings
            .as_ref()
            .map(|settings| {
                AppearanceMode::from_setting(settings.string(SETTINGS_KEY_APPEARANCE_MODE).as_ref())
            })
            .unwrap_or(AppearanceMode::System)
    }

    fn set_appearance_mode(&self, mode: AppearanceMode, persist: bool) {
        let style = adw::StyleManager::default();
        match mode {
            AppearanceMode::System => style.set_color_scheme(adw::ColorScheme::Default),
            AppearanceMode::Light => style.set_color_scheme(adw::ColorScheme::ForceLight),
            AppearanceMode::Dark => style.set_color_scheme(adw::ColorScheme::ForceDark),
        }

        if persist {
            let settings = self.imp().settings.borrow().clone();
            if let Some(settings) = settings.as_ref() {
                let _ = settings.set_string(SETTINGS_KEY_APPEARANCE_MODE, mode.as_setting());
            }
        }
        self.refresh_board_color_for_style();
        self.refresh_board_color_chips();
    }

    fn refresh_board_color_chips(&self) {
        let swatches = self.imp().board_color_swatches.borrow().clone();
        for chip in swatches {
            chip.queue_draw();
        }
        let preview = self.imp().board_color_preview.borrow().clone();
        if let Some(preview) = preview {
            preview.queue_draw();
        }
    }

    fn shift_board_color_for_style(&self, color: &gdk::RGBA) -> gdk::RGBA {
        if adw::StyleManager::default().is_dark() {
            return *color;
        }

        if let Some(mapped) = Self::lookup_light_mode_variant(color) {
            return mapped;
        }

        let luminance = 0.2126 * color.red() + 0.7152 * color.green() + 0.0722 * color.blue();
        let white_mix = if luminance < 0.50 { 0.36 } else { 0.18 };
        let desaturate = 0.08;
        let blend = |channel: f32| {
            let gray = luminance as f32;
            let muted = channel * (1.0 - desaturate) + gray * desaturate;
            (muted * (1.0 - white_mix) + white_mix).clamp(0.0, 1.0)
        };
        gdk::RGBA::new(blend(color.red()), blend(color.green()), blend(color.blue()), 1.0)
    }

    fn lookup_light_mode_variant(color: &gdk::RGBA) -> Option<gdk::RGBA> {
        for (base_hex, light_hex) in Self::LIGHT_MODE_BOARD_VARIANTS {
            let Ok(base) = gdk::RGBA::parse(base_hex) else {
                continue;
            };
            if Self::rgba_close(color, &base) {
                if let Ok(light) = gdk::RGBA::parse(light_hex) {
                    return Some(light);
                }
            }
        }
        None
    }

    fn rgba_close(a: &gdk::RGBA, b: &gdk::RGBA) -> bool {
        const EPS: f32 = 0.015;
        (a.red() - b.red()).abs() <= EPS
            && (a.green() - b.green()).abs() <= EPS
            && (a.blue() - b.blue()).abs() <= EPS
    }

    fn set_board_color(&self, color: &str, persist: bool) {
        let imp = self.imp();
        let normalized = Self::normalize_board_color(color);
        let stored_color = normalized.to_string();
        let shifted = self.shift_board_color_for_style(&normalized);
        let css_color = shifted.to_string();

        let existing_provider = imp.board_color_provider.borrow().clone();
        let provider = if let Some(provider) = existing_provider {
            provider
        } else {
            let provider = gtk::CssProvider::new();
            gtk::style_context_add_provider_for_display(
                &self.display(),
                &provider,
                gtk::STYLE_PROVIDER_PRIORITY_APPLICATION + 1,
            );
            *imp.board_color_provider.borrow_mut() = Some(provider.clone());
            provider
        };

        provider.load_from_string(&format!(
            ".board-background {{ background-color: {}; border-radius: 12px; transition: background-color 180ms ease-in-out; }}",
            css_color
        ));

        *imp.board_color_hex.borrow_mut() = stored_color.clone();
        let preview = imp.board_color_preview.borrow().clone();
        if let Some(preview) = preview.as_ref() {
            preview.queue_draw();
        }

        if persist {
            let settings = imp.settings.borrow().clone();
            if let Some(settings) = settings.as_ref() {
                let _ = settings.set_string(SETTINGS_KEY_BOARD_COLOR, &stored_color);
            }
        }
    }
}

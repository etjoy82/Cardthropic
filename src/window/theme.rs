use super::*;

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

        for color_hex in BOARD_COLOR_SWATCHES {
            let swatch_button = gtk::Button::new();
            swatch_button.set_has_frame(false);
            swatch_button.set_tooltip_text(Some(color_hex));

            let swatch_frame = gtk::Frame::new(None);
            swatch_frame.add_css_class("color-chip-frame");
            let swatch_chip = Self::build_color_chip(color_hex, 18);
            swatch_frame.set_child(Some(&swatch_chip));
            swatch_button.set_child(Some(&swatch_frame));

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

        let palette_popover = gtk::Popover::new();
        palette_popover.set_child(Some(&palette_box));
        color_menu.set_popover(Some(&palette_popover));
        *imp.board_color_preview.borrow_mut() = Some(preview.clone());
        preview.queue_draw();
    }

    fn build_color_chip(color_hex: &str, size: i32) -> gtk::DrawingArea {
        let rgba = Self::normalize_board_color(color_hex);
        let chip = gtk::DrawingArea::new();
        chip.set_content_width(size);
        chip.set_content_height(size);
        chip.set_draw_func(move |_, cr, width, height| {
            Self::draw_color_chip(cr, width, height, &rgba);
        });
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
        Self::normalize_board_color(&color)
    }

    fn set_board_color(&self, color: &str, persist: bool) {
        let imp = self.imp();
        let normalized = Self::normalize_board_color(color);
        let css_color = normalized.to_string();

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

        *imp.board_color_hex.borrow_mut() = css_color.clone();
        let preview = imp.board_color_preview.borrow().clone();
        if let Some(preview) = preview.as_ref() {
            preview.queue_draw();
        }

        if persist {
            let settings = imp.settings.borrow().clone();
            if let Some(settings) = settings.as_ref() {
                let _ = settings.set_string(SETTINGS_KEY_BOARD_COLOR, &css_color);
            }
        }
    }
}

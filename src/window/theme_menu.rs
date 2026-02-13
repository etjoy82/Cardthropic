use super::*;

impl CardthropicWindow {
    fn normalize_pasted_svg(input: &str) -> Result<String, String> {
        let trimmed = input.trim();
        if trimmed.is_empty() {
            return Err("Clipboard did not contain SVG text.".to_string());
        }

        let lower = trimmed.to_ascii_lowercase();
        let Some(start) = lower.find("<svg") else {
            return Err("Clipboard content is not an SVG document.".to_string());
        };
        let Some(end_tag_start) = lower.rfind("</svg>") else {
            return Err("Clipboard SVG is missing a closing </svg> tag.".to_string());
        };
        let end = end_tag_start + "</svg>".len();
        if end <= start || end > trimmed.len() {
            return Err("Clipboard SVG bounds are invalid.".to_string());
        }

        Ok(trimmed[start..end].to_string())
    }

    fn apply_custom_card_svg_from_text(&self, svg: &str) -> Result<(), String> {
        let normalized_svg = Self::normalize_pasted_svg(svg)?;
        if normalized_svg.len() > 4 * 1024 * 1024 {
            return Err("SVG is too large (max 4 MiB).".to_string());
        }

        AngloDeck::load_with_custom_normal_svg(&normalized_svg)?;
        if let Some(settings) = self.imp().settings.borrow().as_ref() {
            let _ = settings.set_string(SETTINGS_KEY_CUSTOM_CARD_SVG, &normalized_svg);
        }

        self.imp().deck_load_attempted.set(false);
        *self.imp().deck.borrow_mut() = None;
        *self.imp().deck_error.borrow_mut() = None;
        *self.imp().status_override.borrow_mut() =
            Some("Card design updated from clipboard SVG.".to_string());
        self.render();
        Ok(())
    }

    fn reset_custom_card_svg(&self) {
        if let Some(settings) = self.imp().settings.borrow().as_ref() {
            let _ = settings.set_string(SETTINGS_KEY_CUSTOM_CARD_SVG, "");
        }
        self.imp().deck_load_attempted.set(false);
        *self.imp().deck.borrow_mut() = None;
        *self.imp().deck_error.borrow_mut() = None;
        *self.imp().status_override.borrow_mut() =
            Some("Card design reset to default.".to_string());
        self.render();
    }

    #[allow(deprecated)]
    pub(super) fn setup_board_color_dropdown(&self) {
        let imp = self.imp();
        let color_menu = imp.board_color_menu_button.get();
        color_menu.set_tooltip_text(Some("Theme presets"));
        color_menu.set_has_frame(true);
        color_menu.add_css_class("board-color-menu-button");
        color_menu.set_label("🎨");

        let palette_box = gtk::Box::new(gtk::Orientation::Vertical, 6);
        palette_box.set_margin_top(8);
        palette_box.set_margin_bottom(8);
        palette_box.set_margin_start(8);
        palette_box.set_margin_end(8);
        let palette_popover = gtk::Popover::new();
        palette_popover.set_has_arrow(true);
        palette_popover.set_width_request(260);

        let theme_label = gtk::Label::new(Some("Theme Presets"));
        theme_label.set_xalign(0.0);
        theme_label.add_css_class("dim-label");
        palette_box.append(&theme_label);

        let preset_list = gtk::ListBox::new();
        preset_list.set_selection_mode(gtk::SelectionMode::Single);
        preset_list.add_css_class("boxed-list");
        preset_list.set_hexpand(true);
        preset_list.set_vexpand(true);

        let preset_names = Self::userstyle_preset_names();
        for (index, name) in preset_names.iter().enumerate() {
            let row = gtk::ListBoxRow::new();
            row.set_selectable(true);
            row.set_activatable(true);
            row.set_tooltip_text(Some(if index == 0 {
                "Open CSS editor"
            } else {
                "Apply preset"
            }));
            let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 8);
            row_box.set_margin_top(8);
            row_box.set_margin_bottom(8);
            row_box.set_margin_start(10);
            row_box.set_margin_end(10);
            let label = gtk::Label::new(Some(name));
            label.set_xalign(0.0);
            label.set_hexpand(true);
            row_box.append(&label);
            row.set_child(Some(&row_box));
            preset_list.append(&row);
        }

        let selected_idx =
            Self::userstyle_preset_for_css(&self.imp().custom_userstyle_css.borrow());
        if let Some(row) = preset_list.row_at_index(selected_idx as i32) {
            preset_list.select_row(Some(&row));
        }

        let palette_popover_for_theme_list = palette_popover.clone();
        preset_list.connect_row_activated(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, row| {
                let selected = row.index() as u32;
                glib::idle_add_local_once(glib::clone!(
                    #[weak]
                    window,
                    #[weak]
                    palette_popover_for_theme_list,
                    move || {
                        palette_popover_for_theme_list.popdown();
                        if selected == 0 {
                            window.open_custom_userstyle_dialog();
                        } else {
                            window.apply_userstyle_preset(selected, true);
                        }
                    }
                ));
            }
        ));
        palette_box.append(&preset_list);

        let bottom_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        bottom_row.set_halign(gtk::Align::End);
        let custom_userstyle_button = gtk::Button::with_label("Custom CSS Userstyle");
        custom_userstyle_button.add_css_class("flat");
        custom_userstyle_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            palette_popover,
            move |_| {
                palette_popover.popdown();
                window.open_custom_userstyle_dialog();
            }
        ));
        bottom_row.append(&custom_userstyle_button);

        let reset_button = gtk::Button::with_label("Reset CSS to Default");
        reset_button.add_css_class("flat");
        reset_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.apply_custom_userstyle(Self::default_userstyle_css(), true);
            }
        ));
        bottom_row.append(&reset_button);
        palette_box.append(&bottom_row);

        let card_design_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        card_design_row.set_halign(gtk::Align::End);

        let paste_svg_button = gtk::Button::with_label("Paste SVG as Card Design");
        paste_svg_button.add_css_class("flat");
        paste_svg_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |button| {
                button.set_sensitive(false);
                button.set_label("Pasting SVG...");
                *window.imp().status_override.borrow_mut() =
                    Some("Applying pasted SVG card design...".to_string());
                window.render();

                let clipboard = window.clipboard();
                clipboard.read_text_async(
                    None::<&gio::Cancellable>,
                    glib::clone!(
                        #[weak]
                        window,
                        #[weak]
                        button,
                        move |result| {
                            button.set_sensitive(true);
                            button.set_label("Paste SVG as Card Design");
                            match result {
                                Ok(Some(text)) => {
                                    if let Err(err) =
                                        window.apply_custom_card_svg_from_text(text.as_str())
                                    {
                                        *window.imp().status_override.borrow_mut() =
                                            Some(format!("Paste SVG failed: {err}"));
                                        window.render();
                                    }
                                }
                                Ok(None) => {
                                    *window.imp().status_override.borrow_mut() =
                                        Some("Paste SVG failed: clipboard is empty.".to_string());
                                    window.render();
                                }
                                Err(err) => {
                                    *window.imp().status_override.borrow_mut() =
                                        Some(format!("Paste SVG failed: {err}"));
                                    window.render();
                                }
                            }
                        }
                    ),
                );
            }
        ));
        card_design_row.append(&paste_svg_button);

        let reset_card_design_button = gtk::Button::with_label("Reset Card Design");
        reset_card_design_button.add_css_class("flat");
        reset_card_design_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.reset_custom_card_svg();
            }
        ));
        card_design_row.append(&reset_card_design_button);
        palette_box.append(&card_design_row);

        let card_design_hint = gtk::Label::new(Some(
            "Card SVG paste expects a full deck sheet layout compatible with anglo.svg.",
        ));
        card_design_hint.set_xalign(0.0);
        card_design_hint.set_wrap(true);
        card_design_hint.add_css_class("dim-label");
        palette_box.append(&card_design_hint);

        palette_popover.set_child(Some(&palette_box));
        color_menu.set_popover(Some(&palette_popover));
    }
}

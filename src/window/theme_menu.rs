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

    fn apply_custom_card_svg_from_text_async(&self, svg: String, status_prefix: &'static str) {
        *self.imp().status_override.borrow_mut() = Some(format!("{status_prefix}..."));
        self.render();

        let (sender, receiver) = mpsc::channel::<Result<String, String>>();
        thread::spawn(move || {
            let result = (|| {
                let normalized_svg = Self::normalize_pasted_svg(&svg)?;
                if normalized_svg.len() > 4 * 1024 * 1024 {
                    return Err("SVG is too large (max 4 MiB).".to_string());
                }
                AngloDeck::load_with_custom_normal_svg(&normalized_svg)?;
                Ok(normalized_svg)
            })();
            let _ = sender.send(result);
        });

        glib::timeout_add_local(
            Duration::from_millis(24),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || match receiver.try_recv() {
                    Ok(Ok(normalized_svg)) => {
                        if let Some(settings) = window.imp().settings.borrow().as_ref() {
                            let _ = settings.set_string(SETTINGS_KEY_CUSTOM_CARD_SVG, &normalized_svg);
                        }
                        window.imp().deck_load_attempted.set(false);
                        *window.imp().deck.borrow_mut() = None;
                        *window.imp().deck_error.borrow_mut() = None;
                        window.invalidate_card_render_cache();
                        *window.imp().status_override.borrow_mut() =
                            Some("Card design updated from SVG.".to_string());
                        window.render();
                        glib::ControlFlow::Break
                    }
                    Ok(Err(err)) => {
                        *window.imp().status_override.borrow_mut() =
                            Some(format!("Load SVG failed: {err}"));
                        window.render();
                        glib::ControlFlow::Break
                    }
                    Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                    Err(mpsc::TryRecvError::Disconnected) => {
                        *window.imp().status_override.borrow_mut() =
                            Some("Load SVG failed: worker disconnected.".to_string());
                        window.render();
                        glib::ControlFlow::Break
                    }
                }
            ),
        );
    }

    fn reset_custom_card_svg(&self) {
        if let Some(settings) = self.imp().settings.borrow().as_ref() {
            let _ = settings.set_string(SETTINGS_KEY_CUSTOM_CARD_SVG, "");
        }
        self.imp().deck_load_attempted.set(false);
        *self.imp().deck.borrow_mut() = None;
        *self.imp().deck_error.borrow_mut() = None;
        self.invalidate_card_render_cache();
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
        palette_box.set_hexpand(true);
        palette_box.set_vexpand(true);

        let row_count = (Self::userstyle_preset_names().len().saturating_sub(1)) as i32;
        let estimated_height = 240 + (row_count * 26);
        let dialog_height = estimated_height.clamp(460, 820);

        let palette_window = gtk::Window::builder()
            .title("Theme Presets")
            .resizable(true)
            .default_width(620)
            .default_height(dialog_height)
            .build();
        palette_window.set_transient_for(Some(self));
        palette_window.set_modal(false);
        palette_window.set_hide_on_close(true);
        palette_window.set_destroy_with_parent(true);

        let theme_label = gtk::Label::new(Some("Theme Presets"));
        theme_label.set_xalign(0.0);
        theme_label.add_css_class("dim-label");
        palette_box.append(&theme_label);

        let preset_label = gtk::Label::new(Some("Theme"));
        preset_label.set_xalign(0.0);
        preset_label.add_css_class("dim-label");
        palette_box.append(&preset_label);

        let preset_names = Self::userstyle_preset_names();
        let display_preset_names: Vec<String> = preset_names
            .iter()
            .skip(1)
            .map(|name| (*name).to_string())
            .collect();

        let preset_list = gtk::ListBox::new();
        preset_list.add_css_class("boxed-list");
        preset_list.set_selection_mode(gtk::SelectionMode::Single);
        preset_list.set_hexpand(true);
        preset_list.set_vexpand(true);

        for name in &display_preset_names {
            let row = gtk::ListBoxRow::new();
            let label = gtk::Label::new(Some(name));
            label.set_xalign(0.0);
            label.set_margin_top(8);
            label.set_margin_bottom(8);
            label.set_margin_start(10);
            label.set_margin_end(10);
            row.set_child(Some(&label));
            preset_list.append(&row);
        }

        let theme_filter_entry = gtk::SearchEntry::new();
        theme_filter_entry.set_placeholder_text(Some("Filter themes..."));
        theme_filter_entry.set_hexpand(true);
        palette_box.append(&theme_filter_entry);

        let selected_preset_idx =
            Self::userstyle_preset_for_css(&self.imp().custom_userstyle_css.borrow());
        if selected_preset_idx > 0 {
            if let Some(row) = preset_list.row_at_index((selected_preset_idx - 1) as i32) {
                preset_list.select_row(Some(&row));
            }
        }

        let preset_list_scroll = gtk::ScrolledWindow::new();
        preset_list_scroll.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        preset_list_scroll.set_hexpand(true);
        preset_list_scroll.set_vexpand(true);
        preset_list_scroll.set_child(Some(&preset_list));
        palette_box.append(&preset_list_scroll);

        theme_filter_entry.connect_search_changed(glib::clone!(
            #[weak]
            preset_list,
            #[strong]
            display_preset_names,
            move |entry| {
                let query = entry.text().to_string().to_ascii_lowercase();
                let mut first_visible: Option<gtk::ListBoxRow> = None;
                let mut current_selected_visible = false;
                let current_selected_index = preset_list.selected_row().map(|row| row.index());

                for (idx, name) in display_preset_names.iter().enumerate() {
                    let Some(row) = preset_list.row_at_index(idx as i32) else {
                        continue;
                    };
                    let visible = query.is_empty() || name.to_ascii_lowercase().contains(&query);
                    row.set_visible(visible);
                    if visible {
                        if first_visible.is_none() {
                            first_visible = Some(row.clone());
                        }
                        if current_selected_index == Some(idx as i32) {
                            current_selected_visible = true;
                        }
                    }
                }

                if !current_selected_visible {
                    preset_list.select_row(first_visible.as_ref());
                }
            }
        ));

        preset_list.connect_row_selected(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, row| {
                let Some(row) = row else {
                    return;
                };
                let preset_index = u32::try_from(row.index()).unwrap_or(0) + 1;
                window.apply_userstyle_preset(preset_index, true);
            }
        ));

        let bottom_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        bottom_row.set_halign(gtk::Align::End);
        let copy_preset_button = gtk::Button::with_label("Copy Preset CSS");
        copy_preset_button.add_css_class("flat");
        copy_preset_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            preset_list,
            move |_| {
                let Some(row) = preset_list.selected_row() else {
                    *window.imp().status_override.borrow_mut() =
                        Some("Select a preset first, then copy.".to_string());
                    window.render();
                    return;
                };
                let preset_index = u32::try_from(row.index()).unwrap_or(0) + 1;
                let Some(css) = Self::userstyle_css_for_preset(preset_index) else {
                    *window.imp().status_override.borrow_mut() =
                        Some("Copy failed: preset has no CSS template.".to_string());
                    window.render();
                    return;
                };
                window.clipboard().set_text(css);
                *window.imp().status_override.borrow_mut() =
                    Some("Copied preset CSS to clipboard.".to_string());
                window.render();
            }
        ));
        bottom_row.append(&copy_preset_button);

        let activate_custom_button = gtk::Button::with_label("Activate Custom CSS");
        activate_custom_button.add_css_class("flat");
        activate_custom_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                let saved_css = window.imp().saved_custom_userstyle_css.borrow().clone();
                if saved_css.trim().is_empty() {
                    *window.imp().status_override.borrow_mut() =
                        Some("No saved Custom CSS yet. Open editor to create one.".to_string());
                    window.render();
                    return;
                }
                window.apply_custom_userstyle(&saved_css, true);
            }
        ));
        bottom_row.append(&activate_custom_button);

        let custom_userstyle_button = gtk::Button::with_label("Custom CSS Userstyle");
        custom_userstyle_button.add_css_class("flat");
        custom_userstyle_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
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

        let load_svg_button = gtk::Button::with_label("Load SVG File...");
        load_svg_button.add_css_class("flat");
        load_svg_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                let file_dialog = gtk::FileDialog::builder()
                    .title("Load Card Design SVG")
                    .modal(true)
                    .build();
                let filter = gtk::FileFilter::new();
                filter.set_name(Some("SVG files"));
                filter.add_pattern("*.svg");
                filter.add_mime_type("image/svg+xml");
                filter.add_mime_type("text/plain");
                let filters = gio::ListStore::new::<gtk::FileFilter>();
                filters.append(&filter);
                file_dialog.set_filters(Some(&filters));
                file_dialog.set_default_filter(Some(&filter));

                file_dialog.open(
                    Some(window.upcast_ref::<gtk::Window>()),
                    None::<&gio::Cancellable>,
                    glib::clone!(
                        #[weak(rename_to = window)]
                        window,
                        move |result: Result<gio::File, glib::Error>| match result {
                            Ok(file) => {
                                file.load_contents_async(
                                    None::<&gio::Cancellable>,
                                    glib::clone!(
                                        #[weak(rename_to = window)]
                                        window,
                                        move |result| match result {
                                            Ok((contents, _)) => {
                                                let svg_text = String::from_utf8_lossy(
                                                    contents.as_ref(),
                                                )
                                                .to_string();
                                                window.apply_custom_card_svg_from_text_async(
                                                    svg_text,
                                                    "Loading SVG card design",
                                                );
                                            }
                                            Err(err) => {
                                                *window.imp().status_override.borrow_mut() =
                                                    Some(format!("Load SVG failed: {err}"));
                                                window.render();
                                            }
                                        }
                                    ),
                                );
                            }
                            Err(err) => {
                                if err.matches(gio::IOErrorEnum::Cancelled) {
                                    return;
                                }
                                *window.imp().status_override.borrow_mut() =
                                    Some(format!("Load SVG failed: {err}"));
                                window.render();
                            }
                        }
                    ),
                );
            }
        ));
        card_design_row.append(&load_svg_button);

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
                                    window.apply_custom_card_svg_from_text_async(
                                        text.to_string(),
                                        "Applying pasted SVG card design",
                                    );
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

        palette_window.set_child(Some(&palette_box));
        palette_window.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                *window.imp().theme_presets_window.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));
        *self.imp().theme_presets_window.borrow_mut() = Some(palette_window.clone());

        color_menu.connect_clicked(glib::clone!(
            #[weak]
            palette_window,
            move |_| {
                palette_window.present();
            }
        ));
    }
}

use super::*;

impl CardthropicWindow {
    pub(super) fn show_theme_presets_window(&self) {
        if self.imp().theme_presets_window.borrow().is_none() {
            self.setup_board_color_dropdown();
        }
        if let Some(window) = self.imp().theme_presets_window.borrow().as_ref() {
            window.present();
        }
    }

    fn theme_preset_matches_query(name: &str, query: &str) -> bool {
        let trimmed = query.trim();
        if trimmed.is_empty() {
            return true;
        }

        let lowered_name = name.to_ascii_lowercase();
        let lowered_query = trimmed.to_ascii_lowercase();
        if lowered_name.contains(&lowered_query) {
            return true;
        }

        let query_chars: Vec<char> = trimmed
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .map(|ch| ch.to_ascii_lowercase())
            .collect();
        if query_chars.is_empty() {
            return true;
        }

        let mut query_idx = 0usize;
        for ch in name.chars() {
            if query_idx >= query_chars.len() {
                break;
            }
            if ch.to_ascii_lowercase() == query_chars[query_idx] {
                query_idx += 1;
            }
        }
        query_idx == query_chars.len()
    }

    fn visible_theme_preset_rows(listbox: &gtk::ListBox) -> Vec<gtk::ListBoxRow> {
        let mut rows = Vec::new();
        let mut index = 0;
        while let Some(row) = listbox.row_at_index(index) {
            if row.is_visible() {
                rows.push(row);
            }
            index += 1;
        }
        rows
    }

    fn ensure_selected_theme_preset_visible(
        listbox: &gtk::ListBox,
        scroller: &gtk::ScrolledWindow,
    ) {
        let Some(row) = listbox.selected_row().filter(|row| row.is_visible()) else {
            return;
        };
        let Some(bounds) = row.compute_bounds(listbox) else {
            return;
        };

        let adj = scroller.vadjustment();
        let current = adj.value();
        let page = adj.page_size().max(1.0);
        let top = f64::from(bounds.y());
        let bottom = f64::from(bounds.y() + bounds.height());
        let min = adj.lower();
        let max = (adj.upper() - page).max(min);
        let target = if top < current {
            top
        } else if bottom > current + page {
            bottom - page
        } else {
            current
        }
        .clamp(min, max);

        if (target - current).abs() > f64::EPSILON {
            adj.set_value(target);
        }
    }

    fn select_next_visible_theme_preset_row(
        listbox: &gtk::ListBox,
        direction: i32,
        scroller: Option<&gtk::ScrolledWindow>,
    ) {
        let visible = Self::visible_theme_preset_rows(listbox);
        if visible.is_empty() {
            listbox.unselect_all();
            return;
        }

        let current = listbox.selected_row().and_then(|selected| {
            visible
                .iter()
                .position(|row| row.index() == selected.index())
        });
        let next_index = match current {
            Some(current_idx) => {
                if direction >= 0 {
                    (current_idx + 1).min(visible.len().saturating_sub(1))
                } else {
                    current_idx.saturating_sub(1)
                }
            }
            None => {
                if direction >= 0 {
                    0
                } else {
                    visible.len().saturating_sub(1)
                }
            }
        };

        if let Some(next) = visible.get(next_index) {
            listbox.select_row(Some(next));
            if let Some(scroller) = scroller {
                Self::ensure_selected_theme_preset_visible(listbox, scroller);
            }
        }
    }

    fn select_theme_preset_row_home_end(
        listbox: &gtk::ListBox,
        to_end: bool,
        scroller: Option<&gtk::ScrolledWindow>,
    ) {
        let visible = Self::visible_theme_preset_rows(listbox);
        if visible.is_empty() {
            listbox.unselect_all();
            return;
        }
        let target = if to_end {
            visible.last()
        } else {
            visible.first()
        };
        if let Some(row) = target {
            listbox.select_row(Some(row));
            if let Some(scroller) = scroller {
                Self::ensure_selected_theme_preset_visible(listbox, scroller);
            }
        }
    }

    fn page_step_for_theme_preset_list(
        listbox: &gtk::ListBox,
        visible: &[gtk::ListBoxRow],
    ) -> usize {
        if visible.is_empty() {
            return 1;
        }
        let fallback = 8usize.min(visible.len().max(1));
        let viewport_height = listbox.height();
        let row_height = visible.first().map(|row| row.height()).unwrap_or(0).max(1);
        if viewport_height <= 1 {
            return fallback;
        }
        let step = (viewport_height / row_height).max(1) as usize;
        step.min(visible.len().max(1))
    }

    fn select_theme_preset_row_page(
        listbox: &gtk::ListBox,
        direction: i32,
        scroller: Option<&gtk::ScrolledWindow>,
    ) {
        let visible = Self::visible_theme_preset_rows(listbox);
        if visible.is_empty() {
            listbox.unselect_all();
            return;
        }

        let page = Self::page_step_for_theme_preset_list(listbox, &visible);
        let current = listbox.selected_row().and_then(|selected| {
            visible
                .iter()
                .position(|row| row.index() == selected.index())
        });
        let current_idx = current.unwrap_or_else(|| {
            if direction >= 0 {
                0
            } else {
                visible.len().saturating_sub(1)
            }
        });
        let next_index = if direction >= 0 {
            current_idx
                .saturating_add(page)
                .min(visible.len().saturating_sub(1))
        } else {
            current_idx.saturating_sub(page)
        };

        if let Some(row) = visible.get(next_index) {
            listbox.select_row(Some(row));
            if let Some(scroller) = scroller {
                Self::ensure_selected_theme_preset_visible(listbox, scroller);
            }
        }
    }

    fn theme_presets_window_size(&self, default_height: i32) -> (i32, i32) {
        const DEFAULT_WIDTH: i32 = 620;
        const MIN_WIDTH: i32 = 420;
        const MIN_HEIGHT: i32 = 320;

        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return (DEFAULT_WIDTH, default_height.max(MIN_HEIGHT));
        };
        let Some(schema) = settings.settings_schema() else {
            return (DEFAULT_WIDTH, default_height.max(MIN_HEIGHT));
        };
        if !schema.has_key(SETTINGS_KEY_THEME_PRESETS_WIDTH)
            || !schema.has_key(SETTINGS_KEY_THEME_PRESETS_HEIGHT)
        {
            return (DEFAULT_WIDTH, default_height.max(MIN_HEIGHT));
        }

        (
            settings
                .int(SETTINGS_KEY_THEME_PRESETS_WIDTH)
                .max(MIN_WIDTH),
            settings
                .int(SETTINGS_KEY_THEME_PRESETS_HEIGHT)
                .max(MIN_HEIGHT),
        )
    }

    fn theme_presets_maximized(&self) -> bool {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return false;
        };
        let Some(schema) = settings.settings_schema() else {
            return false;
        };
        if !schema.has_key(SETTINGS_KEY_THEME_PRESETS_MAXIMIZED) {
            return false;
        }
        settings.boolean(SETTINGS_KEY_THEME_PRESETS_MAXIMIZED)
    }

    fn persist_theme_presets_maximized(&self, maximized: bool) {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_THEME_PRESETS_MAXIMIZED) {
            return;
        }
        if settings.boolean(SETTINGS_KEY_THEME_PRESETS_MAXIMIZED) != maximized {
            let _ = settings.set_boolean(SETTINGS_KEY_THEME_PRESETS_MAXIMIZED, maximized);
        }
    }

    fn persist_theme_presets_window_size(&self, dialog: &gtk::Window) {
        const MIN_WIDTH: i32 = 420;
        const MIN_HEIGHT: i32 = 320;

        if dialog.is_maximized() {
            return;
        }

        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_THEME_PRESETS_WIDTH)
            || !schema.has_key(SETTINGS_KEY_THEME_PRESETS_HEIGHT)
        {
            return;
        }

        let width = dialog.width().max(MIN_WIDTH);
        let height = dialog.height().max(MIN_HEIGHT);
        if settings.int(SETTINGS_KEY_THEME_PRESETS_WIDTH) != width {
            let _ = settings.set_int(SETTINGS_KEY_THEME_PRESETS_WIDTH, width);
        }
        if settings.int(SETTINGS_KEY_THEME_PRESETS_HEIGHT) != height {
            let _ = settings.set_int(SETTINGS_KEY_THEME_PRESETS_HEIGHT, height);
        }
    }

    #[allow(deprecated)]
    pub(super) fn setup_board_color_dropdown(&self) {
        let imp = self.imp();
        let color_menu = imp.board_color_menu_button.get();
        color_menu.set_tooltip_text(Some("Theme presets"));
        color_menu.set_has_frame(true);
        color_menu.add_css_class("board-color-menu-button");
        color_menu.set_label(&self.main_button_label_with_shortcut("🎨", "win.open-theme-presets"));

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
        let (saved_width, saved_height) = self.theme_presets_window_size(dialog_height);
        let saved_maximized = self.theme_presets_maximized();

        let palette_window = gtk::Window::builder()
            .title("Theme Presets")
            .resizable(true)
            .default_width(saved_width)
            .default_height(saved_height)
            .build();
        palette_window.set_transient_for(Some(self));
        palette_window.set_modal(false);
        palette_window.set_hide_on_close(false);
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

        let interface_font_label = gtk::Label::new(Some("Interface and Emoji Font"));
        interface_font_label.set_xalign(0.0);
        interface_font_label.add_css_class("dim-label");
        palette_box.append(&interface_font_label);

        let mut family_names: Vec<String> = self
            .imp()
            .status_label
            .pango_context()
            .list_families()
            .into_iter()
            .map(|family| family.name().to_string())
            .collect();
        family_names.sort_unstable();
        family_names.dedup();

        let default_family = self.default_interface_font_family();
        let mut font_options = Vec::with_capacity(family_names.len() + 1);
        font_options.push(format!("App Default ({})", default_family));
        font_options.extend(family_names.clone());
        let font_option_refs: Vec<&str> = font_options.iter().map(String::as_str).collect();
        let interface_font_dropdown = gtk::DropDown::from_strings(&font_option_refs);
        interface_font_dropdown.set_hexpand(true);
        let interface_font_row = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        interface_font_row.set_hexpand(true);
        interface_font_row.append(&interface_font_dropdown);

        let reset_interface_font_button = gtk::Button::with_label("Reset");
        reset_interface_font_button.add_css_class("flat");
        interface_font_row.append(&reset_interface_font_button);

        let saved_font = self
            .imp()
            .settings
            .borrow()
            .as_ref()
            .map(|settings| {
                settings
                    .string(SETTINGS_KEY_INTERFACE_EMOJI_FONT)
                    .to_string()
            })
            .unwrap_or_default();
        let selected_index = if saved_font.trim().is_empty() {
            0
        } else {
            family_names
                .iter()
                .position(|name| name == &saved_font)
                .map(|idx| idx + 1)
                .unwrap_or(0)
        };
        interface_font_dropdown.set_selected(selected_index as u32);
        palette_box.append(&interface_font_row);

        interface_font_dropdown.connect_selected_notify(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[strong]
            family_names,
            move |dropdown| {
                let selected = dropdown.selected() as usize;
                if selected == 0 {
                    window.apply_interface_emoji_font(None, true);
                } else if let Some(name) = family_names.get(selected.saturating_sub(1)) {
                    window.apply_interface_emoji_font(Some(name.as_str()), true);
                }
                window.render();
            }
        ));

        reset_interface_font_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            interface_font_dropdown,
            move |_| {
                interface_font_dropdown.set_selected(0);
                window.apply_interface_emoji_font(None, true);
                window.render();
            }
        ));

        theme_filter_entry.connect_search_changed(glib::clone!(
            #[weak]
            preset_list,
            #[weak]
            preset_list_scroll,
            #[strong]
            display_preset_names,
            move |entry| {
                let query = entry.text().to_string();
                let mut first_visible: Option<gtk::ListBoxRow> = None;
                let mut current_selected_visible = false;
                let current_selected_index = preset_list.selected_row().map(|row| row.index());

                for (idx, name) in display_preset_names.iter().enumerate() {
                    let Some(row) = preset_list.row_at_index(idx as i32) else {
                        continue;
                    };
                    let visible = Self::theme_preset_matches_query(name, query.as_str());
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
                Self::ensure_selected_theme_preset_visible(&preset_list, &preset_list_scroll);
            }
        ));

        let theme_filter_keys = gtk::EventControllerKey::new();
        theme_filter_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        theme_filter_keys.connect_key_pressed(glib::clone!(
            #[weak]
            preset_list,
            #[weak]
            preset_list_scroll,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| match key {
                gdk::Key::Down | gdk::Key::KP_Down => {
                    Self::select_next_visible_theme_preset_row(
                        &preset_list,
                        1,
                        Some(&preset_list_scroll),
                    );
                    glib::Propagation::Stop
                }
                gdk::Key::Up | gdk::Key::KP_Up => {
                    Self::select_next_visible_theme_preset_row(
                        &preset_list,
                        -1,
                        Some(&preset_list_scroll),
                    );
                    glib::Propagation::Stop
                }
                gdk::Key::Home | gdk::Key::KP_Home => {
                    Self::select_theme_preset_row_home_end(
                        &preset_list,
                        false,
                        Some(&preset_list_scroll),
                    );
                    glib::Propagation::Stop
                }
                gdk::Key::End | gdk::Key::KP_End => {
                    Self::select_theme_preset_row_home_end(
                        &preset_list,
                        true,
                        Some(&preset_list_scroll),
                    );
                    glib::Propagation::Stop
                }
                gdk::Key::Page_Down | gdk::Key::KP_Page_Down => {
                    Self::select_theme_preset_row_page(&preset_list, 1, Some(&preset_list_scroll));
                    glib::Propagation::Stop
                }
                gdk::Key::Page_Up | gdk::Key::KP_Page_Up => {
                    Self::select_theme_preset_row_page(&preset_list, -1, Some(&preset_list_scroll));
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        ));
        theme_filter_entry.add_controller(theme_filter_keys);

        let preset_list_keys = gtk::EventControllerKey::new();
        preset_list_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        preset_list_keys.connect_key_pressed(glib::clone!(
            #[weak]
            preset_list,
            #[weak]
            preset_list_scroll,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| match key {
                gdk::Key::Home | gdk::Key::KP_Home => {
                    Self::select_theme_preset_row_home_end(
                        &preset_list,
                        false,
                        Some(&preset_list_scroll),
                    );
                    glib::Propagation::Stop
                }
                gdk::Key::End | gdk::Key::KP_End => {
                    Self::select_theme_preset_row_home_end(
                        &preset_list,
                        true,
                        Some(&preset_list_scroll),
                    );
                    glib::Propagation::Stop
                }
                gdk::Key::Page_Down | gdk::Key::KP_Page_Down => {
                    Self::select_theme_preset_row_page(&preset_list, 1, Some(&preset_list_scroll));
                    glib::Propagation::Stop
                }
                gdk::Key::Page_Up | gdk::Key::KP_Page_Up => {
                    Self::select_theme_preset_row_page(&preset_list, -1, Some(&preset_list_scroll));
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        ));
        preset_list.add_controller(preset_list_keys);

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

        let bottom_rows = gtk::Box::new(gtk::Orientation::Vertical, 6);
        bottom_rows.set_halign(gtk::Align::End);
        let bottom_row_top = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        bottom_row_top.set_halign(gtk::Align::End);
        let bottom_row_bottom = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        bottom_row_bottom.set_halign(gtk::Align::End);
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
        bottom_row_top.append(&copy_preset_button);

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
        bottom_row_top.append(&activate_custom_button);

        let custom_userstyle_button = gtk::Button::with_label("Custom CSS Userstyle");
        custom_userstyle_button.add_css_class("flat");
        custom_userstyle_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.open_custom_userstyle_dialog();
            }
        ));
        bottom_row_bottom.append(&custom_userstyle_button);

        let reset_button = gtk::Button::with_label("Reset CSS to Default");
        reset_button.add_css_class("flat");
        reset_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.apply_custom_userstyle(Self::default_userstyle_css(), true);
            }
        ));
        bottom_row_bottom.append(&reset_button);
        bottom_rows.append(&bottom_row_top);
        bottom_rows.append(&bottom_row_bottom);
        palette_box.append(&bottom_rows);

        palette_window.set_child(Some(&palette_box));
        palette_window.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |palette_window| {
                let maximized = palette_window.is_maximized();
                window.persist_theme_presets_maximized(maximized);
                window.persist_theme_presets_window_size(palette_window);
                *window.imp().theme_presets_window.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));
        if saved_maximized {
            palette_window.maximize();
        }
        let palette_keys = gtk::EventControllerKey::new();
        palette_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        palette_keys.connect_key_pressed(glib::clone!(
            #[weak]
            palette_window,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                if key == gdk::Key::Escape {
                    palette_window.close();
                    return glib::Propagation::Stop;
                }
                glib::Propagation::Proceed
            }
        ));
        palette_window.add_controller(palette_keys);
        *self.imp().theme_presets_window.borrow_mut() = Some(palette_window.clone());
    }
}

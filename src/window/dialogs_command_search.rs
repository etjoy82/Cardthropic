use super::*;
use std::collections::BTreeSet;
use std::rc::Rc;

#[derive(Clone, Debug)]
struct CommandSearchItem {
    label: String,
    action: String,
    path: String,
    shortcut: Option<String>,
    haystack: String,
}

impl CardthropicWindow {
    fn command_item_prefix_text(item: &CommandSearchItem) -> String {
        let mut parts = vec![item.label.clone()];
        if !item.path.is_empty() {
            parts.push(item.path.clone());
        }
        if let Some(shortcut) = item.shortcut.as_ref() {
            parts.push(shortcut.clone());
        }
        parts.join(" | ")
    }

    fn command_search_items_from_main_menu(&self) -> Vec<CommandSearchItem> {
        let Some(model) = self.imp().main_menu_popover.menu_model() else {
            return Vec::new();
        };
        let mut out = Vec::new();
        Self::collect_command_search_items(&model, &[], &mut out);
        for item in out.iter_mut() {
            item.shortcut = self.command_shortcut_label_for_action(&item.action);
            let prefix = Self::command_item_prefix_text(item);
            item.haystack = format!(
                "{} {}",
                prefix.to_ascii_lowercase(),
                item.action.to_ascii_lowercase()
            );
        }
        out
    }

    fn command_shortcut_label_for_action(&self, detailed_action: &str) -> Option<String> {
        let labels = self.shortcut_labels_for_action(detailed_action);

        if labels.is_empty() {
            None
        } else {
            Some(labels.join(" / "))
        }
    }

    fn collect_command_search_items(
        model: &gio::MenuModel,
        path: &[String],
        out: &mut Vec<CommandSearchItem>,
    ) {
        for index in 0..model.n_items() {
            let label = model
                .item_attribute_value(index, "label", None)
                .and_then(|value| value.get::<String>());

            if let Some(section) = model.item_link(index, "section") {
                Self::collect_command_search_items(&section, path, out);
            }

            if let Some(submenu) = model.item_link(index, "submenu") {
                let mut next_path = path.to_vec();
                if let Some(label) = label.as_ref() {
                    next_path.push(label.clone());
                }
                Self::collect_command_search_items(&submenu, &next_path, out);
                continue;
            }

            let Some(label) = label else {
                continue;
            };
            let Some(action) = model
                .item_attribute_value(index, "action", None)
                .and_then(|value| value.get::<String>())
            else {
                continue;
            };

            let path_text = path.join(" > ");
            let haystack = if path_text.is_empty() {
                format!(
                    "{} {}",
                    label.to_ascii_lowercase(),
                    action.to_ascii_lowercase()
                )
            } else {
                format!(
                    "{} {} {}",
                    label.to_ascii_lowercase(),
                    path_text.to_ascii_lowercase(),
                    action.to_ascii_lowercase()
                )
            };
            out.push(CommandSearchItem {
                label,
                action,
                path: path_text,
                shortcut: None,
                haystack,
            });
        }
    }

    fn command_search_row_labels(row: &gtk::ListBoxRow) -> Option<(gtk::Label, gtk::Label)> {
        let row_box = row.child()?.downcast::<gtk::Box>().ok()?;
        let main = row_box.first_child()?.downcast::<gtk::Label>().ok()?;
        let action = row_box.last_child()?.downcast::<gtk::Label>().ok()?;
        Some((main, action))
    }

    fn fuzzy_match_positions(text: &str, query: &str) -> Option<Vec<usize>> {
        let query = query.trim();
        if query.is_empty() {
            return Some(Vec::new());
        }

        let lowered_text = text.to_ascii_lowercase();
        let lowered_query = query.to_ascii_lowercase();
        if !lowered_query.is_empty() {
            if let Some(byte_start) = lowered_text.find(&lowered_query) {
                let start_chars = lowered_text[..byte_start].chars().count();
                let len_chars = lowered_query.chars().count();
                return Some((start_chars..start_chars.saturating_add(len_chars)).collect());
            }
        }

        let query_chars: Vec<char> = query
            .chars()
            .filter(|ch| !ch.is_whitespace())
            .map(|ch| ch.to_ascii_lowercase())
            .collect();
        if query_chars.is_empty() {
            return Some(Vec::new());
        }

        let mut positions = Vec::new();
        let mut query_idx = 0usize;
        for (text_idx, ch) in text.chars().enumerate() {
            if query_idx >= query_chars.len() {
                break;
            }
            if ch.to_ascii_lowercase() == query_chars[query_idx] {
                positions.push(text_idx);
                query_idx += 1;
            }
        }

        if query_idx == query_chars.len() {
            Some(positions)
        } else {
            None
        }
    }

    fn markup_with_bold_matches(text: &str, positions: &[usize]) -> String {
        if positions.is_empty() {
            return glib::markup_escape_text(text).to_string();
        }

        let matched: BTreeSet<usize> = positions.iter().copied().collect();
        let mut out = String::new();
        let mut in_bold = false;
        for (idx, ch) in text.chars().enumerate() {
            let is_match = matched.contains(&idx);
            if is_match && !in_bold {
                out.push_str("<b>");
                in_bold = true;
            } else if !is_match && in_bold {
                out.push_str("</b>");
                in_bold = false;
            }
            out.push_str(&glib::markup_escape_text(&ch.to_string()));
        }
        if in_bold {
            out.push_str("</b>");
        }
        out
    }

    fn command_search_visible_rows(listbox: &gtk::ListBox) -> Vec<gtk::ListBoxRow> {
        let mut out = Vec::new();
        let mut index = 0;
        while let Some(row) = listbox.row_at_index(index) {
            if row.is_visible() {
                out.push(row);
            }
            index += 1;
        }
        out
    }

    fn first_visible_command_row(listbox: &gtk::ListBox) -> Option<gtk::ListBoxRow> {
        Self::command_search_visible_rows(listbox)
            .into_iter()
            .next()
    }

    fn selected_or_first_visible_command_row(listbox: &gtk::ListBox) -> Option<gtk::ListBoxRow> {
        listbox
            .selected_row()
            .filter(|row| row.is_visible())
            .or_else(|| Self::first_visible_command_row(listbox))
    }

    fn focus_selected_command_row(listbox: &gtk::ListBox) {
        if let Some(row) = listbox.selected_row().filter(|row| row.is_visible()) {
            row.grab_focus();
            return;
        }
        if let Some(row) = Self::first_visible_command_row(listbox) {
            listbox.select_row(Some(&row));
            row.grab_focus();
            return;
        }
        listbox.grab_focus();
    }

    fn apply_command_search_filter(
        listbox: &gtk::ListBox,
        items: &[CommandSearchItem],
        query: &str,
    ) {
        let query = query.trim();
        let mut first_visible: Option<gtk::ListBoxRow> = None;
        let mut selected_visible = false;
        let selected_index = listbox.selected_row().map(|row| row.index());

        for (idx, item) in items.iter().enumerate() {
            let Some(row) = listbox.row_at_index(idx as i32) else {
                continue;
            };
            let Some((main_label, action_label)) = Self::command_search_row_labels(&row) else {
                continue;
            };
            let prefix_text = Self::command_item_prefix_text(item);

            let (visible, main_match_positions, action_match_positions) = if query.is_empty() {
                (true, Vec::new(), Vec::new())
            } else {
                let main_match = Self::fuzzy_match_positions(&prefix_text, query);
                let action_match = Self::fuzzy_match_positions(&item.action, query);
                (
                    main_match.is_some() || action_match.is_some(),
                    main_match.unwrap_or_default(),
                    action_match.unwrap_or_default(),
                )
            };

            main_label.set_markup(&Self::markup_with_bold_matches(
                &format!("{prefix_text} "),
                &main_match_positions,
            ));
            action_label.set_markup(&Self::markup_with_bold_matches(
                &item.action,
                &action_match_positions,
            ));

            row.set_visible(visible);
            if visible {
                if first_visible.is_none() {
                    first_visible = Some(row.clone());
                }
                if selected_index == Some(idx as i32) {
                    selected_visible = true;
                }
            }
        }

        if !selected_visible {
            listbox.select_row(first_visible.as_ref());
        }
    }

    fn ensure_selected_command_row_visible(listbox: &gtk::ListBox, scroller: &gtk::ScrolledWindow) {
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

    fn select_next_visible_command_row(
        listbox: &gtk::ListBox,
        direction: i32,
        move_focus: bool,
        scroller: Option<&gtk::ScrolledWindow>,
    ) {
        let visible = Self::command_search_visible_rows(listbox);
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
            if move_focus {
                next.grab_focus();
            }
            if let Some(scroller) = scroller {
                Self::ensure_selected_command_row_visible(listbox, scroller);
            }
        }
    }

    fn select_command_row_home_end(
        listbox: &gtk::ListBox,
        to_end: bool,
        move_focus: bool,
        scroller: Option<&gtk::ScrolledWindow>,
    ) {
        let visible = Self::command_search_visible_rows(listbox);
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
            if move_focus {
                row.grab_focus();
            }
            if let Some(scroller) = scroller {
                Self::ensure_selected_command_row_visible(listbox, scroller);
            }
        }
    }

    fn page_step_for_command_list(listbox: &gtk::ListBox, visible: &[gtk::ListBoxRow]) -> usize {
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

    fn select_command_row_page(
        listbox: &gtk::ListBox,
        direction: i32,
        move_focus: bool,
        scroller: Option<&gtk::ScrolledWindow>,
    ) {
        let visible = Self::command_search_visible_rows(listbox);
        if visible.is_empty() {
            listbox.unselect_all();
            return;
        }

        let page = Self::page_step_for_command_list(listbox, &visible);
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
            if move_focus {
                row.grab_focus();
            }
            if let Some(scroller) = scroller {
                Self::ensure_selected_command_row_visible(listbox, scroller);
            }
        }
    }

    fn activate_command_search_item(&self, item: &CommandSearchItem) -> bool {
        if gtk::prelude::WidgetExt::activate_action(self, &item.action, None::<&glib::Variant>)
            .is_ok()
        {
            self.popdown_main_menu_later();
            return true;
        }

        if let Some((scope, name)) = item.action.split_once('.') {
            let activated = match scope {
                "win" => self.lookup_action(name).map(|action| {
                    action.activate(None);
                }),
                "app" => self
                    .application()
                    .and_then(|app| app.lookup_action(name))
                    .map(|action| {
                        action.activate(None);
                    }),
                _ => None,
            };
            if activated.is_some() {
                self.popdown_main_menu_later();
                return true;
            }
        }

        *self.imp().status_override.borrow_mut() = Some(format!(
            "Command unavailable: {} ({})",
            item.label, item.action
        ));
        self.render();
        false
    }

    fn close_palette_on_command_enabled(&self) -> bool {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return true;
        };
        let Some(schema) = settings.settings_schema() else {
            return true;
        };
        if !schema.has_key(SETTINGS_KEY_CLOSE_PALETTE_ON_COMMAND) {
            return true;
        }
        settings.boolean(SETTINGS_KEY_CLOSE_PALETTE_ON_COMMAND)
    }

    fn set_close_palette_on_command_enabled(&self, enabled: bool) {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_CLOSE_PALETTE_ON_COMMAND) {
            return;
        }
        let _ = settings.set_boolean(SETTINGS_KEY_CLOSE_PALETTE_ON_COMMAND, enabled);
    }

    fn command_palette_query_text(&self) -> String {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return String::new();
        };
        let Some(schema) = settings.settings_schema() else {
            return String::new();
        };
        if !schema.has_key(SETTINGS_KEY_COMMAND_PALETTE_QUERY) {
            return String::new();
        }
        settings
            .string(SETTINGS_KEY_COMMAND_PALETTE_QUERY)
            .to_string()
    }

    fn persist_command_palette_query_text(&self, query: &str) {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_COMMAND_PALETTE_QUERY) {
            return;
        }
        let _ = settings.set_string(SETTINGS_KEY_COMMAND_PALETTE_QUERY, query);
    }

    fn command_palette_window_size(&self) -> (i32, i32) {
        const DEFAULT_WIDTH: i32 = 400;
        const DEFAULT_HEIGHT: i32 = 600;
        const MIN_WIDTH: i32 = 360;
        const MIN_HEIGHT: i32 = 280;

        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        };
        let Some(schema) = settings.settings_schema() else {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        };
        if !schema.has_key(SETTINGS_KEY_COMMAND_PALETTE_WIDTH)
            || !schema.has_key(SETTINGS_KEY_COMMAND_PALETTE_HEIGHT)
        {
            return (DEFAULT_WIDTH, DEFAULT_HEIGHT);
        }

        (
            settings
                .int(SETTINGS_KEY_COMMAND_PALETTE_WIDTH)
                .max(MIN_WIDTH),
            settings
                .int(SETTINGS_KEY_COMMAND_PALETTE_HEIGHT)
                .max(MIN_HEIGHT),
        )
    }

    fn command_palette_maximized(&self) -> bool {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return false;
        };
        let Some(schema) = settings.settings_schema() else {
            return false;
        };
        if !schema.has_key(SETTINGS_KEY_COMMAND_PALETTE_MAXIMIZED) {
            return false;
        }
        settings.boolean(SETTINGS_KEY_COMMAND_PALETTE_MAXIMIZED)
    }

    fn persist_command_palette_maximized(&self, maximized: bool) {
        let settings = self.imp().settings.borrow().clone();
        let Some(settings) = settings.as_ref() else {
            return;
        };
        let Some(schema) = settings.settings_schema() else {
            return;
        };
        if !schema.has_key(SETTINGS_KEY_COMMAND_PALETTE_MAXIMIZED) {
            return;
        }
        if settings.boolean(SETTINGS_KEY_COMMAND_PALETTE_MAXIMIZED) != maximized {
            let _ = settings.set_boolean(SETTINGS_KEY_COMMAND_PALETTE_MAXIMIZED, maximized);
        }
    }

    fn persist_command_palette_window_size(&self, dialog: &gtk::Window) {
        const MIN_WIDTH: i32 = 360;
        const MIN_HEIGHT: i32 = 280;

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
        if !schema.has_key(SETTINGS_KEY_COMMAND_PALETTE_WIDTH)
            || !schema.has_key(SETTINGS_KEY_COMMAND_PALETTE_HEIGHT)
        {
            return;
        }

        let width = dialog.width().max(MIN_WIDTH);
        let height = dialog.height().max(MIN_HEIGHT);
        if settings.int(SETTINGS_KEY_COMMAND_PALETTE_WIDTH) != width {
            let _ = settings.set_int(SETTINGS_KEY_COMMAND_PALETTE_WIDTH, width);
        }
        if settings.int(SETTINGS_KEY_COMMAND_PALETTE_HEIGHT) != height {
            let _ = settings.set_int(SETTINGS_KEY_COMMAND_PALETTE_HEIGHT, height);
        }
    }

    fn activate_command_search_row(
        &self,
        dialog: &gtk::Window,
        items: &[CommandSearchItem],
        row: &gtk::ListBoxRow,
    ) {
        let row_index = row.index();
        if row_index < 0 {
            return;
        }
        let Some(item) = items.get(row_index as usize) else {
            return;
        };
        if self.activate_command_search_item(item) && self.close_palette_on_command_enabled() {
            dialog.close();
        }
    }

    pub(super) fn show_command_search_dialog(&self) {
        if let Some(existing) = self.imp().command_search_dialog.borrow().as_ref() {
            existing.present();
            if let Some(entry) = self.imp().command_search_filter_entry.borrow().as_ref() {
                entry.grab_focus();
                entry.select_region(0, -1);
            }
            return;
        }

        let items = Rc::new(self.command_search_items_from_main_menu());

        let (saved_width, saved_height) = self.command_palette_window_size();
        let saved_maximized = self.command_palette_maximized();
        let dialog = gtk::Window::builder()
            .title("Command Palette")
            .modal(false)
            .resizable(true)
            .default_width(saved_width)
            .default_height(saved_height)
            .build();
        // Keep this as a normal top-level window so maximize is available.
        dialog.set_transient_for(Option::<&gtk::Window>::None);
        dialog.set_hide_on_close(false);
        dialog.set_destroy_with_parent(false);
        self.connect_close_request(glib::clone!(
            #[weak]
            dialog,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_| {
                dialog.close();
                glib::Propagation::Proceed
            }
        ));
        dialog.connect_close_request(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |dialog| {
                let maximized = dialog.is_maximized();
                window.persist_command_palette_maximized(maximized);
                window.persist_command_palette_window_size(dialog);
                *window.imp().command_search_dialog.borrow_mut() = None;
                *window.imp().command_search_filter_entry.borrow_mut() = None;
                glib::Propagation::Proceed
            }
        ));
        if saved_maximized {
            dialog.maximize();
        }

        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.set_margin_top(12);
        root.set_margin_bottom(12);
        root.set_margin_start(12);
        root.set_margin_end(12);

        let heading = gtk::Label::new(Some("Command Palette"));
        heading.set_xalign(0.0);
        heading.add_css_class("title-4");
        root.append(&heading);

        let search_entry = gtk::SearchEntry::new();
        search_entry.set_hexpand(true);
        search_entry.set_placeholder_text(Some("Type to filter commands..."));
        let initial_query = self.command_palette_query_text();
        search_entry.set_text(&initial_query);
        root.append(&search_entry);

        let listbox = gtk::ListBox::new();
        listbox.add_css_class("boxed-list");
        listbox.set_selection_mode(gtk::SelectionMode::Single);
        listbox.set_activate_on_single_click(true);
        listbox.set_vexpand(true);

        let scroller = gtk::ScrolledWindow::new();
        scroller.set_policy(gtk::PolicyType::Never, gtk::PolicyType::Automatic);
        scroller.set_hexpand(true);
        scroller.set_vexpand(true);
        scroller.set_child(Some(&listbox));

        for item in items.iter() {
            let row = gtk::ListBoxRow::new();
            let row_box = gtk::Box::new(gtk::Orientation::Horizontal, 0);
            row_box.set_margin_top(8);
            row_box.set_margin_bottom(8);
            row_box.set_margin_start(10);
            row_box.set_margin_end(10);

            let prefix_text = Self::command_item_prefix_text(item);
            let main_label = gtk::Label::new(None);
            main_label.set_use_markup(true);
            main_label.set_markup(&glib::markup_escape_text(&format!("{prefix_text} ")));
            main_label.set_xalign(0.0);
            main_label.set_hexpand(true);
            main_label.set_single_line_mode(true);
            main_label.set_wrap(false);
            main_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            main_label.add_css_class("command-search-row");
            row_box.append(&main_label);

            let action_label = gtk::Label::new(None);
            action_label.set_use_markup(true);
            action_label.set_markup(&glib::markup_escape_text(&item.action));
            action_label.set_xalign(0.0);
            action_label.set_single_line_mode(true);
            action_label.set_wrap(false);
            action_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
            action_label.add_css_class("command-search-row");
            action_label.add_css_class("command-search-action-suffix");
            row_box.append(&action_label);

            row.set_child(Some(&row_box));
            listbox.append(&row);
        }

        listbox.connect_row_activated(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            #[strong]
            items,
            move |_, row| {
                window.activate_command_search_row(&dialog, items.as_ref(), row);
            }
        ));

        search_entry.connect_search_changed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            listbox,
            #[strong]
            items,
            move |entry| {
                let query = entry.text();
                window.persist_command_palette_query_text(query.as_str());
                Self::apply_command_search_filter(&listbox, items.as_ref(), query.as_str());
            }
        ));

        search_entry.connect_activate(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            #[weak]
            listbox,
            #[strong]
            items,
            move |_| {
                if let Some(row) = Self::selected_or_first_visible_command_row(&listbox) {
                    window.activate_command_search_row(&dialog, items.as_ref(), &row);
                }
            }
        ));

        let search_keys = gtk::EventControllerKey::new();
        search_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        search_keys.connect_key_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            dialog,
            #[weak]
            listbox,
            #[weak]
            scroller,
            #[strong]
            items,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| match key {
                gdk::Key::Down | gdk::Key::KP_Down => {
                    Self::select_next_visible_command_row(&listbox, 1, false, Some(&scroller));
                    glib::Propagation::Stop
                }
                gdk::Key::Up | gdk::Key::KP_Up => {
                    Self::select_next_visible_command_row(&listbox, -1, false, Some(&scroller));
                    glib::Propagation::Stop
                }
                gdk::Key::Home | gdk::Key::KP_Home => {
                    Self::select_command_row_home_end(&listbox, false, false, Some(&scroller));
                    glib::Propagation::Stop
                }
                gdk::Key::End | gdk::Key::KP_End => {
                    Self::select_command_row_home_end(&listbox, true, false, Some(&scroller));
                    glib::Propagation::Stop
                }
                gdk::Key::Page_Down | gdk::Key::KP_Page_Down => {
                    Self::select_command_row_page(&listbox, 1, false, Some(&scroller));
                    glib::Propagation::Stop
                }
                gdk::Key::Page_Up | gdk::Key::KP_Page_Up => {
                    Self::select_command_row_page(&listbox, -1, false, Some(&scroller));
                    glib::Propagation::Stop
                }
                gdk::Key::Return | gdk::Key::KP_Enter => {
                    if let Some(row) = Self::selected_or_first_visible_command_row(&listbox) {
                        window.activate_command_search_row(&dialog, items.as_ref(), &row);
                    }
                    glib::Propagation::Stop
                }
                gdk::Key::Escape => {
                    dialog.close();
                    glib::Propagation::Stop
                }
                _ => glib::Propagation::Proceed,
            }
        ));
        search_entry.add_controller(search_keys);

        let dialog_keys = gtk::EventControllerKey::new();
        dialog_keys.set_propagation_phase(gtk::PropagationPhase::Capture);
        dialog_keys.connect_key_pressed(glib::clone!(
            #[weak]
            dialog,
            #[weak]
            search_entry,
            #[weak]
            listbox,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, _| {
                if !matches!(key, gdk::Key::Tab | gdk::Key::ISO_Left_Tab) {
                    return glib::Propagation::Proceed;
                }

                let focus_in_search = gtk::prelude::GtkWindowExt::focus(&dialog)
                    .map(|focus: gtk::Widget| {
                        let search_widget: gtk::Widget = search_entry.clone().upcast();
                        focus == search_widget || focus.is_ancestor(&search_entry)
                    })
                    .unwrap_or(false);

                if focus_in_search {
                    Self::focus_selected_command_row(&listbox);
                } else {
                    search_entry.grab_focus();
                    search_entry.select_region(0, -1);
                }

                glib::Propagation::Stop
            }
        ));
        dialog.add_controller(dialog_keys);

        root.append(&scroller);

        let close_on_command_toggle = gtk::CheckButton::with_label("Close Palette On Command");
        close_on_command_toggle.set_active(self.close_palette_on_command_enabled());
        close_on_command_toggle.connect_toggled(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |toggle| {
                window.set_close_palette_on_command_enabled(toggle.is_active());
            }
        ));
        root.append(&close_on_command_toggle);

        let hint = gtk::Label::new(Some("Enter to run command. Esc to close."));
        hint.set_xalign(0.0);
        hint.add_css_class("dim-label");
        root.append(&hint);

        dialog.set_child(Some(&root));

        *self.imp().command_search_filter_entry.borrow_mut() = Some(search_entry.clone());
        *self.imp().command_search_dialog.borrow_mut() = Some(dialog.clone());

        Self::apply_command_search_filter(&listbox, items.as_ref(), initial_query.as_str());
        dialog.present();
        search_entry.grab_focus();
        search_entry.select_region(0, -1);
    }
}

use super::*;

impl CardthropicWindow {
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

        let reset_button = gtk::Button::with_label("Reset Default");
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

        palette_popover.set_child(Some(&palette_box));
        color_menu.set_popover(Some(&palette_popover));
    }
}

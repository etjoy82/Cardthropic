use super::super::SETTINGS_KEY_CHESS_BOARD_ROTATION_DEGREES;
use crate::game::ChessColor;
use crate::CardthropicWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::gio;
use gtk::glib;
use gtk::prelude::*;

const CHESS_ROTATION_OPTIONS: [i32; 4] = [0, 45, 180, 270];

impl CardthropicWindow {
    fn normalize_chess_board_rotation_degrees(degrees: i32) -> i32 {
        let normalized = degrees.rem_euclid(360);
        if CHESS_ROTATION_OPTIONS.contains(&normalized) {
            normalized
        } else {
            0
        }
    }

    pub(in crate::window) fn settings_has_chess_rotation_key(settings: &gio::Settings) -> bool {
        settings
            .settings_schema()
            .map(|schema| schema.has_key(SETTINGS_KEY_CHESS_BOARD_ROTATION_DEGREES))
            .unwrap_or(false)
    }

    pub(in crate::window) fn chess_board_rotation_degrees(&self) -> i32 {
        Self::normalize_chess_board_rotation_degrees(self.imp().chess_board_rotation_degrees.get())
    }

    pub(in crate::window) fn set_chess_board_rotation_degrees(
        &self,
        degrees: i32,
        persist: bool,
        announce: bool,
    ) {
        let imp = self.imp();
        let normalized = Self::normalize_chess_board_rotation_degrees(degrees);
        let changed = imp.chess_board_rotation_degrees.get() != normalized;
        imp.chess_board_rotation_degrees.set(normalized);

        if persist {
            if let Some(settings) = imp.settings.borrow().as_ref() {
                if Self::settings_has_chess_rotation_key(settings)
                    && settings.int(SETTINGS_KEY_CHESS_BOARD_ROTATION_DEGREES) != normalized
                {
                    let _ = settings.set_int(SETTINGS_KEY_CHESS_BOARD_ROTATION_DEGREES, normalized);
                }
            }
        }

        if let Some(action) = self.lookup_action("chess-flip-board") {
            if let Ok(action) = action.downcast::<gio::SimpleAction>() {
                let flipped = normalized == 180;
                let current = action
                    .state()
                    .and_then(|variant| variant.get::<bool>())
                    .unwrap_or(false);
                if current != flipped {
                    action.set_state(&flipped.to_variant());
                }
            }
        }

        if announce {
            *imp.status_override.borrow_mut() =
                Some(format!("Chess board rotation set to {normalized}\u{00b0}."));
        }

        if changed || announce {
            self.render();
        }
    }

    pub(in crate::window) fn chess_board_flipped(&self) -> bool {
        self.chess_board_rotation_degrees() == 180
    }

    pub(in crate::window) fn set_chess_board_flipped(
        &self,
        flipped: bool,
        persist: bool,
        announce: bool,
    ) {
        let target_degrees = if flipped { 180 } else { 0 };
        self.set_chess_board_rotation_degrees(target_degrees, persist, false);
        if announce {
            *self.imp().status_override.borrow_mut() = Some(if flipped {
                "Chess board flipped (Black at bottom).".to_string()
            } else {
                "Chess board unflipped (White at bottom).".to_string()
            });
            self.render();
        }
    }

    pub(in crate::window) fn maybe_auto_flip_chess_board_to_side_to_move(
        &self,
        persist_rotation: bool,
    ) -> bool {
        if !self.chess_auto_flip_board_each_move_enabled() {
            return false;
        }
        if self.imp().robot_mode_running.get() && self.imp().robot_ludicrous_enabled.get() {
            return false;
        }
        if !self.imp().chess_mode_active.get() {
            return false;
        }

        let side_to_move = self.imp().chess_position.borrow().side_to_move();
        let should_flip = matches!(side_to_move, ChessColor::Black);
        if self.chess_board_flipped() == should_flip {
            return false;
        }

        self.set_chess_board_flipped(should_flip, persist_rotation, false);
        true
    }

    pub(in crate::window) fn show_chess_board_rotation_dialog(&self) {
        self.popdown_main_menu_later();

        let dialog = gtk::Window::builder()
            .title("Rotate Board")
            .modal(true)
            .transient_for(self)
            .default_width(360)
            .default_height(220)
            .build();
        dialog.set_resizable(false);
        dialog.set_destroy_with_parent(true);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 10);
        root.set_margin_top(14);
        root.set_margin_bottom(14);
        root.set_margin_start(14);
        root.set_margin_end(14);

        let heading = gtk::Label::new(Some("Rotate Chess Board"));
        heading.set_xalign(0.0);
        heading.add_css_class("title-4");
        root.append(&heading);

        let body = gtk::Label::new(Some(
            "Pick a visual board angle. Applies to Standard Chess, Chess960, and Atomic Chess.",
        ));
        body.set_xalign(0.0);
        body.set_wrap(true);
        body.set_wrap_mode(gtk::pango::WrapMode::WordChar);
        root.append(&body);

        let grid = gtk::Grid::new();
        grid.set_column_spacing(8);
        grid.set_row_spacing(8);
        grid.set_halign(gtk::Align::Start);

        let current = self.chess_board_rotation_degrees();
        let mut group_anchor: Option<gtk::CheckButton> = None;
        for (idx, degrees) in CHESS_ROTATION_OPTIONS.iter().copied().enumerate() {
            let button = gtk::CheckButton::with_label(&format!("{degrees}\u{00b0}"));
            if let Some(anchor) = group_anchor.as_ref() {
                button.set_group(Some(anchor));
            } else {
                group_anchor = Some(button.clone());
            }
            button.set_active(degrees == current);
            button.connect_toggled(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |btn| {
                    if btn.is_active() {
                        window.set_chess_board_rotation_degrees(degrees, true, true);
                    }
                }
            ));
            grid.attach(&button, (idx % 4) as i32, (idx / 4) as i32, 1, 1);
        }
        root.append(&grid);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 8);
        actions.set_halign(gtk::Align::End);

        let reset = gtk::Button::with_label("Reset to 0\u{00b0}");
        reset.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.set_chess_board_rotation_degrees(0, true, true);
            }
        ));
        actions.append(&reset);

        let close = gtk::Button::with_label("Close");
        close.add_css_class("suggested-action");
        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        actions.append(&close);
        root.append(&actions);

        dialog.set_default_widget(Some(&close));
        let _ = close.grab_focus();
        dialog.set_child(Some(&root));
        dialog.present();
    }
}

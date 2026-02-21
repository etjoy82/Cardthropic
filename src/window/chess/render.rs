use crate::game::{
    file_of, is_in_check, legal_moves, rank_of, square, square_name, ChessColor, ChessPiece,
    ChessPieceKind, Square,
};
use crate::CardthropicWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::prelude::*;
use std::collections::HashSet;

const CHESS_BOARD_SIZE: i32 = 8;
const CHESS_MIN_SQUARE_SIZE: i32 = 20;
const CHESS_MAX_SQUARE_SIZE: i32 = 112;
const CHESS_OFF_AXIS_FIT_MARGIN_PX: i32 = 6;

impl CardthropicWindow {
    pub(in crate::window) fn apply_chess_drag_hover_row_classes(&self) {
        let hover_row = self.imp().chess_drag_hover_row_from_top.get();
        for stack in self.tableau_stacks().iter().take(CHESS_BOARD_SIZE as usize) {
            let mut row_from_top = 0;
            let mut child = stack.first_child();
            while let Some(widget) = child {
                if hover_row == Some(row_from_top) {
                    widget.add_css_class("chess-square-drop-row");
                } else {
                    widget.remove_css_class("chess-square-drop-row");
                }
                row_from_top += 1;
                child = widget.next_sibling();
            }
        }
    }

    pub(in crate::window) fn render_chess_board(&self) {
        self.configure_chess_playfield_chrome();
        self.update_tableau_metrics();

        let imp = self.imp();
        let square_size = self.chess_square_size_for_layout();
        imp.chess_square_size.set(square_size);
        self.ensure_chess_css_provider(square_size);

        let position = imp.chess_position.borrow().clone();
        let white_in_check = is_in_check(&position, ChessColor::White);
        let black_in_check = is_in_check(&position, ChessColor::Black);
        let selected = imp.chess_selected_square.get();
        let keyboard_square = self.chess_keyboard_square();
        let last_move_from = imp.chess_last_move_from.get();
        let last_move_to = imp.chess_last_move_to.get();
        let target_squares = selected
            .map(|from| {
                legal_moves(&position)
                    .into_iter()
                    .filter(|mv| mv.from == from)
                    .map(|mv| mv.to)
                    .collect::<HashSet<Square>>()
            })
            .unwrap_or_default();

        self.clear_tableau_render_state_for_chess();
        self.render_chess_grid(
            &position,
            square_size,
            selected,
            last_move_from,
            last_move_to,
            keyboard_square,
            &target_squares,
            white_in_check,
            black_in_check,
        );
        self.apply_chess_board_rotation_transform(square_size);
        self.set_chess_controls_enabled();
        self.update_stats_label();

        let side_to_move = position.side_to_move();
        let side_to_move_in_check = match side_to_move {
            ChessColor::White => white_in_check,
            ChessColor::Black => black_in_check,
        };

        let status = if let Some(message) = imp.status_override.borrow().as_deref() {
            message.to_string()
        } else if let Some(selected) = selected {
            let moves = target_squares.len();
            format!(
                "Selected {}. {} legal destinations.",
                square_name(selected),
                moves
            )
        } else if side_to_move_in_check {
            format!(
                "{} to move and in check. Respond immediately.",
                color_label(side_to_move)
            )
        } else {
            format!(
                "{} to move. Click or use arrows/WASD/HJKL, Enter/Space to act, Esc to clear.",
                color_label(side_to_move)
            )
        };
        self.append_status_line(&status);
        self.apply_mobile_phone_mode_overrides();
        self.mark_session_dirty();
    }

    fn apply_chess_board_rotation_transform(&self, square_size: i32) {
        let imp = self.imp();
        let spacing = imp.tableau_row.spacing().max(0);
        let angle = self.chess_board_rotation_degrees();
        let (board_width, board_height) = chess_unrotated_board_dimensions(square_size, spacing);
        let (rotated_width, rotated_height) =
            chess_rotated_board_dimensions(square_size, spacing, angle);
        let canvas_width = rotated_width.max(board_width).max(1);
        let canvas_height = rotated_height.max(board_height).max(1);

        imp.tableau_canvas
            .set_size_request(canvas_width, canvas_height);
        let offset_x = f64::from(canvas_width.saturating_sub(board_width)) / 2.0;
        let offset_y = f64::from(canvas_height.saturating_sub(board_height)) / 2.0;
        imp.tableau_canvas
            .move_(&imp.tableau_row.get(), offset_x, offset_y);

        // Flip-board mode (180°) is handled by display-square remapping so piece
        // glyphs remain upright while positions are mirrored for the viewer.
        if angle == 0 || angle == 180 {
            imp.tableau_canvas
                .set_child_transform(&imp.tableau_row.get(), None);
            return;
        }

        let cx = board_width as f32 / 2.0;
        let cy = board_height as f32 / 2.0;
        let transform = gtk::gsk::Transform::new()
            .translate(&gtk::graphene::Point::new(cx, cy))
            .rotate(angle as f32)
            .translate(&gtk::graphene::Point::new(-cx, -cy));
        imp.tableau_canvas
            .set_child_transform(&imp.tableau_row.get(), Some(&transform));
    }

    pub(in crate::window) fn clear_chess_board_rotation_transform(&self) {
        let imp = self.imp();
        imp.tableau_canvas
            .set_child_transform(&imp.tableau_row.get(), None);
        imp.tableau_canvas.move_(&imp.tableau_row.get(), 0.0, 0.0);
        imp.tableau_canvas.set_size_request(-1, -1);
        imp.tableau_canvas.set_halign(gtk::Align::Start);
        imp.tableau_canvas.set_valign(gtk::Align::Start);
    }

    fn configure_chess_playfield_chrome(&self) {
        let imp = self.imp();
        imp.tableau_canvas.set_halign(gtk::Align::Center);
        imp.tableau_canvas.set_valign(gtk::Align::Center);
        imp.tableau_row.set_direction(gtk::TextDirection::Ltr);
        for (idx, stack) in self.tableau_stacks().iter().enumerate() {
            if idx < CHESS_BOARD_SIZE as usize {
                stack.set_direction(gtk::TextDirection::Ltr);
            }
            // Chess uses per-square focus styling; clear any leftover
            // column-level keyboard focus class from solitaire modes.
            stack.remove_css_class("keyboard-focus-empty");
            // Keep chess drop styling isolated from generic solitaire drop-target
            // CSS so drag-active borders cannot resize chess columns.
            stack.remove_css_class("tableau-drop-target");
            stack.add_css_class("chess-tableau-drop-target");
        }
        imp.stock_picture.set_visible(false);
        imp.stock_column_box.set_visible(false);
        imp.stock_label.set_visible(false);
        imp.stock_heading_box.set_visible(false);
        imp.waste_overlay.set_visible(false);
        imp.waste_column_box.set_visible(false);
        imp.waste_label.set_visible(false);
        imp.waste_heading_box.set_visible(false);
        imp.foundations_heading_box.set_visible(false);
        imp.foundations_area_box.set_visible(false);
        imp.top_playfield_frame.set_visible(false);
        imp.top_playfield_frame.set_height_request(-1);
        imp.top_heading_row_box.set_visible(false);
        imp.stock_waste_foundations_row_box.set_visible(false);
        imp.top_row_spacer_box.set_visible(false);
        imp.stock_waste_foundation_spacer_box.set_visible(false);
        let hidden_label = gtk::Label::new(None);
        hidden_label.set_visible(false);
        imp.tableau_frame.set_label_widget(Some(&hidden_label));
        imp.tableau_frame.add_css_class("chess-frame-no-label");
        *imp.selected_run.borrow_mut() = None;
        imp.selected_freecell.set(None);
        imp.waste_selected.set(false);
    }

    fn chess_square_size_for_layout(&self) -> i32 {
        let imp = self.imp();
        let spacing = imp.tableau_row.spacing().max(0);
        let viewport_width = imp.tableau_scroller.hadjustment().page_size().floor() as i32;
        let viewport_height = imp.tableau_scroller.vadjustment().page_size().floor() as i32;
        let width_budget = {
            let scroller_width = imp.tableau_scroller.width();
            let width = if viewport_width > 0 {
                viewport_width
            } else if scroller_width > 0 {
                scroller_width
            } else if imp.observed_scroller_width.get() > 0 {
                imp.observed_scroller_width.get()
            } else {
                self.width().saturating_sub(24)
            };
            if width > 0 {
                imp.observed_scroller_width.set(width);
            }
            width
        };
        let height_budget = {
            let scroller_height = imp.tableau_scroller.height();
            let height = if viewport_height > 0 {
                viewport_height
            } else if scroller_height > 0 {
                scroller_height
            } else if imp.observed_scroller_height.get() > 0 {
                imp.observed_scroller_height.get()
            } else {
                let window_height = self.height();
                let reserve = if !imp.hud_enabled.get() {
                    56
                } else if window_height <= 600 {
                    148
                } else if window_height <= 720 {
                    164
                } else if window_height <= 1080 {
                    184
                } else {
                    208
                };
                window_height.saturating_sub(reserve).saturating_sub(24)
            };
            if height > 0 {
                imp.observed_scroller_height.set(height);
            }
            height
        };

        let angle = self.chess_board_rotation_degrees();
        let fit_margin = if angle.rem_euclid(90) == 0 {
            0
        } else {
            CHESS_OFF_AXIS_FIT_MARGIN_PX
        };
        let fit_width_budget = width_budget.saturating_sub(fit_margin);
        let fit_height_budget = height_budget.saturating_sub(fit_margin);
        let mut square_size =
            chess_max_square_size_for_budget(fit_width_budget, fit_height_budget, spacing, angle)
                .clamp(CHESS_MIN_SQUARE_SIZE, CHESS_MAX_SQUARE_SIZE);
        let fallback = imp.card_width.get().max(CHESS_MIN_SQUARE_SIZE);
        if square_size <= 0 {
            square_size = fallback.clamp(CHESS_MIN_SQUARE_SIZE, CHESS_MAX_SQUARE_SIZE);
        }
        square_size
    }

    pub(in crate::window) fn chess_square_from_display_cell(
        &self,
        file_index: usize,
        row_from_top: i32,
    ) -> Option<Square> {
        if file_index >= CHESS_BOARD_SIZE as usize || !(0..CHESS_BOARD_SIZE).contains(&row_from_top)
        {
            return None;
        }

        let display_file = file_index as u8;
        let display_rank_from_top = row_from_top as u8;
        let (file, rank) = if self.chess_board_flipped() {
            (7 - display_file, display_rank_from_top)
        } else {
            (display_file, 7 - display_rank_from_top)
        };
        square(file, rank)
    }

    pub(in crate::window) fn chess_display_cell_for_square(
        &self,
        sq: Square,
    ) -> Option<(usize, i32)> {
        if sq >= 64 {
            return None;
        }
        let file = file_of(sq);
        let rank = rank_of(sq);
        if self.chess_board_flipped() {
            Some((usize::from(7 - file), i32::from(rank)))
        } else {
            Some((usize::from(file), CHESS_BOARD_SIZE - 1 - i32::from(rank)))
        }
    }

    pub(in crate::window) fn chess_widget_for_square(&self, square: Square) -> Option<gtk::Widget> {
        let (file_idx, row_from_top) = self.chess_display_cell_for_square(square)?;

        let stack = self.tableau_stacks().get(file_idx)?.clone();
        let mut child = stack.first_child()?;
        for _ in 0..row_from_top {
            child = child.next_sibling()?;
        }
        Some(child)
    }

    pub(in crate::window) fn clear_tableau_render_state_for_chess(&self) {
        let imp = self.imp();
        let stacks = self.tableau_stacks();
        let mut pictures = imp.tableau_card_pictures.borrow_mut();
        let mut states = imp.tableau_picture_state_cache.borrow_mut();
        for (idx, stack) in stacks.iter().enumerate() {
            while let Some(child) = stack.first_child() {
                stack.remove(&child);
            }
            if let Some(col) = pictures.get_mut(idx) {
                col.clear();
            }
            if let Some(col) = states.get_mut(idx) {
                col.clear();
            }
        }
    }

    fn render_chess_grid(
        &self,
        position: &crate::game::ChessPosition,
        square_size: i32,
        selected: Option<Square>,
        last_move_from: Option<Square>,
        last_move_to: Option<Square>,
        keyboard_square: Option<Square>,
        target_squares: &HashSet<Square>,
        white_in_check: bool,
        black_in_check: bool,
    ) {
        let stacks = self.tableau_stacks();
        let drag_hover_row = self.imp().chess_drag_hover_row_from_top.get();
        let show_edge_markers = self.chess_show_board_coordinates_enabled();
        for (file_idx, stack) in stacks.iter().enumerate() {
            if file_idx >= CHESS_BOARD_SIZE as usize {
                stack.set_visible(false);
                continue;
            }

            stack.set_visible(true);
            stack.set_width_request(square_size);
            stack.set_height_request(square_size * CHESS_BOARD_SIZE);

            for row_from_top in 0..CHESS_BOARD_SIZE {
                let sq = self
                    .chess_square_from_display_cell(file_idx, row_from_top)
                    .expect("valid display chess square");
                let file = file_of(sq);
                let rank = rank_of(sq);
                let square_widget = gtk::Overlay::new();
                square_widget.set_width_request(square_size);
                square_widget.set_height_request(square_size);
                square_widget.set_halign(gtk::Align::Fill);
                square_widget.set_valign(gtk::Align::Fill);
                square_widget.add_css_class("chess-square");
                if (file + rank) % 2 == 0 {
                    square_widget.add_css_class("chess-square-dark");
                } else {
                    square_widget.add_css_class("chess-square-light");
                }
                if last_move_from == Some(sq) {
                    square_widget.add_css_class("chess-square-last-origin");
                }
                if last_move_to == Some(sq) {
                    square_widget.add_css_class("chess-square-last-destination");
                }
                if selected == Some(sq) {
                    square_widget.add_css_class("chess-square-selected");
                } else if target_squares.contains(&sq) {
                    square_widget.add_css_class("chess-square-target");
                }
                if keyboard_square == Some(sq) {
                    square_widget.add_css_class("chess-square-keyboard-focus");
                }
                if drag_hover_row == Some(row_from_top) {
                    square_widget.add_css_class("chess-square-drop-row");
                }

                let piece_label = gtk::Label::new(None);
                piece_label.set_xalign(0.5);
                piece_label.set_yalign(0.5);
                piece_label.set_halign(gtk::Align::Center);
                piece_label.set_valign(gtk::Align::Center);
                piece_label.set_can_target(false);
                square_widget.set_child(Some(&piece_label));

                if show_edge_markers {
                    if row_from_top == 0 {
                        let top_marker = chess_edge_marker_label(
                            chess_file_marker(file).to_string(),
                            gtk::Align::Center,
                            gtk::Align::Start,
                            "chess-edge-marker-top",
                        );
                        square_widget.add_overlay(&top_marker);
                    }
                    if row_from_top == CHESS_BOARD_SIZE - 1 {
                        let bottom_marker = chess_edge_marker_label(
                            chess_file_marker(file).to_string(),
                            gtk::Align::Center,
                            gtk::Align::End,
                            "chess-edge-marker-bottom",
                        );
                        square_widget.add_overlay(&bottom_marker);
                    }
                    if file_idx == 0 {
                        let left_marker = chess_edge_marker_label(
                            chess_rank_marker(rank).to_string(),
                            gtk::Align::Start,
                            gtk::Align::Center,
                            "chess-edge-marker-left",
                        );
                        square_widget.add_overlay(&left_marker);
                    }
                    if file_idx == (CHESS_BOARD_SIZE as usize - 1) {
                        let right_marker = chess_edge_marker_label(
                            chess_rank_marker(rank).to_string(),
                            gtk::Align::End,
                            gtk::Align::Center,
                            "chess-edge-marker-right",
                        );
                        square_widget.add_overlay(&right_marker);
                    }
                }

                match position.piece_at(sq) {
                    Some(piece) => {
                        piece_label.set_label(piece_glyph(piece));
                        match piece.color {
                            ChessColor::White => piece_label.add_css_class("chess-piece-white"),
                            ChessColor::Black => piece_label.add_css_class("chess-piece-black"),
                        }
                        if piece.kind == ChessPieceKind::King
                            && ((piece.color == ChessColor::White && white_in_check)
                                || (piece.color == ChessColor::Black && black_in_check))
                        {
                            square_widget.add_css_class("chess-square-in-check");
                        }
                        square_widget.set_tooltip_text(Some(&format!(
                            "{} {}",
                            square_name(sq),
                            piece_name(piece)
                        )));
                    }
                    None => {
                        piece_label.set_label("");
                        square_widget.set_tooltip_text(Some(&square_name(sq)));
                    }
                }

                stack.put(&square_widget, 0.0, f64::from(row_from_top * square_size));
            }
        }
    }

    fn set_chess_controls_enabled(&self) {
        let imp = self.imp();
        let chess_undo_available = !imp.chess_history.borrow().is_empty();
        let chess_redo_available = !imp.chess_future.borrow().is_empty();
        let global_undo_available = !imp.history.borrow().is_empty();
        let global_redo_available = !imp.future.borrow().is_empty();
        imp.undo_button
            .set_sensitive(chess_undo_available || global_undo_available);
        imp.redo_button
            .set_sensitive(chess_redo_available || global_redo_available);
        imp.auto_hint_button.set_sensitive(true);
        imp.cyclone_shuffle_button.set_sensitive(false);
        imp.peek_button.set_sensitive(false);
        imp.robot_button.set_sensitive(true);
        imp.seed_random_button.set_sensitive(true);
        imp.seed_rescue_button.set_sensitive(false);
        imp.seed_winnable_button.set_sensitive(true);
        imp.seed_repeat_button.set_sensitive(true);
        imp.seed_go_button.set_sensitive(true);
        imp.seed_combo.set_sensitive(true);
    }

    fn ensure_chess_css_provider(&self, square_size: i32) {
        let imp = self.imp();
        let existing_provider = { imp.chess_css_provider.borrow().as_ref().cloned() };
        let provider = if let Some(provider) = existing_provider {
            provider
        } else {
            let provider = gtk::CssProvider::new();
            if let Some(display) = gtk::gdk::Display::default() {
                gtk::style_context_add_provider_for_display(
                    &display,
                    &provider,
                    gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
                );
            }
            *imp.chess_css_provider.borrow_mut() = Some(provider.clone());
            provider
        };

        let piece_font_px = ((square_size * 3) / 4).clamp(16, 96);
        let edge_marker_font_px = ((square_size * 17) / 100).clamp(7, 13);
        // Scope dynamic chess sizing to this specific window so multiple
        // chess windows do not overwrite each other's piece scale.
        let scope_class = format!("cardthropic-chess-scope-{:x}", self.as_ptr() as usize);
        self.add_css_class(&scope_class);
        let mut css = String::from(
            r#"
.chess-square {
  border-radius: 8px;
  border: 1px solid rgba(100, 200, 255, 0.3);
  font-weight: 700;
}

.chess-edge-marker {
  font-weight: 600;
  color: rgba(218, 234, 255, 0.56);
  text-shadow: 0 0 2px rgba(10, 18, 38, 0.85);
}

.chess-edge-marker-top {
  margin-top: 1px;
}

.chess-edge-marker-bottom {
  margin-bottom: 1px;
}

.chess-edge-marker-left {
  margin-left: 1px;
}

.chess-edge-marker-right {
  margin-right: 1px;
}

/* Light squares - Cyan hologram */
.chess-square-light {
  background:
    radial-gradient(circle at 30% 30%, rgba(100, 220, 255, 0.15), transparent 60%),
    linear-gradient(135deg,
      rgba(80, 180, 240, 0.25) 0%,
      rgba(60, 160, 220, 0.20) 50%,
      rgba(80, 180, 240, 0.25) 100%);
  background-color: rgba(40, 100, 160, 0.4);
  box-shadow:
    inset 0 1px 0 rgba(150, 220, 255, 0.15),
    inset 0 -1px 0 rgba(0, 0, 0, 0.2);
}

/* Dark squares - Deep void with purple undertones */
.chess-square-dark {
  background:
    radial-gradient(circle at 70% 70%, rgba(140, 100, 200, 0.12), transparent 60%),
    linear-gradient(135deg,
      rgba(20, 15, 40, 0.95) 0%,
      rgba(15, 10, 35, 0.98) 50%,
      rgba(20, 15, 40, 0.95) 100%);
  background-color: rgba(12, 10, 25, 0.95);
  box-shadow:
    inset 0 1px 0 rgba(100, 80, 180, 0.08),
    inset 0 -1px 0 rgba(0, 0, 0, 0.4);
}

/* Hover effect - Quantum shimmer */
.chess-square:hover {
  border-color: rgba(120, 220, 255, 0.6);
  box-shadow:
    inset 0 0 0 1px rgba(120, 220, 255, 0.3),
    0 0 20px rgba(100, 200, 255, 0.15);
}

/* Last move origin - Gold signal ring.
   Keep selector specificity above generic .chess-square theme overrides. */
.chess-square.chess-square-last-origin {
  border-color: rgba(255, 227, 156, 0.96);
  background-image:
    radial-gradient(circle at 50% 50%,
      rgba(255, 226, 160, 0.24) 0%,
      rgba(255, 190, 82, 0.11) 56%,
      transparent 78%);
  box-shadow:
    inset 0 0 0 2px rgba(255, 241, 198, 0.98),
    inset 0 0 0 5px rgba(255, 186, 72, 0.84),
    0 0 0 1px rgba(255, 220, 144, 0.72),
    0 0 28px rgba(255, 177, 58, 0.48);
}

/* Last move destination - bright aqua arrival marker. */
.chess-square.chess-square-last-destination {
  border-color: rgba(190, 246, 255, 0.98);
  background-image:
    radial-gradient(circle at 50% 50%,
      rgba(168, 243, 255, 0.34) 0%,
      rgba(86, 218, 255, 0.16) 58%,
      transparent 80%);
  box-shadow:
    inset 0 0 0 2px rgba(237, 252, 255, 0.98),
    inset 0 0 0 5px rgba(102, 224, 255, 0.9),
    0 0 0 1px rgba(208, 247, 255, 0.82),
    0 0 34px rgba(84, 214, 255, 0.62);
}

/* Selected piece - Cyan energy field */
.chess-square-selected {
  box-shadow:
    inset 0 0 0 3px rgba(100, 220, 255, 0.9),
    inset 0 0 24px rgba(100, 220, 255, 0.3),
    0 0 30px rgba(100, 220, 255, 0.4),
    0 0 50px rgba(80, 200, 255, 0.2);
  background-color: rgba(100, 220, 255, 0.08);
}

/* Valid move targets - Purple possibility markers */
.chess-square-target {
  box-shadow:
    inset 0 0 0 3px rgba(180, 120, 255, 0.7),
    inset 0 0 20px rgba(160, 100, 255, 0.25),
    0 0 25px rgba(180, 120, 255, 0.3);
  background-image:
    radial-gradient(circle at 50% 50%,
      rgba(180, 120, 255, 0.12),
      transparent 60%);
}

.chess-square-drop-row {
  box-shadow:
    inset 0 0 0 1px rgba(100, 220, 255, 0.52),
    inset 0 0 0 3px rgba(100, 220, 255, 0.16);
  background-image:
    linear-gradient(180deg,
      rgba(100, 220, 255, 0.10),
      rgba(100, 220, 255, 0.03));
}

.chess-square-keyboard-focus {
  box-shadow:
    inset 0 0 0 2px rgba(255, 255, 255, 0.92),
    inset 0 0 0 5px rgba(30, 18, 50, 0.70);
}

/* When keyboard focus lands on the last destination, preserve destination
   prominence instead of letting keyboard-focus override it. */
.chess-square.chess-square-last-destination.chess-square-keyboard-focus {
  border-color: rgba(198, 248, 255, 1.0);
  box-shadow:
    inset 0 0 0 2px rgba(244, 253, 255, 1.0),
    inset 0 0 0 5px rgba(109, 227, 255, 0.94),
    inset 0 0 0 8px rgba(28, 18, 50, 0.45),
    0 0 0 1px rgba(214, 249, 255, 0.9),
    0 0 38px rgba(86, 217, 255, 0.72);
}

/* Check warning - Magenta emergency protocol */
.chess-square-in-check {
  box-shadow:
    inset 0 0 0 3px rgba(255, 100, 180, 1.0),
    inset 0 0 0 7px rgba(20, 10, 30, 0.8),
    inset 0 0 40px rgba(255, 100, 180, 0.4),
    0 0 50px rgba(255, 100, 180, 0.6),
    0 0 80px rgba(240, 80, 160, 0.3);
  background-color: rgba(255, 100, 180, 0.15);
}

/* White pieces - Crystalline cyan energy */
.chess-piece-white {
  color: #e8f8ff;
  text-shadow:
    0 0 8px rgba(100, 220, 255, 0.8),
    0 0 16px rgba(100, 220, 255, 0.5),
    0 0 24px rgba(80, 200, 255, 0.3),
    0 2px 4px rgba(0, 0, 0, 0.6);
}

/* Black pieces - Deep void with purple corona */
.chess-piece-black {
  color: #1a1a2e;
  font-weight: 900;
  text-shadow:
    0 0 2px rgba(140, 100, 220, 0.9),
    0 0 6px rgba(140, 100, 220, 0.6),
    0 0 12px rgba(160, 120, 240, 0.4),
    0 0 20px rgba(180, 140, 255, 0.2);
}

/* Drag icon - Phase-shifted appearance */
.chess-drag-icon {
  background: transparent;
  border: none;
  box-shadow: none;
  padding: 0;
  margin: 0;
  font-size: 56px;
  font-weight: 700;
  opacity: 0.9;
}

/* Hidden labels - Quantum uncertainty */
.chess-frame-no-label > border > label,
.chess-frame-no-label > label {
  opacity: 0;
  min-height: 0;
  padding: 0;
  margin: 0;
}

/* Optional: Piece hover glow */
.chess-square:hover .chess-piece-white {
  text-shadow:
    0 0 12px rgba(120, 240, 255, 1.0),
    0 0 24px rgba(120, 240, 255, 0.7),
    0 0 36px rgba(100, 220, 255, 0.4),
    0 2px 4px rgba(0, 0, 0, 0.6);
}

.chess-square:hover .chess-piece-black {
  text-shadow:
    0 0 4px rgba(160, 120, 240, 1.0),
    0 0 10px rgba(160, 120, 240, 0.8),
    0 0 18px rgba(180, 140, 255, 0.5),
    0 0 28px rgba(200, 160, 255, 0.3);
}

/* Chess drag/drop column target: avoid border-based layout shifts. */
.chess-frame-no-label .chess-tableau-drop-target:drop(active) {
  border: none;
  outline: none;
  padding: 0;
  margin: 0;
  box-shadow: inset 0 0 0 2px rgba(100, 220, 255, 0.75);
  background-color: rgba(100, 220, 255, 0.08);
}
            "#,
        );
        css.push_str(&format!(
            "
.{scope_class} .chess-square {{ font-size: {piece_font_px}px; }}
.{scope_class} .chess-drag-icon {{ font-size: {piece_font_px}px; }}
.{scope_class} .chess-edge-marker {{ font-size: {edge_marker_font_px}px; }}
"
        ));
        provider.load_from_string(&css);
    }
}

fn chess_file_marker(file: u8) -> char {
    char::from(b'A' + file.min(7))
}

fn chess_rank_marker(rank: u8) -> char {
    char::from(b'1' + rank.min(7))
}

fn chess_edge_marker_label(
    text: String,
    halign: gtk::Align,
    valign: gtk::Align,
    edge_class: &str,
) -> gtk::Label {
    let marker = gtk::Label::new(Some(&text));
    marker.set_halign(halign);
    marker.set_valign(valign);
    marker.set_xalign(0.5);
    marker.set_yalign(0.5);
    marker.set_can_target(false);
    marker.add_css_class("chess-edge-marker");
    marker.add_css_class(edge_class);
    marker
}

fn chess_unrotated_board_dimensions(square_size: i32, spacing: i32) -> (i32, i32) {
    let board_width = CHESS_BOARD_SIZE * square_size + (CHESS_BOARD_SIZE - 1) * spacing.max(0);
    let board_height = CHESS_BOARD_SIZE * square_size;
    (board_width.max(1), board_height.max(1))
}

fn chess_rotated_board_dimensions(
    square_size: i32,
    spacing: i32,
    angle_degrees: i32,
) -> (i32, i32) {
    let (board_width, board_height) = chess_unrotated_board_dimensions(square_size, spacing);
    let radians = (angle_degrees as f64).to_radians();
    let cos = radians.cos().abs();
    let sin = radians.sin().abs();
    let rotated_width =
        ((f64::from(board_width) * cos) + (f64::from(board_height) * sin)).ceil() as i32;
    let rotated_height =
        ((f64::from(board_width) * sin) + (f64::from(board_height) * cos)).ceil() as i32;
    (rotated_width.max(1), rotated_height.max(1))
}

fn chess_max_square_size_for_budget(
    width_budget: i32,
    height_budget: i32,
    spacing: i32,
    angle_degrees: i32,
) -> i32 {
    if width_budget <= 0 || height_budget <= 0 {
        return 0;
    }

    let radians = (angle_degrees as f64).to_radians();
    let cos = radians.cos().abs();
    let sin = radians.sin().abs();
    let sum = cos + sin;
    if sum <= f64::EPSILON {
        return 0;
    }

    let spacing_px = spacing.max(0);
    let spacing_f64 = f64::from(spacing_px);
    let gaps = f64::from(CHESS_BOARD_SIZE - 1) * spacing_f64;
    let denom = f64::from(CHESS_BOARD_SIZE) * sum;
    let width_limit = (f64::from(width_budget) - (gaps * cos)) / denom;
    let height_limit = (f64::from(height_budget) - (gaps * sin)) / denom;
    let mut square = width_limit.min(height_limit).floor() as i32;
    if square <= 0 {
        return 0;
    }

    while square > 0 {
        let (rotated_w, rotated_h) =
            chess_rotated_board_dimensions(square, spacing_px, angle_degrees);
        if rotated_w <= width_budget && rotated_h <= height_budget {
            break;
        }
        square -= 1;
    }

    loop {
        let next = square.saturating_add(1);
        if next <= 0 || next > CHESS_MAX_SQUARE_SIZE {
            break;
        }
        let (rotated_w, rotated_h) =
            chess_rotated_board_dimensions(next, spacing_px, angle_degrees);
        if rotated_w > width_budget || rotated_h > height_budget {
            break;
        }
        square = next;
    }

    square
}

fn color_label(color: ChessColor) -> &'static str {
    match color {
        ChessColor::White => "White",
        ChessColor::Black => "Black",
    }
}

fn piece_name(piece: ChessPiece) -> &'static str {
    match piece.kind {
        ChessPieceKind::King => "King",
        ChessPieceKind::Queen => "Queen",
        ChessPieceKind::Rook => "Rook",
        ChessPieceKind::Bishop => "Bishop",
        ChessPieceKind::Knight => "Knight",
        ChessPieceKind::Pawn => "Pawn",
    }
}

fn piece_glyph(piece: ChessPiece) -> &'static str {
    match (piece.color, piece.kind) {
        (ChessColor::White, ChessPieceKind::King) => "♔",
        (ChessColor::White, ChessPieceKind::Queen) => "♕",
        (ChessColor::White, ChessPieceKind::Rook) => "♖",
        (ChessColor::White, ChessPieceKind::Bishop) => "♗",
        (ChessColor::White, ChessPieceKind::Knight) => "♘",
        (ChessColor::White, ChessPieceKind::Pawn) => "♙",
        (ChessColor::Black, ChessPieceKind::King) => "♚",
        (ChessColor::Black, ChessPieceKind::Queen) => "♛",
        (ChessColor::Black, ChessPieceKind::Rook) => "♜",
        (ChessColor::Black, ChessPieceKind::Bishop) => "♝",
        (ChessColor::Black, ChessPieceKind::Knight) => "♞",
        (ChessColor::Black, ChessPieceKind::Pawn) => "♟",
    }
}

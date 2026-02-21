use super::*;
use crate::engine::boundary;
use crate::game::{ChessColor, ChessPiece, ChessPieceKind};

impl CardthropicWindow {
    pub(super) fn setup_drag_and_drop(&self) {
        let imp = self.imp();

        let waste_hotspot = Rc::new(Cell::new((18_i32, 24_i32)));
        let freecell_drag_slot = Rc::new(Cell::new(None::<usize>));
        let waste_drag = gtk::DragSource::new();
        waste_drag.set_actions(gdk::DragAction::MOVE);
        waste_drag.connect_prepare(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[strong]
            waste_hotspot,
            #[strong]
            freecell_drag_slot,
            #[upgrade_or]
            None,
            move |_, x, y| {
                if window.imp().chess_mode_active.get() {
                    freecell_drag_slot.set(None);
                    return None;
                }
                if window.active_game_mode() == GameMode::Freecell {
                    let idx = window.freecell_slot_index_from_waste_x(x);
                    freecell_drag_slot.set(Some(idx));
                    if window
                        .imp()
                        .game
                        .borrow()
                        .freecell()
                        .freecell_card(idx)
                        .is_some()
                    {
                        let imp = window.imp();
                        let max_x = (imp.card_width.get() - 1).max(0);
                        let max_y = (imp.card_height.get() - 1).max(0);
                        let hot_x = (x.round() as i32).clamp(0, max_x);
                        let hot_y = (y.round() as i32).clamp(0, max_y);
                        waste_hotspot.set((hot_x, hot_y));
                        let payload = format!("freecell:{idx}");
                        return Some(gdk::ContentProvider::for_value(&payload.to_value()));
                    }
                    return None;
                }
                if boundary::waste_top(&window.imp().game.borrow(), window.active_game_mode())
                    .is_some()
                {
                    let imp = window.imp();
                    let max_x = (imp.card_width.get() - 1).max(0);
                    let max_y = (imp.card_height.get() - 1).max(0);
                    let hot_x = (x.round() as i32).clamp(0, max_x);
                    let hot_y = (y.round() as i32).clamp(0, max_y);
                    waste_hotspot.set((hot_x, hot_y));
                    Some(gdk::ContentProvider::for_value(&"waste".to_value()))
                } else {
                    None
                }
            }
        ));
        waste_drag.connect_drag_begin(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[strong]
            waste_hotspot,
            #[strong]
            freecell_drag_slot,
            move |source, _| {
                if window.imp().chess_mode_active.get() {
                    freecell_drag_slot.set(None);
                    return;
                }
                let imp = window.imp();
                let card = if window.active_game_mode() == GameMode::Freecell {
                    let Some(idx) = freecell_drag_slot.get() else {
                        return;
                    };
                    let Some(card) = imp.game.borrow().freecell().freecell_card(idx) else {
                        return;
                    };
                    card
                } else {
                    let Some(game) = boundary::clone_klondike_for_automation(
                        &imp.game.borrow(),
                        window.active_game_mode(),
                        window.current_klondike_draw_mode(),
                    ) else {
                        return;
                    };
                    let Some(card) = game.waste_top() else {
                        return;
                    };
                    card
                };
                let mobile = imp.mobile_phone_mode.get();
                let card_width = if mobile {
                    imp.card_width.get().max(1)
                } else {
                    imp.card_width.get().max(62)
                };
                let card_height = if mobile {
                    imp.card_height.get().max(1)
                } else {
                    imp.card_height.get().max(96)
                };
                let deck_slot = imp.deck.borrow();
                let deck = deck_slot.as_ref();
                let Some(paintable) = window.paintable_for_card_display(
                    Some(card),
                    true,
                    deck,
                    card_width,
                    card_height,
                ) else {
                    return;
                };
                let (hot_x, hot_y) = waste_hotspot.get();
                source.set_icon(Some(&paintable), hot_x, hot_y);
                window.start_drag(DragOrigin::Waste);
            }
        ));
        waste_drag.connect_drag_cancel(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            false,
            move |_, _, _| {
                window.finish_drag(false);
                false
            }
        ));
        waste_drag.connect_drag_end(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, delete_data| {
                window.finish_drag(delete_data);
            }
        ));
        imp.waste_overlay.add_controller(waste_drag);

        for (index, stack) in self.tableau_stacks().into_iter().enumerate() {
            stack.add_css_class("tableau-drop-target");
            let drag_start = Rc::new(Cell::new(None::<usize>));
            let drag_hotspot = Rc::new(Cell::new((18_i32, 24_i32)));
            let chess_drag_from_square = Rc::new(Cell::new(None::<u8>));
            let chess_hidden_widget = Rc::new(RefCell::new(None::<gtk::Widget>));
            let drag = gtk::DragSource::new();
            drag.set_actions(gdk::DragAction::MOVE);
            drag.connect_prepare(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                drag_start,
                #[strong]
                drag_hotspot,
                #[strong]
                chess_drag_from_square,
                #[upgrade_or]
                None,
                move |_, x, y| {
                    if window.imp().chess_mode_active.get() {
                        drag_start.set(None);
                        let Some(payload) = window.chess_drag_payload_for_stack_y(index, y) else {
                            window.set_chess_drag_hover_row_from_top(None);
                            return None;
                        };
                        let from_square = payload
                            .strip_prefix("chess:")
                            .and_then(|raw| raw.parse::<u8>().ok())
                            .filter(|sq| *sq < 64)?;
                        chess_drag_from_square.set(Some(from_square));
                        window.set_chess_drag_hover_row_from_top(window.chess_row_from_stack_y(y));
                        let square_size = window.imp().chess_square_size.get().max(1);
                        let hot_x = (x.round() as i32).clamp(0, square_size.saturating_sub(1));
                        let local_y = y.round() as i32;
                        let hot_y = local_y.rem_euclid(square_size);
                        drag_hotspot.set((hot_x, hot_y));
                        return Some(gdk::ContentProvider::for_value(&payload.to_value()));
                    }
                    chess_drag_from_square.set(None);
                    let mode = window.active_game_mode();
                    let start_and_top = match mode {
                        GameMode::Spider => {
                            let game = window.imp().game.borrow().spider().clone();
                            window
                                .tableau_run_start_from_y_spider(&game, index, y)
                                .map(|start| {
                                    let top =
                                        window.tableau_card_y_offset_spider(&game, index, start);
                                    (start, top)
                                })
                        }
                        GameMode::Freecell => {
                            let game = window.imp().game.borrow().freecell().clone();
                            window
                                .tableau_run_start_from_y_freecell(&game, index, y)
                                .map(|start| {
                                    let top =
                                        window.tableau_card_y_offset_freecell(&game, index, start);
                                    (start, top)
                                })
                        }
                        _ => boundary::clone_klondike_for_automation(
                            &window.imp().game.borrow(),
                            mode,
                            window.current_klondike_draw_mode(),
                        )
                        .and_then(|game| {
                            window
                                .tableau_run_start_from_y(&game, index, y)
                                .map(|start| {
                                    (start, window.tableau_card_y_offset(&game, index, start))
                                })
                        }),
                    };
                    if let Some((start, card_top)) = start_and_top {
                        let imp = window.imp();
                        let max_x = (imp.card_width.get() - 1).max(0);
                        let max_y = (imp.card_height.get() - 1).max(0);
                        let hot_x = (x.round() as i32).clamp(0, max_x);
                        let hot_y = ((y - f64::from(card_top)).round() as i32).clamp(0, max_y);
                        drag_hotspot.set((hot_x, hot_y));
                        drag_start.set(Some(start));
                        let payload = format!("tableau:{index}:{start}");
                        Some(gdk::ContentProvider::for_value(&payload.to_value()))
                    } else {
                        drag_start.set(None);
                        None
                    }
                }
            ));
            drag.connect_drag_begin(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                drag_start,
                #[strong]
                drag_hotspot,
                #[strong]
                chess_drag_from_square,
                #[strong]
                chess_hidden_widget,
                move |_source, drag| {
                    if window.imp().chess_mode_active.get() {
                        drag_start.set(None);
                        let Some(from_square) = chess_drag_from_square.get() else {
                            return;
                        };
                        if let Some(origin_widget) = window.chess_widget_for_square(from_square) {
                            origin_widget.set_opacity(0.0);
                            *chess_hidden_widget.borrow_mut() = Some(origin_widget);
                        } else {
                            *chess_hidden_widget.borrow_mut() = None;
                        }

                        let piece = window.imp().chess_position.borrow().piece_at(from_square);
                        let Some(piece) = piece else {
                            return;
                        };
                        let square_size = window.imp().chess_square_size.get().max(1);
                        let icon_label = gtk::Label::new(Some(chess_drag_piece_glyph(piece)));
                        icon_label.set_width_request(square_size);
                        icon_label.set_height_request(square_size);
                        icon_label.set_xalign(0.5);
                        icon_label.set_yalign(0.5);
                        icon_label.add_css_class("chess-drag-icon");
                        match piece.color {
                            ChessColor::White => icon_label.add_css_class("chess-piece-white"),
                            ChessColor::Black => icon_label.add_css_class("chess-piece-black"),
                        }
                        let drag_icon = gtk::DragIcon::for_drag(drag);
                        drag_icon.set_child(Some(&icon_label));
                        let (hot_x, hot_y) = drag_hotspot.get();
                        drag.set_hotspot(hot_x, hot_y);
                        return;
                    }
                    let Some(start) = drag_start.get() else {
                        return;
                    };
                    let imp = window.imp();
                    let deck_slot = imp.deck.borrow();
                    let deck = deck_slot.as_ref();
                    let mobile = imp.mobile_phone_mode.get();
                    let card_width = if mobile {
                        imp.card_width.get().max(1)
                    } else {
                        imp.card_width.get().max(62)
                    };
                    let card_height = if mobile {
                        imp.card_height.get().max(1)
                    } else {
                        imp.card_height.get().max(96)
                    };
                    let drag_widget = match window.active_game_mode() {
                        GameMode::Spider => {
                            let game = imp.game.borrow().spider().clone();
                            window.drag_icon_widget_for_tableau_run_spider(
                                &game,
                                deck,
                                index,
                                start,
                                card_width,
                                card_height,
                            )
                        }
                        GameMode::Freecell => {
                            let game = imp.game.borrow().freecell().clone();
                            window.drag_icon_widget_for_tableau_run_freecell(
                                &game,
                                deck,
                                index,
                                start,
                                card_width,
                                card_height,
                            )
                        }
                        _ => {
                            let Some(game) = boundary::clone_klondike_for_automation(
                                &imp.game.borrow(),
                                window.active_game_mode(),
                                window.current_klondike_draw_mode(),
                            ) else {
                                return;
                            };
                            window.drag_icon_widget_for_tableau_run(
                                &game,
                                deck,
                                index,
                                start,
                                card_width,
                                card_height,
                            )
                        }
                    };
                    let Some(drag_widget) = drag_widget else {
                        return;
                    };
                    let drag_icon = gtk::DragIcon::for_drag(drag);
                    drag_icon.set_child(Some(&drag_widget));
                    let (hot_x, hot_y) = drag_hotspot.get();
                    drag.set_hotspot(hot_x, hot_y);
                    window.start_drag(DragOrigin::Tableau { col: index, start });
                }
            ));
            drag.connect_drag_cancel(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                chess_drag_from_square,
                #[strong]
                chess_hidden_widget,
                #[upgrade_or]
                false,
                move |_, _, _| {
                    chess_drag_from_square.set(None);
                    window.set_chess_drag_hover_row_from_top(None);
                    if let Some(widget) = chess_hidden_widget.borrow_mut().take() {
                        widget.set_opacity(1.0);
                    }
                    window.finish_drag(false);
                    false
                }
            ));
            drag.connect_drag_end(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                chess_drag_from_square,
                #[strong]
                chess_hidden_widget,
                move |_, _, delete_data| {
                    chess_drag_from_square.set(None);
                    window.set_chess_drag_hover_row_from_top(None);
                    if delete_data {
                        let _ = chess_hidden_widget.borrow_mut().take();
                    } else if let Some(widget) = chess_hidden_widget.borrow_mut().take() {
                        widget.set_opacity(1.0);
                    }
                    window.finish_drag(delete_data);
                }
            ));
            stack.add_controller(drag);

            let tableau_drop = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
            tableau_drop.connect_motion(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                gdk::DragAction::MOVE,
                move |_, _, y| {
                    if window.imp().chess_mode_active.get() {
                        window.set_chess_drag_hover_row_from_top(window.chess_row_from_stack_y(y));
                    }
                    gdk::DragAction::MOVE
                }
            ));
            tableau_drop.connect_leave(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    if window.imp().chess_mode_active.get() {
                        window.set_chess_drag_hover_row_from_top(None);
                    }
                }
            ));
            tableau_drop.connect_drop(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                false,
                move |_, value, _, y| {
                    let Ok(payload) = value.get::<String>() else {
                        window.set_chess_drag_hover_row_from_top(None);
                        return false;
                    };
                    if window.imp().chess_mode_active.get() {
                        let dropped =
                            window.handle_chess_board_drop_from_payload(index, y, &payload);
                        window.set_chess_drag_hover_row_from_top(None);
                        return dropped;
                    }
                    window.handle_drop_on_tableau(index, &payload)
                }
            ));
            stack.add_controller(tableau_drop);
        }

        for (index, foundation) in self.foundation_pictures().into_iter().enumerate() {
            let foundation_drop = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
            foundation_drop.connect_drop(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    if window.imp().chess_mode_active.get() {
                        return false;
                    }
                    let Ok(payload) = value.get::<String>() else {
                        return false;
                    };
                    window.handle_drop_on_foundation(index, &payload)
                }
            ));
            foundation.add_controller(foundation_drop);
        }
    }
}

fn chess_drag_piece_glyph(piece: ChessPiece) -> &'static str {
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

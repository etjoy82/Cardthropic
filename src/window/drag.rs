use super::*;

impl CardthropicWindow {
    pub(super) fn setup_drag_and_drop(&self) {
        let imp = self.imp();

        let waste_hotspot = Rc::new(Cell::new((18_i32, 24_i32)));
        let waste_drag = gtk::DragSource::new();
        waste_drag.set_actions(gdk::DragAction::MOVE);
        waste_drag.connect_prepare(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[strong]
            waste_hotspot,
            #[upgrade_or]
            None,
            move |_, x, y| {
                if window.imp().game.borrow().waste_top().is_some() {
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
            move |source, _| {
                let imp = window.imp();
                let game = imp.game.borrow();
                let deck_slot = imp.deck.borrow();
                let Some(deck) = deck_slot.as_ref() else {
                    return;
                };
                let Some(card) = game.waste_top() else {
                    return;
                };
                let card_width = imp.card_width.get().max(62);
                let card_height = imp.card_height.get().max(96);
                let texture = deck.texture_for_card_scaled(card, card_width, card_height);
                let (hot_x, hot_y) = waste_hotspot.get();
                source.set_icon(Some(&texture), hot_x, hot_y);
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
            let drag = gtk::DragSource::new();
            drag.set_actions(gdk::DragAction::MOVE);
            drag.connect_prepare(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[strong]
                drag_start,
                #[strong]
                drag_hotspot,
                #[upgrade_or]
                None,
                move |_, x, y| {
                    let game = window.imp().game.borrow();
                    if let Some(start) = window.tableau_run_start_from_y(&game, index, y) {
                        let card_top = window.tableau_card_y_offset(&game, index, start);
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
                move |source, _| {
                    let Some(start) = drag_start.get() else {
                        return;
                    };
                    let imp = window.imp();
                    let game = imp.game.borrow();
                    let deck_slot = imp.deck.borrow();
                    let Some(deck) = deck_slot.as_ref() else {
                        return;
                    };
                    let Some(card) = game.tableau_card(index, start) else {
                        return;
                    };
                    let card_width = imp.card_width.get().max(62);
                    let card_height = imp.card_height.get().max(96);
                    let texture = window
                        .texture_for_tableau_drag_run(
                            &game,
                            deck,
                            index,
                            start,
                            card_width,
                            card_height,
                        )
                        .unwrap_or_else(|| {
                            if card.face_up {
                                deck.texture_for_card_scaled(card, card_width, card_height)
                            } else {
                                deck.back_texture_scaled(card_width, card_height)
                            }
                        });
                    let (hot_x, hot_y) = drag_hotspot.get();
                    source.set_icon(Some(&texture), hot_x, hot_y);
                    window.start_drag(DragOrigin::Tableau { col: index, start });
                }
            ));
            drag.connect_drag_cancel(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                false,
                move |_, _, _| {
                    window.finish_drag(false);
                    false
                }
            ));
            drag.connect_drag_end(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _, delete_data| {
                    window.finish_drag(delete_data);
                }
            ));
            stack.add_controller(drag);

            let tableau_drop = gtk::DropTarget::new(glib::Type::STRING, gdk::DragAction::MOVE);
            tableau_drop.connect_drop(glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                false,
                move |_, value, _, _| {
                    let Ok(payload) = value.get::<String>() else {
                        return false;
                    };
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
                    let Ok(payload) = value.get::<String>() else {
                        return false;
                    };
                    window.handle_drop_on_foundation(index, &payload)
                }
            ));
            foundation.add_controller(foundation_drop);
        }
    }

    pub(super) fn tableau_card_y_offset(
        &self,
        game: &KlondikeGame,
        col: usize,
        index: usize,
    ) -> i32 {
        let mut y = 0_i32;
        let face_up_step = self.imp().face_up_step.get();
        let face_down_step = self.imp().face_down_step.get();
        for idx in 0..index {
            if let Some(card) = game.tableau_card(col, idx) {
                y += if card.face_up {
                    face_up_step
                } else {
                    face_down_step
                };
            }
        }
        y
    }

    pub(super) fn texture_for_tableau_drag_run(
        &self,
        game: &KlondikeGame,
        deck: &AngloDeck,
        col: usize,
        start: usize,
        card_width: i32,
        card_height: i32,
    ) -> Option<gdk::Texture> {
        let len = game.tableau_len(col)?;
        if start >= len {
            return None;
        }

        let face_up_step = self.imp().face_up_step.get();
        let face_down_step = self.imp().face_down_step.get();
        let mut y = 0_i32;
        let mut layers: Vec<(gdk_pixbuf::Pixbuf, i32)> = Vec::new();

        for idx in start..len {
            let card = game.tableau_card(col, idx)?;
            let pixbuf = if card.face_up {
                deck.pixbuf_for_card_scaled(card, card_width, card_height)
            } else {
                deck.back_pixbuf_scaled(card_width, card_height)
            };
            layers.push((pixbuf, y));
            y += if card.face_up {
                face_up_step
            } else {
                face_down_step
            };
        }

        let run_height = layers
            .last()
            .map(|(_, pos_y)| pos_y + card_height)
            .unwrap_or(card_height)
            .max(card_height);
        let composed = gdk_pixbuf::Pixbuf::new(
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            card_width.max(1),
            run_height.max(1),
        )?;
        composed.fill(0x00000000);

        for (layer, pos_y) in layers {
            layer.copy_area(0, 0, card_width, card_height, &composed, 0, pos_y);
        }

        Some(gdk::Texture::for_pixbuf(&composed))
    }

    pub(super) fn start_drag(&self, origin: DragOrigin) {
        self.cancel_drag_timeouts();
        let imp = self.imp();
        *imp.drag_origin.borrow_mut() = Some(origin);
        imp.drag_widgets.borrow_mut().clear();

        match origin {
            DragOrigin::Waste => {
                let game = imp.game.borrow();
                let visible_waste = game
                    .waste_len()
                    .min(usize::from(game.draw_mode().count().clamp(1, 5)));
                drop(game);
                if visible_waste > 0 {
                    let slots = self.waste_fan_slots();
                    let widget: gtk::Widget = slots[visible_waste - 1].clone().upcast();
                    widget.set_opacity(0.0);
                    imp.drag_widgets.borrow_mut().push(widget);
                }
            }
            DragOrigin::Tableau { col, start } => {
                if let Some(cards) = imp.tableau_card_pictures.borrow().get(col) {
                    let mut dragged = imp.drag_widgets.borrow_mut();
                    for picture in cards.iter().skip(start) {
                        let widget: gtk::Widget = picture.clone().upcast();
                        widget.set_opacity(0.0);
                        dragged.push(widget);
                    }
                }
            }
        }
    }

    pub(super) fn finish_drag(&self, delete_data: bool) {
        let origin = self.imp().drag_origin.borrow_mut().take();
        if origin.is_none() {
            return;
        }
        if matches!(origin, Some(DragOrigin::Waste)) {
            self.imp().suppress_waste_click_once.set(true);
        }
        self.restore_drag_widgets(!delete_data);
    }

    pub(super) fn restore_drag_widgets(&self, animate: bool) {
        let widgets: Vec<gtk::Widget> = self.imp().drag_widgets.borrow_mut().drain(..).collect();
        for widget in &widgets {
            widget.set_opacity(1.0);
        }
        if !animate || widgets.is_empty() {
            return;
        }

        for widget in &widgets {
            widget.add_css_class("drag-return");
        }

        let widgets_for_timeout = widgets;
        let timeout = glib::timeout_add_local(Duration::from_millis(16), move || {
            for widget in &widgets_for_timeout {
                widget.remove_css_class("drag-return");
            }
            glib::ControlFlow::Break
        });
        self.imp().drag_timeouts.borrow_mut().push(timeout);
    }

    pub(super) fn cancel_drag_timeouts(&self) {
        for timeout_id in self.imp().drag_timeouts.borrow_mut().drain(..) {
            Self::remove_source_if_present(timeout_id);
        }
    }

    pub(super) fn remove_source_if_present(source_id: glib::SourceId) {
        if glib::MainContext::default()
            .find_source_by_id(&source_id)
            .is_some()
        {
            source_id.remove();
        }
    }

    pub(super) fn tableau_run_start_from_y(
        &self,
        game: &KlondikeGame,
        col: usize,
        y: f64,
    ) -> Option<usize> {
        let len = game.tableau_len(col)?;
        if len == 0 {
            return None;
        }

        let mut y_pos = 0.0_f64;
        let mut positions = Vec::with_capacity(len);
        let face_up_step = f64::from(self.imp().face_up_step.get());
        let face_down_step = f64::from(self.imp().face_down_step.get());
        for idx in 0..len {
            positions.push((idx, y_pos));
            let card = game.tableau_card(col, idx)?;
            y_pos += if card.face_up {
                face_up_step
            } else {
                face_down_step
            };
        }

        let mut start = positions.last().map(|(idx, _)| *idx)?;
        for (idx, pos) in positions {
            if y >= pos {
                start = idx;
            } else {
                break;
            }
        }

        if game.tableau_card(col, start)?.face_up {
            Some(start)
        } else {
            None
        }
    }
}

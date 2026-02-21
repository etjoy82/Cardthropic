use super::*;
use crate::engine::boundary;
use crate::game::{FreecellGame, SpiderGame};

impl CardthropicWindow {
    fn adjusted_tableau_hit_y(&self, y: f64) -> f64 {
        // Mobile cards are small; add a small vertical hit slop so taps near a
        // card edge still resolve to a useful run start.
        if self.imp().mobile_phone_mode.get() {
            (y + 8.0).max(0.0)
        } else {
            y.max(0.0)
        }
    }

    fn drag_icon_widget_from_layers(
        &self,
        layers: &[(Card, bool, i32)],
        deck: Option<&AngloDeck>,
        card_width: i32,
        card_height: i32,
    ) -> Option<gtk::Widget> {
        if layers.is_empty() {
            return None;
        }
        let run_height = layers
            .last()
            .map(|(_, _, pos_y)| pos_y + card_height)
            .unwrap_or(card_height)
            .max(card_height);

        let fixed = gtk::Fixed::new();
        fixed.set_width_request(card_width.max(1));
        fixed.set_height_request(run_height.max(1));

        for (card, face_up, pos_y) in layers {
            let picture = gtk::Picture::new();
            picture.set_width_request(card_width.max(1));
            picture.set_height_request(card_height.max(1));
            picture.set_can_shrink(true);
            picture.set_content_fit(gtk::ContentFit::Contain);
            if let Some(paintable) = self.paintable_for_card_display(
                Some(*card),
                *face_up,
                deck,
                card_width,
                card_height,
            ) {
                picture.set_paintable(Some(&paintable));
            }
            fixed.put(&picture, 0.0, f64::from(*pos_y));
        }

        Some(fixed.upcast::<gtk::Widget>())
    }

    pub(super) fn drag_icon_widget_for_tableau_run(
        &self,
        game: &KlondikeGame,
        deck: Option<&AngloDeck>,
        col: usize,
        start: usize,
        card_width: i32,
        card_height: i32,
    ) -> Option<gtk::Widget> {
        let len = game.tableau_len(col)?;
        if start >= len {
            return None;
        }
        let face_up_step = self.imp().face_up_step.get();
        let face_down_step = self.imp().face_down_step.get();
        let mut layers: Vec<(Card, bool, i32)> = Vec::new();
        let mut y = 0_i32;
        for idx in start..len {
            let card = game.tableau_card(col, idx)?;
            layers.push((card, card.face_up, y));
            y += if card.face_up {
                face_up_step
            } else {
                face_down_step
            };
        }
        self.drag_icon_widget_from_layers(&layers, deck, card_width, card_height)
    }

    pub(super) fn drag_icon_widget_for_tableau_run_spider(
        &self,
        game: &SpiderGame,
        deck: Option<&AngloDeck>,
        col: usize,
        start: usize,
        card_width: i32,
        card_height: i32,
    ) -> Option<gtk::Widget> {
        let len = game.tableau().get(col).map(Vec::len)?;
        if start >= len {
            return None;
        }
        let face_up_step = self.imp().face_up_step.get();
        let face_down_step = self.imp().face_down_step.get();
        let mut layers: Vec<(Card, bool, i32)> = Vec::new();
        let mut y = 0_i32;
        for idx in start..len {
            let card = game.tableau_card(col, idx)?;
            layers.push((card, card.face_up, y));
            y += if card.face_up {
                face_up_step
            } else {
                face_down_step
            };
        }
        self.drag_icon_widget_from_layers(&layers, deck, card_width, card_height)
    }

    pub(super) fn drag_icon_widget_for_tableau_run_freecell(
        &self,
        game: &FreecellGame,
        deck: Option<&AngloDeck>,
        col: usize,
        start: usize,
        card_width: i32,
        card_height: i32,
    ) -> Option<gtk::Widget> {
        let len = game.tableau().get(col).map(Vec::len)?;
        if start >= len {
            return None;
        }
        let step = self.imp().face_up_step.get();
        let mut layers: Vec<(Card, bool, i32)> = Vec::new();
        let mut y = 0_i32;
        for idx in start..len {
            let card = game.tableau_card(col, idx)?;
            layers.push((card, true, y));
            y += step;
        }
        self.drag_icon_widget_from_layers(&layers, deck, card_width, card_height)
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

    pub(super) fn tableau_card_y_offset_spider(
        &self,
        game: &SpiderGame,
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

    pub(super) fn start_drag(&self, origin: DragOrigin) {
        self.cancel_drag_timeouts();
        let imp = self.imp();
        *imp.drag_origin.borrow_mut() = Some(origin);
        imp.drag_widgets.borrow_mut().clear();

        match origin {
            DragOrigin::Waste => {
                let Some(game) = boundary::clone_klondike_for_automation(
                    &imp.game.borrow(),
                    self.active_game_mode(),
                    self.current_klondike_draw_mode(),
                ) else {
                    return;
                };
                let visible_waste = game
                    .waste_len()
                    .min(usize::from(game.draw_mode().count().clamp(1, 5)));
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
        let y = self.adjusted_tableau_hit_y(y);
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

    pub(super) fn tableau_run_start_from_y_spider(
        &self,
        game: &SpiderGame,
        col: usize,
        y: f64,
    ) -> Option<usize> {
        let y = self.adjusted_tableau_hit_y(y);
        let len = game.tableau().get(col).map(Vec::len)?;
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

    pub(super) fn tableau_card_y_offset_freecell(
        &self,
        _game: &FreecellGame,
        _col: usize,
        index: usize,
    ) -> i32 {
        (index as i32) * self.imp().face_up_step.get()
    }

    pub(super) fn tableau_run_start_from_y_freecell(
        &self,
        game: &FreecellGame,
        col: usize,
        y: f64,
    ) -> Option<usize> {
        let y = self.adjusted_tableau_hit_y(y);
        let len = game.tableau().get(col).map(Vec::len)?;
        if len == 0 {
            return None;
        }
        let step = f64::from(self.imp().face_up_step.get().max(1));
        let idx = (y / step).floor().max(0.0) as usize;
        Some(idx.min(len - 1))
    }
}

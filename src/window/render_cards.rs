use super::*;
use crate::game::rank_label;

impl CardthropicWindow {
    fn append_layout_at(
        snapshot: &gtk::Snapshot,
        layout: &gtk::pango::Layout,
        color: &gdk::RGBA,
        x: f32,
        y: f32,
    ) {
        snapshot.save();
        snapshot.translate(&gtk::graphene::Point::new(x, y));
        snapshot.append_layout(layout, color);
        snapshot.restore();
    }

    pub(super) fn current_card_render_mode(&self) -> CardRenderMode {
        CardRenderMode::Unicode
    }

    pub(super) fn set_card_render_mode(
        &self,
        _mode: CardRenderMode,
        _persist: bool,
        announce: bool,
    ) {
        let imp = self.imp();
        if imp.card_render_mode.get() == CardRenderMode::Unicode {
            return;
        }
        imp.card_render_mode.set(CardRenderMode::Unicode);
        imp.unicode_card_paintable_cache.borrow_mut().clear();
        for column in imp.tableau_picture_state_cache.borrow_mut().iter_mut() {
            for slot in column.iter_mut() {
                *slot = None;
            }
        }

        if announce {
            *imp.status_override.borrow_mut() = Some(format!(
                "Card rendering set to {}.",
                CardRenderMode::Unicode.label()
            ));
        }
        self.render();
    }

    pub(super) fn paintable_for_card_display(
        &self,
        card: Option<Card>,
        face_up: bool,
        _deck: Option<&AngloDeck>,
        width: i32,
        height: i32,
    ) -> Option<gdk::Paintable> {
        let width = width.max(1);
        let height = height.max(1);
        if face_up && card.is_none() {
            return None;
        }

        let paintable = match (face_up, card) {
            (true, Some(card)) => self.unicode_card_paintable(Some(card), true, width, height),
            (false, _) => self.unicode_card_paintable(None, false, width, height),
            (true, None) => return None,
        };
        Some(paintable)
    }

    fn unicode_card_paintable(
        &self,
        card: Option<Card>,
        face_up: bool,
        width: i32,
        height: i32,
    ) -> gdk::Paintable {
        let width = width.max(1);
        let height = height.max(1);
        let key = UnicodeCardPaintableKey {
            card,
            face_up,
            width,
            height,
        };
        if let Some(cached) = self.imp().unicode_card_paintable_cache.borrow().get(&key) {
            return cached.clone();
        }

        let bounds = gtk::graphene::Rect::new(0.0, 0.0, width as f32, height as f32);
        let radius = (width.min(height) as f32 * 0.08).max(3.0);
        let rounded = gtk::gsk::RoundedRect::from_rect(bounds, radius);
        let snapshot = gtk::Snapshot::new();

        let border = gdk::RGBA::new(0.28, 0.33, 0.41, 1.0);
        let face_background = gdk::RGBA::new(0.95, 0.97, 1.0, 1.0);
        let back_background = gdk::RGBA::new(0.18, 0.22, 0.33, 1.0);

        snapshot.push_rounded_clip(&rounded);
        snapshot.append_color(
            if face_up {
                &face_background
            } else {
                &back_background
            },
            &bounds,
        );
        snapshot.pop();
        snapshot.append_border(
            &rounded,
            &[1.0, 1.0, 1.0, 1.0],
            &[border, border, border, border],
        );

        if face_up {
            if let Some(card) = card {
                let context = self.imp().status_label.pango_context();
                let text_color = if card.color_red() {
                    gdk::RGBA::new(0.79, 0.16, 0.2, 1.0)
                } else {
                    gdk::RGBA::new(0.1, 0.13, 0.2, 1.0)
                };
                let rank = rank_label(card.rank);
                let suit = Self::unicode_suit(card.suit);

                let corner_layout = gtk::pango::Layout::new(&context);
                corner_layout.set_markup(&format!(
                    "<span>{rank}</span><span size=\"80%\">{suit}</span>"
                ));
                corner_layout.set_alignment(gtk::pango::Alignment::Center);
                let mut corner_font = gtk::pango::FontDescription::new();
                corner_font.set_family("DejaVu Sans");
                corner_font.set_weight(gtk::pango::Weight::Bold);
                corner_font.set_size(
                    ((height as f64 * 0.17) * f64::from(gtk::pango::SCALE)).round() as i32,
                );
                corner_layout.set_font_description(Some(&corner_font));

                let center_layout = gtk::pango::Layout::new(&context);
                center_layout.set_text(suit);
                center_layout.set_alignment(gtk::pango::Alignment::Center);
                let mut center_font = gtk::pango::FontDescription::new();
                center_font.set_family("DejaVu Sans");
                center_font.set_weight(gtk::pango::Weight::Bold);
                center_font.set_size(
                    ((height as f64 * 0.46) * f64::from(gtk::pango::SCALE)).round() as i32,
                );
                center_layout.set_font_description(Some(&center_font));
                let (center_w, center_h) = center_layout.pixel_size();

                let padding_x = (width as f32 * 0.07).max(3.0);
                let padding_y = (height as f32 * 0.05).max(2.0);
                let top_x = padding_x;
                let top_y = padding_y;
                let pip_x = (width as f32 - padding_x - center_w as f32).max(0.0);
                let pip_y = (height as f32 - padding_y - center_h as f32).max(0.0);

                Self::append_layout_at(&snapshot, &corner_layout, &text_color, top_x, top_y);
                Self::append_layout_at(&snapshot, &center_layout, &text_color, pip_x, pip_y);
            }
        } else {
            let context = self.imp().status_label.pango_context();
            let layout = gtk::pango::Layout::new(&context);
            layout.set_text("◈");
            let mut font = gtk::pango::FontDescription::new();
            font.set_family("DejaVu Sans");
            font.set_weight(gtk::pango::Weight::Bold);
            font.set_size(((height as f64 * 0.28) * f64::from(gtk::pango::SCALE)).round() as i32);
            layout.set_font_description(Some(&font));
            let (text_width, text_height) = layout.pixel_size();
            let text_x = ((width - text_width).max(0) / 2) as f32;
            let text_y = ((height - text_height).max(0) / 2) as f32;
            Self::append_layout_at(
                &snapshot,
                &layout,
                &gdk::RGBA::new(0.71, 0.78, 0.93, 0.95),
                text_x,
                text_y,
            );
        }

        let size = gtk::graphene::Size::new(width as f32, height as f32);
        let paintable = snapshot
            .to_paintable(Some(&size))
            .unwrap_or_else(|| Self::blank_texture(width, height).upcast::<gdk::Paintable>());
        self.imp()
            .unicode_card_paintable_cache
            .borrow_mut()
            .insert(key, paintable.clone());
        paintable
    }

    fn unicode_suit(suit: Suit) -> &'static str {
        match suit {
            Suit::Clubs => "♣",
            Suit::Diamonds => "♦",
            Suit::Hearts => "♥",
            Suit::Spades => "♠",
        }
    }
}

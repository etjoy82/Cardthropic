use super::*;
use crate::engine::render_plan;
use crate::game::SpiderGame;

impl CardthropicWindow {
    pub(super) fn configure_stock_waste_foundation_widgets(
        &self,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();
        let size = (card_width, card_height);
        if imp.last_stock_waste_foundation_size.get() == size {
            if imp.waste_picture.paintable().is_some() {
                imp.waste_picture.set_paintable(None::<&gdk::Paintable>);
            }
            return;
        }
        imp.last_stock_waste_foundation_size.set(size);

        imp.stock_picture.set_width_request(card_width);
        imp.stock_picture.set_height_request(card_height);
        imp.stock_picture.set_can_shrink(false);
        imp.waste_picture.set_width_request(card_width);
        imp.waste_picture.set_height_request(card_height);
        imp.waste_picture.set_can_shrink(false);
        imp.waste_picture.set_halign(gtk::Align::Start);
        imp.waste_picture.set_valign(gtk::Align::Start);
        imp.waste_picture.set_paintable(None::<&gdk::Paintable>);
        imp.waste_placeholder_box.set_width_request(card_width);
        imp.waste_placeholder_box.set_height_request(card_height);
        for picture in self.waste_fan_slots() {
            picture.set_width_request(card_width);
            picture.set_height_request(card_height);
            picture.set_can_shrink(false);
        }
        let waste_fan_step = render_plan::waste_fan_step(card_width);
        let foundation_group_width = render_plan::foundation_group_width(card_width);
        imp.stock_heading_box.set_width_request(card_width);
        imp.waste_heading_box
            .set_width_request(card_width + (waste_fan_step * 4));
        imp.foundations_heading_box
            .set_width_request(foundation_group_width);
        imp.waste_overlay
            .set_width_request(render_plan::waste_overlay_width(card_width));
        imp.waste_overlay.set_height_request(card_height);
        for picture in self.foundation_pictures() {
            picture.set_width_request(card_width);
            picture.set_height_request(card_height);
            picture.set_can_shrink(false);
        }
    }

    pub(super) fn render_stock_picture(
        &self,
        game: &KlondikeGame,
        deck: &AngloDeck,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();
        if game.stock_len() > 0 {
            let back = deck.back_texture_scaled(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&back));
        } else {
            let empty = Self::blank_texture(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&empty));
        }
    }

    pub(super) fn render_stock_picture_spider(
        &self,
        game: &SpiderGame,
        deck: &AngloDeck,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();
        if game.stock_len() > 0 {
            let back = deck.back_texture_scaled(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&back));
        } else {
            let empty = Self::blank_texture(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&empty));
        }
    }

    pub(super) fn render_waste_fan(
        &self,
        game: &KlondikeGame,
        deck: &AngloDeck,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();
        let waste_widgets = self.waste_fan_slots();
        let waste_cards = game.waste_top_n(render_plan::waste_visible_count(
            game.draw_mode(),
            game.waste_len(),
        ));
        let show_count = waste_cards.len();
        let waste_selected = imp.waste_selected.get();
        let waste_fan_step = render_plan::waste_fan_step(card_width);

        for picture in &waste_widgets {
            picture.set_visible(false);
            picture.set_margin_start(0);
            picture.set_paintable(None::<&gdk::Paintable>);
        }

        for (idx, card) in waste_cards.iter().copied().enumerate() {
            if let Some(picture) = waste_widgets.get(idx) {
                if waste_selected && idx + 1 == show_count {
                    picture.add_css_class("waste-selected-card");
                } else {
                    picture.remove_css_class("waste-selected-card");
                }
                let texture = deck.texture_for_card_scaled(card, card_width, card_height);
                picture.set_paintable(Some(&texture));
                if idx > 0 {
                    picture.set_margin_start((idx as i32) * waste_fan_step);
                }
                picture.set_visible(true);
            }
        }
        for picture in waste_widgets.iter().skip(show_count) {
            picture.remove_css_class("waste-selected-card");
        }
        imp.waste_placeholder_label.set_visible(show_count == 0);
    }

    pub(super) fn render_foundations_area(
        &self,
        game: &KlondikeGame,
        deck: &AngloDeck,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();
        for (idx, picture) in self.foundation_pictures().into_iter().enumerate() {
            let top = game.foundations()[idx].last().copied();
            self.set_picture_from_card(&picture, top, deck, card_width, card_height);
        }
        let foundation_empty = render_plan::foundation_empty_flags(game);
        imp.foundation_placeholder_1
            .set_visible(foundation_empty[0]);
        imp.foundation_placeholder_2
            .set_visible(foundation_empty[1]);
        imp.foundation_placeholder_3
            .set_visible(foundation_empty[2]);
        imp.foundation_placeholder_4
            .set_visible(foundation_empty[3]);
    }

    pub(super) fn render_waste_fan_spider(&self) {
        let imp = self.imp();
        for picture in self.waste_fan_slots() {
            picture.set_visible(false);
            picture.set_margin_start(0);
            picture.set_paintable(None::<&gdk::Paintable>);
            picture.remove_css_class("waste-selected-card");
        }
        imp.waste_placeholder_label.set_visible(true);
    }

    pub(super) fn render_foundations_area_spider(
        &self,
        _game: &SpiderGame,
        deck: &AngloDeck,
        card_width: i32,
        card_height: i32,
    ) {
        for picture in self.foundation_pictures() {
            self.set_picture_from_card(&picture, None, deck, card_width, card_height);
        }
        for placeholder in self.foundation_placeholders() {
            placeholder.set_visible(true);
        }
    }
}

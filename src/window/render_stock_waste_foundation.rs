use super::*;
use crate::engine::render_plan;
use crate::game::{
    Card, FreecellGame, SpiderGame, Suit, FREECELL_MAX_CELL_COUNT, FREECELL_MIN_CELL_COUNT,
};

impl CardthropicWindow {
    fn freecell_slot_step(card_width: i32) -> i32 {
        card_width.max(1)
    }

    pub(super) fn configure_stock_waste_foundation_widgets(
        &self,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();

        // Prevent top-row overlays from inheriting vertical expansion and
        // consuming tableau space in FreeCell.
        imp.top_playfield_frame.set_vexpand(false);
        imp.playfield_inner_box.set_vexpand(false);
        imp.stock_waste_foundations_row_box.set_vexpand(false);
        imp.stock_waste_foundations_row_box
            .set_valign(gtk::Align::Start);
        imp.waste_column_box.set_vexpand(false);
        imp.waste_column_box.set_valign(gtk::Align::Start);
        imp.waste_overlay.set_vexpand(false);
        imp.waste_overlay.set_valign(gtk::Align::Start);
        imp.waste_label.set_valign(gtk::Align::Start);

        let freecell_cells = if self.active_game_mode() == GameMode::Freecell {
            self.current_freecell_cell_count()
                .clamp(FREECELL_MIN_CELL_COUNT, FREECELL_MAX_CELL_COUNT)
        } else {
            0
        };
        let size = (
            card_width,
            card_height,
            self.active_game_mode(),
            freecell_cells,
        );
        if imp.last_stock_waste_foundation_size.get() == size {
            if imp.waste_picture.paintable().is_some() {
                imp.waste_picture.set_paintable(None::<&gdk::Paintable>);
            }
            return;
        }
        imp.last_stock_waste_foundation_size.set(size);

        let shrink_cards = self.active_game_mode() == GameMode::Spider;
        let freecell_mode = self.active_game_mode() == GameMode::Freecell;
        if freecell_mode {
            // Keep FreeCell's top lane compact and deterministic so the tableau
            // never gets pushed down by expansion quirks in overlay children.
            let row_height = card_height.saturating_add(28);
            imp.stock_waste_foundations_row_box
                .set_height_request(row_height);
            imp.top_playfield_frame
                .set_height_request(row_height.saturating_add(32));
        } else {
            imp.stock_waste_foundations_row_box.set_height_request(-1);
            imp.top_playfield_frame.set_height_request(-1);
        }
        imp.stock_picture.set_width_request(card_width);
        imp.stock_picture.set_height_request(card_height);
        imp.stock_picture.set_can_shrink(shrink_cards);
        imp.stock_picture.set_content_fit(gtk::ContentFit::Contain);
        imp.waste_picture.set_width_request(card_width);
        imp.waste_picture.set_height_request(card_height);
        imp.waste_picture.set_can_shrink(shrink_cards);
        imp.waste_picture.set_content_fit(gtk::ContentFit::Contain);
        imp.waste_picture.set_halign(gtk::Align::Start);
        imp.waste_picture.set_valign(gtk::Align::Start);
        imp.waste_picture.set_paintable(None::<&gdk::Paintable>);
        imp.waste_placeholder_box.set_width_request(card_width);
        imp.waste_placeholder_box.set_height_request(card_height);
        for picture in self.waste_fan_slots() {
            picture.set_width_request(card_width);
            picture.set_height_request(card_height);
            picture.set_can_shrink(shrink_cards);
            picture.set_content_fit(gtk::ContentFit::Contain);
        }
        let spider_mode = self.active_game_mode() == GameMode::Spider;
        let mobile_mode = imp.mobile_phone_mode.get();
        let foundation_slots = if spider_mode { 8 } else { 4 };
        let foundation_gap = if spider_mode {
            0
        } else if mobile_mode {
            2
        } else {
            8
        };
        let foundation_group_width =
            (card_width * foundation_slots) + (foundation_gap * (foundation_slots - 1));
        let waste_strip_width = if self.active_game_mode() == GameMode::Spider {
            card_width
        } else if self.active_game_mode() == GameMode::Freecell {
            let freecell_slots = i32::from(self.current_freecell_cell_count())
                .clamp(1, i32::from(FREECELL_MAX_CELL_COUNT));
            let step = Self::freecell_slot_step(card_width);
            card_width + step * freecell_slots.saturating_sub(1)
        } else {
            render_plan::waste_overlay_width(card_width)
        };
        imp.stock_heading_box.set_width_request(card_width);
        imp.waste_heading_box.set_width_request(waste_strip_width);
        imp.stock_column_box.set_width_request(card_width);
        imp.waste_column_box.set_width_request(waste_strip_width);
        imp.stock_label.set_width_request(card_width);
        imp.waste_label.set_width_request(waste_strip_width);
        imp.foundations_heading_box
            .set_width_request(foundation_group_width);
        imp.foundations_area_box
            .set_width_request(foundation_group_width);
        imp.waste_overlay.set_width_request(waste_strip_width);
        imp.waste_overlay.set_height_request(card_height);
        for picture in self.foundation_pictures() {
            picture.set_width_request(card_width);
            picture.set_height_request(card_height);
            picture.set_can_shrink(false);
            picture.set_content_fit(gtk::ContentFit::Contain);
        }
    }

    pub(super) fn render_stock_picture(
        &self,
        game: &KlondikeGame,
        deck: Option<&AngloDeck>,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();
        if game.stock_len() > 0 {
            if let Some(back) =
                self.paintable_for_card_display(None, false, deck, card_width, card_height)
            {
                imp.stock_picture.set_paintable(Some(&back));
            } else {
                let empty = Self::blank_texture(card_width, card_height);
                imp.stock_picture.set_paintable(Some(&empty));
            }
        } else {
            let empty = Self::blank_texture(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&empty));
        }
    }

    pub(super) fn render_stock_picture_spider(
        &self,
        game: &SpiderGame,
        deck: Option<&AngloDeck>,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();
        if game.stock_len() > 0 {
            if let Some(back) =
                self.paintable_for_card_display(None, false, deck, card_width, card_height)
            {
                imp.stock_picture.set_paintable(Some(&back));
            } else {
                let empty = Self::blank_texture(card_width, card_height);
                imp.stock_picture.set_paintable(Some(&empty));
            }
        } else {
            let empty = Self::blank_texture(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&empty));
        }
    }

    pub(super) fn render_waste_fan(
        &self,
        game: &KlondikeGame,
        deck: Option<&AngloDeck>,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();
        imp.waste_picture.set_visible(true);
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
                if let Some(paintable) =
                    self.paintable_for_card_display(Some(card), true, deck, card_width, card_height)
                {
                    picture.set_paintable(Some(&paintable));
                } else {
                    picture.set_paintable(None::<&gdk::Paintable>);
                }
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
        deck: Option<&AngloDeck>,
        card_width: i32,
        card_height: i32,
    ) {
        self.sync_foundation_slots_with_state();
        let pictures = self.foundation_pictures();
        let placeholders = self.foundation_placeholders();
        let slot_boxes: Vec<Option<gtk::Box>> = pictures
            .iter()
            .map(|picture| {
                picture
                    .parent()
                    .and_then(|widget| widget.parent())
                    .and_then(|widget| widget.downcast::<gtk::Box>().ok())
            })
            .collect();
        for slot in 0..4 {
            if let Some(slot_box) = slot_boxes[slot].as_ref() {
                slot_box.set_visible(true);
            }
            pictures[slot].set_visible(true);
            let top = self
                .foundation_slot_suit(slot)
                .and_then(|suit| game.foundations()[suit.foundation_index()].last().copied());
            self.set_picture_from_card(&pictures[slot], top, deck, card_width, card_height);
            placeholders[slot].set_label("");
            let empty = self
                .foundation_slot_suit(slot)
                .map(|suit| game.foundations()[suit.foundation_index()].is_empty())
                .unwrap_or(true);
            placeholders[slot].set_visible(empty);
        }
        for slot in 4..8 {
            if let Some(slot_box) = slot_boxes[slot].as_ref() {
                slot_box.set_visible(false);
            }
            pictures[slot].set_visible(false);
            self.set_picture_from_card(&pictures[slot], None, deck, card_width, card_height);
            placeholders[slot].set_label("");
            placeholders[slot].set_visible(false);
        }
    }

    pub(super) fn render_waste_fan_spider(&self, card_width: i32, card_height: i32) {
        let imp = self.imp();
        imp.waste_picture.set_visible(true);
        let empty = Self::blank_texture(card_width, card_height);
        imp.waste_picture.set_paintable(Some(&empty));
        for picture in self.waste_fan_slots() {
            picture.set_visible(false);
            picture.set_margin_start(0);
            picture.set_paintable(None::<&gdk::Paintable>);
            picture.remove_css_class("waste-selected-card");
        }
        imp.waste_placeholder_label.set_visible(false);
    }

    pub(super) fn render_foundations_area_spider(
        &self,
        game: &SpiderGame,
        deck: Option<&AngloDeck>,
        card_width: i32,
        card_height: i32,
    ) {
        let completed = game.completed_runs().min(8);
        let completed_suits = game.completed_run_suits();
        let pictures = self.foundation_pictures();
        let slot_boxes: Vec<Option<gtk::Box>> = pictures
            .iter()
            .map(|picture| {
                picture
                    .parent()
                    .and_then(|widget| widget.parent())
                    .and_then(|widget| widget.downcast::<gtk::Box>().ok())
            })
            .collect();
        for (slot, picture) in pictures.into_iter().enumerate() {
            if let Some(slot_box) = slot_boxes[slot].as_ref() {
                slot_box.set_visible(true);
            }
            picture.set_visible(true);
            if slot < completed {
                let marker = Card {
                    suit: completed_suits.get(slot).copied().unwrap_or(Suit::Spades),
                    rank: 13,
                    face_up: true,
                };
                self.set_picture_from_card(&picture, Some(marker), deck, card_width, card_height);
            } else {
                self.set_picture_from_card(&picture, None, deck, card_width, card_height);
            }
        }
        for (slot, placeholder) in self.foundation_placeholders().into_iter().enumerate() {
            placeholder.set_label("");
            placeholder.set_visible(slot >= completed);
        }
    }

    pub(super) fn freecell_slot_index_from_waste_x(&self, x: f64) -> usize {
        let card_width = self.imp().card_width.get();
        let step = Self::freecell_slot_step(card_width);
        let slot_count = i32::from(self.current_freecell_cell_count()).max(1);
        let idx = (x.max(0.0) as i32) / step.max(1);
        idx.clamp(0, slot_count - 1) as usize
    }

    pub(super) fn render_freecell_slots(
        &self,
        game: &FreecellGame,
        deck: Option<&AngloDeck>,
        card_width: i32,
        card_height: i32,
    ) {
        let imp = self.imp();
        // Keep the overlay's main child out of FreeCell measurement; the six
        // overlay children below render all free-cell slots.
        imp.waste_picture.set_visible(false);
        imp.waste_picture.set_paintable(None::<&gdk::Paintable>);
        let slots = self.freecell_slot_pictures();
        let selected = imp.selected_freecell.get();
        let step = Self::freecell_slot_step(card_width);
        let active_slots = game.freecell_count().min(slots.len());
        let strip_width = card_width + (step * (active_slots.saturating_sub(1) as i32));
        imp.waste_overlay.set_width_request(strip_width);
        imp.waste_heading_box.set_width_request(strip_width);
        imp.waste_column_box.set_width_request(strip_width);
        imp.stock_column_box.set_width_request(card_width);

        imp.waste_placeholder_label.set_visible(false);

        for (idx, picture) in slots.iter().enumerate() {
            if idx < active_slots {
                picture.set_visible(true);
                picture.set_margin_start((idx as i32) * step);
                // FreeCell slots live in a wide overlay strip; using `Contain`
                // lets width-for-height sizing scale natural height with strip
                // width, which can inflate the entire top row. `Fill` keeps a
                // stable slot measure and prevents tableau push-down.
                picture.set_content_fit(gtk::ContentFit::Fill);
                picture.set_halign(gtk::Align::Start);
                picture.set_valign(gtk::Align::Start);
                picture.set_hexpand(false);
                picture.set_vexpand(false);
                let card = game.freecell_card(idx);
                if let Some(card) = card {
                    if let Some(paintable) = self.paintable_for_card_display(
                        Some(card),
                        true,
                        deck,
                        card_width,
                        card_height,
                    ) {
                        picture.set_paintable(Some(&paintable));
                    } else {
                        picture.set_paintable(None::<&gdk::Paintable>);
                    }
                } else {
                    let empty = Self::blank_texture(card_width, card_height);
                    picture.set_paintable(Some(&empty));
                }
                if selected == Some(idx) {
                    picture.add_css_class("waste-selected-card");
                } else {
                    picture.remove_css_class("waste-selected-card");
                }
            } else {
                picture.set_visible(false);
                picture.set_margin_start(0);
                picture.set_paintable(None::<&gdk::Paintable>);
                picture.remove_css_class("waste-selected-card");
            }
        }

        let occupied = game
            .freecells()
            .iter()
            .filter(|slot| slot.is_some())
            .count();
        imp.waste_label
            .set_label(&format!("Free Cells: {occupied}/{}", active_slots));
    }

    pub(super) fn render_foundations_area_freecell(
        &self,
        game: &FreecellGame,
        deck: Option<&AngloDeck>,
        card_width: i32,
        card_height: i32,
    ) {
        self.sync_foundation_slots_with_state();
        let pictures = self.foundation_pictures();
        let placeholders = self.foundation_placeholders();
        let slot_boxes: Vec<Option<gtk::Box>> = pictures
            .iter()
            .map(|picture| {
                picture
                    .parent()
                    .and_then(|widget| widget.parent())
                    .and_then(|widget| widget.downcast::<gtk::Box>().ok())
            })
            .collect();
        for slot in 0..4 {
            if let Some(slot_box) = slot_boxes[slot].as_ref() {
                slot_box.set_visible(true);
            }
            pictures[slot].set_visible(true);
            let top = self
                .foundation_slot_suit(slot)
                .and_then(|suit| game.foundations()[suit.foundation_index()].last().copied());
            self.set_picture_from_card(&pictures[slot], top, deck, card_width, card_height);
            placeholders[slot].set_label("");
            let empty = self
                .foundation_slot_suit(slot)
                .map(|suit| game.foundations()[suit.foundation_index()].is_empty())
                .unwrap_or(true);
            placeholders[slot].set_visible(empty);
        }
        for slot in 4..8 {
            if let Some(slot_box) = slot_boxes[slot].as_ref() {
                slot_box.set_visible(false);
            }
            pictures[slot].set_visible(false);
            self.set_picture_from_card(&pictures[slot], None, deck, card_width, card_height);
            placeholders[slot].set_label("");
            placeholders[slot].set_visible(false);
        }
    }
}

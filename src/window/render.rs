use super::*;

impl CardthropicWindow {
    pub(super) fn render(&self) {
        let imp = self.imp();
        let game = imp.game.borrow();
        let mode = self.active_game_mode();
        let engine_ready = self.is_mode_engine_ready();
        if engine_ready {
            self.note_current_seed_win_if_needed(&game);
            if game.is_won() && imp.timer_started.get() {
                imp.timer_started.set(false);
            }
        }

        imp.stock_label
            .set_label(&format!("{} cards", game.stock_len()));

        imp.waste_label
            .set_label(&format!("{} cards", game.waste_len()));

        let foundation_labels = [
            &imp.foundation_label_1,
            &imp.foundation_label_2,
            &imp.foundation_label_3,
            &imp.foundation_label_4,
        ];

        for label in foundation_labels {
            label.set_label("");
        }

        self.render_card_images(&game);

        imp.undo_button
            .set_sensitive(engine_ready && !imp.history.borrow().is_empty());
        imp.redo_button
            .set_sensitive(engine_ready && !imp.future.borrow().is_empty());
        imp.auto_hint_button.set_sensitive(engine_ready);
        imp.cyclone_shuffle_button.set_sensitive(engine_ready);
        imp.peek_button.set_sensitive(engine_ready);
        imp.robot_button.set_sensitive(engine_ready);
        imp.seed_random_button.set_sensitive(engine_ready);
        imp.seed_rescue_button.set_sensitive(engine_ready);
        imp.seed_winnable_button.set_sensitive(engine_ready);
        imp.seed_repeat_button.set_sensitive(engine_ready);
        imp.seed_go_button.set_sensitive(engine_ready);
        imp.seed_combo.set_sensitive(engine_ready);

        let selected = sanitize_selected_run(&game, *imp.selected_run.borrow());
        *imp.selected_run.borrow_mut() = selected;
        self.update_tableau_selection_styles(selected);
        if imp.waste_selected.get() && game.waste_top().is_none() {
            imp.waste_selected.set(false);
        }
        self.update_waste_selection_style(imp.waste_selected.get() && game.waste_top().is_some());
        self.update_keyboard_focus_style();
        if let Some(err) = imp.deck_error.borrow().as_ref() {
            imp.status_label
                .set_label(&format!("Card deck load failed: {err}"));
        } else if let Some(message) = imp.status_override.borrow().as_ref() {
            imp.status_label.set_label(message);
        } else if game.is_won() {
            imp.status_label
                .set_label("You won! All foundations are complete.");
        } else if let Some(run) = selected {
            let amount = game
                .tableau_len(run.col)
                .unwrap_or(0)
                .saturating_sub(run.start);
            if amount > 1 {
                imp.status_label.set_label(&format!(
                    "Selected {amount} cards from T{}. Click another tableau to move this run.",
                    run.col + 1
                ));
            } else {
                imp.status_label.set_label(&format!(
                    "Selected tableau T{}. Click another tableau to move top card.",
                    run.col + 1
                ));
            }
        } else if imp.waste_selected.get() && game.waste_top().is_some() {
            imp.status_label.set_label(
                "Selected waste. Click a tableau to move it, or click waste again to cancel.",
            );
        } else if imp.peek_active.get() {
            imp.status_label.set_label(
                "Peek active: tableau face-up cards are hidden and face-down cards are revealed.",
            );
        } else if !engine_ready {
            imp.status_label.set_label(&format!(
                "{} mode scaffolded. Rules/engine are in progress.",
                mode.label()
            ));
        } else {
            let controls = match self.smart_move_mode() {
                SmartMoveMode::Disabled => {
                    "Klondike controls: click columns/waste to select and move manually. Smart Move is off."
                }
                SmartMoveMode::SingleClick => {
                    "Klondike controls: single-click cards/waste for Smart Move. Use drag-and-drop for manual runs."
                }
                SmartMoveMode::DoubleClick => {
                    "Klondike controls: click columns to move, click waste to select, double-click cards/waste for Smart Move."
                }
            };
            imp.status_label.set_label(controls);
        }

        self.update_stats_label();
        drop(game);
        self.persist_session_if_changed();
    }

    pub(super) fn update_tableau_selection_styles(&self, selected: Option<SelectedRun>) {
        let stacks = self.tableau_stacks();
        let card_pictures = self.imp().tableau_card_pictures.borrow();

        for (index, stack) in stacks.into_iter().enumerate() {
            stack.remove_css_class("tableau-selected-empty");
            for picture in &card_pictures[index] {
                picture.remove_css_class("tableau-selected-card");
            }

            if let Some(run) = selected {
                if run.col != index {
                    continue;
                }
                if card_pictures[index].is_empty() {
                    stack.add_css_class("tableau-selected-empty");
                    continue;
                }
                let start = run.start.min(card_pictures[index].len().saturating_sub(1));
                for picture in card_pictures[index].iter().skip(start) {
                    picture.add_css_class("tableau-selected-card");
                }
            }
        }
    }

    pub(super) fn update_waste_selection_style(&self, selected: bool) {
        let waste_slots = self.waste_fan_slots();
        for waste in &waste_slots {
            waste.remove_css_class("waste-selected-card");
        }

        if !selected {
            return;
        }

        let game = self.imp().game.borrow();
        let visible_waste_cards = usize::from(game.draw_mode().count().clamp(1, 5));
        let show_count = game.waste_len().min(visible_waste_cards);
        if show_count == 0 {
            return;
        }

        if let Some(top_slot) = waste_slots.get(show_count - 1) {
            top_slot.add_css_class("waste-selected-card");
        }
    }

    pub(super) fn render_card_images(&self, game: &KlondikeGame) {
        let imp = self.imp();

        if !imp.deck_load_attempted.get() {
            imp.deck_load_attempted.set(true);
            match AngloDeck::load() {
                Ok(deck) => {
                    *imp.deck.borrow_mut() = Some(deck);
                    *imp.deck_error.borrow_mut() = None;
                }
                Err(err) => {
                    *imp.deck_error.borrow_mut() = Some(err);
                }
            }
        }

        let deck_slot = imp.deck.borrow();
        let Some(deck) = deck_slot.as_ref() else {
            return;
        };

        self.update_tableau_metrics();
        let card_width = imp.card_width.get();
        let card_height = imp.card_height.get();
        let face_up_step = imp.face_up_step.get();
        let face_down_step = imp.face_down_step.get();
        let peek_active = imp.peek_active.get();

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
        let waste_fan_step = (card_width / 6).clamp(8, 22);
        let foundation_group_width = (card_width * 4) + (8 * 3);
        imp.stock_heading_box.set_width_request(card_width);
        imp.waste_heading_box
            .set_width_request(card_width + (waste_fan_step * 4));
        imp.foundations_heading_box
            .set_width_request(foundation_group_width);
        imp.waste_overlay
            .set_width_request(card_width + (waste_fan_step * 4));
        imp.waste_overlay.set_height_request(card_height);
        for picture in self.foundation_pictures() {
            picture.set_width_request(card_width);
            picture.set_height_request(card_height);
            picture.set_can_shrink(false);
        }

        if game.stock_len() > 0 {
            let back = deck.back_texture_scaled(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&back));
        } else {
            let empty = Self::blank_texture(card_width, card_height);
            imp.stock_picture.set_paintable(Some(&empty));
        }

        let waste_widgets = self.waste_fan_slots();
        let visible_waste_cards = usize::from(game.draw_mode().count().clamp(1, 5));
        let waste_cards = game.waste_top_n(visible_waste_cards);
        let show_count = waste_cards.len();

        for picture in waste_widgets.iter() {
            picture.set_visible(false);
            picture.set_margin_start(0);
            picture.set_paintable(None::<&gdk::Paintable>);
        }

        for (idx, card) in waste_cards.iter().copied().enumerate() {
            if let Some(picture) = waste_widgets.get(idx) {
                let texture = deck.texture_for_card_scaled(card, card_width, card_height);
                picture.set_paintable(Some(&texture));
                if idx > 0 {
                    picture.set_margin_start((idx as i32) * waste_fan_step);
                }
                picture.set_visible(true);
            }
        }
        imp.waste_placeholder_label.set_visible(show_count == 0);

        for (idx, picture) in self.foundation_pictures().into_iter().enumerate() {
            let top = game.foundations()[idx].last().copied();
            self.set_picture_from_card(&picture, top, deck, card_width, card_height);
        }
        imp.foundation_placeholder_1
            .set_visible(game.foundations()[0].is_empty());
        imp.foundation_placeholder_2
            .set_visible(game.foundations()[1].is_empty());
        imp.foundation_placeholder_3
            .set_visible(game.foundations()[2].is_empty());
        imp.foundation_placeholder_4
            .set_visible(game.foundations()[3].is_empty());

        let mut tableau_card_pictures = vec![Vec::new(); 7];

        for (idx, stack) in self.tableau_stacks().into_iter().enumerate() {
            while let Some(child) = stack.first_child() {
                stack.remove(&child);
            }

            stack.set_width_request(card_width);

            let column = &game.tableau()[idx];
            let mut y = 0;
            for (card_idx, card) in column.iter().enumerate() {
                let picture = gtk::Picture::new();
                picture.set_width_request(card_width);
                picture.set_height_request(card_height);
                picture.set_can_shrink(true);
                picture.set_content_fit(gtk::ContentFit::Contain);

                let show_face_up = if peek_active {
                    !card.face_up
                } else {
                    card.face_up
                };
                let texture = if show_face_up {
                    deck.texture_for_card(*card)
                } else {
                    deck.back_texture()
                };
                picture.set_paintable(Some(&texture));
                tableau_card_pictures[idx].push(picture.clone());

                stack.put(&picture, 0.0, f64::from(y));
                if card_idx + 1 < column.len() {
                    y += if card.face_up {
                        face_up_step
                    } else {
                        face_down_step
                    };
                }
            }

            let stack_height = (y + card_height).max(card_height);
            stack.set_height_request(stack_height);
        }

        *imp.tableau_card_pictures.borrow_mut() = tableau_card_pictures;
    }

    pub(super) fn set_picture_from_card(
        &self,
        picture: &gtk::Picture,
        card: Option<Card>,
        deck: &AngloDeck,
        width: i32,
        height: i32,
    ) {
        match card {
            Some(card) => {
                let texture = deck.texture_for_card_scaled(card, width, height);
                picture.set_paintable(Some(&texture));
            }
            None => picture.set_paintable(None::<&gdk::Paintable>),
        }
    }

    pub(super) fn blank_texture(width: i32, height: i32) -> gdk::Texture {
        let pixbuf = gdk_pixbuf::Pixbuf::new(
            gdk_pixbuf::Colorspace::Rgb,
            true,
            8,
            width.max(1),
            height.max(1),
        )
        .expect("failed to allocate blank pixbuf");
        pixbuf.fill(0x00000000);
        gdk::Texture::for_pixbuf(&pixbuf)
    }

    pub(super) fn foundation_pictures(&self) -> [gtk::Picture; 4] {
        let imp = self.imp();
        [
            imp.foundation_picture_1.get(),
            imp.foundation_picture_2.get(),
            imp.foundation_picture_3.get(),
            imp.foundation_picture_4.get(),
        ]
    }

    pub(super) fn foundation_placeholders(&self) -> [gtk::Label; 4] {
        let imp = self.imp();
        [
            imp.foundation_placeholder_1.get(),
            imp.foundation_placeholder_2.get(),
            imp.foundation_placeholder_3.get(),
            imp.foundation_placeholder_4.get(),
        ]
    }

    pub(super) fn waste_fan_slots(&self) -> [gtk::Picture; 5] {
        let imp = self.imp();
        [
            imp.waste_picture_1.get(),
            imp.waste_picture_2.get(),
            imp.waste_picture_3.get(),
            imp.waste_picture_4.get(),
            imp.waste_picture_5.get(),
        ]
    }

    pub(super) fn tableau_stacks(&self) -> [gtk::Fixed; 7] {
        let imp = self.imp();
        [
            imp.tableau_stack_1.get(),
            imp.tableau_stack_2.get(),
            imp.tableau_stack_3.get(),
            imp.tableau_stack_4.get(),
            imp.tableau_stack_5.get(),
            imp.tableau_stack_6.get(),
            imp.tableau_stack_7.get(),
        ]
    }
}

use super::*;
use crate::engine::boundary;
use crate::engine::render_plan;
use crate::engine::status_text;
use crate::engine::variant_engine::engine_for_mode;
use crate::game::SpiderGame;

impl CardthropicWindow {
    pub(super) fn render(&self) {
        if self.active_game_mode() == GameMode::Spider {
            self.render_spider();
            return;
        }

        let imp = self.imp();
        imp.foundations_heading_box.set_visible(true);
        imp.foundations_area_box.set_visible(true);
        let view = boundary::game_view_model(
            &imp.game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        );
        let game = view.klondike();
        let mode = view.mode();
        let engine_ready = view.engine_ready();
        let caps = engine_for_mode(mode).capabilities();
        if engine_ready {
            self.note_current_seed_win_if_needed();
            if game.is_won() && imp.timer_started.get() {
                imp.timer_started.set(false);
            }
        }

        imp.stock_label
            .set_label(&render_plan::card_count_label(game.stock_len()));

        imp.waste_label
            .set_label(&render_plan::card_count_label(game.waste_len()));

        let foundation_labels = [
            &imp.foundation_label_1,
            &imp.foundation_label_2,
            &imp.foundation_label_3,
            &imp.foundation_label_4,
        ];

        for label in foundation_labels {
            label.set_label("");
        }

        let selected_tuple = render_plan::sanitize_selected_run(
            game,
            (*imp.selected_run.borrow()).map(|run| (run.col, run.start)),
        );
        let selected = selected_tuple.map(|(col, start)| SelectedRun { col, start });
        *imp.selected_run.borrow_mut() = selected;
        if imp.waste_selected.get() && game.waste_top().is_none() {
            imp.waste_selected.set(false);
        }

        self.render_card_images(game);

        let controls =
            render_plan::plan_controls(caps, imp.history.borrow().len(), imp.future.borrow().len());
        imp.undo_button.set_sensitive(controls.undo_enabled);
        imp.redo_button.set_sensitive(controls.redo_enabled);
        imp.auto_hint_button
            .set_sensitive(controls.auto_hint_enabled);
        imp.cyclone_shuffle_button
            .set_sensitive(controls.cyclone_enabled);
        imp.peek_button.set_sensitive(controls.peek_enabled);
        imp.robot_button.set_sensitive(controls.robot_enabled);
        imp.seed_random_button
            .set_sensitive(controls.seed_random_enabled);
        imp.seed_rescue_button
            .set_sensitive(controls.seed_rescue_enabled);
        imp.seed_winnable_button
            .set_sensitive(controls.seed_winnable_enabled);
        imp.seed_repeat_button
            .set_sensitive(controls.seed_repeat_enabled);
        imp.seed_go_button.set_sensitive(controls.seed_go_enabled);
        imp.seed_combo.set_sensitive(controls.seed_combo_enabled);

        self.update_keyboard_focus_style();
        let selected_status = selected.map(|run| (run.col, run.start));
        let show_controls_hint = imp.pending_deal_instructions.replace(false);
        let status = status_text::build_status_text(
            game,
            selected_status,
            imp.waste_selected.get(),
            imp.peek_active.get(),
            engine_ready,
            show_controls_hint,
            mode.label(),
            self.smart_move_mode().as_setting(),
            imp.deck_error.borrow().as_deref(),
            imp.status_override.borrow().as_deref(),
        );
        if !status.is_empty() {
            self.append_status_line(&status);
        }

        self.update_stats_label();
        self.mark_session_dirty();
    }

    fn render_spider(&self) {
        let imp = self.imp();
        imp.foundations_heading_box.set_visible(false);
        imp.foundations_area_box.set_visible(false);
        let mode = self.active_game_mode();
        let caps = engine_for_mode(mode).capabilities();
        let spider = imp.game.borrow().spider().clone();
        if spider.is_won() && imp.timer_started.get() {
            imp.timer_started.set(false);
        }
        self.note_current_seed_win_if_needed();

        imp.stock_label
            .set_label(&render_plan::card_count_label(spider.stock_len()));
        imp.waste_label
            .set_label(&format!("{} runs", spider.completed_runs()));

        let selected = (*imp.selected_run.borrow()).and_then(|run| {
            let len = spider.tableau().get(run.col).map(Vec::len)?;
            if run.start >= len {
                return None;
            }
            spider
                .tableau_card(run.col, run.start)
                .filter(|card| card.face_up)
                .map(|_| run)
        });
        *imp.selected_run.borrow_mut() = selected;
        imp.waste_selected.set(false);

        self.render_card_images_spider(&spider);

        let controls =
            render_plan::plan_controls(caps, imp.history.borrow().len(), imp.future.borrow().len());
        imp.undo_button.set_sensitive(controls.undo_enabled);
        imp.redo_button.set_sensitive(controls.redo_enabled);
        imp.auto_hint_button
            .set_sensitive(controls.auto_hint_enabled);
        imp.cyclone_shuffle_button
            .set_sensitive(controls.cyclone_enabled);
        imp.peek_button.set_sensitive(controls.peek_enabled);
        imp.robot_button.set_sensitive(controls.robot_enabled);
        imp.seed_random_button
            .set_sensitive(controls.seed_random_enabled);
        imp.seed_rescue_button
            .set_sensitive(controls.seed_rescue_enabled);
        imp.seed_winnable_button
            .set_sensitive(controls.seed_winnable_enabled);
        imp.seed_repeat_button
            .set_sensitive(controls.seed_repeat_enabled);
        imp.seed_go_button.set_sensitive(controls.seed_go_enabled);
        imp.seed_combo.set_sensitive(controls.seed_combo_enabled);

        self.update_keyboard_focus_style();
        let show_controls_hint = imp.pending_deal_instructions.replace(false);
        let status = if let Some(err) = imp.deck_error.borrow().as_deref() {
            format!("Card deck load failed: {err}")
        } else if let Some(message) = imp.status_override.borrow().as_deref() {
            message.to_string()
        } else if spider.is_won() {
            "Spider won! All runs are complete.".to_string()
        } else if let Some(run) = selected {
            let amount = spider
                .tableau()
                .get(run.col)
                .map(Vec::len)
                .unwrap_or(0)
                .saturating_sub(run.start);
            if amount > 1 {
                format!(
                    "Selected {amount} cards from T{}. Click another tableau to move this run.",
                    run.col + 1
                )
            } else {
                format!(
                    "Selected tableau T{}. Click another tableau to move top card.",
                    run.col + 1
                )
            }
        } else if show_controls_hint {
            "Spider ready. Build suited descending runs from King to Ace.".to_string()
        } else {
            String::new()
        };
        if !status.is_empty() {
            self.append_status_line(&status);
        }

        self.update_stats_label();
        self.mark_session_dirty();
    }

    pub(super) fn flash_smart_move_fail_tableau_run(&self, col: usize, start: usize) {
        let imp = self.imp();
        let previous_selected = *imp.selected_run.borrow();
        let previous_waste_selected = imp.waste_selected.get();

        *imp.selected_run.borrow_mut() = Some(SelectedRun { col, start });
        imp.waste_selected.set(false);
        self.render();

        glib::timeout_add_local_once(
            Duration::from_millis(100),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move || {
                    let imp = window.imp();
                    let current = *imp.selected_run.borrow();
                    if current == Some(SelectedRun { col, start }) {
                        *imp.selected_run.borrow_mut() = previous_selected;
                        imp.waste_selected.set(previous_waste_selected);
                        window.render();
                    }
                }
            ),
        );
    }

    pub(super) fn flash_smart_move_fail_waste_top(&self) {
        let imp = self.imp();
        let game = imp.game.borrow();
        let show_count = render_plan::waste_visible_count(game.draw_mode(), game.waste_len());
        if show_count == 0 {
            return;
        }
        drop(game);

        let previous_selected = *imp.selected_run.borrow();
        let previous_waste_selected = imp.waste_selected.get();

        *imp.selected_run.borrow_mut() = None;
        imp.waste_selected.set(true);
        self.render();

        glib::timeout_add_local_once(
            Duration::from_millis(100),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move || {
                    let imp = window.imp();
                    if imp.waste_selected.get() {
                        *imp.selected_run.borrow_mut() = previous_selected;
                        imp.waste_selected.set(previous_waste_selected);
                        window.render();
                    }
                }
            ),
        );
    }

    pub(super) fn render_card_images(&self, game: &KlondikeGame) {
        let imp = self.imp();

        if !imp.deck_load_attempted.get() {
            imp.deck_load_attempted.set(true);
            let loaded = if let Some(settings) = Self::load_app_settings() {
                let custom_svg = settings.string(SETTINGS_KEY_CUSTOM_CARD_SVG).to_string();
                if custom_svg.trim().is_empty() {
                    AngloDeck::load()
                } else {
                    AngloDeck::load_with_custom_normal_svg(&custom_svg)
                }
            } else {
                AngloDeck::load()
            };

            match loaded {
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

        self.configure_stock_waste_foundation_widgets(card_width, card_height);
        self.render_stock_picture(game, deck, card_width, card_height);
        self.render_waste_fan(game, deck, card_width, card_height);
        self.render_foundations_area(game, deck, card_width, card_height);
        self.render_tableau_columns(
            game,
            deck,
            card_width,
            card_height,
            face_up_step,
            face_down_step,
            peek_active,
        );
    }

    fn render_card_images_spider(&self, game: &SpiderGame) {
        let imp = self.imp();

        if !imp.deck_load_attempted.get() {
            imp.deck_load_attempted.set(true);
            let loaded = if let Some(settings) = Self::load_app_settings() {
                let custom_svg = settings.string(SETTINGS_KEY_CUSTOM_CARD_SVG).to_string();
                if custom_svg.trim().is_empty() {
                    AngloDeck::load()
                } else {
                    AngloDeck::load_with_custom_normal_svg(&custom_svg)
                }
            } else {
                AngloDeck::load()
            };

            match loaded {
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

        self.configure_stock_waste_foundation_widgets(card_width, card_height);
        self.render_stock_picture_spider(game, deck, card_width, card_height);
        self.render_waste_fan_spider();
        self.render_foundations_area_spider(game, deck, card_width, card_height);
        self.render_tableau_columns_spider(
            game,
            deck,
            card_width,
            card_height,
            face_up_step,
            face_down_step,
            peek_active,
        );
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

    pub(super) fn tableau_stacks(&self) -> [gtk::Fixed; 10] {
        let imp = self.imp();
        [
            imp.tableau_stack_1.get(),
            imp.tableau_stack_2.get(),
            imp.tableau_stack_3.get(),
            imp.tableau_stack_4.get(),
            imp.tableau_stack_5.get(),
            imp.tableau_stack_6.get(),
            imp.tableau_stack_7.get(),
            imp.tableau_stack_8.get(),
            imp.tableau_stack_9.get(),
            imp.tableau_stack_10.get(),
        ]
    }

    pub(super) fn invalidate_card_render_cache(&self) {
        for col in self.imp().tableau_picture_state_cache.borrow_mut().iter_mut() {
            col.clear();
        }
    }

    fn append_status_line(&self, status: &str) {
        const MAX_STATUS_LINES: usize = 240;

        let imp = self.imp();
        if imp.status_last_appended.borrow().as_str() == status {
            return;
        }

        *imp.status_last_appended.borrow_mut() = status.to_string();
        let mut history = imp.status_history.borrow_mut();
        history.push_back(status.to_string());
        while history.len() > MAX_STATUS_LINES {
            let _ = history.pop_front();
        }
        let history_joined = if imp.status_history_buffer.borrow().is_some() {
            Some(
                history
                    .iter()
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join("\n"),
            )
        } else {
            None
        };
        drop(history);
        imp.status_label.set_label(status);
        imp.status_label.set_tooltip_text(Some(status));
        if let Some(joined) = history_joined {
            if let Some(buffer) = imp.status_history_buffer.borrow().as_ref() {
                buffer.set_text(&joined);
            }
        }
    }

    pub(super) fn show_status_history_dialog(&self) {
        if let Some(existing) = self.imp().status_history_dialog.borrow().as_ref() {
            existing.present();
            return;
        }

        let joined = self
            .imp()
            .status_history
            .borrow()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join("\n");

        let dialog = gtk::Window::builder()
            .title("Status History")
            .transient_for(self)
            .modal(false)
            .default_width(760)
            .default_height(420)
            .build();
        dialog.set_hide_on_close(true);
        dialog.set_destroy_with_parent(true);

        let root = gtk::Box::new(gtk::Orientation::Vertical, 8);
        root.set_margin_top(10);
        root.set_margin_bottom(10);
        root.set_margin_start(10);
        root.set_margin_end(10);

        let scroller = gtk::ScrolledWindow::new();
        scroller.set_hexpand(true);
        scroller.set_vexpand(true);
        scroller.set_policy(gtk::PolicyType::Automatic, gtk::PolicyType::Automatic);

        let text = gtk::TextView::new();
        text.set_editable(false);
        text.set_cursor_visible(false);
        text.set_monospace(true);
        text.set_wrap_mode(gtk::WrapMode::WordChar);
        text.buffer().set_text(&joined);
        *self.imp().status_history_buffer.borrow_mut() = Some(text.buffer());
        scroller.set_child(Some(&text));
        root.append(&scroller);

        let actions = gtk::Box::new(gtk::Orientation::Horizontal, 6);
        actions.set_halign(gtk::Align::End);

        let clear = gtk::Button::with_label("Clear");
        clear.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[weak]
            text,
            move |_| {
                let imp = window.imp();
                imp.status_history.borrow_mut().clear();
                imp.status_last_appended.borrow_mut().clear();
                imp.status_label.set_label("");
                imp.status_label.set_tooltip_text(None);
                let buffer = imp.status_history_buffer.borrow().as_ref().cloned();
                if let Some(buffer) = buffer {
                    buffer.set_text("");
                } else {
                    text.buffer().set_text("");
                }
            }
        ));
        actions.append(&clear);

        let close = gtk::Button::with_label("Close");
        close.connect_clicked(glib::clone!(
            #[weak]
            dialog,
            move |_| {
                dialog.close();
            }
        ));
        actions.append(&close);
        root.append(&actions);

        dialog.set_child(Some(&root));
        *self.imp().status_history_dialog.borrow_mut() = Some(dialog.clone());
        dialog.present();
    }
}

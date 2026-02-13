use super::*;
use crate::engine::boundary;
use crate::engine::render_plan;
use crate::engine::status_text;
use crate::engine::variant_engine::engine_for_mode;

impl CardthropicWindow {
    pub(super) fn render(&self) {
        let imp = self.imp();
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
            self.note_current_seed_win_if_needed(game);
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
        let status = status_text::build_status_text(
            game,
            selected_status,
            imp.waste_selected.get(),
            imp.peek_active.get(),
            engine_ready,
            mode.label(),
            self.smart_move_mode().as_setting(),
            imp.deck_error.borrow().as_deref(),
            imp.status_override.borrow().as_deref(),
        );
        imp.status_label.set_label(&status);

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

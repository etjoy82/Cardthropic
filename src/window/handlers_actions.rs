use super::*;
use crate::engine::boundary;

impl CardthropicWindow {
    pub(super) fn copy_game_state_to_clipboard(&self) {
        let payload = self.build_saved_session();
        self.clipboard().set_text(&payload);
        *self.imp().status_override.borrow_mut() =
            Some("Copied game state to clipboard.".to_string());
        self.render();
    }

    pub(super) fn paste_game_state_from_clipboard(&self) {
        let clipboard = self.clipboard();
        clipboard.read_text_async(
            None::<&gio::Cancellable>,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |result| match result {
                    Ok(Some(text)) => {
                        let payload = text.to_string();
                        match window.restore_session_from_payload(
                            &payload,
                            "Restored game from clipboard.",
                            true,
                        ) {
                            Ok(()) => {
                                window.render();
                            }
                            Err(err) => {
                                *window.imp().status_override.borrow_mut() =
                                    Some(format!("Paste failed: {err}."));
                                window.render();
                            }
                        }
                    }
                    Ok(None) => {
                        *window.imp().status_override.borrow_mut() =
                            Some("Paste failed: clipboard is empty.".to_string());
                        window.render();
                    }
                    Err(err) => {
                        *window.imp().status_override.borrow_mut() =
                            Some(format!("Paste failed: {err}."));
                        window.render();
                    }
                }
            ),
        );
    }

    pub(super) fn setup_primary_action_handlers(&self) {
        let imp = self.imp();

        imp.undo_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.undo();
            }
        ));
        imp.redo_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.redo();
            }
        ));

        imp.auto_hint_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.play_hint_for_player();
            }
        ));
        let wand_middle_click = gtk::GestureClick::new();
        wand_middle_click.set_button(gdk::BUTTON_MIDDLE);
        wand_middle_click.connect_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _, _| {
                window.trigger_rapid_wand();
            }
        ));
        imp.auto_hint_button.add_controller(wand_middle_click);

        imp.cyclone_shuffle_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.cyclone_shuffle_tableau();
            }
        ));
        imp.peek_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.trigger_peek();
            }
        ));
        imp.robot_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.toggle_robot_mode();
            }
        ));
        let robot_middle_click = gtk::GestureClick::new();
        robot_middle_click.set_button(gdk::BUTTON_MIDDLE);
        robot_middle_click.connect_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _, _| {
                window.start_robot_mode_forever();
            }
        ));
        imp.robot_button.add_controller(robot_middle_click);
        imp.status_history_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.show_status_history_dialog();
            }
        ));
    }

    pub(super) fn setup_robot_stop_capture_handler(&self) {
        let robot_stop_click = gtk::GestureClick::new();
        robot_stop_click.set_button(0);
        robot_stop_click.set_propagation_phase(gtk::PropagationPhase::Capture);
        robot_stop_click.connect_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, x, y| {
                if !window.imp().robot_mode_running.get() {
                    return;
                }
                let imp = window.imp();
                let robot_button = imp.robot_button.get();
                if let Some(picked) = window.pick(x, y, gtk::PickFlags::DEFAULT) {
                    let robot_widget: gtk::Widget = robot_button.clone().upcast();
                    if picked == robot_widget || picked.is_ancestor(&robot_button) {
                        return;
                    }
                    let in_cards_zone = picked.is_ancestor(&imp.stock_picture.get())
                        || picked.is_ancestor(&imp.waste_overlay.get())
                        || picked.is_ancestor(&imp.foundation_picture_1.get())
                        || picked.is_ancestor(&imp.foundation_picture_2.get())
                        || picked.is_ancestor(&imp.foundation_picture_3.get())
                        || picked.is_ancestor(&imp.foundation_picture_4.get())
                        || picked.is_ancestor(&imp.tableau_scroller.get())
                        || picked.is_ancestor(&imp.tableau_row.get());
                    if !in_cards_zone {
                        return;
                    }
                } else {
                    return;
                }
                window.stop_robot_mode();
            }
        ));
        self.add_controller(robot_stop_click);
    }

    pub(super) fn setup_keyboard_navigation_handler(&self) {
        let keyboard_nav = gtk::EventControllerKey::new();
        keyboard_nav.set_propagation_phase(gtk::PropagationPhase::Capture);
        keyboard_nav.connect_key_pressed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            #[upgrade_or]
            glib::Propagation::Proceed,
            move |_, key, _, state| {
                if window.handle_variant_shortcut_key(key, state) {
                    return glib::Propagation::Stop;
                }
                let arrow_key = matches!(
                    key,
                    gdk::Key::Left | gdk::Key::Right | gdk::Key::Up | gdk::Key::Down
                );
                if key == gdk::Key::space {
                    if let Some(entry) = window.seed_text_entry() {
                        if entry.has_focus() {
                            return glib::Propagation::Proceed;
                        }
                    }
                }
                if arrow_key {
                    // Arrow navigation should move board focus, not stay trapped on a focused button.
                    window.grab_focus();
                }
                if state.intersects(
                    gdk::ModifierType::ALT_MASK
                        | gdk::ModifierType::CONTROL_MASK
                        | gdk::ModifierType::SUPER_MASK
                        | gdk::ModifierType::META_MASK,
                ) {
                    return glib::Propagation::Proceed;
                }
                if window.handle_keyboard_navigation_key(key) {
                    glib::Propagation::Stop
                } else {
                    glib::Propagation::Proceed
                }
            }
        ));
        self.add_controller(keyboard_nav);
    }

    #[allow(deprecated)]
    pub(super) fn setup_seed_handlers(&self) {
        let imp = self.imp();
        imp.seed_combo.connect_changed(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |combo| {
                if window.imp().seed_combo_updating.get() {
                    return;
                }
                if let Some(seed) = combo.active_id() {
                    window.set_seed_input_text(seed.as_str());
                    window.start_new_game_from_seed_controls();
                    return;
                }
                window.clear_seed_entry_feedback();
                window.cancel_seed_winnable_check(None);
            }
        ));

        if let Some(seed_entry) = self.seed_text_entry() {
            seed_entry.set_placeholder_text(Some("Blank=random, or enter seed number/word"));
            seed_entry.set_width_chars(1);
            seed_entry.connect_changed(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.clear_seed_entry_feedback();
                    window.cancel_seed_winnable_check(None);
                }
            ));
            seed_entry.connect_activate(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_| {
                    window.start_new_game_from_seed_controls();
                }
            ));
        }

        imp.seed_random_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.start_random_seed_game();
            }
        ));
        imp.seed_rescue_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.start_random_winnable_seed_game();
            }
        ));
        imp.seed_winnable_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.toggle_seed_winnable_check();
            }
        ));
        imp.seed_repeat_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.repeat_current_seed_game();
            }
        ));
        imp.seed_go_button.connect_clicked(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_| {
                window.start_new_game_from_seed_controls();
            }
        ));
    }

    pub(super) fn setup_board_click_handlers(&self) {
        let imp = self.imp();

        let stock_click = gtk::GestureClick::new();
        stock_click.connect_released(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |_, _, _, _| {
                window.draw_card();
            }
        ));
        imp.stock_picture.add_controller(stock_click);

        let waste_click = gtk::GestureClick::new();
        waste_click.set_button(0);
        waste_click.connect_released(glib::clone!(
            #[weak(rename_to = window)]
            self,
            move |gesture, n_press, x, _| {
                let current_button = gesture.current_button();
                if window.active_game_mode() == GameMode::Freecell {
                    if window.smart_move_mode() == SmartMoveMode::RightClick
                        && current_button == gdk::BUTTON_SECONDARY
                    {
                        let idx = window.freecell_slot_index_from_waste_x(x);
                        let _ = window.try_smart_move_from_freecell(idx);
                        return;
                    }
                    window.handle_freecell_click_x(n_press, Some(x));
                } else {
                    if window.smart_move_mode() == SmartMoveMode::RightClick
                        && current_button == gdk::BUTTON_SECONDARY
                    {
                        let _ = window.try_smart_move_from_waste();
                        return;
                    }
                    window.handle_waste_click(n_press);
                }
            }
        ));
        imp.waste_overlay.add_controller(waste_click);

        for (index, stack) in self.tableau_stacks().into_iter().enumerate() {
            let click = gtk::GestureClick::new();
            click.set_button(0);
            click.connect_released(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |gesture, n_press, _, y| {
                    let current_button = gesture.current_button();
                    if window.active_game_mode() == GameMode::Spider {
                        let game = window.imp().game.borrow().spider().clone();
                        let start = window.tableau_run_start_from_y_spider(&game, index, y);
                        match window.smart_move_mode() {
                            SmartMoveMode::DoubleClick if n_press == 2 => {
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::SingleClick if n_press == 1 => {
                                *window.imp().selected_run.borrow_mut() = None;
                                window.imp().waste_selected.set(false);
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::RightClick
                                if current_button == gdk::BUTTON_SECONDARY =>
                            {
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::Disabled | SmartMoveMode::DoubleClick
                                if n_press == 1 =>
                            {
                                window.select_or_move_tableau_with_start(index, start);
                            }
                            _ => {}
                        }
                        return;
                    }
                    if window.active_game_mode() == GameMode::Freecell {
                        let game = window.imp().game.borrow().freecell().clone();
                        let start = window.tableau_run_start_from_y_freecell(&game, index, y);
                        match window.smart_move_mode() {
                            SmartMoveMode::DoubleClick if n_press == 2 => {
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::SingleClick if n_press == 1 => {
                                *window.imp().selected_run.borrow_mut() = None;
                                window.imp().selected_freecell.set(None);
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            SmartMoveMode::RightClick
                                if current_button == gdk::BUTTON_SECONDARY =>
                            {
                                if let Some(start) = start {
                                    window.try_smart_move_from_tableau(index, start);
                                }
                            }
                            _ if n_press == 1 => {
                                window.select_or_move_tableau_with_start(index, start);
                            }
                            _ => {}
                        }
                        return;
                    }

                    match window.smart_move_mode() {
                        SmartMoveMode::DoubleClick if n_press == 2 => {
                            let start = boundary::clone_klondike_for_automation(
                                &window.imp().game.borrow(),
                                window.active_game_mode(),
                                window.current_klondike_draw_mode(),
                            )
                            .and_then(|game| window.tableau_run_start_from_y(&game, index, y));
                            if let Some(start) = start {
                                window.try_smart_move_from_tableau(index, start);
                            }
                        }
                        SmartMoveMode::SingleClick if n_press == 1 => {
                            *window.imp().selected_run.borrow_mut() = None;
                            window.imp().waste_selected.set(false);
                            let start = boundary::clone_klondike_for_automation(
                                &window.imp().game.borrow(),
                                window.active_game_mode(),
                                window.current_klondike_draw_mode(),
                            )
                            .and_then(|game| window.tableau_run_start_from_y(&game, index, y));
                            if let Some(start) = start {
                                window.try_smart_move_from_tableau(index, start);
                            }
                        }
                        SmartMoveMode::RightClick if current_button == gdk::BUTTON_SECONDARY => {
                            let start = boundary::clone_klondike_for_automation(
                                &window.imp().game.borrow(),
                                window.active_game_mode(),
                                window.current_klondike_draw_mode(),
                            )
                            .and_then(|game| window.tableau_run_start_from_y(&game, index, y));
                            if let Some(start) = start {
                                window.try_smart_move_from_tableau(index, start);
                            }
                        }
                        SmartMoveMode::Disabled | SmartMoveMode::DoubleClick if n_press == 1 => {
                            let start = boundary::clone_klondike_for_automation(
                                &window.imp().game.borrow(),
                                window.active_game_mode(),
                                window.current_klondike_draw_mode(),
                            )
                            .and_then(|game| window.tableau_run_start_from_y(&game, index, y));
                            window.select_or_move_tableau_with_start(index, start);
                        }
                        _ => {}
                    }
                }
            ));
            stack.add_controller(click);
        }

        for (index, foundation) in self.foundation_pictures().into_iter().enumerate() {
            let click = gtk::GestureClick::new();
            click.connect_released(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _, _, _| {
                    window.handle_click_on_foundation(index);
                }
            ));
            foundation.add_controller(click);
        }
        for (index, foundation) in self.foundation_placeholders().into_iter().enumerate() {
            let click = gtk::GestureClick::new();
            click.connect_released(glib::clone!(
                #[weak(rename_to = window)]
                self,
                move |_, _, _, _| {
                    window.handle_click_on_foundation(index);
                }
            ));
            foundation.add_controller(click);
        }
    }
}

use super::*;
use crate::engine::boundary;

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
            move |source, _| {
                let imp = window.imp();
                let Some(game) = boundary::clone_klondike_for_automation(
                    &imp.game.borrow(),
                    window.active_game_mode(),
                    window.current_klondike_draw_mode(),
                ) else {
                    return;
                };
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
                    let Some(game) = boundary::clone_klondike_for_automation(
                        &window.imp().game.borrow(),
                        window.active_game_mode(),
                        window.current_klondike_draw_mode(),
                    ) else {
                        drag_start.set(None);
                        return None;
                    };
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
                    let Some(game) = boundary::clone_klondike_for_automation(
                        &imp.game.borrow(),
                        window.active_game_mode(),
                        window.current_klondike_draw_mode(),
                    ) else {
                        return;
                    };
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
}

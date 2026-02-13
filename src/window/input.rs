use super::*;
use crate::engine::boundary;
use crate::engine::keyboard_nav;

impl CardthropicWindow {
    pub(super) fn handle_keyboard_navigation_key(&self, key: gdk::Key) -> bool {
        match key {
            gdk::Key::Left => {
                self.move_keyboard_focus_horizontal(-1);
                true
            }
            gdk::Key::Right => {
                self.move_keyboard_focus_horizontal(1);
                true
            }
            gdk::Key::Up => {
                self.move_keyboard_focus_vertical(-1);
                true
            }
            gdk::Key::Down => {
                self.move_keyboard_focus_vertical(1);
                true
            }
            gdk::Key::Return | gdk::Key::KP_Enter => {
                self.activate_keyboard_target();
                true
            }
            _ => false,
        }
    }

    pub(super) fn activate_keyboard_target(&self) {
        match self.imp().keyboard_target.get() {
            KeyboardTarget::Stock => {
                if self.is_mode_engine_ready() {
                    self.draw_card();
                }
            }
            KeyboardTarget::Waste => {
                if self.is_mode_engine_ready() {
                    self.handle_waste_click(1);
                }
            }
            KeyboardTarget::Foundation(idx) => {
                if self.is_mode_engine_ready() {
                    self.handle_click_on_foundation(idx);
                }
            }
            KeyboardTarget::Tableau { col, start } => {
                if self.is_mode_engine_ready() {
                    self.select_or_move_tableau_with_start(col, start);
                }
            }
        }
    }

    fn move_keyboard_focus_horizontal(&self, delta: i32) {
        let Some(game) = boundary::clone_klondike_for_automation(
            &self.imp().game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        ) else {
            return;
        };
        let next = keyboard_nav::move_horizontal(&game, self.imp().keyboard_target.get(), delta);
        self.imp().keyboard_target.set(next);
        self.update_keyboard_focus_style();
    }

    fn move_keyboard_focus_vertical(&self, delta: i32) {
        let Some(game) = boundary::clone_klondike_for_automation(
            &self.imp().game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        ) else {
            return;
        };
        let next = keyboard_nav::move_vertical(&game, self.imp().keyboard_target.get(), delta);
        self.imp().keyboard_target.set(next);
        self.update_keyboard_focus_style();
    }

    pub(super) fn update_keyboard_focus_style(&self) {
        let imp = self.imp();
        imp.stock_picture.remove_css_class("keyboard-focus-card");
        imp.waste_picture.remove_css_class("keyboard-focus-card");
        for waste in self.waste_fan_slots() {
            waste.remove_css_class("keyboard-focus-card");
        }
        for picture in self.foundation_pictures() {
            picture.remove_css_class("keyboard-focus-card");
        }
        for stack in self.tableau_stacks() {
            stack.remove_css_class("keyboard-focus-empty");
        }
        for col in imp.tableau_card_pictures.borrow().iter() {
            for picture in col {
                picture.remove_css_class("keyboard-focus-card");
            }
        }

        let Some(game) = boundary::clone_klondike_for_automation(
            &imp.game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        ) else {
            return;
        };
        let target = keyboard_nav::normalize_target(&game, imp.keyboard_target.get());
        imp.keyboard_target.set(target);
        match target {
            KeyboardTarget::Stock => imp.stock_picture.add_css_class("keyboard-focus-card"),
            KeyboardTarget::Waste => {
                let visible_waste_cards = usize::from(game.draw_mode().count().clamp(1, 5));
                let show_count = game.waste_len().min(visible_waste_cards);
                if show_count == 0 {
                    imp.waste_picture.add_css_class("keyboard-focus-card");
                } else if let Some(slot) = self.waste_fan_slots().get(show_count - 1) {
                    slot.add_css_class("keyboard-focus-card");
                }
            }
            KeyboardTarget::Foundation(idx) => {
                if let Some(picture) = self.foundation_pictures().get(idx) {
                    picture.add_css_class("keyboard-focus-card");
                }
            }
            KeyboardTarget::Tableau { col, start } => {
                if let Some(start) = start {
                    if let Some(picture) = imp
                        .tableau_card_pictures
                        .borrow()
                        .get(col)
                        .and_then(|cards| cards.get(start))
                    {
                        picture.add_css_class("keyboard-focus-card");
                    }
                } else if let Some(stack) = self.tableau_stacks().get(col) {
                    stack.add_css_class("keyboard-focus-empty");
                }
            }
        }
    }
}

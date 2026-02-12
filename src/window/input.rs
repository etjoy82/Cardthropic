use super::*;

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
        let game = self.imp().game.borrow();
        let current = self.normalize_keyboard_target(&game, self.imp().keyboard_target.get());
        let next = match current {
            KeyboardTarget::Stock => {
                if delta > 0 {
                    KeyboardTarget::Waste
                } else {
                    KeyboardTarget::Stock
                }
            }
            KeyboardTarget::Waste => {
                if delta > 0 {
                    KeyboardTarget::Foundation(0)
                } else {
                    KeyboardTarget::Stock
                }
            }
            KeyboardTarget::Foundation(idx) => {
                let idx = (idx as i32 + delta).clamp(0, 3) as usize;
                if idx == 0 && delta < 0 {
                    KeyboardTarget::Waste
                } else {
                    KeyboardTarget::Foundation(idx)
                }
            }
            KeyboardTarget::Tableau { col, start } => {
                let new_col = (col as i32 + delta).clamp(0, 6) as usize;
                let offset = self.keyboard_tableau_offset_from_top(&game, col, start);
                self.tableau_target_for_column(&game, new_col, Some(offset))
            }
        };
        drop(game);
        self.imp().keyboard_target.set(next);
        self.update_keyboard_focus_style();
    }

    fn move_keyboard_focus_vertical(&self, delta: i32) {
        let game = self.imp().game.borrow();
        let current = self.normalize_keyboard_target(&game, self.imp().keyboard_target.get());
        let next = match current {
            KeyboardTarget::Stock | KeyboardTarget::Waste | KeyboardTarget::Foundation(_) => {
                if delta > 0 {
                    let col = match current {
                        KeyboardTarget::Stock => 0,
                        KeyboardTarget::Waste => 1,
                        KeyboardTarget::Foundation(idx) => [3_usize, 4, 5, 6][idx],
                        _ => 0,
                    };
                    self.tableau_target_for_column(&game, col, Some(0))
                } else {
                    current
                }
            }
            KeyboardTarget::Tableau { col, start } => {
                let faceups = self.tableau_face_up_indices(&game, col);
                if delta < 0 {
                    if let Some(curr) = start {
                        if let Some(pos) = faceups.iter().position(|&idx| idx == curr) {
                            if pos + 1 < faceups.len() {
                                KeyboardTarget::Tableau {
                                    col,
                                    start: Some(faceups[pos + 1]),
                                }
                            } else {
                                let top_idx = match col {
                                    0 => 0,
                                    1 => 1,
                                    2 | 3 => 2,
                                    4 => 3,
                                    5 => 4,
                                    _ => 5,
                                };
                                match top_idx {
                                    0 => KeyboardTarget::Stock,
                                    1 => KeyboardTarget::Waste,
                                    _ => KeyboardTarget::Foundation(top_idx - 2),
                                }
                            }
                        } else {
                            self.tableau_target_for_column(&game, col, Some(0))
                        }
                    } else {
                        let top_idx = match col {
                            0 => 0,
                            1 => 1,
                            2 | 3 => 2,
                            4 => 3,
                            5 => 4,
                            _ => 5,
                        };
                        match top_idx {
                            0 => KeyboardTarget::Stock,
                            1 => KeyboardTarget::Waste,
                            _ => KeyboardTarget::Foundation(top_idx - 2),
                        }
                    }
                } else if let Some(curr) = start {
                    if let Some(pos) = faceups.iter().position(|&idx| idx == curr) {
                        if pos > 0 {
                            KeyboardTarget::Tableau {
                                col,
                                start: Some(faceups[pos - 1]),
                            }
                        } else {
                            KeyboardTarget::Tableau { col, start }
                        }
                    } else {
                        self.tableau_target_for_column(&game, col, Some(0))
                    }
                } else {
                    KeyboardTarget::Tableau { col, start: None }
                }
            }
        };
        drop(game);
        self.imp().keyboard_target.set(next);
        self.update_keyboard_focus_style();
    }

    fn tableau_face_up_indices(&self, game: &KlondikeGame, col: usize) -> Vec<usize> {
        game.tableau()
            .get(col)
            .map(|pile| {
                pile.iter()
                    .enumerate()
                    .filter_map(|(idx, card)| if card.face_up { Some(idx) } else { None })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    }

    fn keyboard_tableau_offset_from_top(
        &self,
        game: &KlondikeGame,
        col: usize,
        start: Option<usize>,
    ) -> usize {
        let faceups = self.tableau_face_up_indices(game, col);
        if faceups.is_empty() {
            return 0;
        }
        let Some(start) = start else {
            return 0;
        };
        faceups
            .iter()
            .position(|&idx| idx == start)
            .map(|pos| faceups.len().saturating_sub(pos + 1))
            .unwrap_or(0)
    }

    fn tableau_target_for_column(
        &self,
        game: &KlondikeGame,
        col: usize,
        prefer_offset_from_top: Option<usize>,
    ) -> KeyboardTarget {
        let faceups = self.tableau_face_up_indices(game, col);
        if faceups.is_empty() {
            return KeyboardTarget::Tableau { col, start: None };
        }
        let offset = prefer_offset_from_top.unwrap_or(0).min(faceups.len() - 1);
        let pos = faceups.len() - 1 - offset;
        KeyboardTarget::Tableau {
            col,
            start: Some(faceups[pos]),
        }
    }

    fn normalize_keyboard_target(
        &self,
        game: &KlondikeGame,
        target: KeyboardTarget,
    ) -> KeyboardTarget {
        match target {
            KeyboardTarget::Stock => KeyboardTarget::Stock,
            KeyboardTarget::Waste => KeyboardTarget::Waste,
            KeyboardTarget::Foundation(idx) => KeyboardTarget::Foundation(idx.min(3)),
            KeyboardTarget::Tableau { col, start } => {
                let col = col.min(6);
                let faceups = self.tableau_face_up_indices(game, col);
                if faceups.is_empty() {
                    KeyboardTarget::Tableau { col, start: None }
                } else if let Some(start) = start {
                    if faceups.contains(&start) {
                        KeyboardTarget::Tableau {
                            col,
                            start: Some(start),
                        }
                    } else {
                        self.tableau_target_for_column(game, col, Some(0))
                    }
                } else {
                    KeyboardTarget::Tableau { col, start: None }
                }
            }
        }
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

        let game = imp.game.borrow();
        let target = self.normalize_keyboard_target(&game, imp.keyboard_target.get());
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

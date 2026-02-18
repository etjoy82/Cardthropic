use super::*;
use crate::engine::boundary;
use crate::engine::keyboard_nav;
use crate::game::SpiderGame;

impl CardthropicWindow {
    pub(super) fn handle_variant_shortcut_key(
        &self,
        key: gdk::Key,
        state: gdk::ModifierType,
    ) -> bool {
        let ctrl = state.contains(gdk::ModifierType::CONTROL_MASK);
        let shift = state.contains(gdk::ModifierType::SHIFT_MASK);
        let alt_like = state.intersects(
            gdk::ModifierType::ALT_MASK
                | gdk::ModifierType::SUPER_MASK
                | gdk::ModifierType::META_MASK,
        );
        if alt_like {
            return false;
        }

        let digit = match key {
            gdk::Key::_1 | gdk::Key::KP_1 | gdk::Key::exclam => Some(1),
            gdk::Key::_2 | gdk::Key::KP_2 | gdk::Key::at => Some(2),
            gdk::Key::_3 | gdk::Key::KP_3 | gdk::Key::numbersign => Some(3),
            gdk::Key::_4 | gdk::Key::KP_4 | gdk::Key::dollar => Some(4),
            gdk::Key::_5 | gdk::Key::KP_5 | gdk::Key::percent => Some(5),
            _ => None,
        };
        let Some(digit) = digit else {
            return false;
        };

        match (ctrl, shift, digit) {
            (false, true, 1) => self.select_klondike_draw_mode(DrawMode::One),
            (false, true, 2) => self.select_klondike_draw_mode(DrawMode::Two),
            (false, true, 3) => self.select_klondike_draw_mode(DrawMode::Three),
            (false, true, 4) => self.select_klondike_draw_mode(DrawMode::Four),
            (false, true, 5) => self.select_klondike_draw_mode(DrawMode::Five),
            (true, false, 1) => self.select_spider_suit_mode(SpiderSuitMode::One),
            (true, false, 2) => self.select_spider_suit_mode(SpiderSuitMode::Two),
            (true, false, 3) => self.select_spider_suit_mode(SpiderSuitMode::Three),
            (true, false, 4) => self.select_spider_suit_mode(SpiderSuitMode::Four),
            (true, true, 1) => {
                self.select_freecell_card_count_mode(FreecellCardCountMode::TwentySix)
            }
            (true, true, 2) => {
                self.select_freecell_card_count_mode(FreecellCardCountMode::ThirtyNine)
            }
            (true, true, 3) | (true, true, 4) => {
                self.select_freecell_card_count_mode(FreecellCardCountMode::FiftyTwo)
            }
            _ => return false,
        }

        true
    }

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
            gdk::Key::space => {
                self.activate_keyboard_target();
                true
            }
            _ => false,
        }
    }

    pub(super) fn activate_keyboard_target(&self) {
        let target = match self.active_game_mode() {
            GameMode::Spider => {
                self.normalize_spider_keyboard_target(self.imp().keyboard_target.get())
            }
            GameMode::Freecell => {
                self.normalize_freecell_keyboard_target(self.imp().keyboard_target.get())
            }
            GameMode::Klondike => self.imp().keyboard_target.get(),
        };
        self.imp().keyboard_target.set(target);
        match target {
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
            KeyboardTarget::Freecell(idx) => {
                if self.is_mode_engine_ready() {
                    self.activate_freecell_slot(idx, 1);
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
        if self.active_game_mode() == GameMode::Freecell {
            self.move_keyboard_focus_horizontal_freecell(delta);
            return;
        }
        if self.active_game_mode() == GameMode::Spider {
            self.move_keyboard_focus_horizontal_spider(delta);
            return;
        }
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
        if self.active_game_mode() == GameMode::Freecell {
            self.move_keyboard_focus_vertical_freecell(delta);
            return;
        }
        if self.active_game_mode() == GameMode::Spider {
            self.move_keyboard_focus_vertical_spider(delta);
            return;
        }
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
        if self.active_game_mode() == GameMode::Spider {
            let target = self.normalize_spider_keyboard_target(imp.keyboard_target.get());
            imp.keyboard_target.set(target);
            match target {
                KeyboardTarget::Stock => {
                    imp.stock_picture.add_css_class("keyboard-focus-card");
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
                KeyboardTarget::Waste
                | KeyboardTarget::Foundation(_)
                | KeyboardTarget::Freecell(_) => {}
            }
            return;
        }
        if self.active_game_mode() == GameMode::Freecell {
            let target = self.normalize_freecell_keyboard_target(imp.keyboard_target.get());
            imp.keyboard_target.set(target);
            match target {
                KeyboardTarget::Freecell(idx) => {
                    if let Some(slot) = self.waste_fan_slots().get(idx) {
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
                KeyboardTarget::Stock | KeyboardTarget::Waste => {}
            }
            return;
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
            KeyboardTarget::Freecell(_) => {}
        }
    }

    fn normalize_freecell_keyboard_target(&self, target: KeyboardTarget) -> KeyboardTarget {
        let game = self.imp().game.borrow();
        let freecell = game.freecell();
        match target {
            KeyboardTarget::Freecell(idx) => KeyboardTarget::Freecell(idx.min(3)),
            KeyboardTarget::Foundation(idx) => KeyboardTarget::Foundation(idx.min(3)),
            KeyboardTarget::Tableau { col, start } => {
                let col = col.min(7);
                let len = freecell.tableau().get(col).map(Vec::len).unwrap_or(0);
                if len == 0 {
                    KeyboardTarget::Tableau { col, start: None }
                } else if let Some(start) = start {
                    KeyboardTarget::Tableau {
                        col,
                        start: Some(start.min(len - 1)),
                    }
                } else {
                    KeyboardTarget::Tableau {
                        col,
                        start: Some(len - 1),
                    }
                }
            }
            KeyboardTarget::Stock | KeyboardTarget::Waste => KeyboardTarget::Freecell(0),
        }
    }

    fn move_keyboard_focus_horizontal_freecell(&self, delta: i32) {
        let current = self.normalize_freecell_keyboard_target(self.imp().keyboard_target.get());
        let next = match current {
            KeyboardTarget::Freecell(idx) => {
                let idx = (idx as i32 + delta).clamp(0, 7) as usize;
                if idx < 4 {
                    KeyboardTarget::Freecell(idx)
                } else {
                    KeyboardTarget::Foundation(idx - 4)
                }
            }
            KeyboardTarget::Foundation(idx) => {
                let idx = ((idx + 4) as i32 + delta).clamp(0, 7) as usize;
                if idx < 4 {
                    KeyboardTarget::Freecell(idx)
                } else {
                    KeyboardTarget::Foundation(idx - 4)
                }
            }
            KeyboardTarget::Tableau { col, start } => {
                let new_col = (col as i32 + delta).clamp(0, 7) as usize;
                KeyboardTarget::Tableau {
                    col: new_col,
                    start,
                }
            }
            KeyboardTarget::Stock | KeyboardTarget::Waste => KeyboardTarget::Freecell(0),
        };
        self.imp().keyboard_target.set(next);
        self.update_keyboard_focus_style();
    }

    fn move_keyboard_focus_vertical_freecell(&self, delta: i32) {
        let current = self.normalize_freecell_keyboard_target(self.imp().keyboard_target.get());
        let next = match current {
            KeyboardTarget::Freecell(idx) => {
                if delta > 0 {
                    let game = self.imp().game.borrow();
                    let len = game
                        .freecell()
                        .tableau()
                        .get(idx)
                        .map(Vec::len)
                        .unwrap_or(0);
                    KeyboardTarget::Tableau {
                        col: idx,
                        start: if len == 0 { None } else { Some(len - 1) },
                    }
                } else {
                    current
                }
            }
            KeyboardTarget::Foundation(idx) => {
                if delta > 0 {
                    let col = idx + 4;
                    let game = self.imp().game.borrow();
                    let len = game
                        .freecell()
                        .tableau()
                        .get(col)
                        .map(Vec::len)
                        .unwrap_or(0);
                    KeyboardTarget::Tableau {
                        col,
                        start: if len == 0 { None } else { Some(len - 1) },
                    }
                } else {
                    current
                }
            }
            KeyboardTarget::Tableau { col, start } => {
                let game = self.imp().game.borrow();
                let len = game
                    .freecell()
                    .tableau()
                    .get(col)
                    .map(Vec::len)
                    .unwrap_or(0);
                if delta < 0 {
                    if len == 0 {
                        if col < 4 {
                            KeyboardTarget::Freecell(col)
                        } else {
                            KeyboardTarget::Foundation(col - 4)
                        }
                    } else if let Some(curr) = start {
                        if curr + 1 < len {
                            KeyboardTarget::Tableau {
                                col,
                                start: Some(curr + 1),
                            }
                        } else if col < 4 {
                            KeyboardTarget::Freecell(col)
                        } else {
                            KeyboardTarget::Foundation(col - 4)
                        }
                    } else if col < 4 {
                        KeyboardTarget::Freecell(col)
                    } else {
                        KeyboardTarget::Foundation(col - 4)
                    }
                } else if len == 0 {
                    KeyboardTarget::Tableau { col, start: None }
                } else if let Some(start) = start {
                    KeyboardTarget::Tableau {
                        col,
                        start: Some(start.saturating_sub(1)),
                    }
                } else {
                    KeyboardTarget::Tableau {
                        col,
                        start: Some(len - 1),
                    }
                }
            }
            KeyboardTarget::Stock | KeyboardTarget::Waste => KeyboardTarget::Freecell(0),
        };
        self.imp().keyboard_target.set(next);
        self.update_keyboard_focus_style();
    }

    fn normalize_spider_keyboard_target(&self, target: KeyboardTarget) -> KeyboardTarget {
        let game = self.imp().game.borrow().spider().clone();
        match target {
            KeyboardTarget::Stock => KeyboardTarget::Stock,
            KeyboardTarget::Tableau { col, start } => {
                let col = col.min(9);
                let faceups = Self::spider_face_up_indices(&game, col);
                if faceups.is_empty() {
                    KeyboardTarget::Tableau { col, start: None }
                } else if let Some(start) = start {
                    if faceups.contains(&start) {
                        KeyboardTarget::Tableau {
                            col,
                            start: Some(start),
                        }
                    } else {
                        KeyboardTarget::Tableau {
                            col,
                            start: Some(*faceups.last().unwrap_or(&0)),
                        }
                    }
                } else {
                    KeyboardTarget::Tableau { col, start: None }
                }
            }
            KeyboardTarget::Waste | KeyboardTarget::Foundation(_) | KeyboardTarget::Freecell(_) => {
                KeyboardTarget::Stock
            }
        }
    }

    fn move_keyboard_focus_horizontal_spider(&self, delta: i32) {
        let game = self.imp().game.borrow().spider().clone();
        let current = self.normalize_spider_keyboard_target(self.imp().keyboard_target.get());
        let next = match current {
            KeyboardTarget::Stock => {
                if delta > 0 {
                    Self::spider_tableau_target_for_column(&game, 0, Some(0))
                } else {
                    KeyboardTarget::Stock
                }
            }
            KeyboardTarget::Tableau { col, start } => {
                let new_col = (col as i32 + delta).clamp(0, 9) as usize;
                let offset = Self::spider_tableau_offset_from_top(&game, col, start);
                Self::spider_tableau_target_for_column(&game, new_col, Some(offset))
            }
            KeyboardTarget::Waste | KeyboardTarget::Foundation(_) | KeyboardTarget::Freecell(_) => {
                KeyboardTarget::Stock
            }
        };
        self.imp().keyboard_target.set(next);
        self.update_keyboard_focus_style();
    }

    fn move_keyboard_focus_vertical_spider(&self, delta: i32) {
        let game = self.imp().game.borrow().spider().clone();
        let current = self.normalize_spider_keyboard_target(self.imp().keyboard_target.get());
        let next = match current {
            KeyboardTarget::Stock => {
                if delta > 0 {
                    Self::spider_tableau_target_for_column(&game, 0, Some(0))
                } else {
                    KeyboardTarget::Stock
                }
            }
            KeyboardTarget::Tableau { col, start } => {
                let faceups = Self::spider_face_up_indices(&game, col);
                if delta < 0 {
                    if let Some(curr) = start {
                        if let Some(pos) = faceups.iter().position(|&idx| idx == curr) {
                            if pos + 1 < faceups.len() {
                                KeyboardTarget::Tableau {
                                    col,
                                    start: Some(faceups[pos + 1]),
                                }
                            } else {
                                KeyboardTarget::Stock
                            }
                        } else {
                            Self::spider_tableau_target_for_column(&game, col, Some(0))
                        }
                    } else {
                        KeyboardTarget::Stock
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
                        Self::spider_tableau_target_for_column(&game, col, Some(0))
                    }
                } else {
                    KeyboardTarget::Tableau { col, start: None }
                }
            }
            KeyboardTarget::Waste | KeyboardTarget::Foundation(_) | KeyboardTarget::Freecell(_) => {
                KeyboardTarget::Stock
            }
        };
        self.imp().keyboard_target.set(next);
        self.update_keyboard_focus_style();
    }

    fn spider_face_up_indices(game: &SpiderGame, col: usize) -> Vec<usize> {
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

    fn spider_tableau_target_for_column(
        game: &SpiderGame,
        col: usize,
        prefer_offset_from_top: Option<usize>,
    ) -> KeyboardTarget {
        let faceups = Self::spider_face_up_indices(game, col);
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

    fn spider_tableau_offset_from_top(
        game: &SpiderGame,
        col: usize,
        start: Option<usize>,
    ) -> usize {
        let faceups = Self::spider_face_up_indices(game, col);
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
}

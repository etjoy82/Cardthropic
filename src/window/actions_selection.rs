use super::*;
use crate::engine::boundary;

impl CardthropicWindow {
    pub(super) fn select_or_move_tableau_with_start(
        &self,
        clicked: usize,
        clicked_start: Option<usize>,
    ) {
        if !self.guard_mode_engine("Tableau selection") {
            return;
        }
        let imp = self.imp();
        let mode = self.active_game_mode();
        if mode == GameMode::Freecell {
            if let Some(cell) = imp.selected_freecell.get() {
                imp.selected_freecell.set(None);
                if self.move_freecell_to_tableau(cell, clicked) {
                    return;
                }
                return;
            }
        }
        if imp.waste_selected.get() {
            imp.waste_selected.set(false);
            let can_move = boundary::can_move_waste_to_tableau(&imp.game.borrow(), mode, clicked);
            if can_move {
                self.move_waste_to_tableau(clicked);
            } else {
                *imp.status_override.borrow_mut() =
                    Some(format!("Waste card cannot move to T{}.", clicked + 1));
                self.render();
            }
            return;
        }
        let selected = *imp.selected_run.borrow();
        match selected {
            None => {
                if let Some(start) = clicked_start {
                    *imp.selected_run.borrow_mut() = Some(SelectedRun {
                        col: clicked,
                        start,
                    });
                    imp.selected_freecell.set(None);
                }
            }
            Some(current) if current.col == clicked => {
                if clicked_start == Some(current.start) || clicked_start.is_none() {
                    *imp.selected_run.borrow_mut() = None;
                } else if let Some(start) = clicked_start {
                    *imp.selected_run.borrow_mut() = Some(SelectedRun {
                        col: clicked,
                        start,
                    });
                    imp.selected_freecell.set(None);
                }
            }
            Some(current) => {
                self.move_tableau_run_to_tableau(current.col, current.start, clicked);
                *imp.selected_run.borrow_mut() = None;
            }
        }
        self.render();
    }

    pub(super) fn handle_waste_click(&self, n_press: i32) {
        if !self.guard_mode_engine("Waste selection") {
            return;
        }
        let imp = self.imp();
        if self.active_game_mode() == GameMode::Freecell {
            self.activate_freecell_slot(0, n_press);
            return;
        }
        if imp.suppress_waste_click_once.replace(false) {
            return;
        }
        let has_waste = boundary::waste_top(&imp.game.borrow(), self.active_game_mode()).is_some();
        if !has_waste {
            imp.waste_selected.set(false);
            self.render();
            return;
        }

        match self.smart_move_mode() {
            SmartMoveMode::DoubleClick if n_press == 2 => {
                imp.waste_selected.set(false);
                self.try_smart_move_from_waste();
                return;
            }
            SmartMoveMode::SingleClick if n_press == 1 => {
                *imp.selected_run.borrow_mut() = None;
                imp.waste_selected.set(false);
                self.try_smart_move_from_waste();
                return;
            }
            SmartMoveMode::Disabled
            | SmartMoveMode::DoubleClick
            | SmartMoveMode::SingleClick
            | SmartMoveMode::RightClick => {}
        }

        *imp.selected_run.borrow_mut() = None;
        imp.waste_selected.set(!imp.waste_selected.get());
        self.render();
    }

    pub(super) fn handle_freecell_click_x(&self, _n_press: i32, x: Option<f64>) {
        let imp = self.imp();
        let n_press = _n_press;
        let idx = x
            .map(|x| self.freecell_slot_index_from_waste_x(x))
            .or_else(|| imp.selected_freecell.get())
            .unwrap_or(0)
            .clamp(0, 3);
        self.activate_freecell_slot(idx, n_press);
    }

    pub(super) fn activate_freecell_slot(&self, idx: usize, n_press: i32) {
        let imp = self.imp();
        let idx = idx.clamp(0, 3);

        if let Some(selected) = *imp.selected_run.borrow() {
            let is_top =
                boundary::tableau_len(&imp.game.borrow(), GameMode::Freecell, selected.col)
                    .map(|len| selected.start + 1 == len)
                    .unwrap_or(false);
            if !is_top {
                *imp.status_override.borrow_mut() =
                    Some("Only top tableau cards can move to free cells.".to_string());
                self.render();
                return;
            }
            let can_move = boundary::can_move_tableau_top_to_freecell(
                &imp.game.borrow(),
                GameMode::Freecell,
                selected.col,
                idx,
            );
            if can_move && self.move_tableau_to_freecell(selected.col, idx) {
                *imp.selected_run.borrow_mut() = None;
                imp.selected_freecell.set(None);
                return;
            }
            *imp.status_override.borrow_mut() = Some(format!(
                "Cannot move tableau card to free cell F{}.",
                idx + 1
            ));
            self.render();
            return;
        }

        let card_exists = imp.game.borrow().freecell().freecell_card(idx).is_some();
        if !card_exists {
            imp.selected_freecell.set(None);
            *imp.status_override.borrow_mut() = Some(format!("Free cell F{} is empty.", idx + 1));
            self.render();
            return;
        }

        match self.smart_move_mode() {
            SmartMoveMode::DoubleClick if n_press == 2 => {
                let _ = self.try_smart_move_from_freecell(idx);
                return;
            }
            SmartMoveMode::SingleClick if n_press == 1 => {
                let _ = self.try_smart_move_from_freecell(idx);
                return;
            }
            SmartMoveMode::RightClick => {}
            _ => {}
        }

        *imp.selected_run.borrow_mut() = None;
        imp.waste_selected.set(false);
        let next = if imp.selected_freecell.get() == Some(idx) {
            None
        } else {
            Some(idx)
        };
        imp.selected_freecell.set(next);
        self.render();
    }

    pub(super) fn handle_drop_on_tableau(&self, dst: usize, payload: &str) -> bool {
        let changed = if payload == "waste" {
            self.move_waste_to_tableau(dst)
        } else if let Some(cell) = parse_freecell_payload(payload) {
            self.move_freecell_to_tableau(cell, dst)
        } else if let Some((src, start)) = parse_tableau_payload(payload) {
            self.move_tableau_run_to_tableau(src, start, dst)
        } else {
            false
        };

        if !changed {
            if payload == "waste" {
                *self.imp().status_override.borrow_mut() =
                    Some("That drag-and-drop move is not legal.".to_string());
                self.render();
            }
        }
        changed
    }

    pub(super) fn handle_drop_on_foundation(&self, foundation_idx: usize, payload: &str) -> bool {
        let mode = self.active_game_mode();
        let changed = if payload == "waste" {
            let card = boundary::waste_top(&self.imp().game.borrow(), mode);
            card.is_some_and(|card| {
                boundary::can_move_waste_to_foundation(&self.imp().game.borrow(), mode)
                    && self.foundation_slot_accepts_card(card, foundation_idx)
                    && self.move_waste_to_foundation_into_slot(Some(foundation_idx))
            })
        } else if let Some((src, _start)) = parse_tableau_payload(payload) {
            let card = boundary::tableau_top(&self.imp().game.borrow(), mode, src);
            card.is_some_and(|card| {
                boundary::can_move_tableau_top_to_foundation(&self.imp().game.borrow(), mode, src)
                    && self.foundation_slot_accepts_card(card, foundation_idx)
                    && self.move_tableau_to_foundation_into_slot(src, Some(foundation_idx))
            })
        } else if let Some(cell) = parse_freecell_payload(payload) {
            let card = self.imp().game.borrow().freecell().freecell_card(cell);
            card.is_some_and(|card| {
                boundary::can_move_freecell_to_foundation(&self.imp().game.borrow(), mode, cell)
                    && self.foundation_slot_accepts_card(card, foundation_idx)
                    && self.move_freecell_to_foundation_into_slot(cell, Some(foundation_idx))
            })
        } else {
            false
        };

        if !changed {
            *self.imp().status_override.borrow_mut() =
                Some("Drop to that foundation was not legal.".to_string());
            self.render();
        }
        changed
    }

    pub(super) fn handle_click_on_foundation(&self, foundation_idx: usize) {
        if !self.guard_mode_engine("Foundation move") {
            return;
        }

        let imp = self.imp();
        let mode = self.active_game_mode();
        let mut did_move = false;

        if imp.waste_selected.get() {
            let card = boundary::waste_top(&imp.game.borrow(), mode);
            if card.is_some_and(|card| {
                boundary::can_move_waste_to_foundation(&imp.game.borrow(), mode)
                    && self.foundation_slot_accepts_card(card, foundation_idx)
            }) {
                did_move = self.move_waste_to_foundation_into_slot(Some(foundation_idx));
                if did_move {
                    return;
                }
            }
        }

        let selected_run = { *imp.selected_run.borrow() };
        if let Some(selected) = selected_run {
            let selected_is_top = boundary::tableau_len(&imp.game.borrow(), mode, selected.col)
                .map(|len| selected.start + 1 == len)
                .unwrap_or(false);
            if !selected_is_top {
                *imp.status_override.borrow_mut() =
                    Some("Only the top card of a tableau can move to foundation.".to_string());
                self.render();
                return;
            }
            let card = boundary::tableau_top(&imp.game.borrow(), mode, selected.col);
            if card.is_some_and(|card| {
                boundary::can_move_tableau_top_to_foundation(&imp.game.borrow(), mode, selected.col)
                    && self.foundation_slot_accepts_card(card, foundation_idx)
            }) {
                did_move =
                    self.move_tableau_to_foundation_into_slot(selected.col, Some(foundation_idx));
            }
            if did_move {
                *imp.selected_run.borrow_mut() = None;
                return;
            }
        }

        if mode == GameMode::Freecell {
            if let Some(cell) = imp.selected_freecell.get() {
                let suit_ok = imp
                    .game
                    .borrow()
                    .freecell()
                    .freecell_card(cell)
                    .is_some_and(|card| {
                        boundary::can_move_freecell_to_foundation(&imp.game.borrow(), mode, cell)
                            && self.foundation_slot_accepts_card(card, foundation_idx)
                    });
                if suit_ok && self.move_freecell_to_foundation_into_slot(cell, Some(foundation_idx))
                {
                    imp.selected_freecell.set(None);
                    return;
                }
            }
        }

        if let Some(suit_foundation_idx) = self.foundation_suit_index_for_slot(foundation_idx) {
            let foundation_top_exists =
                boundary::foundation_top_exists(&imp.game.borrow(), mode, suit_foundation_idx);
            if foundation_top_exists {
                let tableau_columns = match mode {
                    GameMode::Spider => 10,
                    GameMode::Freecell => 8,
                    _ => 7,
                };
                for dst in 0..tableau_columns {
                    let can_move = boundary::can_move_foundation_top_to_tableau(
                        &imp.game.borrow(),
                        mode,
                        suit_foundation_idx,
                        dst,
                    );
                    if can_move && self.move_foundation_to_tableau(foundation_idx, dst) {
                        *imp.status_override.borrow_mut() =
                            Some(format!("Moved foundation card to T{}.", dst + 1));
                        self.render();
                        return;
                    }
                }
            }
        }

        *imp.status_override.borrow_mut() =
            Some("That move to foundation is not legal.".to_string());
        self.render();
    }
}

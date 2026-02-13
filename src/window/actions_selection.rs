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
            SmartMoveMode::Disabled | SmartMoveMode::DoubleClick | SmartMoveMode::SingleClick => {}
        }

        *imp.selected_run.borrow_mut() = None;
        imp.waste_selected.set(!imp.waste_selected.get());
        self.render();
    }

    pub(super) fn handle_drop_on_tableau(&self, dst: usize, payload: &str) -> bool {
        let changed = if payload == "waste" {
            self.move_waste_to_tableau(dst)
        } else if let Some((src, start)) = parse_tableau_payload(payload) {
            self.move_tableau_run_to_tableau(src, start, dst)
        } else {
            false
        };

        if !changed {
            *self.imp().status_override.borrow_mut() =
                Some("That drag-and-drop move is not legal.".to_string());
            self.render();
        }
        changed
    }

    pub(super) fn handle_drop_on_foundation(&self, foundation_idx: usize, payload: &str) -> bool {
        let mode = self.active_game_mode();
        let changed = if payload == "waste" {
            let suit_ok = boundary::waste_top_matches_foundation(
                &self.imp().game.borrow(),
                mode,
                foundation_idx,
            );
            suit_ok && self.move_waste_to_foundation()
        } else if let Some((src, _start)) = parse_tableau_payload(payload) {
            let suit_ok = boundary::tableau_top_matches_foundation(
                &self.imp().game.borrow(),
                mode,
                src,
                foundation_idx,
            );
            suit_ok && self.move_tableau_to_foundation(src)
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
            let suit_ok =
                boundary::waste_top_matches_foundation(&imp.game.borrow(), mode, foundation_idx);
            if suit_ok {
                did_move = self.move_waste_to_foundation();
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
            let suit_ok = boundary::tableau_top_matches_foundation(
                &imp.game.borrow(),
                mode,
                selected.col,
                foundation_idx,
            );
            if suit_ok {
                did_move = self.move_tableau_to_foundation(selected.col);
            }
            if did_move {
                *imp.selected_run.borrow_mut() = None;
                return;
            }
        }

        let foundation_top_exists =
            boundary::foundation_top_exists(&imp.game.borrow(), mode, foundation_idx);
        if foundation_top_exists {
            for dst in 0..7 {
                let can_move = boundary::can_move_foundation_top_to_tableau(
                    &imp.game.borrow(),
                    mode,
                    foundation_idx,
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

        *imp.status_override.borrow_mut() =
            Some("That move to foundation is not legal.".to_string());
        self.render();
    }
}

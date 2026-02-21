use crate::game::{apply_move, legal_moves};
use crate::CardthropicWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;

impl CardthropicWindow {
    fn chess_transition_move(
        previous: &crate::game::ChessPosition,
        next: &crate::game::ChessPosition,
    ) -> Option<(crate::game::Square, crate::game::Square)> {
        legal_moves(previous).into_iter().find_map(|candidate| {
            let mut probe = previous.clone();
            if !apply_move(&mut probe, candidate) {
                return None;
            }
            if probe == *next {
                Some((candidate.from, candidate.to))
            } else {
                None
            }
        })
    }

    pub(in crate::window) fn reset_chess_session_state(&self) {
        let imp = self.imp();
        self.cancel_pending_chess_ai_search();
        imp.chess_selected_square.set(None);
        imp.chess_keyboard_square.set(None);
        imp.chess_last_move_from.set(None);
        imp.chess_last_move_to.set(None);
        imp.chess_history.borrow_mut().clear();
        imp.chess_future.borrow_mut().clear();
    }

    pub(in crate::window) fn push_chess_history_position(
        &self,
        previous_position: crate::game::ChessPosition,
    ) {
        let imp = self.imp();
        imp.chess_history.borrow_mut().push(previous_position);
        imp.chess_future.borrow_mut().clear();
    }

    pub(in crate::window) fn chess_undo(&self) -> bool {
        let imp = self.imp();
        let Some(previous) = imp.chess_history.borrow_mut().pop() else {
            *imp.status_override.borrow_mut() = Some("Nothing to undo yet.".to_string());
            self.render();
            return false;
        };

        let current = imp.chess_position.borrow().clone();
        imp.chess_future.borrow_mut().push(current);
        *imp.chess_position.borrow_mut() = previous;
        imp.chess_selected_square.set(None);
        imp.chess_keyboard_square.set(None);
        let position_after_undo = imp.chess_position.borrow().clone();
        let last_move = imp
            .chess_history
            .borrow()
            .last()
            .and_then(|prior| Self::chess_transition_move(prior, &position_after_undo));
        imp.chess_last_move_from.set(last_move.map(|mv| mv.0));
        imp.chess_last_move_to.set(last_move.map(|mv| mv.1));
        imp.move_count.set(imp.move_count.get().saturating_sub(1));
        let has_legal_moves = !legal_moves(&imp.chess_position.borrow()).is_empty();
        imp.timer_started
            .set(imp.move_count.get() > 0 && has_legal_moves);
        *imp.status_override.borrow_mut() = Some("Undid last chess move.".to_string());
        let rendered_by_flip = self.maybe_auto_flip_chess_board_to_side_to_move(false);
        if !rendered_by_flip {
            self.render();
        }
        true
    }

    pub(in crate::window) fn chess_redo(&self) -> bool {
        let imp = self.imp();
        let Some(next) = imp.chess_future.borrow_mut().pop() else {
            *imp.status_override.borrow_mut() = Some("Nothing to redo yet.".to_string());
            self.render();
            return false;
        };

        let current = imp.chess_position.borrow().clone();
        let redone_move = Self::chess_transition_move(&current, &next);
        imp.chess_history.borrow_mut().push(current);
        *imp.chess_position.borrow_mut() = next;
        imp.chess_selected_square.set(None);
        imp.chess_keyboard_square.set(None);
        imp.chess_last_move_from.set(redone_move.map(|mv| mv.0));
        imp.chess_last_move_to.set(redone_move.map(|mv| mv.1));
        imp.move_count.set(imp.move_count.get().saturating_add(1));
        let has_legal_moves = !legal_moves(&imp.chess_position.borrow()).is_empty();
        imp.timer_started
            .set(imp.move_count.get() > 0 && has_legal_moves);
        *imp.status_override.borrow_mut() = Some("Redid chess move.".to_string());
        let rendered_by_flip = self.maybe_auto_flip_chess_board_to_side_to_move(false);
        if !rendered_by_flip {
            self.render();
        }
        true
    }
}

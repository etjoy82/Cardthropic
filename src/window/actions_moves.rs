use super::*;
use crate::engine::boundary;
use crate::engine::commands::EngineCommand;
use crate::game::Suit;
use crate::window::motion::MotionTarget;

fn foundation_index_for_suit(suit: Suit) -> usize {
    match suit {
        Suit::Clubs => 0,
        Suit::Diamonds => 1,
        Suit::Hearts => 2,
        Suit::Spades => 3,
    }
}

impl CardthropicWindow {
    pub(super) fn is_face_up_tableau_run(&self, col: usize, start: usize) -> bool {
        let Some(game) = boundary::clone_klondike_for_automation(
            &self.imp().game.borrow(),
            self.active_game_mode(),
            self.current_klondike_draw_mode(),
        ) else {
            return false;
        };

        let Some(len) = game.tableau_len(col) else {
            return false;
        };
        if start >= len {
            return false;
        }

        (start..len).all(|idx| game.tableau_card(col, idx).is_some_and(|card| card.face_up))
    }

    pub(super) fn draw_card(&self) -> bool {
        if !self.guard_mode_engine("Draw") {
            return false;
        }
        let mode = self.active_game_mode();
        let should_animate = self.should_play_non_drag_move_animation();
        let animation_from = self.capture_motion_source(MotionTarget::Stock);
        let animation_to = self.capture_motion_source(MotionTarget::WasteTop);
        let draw_mode = self.current_klondike_draw_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::DrawOrRecycle { draw_mode },
        )
        .changed;

        if !self.apply_changed_move(snapshot, changed) {
            *self.imp().status_override.borrow_mut() = Some("Nothing to draw.".to_string());
        }
        self.render();
        if changed && should_animate {
            let top_card = boundary::waste_top(&self.imp().game.borrow(), mode);
            if let (Some(from), Some(to), Some(card)) = (animation_from, animation_to, top_card) {
                if let Some(texture) = self.glitched_texture_for_card_motion(card) {
                    self.play_move_animation_to_point(texture, from, to);
                }
            }
        }
        changed
    }

    pub(super) fn cyclone_shuffle_tableau(&self) -> bool {
        if !self.guard_mode_engine("Cyclone shuffle") {
            return false;
        }

        let mode = self.active_game_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::CycloneShuffleTableau,
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        if changed {
            *self.imp().selected_run.borrow_mut() = None;
            *self.imp().status_override.borrow_mut() = Some(
                "Cyclone shuffle complete: rerolled tableau while preserving each column's geometry."
                    .to_string(),
            );
        } else {
            *self.imp().status_override.borrow_mut() =
                Some("Cyclone shuffle had no effect.".to_string());
        }
        self.render();
        changed
    }

    pub(super) fn trigger_peek(&self) {
        if !self.guard_mode_engine("Peek") {
            return;
        }
        let imp = self.imp();
        let generation = imp.peek_generation.get().wrapping_add(1);
        imp.peek_generation.set(generation);
        imp.peek_active.set(true);
        *imp.status_override.borrow_mut() =
            Some("Peek active: face-up cards hidden, face-down cards revealed for 3s.".to_string());
        self.render();

        glib::timeout_add_local_once(
            Duration::from_secs(3),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                move || {
                    let imp = window.imp();
                    if imp.peek_generation.get() != generation {
                        return;
                    }
                    imp.peek_active.set(false);
                    *imp.status_override.borrow_mut() =
                        Some("Peek ended. Resume play or trigger Peek again.".to_string());
                    window.render();
                }
            ),
        );
    }

    pub(super) fn move_waste_to_foundation(&self) -> bool {
        if !self.guard_mode_engine("Waste-to-foundation move") {
            return false;
        }
        let mode = self.active_game_mode();
        let should_animate = self.should_play_non_drag_move_animation();
        let waste_top = boundary::waste_top(&self.imp().game.borrow(), mode);
        let animation_from = self.capture_motion_source(MotionTarget::WasteTop);
        let animation_to = waste_top.and_then(|card| {
            self.capture_motion_source(MotionTarget::Foundation(foundation_index_for_suit(
                card.suit,
            )))
        });
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveWasteToFoundation,
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        if changed && should_animate {
            if let (Some(card), Some(from), Some(to)) = (waste_top, animation_from, animation_to) {
                if let Some(texture) = self.glitched_texture_for_card_motion(card) {
                    self.play_move_animation_to_point(texture, from, to);
                }
            }
        }
        changed
    }

    pub(super) fn move_waste_to_tableau(&self, dst: usize) -> bool {
        if !self.guard_mode_engine("Waste-to-tableau move") {
            return false;
        }
        let mode = self.active_game_mode();
        let should_animate = self.should_play_non_drag_move_animation();
        let waste_top = boundary::waste_top(&self.imp().game.borrow(), mode);
        let animation_from = self.capture_motion_source(MotionTarget::WasteTop);
        let animation_to = self.capture_tableau_landing_point(dst);
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveWasteToTableau { dst },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        if changed && should_animate {
            if let (Some(card), Some(from), Some(to)) = (waste_top, animation_from, animation_to) {
                if let Some(texture) = self.glitched_texture_for_card_motion(card) {
                    self.play_move_animation_to_point(texture, from, to);
                }
            }
        }
        changed
    }

    pub(super) fn move_tableau_run_to_tableau(&self, src: usize, start: usize, dst: usize) -> bool {
        if !self.guard_mode_engine("Tableau move") {
            return false;
        }
        if !self.is_face_up_tableau_run(src, start) {
            *self.imp().status_override.borrow_mut() =
                Some("That move includes hidden cards and is not legal.".to_string());
            self.render();
            return false;
        }
        let mode = self.active_game_mode();
        let should_animate = self.should_play_non_drag_move_animation();
        let animation_from = self.capture_motion_source(MotionTarget::TableauCard {
            col: src,
            index: start,
        });
        let animation_to = self.capture_tableau_landing_point(dst);
        let run_texture = self.glitched_texture_for_tableau_run_motion(src, start);
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveTableauRunToTableau { src, start, dst },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        if changed && should_animate {
            if let (Some(texture), Some(from), Some(to)) =
                (run_texture, animation_from, animation_to)
            {
                self.play_move_animation_to_point(texture, from, to);
            }
        }
        changed
    }

    pub(super) fn move_tableau_to_foundation(&self, src: usize) -> bool {
        if !self.guard_mode_engine("Tableau-to-foundation move") {
            return false;
        }
        let mode = self.active_game_mode();
        let should_animate = self.should_play_non_drag_move_animation();
        let top_card = boundary::tableau_top(&self.imp().game.borrow(), mode, src);
        let animation_from = boundary::tableau_len(&self.imp().game.borrow(), mode, src)
            .and_then(|len| len.checked_sub(1))
            .and_then(|index| {
                self.capture_motion_source(MotionTarget::TableauCard { col: src, index })
            });
        let animation_to = top_card.and_then(|card| {
            self.capture_motion_source(MotionTarget::Foundation(foundation_index_for_suit(
                card.suit,
            )))
        });
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveTableauTopToFoundation { src },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        if changed && should_animate {
            if let (Some(card), Some(from), Some(to)) = (top_card, animation_from, animation_to) {
                if let Some(texture) = self.glitched_texture_for_card_motion(card) {
                    self.play_move_animation_to_point(texture, from, to);
                }
            }
        }
        changed
    }

    pub(super) fn move_foundation_to_tableau(&self, foundation_idx: usize, dst: usize) -> bool {
        if !self.guard_mode_engine("Foundation-to-tableau move") {
            return false;
        }
        let mode = self.active_game_mode();
        let should_animate = self.should_play_non_drag_move_animation();
        let card = boundary::clone_klondike_for_automation(
            &self.imp().game.borrow(),
            mode,
            self.current_klondike_draw_mode(),
        )
        .and_then(|game| game.foundations()[foundation_idx].last().copied());
        let animation_from = self.capture_motion_source(MotionTarget::Foundation(foundation_idx));
        let animation_to = self.capture_tableau_landing_point(dst);
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveFoundationTopToTableau {
                foundation_idx,
                dst,
            },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        if changed && should_animate {
            if let (Some(card), Some(from), Some(to)) = (card, animation_from, animation_to) {
                if let Some(texture) = self.glitched_texture_for_card_motion(card) {
                    self.play_move_animation_to_point(texture, from, to);
                }
            }
        }
        changed
    }
}

use super::*;
use crate::engine::boundary;
use crate::engine::commands::EngineCommand;
use crate::window::motion::MotionTarget;

impl CardthropicWindow {
    fn freecell_movable_capacity_for_dst(&self, dst: usize) -> usize {
        let game = self.imp().game.borrow();
        let freecell = game.freecell();
        let free_empty = freecell
            .freecells()
            .iter()
            .filter(|slot| slot.is_none())
            .count();
        let mut empty_tableau = freecell
            .tableau()
            .iter()
            .filter(|pile| pile.is_empty())
            .count();
        if freecell
            .tableau()
            .get(dst)
            .is_some_and(|pile| pile.is_empty())
        {
            empty_tableau = empty_tableau.saturating_sub(1);
        }
        (free_empty + 1) * (1usize << empty_tableau)
    }

    fn is_descending_alternating(cards: &[Card]) -> bool {
        cards.windows(2).all(|pair| {
            let a = pair[0];
            let b = pair[1];
            a.rank == b.rank + 1 && a.color_red() != b.color_red()
        })
    }

    fn freecell_tableau_run_failure_message(&self, src: usize, start: usize, dst: usize) -> String {
        let game = self.imp().game.borrow();
        let freecell = game.freecell();
        if src == dst {
            return "Source and destination tableau columns are the same.".to_string();
        }
        let Some(source) = freecell.tableau().get(src) else {
            return "That source tableau column does not exist.".to_string();
        };
        let Some(dest) = freecell.tableau().get(dst) else {
            return "That destination tableau column does not exist.".to_string();
        };
        if start >= source.len() {
            return "That run start is outside the source column.".to_string();
        }
        let run = &source[start..];
        if run.is_empty() || !Self::is_descending_alternating(run) {
            return "The selected run must descend by one and alternate colors.".to_string();
        }
        let first = run[0];
        if let Some(top) = dest.last().copied() {
            if top.rank != first.rank + 1 || top.color_red() == first.color_red() {
                return format!(
                    "{} cannot move to {}. Need opposite color and one rank higher.",
                    first.label(),
                    top.label()
                );
            }
        }
        let capacity = self.freecell_movable_capacity_for_dst(dst);
        if run.len() > capacity {
            return format!(
                "Need capacity {}, have {}. Free up cells/columns to move this run.",
                run.len(),
                capacity
            );
        }
        "That tableau move is not legal.".to_string()
    }

    fn freecell_freecell_to_tableau_failure_message(&self, cell: usize, dst: usize) -> String {
        let game = self.imp().game.borrow();
        let freecell = game.freecell();
        let Some(card) = freecell.freecell_card(cell) else {
            return format!("Free cell F{} is empty.", cell + 1);
        };
        let Some(dest) = freecell.tableau().get(dst) else {
            return "That destination tableau column does not exist.".to_string();
        };
        if let Some(top) = dest.last().copied() {
            if top.rank != card.rank + 1 || top.color_red() == card.color_red() {
                return format!(
                    "{} cannot move to {}. Need opposite color and one rank higher.",
                    card.label(),
                    top.label()
                );
            }
        }
        format!("{} cannot move to T{}.", card.label(), dst + 1)
    }

    pub(super) fn is_face_up_tableau_run(&self, col: usize, start: usize) -> bool {
        match self.active_game_mode() {
            GameMode::Spider => {
                let game = self.imp().game.borrow();
                let spider = game.spider();
                let Some(len) = spider.tableau().get(col).map(Vec::len) else {
                    return false;
                };
                if start >= len {
                    return false;
                }
                (start..len).all(|idx| spider.tableau_card(col, idx).is_some_and(|c| c.face_up))
            }
            GameMode::Freecell => {
                let game = self.imp().game.borrow();
                let freecell = game.freecell();
                let Some(len) = freecell.tableau().get(col).map(Vec::len) else {
                    return false;
                };
                start < len
            }
            _ => {
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
        }
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
        self.move_waste_to_foundation_into_slot(None)
    }

    pub(super) fn move_waste_to_foundation_into_slot(&self, preferred_slot: Option<usize>) -> bool {
        if !self.guard_mode_engine("Waste-to-foundation move") {
            return false;
        }
        let mode = self.active_game_mode();
        let should_animate = self.should_play_non_drag_move_animation();
        let waste_top = boundary::waste_top(&self.imp().game.borrow(), mode);
        let Some(card) = waste_top else {
            return false;
        };
        let Some(target_slot) = self.resolve_foundation_slot_for_card(card, preferred_slot) else {
            return false;
        };
        let animation_from = self.capture_motion_source(MotionTarget::WasteTop);
        let animation_to = self.capture_motion_source(MotionTarget::Foundation(target_slot));
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveWasteToFoundation,
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        if changed {
            self.establish_foundation_slot_for_card(card, target_slot);
        }
        self.render();
        if changed && should_animate {
            if let (Some(from), Some(to)) = (animation_from, animation_to) {
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
        if !changed && mode == GameMode::Freecell {
            *self.imp().status_override.borrow_mut() =
                Some(self.freecell_tableau_run_failure_message(src, start, dst));
        }
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
        self.move_tableau_to_foundation_into_slot(src, None)
    }

    pub(super) fn move_tableau_to_foundation_into_slot(
        &self,
        src: usize,
        preferred_slot: Option<usize>,
    ) -> bool {
        if !self.guard_mode_engine("Tableau-to-foundation move") {
            return false;
        }
        let mode = self.active_game_mode();
        let should_animate = self.should_play_non_drag_move_animation();
        let top_card = boundary::tableau_top(&self.imp().game.borrow(), mode, src);
        let Some(card) = top_card else {
            return false;
        };
        let Some(target_slot) = self.resolve_foundation_slot_for_card(card, preferred_slot) else {
            return false;
        };
        let animation_from = boundary::tableau_len(&self.imp().game.borrow(), mode, src)
            .and_then(|len| len.checked_sub(1))
            .and_then(|index| {
                self.capture_motion_source(MotionTarget::TableauCard { col: src, index })
            });
        let animation_to = self.capture_motion_source(MotionTarget::Foundation(target_slot));
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveTableauTopToFoundation { src },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        if changed {
            self.establish_foundation_slot_for_card(card, target_slot);
        }
        self.render();
        if changed && should_animate {
            if let (Some(from), Some(to)) = (animation_from, animation_to) {
                if let Some(texture) = self.glitched_texture_for_card_motion(card) {
                    self.play_move_animation_to_point(texture, from, to);
                }
            }
        }
        changed
    }

    pub(super) fn move_foundation_to_tableau(
        &self,
        foundation_slot_idx: usize,
        dst: usize,
    ) -> bool {
        if !self.guard_mode_engine("Foundation-to-tableau move") {
            return false;
        }
        let Some(foundation_idx) = self.foundation_suit_index_for_slot(foundation_slot_idx) else {
            return false;
        };
        let mode = self.active_game_mode();
        let should_animate = self.should_play_non_drag_move_animation();
        let card = if mode == GameMode::Freecell {
            self.imp()
                .game
                .borrow()
                .freecell()
                .foundations()
                .get(foundation_idx)
                .and_then(|pile| pile.last().copied())
        } else {
            boundary::clone_klondike_for_automation(
                &self.imp().game.borrow(),
                mode,
                self.current_klondike_draw_mode(),
            )
            .and_then(|game| game.foundations()[foundation_idx].last().copied())
        };
        let animation_from =
            self.capture_motion_source(MotionTarget::Foundation(foundation_slot_idx));
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

    pub(super) fn move_tableau_to_freecell(&self, src: usize, cell: usize) -> bool {
        if !self.guard_mode_engine("Tableau-to-freecell move") {
            return false;
        }
        let mode = self.active_game_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveTableauTopToFreecell { src, cell },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        self.render();
        changed
    }

    pub(super) fn move_freecell_to_tableau(&self, cell: usize, dst: usize) -> bool {
        if !self.guard_mode_engine("Freecell-to-tableau move") {
            return false;
        }
        let mode = self.active_game_mode();
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveFreecellToTableau { cell, dst },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        if !changed && mode == GameMode::Freecell {
            *self.imp().status_override.borrow_mut() =
                Some(self.freecell_freecell_to_tableau_failure_message(cell, dst));
        }
        self.render();
        changed
    }

    pub(super) fn move_freecell_to_foundation(&self, cell: usize) -> bool {
        self.move_freecell_to_foundation_into_slot(cell, None)
    }

    pub(super) fn move_freecell_to_foundation_into_slot(
        &self,
        cell: usize,
        preferred_slot: Option<usize>,
    ) -> bool {
        if !self.guard_mode_engine("Freecell-to-foundation move") {
            return false;
        }
        let mode = self.active_game_mode();
        let card = self.imp().game.borrow().freecell().freecell_card(cell);
        let Some(card) = card else {
            return false;
        };
        let Some(target_slot) = self.resolve_foundation_slot_for_card(card, preferred_slot) else {
            return false;
        };
        let snapshot = self.snapshot();
        let changed = boundary::execute_command(
            &mut self.imp().game.borrow_mut(),
            mode,
            EngineCommand::MoveFreecellToFoundation { cell },
        )
        .changed;
        let changed = self.apply_changed_move(snapshot, changed);
        if changed {
            self.establish_foundation_slot_for_card(card, target_slot);
        }
        self.render();
        changed
    }
}

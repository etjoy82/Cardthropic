use super::*;
use crate::engine::render_plan;
use crate::game::SpiderGame;

impl CardthropicWindow {
    pub(super) fn render_tableau_columns(
        &self,
        game: &KlondikeGame,
        deck: &AngloDeck,
        card_width: i32,
        card_height: i32,
        face_up_step: i32,
        face_down_step: i32,
        peek_active: bool,
    ) {
        let imp = self.imp();
        let mode_columns = game.tableau().len();
        let stacks = self.tableau_stacks();
        let mut tableau_card_pictures = imp.tableau_card_pictures.borrow_mut();
        let mut tableau_picture_state_cache = imp.tableau_picture_state_cache.borrow_mut();
        if tableau_card_pictures.len() < stacks.len() {
            tableau_card_pictures.resize_with(stacks.len(), Vec::new);
        }
        if tableau_picture_state_cache.len() < stacks.len() {
            tableau_picture_state_cache.resize_with(stacks.len(), Vec::new);
        }

        for (idx, stack) in stacks.iter().enumerate() {
            let pictures = &mut tableau_card_pictures[idx];
            let states = &mut tableau_picture_state_cache[idx];

            if idx >= mode_columns {
                if stack.is_visible() {
                    stack.set_visible(false);
                }
                while let Some(picture) = pictures.pop() {
                    stack.remove(&picture);
                }
                states.clear();
                continue;
            }

            if !stack.is_visible() {
                stack.set_visible(true);
            }
            if stack.width_request() != card_width {
                stack.set_width_request(card_width);
            }

            let column = &game.tableau()[idx];
            let selected_run = *imp.selected_run.borrow();
            while pictures.len() > column.len() {
                if let Some(picture) = pictures.pop() {
                    stack.remove(&picture);
                }
                let _ = states.pop();
            }
            while pictures.len() < column.len() {
                let picture = gtk::Picture::new();
                picture.set_can_shrink(true);
                picture.set_content_fit(gtk::ContentFit::Contain);
                stack.put(&picture, 0.0, 0.0);
                pictures.push(picture);
                states.push(None);
            }
            let mut y = 0;
            for (card_idx, card) in column.iter().enumerate() {
                let picture = &pictures[card_idx];
                if picture.width_request() != card_width {
                    picture.set_width_request(card_width);
                }
                if picture.height_request() != card_height {
                    picture.set_height_request(card_height);
                }

                let show_face_up = if peek_active {
                    !card.face_up
                } else {
                    card.face_up
                };
                let selected = selected_run.is_some_and(|run| run.col == idx && card_idx >= run.start);
                let previous = states.get(card_idx).copied().flatten();
                if previous.map(|state| state.selected) != Some(selected) {
                    if selected {
                        picture.add_css_class("tableau-selected-card");
                    } else {
                        picture.remove_css_class("tableau-selected-card");
                    }
                }

                if previous.map(|state| (state.card, state.display_face_up))
                    != Some((*card, show_face_up))
                {
                    let texture = if show_face_up {
                        deck.texture_for_card(*card)
                    } else {
                        deck.back_texture()
                    };
                    picture.set_paintable(Some(&texture));
                }

                if previous.map(|state| state.y) != Some(y) {
                    stack.move_(picture, 0.0, f64::from(y));
                }

                if let Some(slot) = states.get_mut(card_idx) {
                    *slot = Some(TableauPictureRenderState {
                        card: *card,
                        display_face_up: show_face_up,
                        selected,
                        y,
                        card_width,
                        card_height,
                    });
                }

                if card_idx + 1 < column.len() {
                    y += if card.face_up {
                        face_up_step
                    } else {
                        face_down_step
                    };
                }
            }
            while states.len() > pictures.len() {
                let _ = states.pop();
            }

            let stack_height = render_plan::tableau_stack_height(
                column,
                card_height,
                face_up_step,
                face_down_step,
            );
            if stack.height_request() != stack_height {
                stack.set_height_request(stack_height);
            }
        }
    }

    pub(super) fn render_tableau_columns_spider(
        &self,
        game: &SpiderGame,
        deck: &AngloDeck,
        card_width: i32,
        card_height: i32,
        face_up_step: i32,
        face_down_step: i32,
        peek_active: bool,
    ) {
        let imp = self.imp();
        let mode_columns = game.tableau().len();
        let stacks = self.tableau_stacks();
        let mut tableau_card_pictures = imp.tableau_card_pictures.borrow_mut();
        let mut tableau_picture_state_cache = imp.tableau_picture_state_cache.borrow_mut();
        if tableau_card_pictures.len() < stacks.len() {
            tableau_card_pictures.resize_with(stacks.len(), Vec::new);
        }
        if tableau_picture_state_cache.len() < stacks.len() {
            tableau_picture_state_cache.resize_with(stacks.len(), Vec::new);
        }

        for (idx, stack) in stacks.iter().enumerate() {
            let pictures = &mut tableau_card_pictures[idx];
            let states = &mut tableau_picture_state_cache[idx];

            if idx >= mode_columns {
                if stack.is_visible() {
                    stack.set_visible(false);
                }
                while let Some(picture) = pictures.pop() {
                    stack.remove(&picture);
                }
                states.clear();
                continue;
            }

            if !stack.is_visible() {
                stack.set_visible(true);
            }
            if stack.width_request() != card_width {
                stack.set_width_request(card_width);
            }

            let column = &game.tableau()[idx];
            let selected_run = *imp.selected_run.borrow();
            while pictures.len() > column.len() {
                if let Some(picture) = pictures.pop() {
                    stack.remove(&picture);
                }
                let _ = states.pop();
            }
            while pictures.len() < column.len() {
                let picture = gtk::Picture::new();
                picture.set_can_shrink(true);
                picture.set_content_fit(gtk::ContentFit::Contain);
                stack.put(&picture, 0.0, 0.0);
                pictures.push(picture);
                states.push(None);
            }
            let mut y = 0;
            for (card_idx, card) in column.iter().enumerate() {
                let picture = &pictures[card_idx];
                if picture.width_request() != card_width {
                    picture.set_width_request(card_width);
                }
                if picture.height_request() != card_height {
                    picture.set_height_request(card_height);
                }

                let show_face_up = if peek_active {
                    !card.face_up
                } else {
                    card.face_up
                };
                let selected = selected_run.is_some_and(|run| run.col == idx && card_idx >= run.start);
                let previous = states.get(card_idx).copied().flatten();
                if previous.map(|state| state.selected) != Some(selected) {
                    if selected {
                        picture.add_css_class("tableau-selected-card");
                    } else {
                        picture.remove_css_class("tableau-selected-card");
                    }
                }

                if previous.map(|state| (state.card, state.display_face_up))
                    != Some((*card, show_face_up))
                {
                    let texture = if show_face_up {
                        deck.texture_for_card(*card)
                    } else {
                        deck.back_texture()
                    };
                    picture.set_paintable(Some(&texture));
                }

                if previous.map(|state| state.y) != Some(y) {
                    stack.move_(picture, 0.0, f64::from(y));
                }

                if let Some(slot) = states.get_mut(card_idx) {
                    *slot = Some(TableauPictureRenderState {
                        card: *card,
                        display_face_up: show_face_up,
                        selected,
                        y,
                        card_width,
                        card_height,
                    });
                }

                if card_idx + 1 < column.len() {
                    y += if card.face_up {
                        face_up_step
                    } else {
                        face_down_step
                    };
                }
            }
            while states.len() > pictures.len() {
                let _ = states.pop();
            }

            let stack_height = render_plan::tableau_stack_height(
                column,
                card_height,
                face_up_step,
                face_down_step,
            );
            if stack.height_request() != stack_height {
                stack.set_height_request(stack_height);
            }
        }
    }
}

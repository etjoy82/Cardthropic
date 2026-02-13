use super::*;
use crate::engine::render_plan;

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
        let mut tableau_card_pictures = vec![Vec::new(); 7];

        for (idx, stack) in self.tableau_stacks().into_iter().enumerate() {
            while let Some(child) = stack.first_child() {
                stack.remove(&child);
            }

            stack.set_width_request(card_width);

            let column = &game.tableau()[idx];
            let mut y = 0;
            for (card_idx, card) in column.iter().enumerate() {
                let picture = gtk::Picture::new();
                picture.set_width_request(card_width);
                picture.set_height_request(card_height);
                picture.set_can_shrink(true);
                picture.set_content_fit(gtk::ContentFit::Contain);

                let show_face_up = if peek_active {
                    !card.face_up
                } else {
                    card.face_up
                };
                let texture = if show_face_up {
                    deck.texture_for_card(*card)
                } else {
                    deck.back_texture()
                };
                picture.set_paintable(Some(&texture));
                tableau_card_pictures[idx].push(picture.clone());

                stack.put(&picture, 0.0, f64::from(y));
                if card_idx + 1 < column.len() {
                    y += if card.face_up {
                        face_up_step
                    } else {
                        face_down_step
                    };
                }
            }

            let stack_height = render_plan::tableau_stack_height(
                column,
                card_height,
                face_up_step,
                face_down_step,
            );
            stack.set_height_request(stack_height);
        }

        *imp.tableau_card_pictures.borrow_mut() = tableau_card_pictures;
    }
}

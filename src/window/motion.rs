use super::*;
use crate::engine::boundary;
use std::time::{Duration, Instant};

const MOVE_ANIMATION_DURATION_MS: u64 = 100;
const MOVE_ANIMATION_TICK_MS: u64 = 16;

#[derive(Debug, Clone, Copy)]
pub(super) enum MotionTarget {
    Stock,
    WasteTop,
    Foundation(usize),
    Tableau(usize),
    TableauCard { col: usize, index: usize },
}

impl CardthropicWindow {
    fn ease_out_cubic(t: f64) -> f64 {
        let inv = 1.0 - t.clamp(0.0, 1.0);
        1.0 - inv * inv * inv
    }

    fn widget_center_in_motion_layer<W: IsA<gtk::Widget>>(&self, widget: &W) -> Option<(f64, f64)> {
        let imp = self.imp();
        let layer = imp.motion_layer.get();
        let rect = widget.as_ref().compute_bounds(&layer)?;
        Some((
            f64::from(rect.x()) + f64::from(rect.width()) / 2.0,
            f64::from(rect.y()) + f64::from(rect.height()) / 2.0,
        ))
    }

    fn motion_target_center(&self, target: MotionTarget) -> Option<(f64, f64)> {
        match target {
            MotionTarget::Stock => {
                self.widget_center_in_motion_layer(&self.imp().stock_picture.get())
            }
            MotionTarget::WasteTop => {
                let waste_slots = self.waste_fan_slots();
                for slot in waste_slots.into_iter().rev() {
                    if slot.is_visible() {
                        return self.widget_center_in_motion_layer(&slot);
                    }
                }
                self.widget_center_in_motion_layer(&self.imp().waste_picture.get())
            }
            MotionTarget::Foundation(index) => {
                let pictures = self.foundation_pictures();
                let picture = pictures.get(index)?;
                self.widget_center_in_motion_layer(picture)
            }
            MotionTarget::Tableau(col) => {
                if let Some(cards) = self.imp().tableau_card_pictures.borrow().get(col) {
                    if let Some(last) = cards.last() {
                        return self.widget_center_in_motion_layer(last);
                    }
                }
                let stacks = self.tableau_stacks();
                let stack = stacks.get(col)?;
                self.widget_center_in_motion_layer(stack)
            }
            MotionTarget::TableauCard { col, index } => {
                let cards = self.imp().tableau_card_pictures.borrow();
                let picture = cards.get(col)?.get(index)?;
                self.widget_center_in_motion_layer(picture)
            }
        }
    }

    pub(super) fn capture_motion_source(&self, target: MotionTarget) -> Option<(f64, f64)> {
        self.motion_target_center(target)
    }

    pub(super) fn should_play_non_drag_move_animation(&self) -> bool {
        self.imp().drag_origin.borrow().is_none()
    }

    pub(super) fn capture_tableau_landing_point(&self, col: usize) -> Option<(f64, f64)> {
        let mode = self.active_game_mode();
        let game = boundary::clone_klondike_for_automation(
            &self.imp().game.borrow(),
            mode,
            self.current_klondike_draw_mode(),
        )?;
        let len = game.tableau_len(col)?;
        if len == 0 {
            return self.capture_motion_source(MotionTarget::Tableau(col));
        }
        let last_index = len - 1;
        let last_center = self.capture_motion_source(MotionTarget::TableauCard {
            col,
            index: last_index,
        })?;
        let step = game
            .tableau_card(col, last_index)
            .map(|card| {
                if card.face_up {
                    self.imp().face_up_step.get()
                } else {
                    self.imp().face_down_step.get()
                }
            })
            .unwrap_or(self.imp().face_up_step.get());
        Some((last_center.0, last_center.1 + f64::from(step)))
    }

    pub(super) fn play_move_animation_to_point(
        &self,
        texture: gdk::Texture,
        from_center: (f64, f64),
        to_center: (f64, f64),
    ) {
        let imp = self.imp();
        let layer = imp.motion_layer.get();
        let width = texture.width().max(1);
        let height = texture.height().max(1);
        let picture = gtk::Picture::new();
        picture.set_width_request(width);
        picture.set_height_request(height);
        picture.set_can_shrink(false);
        picture.set_content_fit(gtk::ContentFit::Contain);
        picture.set_paintable(Some(&texture));
        picture.set_can_target(false);
        picture.set_sensitive(false);
        picture.set_opacity(0.92);
        picture.add_css_class("motion-fly-card");

        let start_x = from_center.0 - f64::from(width) / 2.0;
        let start_y = from_center.1 - f64::from(height) / 2.0;
        layer.put(&picture, start_x, start_y);

        let duration = Duration::from_millis(MOVE_ANIMATION_DURATION_MS);
        let started = Instant::now();
        let delta_x = to_center.0 - from_center.0;
        let delta_y = to_center.1 - from_center.1;

        glib::timeout_add_local(
            Duration::from_millis(MOVE_ANIMATION_TICK_MS),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[weak]
                picture,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let layer = window.imp().motion_layer.get();
                    let elapsed = started.elapsed();
                    let t = (elapsed.as_secs_f64() / duration.as_secs_f64()).clamp(0.0, 1.0);
                    let eased = Self::ease_out_cubic(t);
                    let x = start_x + delta_x * eased;
                    let y = start_y + delta_y * eased;
                    let opacity = 0.88 + (0.12 * eased);
                    picture.set_opacity(opacity);
                    layer.move_(&picture, x, y);

                    if t >= 1.0 {
                        picture.remove_css_class("motion-fly-card");
                        layer.remove(&picture);
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                }
            ),
        );
    }

    pub(super) fn glitched_texture_for_card_motion(&self, card: Card) -> Option<gdk::Texture> {
        let imp = self.imp();
        let deck = imp.deck.borrow();
        let deck = deck.as_ref()?;
        let width = imp.card_width.get().max(1);
        let height = imp.card_height.get().max(1);
        Some(deck.texture_for_card_scaled(card, width, height))
    }

    pub(super) fn glitched_texture_for_tableau_run_motion(
        &self,
        col: usize,
        start: usize,
    ) -> Option<gdk::Texture> {
        let imp = self.imp();
        let mode = self.active_game_mode();
        let draw_mode = self.current_klondike_draw_mode();
        let game = boundary::clone_klondike_for_automation(&imp.game.borrow(), mode, draw_mode)?;
        let deck = imp.deck.borrow();
        let deck = deck.as_ref()?;
        let width = imp.card_width.get().max(1);
        let height = imp.card_height.get().max(1);
        self.texture_for_tableau_drag_run(&game, deck, col, start, width, height)
    }
}

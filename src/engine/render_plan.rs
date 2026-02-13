use crate::engine::variant_engine::VariantCapabilities;
use crate::game::{Card, DrawMode, KlondikeGame};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RenderControls {
    pub undo_enabled: bool,
    pub redo_enabled: bool,
    pub auto_hint_enabled: bool,
    pub cyclone_enabled: bool,
    pub peek_enabled: bool,
    pub robot_enabled: bool,
    pub seed_random_enabled: bool,
    pub seed_rescue_enabled: bool,
    pub seed_winnable_enabled: bool,
    pub seed_repeat_enabled: bool,
    pub seed_go_enabled: bool,
    pub seed_combo_enabled: bool,
}

pub fn card_count_label(count: usize) -> String {
    format!("{count} cards")
}

pub fn sanitize_selected_run(
    game: &KlondikeGame,
    selected: Option<(usize, usize)>,
) -> Option<(usize, usize)> {
    let (col, start) = selected?;
    let len = game.tableau_len(col)?;
    if start >= len {
        return None;
    }
    let card = game.tableau_card(col, start)?;
    if card.face_up {
        Some((col, start))
    } else {
        None
    }
}

pub fn plan_controls(
    caps: VariantCapabilities,
    history_len: usize,
    future_len: usize,
) -> RenderControls {
    RenderControls {
        undo_enabled: caps.undo_redo && history_len > 0,
        redo_enabled: caps.undo_redo && future_len > 0,
        auto_hint_enabled: caps.autoplay,
        cyclone_enabled: caps.cyclone_shuffle,
        peek_enabled: caps.peek,
        robot_enabled: caps.robot_mode,
        seed_random_enabled: caps.seeded_deals,
        seed_rescue_enabled: caps.seeded_deals,
        seed_winnable_enabled: caps.winnability,
        seed_repeat_enabled: caps.seeded_deals,
        seed_go_enabled: caps.seeded_deals,
        seed_combo_enabled: caps.seeded_deals,
    }
}

pub fn waste_visible_count(draw_mode: DrawMode, waste_len: usize) -> usize {
    let visible_waste_cards = usize::from(draw_mode.count().clamp(1, 5));
    waste_len.min(visible_waste_cards)
}

pub fn waste_fan_step(card_width: i32) -> i32 {
    (card_width / 6).clamp(8, 22)
}

pub fn foundation_group_width(card_width: i32) -> i32 {
    (card_width * 4) + (8 * 3)
}

pub fn waste_overlay_width(card_width: i32) -> i32 {
    card_width + (waste_fan_step(card_width) * 4)
}

pub fn foundation_empty_flags(game: &KlondikeGame) -> [bool; 4] {
    [
        game.foundations()[0].is_empty(),
        game.foundations()[1].is_empty(),
        game.foundations()[2].is_empty(),
        game.foundations()[3].is_empty(),
    ]
}

pub fn tableau_stack_height(
    column: &[Card],
    card_height: i32,
    face_up_step: i32,
    face_down_step: i32,
) -> i32 {
    if column.is_empty() {
        return card_height;
    }
    let mut y = 0;
    for (card_idx, card) in column.iter().enumerate() {
        if card_idx + 1 < column.len() {
            y += if card.face_up {
                face_up_step
            } else {
                face_down_step
            };
        }
    }
    (y + card_height).max(card_height)
}

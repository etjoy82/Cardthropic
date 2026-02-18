use crate::game::KlondikeGame;

pub fn build_status_text(
    game: &KlondikeGame,
    selected: Option<(usize, usize)>,
    waste_selected: bool,
    peek_active: bool,
    engine_ready: bool,
    show_controls_hint: bool,
    mode_label: &str,
    smart_move_mode: &str,
    deck_error: Option<&str>,
    status_override: Option<&str>,
) -> String {
    if let Some(err) = deck_error {
        return format!("Card deck load failed: {err}");
    }
    if let Some(message) = status_override {
        return message.to_string();
    }
    if game.is_won() {
        return "You won! All foundations are complete.".to_string();
    }
    if let Some((col, start)) = selected {
        let amount = game.tableau_len(col).unwrap_or(0).saturating_sub(start);
        if amount > 1 {
            return format!(
                "Selected {amount} cards from T{}. Click another tableau to move this run.",
                col + 1
            );
        }
        return format!(
            "Selected tableau T{}. Click another tableau to move top card.",
            col + 1
        );
    }
    if waste_selected && game.waste_top().is_some() {
        return "Selected waste. Click a tableau to move it, or click waste again to cancel."
            .to_string();
    }
    if peek_active {
        return "Peek active: tableau face-up cards are hidden and face-down cards are revealed."
            .to_string();
    }
    if !engine_ready {
        return format!("{mode_label} mode scaffolded. Rules/engine are in progress.");
    }

    if !show_controls_hint {
        return String::new();
    }

    match smart_move_mode {
        "disabled" => {
            "Klondike controls: click columns/waste to select and move manually. Smart Move is off."
                .to_string()
        }
        "single-click" => {
            "Klondike controls: single-click cards/waste for Smart Move. Use drag-and-drop for manual runs. Keyboard: arrows move focus, Enter activates."
                .to_string()
        }
        _ => "Klondike controls: click columns to move, click waste to select, double-click cards/waste for Smart Move. Keyboard: arrows move focus, Enter activates."
            .to_string(),
    }
}

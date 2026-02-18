use crate::game::KlondikeGame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyboardTarget {
    Stock,
    Waste,
    Freecell(usize),
    Foundation(usize),
    Tableau { col: usize, start: Option<usize> },
}

pub fn move_horizontal(game: &KlondikeGame, target: KeyboardTarget, delta: i32) -> KeyboardTarget {
    let current = normalize_target(game, target);
    match current {
        KeyboardTarget::Stock => {
            if delta > 0 {
                KeyboardTarget::Waste
            } else {
                KeyboardTarget::Stock
            }
        }
        KeyboardTarget::Freecell(idx) => KeyboardTarget::Freecell(idx.min(3)),
        KeyboardTarget::Waste => {
            if delta > 0 {
                KeyboardTarget::Foundation(0)
            } else {
                KeyboardTarget::Stock
            }
        }
        KeyboardTarget::Foundation(idx) => {
            let idx = (idx as i32 + delta).clamp(0, 3) as usize;
            if idx == 0 && delta < 0 {
                KeyboardTarget::Waste
            } else {
                KeyboardTarget::Foundation(idx)
            }
        }
        KeyboardTarget::Tableau { col, start } => {
            let new_col = (col as i32 + delta).clamp(0, 6) as usize;
            let offset = tableau_offset_from_top(game, col, start);
            tableau_target_for_column(game, new_col, Some(offset))
        }
    }
}

pub fn move_vertical(game: &KlondikeGame, target: KeyboardTarget, delta: i32) -> KeyboardTarget {
    let current = normalize_target(game, target);
    match current {
        KeyboardTarget::Stock | KeyboardTarget::Waste | KeyboardTarget::Foundation(_) => {
            if delta > 0 {
                let col = match current {
                    KeyboardTarget::Stock => 0,
                    KeyboardTarget::Waste => 1,
                    KeyboardTarget::Foundation(idx) => [3_usize, 4, 5, 6][idx],
                    _ => 0,
                };
                tableau_target_for_column(game, col, Some(0))
            } else {
                current
            }
        }
        KeyboardTarget::Freecell(idx) => KeyboardTarget::Freecell(idx.min(3)),
        KeyboardTarget::Tableau { col, start } => {
            let faceups = tableau_face_up_indices(game, col);
            if delta < 0 {
                if let Some(curr) = start {
                    if let Some(pos) = faceups.iter().position(|&idx| idx == curr) {
                        if pos + 1 < faceups.len() {
                            KeyboardTarget::Tableau {
                                col,
                                start: Some(faceups[pos + 1]),
                            }
                        } else {
                            let top_idx = match col {
                                0 => 0,
                                1 => 1,
                                2 | 3 => 2,
                                4 => 3,
                                5 => 4,
                                _ => 5,
                            };
                            match top_idx {
                                0 => KeyboardTarget::Stock,
                                1 => KeyboardTarget::Waste,
                                _ => KeyboardTarget::Foundation(top_idx - 2),
                            }
                        }
                    } else {
                        tableau_target_for_column(game, col, Some(0))
                    }
                } else {
                    let top_idx = match col {
                        0 => 0,
                        1 => 1,
                        2 | 3 => 2,
                        4 => 3,
                        5 => 4,
                        _ => 5,
                    };
                    match top_idx {
                        0 => KeyboardTarget::Stock,
                        1 => KeyboardTarget::Waste,
                        _ => KeyboardTarget::Foundation(top_idx - 2),
                    }
                }
            } else if let Some(curr) = start {
                if let Some(pos) = faceups.iter().position(|&idx| idx == curr) {
                    if pos > 0 {
                        KeyboardTarget::Tableau {
                            col,
                            start: Some(faceups[pos - 1]),
                        }
                    } else {
                        KeyboardTarget::Tableau { col, start }
                    }
                } else {
                    tableau_target_for_column(game, col, Some(0))
                }
            } else {
                KeyboardTarget::Tableau { col, start: None }
            }
        }
    }
}

pub fn normalize_target(game: &KlondikeGame, target: KeyboardTarget) -> KeyboardTarget {
    match target {
        KeyboardTarget::Stock => KeyboardTarget::Stock,
        KeyboardTarget::Waste => KeyboardTarget::Waste,
        KeyboardTarget::Freecell(idx) => KeyboardTarget::Freecell(idx.min(3)),
        KeyboardTarget::Foundation(idx) => KeyboardTarget::Foundation(idx.min(3)),
        KeyboardTarget::Tableau { col, start } => {
            let col = col.min(6);
            let faceups = tableau_face_up_indices(game, col);
            if faceups.is_empty() {
                KeyboardTarget::Tableau { col, start: None }
            } else if let Some(start) = start {
                if faceups.contains(&start) {
                    KeyboardTarget::Tableau {
                        col,
                        start: Some(start),
                    }
                } else {
                    tableau_target_for_column(game, col, Some(0))
                }
            } else {
                KeyboardTarget::Tableau { col, start: None }
            }
        }
    }
}

pub fn tableau_face_up_indices(game: &KlondikeGame, col: usize) -> Vec<usize> {
    game.tableau()
        .get(col)
        .map(|pile| {
            pile.iter()
                .enumerate()
                .filter_map(|(idx, card)| if card.face_up { Some(idx) } else { None })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub fn tableau_target_for_column(
    game: &KlondikeGame,
    col: usize,
    prefer_offset_from_top: Option<usize>,
) -> KeyboardTarget {
    let faceups = tableau_face_up_indices(game, col);
    if faceups.is_empty() {
        return KeyboardTarget::Tableau { col, start: None };
    }
    let offset = prefer_offset_from_top.unwrap_or(0).min(faceups.len() - 1);
    let pos = faceups.len() - 1 - offset;
    KeyboardTarget::Tableau {
        col,
        start: Some(faceups[pos]),
    }
}

pub fn tableau_offset_from_top(game: &KlondikeGame, col: usize, start: Option<usize>) -> usize {
    let faceups = tableau_face_up_indices(game, col);
    if faceups.is_empty() {
        return 0;
    }
    let Some(start) = start else {
        return 0;
    };
    faceups
        .iter()
        .position(|&idx| idx == start)
        .map(|pos| faceups.len().saturating_sub(pos + 1))
        .unwrap_or(0)
}

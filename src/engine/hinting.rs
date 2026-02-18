use crate::engine::foundation_safety;
use crate::engine::moves::HintMove;
use crate::game::KlondikeGame;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HintNode {
    Stock,
    Waste,
    Freecell(usize),
    Foundation(usize),
    Tableau { col: usize, index: Option<usize> },
}

#[derive(Debug, Clone)]
pub struct HintSuggestion {
    pub message: String,
    pub source: Option<HintNode>,
    pub target: Option<HintNode>,
    pub hint_move: Option<HintMove>,
}

pub fn enumerate_hint_candidates(game: &KlondikeGame) -> Vec<HintSuggestion> {
    let mut candidates = Vec::new();

    if foundation_safety::can_auto_move_waste_to_foundation(game) {
        let foundation = game
            .waste_top()
            .map(|card| card.suit.foundation_index())
            .unwrap_or(0);
        candidates.push(HintSuggestion {
            message: "Hint: Move waste to foundation.".to_string(),
            source: Some(HintNode::Waste),
            target: Some(HintNode::Foundation(foundation)),
            hint_move: Some(HintMove::WasteToFoundation),
        });
    }

    for src in 0..7 {
        if !foundation_safety::can_auto_move_tableau_to_foundation(game, src) {
            continue;
        }
        let foundation = game
            .tableau_top(src)
            .map(|card| card.suit.foundation_index())
            .unwrap_or(0);
        let len = game.tableau_len(src).unwrap_or(1);
        candidates.push(HintSuggestion {
            message: format!("Hint: Move T{} top card to foundation.", src + 1),
            source: Some(HintNode::Tableau {
                col: src,
                index: len.checked_sub(1),
            }),
            target: Some(HintNode::Foundation(foundation)),
            hint_move: Some(HintMove::TableauTopToFoundation { src }),
        });
    }

    for dst in 0..7 {
        if game.can_move_waste_to_tableau(dst) {
            candidates.push(HintSuggestion {
                message: format!("Hint: Move waste card to T{}.", dst + 1),
                source: Some(HintNode::Waste),
                target: Some(HintNode::Tableau {
                    col: dst,
                    index: None,
                }),
                hint_move: Some(HintMove::WasteToTableau { dst }),
            });
        }
    }

    for src in 0..7 {
        let len = game.tableau_len(src).unwrap_or(0);
        for start in 0..len {
            for dst in 0..7 {
                if !game.can_move_tableau_run_to_tableau(src, start, dst) {
                    continue;
                }
                let amount = len.saturating_sub(start);
                candidates.push(HintSuggestion {
                    message: format!("Hint: Move {amount} card(s) T{} -> T{}.", src + 1, dst + 1),
                    source: Some(HintNode::Tableau {
                        col: src,
                        index: Some(start),
                    }),
                    target: Some(HintNode::Tableau {
                        col: dst,
                        index: None,
                    }),
                    hint_move: Some(HintMove::TableauRunToTableau { src, start, dst }),
                });
            }
        }
    }

    if game.stock_len() > 0 || game.waste_top().is_some() {
        candidates.push(HintSuggestion {
            message: "Hint: Draw from stock.".to_string(),
            source: Some(HintNode::Stock),
            target: Some(HintNode::Stock),
            hint_move: Some(HintMove::Draw),
        });
    }

    candidates
}

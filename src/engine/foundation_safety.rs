use crate::game::{Card, KlondikeGame, Suit};

pub fn can_auto_move_waste_to_foundation(game: &KlondikeGame) -> bool {
    let Some(card) = game.waste_top() else {
        return false;
    };
    game.can_move_waste_to_foundation() && is_safe_auto_foundation(game, card)
}

pub fn can_auto_move_tableau_to_foundation(game: &KlondikeGame, src: usize) -> bool {
    let Some(card) = game.tableau_top(src) else {
        return false;
    };
    game.can_move_tableau_top_to_foundation(src) && is_safe_auto_foundation(game, card)
}

pub fn is_safe_auto_foundation(game: &KlondikeGame, card: Card) -> bool {
    if card.rank <= 2 {
        return true;
    }

    match card.suit {
        Suit::Hearts | Suit::Diamonds => {
            game.foundation_top_rank(Suit::Clubs) >= card.rank - 1
                && game.foundation_top_rank(Suit::Spades) >= card.rank - 1
        }
        Suit::Clubs | Suit::Spades => {
            game.foundation_top_rank(Suit::Hearts) >= card.rank - 1
                && game.foundation_top_rank(Suit::Diamonds) >= card.rank - 1
        }
    }
}

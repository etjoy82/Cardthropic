use super::*;

impl KlondikeGame {
    pub fn encode_for_session(&self) -> String {
        let parts = [
            format!("draw={}", self.draw_mode.count()),
            format!("stock={}", encode_pile(&self.stock)),
            format!("waste={}", encode_pile(&self.waste)),
            format!("f0={}", encode_pile(&self.foundations[0])),
            format!("f1={}", encode_pile(&self.foundations[1])),
            format!("f2={}", encode_pile(&self.foundations[2])),
            format!("f3={}", encode_pile(&self.foundations[3])),
            format!("t0={}", encode_pile(&self.tableau[0])),
            format!("t1={}", encode_pile(&self.tableau[1])),
            format!("t2={}", encode_pile(&self.tableau[2])),
            format!("t3={}", encode_pile(&self.tableau[3])),
            format!("t4={}", encode_pile(&self.tableau[4])),
            format!("t5={}", encode_pile(&self.tableau[5])),
            format!("t6={}", encode_pile(&self.tableau[6])),
        ];
        parts.join(";")
    }

    pub fn decode_from_session(data: &str) -> Option<Self> {
        let mut fields = std::collections::HashMap::<&str, &str>::new();
        for part in data.split(';') {
            let (key, value) = part.split_once('=')?;
            fields.insert(key, value);
        }

        let draw_mode = DrawMode::from_count(fields.get("draw")?.parse::<u8>().ok()?)?;
        let stock = decode_pile(fields.get("stock")?)?;
        let waste = decode_pile(fields.get("waste")?)?;
        let foundations = [
            decode_pile(fields.get("f0")?)?,
            decode_pile(fields.get("f1")?)?,
            decode_pile(fields.get("f2")?)?,
            decode_pile(fields.get("f3")?)?,
        ];
        let tableau = [
            decode_pile(fields.get("t0")?)?,
            decode_pile(fields.get("t1")?)?,
            decode_pile(fields.get("t2")?)?,
            decode_pile(fields.get("t3")?)?,
            decode_pile(fields.get("t4")?)?,
            decode_pile(fields.get("t5")?)?,
            decode_pile(fields.get("t6")?)?,
        ];

        let foundations_count: usize = foundations.iter().map(Vec::len).sum();
        let tableau_count: usize = tableau.iter().map(Vec::len).sum();
        if stock.len() + waste.len() + foundations_count + tableau_count != 52 {
            return None;
        }

        Some(Self {
            draw_mode,
            stock,
            waste,
            foundations,
            tableau,
        })
    }
}

fn encode_pile(cards: &[Card]) -> String {
    if cards.is_empty() {
        return "-".to_string();
    }
    cards
        .iter()
        .map(|card| {
            let suit = match card.suit {
                Suit::Clubs => 'C',
                Suit::Diamonds => 'D',
                Suit::Hearts => 'H',
                Suit::Spades => 'S',
            };
            let face = if card.face_up { 'U' } else { 'D' };
            format!("{suit}{}{}", card.rank, face)
        })
        .collect::<Vec<_>>()
        .join(".")
}

fn decode_pile(encoded: &str) -> Option<Vec<Card>> {
    if encoded == "-" {
        return Some(Vec::new());
    }
    let mut cards = Vec::new();
    for token in encoded.split('.') {
        let mut chars = token.chars();
        let suit = match chars.next()? {
            'C' => Suit::Clubs,
            'D' => Suit::Diamonds,
            'H' => Suit::Hearts,
            'S' => Suit::Spades,
            _ => return None,
        };
        let face = match token.chars().last()? {
            'U' => true,
            'D' => false,
            _ => return None,
        };
        if token.len() < 3 {
            return None;
        }
        let rank_raw = &token[1..token.len() - 1];
        let rank = rank_raw.parse::<u8>().ok()?;
        if !(1..=13).contains(&rank) {
            return None;
        }
        cards.push(Card {
            suit,
            rank,
            face_up: face,
        });
    }
    Some(cards)
}

use super::types::ChessVariant;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChessRuleset {
    Classical,
    Atomic,
}

impl ChessRuleset {
    pub const fn for_variant(variant: ChessVariant) -> Self {
        match variant {
            ChessVariant::Standard | ChessVariant::Chess960 => Self::Classical,
            ChessVariant::Atomic => Self::Atomic,
        }
    }
}

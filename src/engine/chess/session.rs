use crate::game::{decode_fen, encode_fen, ChessPosition, ChessVariant};

pub fn encode_position(position: &ChessPosition) -> String {
    encode_fen(position)
}

pub fn decode_position(raw: &str, variant: ChessVariant) -> Option<ChessPosition> {
    decode_fen(raw, variant)
}

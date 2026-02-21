use std::str::FromStr;

use super::position::{CastlingRights, ChessPosition};
use super::types::{parse_square, square, square_name, ChessColor, ChessPiece, ChessPieceKind};
use super::ChessVariant;

pub fn encode_fen(position: &ChessPosition) -> String {
    let board = encode_board(position);
    let side_to_move = position.side_to_move().fen_char();
    let castling = encode_castling(position.castling_rights());
    let en_passant = position
        .en_passant()
        .map(square_name)
        .unwrap_or_else(|| "-".to_string());

    format!(
        "{board} {side_to_move} {castling} {en_passant} {} {}",
        position.halfmove_clock(),
        position.fullmove_number()
    )
}

pub fn decode_fen(raw: &str, variant: ChessVariant) -> Option<ChessPosition> {
    let fields: Vec<&str> = raw.split_whitespace().collect();
    if fields.len() != 6 {
        return None;
    }

    let mut position = ChessPosition::empty(variant);
    position.clear_board();
    decode_board(fields[0], &mut position)?;
    position.set_side_to_move(ChessColor::from_fen_char(fields[1].chars().next()?)?);
    position.set_castling_rights(decode_castling(fields[2])?);
    position.set_en_passant(if fields[3] == "-" {
        None
    } else {
        parse_square(fields[3])
    });
    position.set_halfmove_clock(u16::from_str(fields[4]).ok()?);
    position.set_fullmove_number(u16::from_str(fields[5]).ok()?);

    let white_back_rank = collect_back_rank(&position, ChessColor::White);
    let black_back_rank = collect_back_rank(&position, ChessColor::Black);
    position.set_back_ranks(white_back_rank, black_back_rank);
    Some(position)
}

fn encode_board(position: &ChessPosition) -> String {
    let mut ranks = Vec::with_capacity(8);
    for rank in (0..8_u8).rev() {
        let mut segment = String::new();
        let mut empty_run = 0_u8;
        for file in 0..8_u8 {
            let sq = square(file, rank).expect("rank/file in range");
            match position.piece_at(sq) {
                Some(piece) => {
                    if empty_run > 0 {
                        segment.push(char::from(b'0' + empty_run));
                        empty_run = 0;
                    }
                    segment.push(piece.kind.fen_char(piece.color));
                }
                None => {
                    empty_run += 1;
                }
            }
        }
        if empty_run > 0 {
            segment.push(char::from(b'0' + empty_run));
        }
        ranks.push(segment);
    }
    ranks.join("/")
}

fn decode_board(board: &str, position: &mut ChessPosition) -> Option<()> {
    let ranks: Vec<&str> = board.split('/').collect();
    if ranks.len() != 8 {
        return None;
    }

    for (fen_rank_idx, fen_rank) in ranks.iter().enumerate() {
        let rank = 7_u8.saturating_sub(fen_rank_idx as u8);
        let mut file = 0_u8;
        for ch in fen_rank.chars() {
            if ch.is_ascii_digit() {
                let skip = ch.to_digit(10)? as u8;
                if skip == 0 || file + skip > 8 {
                    return None;
                }
                file += skip;
                continue;
            }
            let (kind, color) = ChessPieceKind::from_fen_char(ch)?;
            let sq = square(file, rank)?;
            let _ = position.set_piece(sq, Some(ChessPiece { color, kind }));
            file += 1;
        }
        if file != 8 {
            return None;
        }
    }

    Some(())
}

fn encode_castling(rights: CastlingRights) -> String {
    if !rights.has_any() {
        return "-".to_string();
    }

    let mut text = String::new();
    if rights.white_king_side {
        text.push('K');
    }
    if rights.white_queen_side {
        text.push('Q');
    }
    if rights.black_king_side {
        text.push('k');
    }
    if rights.black_queen_side {
        text.push('q');
    }
    text
}

fn decode_castling(field: &str) -> Option<CastlingRights> {
    if field == "-" {
        return Some(CastlingRights::none());
    }

    let mut rights = CastlingRights::none();
    for ch in field.chars() {
        match ch {
            'K' => rights.white_king_side = true,
            'Q' => rights.white_queen_side = true,
            'k' => rights.black_king_side = true,
            'q' => rights.black_queen_side = true,
            _ => return None,
        }
    }
    Some(rights)
}

fn collect_back_rank(position: &ChessPosition, color: ChessColor) -> [ChessPieceKind; 8] {
    let rank = match color {
        ChessColor::White => 0,
        ChessColor::Black => 7,
    };
    let mut pieces = [ChessPieceKind::Pawn; 8];
    for file in 0..8_u8 {
        let sq = square(file, rank).expect("rank/file in range");
        pieces[file as usize] = position
            .piece_at(sq)
            .filter(|piece| piece.color == color)
            .map(|piece| piece.kind)
            .unwrap_or(ChessPieceKind::Pawn);
    }
    pieces
}

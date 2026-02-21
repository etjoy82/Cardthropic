use crate::game::{file_of, ChessColor, ChessPieceKind, ChessPosition};

#[derive(Default, Clone, Copy)]
struct PawnFileStats {
    counts: [i32; 8],
}

impl PawnFileStats {
    fn count(&self, file: i32) -> i32 {
        if !(0..8).contains(&file) {
            0
        } else {
            self.counts[file as usize]
        }
    }
}

pub fn white_minus_black(position: &ChessPosition) -> i32 {
    let mut white = PawnFileStats::default();
    let mut black = PawnFileStats::default();

    for sq in 0_u8..64 {
        let Some(piece) = position.piece_at(sq) else {
            continue;
        };
        if piece.kind != ChessPieceKind::Pawn {
            continue;
        }
        let file_idx = usize::from(file_of(sq));
        match piece.color {
            ChessColor::White => white.counts[file_idx] += 1,
            ChessColor::Black => black.counts[file_idx] += 1,
        }
    }

    let white_penalty = pawn_structure_penalty(&white);
    let black_penalty = pawn_structure_penalty(&black);
    black_penalty - white_penalty
}

fn pawn_structure_penalty(stats: &PawnFileStats) -> i32 {
    let mut penalty = 0_i32;
    for file in 0_i32..8_i32 {
        let count = stats.count(file);
        if count <= 0 {
            continue;
        }

        if count > 1 {
            penalty += (count - 1) * 12;
        }

        let left = stats.count(file - 1);
        let right = stats.count(file + 1);
        if left == 0 && right == 0 {
            penalty += count * 8;
        }
    }
    penalty
}

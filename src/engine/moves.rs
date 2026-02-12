use crate::game::{DrawResult, KlondikeGame, SolverMove};

#[derive(Debug, Clone, Copy)]
pub enum HintMove {
    WasteToFoundation,
    TableauTopToFoundation {
        src: usize,
    },
    WasteToTableau {
        dst: usize,
    },
    TableauRunToTableau {
        src: usize,
        start: usize,
        dst: usize,
    },
    Draw,
}

pub fn apply_hint_move_to_game(game: &mut KlondikeGame, hint_move: HintMove) -> bool {
    match hint_move {
        HintMove::WasteToFoundation => game.move_waste_to_foundation(),
        HintMove::TableauTopToFoundation { src } => game.move_tableau_top_to_foundation(src),
        HintMove::WasteToTableau { dst } => game.move_waste_to_tableau(dst),
        HintMove::TableauRunToTableau { src, start, dst } => {
            game.move_tableau_run_to_tableau(src, start, dst)
        }
        HintMove::Draw => game.draw_or_recycle() != DrawResult::NoOp,
    }
}

pub fn map_solver_move_to_hint_move(solver_move: SolverMove) -> HintMove {
    match solver_move {
        SolverMove::Draw => HintMove::Draw,
        SolverMove::WasteToFoundation => HintMove::WasteToFoundation,
        SolverMove::WasteToTableau { dst } => HintMove::WasteToTableau { dst },
        SolverMove::TableauTopToFoundation { src } => HintMove::TableauTopToFoundation { src },
        SolverMove::TableauRunToTableau { src, start, dst } => {
            HintMove::TableauRunToTableau { src, start, dst }
        }
    }
}

pub fn map_solver_line_to_hint_line(
    start: &KlondikeGame,
    solver_line: &[SolverMove],
) -> Option<Vec<HintMove>> {
    let mut game = start.clone();
    let mut line = Vec::with_capacity(solver_line.len());
    for solver_move in solver_line {
        let hint_move = map_solver_move_to_hint_move(*solver_move);
        if !apply_hint_move_to_game(&mut game, hint_move) {
            return None;
        }
        line.push(hint_move);
    }
    Some(line)
}

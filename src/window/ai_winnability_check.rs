use super::*;
use crate::engine::chess::ai::{self as chess_ai, api::SearchTermination, AiConfig};
use crate::engine::seed_ops;
use crate::game::{
    is_in_check, legal_moves, square_name, ChessColor, ChessPieceKind, ChessPosition, Square,
};

const SEED_WINNABILITY_TIMEOUT_SECS: u32 = 300;
const SEED_WINNABILITY_MEMORY_HEADROOM_MIB: u64 = 512;
const SEED_WINNABILITY_MEMORY_MAX_MIB: u64 = 1024;

impl CardthropicWindow {
    pub(super) fn cancel_seed_winnable_check(&self, status: Option<&str>) {
        let imp = self.imp();
        if !imp.seed_check_running.get() {
            return;
        }

        imp.seed_check_running.set(false);
        imp.seed_check_generation
            .set(imp.seed_check_generation.get().wrapping_add(1));
        if let Some(cancel_flag) = imp.seed_check_cancel.borrow_mut().take() {
            cancel_flag.store(true, Ordering::Relaxed);
        }
        if let Some(source_id) = imp.seed_check_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        imp.seed_winnable_button
            .set_label(&self.seed_winnable_idle_button_label());
        imp.seed_check_memory_guard_triggered.set(false);
        imp.seed_check_memory_limit_mib.set(0);
        self.trim_process_memory_if_supported();

        if let Some(message) = status {
            *imp.status_override.borrow_mut() = Some(message.to_string());
            self.render();
        }
    }

    pub(super) fn finish_seed_winnable_check(&self, generation: u64) -> bool {
        let imp = self.imp();
        if !imp.seed_check_running.get() || imp.seed_check_generation.get() != generation {
            return false;
        }

        imp.seed_check_running.set(false);
        imp.seed_check_cancel.borrow_mut().take();
        if let Some(source_id) = imp.seed_check_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        imp.seed_winnable_button
            .set_label(&self.seed_winnable_idle_button_label());
        imp.seed_check_memory_guard_triggered.set(false);
        imp.seed_check_memory_limit_mib.set(0);
        self.trim_process_memory_if_supported();
        true
    }

    pub(super) fn toggle_seed_winnable_check(&self) {
        if self.imp().chess_mode_active.get() {
            self.toggle_chess_w_question_analysis();
            return;
        }
        if !self.guard_mode_engine("Winnability analysis") {
            return;
        }
        let mode = self.active_game_mode();
        if self.imp().seed_check_running.get() {
            let cancel_message = if mode == GameMode::Spider {
                let suit_count = self.current_spider_suit_mode().suit_count();
                format!("Winnability check canceled (Spider {suit_count}-suit).")
            } else if mode == GameMode::Freecell {
                "Winnability check canceled (FreeCell).".to_string()
            } else {
                seed_ops::msg_winnability_check_canceled(self.current_klondike_draw_mode().count())
            };
            self.cancel_seed_winnable_check(Some(cancel_message.as_str()));
            return;
        }

        self.clear_seed_entry_feedback();
        let seed = match self.seed_from_controls_or_random() {
            Ok(seed) => seed,
            Err(message) => {
                if let Some(entry) = self.seed_text_entry() {
                    entry.add_css_class("error");
                }
                *self.imp().status_override.borrow_mut() = Some(message);
                self.render();
                return;
            }
        };

        let imp = self.imp();
        imp.seed_check_running.set(true);
        let generation = imp.seed_check_generation.get().wrapping_add(1);
        imp.seed_check_generation.set(generation);
        imp.seed_check_seconds.set(1);
        imp.seed_check_memory_guard_triggered.set(false);
        let baseline_mib = self.current_memory_mib();
        let memory_limit_mib = baseline_mib
            .map(|m| m.saturating_add(SEED_WINNABILITY_MEMORY_HEADROOM_MIB))
            .unwrap_or(SEED_WINNABILITY_MEMORY_MAX_MIB)
            .min(SEED_WINNABILITY_MEMORY_MAX_MIB);
        imp.seed_check_memory_limit_mib.set(memory_limit_mib);
        imp.seed_winnable_button
            .set_label(&self.seed_winnable_progress_button_label("Checking", 1));
        if let Some(source_id) = imp.seed_check_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }

        let tick = glib::timeout_add_seconds_local(
            1,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let imp = window.imp();
                    if !imp.seed_check_running.get()
                        || imp.seed_check_generation.get() != generation
                    {
                        return glib::ControlFlow::Break;
                    }

                    let next = imp.seed_check_seconds.get().saturating_add(1);
                    imp.seed_check_seconds.set(next);
                    imp.seed_winnable_button
                        .set_label(&window.seed_winnable_progress_button_label("Checking", next));
                    if window
                        .current_memory_mib()
                        .is_some_and(|mib| mib > imp.seed_check_memory_limit_mib.get())
                    {
                        imp.seed_check_memory_guard_triggered.set(true);
                        if let Some(cancel_flag) = imp.seed_check_cancel.borrow().as_ref() {
                            cancel_flag.store(true, Ordering::Relaxed);
                        }
                        imp.seed_winnable_button
                            .set_label(&window.seed_winnable_stopping_button_label());
                        return glib::ControlFlow::Continue;
                    }
                    if next >= SEED_WINNABILITY_TIMEOUT_SECS {
                        if let Some(cancel_flag) = imp.seed_check_cancel.borrow().as_ref() {
                            cancel_flag.store(true, Ordering::Relaxed);
                        }
                        imp.seed_winnable_button
                            .set_label(&window.seed_winnable_stopping_button_label());
                        return glib::ControlFlow::Continue;
                    }
                    glib::ControlFlow::Continue
                }
            ),
        );
        *self.imp().seed_check_timer.borrow_mut() = Some(tick);

        let cancel_flag = Arc::new(AtomicBool::new(false));
        *self.imp().seed_check_cancel.borrow_mut() = Some(Arc::clone(&cancel_flag));

        let (sender, receiver) = mpsc::channel::<Option<winnability::SeedWinnabilityCheckResult>>();
        let draw_mode = self.current_klondike_draw_mode();
        let deal_count = draw_mode.count();
        let spider_suit_mode = self.current_spider_suit_mode();
        let spider_suit_count = spider_suit_mode.suit_count();
        let freecell_card_count_mode = self.current_freecell_card_count_mode();
        let freecell_card_count = freecell_card_count_mode.card_count();
        let profile = self.automation_profile();
        *self.imp().status_override.borrow_mut() = Some(match mode {
            GameMode::Spider => format!(
                "W? checking seed {seed} for Spider {spider_suit_count}-suit (up to {}s)...",
                SEED_WINNABILITY_TIMEOUT_SECS
            ),
            GameMode::Freecell => format!(
                "W? checking seed {seed} for FreeCell {freecell_card_count} (up to {}s)...",
                SEED_WINNABILITY_TIMEOUT_SECS
            ),
            GameMode::Klondike => format!(
                "W? checking seed {seed} for Deal {deal_count} (up to {}s)...",
                SEED_WINNABILITY_TIMEOUT_SECS
            ),
        });
        self.render();
        thread::spawn(move || {
            let result = if mode == GameMode::Spider {
                winnability::is_spider_seed_winnable(
                    seed,
                    spider_suit_mode,
                    profile.dialog_seed_guided_budget,
                    profile.dialog_seed_exhaustive_budget,
                    &cancel_flag,
                )
            } else if mode == GameMode::Freecell {
                winnability::is_freecell_seed_winnable(
                    seed,
                    freecell_card_count_mode,
                    profile.dialog_seed_guided_budget,
                    profile.dialog_seed_exhaustive_budget,
                    &cancel_flag,
                )
            } else {
                winnability::is_seed_winnable(
                    seed,
                    draw_mode,
                    profile.dialog_seed_guided_budget,
                    profile.dialog_seed_exhaustive_budget,
                    &cancel_flag,
                )
            };
            let _ = sender.send(result);
        });

        glib::timeout_add_local(
            Duration::from_millis(40),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    if !window.imp().seed_check_running.get()
                        || window.imp().seed_check_generation.get() != generation
                    {
                        return glib::ControlFlow::Break;
                    }

                    match receiver.try_recv() {
                        Ok(Some(result)) => {
                            if !window.finish_seed_winnable_check(generation) {
                                return glib::ControlFlow::Break;
                            }

                            let current_seed =
                                seed_ops::parse_seed_input(&window.seed_input_text())
                                    .ok()
                                    .flatten();
                            if current_seed != Some(seed) {
                                return glib::ControlFlow::Break;
                            }

                            if result.canceled {
                                let memory_guarded =
                                    window.imp().seed_check_memory_guard_triggered.get();
                                let memory_limit = window.imp().seed_check_memory_limit_mib.get();
                                let message = if memory_guarded {
                                    match mode {
                                        GameMode::Spider => format!(
                                            "Winnability check stopped by memory guard at ~{} MiB (Spider {spider_suit_count}-suit): solver found no winning line within {} iterations.",
                                            memory_limit, result.iterations
                                        ),
                                        GameMode::Freecell => format!(
                                            "Winnability check stopped by memory guard at ~{} MiB (FreeCell {freecell_card_count}): solver found no winning line within {} iterations.",
                                            memory_limit, result.iterations
                                        ),
                                        GameMode::Klondike => format!(
                                            "Winnability check stopped by memory guard at ~{} MiB (Deal {deal_count}): solver found no winning line within {} iterations.",
                                            memory_limit, result.iterations
                                        ),
                                    }
                                } else {
                                    match mode {
                                        GameMode::Spider => format!(
                                            "Winnability check timed out after {}s (Spider {spider_suit_count}-suit): solver found no winning line within {} iterations.",
                                            SEED_WINNABILITY_TIMEOUT_SECS, result.iterations
                                        ),
                                        GameMode::Freecell => format!(
                                            "Winnability check timed out after {}s (FreeCell {freecell_card_count}): solver found no winning line within {} iterations.",
                                            SEED_WINNABILITY_TIMEOUT_SECS, result.iterations
                                        ),
                                        GameMode::Klondike => seed_ops::msg_winnability_check_timed_out(
                                            deal_count,
                                            SEED_WINNABILITY_TIMEOUT_SECS,
                                            result.iterations,
                                        ),
                                    }
                                };
                                *window.imp().status_override.borrow_mut() = Some(message);
                                window.render();
                                return glib::ControlFlow::Break;
                            }

                            window.clear_seed_entry_feedback();
                            if result.winnable {
                                if let Some(entry) = window.seed_text_entry() {
                                    entry.add_css_class("seed-winnable");
                                }
                                if mode == GameMode::Freecell {
                                    if window.imp().move_count.get() == 0 {
                                        if let Some(line) = result.freecell_line.clone() {
                                            window
                                                .arm_robot_freecell_solver_anchor_for_current_state(
                                                    line,
                                                );
                                        }
                                    }
                                } else if let Some(line) = result.hint_line.clone().or_else(|| {
                                    result.solver_line.as_ref().and_then(|line| {
                                        let game = window.imp().game.borrow().clone();
                                        map_solver_line_to_hint_line(&game, line.as_slice())
                                    })
                                }) {
                                    window.arm_robot_solver_anchor_for_current_state(line);
                                }
                                let moves = result.moves_to_win.unwrap_or(0);
                                let message = match mode {
                                    GameMode::Spider => format!(
                                        "Seed {seed} is winnable for Spider {spider_suit_count}-suit from a fresh deal (solver line: {moves} moves, {} iterations). Use Robot as first action to see win.",
                                        result.iterations
                                    ),
                                    GameMode::Freecell if window.imp().move_count.get() == 0 => format!(
                                        "Seed {seed} is winnable for FreeCell {freecell_card_count} from a fresh deal (solver line: {moves} moves, {} iterations). Use Robot as first action to see win.",
                                        result.iterations
                                    ),
                                    GameMode::Freecell => format!(
                                        "Seed {seed} is winnable for FreeCell {freecell_card_count} from a fresh deal (solver line: {moves} moves, {} iterations). Start a fresh deal and use Robot as first action to see win.",
                                        result.iterations
                                    ),
                                    GameMode::Klondike => seed_ops::msg_seed_winnable(
                                        seed,
                                        deal_count,
                                        moves,
                                        result.iterations,
                                    ),
                                };
                                *window.imp().status_override.borrow_mut() = Some(message);
                            } else {
                                if let Some(entry) = window.seed_text_entry() {
                                    entry.add_css_class("seed-unwinnable");
                                }
                                let message = match mode {
                                    GameMode::Spider if result.hit_state_limit => format!(
                                        "Seed {seed} not proven winnable for Spider {spider_suit_count}-suit from a fresh deal ({} iterations, limits hit).",
                                        result.iterations
                                    ),
                                    GameMode::Spider => format!(
                                        "Seed {seed}: solver found no winning line for Spider {spider_suit_count}-suit from a fresh deal ({} iterations).",
                                        result.iterations
                                    ),
                                    GameMode::Freecell if result.hit_state_limit => format!(
                                        "Seed {seed} not proven winnable for FreeCell {freecell_card_count} from a fresh deal ({} iterations, limits hit).",
                                        result.iterations
                                    ),
                                    GameMode::Freecell => format!(
                                        "Seed {seed}: solver found no winning line for FreeCell {freecell_card_count} from a fresh deal ({} iterations).",
                                        result.iterations
                                    ),
                                    GameMode::Klondike if result.hit_state_limit => {
                                        seed_ops::msg_seed_unwinnable_limited(
                                            seed,
                                            deal_count,
                                            result.iterations,
                                        )
                                    }
                                    GameMode::Klondike => {
                                        seed_ops::msg_seed_unwinnable(
                                            seed,
                                            deal_count,
                                            result.iterations,
                                        )
                                    }
                                };
                                *window.imp().status_override.borrow_mut() = Some(message);
                            }
                            window.render();
                            glib::ControlFlow::Break
                        }
                        Ok(None) => {
                            window.finish_seed_winnable_check(generation);
                            glib::ControlFlow::Break
                        }
                        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            if window.finish_seed_winnable_check(generation) {
                                let message = match mode {
                                    GameMode::Spider => format!(
                                        "Winnability check stopped unexpectedly (Spider {spider_suit_count}-suit)."
                                    ),
                                    GameMode::Freecell => {
                                        format!(
                                            "Winnability check stopped unexpectedly (FreeCell {freecell_card_count})."
                                        )
                                    }
                                    GameMode::Klondike => {
                                        seed_ops::msg_winnability_check_stopped_unexpectedly(
                                            deal_count,
                                        )
                                    }
                                };
                                *window.imp().status_override.borrow_mut() = Some(message);
                                window.render();
                            }
                            glib::ControlFlow::Break
                        }
                    }
                }
            ),
        );
    }

    fn toggle_chess_w_question_analysis(&self) {
        if self.imp().seed_check_running.get() {
            self.cancel_seed_winnable_check(Some("W? Chess analysis canceled."));
            return;
        }
        self.run_chess_w_question_analysis();
    }

    fn run_chess_w_question_analysis(&self) {
        if !self.guard_mode_engine("W? Chess analysis") {
            return;
        }
        let imp = self.imp();
        let position = imp.chess_position.borrow().clone();
        let side_to_move = position.side_to_move();
        let side_to_move_label = chess_color_label(side_to_move);
        let analysis_anchor = chess_analysis_anchor_label(
            imp.move_count.get(),
            side_to_move,
            imp.chess_last_move_from.get(),
            imp.chess_last_move_to.get(),
        );
        let analysis_prefix = format!("W? [Based on {analysis_anchor}]");
        let captured_by_white = format_captured_piece_counts(
            captured_against_side(&position, ChessColor::Black),
            ChessColor::Black,
        );
        let captured_by_black = format_captured_piece_counts(
            captured_against_side(&position, ChessColor::White),
            ChessColor::White,
        );

        let next_moves = legal_moves(&position);
        if next_moves.is_empty() {
            let status = if is_in_check(&position, side_to_move) {
                format!(
                    "{analysis_prefix} Chess analysis ({side_to_move_label} to move): checkmate on board ({} already won). Captured by White: {captured_by_white}. Captured by Black: {captured_by_black}.",
                    chess_color_label(side_to_move.opposite())
                )
            } else {
                format!(
                    "{analysis_prefix} Chess analysis ({side_to_move_label} to move): draw by stalemate. Captured by White: {captured_by_white}. Captured by Black: {captured_by_black}."
                )
            };
            *self.imp().status_override.borrow_mut() = Some(status);
            self.render();
            return;
        }

        let ai_limits = self.chess_w_question_ai_search_limits();
        let strength_label = self.chess_w_question_ai_strength_label();
        let started_elapsed_seconds = imp.elapsed_seconds.get();
        imp.seed_check_running.set(true);
        let generation = imp.seed_check_generation.get().wrapping_add(1);
        imp.seed_check_generation.set(generation);
        imp.seed_check_seconds.set(1);
        imp.seed_check_memory_guard_triggered.set(false);
        imp.seed_check_memory_limit_mib.set(0);
        imp.seed_winnable_button
            .set_label(&self.seed_winnable_progress_button_label("Analyzing", 1));
        if let Some(source_id) = imp.seed_check_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
        let tick = glib::timeout_add_seconds_local(
            1,
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let imp = window.imp();
                    if !imp.seed_check_running.get()
                        || imp.seed_check_generation.get() != generation
                    {
                        return glib::ControlFlow::Break;
                    }
                    let next = imp.seed_check_seconds.get().saturating_add(1);
                    imp.seed_check_seconds.set(next);
                    imp.seed_winnable_button
                        .set_label(&window.seed_winnable_progress_button_label("Analyzing", next));
                    glib::ControlFlow::Continue
                }
            ),
        );
        *self.imp().seed_check_timer.borrow_mut() = Some(tick);

        let cancel_flag = Arc::new(AtomicBool::new(false));
        *self.imp().seed_check_cancel.borrow_mut() = Some(Arc::clone(&cancel_flag));
        let will_finish_suffix =
            self.chess_think_will_finish_suffix(started_elapsed_seconds, ai_limits.time_budget_ms);

        *self.imp().status_override.borrow_mut() = Some(format!(
            "{analysis_prefix} Chess analysis started ({side_to_move_label} to move, {strength_label}, ply={}, time={}, nodes={}).{} Click W? again to cancel.",
            ai_limits.max_depth,
            Self::chess_time_budget_seconds_label(ai_limits.time_budget_ms),
            ai_limits.node_budget,
            will_finish_suffix,
        ));
        self.render();

        let (sender, receiver) = mpsc::channel::<chess_ai::SearchResult>();
        let position_for_search = position.clone();
        thread::spawn(move || {
            let result = chess_ai::search::iterative::search(
                &position_for_search,
                ai_limits,
                AiConfig::default(),
                Some(cancel_flag.as_ref()),
            );
            let _ = sender.send(result);
        });

        glib::timeout_add_local(
            Duration::from_millis(40),
            glib::clone!(
                #[weak(rename_to = window)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let imp = window.imp();
                    if !imp.seed_check_running.get()
                        || imp.seed_check_generation.get() != generation
                    {
                        return glib::ControlFlow::Break;
                    }

                    match receiver.try_recv() {
                        Ok(result) => {
                            if !window.finish_seed_winnable_check(generation) {
                                return glib::ControlFlow::Break;
                            }

                            let status = if matches!(
                                result.termination,
                                SearchTermination::Canceled
                            ) {
                                "W? Chess analysis canceled.".to_string()
                            } else {
                                let score_white_cp = match side_to_move {
                                    ChessColor::White => result.best_score_cp,
                                    ChessColor::Black => -result.best_score_cp,
                                };
                                let verdict = chess_eval_advantage_label(score_white_cp);
                                let score_pawns = score_white_cp as f32 / 100.0;
                                format!(
                                    "{analysis_prefix} Chess analysis ({side_to_move_label} to move): {verdict} ({score_pawns:+.2} for White, ply={}, time={}, nodes={}). Captured by White: {captured_by_white}. Captured by Black: {captured_by_black}.",
                                    ai_limits.max_depth,
                                    Self::chess_time_budget_seconds_label(ai_limits.time_budget_ms),
                                    ai_limits.node_budget,
                                )
                            };

                            *window.imp().status_override.borrow_mut() = Some(status);
                            window.render();
                            glib::ControlFlow::Break
                        }
                        Err(mpsc::TryRecvError::Empty) => glib::ControlFlow::Continue,
                        Err(mpsc::TryRecvError::Disconnected) => {
                            if window.finish_seed_winnable_check(generation) {
                                *window.imp().status_override.borrow_mut() =
                                    Some("W? Chess analysis stopped unexpectedly.".to_string());
                                window.render();
                            }
                            glib::ControlFlow::Break
                        }
                    }
                }
            ),
        );
    }
}

#[derive(Default, Clone, Copy)]
struct ChessPieceCounts {
    pawns: u8,
    knights: u8,
    bishops: u8,
    rooks: u8,
    queens: u8,
}

fn piece_counts_for_side(position: &ChessPosition, side: ChessColor) -> ChessPieceCounts {
    let mut counts = ChessPieceCounts::default();
    for piece in position.board().iter().flatten() {
        if piece.color != side {
            continue;
        }
        match piece.kind {
            ChessPieceKind::Pawn => counts.pawns = counts.pawns.saturating_add(1),
            ChessPieceKind::Knight => counts.knights = counts.knights.saturating_add(1),
            ChessPieceKind::Bishop => counts.bishops = counts.bishops.saturating_add(1),
            ChessPieceKind::Rook => counts.rooks = counts.rooks.saturating_add(1),
            ChessPieceKind::Queen => counts.queens = counts.queens.saturating_add(1),
            ChessPieceKind::King => {}
        }
    }
    counts
}

fn captured_against_side(position: &ChessPosition, side: ChessColor) -> ChessPieceCounts {
    let on_board = piece_counts_for_side(position, side);
    ChessPieceCounts {
        pawns: 8_u8.saturating_sub(on_board.pawns),
        knights: 2_u8.saturating_sub(on_board.knights),
        bishops: 2_u8.saturating_sub(on_board.bishops),
        rooks: 2_u8.saturating_sub(on_board.rooks),
        queens: 1_u8.saturating_sub(on_board.queens),
    }
}

fn piece_unicode(color: ChessColor, kind: ChessPieceKind) -> &'static str {
    match (color, kind) {
        (ChessColor::White, ChessPieceKind::King) => "♔",
        (ChessColor::White, ChessPieceKind::Queen) => "♕",
        (ChessColor::White, ChessPieceKind::Rook) => "♖",
        (ChessColor::White, ChessPieceKind::Bishop) => "♗",
        (ChessColor::White, ChessPieceKind::Knight) => "♘",
        (ChessColor::White, ChessPieceKind::Pawn) => "♙",
        (ChessColor::Black, ChessPieceKind::King) => "♚",
        (ChessColor::Black, ChessPieceKind::Queen) => "♛",
        (ChessColor::Black, ChessPieceKind::Rook) => "♜",
        (ChessColor::Black, ChessPieceKind::Bishop) => "♝",
        (ChessColor::Black, ChessPieceKind::Knight) => "♞",
        (ChessColor::Black, ChessPieceKind::Pawn) => "♟",
    }
}

fn push_capture_glyph(parts: &mut Vec<String>, count: u8, color: ChessColor, kind: ChessPieceKind) {
    if count == 0 {
        return;
    }
    let glyph = piece_unicode(color, kind);
    if count == 1 {
        parts.push(glyph.to_string());
    } else {
        parts.push(format!("{glyph}x{count}"));
    }
}

fn format_captured_piece_counts(captured: ChessPieceCounts, captured_color: ChessColor) -> String {
    let mut parts = Vec::new();
    push_capture_glyph(
        &mut parts,
        captured.queens,
        captured_color,
        ChessPieceKind::Queen,
    );
    push_capture_glyph(
        &mut parts,
        captured.rooks,
        captured_color,
        ChessPieceKind::Rook,
    );
    push_capture_glyph(
        &mut parts,
        captured.bishops,
        captured_color,
        ChessPieceKind::Bishop,
    );
    push_capture_glyph(
        &mut parts,
        captured.knights,
        captured_color,
        ChessPieceKind::Knight,
    );
    push_capture_glyph(
        &mut parts,
        captured.pawns,
        captured_color,
        ChessPieceKind::Pawn,
    );
    if parts.is_empty() {
        "—".to_string()
    } else {
        parts.join(", ")
    }
}

fn chess_eval_advantage_label(score_white_cp: i32) -> &'static str {
    if score_white_cp >= 250 {
        "White is winning"
    } else if score_white_cp >= 70 {
        "White is better"
    } else if score_white_cp <= -250 {
        "Black is winning"
    } else if score_white_cp <= -70 {
        "Black is better"
    } else {
        "Position is roughly equal"
    }
}

fn chess_analysis_anchor_label(
    move_count: u32,
    side_to_move: ChessColor,
    last_move_from: Option<Square>,
    last_move_to: Option<Square>,
) -> String {
    if move_count == 0 {
        return "opening position before 1. White".to_string();
    }
    let fullmove = move_count.saturating_sub(1) / 2 + 1;
    let mover = side_to_move.opposite();
    let move_marker = match mover {
        ChessColor::White => format!("{fullmove}."),
        ChessColor::Black => format!("{fullmove}..."),
    };
    if let (Some(from), Some(to)) = (last_move_from, last_move_to) {
        format!(
            "after {move_marker} {} {} -> {}",
            chess_color_label(mover),
            square_name(from),
            square_name(to),
        )
    } else {
        format!("after {move_marker} {}", chess_color_label(mover))
    }
}

fn chess_color_label(color: ChessColor) -> &'static str {
    match color {
        ChessColor::White => "White",
        ChessColor::Black => "Black",
    }
}

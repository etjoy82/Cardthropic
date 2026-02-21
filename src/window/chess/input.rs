use crate::engine::chess::ai::{self as chess_ai, AiConfig};
use crate::engine::chess::boundary as chess_boundary;
use crate::engine::chess::commands::ChessCommand;
use crate::game::{
    file_of, is_in_check, legal_moves, rank_of, square, square_name, terminal_state, ChessColor,
    ChessMove, ChessPiece, ChessPieceKind, ChessPosition, ChessTerminalState, Square,
};
use crate::window::types::ChessAiPendingKind;
use crate::CardthropicWindow;
use adw::subclass::prelude::ObjectSubclassIsExt;
use gtk::{gdk, glib};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::Duration;

impl CardthropicWindow {
    pub(in crate::window) fn play_chess_ai_hint_move(&self) -> bool {
        self.play_chess_ai_hint_move_internal(true)
    }

    pub(in crate::window) fn play_chess_ai_hint_move_single(&self) -> bool {
        self.play_chess_ai_hint_move_internal(false)
    }

    fn play_chess_ai_hint_move_internal(&self, include_opponent_auto_response: bool) -> bool {
        self.ensure_chess_opening_side_to_move_is_white();
        self.queue_chess_ai_search(ChessAiPendingKind::Wand {
            include_opponent_auto_response,
        })
    }

    fn ensure_chess_opening_side_to_move_is_white(&self) {
        let imp = self.imp();
        if imp.move_count.get() != 0 {
            return;
        }
        let mut position = imp.chess_position.borrow_mut();
        if position.side_to_move() == ChessColor::Black {
            position.set_side_to_move(ChessColor::White);
            self.append_status_history_only(
                "chess_turn_guard: opening side-to-move corrected from black to white",
            );
        }
    }

    pub(in crate::window) fn play_chess_ai_robot_move(&self) -> bool {
        let side_to_move = self.imp().chess_position.borrow().side_to_move();
        self.queue_chess_ai_search(ChessAiPendingKind::Robot { side_to_move })
    }

    pub(in crate::window) fn has_pending_chess_ai_search(&self) -> bool {
        self.imp().chess_ai_pending_search.borrow().is_some()
    }

    pub(in crate::window) fn cancel_pending_chess_ai_search(&self) {
        let imp = self.imp();
        let pending_kind = imp.chess_ai_pending_kind.get();
        let pending_limits = imp.chess_ai_pending_limits.get();
        let pending_legal_moves = imp.chess_ai_pending_legal_moves.get();
        let started_mono_us = imp.chess_ai_pending_started_mono_us.get();
        let pending_hash = imp.chess_ai_pending_position_hash.get();
        let pending_polls = imp.chess_ai_pending_poll_count.get();
        if let Some(search) = imp.chess_ai_pending_search.borrow_mut().take() {
            search.cancel();
            if imp.robot_debug_enabled.get() {
                if let Some(kind) = pending_kind {
                    let elapsed_ms = Self::chess_ai_elapsed_ms(started_mono_us);
                    let limits = pending_limits
                        .unwrap_or_else(|| self.chess_ai_search_limits_for_kind(kind));
                    self.append_status_history_only(&format!(
                        "chess_ai_v=1 event=search_cancel source={} elapsed_ms={} polls={} legal_moves={} depth_limit={} ply_limit={} time_budget_ms={} node_budget={} expected_hash={}",
                        Self::chess_ai_kind_source(kind),
                        elapsed_ms,
                        pending_polls,
                        pending_legal_moves,
                        limits.max_depth,
                        limits.max_depth,
                        limits.time_budget_ms,
                        limits.node_budget,
                        pending_hash.unwrap_or_default(),
                    ));
                }
            }
        }
        self.clear_chess_ai_pending_metadata();
        if let Some(source_id) = imp.chess_ai_search_poll_timer.borrow_mut().take() {
            Self::remove_source_if_present(source_id);
        }
    }

    fn queue_chess_ai_search(&self, kind: ChessAiPendingKind) -> bool {
        let imp = self.imp();
        if !imp.chess_mode_active.get() {
            return false;
        }

        if imp.chess_ai_pending_search.borrow().is_some() {
            let mut effective_kind = imp.chess_ai_pending_kind.get().unwrap_or(kind);
            if matches!(
                kind,
                ChessAiPendingKind::Wand {
                    include_opponent_auto_response: true
                }
            ) {
                if let ChessAiPendingKind::Wand {
                    include_opponent_auto_response: false,
                } = effective_kind
                {
                    effective_kind = ChessAiPendingKind::Wand {
                        include_opponent_auto_response: true,
                    };
                    imp.chess_ai_pending_kind.set(Some(effective_kind));
                    if imp.robot_debug_enabled.get() {
                        self.append_status_history_only(
                            "chess_ai_v=1 event=search_upgrade source=wand reason=merge_auto_response",
                        );
                    }
                }
            }
            let limits = imp
                .chess_ai_pending_limits
                .get()
                .unwrap_or_else(|| self.chess_ai_search_limits_for_kind(effective_kind));
            let started_elapsed_seconds = imp.chess_ai_pending_started_elapsed_seconds.get();
            let will_finish_suffix =
                self.chess_think_will_finish_suffix(started_elapsed_seconds, limits.time_budget_ms);
            *imp.status_override.borrow_mut() = Some(format!(
                "{}: still thinking...{}",
                self.chess_ai_status_prefix(effective_kind),
                will_finish_suffix,
            ));
            self.render();
            return true;
        }

        let position = imp.chess_position.borrow().clone();
        let legal_move_count = legal_moves(&position).len() as u32;
        if legal_move_count == 0 {
            *imp.status_override.borrow_mut() = Some(format!(
                "{}: no legal chess move available.",
                self.chess_ai_status_prefix(kind)
            ));
            self.render();
            return false;
        }

        let expected_hash = Self::chess_position_hash(&position);
        let limits = self.chess_ai_search_limits_for_kind(kind);
        let started_elapsed_seconds = imp.elapsed_seconds.get();
        let search = chess_ai::spawn_search(position.clone(), limits, AiConfig::default());
        *imp.chess_ai_pending_search.borrow_mut() = Some(search);
        imp.chess_ai_pending_kind.set(Some(kind));
        imp.chess_ai_pending_position_hash.set(Some(expected_hash));
        imp.chess_ai_pending_started_mono_us
            .set(glib::monotonic_time());
        imp.chess_ai_pending_started_elapsed_seconds
            .set(started_elapsed_seconds);
        imp.chess_ai_pending_poll_count.set(0);
        imp.chess_ai_pending_legal_moves.set(legal_move_count);
        imp.chess_ai_pending_limits.set(Some(limits));
        imp.chess_ai_pending_wait_log_step.set(0);
        let will_finish_suffix =
            self.chess_think_will_finish_suffix(started_elapsed_seconds, limits.time_budget_ms);

        *imp.status_override.borrow_mut() = Some(format!(
            "{}: thinking for {}...{}",
            self.chess_ai_status_prefix(kind),
            chess_color_label(position.side_to_move()),
            will_finish_suffix,
        ));
        self.render();
        if imp.robot_debug_enabled.get() {
            self.append_status_history_only(&format!(
                "chess_ai_v=1 event=search_start source={} side={} legal_moves={} depth_limit={} ply_limit={} time_budget_ms={} node_budget={} expected_hash={} move_count={} seed={}",
                Self::chess_ai_kind_source(kind),
                chess_color_label(position.side_to_move()),
                legal_move_count,
                limits.max_depth,
                limits.max_depth,
                limits.time_budget_ms,
                limits.node_budget,
                expected_hash,
                imp.move_count.get(),
                imp.current_seed.get(),
            ));
        }

        if imp.chess_ai_search_poll_timer.borrow().is_none() {
            let source_id = glib::timeout_add_local(
                Duration::from_millis(8),
                glib::clone!(
                    #[weak(rename_to = window)]
                    self,
                    #[upgrade_or]
                    glib::ControlFlow::Break,
                    move || window.poll_pending_chess_ai_search()
                ),
            );
            *imp.chess_ai_search_poll_timer.borrow_mut() = Some(source_id);
        }

        true
    }

    fn poll_pending_chess_ai_search(&self) -> glib::ControlFlow {
        let imp = self.imp();
        if imp.chess_ai_pending_search.borrow().is_some() {
            let next_polls = imp.chess_ai_pending_poll_count.get().saturating_add(1);
            imp.chess_ai_pending_poll_count.set(next_polls);
        }
        let maybe_result = {
            let mut search_slot = imp.chess_ai_pending_search.borrow_mut();
            let Some(search) = search_slot.as_mut() else {
                imp.chess_ai_search_poll_timer.borrow_mut().take();
                return glib::ControlFlow::Break;
            };
            search.try_recv()
        };

        let Some(result) = maybe_result else {
            self.maybe_emit_chess_ai_wait_metrics();
            return glib::ControlFlow::Continue;
        };

        imp.chess_ai_pending_search.borrow_mut().take();
        let pending_kind = imp.chess_ai_pending_kind.replace(None);
        let expected_hash = imp.chess_ai_pending_position_hash.replace(None);
        let started_mono_us = imp.chess_ai_pending_started_mono_us.replace(0);
        let pending_polls = imp.chess_ai_pending_poll_count.replace(0);
        let pending_legal_moves = imp.chess_ai_pending_legal_moves.replace(0);
        imp.chess_ai_pending_wait_log_step.set(0);
        let Some(kind) = pending_kind else {
            return self.next_chess_ai_poll_control_flow();
        };
        let pending_limits = imp
            .chess_ai_pending_limits
            .take()
            .unwrap_or_else(|| self.chess_ai_search_limits_for_kind(kind));
        let elapsed_ms = Self::chess_ai_elapsed_ms(started_mono_us);

        if !imp.chess_mode_active.get() {
            return self.next_chess_ai_poll_control_flow();
        }

        let current_hash = {
            let position = imp.chess_position.borrow();
            Self::chess_position_hash(&position)
        };
        if Some(current_hash) != expected_hash {
            if imp.robot_debug_enabled.get() {
                let best = result
                    .best_move
                    .map(Self::chess_ai_format_move)
                    .unwrap_or_else(|| "none".to_string());
                let term = format!("{:?}", result.termination).to_lowercase();
                self.append_status_history_only(&format!(
                    "chess_ai_v=1 event=search_stale_drop source={} elapsed_ms={} polls={} legal_moves={} depth_limit={} ply_limit={} time_budget_ms={} node_budget={} expected_hash={} current_hash={} best={} score_cp={} depth_reached={} ply_reached={} nodes={} pv_len={} termination={}",
                    Self::chess_ai_kind_source(kind),
                    elapsed_ms,
                    pending_polls,
                    pending_legal_moves,
                    pending_limits.max_depth,
                    pending_limits.max_depth,
                    pending_limits.time_budget_ms,
                    pending_limits.node_budget,
                    expected_hash.unwrap_or_default(),
                    current_hash,
                    best,
                    result.best_score_cp,
                    result.depth_reached,
                    result.depth_reached,
                    result.nodes,
                    result.pv.len(),
                    term,
                ));
            }
            return self.next_chess_ai_poll_control_flow();
        }

        let Some(chosen_move) = result.best_move else {
            *imp.status_override.borrow_mut() = Some(format!(
                "{}: no legal chess move available.",
                self.chess_ai_status_prefix(kind)
            ));
            self.render();
            if imp.robot_debug_enabled.get() {
                let term = format!("{:?}", result.termination).to_lowercase();
                self.append_status_history_only(&format!(
                    "chess_ai_v=1 event=search_done source={} elapsed_ms={} polls={} legal_moves={} depth_limit={} ply_limit={} time_budget_ms={} node_budget={} best=none score_cp={} depth_reached={} ply_reached={} nodes={} nps={} pv_len={} termination={} applied=false",
                    Self::chess_ai_kind_source(kind),
                    elapsed_ms,
                    pending_polls,
                    pending_legal_moves,
                    pending_limits.max_depth,
                    pending_limits.max_depth,
                    pending_limits.time_budget_ms,
                    pending_limits.node_budget,
                    result.best_score_cp,
                    result.depth_reached,
                    result.depth_reached,
                    result.nodes,
                    if elapsed_ms > 0 {
                        result.nodes.saturating_mul(1000) / elapsed_ms
                    } else {
                        0
                    },
                    result.pv.len(),
                    term,
                ));
            }
            return self.next_chess_ai_poll_control_flow();
        };

        let mut applied = false;
        match kind {
            ChessAiPendingKind::Wand {
                include_opponent_auto_response,
            } => {
                let source = self.chess_ai_status_prefix(ChessAiPendingKind::Wand {
                    include_opponent_auto_response,
                });
                applied = self.apply_chess_ai_move(chosen_move, &source);
                if applied
                    && include_opponent_auto_response
                    && self.chess_wand_ai_opponent_auto_response_enabled()
                {
                    let after = imp.chess_position.borrow().clone();
                    let should_respond =
                        self.chess_auto_response_side_matches(after.side_to_move());
                    if should_respond && !legal_moves(&after).is_empty() {
                        let _ = self.queue_chess_ai_search(ChessAiPendingKind::Wand {
                            include_opponent_auto_response: false,
                        });
                    }
                }
            }
            ChessAiPendingKind::Robot { side_to_move } => {
                let source =
                    self.chess_ai_status_prefix(ChessAiPendingKind::Robot { side_to_move });
                if self.apply_chess_ai_move(chosen_move, &source) {
                    applied = true;
                    let next_moves = imp.robot_moves_applied.get().saturating_add(1);
                    imp.robot_moves_applied.set(next_moves);
                }
            }
        }

        if imp.robot_debug_enabled.get() {
            let best = Self::chess_ai_format_move(chosen_move);
            let term = format!("{:?}", result.termination).to_lowercase();
            self.append_status_history_only(&format!(
                "chess_ai_v=1 event=search_done source={} elapsed_ms={} polls={} legal_moves={} depth_limit={} ply_limit={} time_budget_ms={} node_budget={} best={} score_cp={} depth_reached={} ply_reached={} nodes={} nps={} pv_len={} termination={} applied={}",
                Self::chess_ai_kind_source(kind),
                elapsed_ms,
                pending_polls,
                pending_legal_moves,
                pending_limits.max_depth,
                pending_limits.max_depth,
                pending_limits.time_budget_ms,
                pending_limits.node_budget,
                best,
                result.best_score_cp,
                result.depth_reached,
                result.depth_reached,
                result.nodes,
                if elapsed_ms > 0 {
                    result.nodes.saturating_mul(1000) / elapsed_ms
                } else {
                    0
                },
                result.pv.len(),
                term,
                if applied { "true" } else { "false" },
            ));
        }

        self.next_chess_ai_poll_control_flow()
    }

    fn next_chess_ai_poll_control_flow(&self) -> glib::ControlFlow {
        if self.imp().chess_ai_pending_search.borrow().is_some() {
            glib::ControlFlow::Continue
        } else {
            self.imp().chess_ai_search_poll_timer.borrow_mut().take();
            glib::ControlFlow::Break
        }
    }

    fn clear_chess_ai_pending_metadata(&self) {
        let imp = self.imp();
        imp.chess_ai_pending_kind.set(None);
        imp.chess_ai_pending_position_hash.set(None);
        imp.chess_ai_pending_started_mono_us.set(0);
        imp.chess_ai_pending_started_elapsed_seconds.set(0);
        imp.chess_ai_pending_poll_count.set(0);
        imp.chess_ai_pending_legal_moves.set(0);
        imp.chess_ai_pending_limits.set(None);
        imp.chess_ai_pending_wait_log_step.set(0);
    }

    fn maybe_emit_chess_ai_wait_metrics(&self) {
        let imp = self.imp();
        if !imp.robot_debug_enabled.get() {
            return;
        }
        let Some(kind) = imp.chess_ai_pending_kind.get() else {
            return;
        };
        let started_mono_us = imp.chess_ai_pending_started_mono_us.get();
        if started_mono_us <= 0 {
            return;
        }
        let elapsed_ms = Self::chess_ai_elapsed_ms(started_mono_us);
        let emitted_steps = imp.chess_ai_pending_wait_log_step.get();
        let next_threshold_ms = u64::from(emitted_steps.saturating_add(1)) * 250;
        if elapsed_ms < next_threshold_ms {
            return;
        }
        imp.chess_ai_pending_wait_log_step
            .set(emitted_steps.saturating_add(1));
        let limits = imp
            .chess_ai_pending_limits
            .get()
            .unwrap_or_else(|| self.chess_ai_search_limits_for_kind(kind));
        self.append_status_history_only(&format!(
            "chess_ai_v=1 event=search_wait source={} elapsed_ms={} polls={} legal_moves={} depth_limit={} ply_limit={} time_budget_ms={} node_budget={} expected_hash={}",
            Self::chess_ai_kind_source(kind),
            elapsed_ms,
            imp.chess_ai_pending_poll_count.get(),
            imp.chess_ai_pending_legal_moves.get(),
            limits.max_depth,
            limits.max_depth,
            limits.time_budget_ms,
            limits.node_budget,
            imp.chess_ai_pending_position_hash.get().unwrap_or_default(),
        ));
    }

    fn chess_position_hash(position: &ChessPosition) -> u64 {
        let mut hasher = DefaultHasher::new();
        position.hash(&mut hasher);
        hasher.finish()
    }

    fn chess_ai_kind_source(kind: ChessAiPendingKind) -> &'static str {
        match kind {
            ChessAiPendingKind::Wand { .. } => "wand",
            ChessAiPendingKind::Robot { side_to_move } => match side_to_move {
                ChessColor::White => "robot-white",
                ChessColor::Black => "robot-black",
            },
        }
    }

    fn chess_ai_status_prefix(&self, kind: ChessAiPendingKind) -> String {
        let limits = self.chess_ai_search_limits_for_kind(kind);
        let ply = limits.max_depth;
        let think_seconds = Self::chess_time_budget_seconds_label(limits.time_budget_ms);
        match kind {
            ChessAiPendingKind::Wand {
                include_opponent_auto_response: true,
            } => format!(
                "Wand Wave (Player) [{} | ply {} | think {}]",
                self.chess_wand_ai_strength_label(),
                ply,
                think_seconds
            ),
            ChessAiPendingKind::Wand {
                include_opponent_auto_response: false,
            } => format!(
                "Wand Wave (AI) [{} | ply {} | think {}]",
                self.chess_auto_response_ai_strength_label(),
                ply,
                think_seconds
            ),
            ChessAiPendingKind::Robot { side_to_move } => format!(
                "Robot ({}) [{} | ply {} | think {}]",
                chess_color_label(side_to_move),
                self.chess_robot_ai_strength_label_for_side(side_to_move),
                ply,
                think_seconds
            ),
        }
    }

    fn chess_ai_search_limits_for_kind(
        &self,
        kind: ChessAiPendingKind,
    ) -> crate::engine::chess::ai::SearchLimits {
        match kind {
            ChessAiPendingKind::Robot { side_to_move } => {
                self.chess_robot_ai_search_limits_for_side(side_to_move)
            }
            ChessAiPendingKind::Wand {
                include_opponent_auto_response,
            } => {
                if include_opponent_auto_response {
                    self.chess_wand_ai_search_limits()
                } else {
                    self.chess_auto_response_ai_search_limits()
                }
            }
        }
    }

    fn chess_ai_elapsed_ms(started_mono_us: i64) -> u64 {
        if started_mono_us <= 0 {
            return 0;
        }
        let now_us = glib::monotonic_time();
        if now_us <= started_mono_us {
            return 0;
        }
        ((now_us - started_mono_us) as u64) / 1000
    }

    fn chess_ai_format_move(chosen_move: ChessMove) -> String {
        let mut text = format!(
            "{}{}",
            square_name(chosen_move.from),
            square_name(chosen_move.to),
        );
        if let Some(promotion) = chosen_move.promotion {
            let promoted = match promotion {
                ChessPieceKind::Queen => "q",
                ChessPieceKind::Rook => "r",
                ChessPieceKind::Bishop => "b",
                ChessPieceKind::Knight => "n",
                ChessPieceKind::King | ChessPieceKind::Pawn => "",
            };
            text.push_str(promoted);
        }
        text
    }

    pub(in crate::window) fn chess_auto_response_side_matches(&self, side: ChessColor) -> bool {
        if self.chess_auto_response_plays_white_enabled() {
            side == ChessColor::White
        } else {
            side == ChessColor::Black
        }
    }

    pub(in crate::window) fn maybe_trigger_chess_auto_response_after_manual_move(&self) {
        let imp = self.imp();
        if !imp.chess_mode_active.get() || imp.robot_mode_running.get() {
            return;
        }
        if !self.chess_wand_ai_opponent_auto_response_enabled() {
            return;
        }
        if self.has_pending_chess_ai_search() {
            return;
        }

        let position = imp.chess_position.borrow().clone();
        let next_side = position.side_to_move();
        if !self.chess_auto_response_side_matches(next_side) {
            return;
        }
        if legal_moves(&position).is_empty() {
            return;
        }
        let _ = self.play_chess_ai_hint_move_single();
    }

    fn apply_chess_ai_move(&self, chosen_move: ChessMove, source: &str) -> bool {
        let imp = self.imp();
        let position = imp.chess_position.borrow().clone();
        let side_to_move = position.side_to_move();
        let capture_suffix = chess_move_capture_suffix(&position, chosen_move);

        let undo_anchor = self.snapshot();
        let result = {
            let mut live_position = imp.chess_position.borrow_mut();
            chess_boundary::execute(&mut live_position, ChessCommand::TryMove(chosen_move))
        };
        if !result.changed {
            *imp.status_override.borrow_mut() =
                Some(format!("{source}: move was not legal anymore."));
            self.render();
            return false;
        }

        self.push_chess_history_position(position);
        imp.history.borrow_mut().push(undo_anchor);
        imp.future.borrow_mut().clear();
        imp.chess_selected_square.set(None);
        imp.chess_last_move_from.set(Some(chosen_move.from));
        imp.chess_last_move_to.set(Some(chosen_move.to));
        imp.chess_keyboard_square.set(Some(chosen_move.to));
        let next_move_count = imp.move_count.get().saturating_add(1);
        imp.move_count.set(next_move_count);
        if !imp.timer_started.get() {
            imp.timer_started.set(true);
        }

        let after = imp.chess_position.borrow().clone();
        let next_side = after.side_to_move();
        let status = if let Some(terminal_status) = chess_terminal_status_text(&after) {
            imp.timer_started.set(false);
            terminal_status
        } else if is_in_check(&after, next_side) {
            format!("Check on {}.", chess_color_label(next_side))
        } else {
            let move_prefix = chess_move_number_prefix(next_move_count, side_to_move);
            format!(
                "{source}: {move_prefix}{} played {} -> {}{}.",
                chess_color_label(side_to_move),
                square_name(chosen_move.from),
                square_name(chosen_move.to),
                capture_suffix,
            )
        };
        *imp.status_override.borrow_mut() = Some(status);
        self.maybe_play_chess_system_move_sound();
        let rendered_by_flip = self.maybe_auto_flip_chess_board_to_side_to_move(false);
        if !rendered_by_flip {
            self.render();
        }
        true
    }

    pub(crate) fn handle_chess_board_stack_click(&self, file_index: usize, y: f64) {
        if file_index >= 8 {
            return;
        }
        let Some(target_square) = self.chess_square_from_stack_y(file_index, y) else {
            return;
        };
        self.activate_chess_square(target_square);
    }

    pub(in crate::window) fn chess_keyboard_square(&self) -> Option<Square> {
        let imp = self.imp();
        if let Some(existing) = imp.chess_keyboard_square.get() {
            return Some(existing);
        }
        let position = imp.chess_position.borrow().clone();
        let fallback = Self::default_chess_keyboard_square(&position);
        imp.chess_keyboard_square.set(fallback);
        fallback
    }

    pub(in crate::window) fn handle_chess_keyboard_key(&self, key: gdk::Key) -> bool {
        let horizontal_left_delta = if self.chess_board_flipped() { 1 } else { -1 };
        let horizontal_right_delta = -horizontal_left_delta;
        let vertical_up_delta = if self.chess_board_flipped() { -1 } else { 1 };
        let vertical_down_delta = -vertical_up_delta;
        match key {
            gdk::Key::KP_7 => {
                self.undo();
                true
            }
            gdk::Key::KP_9 => {
                self.redo();
                true
            }
            gdk::Key::KP_1 => {
                self.toggle_robot_mode();
                true
            }
            gdk::Key::KP_3 => {
                let _ = self.play_chess_ai_robot_move();
                true
            }
            gdk::Key::KP_Multiply => {
                let _ = self.play_chess_ai_hint_move();
                true
            }
            gdk::Key::KP_Subtract => {
                self.undo();
                true
            }
            gdk::Key::KP_Add => {
                self.redo();
                true
            }
            gdk::Key::KP_Decimal | gdk::Key::KP_Delete => {
                let imp = self.imp();
                imp.chess_selected_square.set(None);
                *imp.status_override.borrow_mut() = Some("Selection cleared.".to_string());
                self.render();
                true
            }
            gdk::Key::Left
            | gdk::Key::KP_Left
            | gdk::Key::h
            | gdk::Key::H
            | gdk::Key::a
            | gdk::Key::A => {
                self.move_chess_keyboard_square(horizontal_left_delta, 0);
                true
            }
            gdk::Key::Right
            | gdk::Key::KP_Right
            | gdk::Key::l
            | gdk::Key::L
            | gdk::Key::d
            | gdk::Key::D => {
                self.move_chess_keyboard_square(horizontal_right_delta, 0);
                true
            }
            gdk::Key::Up
            | gdk::Key::KP_Up
            | gdk::Key::k
            | gdk::Key::K
            | gdk::Key::w
            | gdk::Key::W => {
                self.move_chess_keyboard_square(0, vertical_up_delta);
                true
            }
            gdk::Key::Down
            | gdk::Key::KP_Down
            | gdk::Key::j
            | gdk::Key::J
            | gdk::Key::s
            | gdk::Key::S => {
                self.move_chess_keyboard_square(0, vertical_down_delta);
                true
            }
            gdk::Key::Return | gdk::Key::KP_Enter | gdk::Key::space => {
                self.activate_chess_keyboard_square();
                true
            }
            gdk::Key::Escape => {
                let imp = self.imp();
                imp.chess_selected_square.set(None);
                *imp.status_override.borrow_mut() = Some("Selection cleared.".to_string());
                self.render();
                true
            }
            _ => false,
        }
    }

    fn move_chess_keyboard_square(&self, file_delta: i32, rank_delta: i32) {
        let imp = self.imp();
        let Some(current) = self.chess_keyboard_square() else {
            return;
        };
        let file = (i32::from(file_of(current)) + file_delta).clamp(0, 7) as u8;
        let rank = (i32::from(rank_of(current)) + rank_delta).clamp(0, 7) as u8;
        if let Some(next) = square(file, rank) {
            imp.chess_keyboard_square.set(Some(next));
            self.render();
        }
    }

    fn activate_chess_keyboard_square(&self) {
        let Some(target_square) = self.chess_keyboard_square() else {
            return;
        };
        self.activate_chess_square(target_square);
    }

    fn activate_chess_square(&self, target_square: Square) {
        let imp = self.imp();
        imp.chess_keyboard_square.set(Some(target_square));
        let position = imp.chess_position.borrow().clone();
        let side_to_move = position.side_to_move();
        let selected = imp.chess_selected_square.get();

        let mut move_applied = false;
        let mut status = match selected {
            None => {
                if let Some(piece) = position.piece_at(target_square) {
                    if piece.color == side_to_move {
                        imp.chess_selected_square.set(Some(target_square));
                        Some(format!("Selected {}.", square_name(target_square)))
                    } else {
                        Some(format!(
                            "It is {} to move. Select one of your pieces.",
                            chess_color_label(side_to_move)
                        ))
                    }
                } else {
                    Some(format!(
                        "It is {} to move. Select a piece first.",
                        chess_color_label(side_to_move)
                    ))
                }
            }
            Some(from) => {
                if from == target_square {
                    imp.chess_selected_square.set(None);
                    Some("Selection cleared.".to_string())
                } else if let Some(chosen_move) =
                    choose_move_for_destination(&position, from, target_square)
                {
                    // Manual board input takes priority over any stale pending AI search.
                    // Cancel first so post-move auto-response can be enqueued for the
                    // newly applied position.
                    self.cancel_pending_chess_ai_search();
                    let undo_anchor = self.snapshot();
                    let result = {
                        let mut live_position = imp.chess_position.borrow_mut();
                        chess_boundary::execute(
                            &mut live_position,
                            ChessCommand::TryMove(chosen_move),
                        )
                    };
                    if result.changed {
                        let capture_suffix = chess_move_capture_suffix(&position, chosen_move);
                        self.push_chess_history_position(position.clone());
                        imp.history.borrow_mut().push(undo_anchor);
                        imp.future.borrow_mut().clear();
                        imp.chess_selected_square.set(None);
                        imp.chess_last_move_from.set(Some(from));
                        imp.chess_last_move_to.set(Some(target_square));
                        let next_move_count = imp.move_count.get().saturating_add(1);
                        imp.move_count.set(next_move_count);
                        if !imp.timer_started.get() {
                            imp.timer_started.set(true);
                        }
                        move_applied = true;
                        Some(format!(
                            "{}{}: {} -> {}{}",
                            chess_move_number_prefix(next_move_count, side_to_move),
                            chess_color_label(side_to_move),
                            square_name(from),
                            square_name(target_square),
                            capture_suffix,
                        ))
                    } else {
                        Some("Illegal move.".to_string())
                    }
                } else if let Some(piece) = position.piece_at(target_square) {
                    if piece.color == side_to_move {
                        imp.chess_selected_square.set(Some(target_square));
                        Some(format!("Selected {}.", square_name(target_square)))
                    } else {
                        Some("Illegal move.".to_string())
                    }
                } else {
                    Some("Illegal move.".to_string())
                }
            }
        };

        if move_applied {
            let after = imp.chess_position.borrow().clone();
            let next_side = after.side_to_move();
            if let Some(terminal_status) = chess_terminal_status_text(&after) {
                status = Some(terminal_status);
                imp.timer_started.set(false);
            } else if is_in_check(&after, next_side) {
                status = Some(format!("Check on {}.", chess_color_label(next_side)));
            }
        }

        if let Some(status) = status {
            *imp.status_override.borrow_mut() = Some(status);
        }
        let rendered_by_flip = if move_applied {
            self.maybe_auto_flip_chess_board_to_side_to_move(false)
        } else {
            false
        };
        if !rendered_by_flip {
            self.render();
        }
        if move_applied {
            self.maybe_play_chess_system_move_sound();
            self.maybe_trigger_chess_auto_response_after_manual_move();
        }
    }

    pub(in crate::window) fn chess_drag_payload_for_stack_y(
        &self,
        file_index: usize,
        y: f64,
    ) -> Option<String> {
        if file_index >= 8 {
            return None;
        }
        let sq = self.chess_square_from_stack_y(file_index, y)?;
        let position = self.imp().chess_position.borrow();
        let piece = position.piece_at(sq)?;
        if piece.color != position.side_to_move() {
            return None;
        }
        Some(format!("chess:{sq}"))
    }

    pub(in crate::window) fn handle_chess_board_drop_from_payload(
        &self,
        file_index: usize,
        y: f64,
        payload: &str,
    ) -> bool {
        if file_index >= 8 {
            return false;
        }
        let Some(from) = parse_chess_drag_payload(payload) else {
            return false;
        };
        let Some(target_square) = self.chess_square_from_stack_y(file_index, y) else {
            return false;
        };

        let imp = self.imp();
        imp.chess_keyboard_square.set(Some(target_square));
        let position = imp.chess_position.borrow().clone();
        let side_to_move = position.side_to_move();
        imp.chess_selected_square.set(None);

        if from == target_square {
            return false;
        }
        let Some(piece) = position.piece_at(from) else {
            *imp.status_override.borrow_mut() = Some("Drag source square is empty.".to_string());
            self.render();
            return false;
        };
        if piece.color != side_to_move {
            *imp.status_override.borrow_mut() = Some(format!(
                "It is {} to move. Drag one of your own pieces.",
                chess_color_label(side_to_move)
            ));
            self.render();
            return false;
        }

        let Some(chosen_move) = choose_move_for_destination(&position, from, target_square) else {
            *imp.status_override.borrow_mut() = Some("That drag move is not legal.".to_string());
            self.render();
            return false;
        };
        // Manual board input takes priority over any stale pending AI search.
        // Cancel first so post-move auto-response can be enqueued for the
        // newly applied position.
        self.cancel_pending_chess_ai_search();
        let capture_suffix = chess_move_capture_suffix(&position, chosen_move);

        let undo_anchor = self.snapshot();
        let result = {
            let mut live_position = imp.chess_position.borrow_mut();
            chess_boundary::execute(&mut live_position, ChessCommand::TryMove(chosen_move))
        };
        if !result.changed {
            *imp.status_override.borrow_mut() = Some("That drag move is not legal.".to_string());
            self.render();
            return false;
        }

        self.push_chess_history_position(position);
        imp.history.borrow_mut().push(undo_anchor);
        imp.future.borrow_mut().clear();
        imp.chess_last_move_from.set(Some(from));
        imp.chess_last_move_to.set(Some(target_square));
        let next_move_count = imp.move_count.get().saturating_add(1);
        imp.move_count.set(next_move_count);
        imp.timer_started.set(true);

        let after = imp.chess_position.borrow().clone();
        let next_side = after.side_to_move();
        let status = if let Some(terminal_status) = chess_terminal_status_text(&after) {
            imp.timer_started.set(false);
            terminal_status
        } else if is_in_check(&after, next_side) {
            format!("Check on {}.", chess_color_label(next_side))
        } else {
            format!(
                "{}{}: {} -> {}{}",
                chess_move_number_prefix(next_move_count, side_to_move),
                chess_color_label(side_to_move),
                square_name(from),
                square_name(target_square),
                capture_suffix,
            )
        };

        *imp.status_override.borrow_mut() = Some(status);
        let rendered_by_flip = self.maybe_auto_flip_chess_board_to_side_to_move(false);
        if !rendered_by_flip {
            self.render();
        }
        self.maybe_play_chess_system_move_sound();
        self.maybe_trigger_chess_auto_response_after_manual_move();
        true
    }

    pub(in crate::window) fn chess_square_from_stack_y(
        &self,
        file_index: usize,
        y: f64,
    ) -> Option<Square> {
        let row_from_top = self.chess_row_from_stack_y(y)?;
        self.chess_square_from_display_cell(file_index, row_from_top)
    }

    pub(in crate::window) fn chess_row_from_stack_y(&self, y: f64) -> Option<i32> {
        if y.is_sign_negative() {
            return None;
        }
        let square_size = self.imp().chess_square_size.get().max(1) as f64;
        let row_from_top = (y / square_size).floor() as i32;
        (0..8).contains(&row_from_top).then_some(row_from_top)
    }

    pub(in crate::window) fn set_chess_drag_hover_row_from_top(&self, row_from_top: Option<i32>) {
        let normalized = row_from_top.filter(|row| (0..8).contains(row));
        let imp = self.imp();
        if imp.chess_drag_hover_row_from_top.get() == normalized {
            return;
        }
        imp.chess_drag_hover_row_from_top.set(normalized);
        if imp.chess_mode_active.get() {
            self.apply_chess_drag_hover_row_classes();
        }
    }

    fn default_chess_keyboard_square(position: &ChessPosition) -> Option<Square> {
        if let Some(opening_from) = legal_moves(position).into_iter().next().map(|mv| mv.from) {
            return Some(opening_from);
        }

        let side = position.side_to_move();
        for sq in 0u8..64 {
            if position
                .piece_at(sq)
                .map(|piece| piece.color == side)
                .unwrap_or(false)
            {
                return Some(sq);
            }
        }
        (0u8..64).find(|sq| position.piece_at(*sq).is_some())
    }
}

fn choose_move_for_destination(
    position: &ChessPosition,
    from: Square,
    to: Square,
) -> Option<ChessMove> {
    let mut candidates = legal_moves(position)
        .into_iter()
        .filter(|mv| mv.from == from && mv.to == to)
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return None;
    }
    if candidates.len() == 1 {
        return candidates.pop();
    }
    candidates
        .iter()
        .copied()
        .find(|mv| mv.promotion == Some(ChessPieceKind::Queen))
        .or_else(|| candidates.into_iter().next())
}

fn chess_color_label(color: ChessColor) -> &'static str {
    match color {
        ChessColor::White => "White",
        ChessColor::Black => "Black",
    }
}

fn chess_terminal_status_text(position: &ChessPosition) -> Option<String> {
    match terminal_state(position)? {
        ChessTerminalState::Checkmate { winner } => {
            Some(format!("Checkmate. {} wins.", chess_color_label(winner)))
        }
        ChessTerminalState::DrawStalemate => Some("Stalemate.".to_string()),
        ChessTerminalState::DrawFiftyMoveRule => Some("Draw by fifty-move rule.".to_string()),
        ChessTerminalState::DrawInsufficientMaterial => {
            Some("Draw by insufficient material.".to_string())
        }
    }
}

fn chess_piece_kind_label(kind: ChessPieceKind) -> &'static str {
    match kind {
        ChessPieceKind::King => "King",
        ChessPieceKind::Queen => "Queen",
        ChessPieceKind::Rook => "Rook",
        ChessPieceKind::Bishop => "Bishop",
        ChessPieceKind::Knight => "Knight",
        ChessPieceKind::Pawn => "Pawn",
    }
}

fn chess_move_captured_piece(
    position: &ChessPosition,
    chess_move: ChessMove,
) -> Option<ChessPiece> {
    let side_to_move = position.side_to_move();
    if chess_move.is_en_passant {
        let capture_square = square(file_of(chess_move.to), rank_of(chess_move.from))?;
        let captured = position.piece_at(capture_square)?;
        if captured.color == side_to_move {
            return None;
        }
        return Some(captured);
    }
    let captured = position.piece_at(chess_move.to)?;
    (captured.color != side_to_move).then_some(captured)
}

fn chess_move_capture_suffix(position: &ChessPosition, chess_move: ChessMove) -> String {
    if let Some(captured) = chess_move_captured_piece(position, chess_move) {
        format!(
            " capturing {} {}",
            chess_color_label(captured.color),
            chess_piece_kind_label(captured.kind)
        )
    } else {
        String::new()
    }
}

fn chess_move_number_prefix(move_count_after_apply: u32, side: ChessColor) -> String {
    let fullmove = move_count_after_apply.saturating_sub(1) / 2 + 1;
    match side {
        ChessColor::White => format!("{fullmove}. "),
        ChessColor::Black => format!("{fullmove}... "),
    }
}

fn parse_chess_drag_payload(payload: &str) -> Option<Square> {
    let rest = payload.strip_prefix("chess:")?;
    let square = rest.parse::<u8>().ok()?;
    (square < 64).then_some(square)
}

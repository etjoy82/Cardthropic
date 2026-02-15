# FreeCell Test Plan (Target: 0.9.0)

Concrete FreeCell test inventory with implementation status.
Use alongside `FREECELL_ROLLOUT.md`.

## Scope

- Variant: FreeCell
- Levels: game rules, boundary/dispatch, session/runtime isolation, UI interaction safety, automation parity
- Primary targets:
  - `src/game/freecell.rs`
  - `src/game/tests.rs`
  - `src/engine/boundary.rs`
  - `src/engine/tests.rs`
  - `src/engine/variant_engine/*`
  - `src/engine/variant_state.rs`
  - `src/engine/session.rs`
  - `src/window/actions_history.rs`
  - `src/window/actions_selection.rs`
  - `src/window/actions_moves.rs`
  - `src/window/drag.rs`
  - `src/window/drag_setup.rs`
  - `src/window/hint_*`
  - `src/window/robot.rs`
  - `src/winnability.rs`

## Test Commands

- Full: `cargo test -q`
- Targeted:
  - `cargo test -q freecell_rule_`
  - `cargo test -q freecell_boundary_`
  - `cargo test -q freecell_session_`
  - `cargo test -q freecell_history_`
  - `cargo test -q freecell_ui_`
  - `cargo test -q freecell_auto_`

---

## A. Core Rules (Game Layer)

Implementation status in `src/game/tests.rs`:

- [ ] `freecell_rule_001_seeded_setup_is_deterministic`
- [ ] `freecell_rule_002_initial_deal_counts_are_correct`
- [ ] `freecell_rule_003_cascade_to_cascade_requires_descending_alternating`
- [ ] `freecell_rule_004_cascade_to_freecell_requires_empty_cell`
- [ ] `freecell_rule_005_freecell_to_cascade_requires_legal_target`
- [ ] `freecell_rule_006_card_to_foundation_requires_next_rank_same_suit`
- [ ] `freecell_rule_007_move_from_empty_source_is_rejected`
- [ ] `freecell_rule_008_multi_card_move_respects_capacity_formula`
- [ ] `freecell_rule_009_multi_card_move_rejects_excess_length`
- [ ] `freecell_rule_010_empty_cascade_as_destination_is_legal_when_rules_allow`
- [ ] `freecell_rule_011_win_condition_requires_all_foundations_complete`
- [ ] `freecell_rule_012_state_hash_changes_after_legal_move`

Additional expected game-level coverage:

- [ ] 52-card accounting invariants after every move type
- [ ] Foundation monotonicity invariants
- [ ] Free-cell occupancy invariants

---

## B. Boundary / Variant Engine

Implementation status in `src/engine/tests.rs`:

- [ ] `freecell_boundary_001_initialize_seeded_updates_freecell_only`
- [ ] `freecell_boundary_002_execute_command_routes_all_freecell_moves`
- [ ] `freecell_boundary_003_out_of_range_indices_are_rejected`
- [ ] `freecell_boundary_004_illegal_move_noops_without_state_corruption`
- [ ] `freecell_boundary_005_mode_switch_preserves_other_variants`

Engine capability assertions:

- [ ] FreeCell engine marked ready when rules are complete
- [ ] Capability policy explicitly set (seeded deals, undo/redo, automation flags)
- [ ] Render controls reflect capability policy for FreeCell

---

## C. Session / Runtime / History Integrity

Expected automated coverage:

- [ ] `freecell_session_001_persisted_session_round_trip`
- [ ] `freecell_session_002_variant_runtime_isolation`
- [ ] `freecell_history_001_undo_redo_round_trip_within_freecell`
- [ ] `freecell_history_002_resume_restores_undo_stack`
- [ ] `freecell_history_003_cross_mode_undo_jumps_include_freecell`
- [ ] `freecell_history_004_mode_switch_always_anchors_history`

Manual verification:

- [ ] Make moves, restart app, confirm undo is still enabled
- [ ] Jump between Klondike/Spider/FreeCell through undo/redo chain

---

## D. UI Interaction Safety

Current/expected coverage:

- [ ] `freecell_ui_001_click_select_and_move_legal_card`
- [ ] `freecell_ui_002_drag_drop_legal_card`
- [ ] `freecell_ui_003_drag_drop_illegal_move_rejected_with_status`
- [ ] `freecell_ui_004_keyboard_navigation_targets_cover_cells_cascades_foundations`
- [ ] `freecell_ui_005_enter_in_seed_box_starts_seed_not_board_action`
- [ ] `freecell_ui_006_resize_keeps_all_cascades_visible`

Manual regression:

- [ ] Illegal drops never mutate board state
- [ ] No focus/shortcut conflict with seed field or dialogs
- [ ] Status label/history remains readable and compact

---

## E. Automation / Solver / Robot / W?

Expected coverage:

- [ ] `freecell_auto_001_hint_returns_legal_move`
- [ ] `freecell_auto_002_magic_wand_applies_legal_move_sequence`
- [ ] `freecell_auto_003_robot_progresses_or_reports_stop_reason`
- [ ] `freecell_auto_004_robot_debug_schema_matches_standard_format`
- [ ] `freecell_auto_005_winnability_check_reports_result_or_limits`
- [ ] `freecell_auto_006_non_debug_mode_is_not_log-spammy`

Policy checks:

- [ ] If any automation is deferred, controls are disabled and help text matches reality
- [ ] Shortcuts cannot bypass disabled policy

---

## F. Seed UX + Word Seed Compatibility

Expected coverage:

- [ ] `freecell_seed_001_numeric_seed_starts_expected_deal`
- [ ] `freecell_seed_002_word_seed_maps_deterministically_case_insensitive`
- [ ] `freecell_seed_003_word_seed_max_length_enforced`
- [ ] `freecell_seed_004_word_seed_text_remains_visible_after_start`
- [ ] `freecell_seed_005_status_includes_seed_number_and_word_label`

Manual checks:

- [ ] Entering word + pressing Enter starts FreeCell seed correctly
- [ ] Blank seed still generates random numeric seed

---

## G. Help / Docs / UX Accuracy

Verification checklist:

- [ ] In-app help includes FreeCell controls and rules
- [ ] README feature list includes FreeCell accurately
- [ ] No stale messaging says FreeCell is disabled after enablement
- [ ] Release notes describe FreeCell maturity clearly

---

## Regression Checklist Before 0.9.0 Candidate

- [ ] FreeCell rule suite implemented and green
- [ ] FreeCell boundary suite implemented and green
- [ ] FreeCell session/history suite implemented and green
- [ ] FreeCell UI/automation suite implemented and green
- [ ] `cargo check -q` on candidate tree
- [ ] `cargo test -q` on candidate tree
- [ ] `scripts/release/maintainer-gate.sh --strict-tools` on candidate tree
- [ ] Manual smoke matrix complete (seed, move, undo, mode switch, resume, resize)
- [ ] Local backup created (`backupct "freecell-0.9.0-candidate"`)

---

## Naming Convention (Keep)

- `freecell_rule_*`
- `freecell_boundary_*`
- `freecell_session_*`
- `freecell_history_*`
- `freecell_ui_*`
- `freecell_auto_*`
- `freecell_seed_*`

# Spider Test Plan (0.7.0-dev)

Concrete Spider test inventory with implementation status.
Use alongside `SPIDER_ROLLOUT.md`.

## Scope

- Variant: Spider Solitaire
- Levels: game rules, boundary/dispatch, session/runtime isolation, UI interaction safety
- Primary targets:
  - `src/game/spider.rs`
  - `src/game/tests.rs`
  - `src/engine/boundary.rs`
  - `src/engine/tests.rs`
  - `src/engine/variant_engine/stubs.rs`
  - `src/engine/variant_state.rs`
  - `src/engine/session.rs`
  - `src/window/actions_history.rs`
  - `src/window/actions_selection.rs`
  - `src/window/drag.rs`
  - `src/window/drag_setup.rs`

## Test Commands

- Full: `cargo test -q`
- Targeted:
  - `cargo test -q spider_rule_`
  - `cargo test -q spider_boundary_`
  - `cargo test -q spider_session_`

---

## A. Core Rules (Game Layer)

Implementation status in `src/game/tests.rs`:

- [x] `spider_rule_001_valid_descending_same_suit_run_can_move`
- [x] `spider_rule_002_run_starting_on_face_down_card_is_rejected`
- [x] `spider_rule_003_run_containing_face_down_card_is_rejected`
- [x] `spider_rule_004_invalid_sequence_is_rejected`
- [x] `spider_rule_005_move_to_illegal_destination_is_rejected`
- [x] `spider_rule_006_move_to_empty_tableau_follows_spider_rules`
- [x] `spider_rule_007_completed_suit_run_is_extracted_correctly`
- [x] `spider_rule_008_deal_blocked_when_any_tableau_empty_when_rule_enabled`
- [x] `spider_rule_009_deal_adds_one_face_up_card_per_tableau_when_legal`
- [x] `spider_rule_010_win_condition_only_when_all_required_runs_complete`

Additional Spider game-level coverage already present:

- [x] Seed determinism
- [x] 104-card accounting
- [x] Initial geometry checks
- [x] Deal behavior checks
- [x] Session codec round-trip for Spider game state

---

## B. Boundary / Variant Engine

Implementation status in `src/engine/tests.rs`:

- [x] `spider_boundary_001_initialize_seeded_updates_spider_without_mutating_klondike`
- [x] `spider_boundary_002_execute_command_routes_spider_draw_and_move`
- [x] `spider_boundary_003_spider_can_move_helpers_reject_out_of_range_indices`
- [x] `spider_boundary_004_draw_noops_when_deal_is_not_allowed`
- [x] `spider_boundary_005_move_tableau_run_executes_when_legal`

Engine capability assertions:

- [x] Spider engine marked ready
- [x] Spider capability policy assertions (draw/undo on, automation/winnability off)

---

## C. Session / Runtime / History Integrity

Current automated coverage:

- [x] Persisted session v2 round-trip for Spider runtime (`persisted_session_v2_round_trip_for_spider_runtime`)
- [x] Variant state runtime isolation (`variant_state_set_runtime_spider_does_not_mutate_klondike`)
- [x] Variant state stores Spider and Klondike independently (`variant_state_store_tracks_klondike_and_spider_separately`)

Still needed:

- [ ] Dedicated Spider undo/redo behavior tests at window/action layer
- [ ] Mixed-mode history stack stress tests (mode-switch + undo/redo interleaving)

---

## D. UI Interaction Safety

Current implementation status (manual-first, not yet fully unit-tested):

- [x] Spider click selection path uses Spider run-start mapping (`handlers_actions.rs`)
- [x] Spider drag prepare/begin path exists (`drag_setup.rs`)
- [x] Spider drag texture path exists (`drag.rs`)
- [x] Action-layer guard rejects hidden-card runs (`actions_moves.rs`)

Still needed:

- [ ] Automated UI-focused tests for illegal click/drag boundaries
- [ ] Manual regression pass for invalid drops + status clarity
- [ ] Resize stress pass to ensure no legality/render drift

---

## E. Automation Policy (Spider Alpha)

Current declared policy (enforced by capabilities):

- [x] Smart Move disabled
- [x] Autoplay disabled
- [x] Rapid Wand disabled
- [x] Robot mode disabled
- [x] Winnability tooling disabled

Remaining validation:

- [ ] Verify all action/shortcut entrypoints respect disabled policy in Spider
- [ ] Ensure help/status text explicitly communicates policy

---

## Regression Checklist Before Spider Alpha Cut

- [x] SPIDER-RULE suite implemented
- [x] SPIDER-BOUNDARY suite implemented
- [x] Session/runtime Spider coverage exists
- [ ] Spider undo/redo dedicated test coverage complete
- [ ] UI manual smoke pass complete (click/drag/undo/redo/resize)
- [ ] `cargo check -q` (fresh run on release candidate tree)
- [ ] `cargo test -q` (fresh run on release candidate tree)
- [ ] `scripts/release/maintainer-gate.sh --strict-tools` (fresh run)
- [ ] Local backup created (`backupct "spider-alpha-candidate"`)

---

## Naming Convention (Keep)

- `spider_rule_*`
- `spider_boundary_*`
- `spider_session_*`
- `spider_history_*`
- `spider_ui_*`
- `spider_auto_*`

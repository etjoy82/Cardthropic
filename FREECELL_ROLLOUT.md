# FreeCell Rollout Checklist (Target: 0.9.0)

Authoritative tracker for implementing full FreeCell without destabilizing Klondike/Spider.
Use this as the execution order for the 0.9.0 feature pass.

## Current Snapshot

- FreeCell is intentionally not feature-ready.
- Main menu currently disables FreeCell to avoid misleading users.
- No release gate for 0.9.0 should pass until FreeCell reaches engine + UX parity baseline.

## Baseline Gate (Before Work Starts)

- [ ] Create checkpoint backup/tag before FreeCell pass (`backupct "freecell-pass0-start"`)
- [ ] Confirm clean compile baseline: `cargo check -q`
- [ ] Confirm existing suites green: `cargo test -q`
- [ ] Record current behavior for mode switch/undo/session resume (Klondike + Spider)

---

## Pass 1: Engine Rule Lock (FreeCell Core)

Files (expected):
- `src/game/freecell.rs` (new or expanded)
- `src/engine/boundary.rs`
- `src/engine/variant_engine/*`
- `src/engine/variant_state.rs`

Status:
- [ ] Implement FreeCell deck setup (8 cascades, 4 free cells, 4 foundations)
- [ ] Implement legal move primitives:
- [ ] Cascade -> Cascade (descending, alternating color)
- [ ] Cascade -> FreeCell
- [ ] FreeCell -> Cascade
- [ ] Cascade/FreeCell -> Foundation
- [ ] Implement move-capacity rule for multi-card transfer based on empty free cells/cascades
- [ ] Implement win condition (all foundations complete)
- [ ] Add deterministic seeded setup with existing seed system
- [ ] Add command-level boundary execution for all FreeCell move types

---

## Pass 2: Variant Engine Capabilities + Mode Wiring

Files (expected):
- `src/engine/variant_engine/stubs.rs`
- `src/engine/variant_engine/mod.rs`
- `src/engine/render_plan.rs`
- `src/window/variant_flow.rs`
- `src/window/menu.rs`

Status:
- [ ] Add FreeCell capabilities definition (seeded deals, undo/redo, hint/smart move policy)
- [ ] Wire mode switch lifecycle for FreeCell with no cross-mode state mutation
- [ ] Enable FreeCell mode button/menu entry once baseline is playable
- [ ] Ensure disabled-state policy remains explicit until each feature is ready

---

## Pass 3: Session, Undo/Redo, Cross-Mode History Integrity

Files (expected):
- `src/engine/session.rs`
- `src/window/actions_history.rs`
- `src/window/session.rs`
- `src/engine/tests.rs`

Status:
- [ ] Persist FreeCell runtime in session encode/decode
- [ ] Ensure resume restores FreeCell board correctly
- [ ] Ensure undo/redo persists across app restart for FreeCell
- [ ] Ensure cross-mode undo/redo can jump between Klondike/Spider/FreeCell snapshots
- [ ] Ensure mode switches are always anchored in history

---

## Pass 4: FreeCell Rendering + Layout Contract

Files (expected):
- `src/window/render.rs`
- `src/window/render_tableau.rs`
- `src/window/render_stock_waste_foundation.rs` (or FreeCell-specific render module)
- `src/window/layout.rs`
- `src/window.ui`
- `src/style.css`

Status:
- [ ] Add FreeCell render path (cascades, free cells, foundations)
- [ ] Add mode-aware column/slot geometry and scaling rules
- [ ] Verify resize smoothness and no off-screen overflow at common sizes
- [ ] Keep status/HUD compact and consistent with existing modes
- [ ] Keep theme/userstyle selectors targeting FreeCell surfaces correctly

---

## Pass 5: Input Contract + Interaction Safety

Files (expected):
- `src/window/actions_selection.rs`
- `src/window/actions_moves.rs`
- `src/window/handlers_actions.rs`
- `src/window/drag.rs`
- `src/window/drag_setup.rs`
- `src/window/input.rs`

Status:
- [ ] Click-select semantics for FreeCell (single card and legal destinations)
- [ ] Drag-and-drop legality checks with consistent rejection feedback
- [ ] Keyboard navigation/focus support for FreeCell targets
- [ ] Smart move behavior defined and implemented for FreeCell
- [ ] No input path may bypass legality checks

---

## Pass 6: Hints, Wand, Robot, Winnability

Files (expected):
- `src/window/hint_core.rs`
- `src/window/hint_smart_move.rs`
- `src/window/hint_autoplay.rs`
- `src/window/robot.rs`
- `src/winnability.rs`
- `src/engine/robot.rs`

Status:
- [ ] Hint line generation for FreeCell
- [ ] Magic Wand support for FreeCell
- [ ] Robot mode support for FreeCell with status telemetry parity
- [ ] FreeCell W? support (seed winnability path) or explicit policy if deferred
- [ ] Ensure debug vs non-debug verbosity behavior matches existing standards

---

## Pass 7: Seed UX Parity + Word Seed Compatibility

Files (expected):
- `src/engine/seed_ops.rs`
- `src/window/seed_input.rs`
- `src/window/ai_winnability_check.rs`

Status:
- [ ] Confirm FreeCell seeded deal starts via numeric seeds
- [ ] Confirm FreeCell seeded deal starts via word seeds
- [ ] Keep word seed visibility in textbox after start
- [ ] Ensure status text includes numeric seed + `[word]` when applicable

---

## Pass 8: Help, Copy, Docs, Release Notes

Files (expected):
- `src/window/dialogs_help.rs`
- `README.md`
- `CHANGELOG.md`
- `RELEASE.md`

Status:
- [ ] Add FreeCell controls/rules to in-app help
- [ ] Update README feature matrix (Klondike + Spider + FreeCell)
- [ ] Add clear release notes for FreeCell maturity level
- [ ] Ensure no stale wording implies missing/disabled FreeCell features

---

## Pass 9: QA Matrix + 0.9.0 Gate

Commands:
- [ ] `cargo fmt`
- [ ] `cargo check -q`
- [ ] `cargo test -q`
- [ ] `scripts/release/maintainer-gate.sh --strict-tools`

Manual matrix:
- [ ] New game / seeded game / word seeded game
- [ ] Legal/illegal moves across all FreeCell destinations
- [ ] Undo/redo depth and restart-resume undo availability
- [ ] Mode switching with cross-mode undo/redo jumps
- [ ] Hint/Wand/Robot/W? behavior matches declared policy
- [ ] Resize stress (small/medium/large windows)
- [ ] Theme/userstyle sanity on FreeCell UI

Release gate decision:
- [ ] FreeCell feature-complete enough for 0.9.0
- [ ] FreeCell no longer disabled in main menu
- [ ] Backup/tag before release candidate (`backupct "freecell-0.9.0-rc"`)

---

## Immediate Next Phase (After Rest)

1. Lock FreeCell engine + boundary primitives (Pass 1).
2. Enable mode wiring behind capability policy (Pass 2).
3. Land session/undo/cross-mode integrity before UI polish (Pass 3).

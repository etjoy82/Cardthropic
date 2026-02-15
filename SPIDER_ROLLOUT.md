# Spider Rollout Checklist (0.7.0-dev)

Authoritative status tracker for Spider rollout without destabilizing Klondike.
This file is now stateful: it reflects what is already implemented in-tree.

## Current Snapshot

- Spider is now selectable and marked engine-ready.
- Spider is playable in alpha baseline (deal + tableau run moves).
- Spider has dedicated render path (10 tableau columns, stock, completed-run count).
- Core Spider tests and boundary tests exist and are active.
- Session/runtime separation for Spider vs Klondike is implemented and tested.
- Smart Move/Hint/Robot remain intentionally disabled for Spider via capabilities.

## Baseline Gate (Current Tree)

- [x] Spider docs added and tracked (`SPIDER_ROLLOUT.md`, `SPIDER_TEST_PLAN.md`)
- [x] `cargo test -q` has previously passed with Spider suite enabled
- [ ] Re-run full maintainer gate on latest tree before next checkpoint
- [ ] Create new checkpoint backup/tag before next large Spider pass

---

## Pass 1: Engine Rule Lock

Files:
- `src/game/spider.rs`
- `src/engine/boundary.rs`
- `src/engine/variant_engine/stubs.rs`
- `src/engine/variant_engine/mod.rs`

Status:
- [x] Spider legality primitives are implemented in game layer
- [x] Spider commands route via boundary/engine dispatch
- [x] Spider win-condition path is wired through engine boundary
- [x] Spider deal no-op behavior is covered at boundary when disallowed
- [ ] Finalize explicit policy notes for any intentional rule deviations

---

## Pass 2: Spider Test Matrix

Files:
- `src/game/tests.rs`
- `src/engine/tests.rs`

Status:
- [x] `spider_rule_001..010` implemented and active
- [x] Boundary routing tests added (`spider_boundary_001..005`)
- [x] Spider session/runtime tests added in engine test suite
- [x] Spider mode metadata/engine-ready assertions updated
- [ ] Add UI-level regression tests once Spider input behavior stabilizes further

---

## Pass 3: Session and History Integrity

Files:
- `src/engine/session.rs`
- `src/engine/variant_state.rs`
- `src/window/actions_history.rs`
- `src/engine/tests.rs`

Status:
- [x] Spider runtime persists through v2 session encode/decode tests
- [x] Mode-specific runtime isolation is tested (Spider does not mutate Klondike)
- [x] Undo/redo infrastructure is mode-aware and wired for Spider runtime snapshots
- [ ] Add focused Spider undo/redo parity tests at window layer

---

## Pass 4: Minimal Playable UI

Files:
- `src/window/variant_flow.rs`
- `src/window/menu.rs`
- `src/window/render.rs`
- `src/window/render_tableau.rs`
- `src/window/render_stock_waste_foundation.rs`
- `src/window/layout.rs`
- `src/window.ui`
- `src/window.rs`

Status:
- [x] Spider appears in mode selection flow
- [x] Tableau expanded to 10 columns in UI/template/layout
- [x] Mode-aware column layout added (`Spider => 10`, Klondike => 7)
- [x] Spider-specific rendering path is active
- [x] Spider stock and completed-run surface text present
- [ ] Polish Spider status/help copy to remove scaffold wording
- [ ] Validate resize/reflow behavior specifically for Spider edge sizes

---

## Pass 5: Input Contract and Move Safety

Files:
- `src/window/actions_selection.rs`
- `src/window/handlers_actions.rs`
- `src/window/drag.rs`
- `src/window/drag_setup.rs`
- `src/window/actions_moves.rs`

Status:
- [x] Spider click selection uses Spider-specific run-start mapping
- [x] Spider drag setup has Spider-specific path and texture generation
- [x] Move execute paths include face-up run guards in actions layer
- [ ] Add explicit defensive guards on every shortcut/action entrypoint for Spider-disabled features
- [ ] Add targeted invalid-drop status message consistency pass

---

## Pass 6: Smart Move / Hint / Robot Policy

Files:
- `src/window/hint_smart_move.rs`
- `src/window/hint_autoplay.rs`
- `src/window/hint_core.rs`
- `src/window/robot.rs`
- `src/engine/variant_engine/stubs.rs`
- `src/window/render.rs`

Status:
- [x] Spider capabilities explicitly disable smart move/autoplay/rapid wand/robot/winnability
- [x] Render controls are capability-sensitive (buttons disabled in Spider)
- [ ] Verify shortcuts/actions cannot execute disabled paths in Spider
- [ ] Make Spider help text explicitly document these policy constraints
- [ ] Remove/replace any remaining Spider scaffold status copy

---

## Pass 7: UX Text and Help Accuracy

Files:
- `src/window/dialogs_help.rs`
- `src/window/render.rs`
- `README.md`
- `CHANGELOG.md`

Status:
- [ ] Mode-aware help rows still need Spider-specific final wording
- [ ] Spider status text currently includes scaffold phrasing in default state
- [ ] README/CHANGELOG Spider-alpha section still needs sync with current implementation

---

## Pass 8: Release Hardening (for 0.7.0 pre-release)

Tasks:
- [ ] `cargo fmt`
- [ ] `cargo check -q`
- [ ] `cargo test -q`
- [ ] `scripts/release/maintainer-gate.sh --strict-tools`
- [ ] Manual Spider smoke matrix (deal/move/undo/redo/switch/resize)
- [ ] Backup + local checkpoint (`backupct "spider-pass8-rc"`)

---

## Immediate Next Phase

1. Hard-enforce Spider disabled-feature policy for shortcuts/actions.
2. Update help/status text to be explicit and mode-correct.
3. Run full gates and checkpoint once that pass is green.

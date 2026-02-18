# Board Resize Phase 1 Results

## Run Type
- Static code-path validation + targeted fix implementation.
- Note: full interactive GUI pass/fail matrix still requires manual runtime execution.

## Findings

### F1 (Confirmed) - Mode switch can skip top-row resize reconfiguration
- Symptom: FreeCell -> Klondike can retain stale top-row width requests until another geometry event.
- Root cause: top-row widget sizing cache key used only `(card_width, card_height)`, so mode switch with same card size hit early-return.
- Previously affected path:
  - `src/window/render_stock_waste_foundation.rs:17`
  - `src/window/render_stock_waste_foundation.rs:23`
  - `src/window.rs:340`
- Fix applied:
  - Cache key now includes active mode `(card_width, card_height, mode)`.
  - Updated fields:
    - `src/window.rs:340`
    - `src/window.rs:528`
    - `src/window/render_stock_waste_foundation.rs:16`
- Validation:
  - `cargo check -q` passed.

## Phase 1 Risk Review (remaining likely edge risks)

### R1 - Mode switch render depends on immediate render path, not explicit geometry invalidation
- Observation: `select_game_mode()` uses `render()` but does not explicitly force geometry invalidation.
- File:
  - `src/window/variant_flow.rs:127`
- Risk: if future cache keys miss a mode-specific parameter, issue can recur.

### R2 - Top-row width logic split between shared config + mode-specific overrides
- Observation: FreeCell overrides waste strip width after shared config.
- Files:
  - `src/window/render_stock_waste_foundation.rs:44`
  - `src/window/render_stock_waste_foundation.rs:203`
- Risk: divergence when shared/override assumptions change.

## Next Manual Runtime Checks (to complete Phase 1)
1. Re-run known repro: FreeCell -> Klondike without window resize.
2. Run full mode transition matrix at fixed window size (no manual resize between switches).
3. Re-run mode transitions with maximized on/off.
4. Verify rightmost tableau visibility after each switch.
5. Verify top-row widths reset correctly for each mode.

## Automated Desktop Overflow Coverage (Added)
- Added deterministic desktop overflow regression tests in `src/window/layout.rs`:
  - `desktop_layout_no_horizontal_tableau_overflow`
  - `desktop_layout_top_row_fits_for_all_modes`
- Coverage scope:
  - Modes: Klondike, Spider, FreeCell.
  - Resolutions: `800x600`, `1280x720`, `1920x1080`, `2560x1440`.
  - Maximized-state variants included.
- Validation command:
  - `cargo test -q desktop_layout_ -- --nocapture`
- Latest run:
  - `2 passed; 0 failed`.

## Debug Output Policy
- `layout_debug` and `resize_perf` status-history lines are now debug-gated.
- They emit only when Robot Debug Mode is enabled from the Automation menu.
- Normal gameplay runs no longer spam resize instrumentation lines.

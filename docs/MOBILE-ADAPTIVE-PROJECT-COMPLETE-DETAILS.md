# Mobile Adaptive Project Complete Details

## Overview

This document captures the completed work to make Cardthropic (CT) adaptive across extreme window sizes, including mobile-phone-class dimensions and large desktop/4K/8K displays.

Target behavior:

- Fluid resize from `250x250` through ultra-wide and high-resolution displays.
- Stable card scaling without runaway dimensions or stale paint states.
- Mobile-appropriate compact layout at small sizes.
- Correct control visibility behavior (HUD/tooling) under size pressure.
- No resize "fight back" behavior while dragging horizontally.

## Scope and Constraints

- GTK4 + libadwaita UI stack.
- Variant-aware layout (Klondike, Spider, FreeCell with different tableau widths).
- Continuous resize with live interaction (not discrete step-only adaptation).
- Maintain one render architecture; avoid separate mobile and desktop codepaths where possible.

## 1. Layout Engine Refactor

### Deterministic metric computation

Sizing now derives from a central metrics pipeline instead of scattered ad-hoc widget updates.

Computed metrics include:

- `card_width`
- `card_height`
- `face_up_step`
- `face_down_step`
- inter-column gap
- top-row and tableau fit caps

### Variant-aware column logic

- Klondike: 7 tableau columns
- FreeCell: 8 tableau columns
- Spider: 10 tableau columns

This is integrated into sizing and fit calculations so each variant scales correctly under the same adaptive model.

### Fit bounds

Explicit top-row/tableau width limits were added so stock/waste/foundation/tableau can coexist without pathological overflow or card oversizing.

## 2. Mobile Mode as Policy Layer

### Isolated policy, shared renderer

Mobile phone mode was implemented as a policy overlay, not a separate renderer branch.

Mobile policy controls:

- compact margins/padding
- tighter spacing and stack density
- small-window visibility behavior
- constrained card-step behavior for readability

### Breakpoint + hysteresis

Breakpoint logic uses hysteresis to avoid oscillation/flicker when dragging near threshold boundaries.

Result:

- stable mode transition near breakpoint
- no rapid desktop/mobile bouncing

## 3. Resize Pipeline Stabilization

### Coalesced geometry updates

Resize event storms are coalesced:

- geometry events mark state dirty
- one timed pass (about 16ms cadence) recomputes metrics and conditionally renders

### Render skip guard

A metrics-key/state comparison prevents unnecessary re-renders when effective geometry outputs are unchanged.

This removed redundant paint churn and improved interaction smoothness.

### Hard cache reset on mode transitions

Geometry-sensitive caches are explicitly reset when switching variants/modes to avoid stale width/spacing artifacts.

## 4. Overflow and Horizontal Resize "Fight Back" Fix

Root cause: strict "never horizontal scroll" behavior under intermediate resize states caused GTK to resist horizontal drag in some ranges.

Fix approach:

- Re-enable appropriate automatic horizontal scroller behavior during live resize.
- Ensure final settled layout converges to expected no-practical-overflow states for normal windows.

Result:

- horizontal drag no longer resists/shoves back
- vertical and horizontal resize both feel continuous

## 5. Card Rendering Consistency

### Drag/pickup scaling parity

Picked/dragged cards now obey the same current metrics as static cards, preventing stale-size drag visuals in compact mode.

### FreeCell oversize edge-case mitigation

The FreeCell-specific large-card blowout condition under aggressive resize was addressed through fit caps + metric normalization + cache invalidation discipline.

## 6. Compactness and Usable Space

Padding and spacing were tightened, especially for mobile-phone-class windows.

Adjustments included:

- removing unnecessary internal gaps
- reducing frame/row padding overhead
- improving first-column alignment in constrained mode (avoid clipping)
- tightening tableau stack behavior in low-height windows

Outcome: significantly more board area per pixel with better readability.

## 7. Instrumentation and Debugging

To make behavior measurable, structured diagnostics were added.

### `layout_debug`

Includes:

- window dimensions
- mobile mode state
- observed/live scroller widths
- available vs used width
- overflow delta
- column count
- card size
- top-row/tableau caps

### `resize_perf`

Includes:

- geometry event counts
- source attribution (poll/width/height/maximized)
- geometry-render vs total-render counts
- average/max render timing
- deck cache hit/miss/insert/clear counters

Diagnostics are gated behind debug toggles to avoid normal-mode log spam.

## 8. UX Integration Decisions

- HUD behavior and control-row behavior were tuned for small-window usability.
- Mobile compact behavior hides or compresses elements that become unusable at tiny scales.
- Functionality parity was preserved through menu/shortcut pathways even when visible controls are reduced.

## 9. Validation Approach

Validation covered:

- manual live resize sweeps across small/medium/large windows
- breakpoint crossing tests (desktop <-> mobile mode)
- variant-specific checks (Klondike, Spider, FreeCell)
- repeated horizontal stress drags (previous failure path)
- overflow regression checks
- compile/regression checks (`cargo check`, targeted layout tests/recipes)

Observed final behavior:

- fluid adaptation between `250x250` and very large displays
- no persistent horizontal resize lock/fight-back
- stable card scaling and layout convergence
- mobile mode usable and visually dense

## 10. Final State

The mobile-adaptive project is complete.

CT now has:

- one maintainable adaptive layout architecture
- breakpointed compact mode with hysteresis
- stabilized resize pipeline with coalescing and render guards
- instrumentation-backed confidence for future tuning
- consistent behavior from tiny windows to high-resolution displays

## 11. Follow-on Opportunities (Optional)

- add screenshot-based golden tests for a matrix of sizes/modes
- add a one-command benchmark suite for resize/paint timing deltas
- export `layout_debug` snapshots to structured artifacts for CI trend tracking


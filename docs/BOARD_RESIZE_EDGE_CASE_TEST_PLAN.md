# Board Resize Edge-Case Test Plan

## Goal
Validate that board geometry always reflows correctly when layout-affecting state changes, across Klondike, Spider, and FreeCell.

## Current Known Regression
- FreeCell -> Klondike mode switch can keep stale top-row sizing, causing incorrect board fit until an additional geometry event occurs.

## Scope
- Card width and height recalculation.
- Face-up and face-down tableau step recalculation.
- Top-row slot/heading/overlay width requests.
- Tableau column visibility and width requests.
- Scroll-area sizing behavior after mode and setting changes.

## Resize Invariants (must always hold)
- All active tableau columns are fully within the scroller viewport width at initial paint after a mode switch.
- No horizontal clipping of rightmost active tableau column in non-scroll-intended layouts.
- Top row does not overlap or leave stale mode-specific spacing.
- FreeCell free-cell strip and Klondike waste fan widths switch immediately on mode change.
- Spider foundation area remains hidden without leaving stale width pressure.
- Card dimensions and overlap ratios update within one render cycle after a trigger.

## High-Risk Trigger Matrix
Each case should be run at 600p, 720p, 1080p, and maximized desktop windows.

1. Window geometry triggers
- Resize width continuously smaller->larger.
- Resize height continuously smaller->larger.
- Toggle maximize on/off.
- Initial cold launch (first render before manual resize).

2. Mode transitions
- Klondike -> Spider.
- Spider -> Klondike.
- Klondike -> FreeCell.
- FreeCell -> Klondike.
- Spider -> FreeCell.
- FreeCell -> Spider.

3. In-mode layout modifiers
- Klondike deal switch 1..5.
- Spider suit switch 1..4.
- HUD toggle on/off.
- Status content growth (short status vs long status lines).

4. Content-depth stress
- Minimal tableau depth (new deal with many short stacks).
- Deep tableau depth (late game, long columns).
- Mixed face-up/face-down depths in Klondike/Spider.
- FreeCell with high occupancy in free cells and foundations.

5. Session/state transitions
- Resume saved session for each mode.
- Undo/redo that crosses mode boundaries.
- Seed restart in same mode.
- Robot-triggered reseed while window is resized.

## Mode-Specific Edge Cases

### Klondike
- Deal 5 with dense waste fan and long tableau columns at narrow width.
- Switch in from FreeCell while card size remains unchanged.
- Verify waste overlay width request reflects fan mode immediately.

### Spider
- 10-column fit after switching from 7-column Klondike and 8-column FreeCell.
- Foundations hidden without leftover reserved width.
- Stock/waste row remains aligned after suit-count changes.

### FreeCell
- Free-cell strip uses all 4 slots without over-compressing foundations.
- Transition in/out of FreeCell does not leave stale strip width in other modes.
- Right-edge gap between free cells and foundations remains mode-appropriate.

## Repro Script for Known Regression
1. Launch app in Klondike.
2. Switch to FreeCell.
3. Without resizing the window, switch back to Klondike.
4. Observe whether top row/tableau widths are immediately recalculated.
5. Pass condition: no stale spacing, no clipped columns, no need for manual resize.

## Instrumentation Checks
- Log card metrics on each render: mode, window size, scroller size, card width/height, face_up_step, face_down_step.
- Log top-row width requests: waste overlay, waste heading, foundations heading.
- Log metrics cache key before/after each trigger.
- Log whether geometry handler fired for trigger and whether render recomputed metrics.

## Pass/Fail Criteria
- Pass: all triggers produce correct layout on first repaint with no manual correction.
- Fail: any trigger requires a second action (resize, toggle, reopen) to correct layout.

## Execution Order (recommended)
1. Repro known regression first.
2. Run mode transition matrix at 720p.
3. Run geometry triggers at 1080p and maximized.
4. Run content-depth stress cases.
5. Run session/state transition cases.
6. Repeat failures with instrumentation enabled.

## Candidate Root-Cause Areas to Inspect if Failures Occur
- Cache keys that ignore mode-specific width-request differences.
- Early-return paths in top-row widget configuration when card size is unchanged.
- Missing `handle_window_geometry_change()` call sites on mode/settings transitions.
- Timing differences between mode switch render and scroller size notify.

## Reporting Template
- Case ID:
- Environment (resolution, maximized state, mode):
- Trigger:
- Expected:
- Actual:
- Repro frequency:
- Metrics log excerpt:
- Suspected subsystem:


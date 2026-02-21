# Handoff: 0.9.5-beta -> 0.10.0-beta (pre)

Date: 2026-02-20
Audience: LLM-to-LLM technical handoff (Claude Sonnet 4.5 baseline at `0.9.5-beta`)

## Scope + Confidence

- This summary describes the current `0.10.0-beta (pre)` workspace state relative to `v0.9.5-beta`.
- `HEAD` remained on a `0.9.5-beta` era commit while `0.10.0-beta` work existed in tracked + untracked working tree changes.
- Tracked diff summary against `v0.9.5-beta`: `74 files changed, 6894 insertions(+), 3846 deletions(-)`.
- Untracked files add additional delta not captured by that shortstat.

## 1) Release, versioning, and packaging hardening

- Project version advanced to `0.10.0-beta` across runtime/build/docs surfaces.
- `CHANGELOG.md` now documents `0.10.0-beta` themes and pre-release notes.
- `RELEASE.md` expanded with stronger post-release verification.
- New post-release helper script: `scripts/release/post-release-check.sh`.
- Flatpak/AppStream tooling hardened (repo URL handling, offline appstream refresh, metadata verification).

## 2) Rendering architecture shift: SVG/PNG -> CSS/Unicode

- Card rendering moved from SVG/PNG deck-sheet extraction to Unicode/CSS-style rendering.
- `src/window/render_cards.rs` became the primary card rendering path.
- `src/deck.rs` reduced to compatibility behavior instead of core raster pipeline.
- Card-sheet gresource wiring removed.
- Legacy card assets removed (SVG/PNG sheet artifacts).
- Direct `resvg` and `png` dependency usage removed from main path.

### Reported impact

- FreeCell ludicrous pacing: ~`40 ms/move`.
- Stable RSS observed around `38 MiB`.
- Lockfile package count reduced from `162` to `107`.

Reference doc: `docs/SVG_TO_CSS_UNICODE_TRANSITION.md`.

## 3) FreeCell layout and scaling fixes

- Fixed regression where top area height pushed tableau down as freecell count increased.
- Free cells + foundations now scale to horizontal window constraints.
- Tableau horizontal fit updated with adaptive width/gap behavior.
- Configurable freecell count (`1..6`) with setting-backed dialog/action.

## 4) Chess platform expansion and variant modularization

- Chess moved toward clearer module boundaries (`game`, `engine`, `window` layers).
- Variants unified under shared framework:
  - Standard Chess
  - Chess960
  - Atomic Chess
- Atomic was added under Chess960 in menu/action flow.
- Chess-mode render path now suppresses solitaire top zone.

## 5) Atomic Chess rules + robot safety

- Atomic capture/explosion semantics implemented in legal/apply paths.
- Variant-specific legality/check constraints integrated.
- Robot anti-loop guards strengthened for atomic and general chess loops:
  - repetition/cycle detection
  - explicit stop reasons in status

## 6) AI orchestration upgrades (Wand/W?/robot)

- Search orchestration improved to run off main thread where applicable.
- Cancellation behavior tightened:
  - re-invoking Wand while pending cancels active search
  - W? cancel behavior on repeated invocation
- Search lifecycle statuses now include richer metadata:
  - profile name
  - ply
  - configured think time
  - timing details and end-state telemetry
- “Will Finish mm:ss” derived from current game time + think budget.
- If game clock has not started (new game, no moves), “Will Finish …” is suppressed.

## 7) AI strength model normalization

- Strength controls separated so distinct channels no longer overwrite each other:
  - auto-response strength
  - Wand strength
  - W? strength
  - robot white strength
  - robot black strength
- Atomic mode now has full strength-mode coverage.
- W? strength no longer hijacks auto-response strength dialog/state.

## 8) Auto-response and manual interaction logic

- Auto-response now correctly resumes after manual board actions.
- Changing “Auto-Response Plays White/Black” triggers immediate move when side-to-move matches selected side.
- Opening-side auto behavior integrates with new-game flow where configured.

## 9) Chess board UX and input behavior

- Last-move highlighting expanded and strengthened:
  - origin square highlight
  - destination square highlight
- Added subtle coordinate overlays (`A-H`, `1-8`) on board edges.
- Coordinate overlays are flip-aware and orientation-correct.
- New setting + shortcut toggle for coordinate visibility.
- Flipped-board keyboard navigation fixed (left/right inversion resolved for arrows/WASD/HJKL/numpad patterns).

## 10) System sounds without extra Flatpak audio permissions

- Added chess system-sounds toggle backed by settings.
- Move notifications use system event/beep path.
- Rate-limited to avoid spam (`>= 2s` between triggers).
- Designed to avoid introducing new Flatpak sound permissions.

## 11) Session restore and chess state persistence

- Startup session restore fixed for chess modes.
- Saved session payload now includes chess variant/state/history/future/last-move fields.
- New-game status now logs date/time/timezone context.

## 12) Status history and note workflow

- Status history window iterated to non-modal UX.
- Added `Insert Note` flow:
  - available in status history and game-state menu
  - shortcut bound to `Ctrl+Shift+N`
  - default note text: `Note: `
  - Enter save, Escape cancel, Ctrl+Enter newline
- Copy action became “Copy All”.
- Retention default adjusted to `1000` lines.
- Line/char counters and controls repositioned per UX refinements.
- Klondike controls spam guarded to avoid noisy repeated status entries.

## 13) GSettings export/import hardening

- Added action to copy all Cardthropic settings payload to clipboard.
- Added strict import path for full settings payload restore.
- Validation is fail-fast and strict:
  - unknown keys rejected
  - unsupported metadata rejected
  - duplicate keys rejected
  - type/value mismatch rejected
  - missing keys rejected
- Apply path is transactional: invalid payload refuses load.

## 14) Main-window command affordance and shortcut labeling

- Command button moved to titlebar first slot.
- Label changed to `/`.
- Seed submit label changed from `Go` to `Enter`.
- Shortcut hints appended across main game window controls, including seed controls.

## 15) Overall interpretation

`0.10.0-beta (pre)` is a substantial platform step from `0.9.5-beta`, not a patch-level change. The core themes are:

- renderer simplification + measurable perf/memory gains
- chess subsystem maturity (variants, async AI orchestration, anti-loop controls)
- stronger packaging/release reliability
- improved reproducibility and observability (status/history/settings tooling)

## Caveat for downstream consumers

Treat this as a pre-release workspace handoff snapshot. It includes meaningful tracked + untracked changes that may not yet be represented as a single immutable tag/release artifact.

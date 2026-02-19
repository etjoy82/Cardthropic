# GSettings Audit (0.9)

This document audits Cardthropic feature/state persistence and explains why some state is not gsettings-backed.

## Scope
- User-facing toggles and settings
- Variant/mode selectors
- Session/game continuity state
- Runtime-only state (robot/search/render/memory internals)

## A) GSettings-backed settings (authoritative)

Schema: `data/io.codeberg.emviolet.cardthropic.gschema.xml`  
Loaded in: `src/window/theme_core.rs` via `gio::Settings`

### Appearance and UX
- `board-color`  
Purpose: board background color  
Write path: `src/window/theme_color.rs`
- `custom-userstyle-css`  
Purpose: active custom CSS  
Write path: `src/window/theme_userstyle.rs`
- `saved-custom-userstyle-css`  
Purpose: saved custom CSS buffer for preset round-trips  
Write path: `src/window/theme_userstyle.rs`, migration write in `src/window/theme_core.rs`
- `custom-userstyle-word-wrap`  
Purpose: custom CSS editor word-wrap toggle  
Write path: `src/window/theme_userstyle.rs`
- `custom-card-svg`  
Purpose: user-pasted card SVG sheet  
Write path: `src/window/theme_menu.rs`
- `interface-emoji-font`  
Purpose: optional UI font override  
Write path: `src/window/theme_core.rs`
- `enable-hud`  
Purpose: HUD visibility toggle  
Write path: `src/window/state.rs`
- `smart-move-mode`  
Purpose: smart-move input behavior  
Write path: `src/window/state.rs`

### Variant preferences
- `spider-suit-mode`  
Purpose: Spider suit count preference  
Write path: `src/window/variant_flow.rs`
- `freecell-card-count-mode`  
Purpose: FreeCell card-count preference  
Write path: `src/window/variant_flow.rs`

### Automation toggles
- `forever-mode`  
Purpose: robot auto-reseed behavior  
Write path: `src/window/state.rs`
- `robot-auto-new-game-on-loss`  
Purpose: auto-start new game after loss  
Write path: `src/window/state.rs`
- `ludicrous-speed`  
Purpose: fast robot step interval  
Write path: `src/window/state.rs`
- `robot-debug-enabled`  
Purpose: debug/status verbosity and startup trace history emission  
Write path: `src/window/state.rs`
- `robot-strict-debug-invariants`  
Purpose: strict invariant checks in robot mode  
Write path: `src/window/state.rs`

### Memory guard
- `memory-guard-enabled`  
Purpose: enable/disable guard behavior  
Write path: `src/window/state.rs`
- `memory-guard-soft-limit-mib`  
Purpose: soft guard threshold  
Read on startup: `src/window/theme_core.rs`
- `memory-guard-hard-limit-mib`  
Purpose: hard guard threshold  
Read on startup: `src/window/theme_core.rs`

### Persisted stores
- `saved-session`  
Purpose: full crash-safe resume payload  
Write path: `src/window/session.rs`
- `seed-history`  
Purpose: serialized per-seed play/win history  
Write path: `src/window/seed_history.rs`

## B) User-facing features not directly gsettings-backed

These are intentional and backed by session/runtime logic instead.

- Klondike draw mode (`Deal 1..5`)  
Backing: in-memory + persisted inside `saved-session` payload (`draw=` field).  
Why not separate gsettings key: current deal behavior is treated as session state; avoids split source-of-truth with session restore.

- Active game mode (Klondike/Spider/FreeCell)  
Backing: in-memory + `saved-session` payload (`mode=` field).  
Why not separate gsettings key: mode is part of resumable game session, not a global preference independent of session.

- Current seed / board position / move count / elapsed time / undo+redo stacks  
Backing: `saved-session` payload and snapshot stack fields in `src/window/session.rs`.  
Why not separate gsettings keys: high-churn, structured game state is better serialized as one session blob.

- Manual actions (`Draw`, `Undo`, `Redo`, `Wand`, `Rapid Wand`, `Peek`, `Copy/Paste Game State`)  
Backing: runtime actions only.  
Why not gsettings-backed: these are commands, not preferences.

- Status history / APM graph windows and ephemeral status text  
Backing: runtime buffers/timers.  
Why not gsettings-backed: transient UI telemetry; persisted values would be stale/noisy across launches.

## C) Runtime state intentionally not persisted to gsettings

Examples (from `src/window.rs` state fields):
- Render/layout caches and metrics (`tableau_picture_state_cache`, deck cache flags, geometry perf counters)
- In-flight operation state (seed-check/search cancel tokens, timers, drag state)
- Robot planner internals and counters (recent hashes/signatures, drought/cycle stats, CPU sample fields, fallback streaks)
- Memory guard runtime latches (`memory_guard_soft_triggered`, `memory_guard_hard_triggered`, dialog cooldown timestamp)

Why these are not gsettings-backed:
- They are ephemeral process state.
- Persisting them would risk stale behavior after restart.
- They are either recalculated or intentionally reset each launch/session.

## D) Potentially surprising edge (documented)

- `memory-guard-soft-limit-mib` and `memory-guard-hard-limit-mib` are gsettings-backed but currently not changed by an in-app control.  
Effect: they can be configured externally (e.g., `gsettings set`), and are applied on startup.

## E) Bottom line

- Persistent user preferences/toggles are gsettings-backed.
- Full gameplay continuity is persisted via `saved-session` (single serialized payload), not many discrete keys.
- Non-gsettings state is intentionally transient runtime data or command execution state.

## F) Exhaustive Field Matrix

- Full one-row-per-field matrix: `docs/STATE_FIELD_MATRIX.md` (all `imp::CardthropicWindow` fields, with persistence classification and rationale).

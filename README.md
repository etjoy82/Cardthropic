# Cardthropic

Cardthropic is a Libadwaita + Rust GNOME Solitaire app focused on Klondike.

Current project version: `0.2.1`

## Changelog

### 0.2.1 - 2026-02-11

- Fixed tableau column pixel-shift jitter when moves changed empty/non-empty piles.
- Added `Rapid Wand` (`Ctrl+Shift+Space`) and middle-click wand trigger (non-stackable timed burst).
- Fixed Builder + installed Flatpak dock/taskbar icon resolution by aligning runtime mapping and packaging assets.

### 0.2.0 - 2026-02-11

- Initial public preview with Klondike gameplay and adaptive layout.

## Highlights

- Native GTK4/Libadwaita desktop UI.
- Drag-and-drop Klondike gameplay with clickable alternatives.
- Undo/redo history.
- Magic Wand autoplay system (`🪄`).
- Seeded deals with in-window seed controls.
- Winnability checks and winnable-seed generation.
- Persistent seed history with plays/wins tracking.
- Adaptive card sizing across common desktop viewports.
- Tableau column layout stabilized to eliminate pixel shifting when piles become empty/non-empty.
- Shared game-mode scaffold for Klondike, Spider, and FreeCell (Klondike engine implemented first).

## Controls

- Click stock to draw.
- Click waste once to select it for manual placement.
- Double-click waste to trigger Smart Move auto-play (when Smart Move is enabled).
- Click tableau cards/runs to select and move.
- Double-click any tableau card/run to trigger Smart Move when enabled.
- Drag waste or tableau runs onto tableau/foundation targets.
- `Undo` / `Redo` via toolbar buttons or keyboard shortcuts.
- `Ctrl+Y` redoes the last undone move.
- `🪄` waves the magic wand and plays the current best move.
- `🌀` cyclone-shuffles all tableau cards while preserving each column's face-up/face-down shape.
- `🫣` Peek: for 3 seconds, tableau face-up cards are hidden and face-down cards are revealed.
- `Smart Move` (hamburger checkbox) controls double-click auto moves and waste auto-play.
- `Ctrl+Space` waves the magic wand.
- `F2` toggles Smart Move.
- `F3` triggers Peek.
- `F5` triggers Cyclone Shuffle Tableau.
- `Ctrl+R` starts a random seeded game.
- `Ctrl+Shift+R` starts a winnable seeded game search.
- `F1` opens About Cardthropic.

## Seed Tools

- `🎲` starts a new random seeded game.
- `🛟` starts a new winnable seeded game.
- Seed field is editable and also supports selecting prior seeds.
- `Is Seed W?` checks the current seed and reports analysis in the status line.

## Appearance

- Hamburger menu includes a board-color picker with swatches.
- Hamburger menu includes quick game-mode buttons (`🥇` Klondike, `🕷️` Spider, `🗽` FreeCell).
- Header bar includes a game-mode emoji settings menu that changes with the active game.
- Curated themes (`Felt`, `Slate`, `Sunset`, `Ocean`) are available in the same picker.
- `Reset Default` restores the default board color.
- Board color changes animate smoothly.

## Build

### Cargo

```bash
cargo check
cargo run
```

### GNOME Builder / Meson

This repository includes Meson and Flatpak metadata for GNOME-style development.

### Dev Runtime

- Flatpak development is pinned to stable GNOME runtime `org.gnome.Platform//48`.
- Using the stable runtime avoids drag-related GTK warnings seen on `master`.

## Project Status

Cardthropic is actively developed. Core Klondike play is in place, and ongoing work is focused on polishing responsiveness, ergonomics, and Circle-readiness.

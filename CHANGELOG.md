# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed
- Documentation and maintainer release tooling continue to be refined for the beta testbed workflow.


## [0.9.5-beta] - 2026-02-18

### Changed
- Documentation and maintainer release tooling continue to be refined for the beta testbed workflow.
- Release helper scripts now accept SemVer prerelease identifiers (for example, 0.9.5-beta).

## [0.9.0-beta.1] - 2026-02-18

### Added
- Automation toggle `Auto start new game on loss` with GSettings backing, menu integration, and shortcut support.
- Debug-only robot invariant checks with a strict runtime toggle to catch state-rule regressions earlier during automation.

### Fixed
- Spider rule enforcement for completed same-suit A-K run clearing.
- Spider stock dealing rule: dealing is blocked unless every tableau column is occupied.
- Spider `W?`/winnable-seed flow reliability and robot handoff behavior.
- Seed input now accepts underscore-containing word seeds consistently and reflects them correctly in status text.
- Return/Enter key behavior no longer steals seed-box input focus.


## [0.8.0] - 2026-02-15

### Changed
- Major Spider expansion: suit selector (1/2/3/4), full engine parity work, and broad move/interaction reliability improvements.
- Automation upgrades across Spider and Klondike, including improved Robot behavior, richer status reporting, and reduced loop-prone exploration.
- Added Spider `W?` seed winnability analysis path with solver-line handoff to Robot mode when available.
- Added Smart Move support for Spider and aligned click-mode behavior (single/double/disabled) with mode-specific move handling.
- UI polish: Spider foundations are hidden, status log auto-scroll is more reliable to true bottom, and FreeCell mode is visibly disabled in the menu until engine-ready.

## [0.6.0] - 2026-02-13

### Changed
- Major maintainer workflow hardening: strict shell policy checks, release consistency checks, shell lint policy, and one-command gate targets.
- Added CI enforcement via Woodpecker to run maintainer-gate in strict mode on push and pull requests.
- Improved Flatpak release/publish flow with integrated AppStream verification and tighter maintainer-only script conventions.
- Expanded gameplay/automation reliability with continued solver, robot, and winnability-path improvements.
- Refined theming and UI behavior, including theme menu/userstyle workflow and rendering/session polish.

## [0.5.1] - 2026-02-13

### Fixed
- Solver/automation memory growth by making parallel winnability work cancelable and joining worker threads cleanly.
- Background CPU churn after robot runs by replacing per-frame geometry polling with interval polling and canceling stale loss-analysis work.

### Changed
- Added bounded caches for autoplay/loss-analysis state to prevent unbounded retention in long sessions.
- Reduced background disk writes with debounced session persistence while preserving resume safety.
- Added card texture caching to reduce repeated texture allocation churn during rapid render/automation loops.

## [0.5.0] - 2026-02-13

### Added
- Expanded theming system with curated presets and improved `🎨` theme popover UX.
- Custom CSS editor upgrades: dark code editor scheme, clipboard actions/shortcuts, font-size control, resizable/snap/maximize-capable dialog behavior.

### Changed
- Smart Move correctness improvements and stronger solver-aligned move logic.
- Deep engine/window modularization for long-term maintainability and variant expansion.
- Metadata refresh with new screenshot and release details.

## [0.3.1] - 2026-02-12

### Fixed
- Appearance controls and interaction polish hotfixes.

## [0.3.0] - 2026-02-12

### Changed
- Gameplay polish with smarter automation, expanded controls, persistent sessions, and refreshed Flatpak metadata/screenshots.

## [0.2.1] - 2026-02-11

### Fixed
- Tableau pixel-shift jitter.
- Dock/taskbar icon resolution for Builder and Flatpak installs.

### Added
- Rapid Wand automation.

## [0.2.0] - 2026-02-11

### Added
- Initial public preview with Klondike gameplay and adaptive layout.

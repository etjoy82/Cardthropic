# SVG to CSS/Unicode Card Rendering Transition

Date: 2026-02-20

## Summary

Card rendering moved from a rasterized SVG/PNG deck-sheet pipeline to a CSS + Unicode card pipeline.

The practical goals were:

- remove renderer stutter during rapid robot playback
- simplify rendering/deck code and reduce maintenance surface
- remove heavyweight raster/SVG dependencies
- reduce asset footprint and resource wiring complexity

## What Changed

### Before (SVG/PNG sheet pipeline)

- `src/deck.rs` loaded/rasterized `data/cards/anglo.svg` (fallback to `anglo.png`)
- per-card textures were extracted from a pre-rendered deck sheet
- card asset was wired through gresource (`src/cardthropic.gresource.xml`)
- dependencies included direct `resvg` and `png`

### After (CSS + Unicode pipeline)

- cards are drawn from Unicode glyphs and CSS-like styling in the GTK render path (`src/window/render_cards.rs`)
- `src/deck.rs` became a tiny compatibility shim/stub
- card-sheet asset wiring was removed from gresource
- direct `resvg` and `png` dependencies were removed

## Performance + Memory Baseline

Measured on 2026-02-20 and recorded in `benchmarks/freecell_baseline.json`:

- FreeCell robot mode (`ludicrous`): `~40 ms/move`
- Visual stutter: `none observed`
- RSS: `38 MiB` stable (`peak_rss_mib=38`, `steady_rss_mib=38`)

Baseline metadata now includes:

- `render_pipeline.cards = "css-unicode"`
- `render_pipeline.robot_ludicrous_ms_per_move = 40`
- `memory.steady_rss_mib = 38`

### Measurement Method

- Scenario: FreeCell gameplay, Robot enabled, Ludicrous speed enabled.
- Move pacing: observed from robot progression timestamps over sustained runs.
- Memory: process RSS sampled from Linux status telemetry and confirmed as stable at 38 MiB.
- Baseline snapshot persisted in `benchmarks/freecell_baseline.json`.

## Dependency Impact

Direct dependency removals:

- `resvg`
- `png`

Lockfile package count:

- before: `162`
- after: `107`
- delta: `-55` packages (~34% reduction)

## File/Asset Size Impact

Observed repository-level delta across transitioned/removed-added files:

- removed: `7,600,554` bytes (`7.25 MiB`)
- added: `1,731,224` bytes (`1.65 MiB`)
- net: `-5,869,330` bytes (`-5.60 MiB`)

Breakdown highlights:

- card deck assets removed (`data/cards/anglo.svg`, `data/cards/anglo.png`, `data/cards/README.anglo`):
  - `-1,296,861` bytes (`-1.24 MiB`)
- icon asset transition (`*.svg` -> `*.png` in current tree):
  - net `-4,571,959` bytes (`-4.36 MiB`)

## Code Surface Impact

- `src/deck.rs` reduced from `493` lines to `91` lines.
- gresource card asset entry removed from `src/cardthropic.gresource.xml`.
- rendering logic is now concentrated in the Unicode render path (`src/window/render_cards.rs`) and mode renderers.

## Build + Packaging Impact

- install rules switched scalable/symbolic app icons from `.svg` to `.png` in `data/icons/meson.build`
- card deck asset is no longer bundled through gresource
- fewer crates and less renderer-specific code simplify local dev and CI dependency churn

## Regression Guardrails

Use:

- `just perf-gate` for current Unicode rendering baseline thresholds
- `just perf-freecell` / `just perf-record` for deeper profiling

When changing card rendering again:

1. re-measure robot ludicrous move pacing
2. re-check steady RSS
3. update `benchmarks/freecell_baseline.json`
4. update this document with new numbers and date

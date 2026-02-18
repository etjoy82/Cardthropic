# FreeCell AI Workplan (Smart Move + Wand + Robot + W?)

Focused test list for improving FreeCell automation quality before `0.9.0`.

## Progress Log

- 2026-02-16: Pass 1 landed
- linked from `FREECELL_ROLLOUT.md`
- increased FreeCell search depth/beam for Balanced/Deep
- shifted evaluation to encourage tactical use of multiple free cells (not just preserving empties)
- added unlock-aware scoring for `tableau -> freecell`
- added stronger freeing-space bias for `freecell -> tableau`
- added explicit repeat-state and 2-cycle loop penalties for FreeCell move selection
- debug decision lines now include `loop_penalty` and `repeat_distance`
- disallowed FreeCell foundation rollback moves in rules, AI candidates, and winnability successors
- added invariant test: `freecell_foundation_to_tableau_is_disallowed`
- added FreeCell loss detection (`is_lost`) from legal-move exhaustion
- added robot action-mix counters (`t2f`, `c2f`, `t2t`, `t2c`, `c2t`) and `fc_peak_used`
- added explicit 5-ply short-loop scan penalty (`loop_scan5_penalty`) in FreeCell move selection
- verified with `cargo check -q` and `cargo test -q freecell_`
- 2026-02-16: Pass 2 in progress
- added `fc_moves` and `fc_touch_pct` private metrics to FreeCell robot benchmark/status suffix
- increased tactical bias toward multi-cell utilization when mobility is constrained
- added utilization-aware transition bias for `t2c` and `c2t` candidate scoring

## Goal

- Make FreeCell automation reliably use all 4 free cells.
- Improve win rate and reduce low-value loops.
- Keep non-debug UX clean while keeping debug logs analyzable.

## Test Setup

- Build: `cargo check -q`
- Core tests: `cargo test -q`
- FreeCell-focused tests: `cargo test -q freecell_`
- Run app in FreeCell mode with fixed seeds for A/B comparisons.

## Phase 1: Correctness Baseline

- [x] `smart move` never proposes an illegal move.
- [x] `wand` only executes legal moves.
- [x] `robot` only executes legal moves.
- [ ] `W?` path is legal from initial state to terminal state.
- [ ] No state corruption after 500+ robot moves.
- [ ] Undo/redo remains valid during and after automation.

## Phase 2: Free Cell Utilization

- [ ] Add metric: `freecell_peak_used` per run.
- [ ] Add metric: `% moves that touch free cells`.
- [ ] Add metric: `freecell_to_tableau`, `tableau_to_freecell`, `freecell_to_foundation`.
- [ ] Target: solver uses more than one free cell when tactically useful.
- [ ] Verify no bias toward keeping 3-4 free cells permanently idle.

## Phase 3: Heuristic Scoring Improvements

- [ ] Prioritize moves that reveal hidden cascade mobility.
- [ ] Reward freeing blocked low-rank foundation starters (A/2/3).
- [ ] Penalize reversible loop patterns (A->B->A churn).
- [ ] Reward preserving empty cascades when they increase move capacity.
- [ ] Reward opening a free cell if all 4 are occupied and mobility is low.
- [ ] Add tie-breaker stability so repeated seeds produce deterministic choices.

## Phase 4: Wand Strategy Quality

- [ ] Wand should prefer high-confidence progress moves over neutral churn.
- [ ] Wand should use free cells as temporary buffers, not long-term parking.
- [ ] Wand should opportunistically push safe foundation moves.
- [ ] Wand should stop with clear reason if no progress move exists.

## Phase 5: Robot Strategy Quality

- [ ] Robot should avoid repeated no-progress cycles.
- [ ] Robot should detect stagnation and switch fallback heuristic tier.
- [ ] Robot should report `stop_reason` when halting (`won`, `stuck`, `budget`, `guard`).
- [ ] Robot should expose per-run stats in debug mode only.
- [ ] Non-debug status remains concise and human-readable.

## Phase 6: W? (Winnability) Validation

- [ ] Validate `W?` result against independent replay of returned path.
- [ ] Ensure impossible seeds report clear failure mode.
- [ ] Ensure budget-limited checks report budget exhaustion explicitly.
- [ ] Ensure handoff to Robot/Wand does not lose solver intent.

## Benchmark Matrix

- [ ] Draw deterministic seed set (at least 100 seeds).
- [ ] Run baseline strategy over all seeds.
- [ ] Run candidate strategy over same seeds.
- [ ] Compare:
- [ ] `win_rate_pct`
- [ ] `avg_moves_to_win`
- [ ] `median_time_to_terminal`
- [ ] `loop_guard_triggers`
- [ ] `freecell_peak_used`
- [ ] Keep CPU/memory notes for long runs.

## Acceptance Gates

- [ ] Legal move integrity: 100%.
- [ ] No crash/panic in 1,000-run robot batch.
- [ ] Win rate improves vs baseline on fixed seed matrix.
- [ ] Free cell utilization shows tactical multi-cell use.
- [ ] Status/debug output matches project format standards.

## Logging Schema (Debug Mode)

- [ ] Include: mode, seed, move index, move kind, source, destination.
- [ ] Include: free cell occupancy snapshot (`F1..F4`).
- [ ] Include: mobility metrics (empty cascades, legal move count estimate).
- [ ] Include: score components and final composite score.
- [ ] Include: loop/stagnation counters and guard decisions.

## Daily Loop (Practical)

- [ ] Pick one heuristic change.
- [ ] Run fixed-seed benchmark batch.
- [ ] Compare metrics against previous snapshot.
- [ ] Keep change only if objective metrics improve or regressions are acceptable.
- [ ] Record result in `AI_BENCHMARK_TUNING.md`.

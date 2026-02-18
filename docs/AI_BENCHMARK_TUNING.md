# AI Benchmark + Tuning Process

Process guide for benchmarking and improving automation quality across Klondike and Spider.
Use this as the repeatable loop before changing AI scoring logic.

## Goals

- Produce reproducible AI win-rate and efficiency metrics.
- Compare strategies (`fast`, `balanced`, `deep`) fairly.
- Tune heuristics with measurable improvement, not vibes.
- Prevent regressions with targeted tests.

## Benchmark Matrix

Run all combinations below as a standard matrix:

- Modes:
- Klondike
- Spider 1-suit
- Spider 2-suit
- Spider 3-suit
- Spider 4-suit

- Strategy profiles:
- fast
- balanced
- deep

- Run lengths (minimum):
- smoke: 25 runs
- candidate: 100 runs
- release gate: 500+ runs

Keep rule/config fixed during each run batch:

- same mode + suit/deal settings
- same strategy profile
- Forever Mode on
- no manual interventions

---

## Private Metrics Contract

Use status lines as source of truth.

Primary fields to capture:

- `runs`
- `wins`
- `losses`
- `win_rate_pct`
- `strategy`
- `mode`
- `draw`
- `forever`
- `robot_moves`
- `deals`
- `elapsed_s`
- `mem`

Primary events:

- periodic benchmark dumps (`bench_v=1 ...`)
- reseed-on-win (`won_reseed`)
- reseed-on-loss (`stuck_or_lost`)

---

## Run Procedure

1. Select mode and suit/deal config.
2. Select automation strategy.
3. Start robot in Forever Mode.
4. Let it run until target sample size is reached.
5. Use `Copy Benchmark Snapshot` (`Ctrl+Shift+B`) at checkpoints.
6. Save snapshots to a local notes file/spreadsheet.
7. Repeat for every matrix row.

Checkpoint cadence:

- Every 25 runs minimum.
- Always record final checkpoint.

---

## Acceptance Thresholds (Initial)

Use these as tuning targets, then tighten as AI improves.

- Klondike draw-1:
- no new tuning accepted if win rate drops by >1.5% vs baseline at 100 runs.

- Spider 1-suit:
- should stay high and stable; regressions >2% at 100 runs block merge.

- Spider 2/3/4-suit:
- measure trend first; accept changes only if win rate improves or equal with better speed/memory.

Efficiency guardrails:

- No sustained memory growth beyond expected baseline envelope.
- No runaway loop behavior increase.

---

## Tuning Workflow (Per Change)

1. Hypothesis:
- Write one sentence: what behavior is wrong and why this change should help.

2. Change set:
- Keep patch narrow (one heuristic family at a time).

3. Fast validation:
- 25-run smoke on affected matrix rows.

4. Candidate validation:
- 100-run pass on affected rows.

5. Regression scan:
- At least one non-target row (e.g., Klondike if tuning Spider).

6. Decide:
- Promote / iterate / revert.

---

## Test-Driven Coverage Plan

Create or update tests with every meaningful heuristic or move-policy change.

Test categories:

- `ai_rule_*`
- deterministic legality + invariant guards

- `ai_scoring_*`
- ordering and preference tests for move ranking

- `ai_loop_*`
- anti-loop and repetition penalties

- `ai_benchmark_*`
- coarse performance sanity checks (non-flaky thresholds only)

- `ai_strategy_*`
- profile-specific behavior differences (`fast` vs `deep`)

Minimum per tuning PR:

- 1 targeted unit test for the tuned heuristic
- 1 regression test for previously fixed edge case

---

## Suggested Test Cases

- Spider 2-suit:
- prefers reveal move over non-reveal cosmetic move
- avoids breaking long suited run unless reveal or mobility gain
- prefers creating empty column when legal alternatives are equal

- Klondike:
- avoids reversible waste-tableau oscillation
- respects foundation progress without dead-ending tableau mobility

- Cross-mode:
- strategy selection changes lookahead/beam behavior deterministically

---

## Logging + Reporting Template

Use this format in notes/changelog when reporting benchmark results:

- Config: `<mode>/<suit-or-draw>, strategy=<profile>, runs=<N>`
- Result: `wins=<W> losses=<L> win_rate=<P>%`
- Efficiency: `avg elapsed/run=<...>s, mem=<...>`
- Decision: `promote | iterate | revert`
- Notes: one sentence on observed behavior

---

## Release Gate (AI)

Before release candidate:

- [ ] Matrix run complete for required rows
- [ ] No critical regressions vs previous baseline
- [ ] Tuning-related tests added and green
- [ ] `cargo check -q`
- [ ] `cargo test -q`
- [ ] Bench snapshots archived for release notes

---

## Immediate Next Steps

1. Establish baseline table for all current strategy/mode rows.
2. Prioritize Spider 2-suit tuning first (currently weakest row).
3. Add first targeted `ai_scoring_spider_2s_*` tests before heuristic edits.

# Tooling Workflow

This project uses local Rust and system tooling to keep quality and performance high.

## Daily Loop

1. `just doctor` once per machine/session to verify tools.
2. `just bacon` while coding.
3. `just test-freecell` for focused gameplay changes.
4. `just ci-local` before push.

## Weekly Hygiene

1. `just deps-weekly`
2. `just cov`
3. `just size`

## CI Mapping

- Push / PR pipeline runs:
  - `scripts/release/maintainer-gate.sh --strict-tools --skip-cargo`
  - `just ci-local`
- Cron pipeline runs:
  - `just ci-nightly`
  - emits `reports/ci-bundle.tgz` (lcov + size + dependency/security reports)
  - runs `scripts/ci/upload-artifacts.sh reports/ci-bundle.tgz`

## Recipes

- `just check`: compile sanity check.
- `just doctor`: verify toolchain + tool availability.
- `just fmt-check`: format verification.
- `just clippy`: strict lint gate (`-D warnings`).
- `just clippy-warn`: non-fatal lint pass for day-to-day flow.
- `just test-nextest`: fast full tests with `cargo-nextest`.
- `just cov` / `just cov-ci`: coverage reports with `cargo-llvm-cov`.
- `just audit`: RustSec advisories.
- `just deny`: policy checks (licenses/advisories/bans).
- `just deps-udeps`: unused dependencies with nightly.
- `just deps-machete`: unused dependencies quick pass.
- `just deps-outdated`: dependency update view (if installed).
- `just size`: binary size report.
- `just msrv`: verify minimum supported Rust version.
- `just perf-freecell` / `just perf-record`: CPU profile tools.
- `just perf-gate`: prints current Unicode render regression guardrails from baseline.
- `just ci-artifacts`: generate report bundle at `reports/ci-bundle.tgz`.

## Artifact Upload Config

The nightly upload step is opt-in and no-ops unless configured.

- HTTP PUT backend:
  - `ARTIFACT_UPLOAD_URL`
  - optional `ARTIFACT_UPLOAD_TOKEN`
- S3 backend:
  - `ARTIFACT_S3_URI`
  - standard AWS credentials/env vars

## Notes

- `deps-udeps` requires nightly Rust.
- `deps-outdated` is optional and skips if not installed.
- recipes now fail with explicit guidance when required tools are missing.
- Keep `justfile` as the single command entrypoint.
- Toolchain is pinned in `rust-toolchain.toml` (stable + `rustfmt` + `clippy`).
- Dependency/license/security policy is pinned in `deny.toml`.
- Clippy policy tuning is tracked in `clippy.toml`.

set shell := ["bash", "--noprofile", "--norc", "-eu", "-o", "pipefail", "-c"]

# Show available recipes.
default:
    @just --list

timestamp := `date +%Y%m%d-%H%M%S`

# Check tool availability and versions.
doctor:
    @echo "Rust toolchain:"
    @cargo --version
    @rustc --version
    @echo
    @for t in \
      just rg fd bat tokei hyperfine perf gdb lldb \
      cargo-nextest cargo-llvm-cov cargo-deny cargo-audit cargo-bloat cargo-machete cargo-msrv bacon; do \
      if command -v "$t" >/dev/null 2>&1; then \
        printf "  %-20s yes\\n" "$t"; \
      else \
        printf "  %-20s no\\n" "$t"; \
      fi; \
    done

# Fast compile check.
check:
    cargo check -q

# Format code.
fmt:
    cargo fmt

# Verify formatting.
fmt-check:
    cargo fmt -- --check

# Lints.
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# Baseline lint pass that remains usable while clippy debt is being paid down.
clippy-warn:
    cargo clippy --all-targets --all-features

# Run full test suite.
test:
    cargo test -q

# Run tests with nextest (preferred).
test-nextest:
    if command -v cargo-nextest >/dev/null 2>&1; then \
      cargo nextest run --all-features; \
    else \
      echo "cargo-nextest not installed; falling back to cargo test"; \
      cargo test --all-features; \
    fi

# Run focused FreeCell tests.
test-freecell:
    cargo test -q freecell_

# Run desktop layout overflow regression tests (Klondike/Spider/FreeCell).
test-overflow-desktop:
    cargo test -q desktop_layout_

# Run desktop overflow checks plus the full resize suite.
test-overflow-all:
    just test-overflow-desktop
    cargo test -q

# Run focused winnability tests.
test-winnability:
    cargo test -q winnability

# Continuous dev loop using cargo-watch.
watch:
    if command -v cargo-watch >/dev/null 2>&1; then \
      cargo watch -x 'check -q' -x 'test -q freecell_'; \
    else \
      echo "cargo-watch not installed"; \
      exit 1; \
    fi

# Continuous dev loop using bacon.
bacon:
    if command -v bacon >/dev/null 2>&1; then \
      bacon; \
    else \
      echo "bacon not installed"; \
      exit 1; \
    fi

# Coverage report (terminal summary + html in target/llvm-cov/html).
cov:
    if command -v cargo-llvm-cov >/dev/null 2>&1; then \
      cargo llvm-cov --all-features --workspace --html; \
    else \
      echo "cargo-llvm-cov not installed"; \
      exit 1; \
    fi

# Coverage report for CI systems.
cov-ci:
    if command -v cargo-llvm-cov >/dev/null 2>&1; then \
      cargo llvm-cov --all-features --workspace --lcov --output-path target/llvm-cov/lcov.info; \
    else \
      echo "cargo-llvm-cov not installed"; \
      exit 1; \
    fi

# Security audit.
audit:
    if command -v cargo-audit >/dev/null 2>&1; then \
      mkdir -p .cargo-home; \
      CARGO_HOME="${CARGO_HOME:-$PWD/.cargo-home}" cargo audit; \
    else \
      echo "cargo-audit not installed"; \
      exit 1; \
    fi

# License + advisory + policy checks.
deny:
    if command -v cargo-deny >/dev/null 2>&1; then \
      mkdir -p .cargo-home; \
      CARGO_HOME="${CARGO_HOME:-$PWD/.cargo-home}" cargo deny check; \
    else \
      echo "cargo-deny not installed"; \
      exit 1; \
    fi

# Unused dependency checks.
deps-udeps:
    if command -v cargo-udeps >/dev/null 2>&1; then \
      if rustup toolchain list | rg -q '^nightly'; then \
        cargo +nightly udeps --workspace --all-targets; \
      else \
        echo "nightly toolchain missing; run: rustup toolchain install nightly"; \
        exit 1; \
      fi; \
    else \
      echo "cargo-udeps not installed"; \
      exit 1; \
    fi

deps-machete:
    if command -v cargo-machete >/dev/null 2>&1; then \
      cargo machete; \
    else \
      echo "cargo-machete not installed"; \
      exit 1; \
    fi

# Outdated dependencies (if cargo-outdated is installed).
deps-outdated:
    if command -v cargo-outdated >/dev/null 2>&1; then \
      cargo outdated -R; \
    else \
      echo "cargo-outdated not installed (skipping)"; \
    fi

# Full dependency hygiene sweep.
deps-weekly:
    just audit
    just deny
    just deps-machete
    just deps-outdated
    just deps-udeps

# Binary size report.
size:
    if command -v cargo-bloat >/dev/null 2>&1; then \
      cargo bloat --release --crates --filter cardthropic -n 30; \
    else \
      echo "cargo-bloat not installed"; \
      exit 1; \
    fi

# Verify minimum supported Rust version.
msrv:
    if command -v cargo-msrv >/dev/null 2>&1; then \
      cargo msrv verify; \
    else \
      echo "cargo-msrv not installed"; \
      exit 1; \
    fi

# Benchmark two hot test targets.
bench-tests:
    hyperfine --warmup 1 \
      'cargo test -q freecell_' \
      'cargo test -q winnability'

# Daily driver: check + focused tests + benchmark report.
daily:
    mkdir -p reports
    report="reports/daily-{{timestamp}}.txt"; \
    { \
      echo "Cardthropic Daily Report"; \
      echo "timestamp={{timestamp}}"; \
      echo; \
      echo "== cargo check -q =="; \
      cargo check -q; \
      echo "ok"; \
      echo; \
      echo "== cargo test -q freecell_ =="; \
      cargo test -q freecell_; \
      echo; \
      echo "== hyperfine (freecell vs winnability) =="; \
      hyperfine --warmup 1 \
        'cargo test -q freecell_' \
        'cargo test -q winnability'; \
    } | tee "$report"; \
    echo; \
    echo "Saved report: $report"

# Lightweight perf counters on focused FreeCell tests.
perf-freecell:
    perf stat -d cargo test -q freecell_

# Record and report perf profile for focused FreeCell tests.
perf-record:
    perf record -g -- cargo test -q freecell_
    perf report

# Build using mold linker for faster links (opt-in).
build-mold:
    RUSTFLAGS='-C link-arg=-fuse-ld=mold' cargo build

# Debug test binary via gdb.
gdb-test test_name='freecell_':
    gdb --args cargo test {{test_name}} -- --nocapture

# Debug test binary via lldb.
lldb-test test_name='freecell_':
    lldb -- cargo test {{test_name}} -- --nocapture

# Expand macro output for a package/item.
expand item:
    cargo expand --bin cardthropic {{item}}

# Fast codebase search helpers.
find-files pattern='':
    fd {{pattern}}

find-text pattern:
    rg -n {{pattern}} src README.md

# Read a file with syntax highlighting + line numbers.
view path:
    bat --style=numbers --paging=never {{path}}

# High-level project size summary.
stats:
    tokei src

# Fast local gate (pre-push).
ci-local:
    just fmt-check
    just check
    just clippy-warn
    just test-nextest
    just audit
    just deny

# Deeper periodic gate.
ci-nightly:
    just ci-local
    just deps-weekly
    just ci-artifacts

# Collect machine-readable nightly artifacts under reports/ci and bundle them.
ci-artifacts:
    mkdir -p reports/ci
    just cov-ci
    cp -f target/llvm-cov/lcov.info reports/ci/lcov.info
    just size | tee reports/ci/size.txt
    just deps-machete | tee reports/ci/deps-machete.txt
    if command -v cargo-udeps >/dev/null 2>&1; then \
      just deps-udeps | tee reports/ci/deps-udeps.txt; \
    else \
      echo "cargo-udeps not installed" | tee reports/ci/deps-udeps.txt; \
    fi
    just audit | tee reports/ci/audit.txt
    just deny | tee reports/ci/deny.txt
    tar -czf reports/ci-bundle.tgz -C reports ci
    echo "Nightly artifacts: reports/ci-bundle.tgz"

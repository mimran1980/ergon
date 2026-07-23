# ergon — reproducible workspace gates
#
# ── ergo-aeron-cluster + --all-features ─────────────────────────────────
# `cluster` IS a workspace member. Commands that pass `--all-features` still
# use `--exclude ergo-aeron-cluster` and then re-run that crate alone because:
#
#   • cluster's optional feature `test-harness` enables the in-crate
#     `test_support` module (Java ClusterLauncher / Aeron jars via rusteron-archive).
#   • `--all-features` turns that feature on, so a single
#     `cargo {build,test,clippy} --workspace --all-features` would pull the
#     harness into the default Rust-only gate.
#   • Default `cluster` features (`default = []`) are pure Rust and safe for CI.
#
# Pattern:
#   cargo … --workspace --all-features --exclude ergo-aeron-cluster
#   cargo … -p ergo-aeron-cluster            # default features only
#
# Full harness: `just build-aeron-jars` then `just test-aeron-cluster-harness`.
# Samples are workspace-excluded packages (standalone).
#
# ── Release (crates.io) ─────────────────────────────────────────────────
# Publish product crates individually; do NOT `--all-features` for release.
#   1. ergo-sbe             (sbe/)
#   2. ergo-aeron-cluster   with default features only (never require test-harness)
# Do not publish: ergo-sbe-benchmarks (publish=false), samples.
# Consumers depend on crates.io versions; monorepo samples keep `path = …`.
# Tag the repo after publish; Aeron submodule pin is independent of crate release.

import 'just/samples.just'
import 'just/aeron.just'
import 'just/housekeeping.just'

# Default: list available commands.
default:
    @just --list

# ── build ─────────────────────────────────────────────────────────────────

# Compile product workspace + sample harnesses (no tests, no Java jars).
build:
    cargo build --workspace --all-features --exclude ergo-aeron-cluster
    cargo build -p ergo-aeron-cluster
    cd samples/exchange-example && cargo build
    cd samples/cluster-ha-orderbook && cargo build
    cd samples/cluster-rfq && cargo build

# ── check ─────────────────────────────────────────────────────────────────

# Full local check: hygiene, format, clippy, tests (no external services / no Java).
check:
    ./scripts/check-repository-hygiene.sh
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib
    cd samples/exchange-example && cargo fmt --check
    cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo fmt --check
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    cd samples/cluster-rfq && cargo fmt --check
    cd samples/cluster-rfq && cargo clippy --all-targets -- -D warnings

# Product-only gate: fmt, clippy, and tests for the two publishable prototype crates only.
check-products:
    cargo fmt --all --check
    cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    cargo test -p ergo-sbe --all-features -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib

# Sample crates gate (unpublished).
check-samples:
    cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    cd samples/cluster-rfq && cargo clippy --all-targets -- -D warnings

# Pre-release check: products + bench compile + package verification.
release-check: check-products
    cargo bench -p ergo-sbe-benchmarks --no-run
    cargo bench -p ergo-aeron-cluster --no-run
    cargo publish -p ergo-sbe --dry-run --allow-dirty
    @echo "release-check: product crates pass, benches compile, dry-run publish OK"

# ── test ──────────────────────────────────────────────────────────────────

# Comprehensive test suite: runs everything possible (unit, integration,
# cluster lib, sample offline tests, and bench compilation).
# Gated tests (Java harness) run only when their services are available.
test:
    @echo "=== 1/5 fmt ==="
    cargo fmt --all --check
    @echo "=== 2/5 clippy (workspace + samples) ==="
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    @echo "=== 3/5 unit + integration tests ==="
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib
    @echo "=== 4/5 sample offline tests ==="
    cd samples/exchange-example && cargo test --lib -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    @echo "=== 5/5 bench compilation ==="
    cargo bench -p ergo-sbe-benchmarks --no-run
    @echo ""
    @echo "=== Gated: Aeron Cluster Java harness ==="
    @if cargo test -p ergo-aeron-cluster --features test-harness --no-run 2>/dev/null; then \
        echo "test-harness compiles — running cluster integration tests"; \
        cargo test -p ergo-aeron-cluster --features test-harness -- --test-threads=1; \
    else \
        echo "Java harness not available — skipping (build jars with: just build-aeron-jars)"; \
    fi
    @echo ""
    @echo "=== test: complete ==="

# Workspace unit tests only.
test-unit:
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib

# ── formatting ─────────────────────────────────────────────────────────────

# Format all handwritten source.
fmt:
    cargo fmt --all
    cd samples/exchange-example && cargo fmt
    cd samples/cluster-ha-orderbook && cargo fmt
    cd samples/cluster-rfq && cargo fmt

# Auto-fix: fmt + clippy --fix (same feature split as check).
fix:
    just fmt
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster --fix --allow-dirty --allow-staged -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/exchange-example && cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/cluster-rfq && cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings

# ── benchmarks ─────────────────────────────────────────────────────────────

# Benchmark parity — both checked (default) and bound-check-disabled modes.
# Gate: ALL maintained ergo-sbe/Aeron ratios ≤ 1.00 in both modes,
# AND unchecked ergo-sbe must not regress vs checked ergo-sbe.
bench:
    @echo "=== SBE perf parity: checked mode (save baseline) ==="
    cd sbe/benchmarks && cargo bench --bench perf_parity_bench -- --save-baseline checked
    @echo ""
    @echo "=== SBE perf parity: unchecked (compare vs checked baseline) ==="
    cd sbe/benchmarks && cargo bench --bench perf_parity_bench --features bound-check-disabled -- --baseline checked
    @echo ""
    @echo "=== Gate ==="
    ./scripts/check-bench-gate.sh target/criterion

# Cluster codec benchmarks (ergo-sbe vs sbe-tool head-to-head).
bench-cluster:
    cargo bench -p ergo-aeron-cluster
    @echo ""
    @echo "=== Gate ==="
    ./scripts/check-bench-gate.sh target/criterion

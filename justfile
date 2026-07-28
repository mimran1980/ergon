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
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1 --skip explicit_implicit
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
    ./scripts/regenerate-sbe-tool-reference.sh --check
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test --manifest-path sbe/miri-fixtures/Cargo.toml
    cargo test -p ergo-sbe --all-features -- --test-threads=1
    cargo test -p ergo-sbe --doc --all-features -- --test-threads=1
    cargo test -p ergo-sbe --test docs_validation_test --all-features -- --test-threads=1
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    cargo test -p ergo-aeron-cluster --lib

# Sample crates gate (unpublished).
check-samples:
    cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    cd samples/cluster-rfq && cargo clippy --all-targets -- -D warnings

# Pre-release check: product crates + bench compile + package + strict rustdoc.
release-check: check-products
    cargo bench -p ergo-sbe-benchmarks --no-run
    cargo bench -p ergo-aeron-cluster --no-run
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    cargo publish -p ergo-sbe --dry-run --allow-dirty
    cargo publish -p ergo-aeron-cluster --dry-run --allow-dirty
    @echo "release-check: product crates pass, benches compile, dry-run publish OK"

# ── test ──────────────────────────────────────────────────────────────────

# Comprehensive test suite: runs everything possible (unit, integration,
# doctests/rustdoc for ergo-sbe, cluster lib, sample offline tests, benches).
# Gated tests (Java harness) run only when their services are available.
test:
    @echo "=== 1/6 fmt ==="
    cargo fmt --all --check
    @echo "=== 2/6 clippy (workspace + samples) ==="
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    @echo "=== 3/6 unit + integration tests ==="
    ./scripts/regenerate-sbe-tool-reference.sh --check
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test --manifest-path sbe/miri-fixtures/Cargo.toml
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1 --skip explicit_implicit
    cargo test -p ergo-aeron-cluster --lib
    @echo "=== 4/6 ergo-sbe doctests + rustdoc (-D warnings) + docs_validation ==="
    cargo test -p ergo-sbe --doc --all-features -- --test-threads=1
    cargo test -p ergo-sbe --test docs_validation_test --all-features -- --test-threads=1
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    @echo "=== 5/6 sample offline tests ==="
    cd samples/l3-book && cargo test -- --test-threads=1
    cd samples/sbe-feature-tour && cargo test -- --test-threads=1
    cd samples/sbe-codegen-examples && cargo run --example flyweight >/dev/null && cargo run --example domain_objects >/dev/null
    cd samples/exchange-example && cargo test --lib -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    cd samples/cluster-rfq && cargo build --examples
    @echo "=== 6/6 bench compilation ==="
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
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1 --skip explicit_implicit
    cargo test -p ergo-aeron-cluster --lib

# Every test gate including nightly-only miri and fuzz.
# Runs: standard suite + Miri UB detection + fuzz corpus replay.
test-all: test
    @echo "=== 7/7 miri (UB detection) ==="
    cargo +nightly miri test --manifest-path sbe/miri-fixtures/Cargo.toml
    @echo "=== 8/7 fuzz corpus replay ==="
    cd sbe/fuzz && cargo +nightly fuzz run generated_verify -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run nested_group_decode -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run bulk_decode -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run flat_group_decode -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run any_message_frame_cursor -- -max_total_time=30
    cd sbe/fuzz && cargo +nightly fuzz run schema_parse -- -max_total_time=30
    @echo ""
    @echo "=== test-all: complete ==="

# Rebuild every pinned official sbe-tool reference in a temporary directory.
check-sbe-references:
    ./scripts/regenerate-sbe-tool-reference.sh --check

# Compile all libFuzzer targets and run the deterministic corpus replay.
check-fuzz:
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test -p ergo-sbe --test hostile_input_replay_test -- --test-threads=1

# Run and compare line/function/region coverage with the checked-in ratchet.
check-coverage:
    ./scripts/check-coverage-ratchet.sh

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

# Benchmark parity — ergo-sbe vs sbe-tool head-to-head.
# Gate: ALL maintained ergo-sbe/sbe-tool ratios ≤ 1.00.
# Uses wrap_unchecked for fair comparison (sbe-tool's wrap does not validate).
bench:
    @echo "=== SBE perf parity ==="
    cd sbe/benchmarks && cargo bench --bench perf_parity_bench
    @echo ""
    @echo "=== Gate ==="
    ./scripts/check-bench-gate.sh target/criterion

# Expanded non-gating codec matrix and offset/alignment diagnostics.
bench-diagnostics:
    cargo bench -p ergo-sbe-benchmarks --bench codec_matrix_bench
    cargo bench -p ergo-sbe-benchmarks --bench alignment_bench
    cargo bench -p ergo-sbe-benchmarks --bench cold_path_bench
    cargo bench -p ergo-sbe-benchmarks --bench latency_distribution

# Fresh generated-crate compile/source/binary-size report.
bench-cold:
    ./scripts/measure-codegen-cold-path.sh

# Linux/Valgrind instruction counts (requires iai-callgrind-runner).
bench-instructions:
    cargo bench -p ergo-sbe-benchmarks --bench instruction_counts

# Cluster codec benchmarks (ergo-sbe vs sbe-tool head-to-head).
bench-cluster:
    cargo bench -p ergo-aeron-cluster
    @echo ""
    @echo "=== Gate ==="
    ./scripts/check-bench-gate.sh target/criterion

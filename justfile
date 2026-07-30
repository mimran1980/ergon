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

# Enforce test ownership and prove the policy checker catches every bypass.
policy:
    bash scripts/tests/test-test-policy.sh
    bash scripts/tests/test-quality-ratchets.sh
    ./scripts/check-test-policy.sh
    ./scripts/check-mutation-config.sh

# Full local check: hygiene, format, clippy, tests (no external services / no Java).
check: policy
    ./scripts/check-repository-hygiene.sh
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib
    cargo test -p ergo-aeron-cluster --doc
    cd samples/exchange-example && cargo fmt --check
    cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo fmt --check
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    cd samples/cluster-rfq && cargo fmt --check
    cd samples/cluster-rfq && cargo clippy --all-targets -- -D warnings

# Product-only gate: fmt, clippy, and tests for the two publishable prototype crates only.
check-products: policy
    ./scripts/regenerate-golden.sh --check
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
    cargo test -p ergo-aeron-cluster --doc
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-aeron-cluster --no-deps

# Sample crates gate (unpublished).
check-samples: policy
    cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    cd samples/cluster-rfq && cargo clippy --all-targets -- -D warnings

# Pre-release check: product crates + bench compile + package + strict rustdoc.
release-check: test check-products check-coverage
    cargo bench -p ergo-sbe-benchmarks --no-run
    cargo bench -p ergo-aeron-cluster --no-run
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-aeron-cluster --no-deps
    cargo publish -p ergo-sbe --dry-run --allow-dirty
    cargo publish -p ergo-aeron-cluster --dry-run --allow-dirty
    @echo "release-check: product crates pass, benches compile, dry-run publish OK"

# Full release gate: test + bench → publish → tag → GitHub release → bump.
# The LLM must bump the version + write changelog + write release notes before
# calling this. The version is read from workspace Cargo.toml.
release:
    @echo "=== Gate: test suite ==="
    just test
    @echo "=== Gate: cluster benchmarks ==="
    just bench-cluster
    @echo "=== Gate: SBE benchmarks ==="
    just bench
    @echo "=== publish ergo-sbe ==="
    cargo publish -p ergo-sbe
    @echo "=== publish ergo-aeron-cluster ==="
    cargo publish -p ergo-aeron-cluster
    @echo "=== tag ==="
    git tag v$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    git push origin v$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
    @echo "=== GitHub release ==="
    gh release create v$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/') --title "ergon v$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')" --notes-file /tmp/ergon-release-notes.md
    @echo "=== release v$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/') complete ==="

# ── test ──────────────────────────────────────────────────────────────────

# Comprehensive test suite: unit, integration, doctests/rustdoc, Java cluster
# lifecycle tests, sample tests, and benchmark compilation. Missing Java,
# Gradle, jars, or another required dependency is a failure.
test: policy
    @echo "=== 1/7 fmt ==="
    cargo fmt --all --check
    @echo "=== 2/7 clippy (workspace + samples) ==="
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    @echo "=== 3/7 unit + integration tests ==="
    ./scripts/regenerate-golden.sh --check
    ./scripts/regenerate-sbe-tool-reference.sh --check
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test --manifest-path sbe/miri-fixtures/Cargo.toml
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib
    @echo "=== 4/7 product doctests + rustdoc (-D warnings) + docs_validation ==="
    cargo test -p ergo-sbe --doc --all-features -- --test-threads=1
    cargo test -p ergo-sbe --test docs_validation_test --all-features -- --test-threads=1
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-sbe --all-features --no-deps
    cargo test -p ergo-aeron-cluster --doc
    RUSTDOCFLAGS='-D warnings' cargo doc -p ergo-aeron-cluster --no-deps
    @echo "=== 5/7 sample tests ==="
    cd samples/l3-book && cargo test -- --test-threads=1
    cd samples/sbe-feature-tour && cargo test -- --test-threads=1
    cd samples/sbe-codegen-examples && cargo run --example flyweight >/dev/null && cargo run --example domain_objects >/dev/null
    cd samples/exchange-example && cargo test -- --test-threads=1
    cd samples/cluster-rfq && cargo build --examples
    @echo "=== 6/7 Aeron Cluster Java lifecycle + HA sample ==="
    just build-aeron-jars
    cargo test -p ergo-aeron-cluster --features test-harness -- --test-threads=1
    cd samples/cluster-ha-orderbook && cargo test --features test-harness -- --test-threads=1
    @echo "=== 7/7 benchmark compilation ==="
    cargo bench -p ergo-sbe-benchmarks --no-run
    cargo bench -p ergo-aeron-cluster --no-run
    cd samples/l3-book && cargo bench --no-run
    @echo ""
    @echo "=== test: complete ==="

# Workspace unit tests only.
test-unit: policy
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib

# Every test gate including nightly-only miri and fuzz.
# Runs: standard suite + Miri UB detection + fuzz corpus replay.
#
# ── AI ASSISTANTS: READ THIS BEFORE TOUCHING ANYTHING ─────────────────────────
# DO NOT add test-selection filters, ignored attributes, or another silencing mechanism.
# DO NOT comment out failing tests or benches.
# DO NOT add conditions that silently skip tests ("if java then … else …").
# If a test fails, the test is correct and the code is wrong — FIX THE CODE.
# If the environment doesn't support a test, ask the user — NEVER skip.
# This file is the single source of truth for "did everything pass?"
# A green `just test-all` means EVERY test ran and EVERY test passed.
# ───────────────────────────────────────────────────────────────────────────────
test-all: policy test
    @echo "=== 8/9 miri (UB detection) ==="
    cargo +nightly miri test --manifest-path sbe/miri-fixtures/Cargo.toml
    @echo "=== 9/9 fuzz corpus replay ==="
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

# Regenerate the checked-in generated-code golden through a non-test command.
update-golden:
    ./scripts/regenerate-golden.sh

# Rebuild the generated-code golden in a temporary file and compare bytes.
check-golden:
    ./scripts/regenerate-golden.sh --check

# Compile all libFuzzer targets and run the deterministic corpus replay.
check-fuzz:
    cargo check --manifest-path sbe/fuzz/Cargo.toml --bins
    cargo test -p ergo-sbe --test hostile_input_replay_test -- --test-threads=1

# Run and compare line/function/region coverage with the checked-in ratchet.
check-coverage:
    ./scripts/check-coverage-ratchet.sh

# Mutate parser/resolver/codegen critical paths and reject missed or timed-out
# mutants. This is a MANUAL gate — too slow for CI (~16 h with --jobs 1).
# Run locally before landing codegen changes; use --jobs 1 to avoid
# exhausting disk space with parallel build trees.
check-mutation:
    ./scripts/check-mutation-config.sh
    cargo mutants --jobs 1
    ./scripts/check-mutation-ratchet.sh

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
# Gate: every maintained ergon/sbe-tool ratio must stay at or below 1.00.
# Uses trusted direct wraps for fair comparison (sbe-tool's wrap does not validate).
bench:
    @echo "=== SBE perf parity — LTO ==="
    cd sbe/benchmarks && cargo bench --bench perf_parity_bench
    @echo ""
    @echo "=== Gate — LTO ==="
    ./scripts/check-bench-gate.sh target/criterion 0.005 sbe
    @echo ""
    @echo "=== SBE perf parity — no LTO ==="
    CARGO_TARGET_DIR=target/bench-no-lto CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 cargo bench -p ergo-sbe-benchmarks --bench perf_parity_bench
    @echo ""
    @echo "=== Gate — no LTO ==="
    ./scripts/check-bench-gate.sh sbe/benchmarks/target/bench-no-lto/criterion 0.005 sbe

# Group-codegen comparison under both optimization profiles. sbe-tool is
# intentionally measured in both: the audit found it stable without LTO while
# pre-fix ergon regressed because generated entry setters did not inline.
bench-groups:
    cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench
    cargo bench -p ergo-sbe-benchmarks --bench group_encode_decimal_bench
    CARGO_TARGET_DIR=target/bench-no-lto CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 cargo bench -p ergo-sbe-benchmarks --bench group_encode_bench
    CARGO_TARGET_DIR=target/bench-no-lto CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 cargo bench -p ergo-sbe-benchmarks --bench group_encode_decimal_bench

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
    ./scripts/check-bench-gate.sh target/criterion 0.005 cluster

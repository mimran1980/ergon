# ErgoSBE — build, test, lint, and samples

set shell := ["bash", "-cu"]

default:
    @just --list --unsorted

# ── Build ──────────────────────────────────────────────────────

# Build all workspace crates (sbe, persist, persist/derive)
build:
    RUSTC_WRAPPER="" cargo build --workspace

# Build all crates including samples
build-all: build
    RUSTC_WRAPPER="" cargo build --manifest-path samples/exchange-orderbook/Cargo.toml

# Build all projects + run all unit and integration tests
build-test: build-all
    RUSTC_WRAPPER="" cargo test --workspace -- --test-threads=1
    RUSTC_WRAPPER="" cargo test --manifest-path samples/exchange-orderbook/Cargo.toml

# ── Test ───────────────────────────────────────────────────────

# Run workspace unit tests
test:
    RUSTC_WRAPPER="" cargo test --workspace -- --test-threads=1

# Run all tests including samples
test-all: test
    RUSTC_WRAPPER="" cargo test --manifest-path samples/exchange-orderbook/Cargo.toml

# ── Format / Lint ──────────────────────────────────────────────

fmt:
    cargo fmt --all --check

fmt-fix:
    cargo fmt --all

clippy:
    RUSTC_WRAPPER="" cargo clippy --workspace --all-targets -- -D warnings

clippy-all: clippy
    RUSTC_WRAPPER="" cargo clippy --manifest-path samples/exchange-orderbook/Cargo.toml --all-targets -- -D warnings

fix:
    cargo fmt --all
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# ── CI ─────────────────────────────────────────────────────────

ci: fmt clippy test
    @echo "CI: all checks passed"

ci-all: fmt clippy-all test-all
    @echo "CI-all: all checks passed"

# ── Golden file ────────────────────────────────────────────────

update-golden:
    RUSTC_WRAPPER="" cargo test -p ergosbe --test stability_test -- update_golden --ignored

# ── Samples ────────────────────────────────────────────────────

samples-orderbook:
    @echo "=== Starting ClickHouse ==="
    @docker start ergo-clickhouse 2>/dev/null || docker run -d --name ergo-clickhouse -p 8123:8123 -p 9000:9000 clickhouse/clickhouse-server
    @echo "=== Waiting for ClickHouse ==="
    @until curl -s http://localhost:8123/ping >/dev/null 2>&1; do sleep 1; done
    @echo "=== Running exchange orderbook ==="
    CLICKHOUSE_URL=http://localhost:8123 RUSTC_WRAPPER="" cargo run --manifest-path samples/exchange-orderbook/Cargo.toml

samples-clickhouse-stop:
    docker stop ergo-clickhouse 2>/dev/null || true
    docker rm ergo-clickhouse 2>/dev/null || true

# ── Docs ───────────────────────────────────────────────────────

docs:
    cargo doc --no-deps --workspace --open

# ── Benchmarks ──────────────────────────────────────────────────

# Run all Criterion benchmarks
bench:
    RUSTC_WRAPPER="" cargo bench --workspace

# Run just the Aeron parity head-to-head
bench-parity:
    RUSTC_WRAPPER="" cargo bench --bench perf_parity_bench

# Quick benchmark smoke test (compile check only, no long runs)
bench-check:
    RUSTC_WRAPPER="" cargo bench --workspace --no-run

# Benchmark with bound-check-disabled feature enabled
bench-fast:
    RUSTC_WRAPPER="" cargo bench --workspace --features bound-check-disabled

# ── Clean ───────────────────────────────────────────────────────

clean:
    cargo clean

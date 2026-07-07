# ErgoSBE — build, test, lint, and samples

set shell := ["bash", "-cu"]

default:
    @just --list --unsorted

# ── Build ──────────────────────────────────────────────────────

build:
    RUSTC_WRAPPER="" cargo build --workspace

build-all: build
    RUSTC_WRAPPER="" cargo build --manifest-path samples/exchange-orderbook/Cargo.toml

# ── Test ───────────────────────────────────────────────────────

test:
    RUSTC_WRAPPER="" cargo test --workspace

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

# ── Clean ───────────────────────────────────────────────────────

clean:
    cargo clean

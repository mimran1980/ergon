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
# Suggested order (path deps first):
#   1. ergo-sbe             (sbe/)
#   2. ergo-clickhouse-persist-derive  then  ergo-clickhouse-persist
#   3. ergo-aeron-cluster   with default features only (never require test-harness)
# Do not publish: ergo-sbe-benchmarks (publish=false), samples.
# Consumers depend on crates.io versions; monorepo samples keep `path = …`.
# Tag the repo after publish; Aeron submodule pin is independent of crate release.

# Default: list available commands.
default:
    @just --list

# Wipe Cargo build artifacts and reset git submodules to the commits pinned by
# this repo (fetch origin, hard-reset + clean dirt, force-checkout).
clean:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== cargo clean (workspace) ==="
    cargo clean
    for d in samples/exchange-example samples/cluster-ha-orderbook; do
      if [ -f "$d/Cargo.toml" ]; then
        echo "=== cargo clean ($d) ==="
        (cd "$d" && cargo clean)
      fi
    done
    echo "=== git submodules → origin / pinned commits ==="
    git submodule sync --recursive
    git submodule foreach --recursive '
      set -e
      git fetch origin --tags 2>/dev/null || git fetch origin || true
      git reset --hard
      git clean -fdx
    '
    git submodule update --init --recursive --force
    echo "clean: done (targets removed; submodules hard-reset to parent pins)"

# Compile product workspace + sample harnesses (no tests, no Java jars).
# See header: --all-features excludes cluster so test-harness is not enabled;
# cluster is built next at default features only.
build:
    cargo build --workspace --all-features --exclude ergo-aeron-cluster
    cargo build -p ergo-aeron-cluster
    cd samples/exchange-example && cargo build
    cd samples/cluster-ha-orderbook && cargo build

# Full local check: hygiene, format, clippy, tests (no external services / no Java).
# Cluster: --all-features workspace pass excludes it; second pass is default-features
# --lib only. Harness: just test-aeron-cluster-harness (after just build-aeron-jars).
check:
    ./scripts/check-repository-hygiene.sh
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib
    cd samples/exchange-example && cargo fmt --check
    cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/exchange-example && cargo test -- --test-threads=1 --skip clickhouse
    cd samples/cluster-ha-orderbook && cargo fmt --check
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1


# Product-only gate: fmt, clippy, and tests for the two publishable prototype crates only.
check-products:
	cargo fmt --all --check
	cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
	cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
	cargo test -p ergo-sbe --all-features -- --test-threads=1
	cargo test -p ergo-aeron-cluster --lib

# Lab-only gate: persist and sample crates (unpublished).
check-labs:
	cargo clippy -p ergo-clickhouse-persist --all-targets --all-features -- -D warnings
	cargo test -p ergo-clickhouse-persist --all-targets --all-features -- --test-threads=1
	cd samples/exchange-example && cargo clippy --all-targets --all-features -- -D warnings
	cd samples/exchange-example && cargo test -- --test-threads=1 --skip clickhouse
	cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
	cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1

# Pre-release check: products + bench compile + package verification.
release-check: check-products
	cargo bench -p ergo-sbe-benchmarks --no-run
	cargo bench -p ergo-aeron-cluster --no-run
	cargo publish -p ergo-sbe --dry-run --allow-dirty
	@echo "release-check: product crates pass, benches compile, dry-run publish OK"

# Workspace unit tests only
# Comprehensive test suite: runs everything possible (unit, integration, IPC,
# persistence, cluster lib, sample offline tests, and bench compilation).
# Gated tests (ClickHouse live, Java harness) run only when their services are
# available — they are skipped if the preflight fails rather than erroring.
test:
	@echo "=== 1/6 fmt ==="
	cargo fmt --all --check
	@echo "=== 2/6 clippy (workspace products + labs) ==="
	cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster -- -D warnings
	cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
	@echo "=== 3/6 unit + integration tests (no external services) ==="
	cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
	cargo test -p ergo-aeron-cluster --lib
	@echo "=== 4/6 allocation proofs ==="
	cargo test -p ergo-sbe --test allocation_count_test -- --test-threads=1
	@echo "=== 5/6 sample offline tests ==="
	cd samples/exchange-example && cargo test --lib -- --test-threads=1 --skip clickhouse
	cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
	@echo "=== 6/6 bench compilation ==="
	cargo bench -p ergo-sbe-benchmarks --no-run
	@echo ""
	@echo "=== Gated: ClickHouse live tests ==="
	@if curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; then \
	    echo "ClickHouse available — running live tests"; \
	    cargo test -p ergo-clickhouse-persist --all-features -- --test-threads=1 --include-ignored; \
	    (cd samples/exchange-example && cargo test --test clickhouse_e2e_test --test e2e_persist_test -- --include-ignored --test-threads=1 --nocapture); \
	    (cd samples/cluster-ha-orderbook && cargo test --test ha_latency_clickhouse -- --include-ignored --test-threads=1 --nocapture); \
	else \
	    echo "ClickHouse not available — skipping live tests (start with: bash persist/tests/run-clickhouse.sh start)"; \
	fi
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

# Workspace unit tests only (same --all-features / cluster exclude pattern as check).
test-unit:
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib

# Sample IPC tests (embedded Aeron driver — no external services)
test-ipc:
    cd samples/exchange-example && cargo test -- --test-threads=1 --skip clickhouse

# Live ClickHouse tests for the IPC sample (requires Docker CH on 127.0.0.1:8123)
test-clickhouse-live:
    @echo "Preflight: checking ClickHouse on 127.0.0.1:8123..."
    @if ! curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; then \
        echo "ClickHouse not available. Start it:"; \
        echo "  bash persist/tests/run-clickhouse.sh start"; \
        exit 1; \
    fi
    @echo "Preflight OK — endpoint http://127.0.0.1:8123 (external Docker)."
    (cd samples/exchange-example && cargo test --test clickhouse_e2e_test --test e2e_persist_test -- --include-ignored --test-threads=1 --nocapture

# Alias: IPC sample live CH E2E (typed + Persist DTO snapshot)
samples-orderbook: test-clickhouse-live

# HA cluster sample: offline try_claim + never-stale book proofs, then live
# feed_latency ClickHouse rows (requires Docker CH on 127.0.0.1:8123).
samples-cluster-ha:
    @echo "=== cluster-ha-orderbook offline pipeline (try_claim path + stale-book) ==="
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1
    @echo "Preflight: ClickHouse on 127.0.0.1:8123..."
    @if ! curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; then \
        echo "ClickHouse not available — starting via persist/tests/run-clickhouse.sh"; \
        bash persist/tests/run-clickhouse.sh start || true; \
        sleep 2; \
    fi
    @if curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; then \
        echo "=== feed_latency DynamicRow → ClickHouse ==="; \
        (cd samples/cluster-ha-orderbook && cargo test --test ha_latency_clickhouse -- --include-ignored --test-threads=1 --nocapture); \
    else \
        echo "WARNING: ClickHouse still unreachable; offline HA tests already ran."; \
        exit 1; \
    fi

# Multi-node Java kill-leader never-stale book (needs Aeron jars + Java).
samples-cluster-ha-kill-leader:
    cd samples/cluster-ha-orderbook && cargo test --features test-harness --test ha_kill_leader -- --test-threads=1 --nocapture

# Build rustdoc for the three main product crates (no deps)
docs:
    cargo doc -p ergo-sbe --no-deps
    cargo doc -p ergo-clickhouse-persist --no-deps
    cargo doc -p ergo-aeron-cluster --no-deps

# Format all handwritten source
fmt:
    cargo fmt --all
    cd samples/exchange-example && cargo fmt
    cd samples/cluster-ha-orderbook && cargo fmt

# Auto-fix: fmt + clippy --fix (same feature split as check).
fix:
    just fmt
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster --fix --allow-dirty --allow-staged -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/exchange-example && cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets --fix --allow-dirty --allow-staged -- -D warnings

# Coverage (requires nightly toolchain)
cov:
    RUSTC_WRAPPER="" cargo +nightly llvm-cov -p ergo-sbe --lib --branch --summary-only

# Start ClickHouse Docker and run the advanced sample
run-sample:
    @echo "Starting ClickHouse..."
    @docker start ergo-clickhouse 2>/dev/null || \
        docker run -d --name ergo-clickhouse \
            -p 8123:8123 -p 9000:9000 \
            -e CLICKHOUSE_USER=default -e CLICKHOUSE_PASSWORD=ergon \
            clickhouse/clickhouse-server:latest
    @echo "Waiting for ClickHouse..."
    @until curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; do sleep 2; done
    @echo "ClickHouse ready. Running sample..."
    cd samples/exchange-example && cargo run --quiet

# Benchmark parity — both checked (default) and bound-check-disabled modes.
# Uses Criterion's baseline feature for statistically valid cross-mode comparison.
# Gate: ALL maintained ergo-sbe/Aeron ratios ≤ 1.00 in both modes,
# AND unchecked ergo-sbe must not regress vs checked ergo-sbe.
# The check-bench-gate.sh script enforces this (exit non-zero on failure).
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
# Gate enforced by check-bench-gate.sh after the run.
bench-cluster:
    cargo bench -p ergo-aeron-cluster
    @echo ""
    @echo "=== Gate ==="
    ./scripts/check-bench-gate.sh target/criterion

# =============================================================================
# cluster/ = crate ergo-aeron-cluster (client + optional in-crate test_support)
# =============================================================================

# Check the cluster crate (lib: fmt + clippy + tests, no Java required)
check-aeron-cluster:
    cd cluster && cargo fmt --check
    cd cluster && cargo clippy --all-targets -- -D warnings
    cd cluster && cargo test --lib

# Cluster integration tests (requires Java 17+ and built Aeron jars — run
# `just build-aeron-jars` once first; slow).
test-aeron-cluster-harness:
    cd cluster && cargo test --features test-harness -- --test-threads=1

# Build Aeron cluster test jars (requires Java 17+; run once before test-harness tests)
build-aeron-jars:
    cd aeron && ./gradlew :aeron-cluster:jar :aeron-archive:jar :aeron-all:jar

hash-aeron-jars:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v sha256sum >/dev/null 2>&1; then SHA256=(sha256sum); elif command -v shasum >/dev/null 2>&1; then SHA256=(shasum -a 256); else echo "ERROR: neither sha256sum nor shasum available" >&2; exit 1; fi
    echo "# SHA-256 hashes of test jars" > cluster/test-jars.sha256
    for dir in aeron-all aeron-cluster aeron-archive; do
      jar=$(find aeron/$dir/build/libs -name '*.jar' ! -name '*sources*' ! -name '*javadoc*' -print -quit)
      if [ -n "$jar" ]; then
        "${SHA256[@]}" "$jar" | tee -a cluster/test-jars.sha256
      fi
    done
    echo "=== SHA-256 hashes saved to cluster/test-jars.sha256 ==="

check-aeron-jars:
    #!/usr/bin/env bash
    set -euo pipefail
    just hash-aeron-jars
    if ! git diff --exit-code cluster/test-jars.sha256; then
      echo "ERROR: Jar SHA-256 mismatch. Run 'just hash-aeron-jars' and commit." >&2
      exit 1
    fi
    echo "OK: Jar hashes match committed."

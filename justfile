# ErgoSBE — reproducible workspace gates
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
# Do not publish: ergosbe-benchmarks (publish=false), samples.
# Consumers depend on crates.io versions; monorepo samples keep `path = …`.
# Tag the repo after publish; Aeron submodule pin is independent of crate release.

# Wipe Cargo build artifacts and reset git submodules to the commits pinned by
# this repo (fetch origin, hard-reset + clean dirt, force-checkout).
clean:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== cargo clean (workspace) ==="
    cargo clean
    for d in samples/advanced-bitget samples/cluster-ha-orderbook; do
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
    cd samples/advanced-bitget && cargo build
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
    cd samples/advanced-bitget && cargo fmt --check
    cd samples/advanced-bitget && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/advanced-bitget && cargo test -- --test-threads=1 --skip clickhouse
    cd samples/cluster-ha-orderbook && cargo fmt --check
    cd samples/cluster-ha-orderbook && cargo clippy --all-targets -- -D warnings
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1

# Workspace unit tests only (same --all-features / cluster exclude pattern as check).
test-unit:
    cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
    cargo test -p ergo-aeron-cluster --lib

# Sample IPC tests (embedded Aeron driver — no external services)
test-ipc:
    cd samples/advanced-bitget && cargo test -- --test-threads=1 --skip clickhouse

# Live ClickHouse tests for the IPC sample (requires Docker CH on 127.0.0.1:8123)
test-clickhouse-live:
    @echo "Preflight: checking ClickHouse on 127.0.0.1:8123..."
    @if ! curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; then \
        echo "ClickHouse not available. Start it:"; \
        echo "  bash persist/tests/run-clickhouse.sh start"; \
        exit 1; \
    fi
    @echo "Preflight OK — endpoint http://127.0.0.1:8123 (external Docker)."
    cd samples/advanced-bitget && cargo test --test clickhouse_e2e_test --test e2e_persist_test -- --include-ignored --test-threads=1 --nocapture

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
        cd samples/cluster-ha-orderbook && cargo test --test ha_latency_clickhouse -- --include-ignored --test-threads=1 --nocapture; \
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
    cd samples/advanced-bitget && cargo fmt
    cd samples/cluster-ha-orderbook && cargo fmt

# Auto-fix: fmt + clippy --fix (same feature split as check).
fix:
    just fmt
    cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster --fix --allow-dirty --allow-staged -- -D warnings
    cargo clippy -p ergo-aeron-cluster --all-targets --fix --allow-dirty --allow-staged -- -D warnings
    cd samples/advanced-bitget && cargo clippy --all-targets --all-features --fix --allow-dirty --allow-staged -- -D warnings
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
            -e CLICKHOUSE_USER=default -e CLICKHOUSE_PASSWORD=ergosbe \
            clickhouse/clickhouse-server:latest
    @echo "Waiting for ClickHouse..."
    @until curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; do sleep 2; done
    @echo "ClickHouse ready. Running sample..."
    cd samples/advanced-bitget && cargo run --quiet

# Benchmark parity
bench:
    cd ergosbe-benchmarks && cargo bench --bench perf_parity_bench

# Cluster codec benchmarks (ErgoSBE vs sbe-tool head-to-head)
bench-cluster:
    cargo bench -p ergo-aeron-cluster

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

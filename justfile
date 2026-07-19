# ErgoSBE — reproducible workspace gates

# Wipe Cargo build artifacts and reset git submodules to the commits pinned by
# this repo (fetch origin, hard-reset + clean dirt, force-checkout).
clean:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== cargo clean (workspace) ==="
    cargo clean
    for d in samples/advanced-bitget samples/cluster-ha-orderbook cluster-test-support; do
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
build:
    cargo build --workspace --all-features --exclude ergo-aeron-cluster
    cargo build -p ergo-aeron-cluster
    cd samples/advanced-bitget && cargo build
    cd samples/cluster-ha-orderbook && cargo build

# Full local check: hygiene, format, clippy, tests (no external services).
# ergo-aeron-cluster's test-harness feature needs Java, so it is excluded from
# the --all-features workspace gates below and checked at default features
# (lib only). Run `just test-aeron-cluster-harness` for the Java integration tests.
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

# Workspace unit tests only
test-unit:
    cargo test --workspace --all-features -- --test-threads=1

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
    cargo doc -p ergosbe --no-deps
    cargo doc -p ergo-clickhouse-persist --no-deps
    cargo doc -p ergo-aeron-cluster --no-deps

# Format all handwritten source
fmt:
    cargo fmt --all
    cd samples/advanced-bitget && cargo fmt
    cd samples/cluster-ha-orderbook && cargo fmt

# Coverage (requires nightly toolchain)
cov:
    RUSTC_WRAPPER="" cargo +nightly llvm-cov -p ergosbe --lib --branch --summary-only

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
# cluster/ = crate ergo-aeron-cluster (AI-driven Aeron Cluster client — workspace
# member; cluster-test-support/ = crate ergo-aeron-cluster-test-support, excluded)
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

# Generate cluster SBE codecs from pinned schemas.
# Portable across macOS (BSD sed/shasum) and Linux (GNU sed/sha256sum).
# NOTE: regenerates only the two cluster schemas. The RFQ schema
# (generated/com_aeroncookbook_cluster_rfq_sbe + rfq_codecs/) has no in-repo
# generator yet; this recipe preserves it instead of deleting it.
generate-aeron-cluster-codecs:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== Generating Rust Cluster SBE codecs ==="
    SBE_JAR=$(find ~/.gradle/caches -name 'sbe-tool-1.39.0*.jar' 2>/dev/null | head -1)
    if [ -z "$SBE_JAR" ]; then
      echo "ERROR: sbe-tool-1.39.0.jar not found in ~/.gradle/caches" >&2
      echo "Run the Gradle build once: cd aeron && ./gradlew :aeron-cluster:generateCodecs" >&2
      exit 1
    fi
    AGRONA_JAR=$(find ~/.gradle/caches -name 'agrona-2.5.0*.jar' 2>/dev/null | head -1)
    if [ -z "$AGRONA_JAR" ]; then echo "ERROR: agrona-2.5.0.jar not found" >&2; exit 1; fi
    GEN_DIR="cluster/src/codecs/generated"
    SCHEMA_DIR="aeron/aeron-cluster/src/main/resources/cluster"
    CLUSTER_CODECS="cluster/src/codecs/cluster_codecs"
    MARK_CODECS="cluster/src/codecs/cluster_codecs_mark"

    rm -rf "$GEN_DIR/io_aeron_cluster_codecs" "$GEN_DIR/io_aeron_cluster_codecs_mark"
    java -Dsbe.target.language=Rust -Dsbe.output.dir="$GEN_DIR" -cp "$SBE_JAR:$AGRONA_JAR" \
      uk.co.real_logic.sbe.SbeTool "$SCHEMA_DIR/aeron-cluster-codecs.xml"
    java -Dsbe.target.language=Rust -Dsbe.output.dir="$GEN_DIR" -cp "$SBE_JAR:$AGRONA_JAR" \
      uk.co.real_logic.sbe.SbeTool "$SCHEMA_DIR/aeron-cluster-mark-codecs.xml"

    mkdir -p "$CLUSTER_CODECS" "$MARK_CODECS"
    cp "$GEN_DIR/io_aeron_cluster_codecs/src/lib.rs" "$CLUSTER_CODECS/mod.rs"
    cp "$GEN_DIR/io_aeron_cluster_codecs/src/"*.rs "$CLUSTER_CODECS/"
    cp "$GEN_DIR/io_aeron_cluster_codecs_mark/src/lib.rs" "$MARK_CODECS/mod.rs"
    cp "$GEN_DIR/io_aeron_cluster_codecs_mark/src/"*.rs "$MARK_CODECS/"

    for dir in "$CLUSTER_CODECS" "$MARK_CODECS"; do
      for f in "$dir"/*.rs; do
        sed -i.bak 's/use crate::\*;/use super::*;/g' "$f" && rm -f "$f.bak"
        sed -i.bak 's/pub use crate::SBE_/pub use super::SBE_/g' "$f" && rm -f "$f.bak"
      done
    done
    (cd cluster && cargo fmt)

    echo "=== Codecs updated ==="
    if command -v sha256sum >/dev/null 2>&1; then SHA256=(sha256sum); elif command -v shasum >/dev/null 2>&1; then SHA256=(shasum -a 256); else echo "ERROR: neither sha256sum nor shasum available" >&2; exit 1; fi
    find "$CLUSTER_CODECS" "$MARK_CODECS" -name '*.rs' -exec "${SHA256[@]}" {} \; | sort > "$GEN_DIR/.checksum"
    echo "=== Checksum saved ==="

# Check for codec drift across every directory generate-aeron-cluster-codecs writes.
check-aeron-cluster-codec-drift:
    #!/usr/bin/env bash
    set -euo pipefail
    just generate-aeron-cluster-codecs
    # Residual sbe-tool trees (benches + RFQ). Production ErgoSBE codecs are OUT_DIR-only.
    if ! git diff --exit-code \
        cluster/src/codecs/cluster_codecs/ \
        cluster/src/codecs/cluster_codecs_mark/ \
        cluster/src/codecs/rfq_codecs/; then
      echo "ERROR: Codec drift detected! Run 'just generate-aeron-cluster-codecs' and commit." >&2
      exit 1
    fi
    echo "OK: Residual sbe-tool codecs match committed (RFQ/benches)."

# Build Aeron cluster test jars (requires Java 17+; run once before test-harness tests)
build-aeron-jars:
    cd aeron && ./gradlew :aeron-cluster:jar :aeron-archive:jar :aeron-all:jar

hash-aeron-jars:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v sha256sum >/dev/null 2>&1; then SHA256=(sha256sum); elif command -v shasum >/dev/null 2>&1; then SHA256=(shasum -a 256); else echo "ERROR: neither sha256sum nor shasum available" >&2; exit 1; fi
    echo "# SHA-256 hashes of test jars" > cluster-test-support/test-jars.sha256
    for dir in aeron-all aeron-cluster aeron-archive; do
      jar=$(find aeron/$dir/build/libs -name '*.jar' ! -name '*sources*' ! -name '*javadoc*' -print -quit)
      if [ -n "$jar" ]; then
        "${SHA256[@]}" "$jar" | tee -a cluster-test-support/test-jars.sha256
      fi
    done
    echo "=== SHA-256 hashes saved to cluster-test-support/test-jars.sha256 ==="

check-aeron-jars:
    #!/usr/bin/env bash
    set -euo pipefail
    just hash-aeron-jars
    if ! git diff --exit-code cluster-test-support/test-jars.sha256; then
      echo "ERROR: Jar SHA-256 mismatch. Run 'just hash-aeron-jars' and commit." >&2
      exit 1
    fi
    echo "OK: Jar hashes match committed."

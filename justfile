# ErgoSBE — reproducible workspace gates

# Full local check: hygiene, format, clippy, tests (no external services)
check:
    ./scripts/check-repository-hygiene.sh
    cargo fmt --all --check
    cargo clippy --workspace --all-targets --all-features -- -D warnings
    cargo test --workspace --all-features -- --test-threads=1
    cd samples/advanced-bitget && cargo fmt --check
    cd samples/advanced-bitget && cargo clippy --all-targets --all-features -- -D warnings
    cd samples/advanced-bitget && cargo test -- --test-threads=1 --skip clickhouse

# Workspace unit tests only
test-unit:
    cargo test --workspace --all-features -- --test-threads=1

# Sample IPC tests (embedded Aeron driver — no external services)
test-ipc:
    cd samples/advanced-bitget && cargo test -- --test-threads=1 --skip clickhouse

# Live ClickHouse tests (requires Docker ClickHouse on 127.0.0.1:8123)
test-clickhouse-live:
    @echo "Preflight: checking ClickHouse on 127.0.0.1:8123..."
    @if ! curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; then \
        echo "ClickHouse not available. Start it:"; \
        echo "  docker run -d --name ergo-clickhouse -p 8123:8123 -p 9000:9000 clickhouse/clickhouse-server:latest"; \
        exit 1; \
    fi
    cd samples/advanced-bitget && cargo test -- --test-threads=1 clickhouse --nocapture

# Format all handwritten source
fmt:
    cargo fmt --all
    cd samples/advanced-bitget && cargo fmt

# Coverage (requires nightly toolchain)
cov:
    RUSTC_WRAPPER="" cargo +nightly llvm-cov -p ergosbe --lib --branch --summary-only

# Start ClickHouse Docker and run the advanced sample
run-sample:
    @echo "Starting ClickHouse..."
    @docker start ergo-clickhouse 2>/dev/null || \
        docker run -d --name ergo-clickhouse \
            -p 8123:8123 -p 9000:9000 \
            clickhouse/clickhouse-server:latest
    @echo "Waiting for ClickHouse..."
    @until curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; do sleep 2; done
    @echo "ClickHouse ready. Running sample..."
    cd samples/advanced-bitget && cargo run --quiet

# Benchmark parity
bench:
    cd ergosbe-benchmarks && cargo bench --bench perf_parity_bench

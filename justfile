# ErgoSBE — just tasks

# Start ClickHouse Docker and run the advanced SBE sample
run-sample:
    @echo "Starting ClickHouse..."
    @docker start ergo-clickhouse 2>/dev/null || \
        docker run -d --name ergo-clickhouse \
            -p 8123:8123 -p 9000:9000 \
            -e CLICKHOUSE_USER=default \
            -e CLICKHOUSE_PASSWORD=ergosbe \
            clickhouse/clickhouse-server:latest
    @echo "Waiting for ClickHouse..."
    @until curl -s -o /dev/null -w "" http://127.0.0.1:8123/ping --user default:ergosbe; do sleep 2; done
    @echo "ClickHouse ready. Running sample..."
    cd samples/advanced-bitget && cargo run --quiet

# Run all tests
test:
    cargo test --workspace -- --test-threads=1
    cd samples/advanced-bitget && cargo test -- --test-threads=1

# Format and lint
check:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings

# Benchmark
bench:
    cd ergosbe-benchmarks && cargo bench --bench perf_parity_bench

# Coverage (requires nightly)
cov:
    RUSTC_WRAPPER="" cargo +nightly llvm-cov -p ergosbe --lib --branch --summary-only

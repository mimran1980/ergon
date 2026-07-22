# ── sample recipes ───────────────────────────────────────────────────────

# Start ClickHouse Docker and run the exchange example.
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

# Sample IPC tests (embedded Aeron driver — no external services).
test-ipc:
    cd samples/exchange-example && cargo test -- --test-threads=1 --skip clickhouse

# Live ClickHouse tests for the exchange example (requires Docker CH on 127.0.0.1:8123).
test-clickhouse-live:
    @echo "Preflight: checking ClickHouse on 127.0.0.1:8123..."
    @if ! curl -sf http://127.0.0.1:8123/ping >/dev/null 2>&1; then \
        echo "ClickHouse not available. Start it:"; \
        echo "  bash persist/tests/run-clickhouse.sh start"; \
        exit 1; \
    fi
    @echo "Preflight OK — endpoint http://127.0.0.1:8123 (external Docker)."
    (cd samples/exchange-example && cargo test --test clickhouse_e2e_test --test e2e_persist_test -- --include-ignored --test-threads=1 --nocapture

# Alias: exchange example live CH E2E (typed + Persist DTO snapshot).
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

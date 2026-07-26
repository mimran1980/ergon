# ── sample recipes ───────────────────────────────────────────────────────

# Run the exchange example (pure Rust — no external services).
run-sample:
    cd samples/exchange-example && cargo run --quiet

# Sample IPC tests (embedded Aeron driver — no external services).
test-ipc:
    cd samples/exchange-example && cargo test -- --test-threads=1

# HA cluster sample: offline try_claim + never-stale book proofs.
samples-cluster-ha:
    @echo "=== cluster-ha-orderbook offline pipeline (try_claim path + stale-book) ==="
    cd samples/cluster-ha-orderbook && cargo test --lib --test ha_offline_pipeline -- --test-threads=1

# Multi-node Java kill-leader never-stale book (needs Aeron jars + Java).
samples-cluster-ha-kill-leader:
    cd samples/cluster-ha-orderbook && cargo test --features test-harness --test ha_kill_leader -- --test-threads=1 --nocapture

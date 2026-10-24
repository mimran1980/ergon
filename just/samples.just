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

# Low-latency Kubernetes sample: fmt, clippy, unit and logger tests, manifests (kubectl, helm, Docker for kubeconform), entrypoint fixtures, shellcheck
samples-ll:
    cd samples/low-latency-k8s && cargo fmt --check
    cd samples/low-latency-k8s && cargo clippy --all-targets -- -D warnings
    cd samples/low-latency-k8s && cargo test --lib --test logger_nonblocking --test logger_alloc -- --test-threads=1
    cd samples/low-latency-k8s && cargo test --test manifests -- --test-threads=1
    bash samples/low-latency-k8s/scripts/test-entrypoint.sh
    docker run --rm -v "$PWD/samples/low-latency-k8s:/mnt:ro" -w /mnt koalaman/shellcheck:v0.10.0 -S warning docker/*.sh scripts/*.sh node/*.sh

# The sample's Linux lanes in Docker (placement tests included), arm64 and amd64
samples-ll-docker:
    bash samples/low-latency-k8s/scripts/test-docker.sh --arch arm64
    bash samples/low-latency-k8s/scripts/test-docker.sh --arch amd64

# The vdev end-to-end run in Docker: driver, ping, pong, ll-probe (builds ll:arm64-generic if missing)
samples-ll-e2e *args:
    bash samples/low-latency-k8s/scripts/e2e-vdev.sh --docker {{args}}

# The driver entrypoint against fixtures
samples-ll-entrypoint:
    bash samples/low-latency-k8s/scripts/test-entrypoint.sh

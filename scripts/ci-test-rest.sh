#!/usr/bin/env bash
# CI's `test` job, everything but ergo-sbe's integration test binaries (those
# run in scripts/ci-test-shard.sh): the reference and golden checks, ergo-sbe's
# lib tests, the rest of the workspace, the cluster crate, the doc tests and
# ergon-io's mio parity gate (test-lanes.tsv `io-parity`; last, so no other
# test shares the runner's CPUs with its timed rounds).
set -euo pipefail
cd "$(dirname "$0")/.."

./scripts/regenerate-sbe-tool-reference.sh --check
./scripts/regenerate-golden.sh --check
./scripts/regenerate-codegen-matrix-fixture.py --check
cargo test -p ergo-sbe --all-features --lib
cargo test --workspace --all-features \
    --exclude ergo-sbe --exclude ergo-aeron-cluster --exclude ergo-aeron-cluster-test-harness \
    --exclude ergon-md-dpdk --exclude ergon-md-dpdk-harness -- --test-threads=2
cargo test -p ergo-aeron-cluster -- --test-threads=1
cargo test --doc --workspace --exclude ergo-aeron-cluster --exclude ergo-aeron-cluster-test-harness
cargo test --release --manifest-path io/benches/Cargo.toml --test mio_parity -- --test-threads=1 --nocapture

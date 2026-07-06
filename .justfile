# ErgoSBE task runner — `just <task>`

set shell := ["bash", "-cu"]

# —— Rust tasks ——

default:
    @just --list --unsorted

# fmt + clippy + check
check:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo doc --no-deps --workspace

# auto-fix formatting and clippy lints
fix:
    cargo fmt --all
    cargo clippy --workspace --all-targets --fix --allow-dirty --allow-staged

# run all tests (with and without disable-bounds-checks feature)
test:
    echo "=== default features ===" && cargo test --workspace --all-targets -- --nocapture
    echo "=== disable-bounds-checks ===" && cargo test --workspace --all-targets --features disable-bounds-checks -- --nocapture
    cargo test --doc

# run benchmarks (once they exist)
bench:
    cargo bench --workspace

# HFT-specific benchmarks: decode tight-loop, field striding, throughput
bench-hft:
    cargo bench --bench decode_bench -- hft
    cargo bench --bench throughput_bench

# open generated docs
docs:
    cargo doc --no-deps --workspace --open

# CI monitoring — check latest CI run status
ci-status limit='3':
    ./ci-monitor.sh {{limit}}

# check for unused deps
deps:
    cargo +nightly udeps --workspace 2>/dev/null || echo "install nightly and cargo-udeps"

# clean build artifacts
clean:
    cargo clean

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

# run all tests
test:
    cargo test --workspace --all-targets -- --nocapture
    cargo test --doc

# run benchmarks (once they exist)
bench:
    cargo bench --workspace

# open generated docs
docs:
    cargo doc --no-deps --workspace --open

# check for unused deps
deps:
    cargo +nightly udeps --workspace 2>/dev/null || echo "install nightly and cargo-udeps"

# clean build artifacts
clean:
    cargo clean

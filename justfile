# ErgoSBE — build, test, and lint recipes

# Default: build workspace crates (sbe + persist)
default: build

# Build workspace crates only
build:
    RUSTC_WRAPPER="" cargo build --workspace

# Build all crates including samples
build-all: build
    RUSTC_WRAPPER="" cargo build --manifest-path samples/exchange-orderbook/Cargo.toml

# Run all workspace tests
test:
    RUSTC_WRAPPER="" cargo test --workspace

# Run all tests including samples
test-all: test
    RUSTC_WRAPPER="" cargo test --manifest-path samples/exchange-orderbook/Cargo.toml

# Format check
fmt:
    cargo fmt --all --check

# Format apply
fmt-fix:
    cargo fmt --all

# Clippy (workspace)
clippy:
    RUSTC_WRAPPER="" cargo clippy --workspace --all-targets -- -D warnings

# Clippy including samples
clippy-all: clippy
    RUSTC_WRAPPER="" cargo clippy --manifest-path samples/exchange-orderbook/Cargo.toml --all-targets -- -D warnings

# Full CI check: format, clippy, test
ci: fmt clippy test
    @echo "CI: all checks passed"

# Full CI including samples
ci-all: fmt clippy-all test-all
    @echo "CI-all: all checks passed"

# Regenerate golden file after codegen changes
update-golden:
    RUSTC_WRAPPER="" cargo test -p ergosbe --test stability_test -- update_golden --ignored

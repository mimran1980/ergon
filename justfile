# ── ErgoSBE command runner ──────────────────────────────────────────────

check:
    cargo check --workspace
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- -D warnings

fix:
    cargo fmt --all
    cargo clippy --workspace --fix --allow-dirty

test:
    cargo test --workspace --all-targets -- --nocapture
    cargo test --doc

bench:
    cargo bench --workspace

bench-hft:
    cargo bench --workspace -- decode/hft/

ci: check test docs bench

docs:
    cargo doc --no-deps --workspace --open

clean:
    cargo clean

deps:
    cargo tree --workspace
    cargo check --workspace 2>&1 | head -20 || true

# ── housekeeping recipes ──────────────────────────────────────────────────

# Wipe Cargo build artifacts and reset git submodules.
clean:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== cargo clean (workspace) ==="
    cargo clean
    for d in samples/exchange-example samples/cluster-ha-orderbook samples/cluster-rfq; do
      if [ -f "$d/Cargo.toml" ]; then
        echo "=== cargo clean ($d) ==="
        (cd "$d" && cargo clean)
      fi
    done
    echo "=== git submodules → origin / pinned commits ==="
    git submodule sync --recursive
    git submodule foreach --recursive '
      set -e
      git fetch origin --tags 2>/dev/null || git fetch origin || true
      git reset --hard
      git clean -fdx
    '
    git submodule update --init --recursive --force
    echo "clean: done"

# Build rustdoc for the two product crates (no deps).
docs:
    cargo doc -p ergo-sbe --no-deps
    cargo doc -p ergo-aeron-cluster --no-deps

# Coverage (requires nightly toolchain).
cov:
    RUSTC_WRAPPER="" cargo +nightly llvm-cov -p ergo-sbe --lib --branch --summary-only

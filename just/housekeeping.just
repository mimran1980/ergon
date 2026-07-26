# ── housekeeping recipes ──────────────────────────────────────────────────

# Wipe Cargo build artifacts, gitignored sample codegen, and reset submodules.
clean:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== cargo clean (workspace) ==="
    cargo clean
    for d in samples/*/; do
      if [ -f "${d}Cargo.toml" ]; then
        echo "=== cargo clean (${d%/}) ==="
        (cd "$d" && cargo clean)
      fi
    done
    # IDE-friendly codecs from build.rs (**/src/generated/ is gitignored).
    echo "=== remove sample src/generated ==="
    for d in samples/*/src/generated; do
      if [ -d "$d" ]; then
        echo "rm -rf $d"
        rm -rf "$d"
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

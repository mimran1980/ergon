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
    just _reset-sources

# The source half of `clean`, which `just release` starts with: remove the
# gitignored sample codegen (and make cargo regenerate it) and reset every
# submodule to its pinned commit (`git clean -fdx` also removes Aeron jars
# built from other sources). Cargo target directories are kept.
_reset-sources:
    #!/usr/bin/env bash
    set -euo pipefail
    # IDE-friendly codecs from build.rs (**/src/generated/ is gitignored).
    echo "=== remove sample src/generated ==="
    for d in samples/*/src/generated; do
      if [ -d "$d" ]; then
        echo "rm -rf $d"
        rm -rf "$d"
      fi
    done
    # Those build scripts rerun only when their schema or ergo-sbe changes, so
    # with a warm target directory the deleted codecs would not come back and
    # the sample would fail to compile. A newer build.rs mtime makes cargo
    # rebuild and rerun each one (seconds; no dependency rebuilds).
    for b in samples/*/build.rs; do
      touch "$b"
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
    echo "sources reset"

# Build rustdoc for the two product crates (no deps).
docs:
    cargo doc -p ergo-sbe --no-deps
    cargo doc -p ergo-aeron-cluster --no-deps

# Coverage (requires nightly toolchain).
cov:
    RUSTC_WRAPPER="" cargo +nightly llvm-cov -p ergo-sbe --lib --branch --summary-only

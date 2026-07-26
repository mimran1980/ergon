# ── Aeron / Java / cluster harness recipes ────────────────────────────────

# Check the cluster crate (lib: fmt + clippy + tests, no Java required).
check-aeron-cluster:
    cd cluster && cargo fmt --check
    cd cluster && cargo clippy --all-targets -- -D warnings
    cd cluster && cargo test --lib

# Cluster integration tests (requires Java 17+ and built Aeron jars — run
# `just build-aeron-jars` once first; slow).
test-aeron-cluster-harness:
    cd cluster && cargo test --features test-harness -- --test-threads=1

# Build Aeron cluster test jars (requires Java 17+; run once before test-harness tests).
build-aeron-jars:
    cd aeron && ./gradlew :aeron-cluster:jar :aeron-archive:jar :aeron-all:jar

hash-aeron-jars:
    #!/usr/bin/env bash
    set -euo pipefail
    if command -v sha256sum >/dev/null 2>&1; then SHA256=(sha256sum); elif command -v shasum >/dev/null 2>&1; then SHA256=(shasum -a 256); else echo "ERROR: neither sha256sum nor shasum available" >&2; exit 1; fi
    echo "# SHA-256 hashes of test jars" > cluster/test-jars.sha256
    for dir in aeron-all aeron-cluster aeron-archive; do
      jar=$(find aeron/$dir/build/libs -name '*.jar' ! -name '*sources*' ! -name '*javadoc*' -print -quit)
      if [ -n "$jar" ]; then
        "${SHA256[@]}" "$jar" | tee -a cluster/test-jars.sha256
      fi
    done
    echo "=== SHA-256 hashes saved to cluster/test-jars.sha256 ==="

check-aeron-jars:
    #!/usr/bin/env bash
    set -euo pipefail
    just hash-aeron-jars
    if ! git diff --exit-code cluster/test-jars.sha256; then
      echo "ERROR: Jar SHA-256 mismatch. Run 'just hash-aeron-jars' and commit." >&2
      exit 1
    fi
    echo "OK: Jar hashes match committed."

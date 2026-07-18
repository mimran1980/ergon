# Phase 1: Scaffolding

> **Historical.** Written when the crate was `rusteron-cluster`; it is now crate `ergo-aeron-cluster` in `cluster/` (test harness: crate `ergo-aeron-cluster-test-support` in `cluster-test-support/`). Living doc: [2026-07-18-ergosbe-experimental-master-plan.md](2026-07-18-ergosbe-experimental-master-plan.md).

> Part of [master plan](./2026-07-17-rusteron-cluster-master.md)

**Goal:** Create the `rusteron-cluster` and `rusteron-java-test-support` crate skeletons, wire them into the workspace, and add `just` recipe stubs so `just build` compiles both crates.

**Gate:** `just build` succeeds with both new crates; `cargo test -p rusteron-cluster --lib` passes (no-op test).

---

## Task 1.1: Create `rusteron-cluster` crate

**Files:**
- Create: `rusteron-cluster/Cargo.toml`
- Create: `rusteron-cluster/src/lib.rs`
- Create: `rusteron-cluster/build.rs`
- Create: `rusteron-cluster/README.md`

- [ ] **Write `rusteron-cluster/Cargo.toml`**

```toml
[package]
name = "rusteron-cluster"
version = "0.1.0"
edition = "2024"
description = "Prototype handwritten Rust reimplementation of the Aeron Cluster client"
license = "Apache-2.0"
repository = "https://github.com/pistonite/rusteron"

[dependencies]
rusteron-client = { path = "../rusteron-client" }
rusteron-code-gen = { path = "../rusteron-code-gen" }
rusteron-java-test-support = { path = "../rusteron-java-test-support", optional = true }

[features]
default = []
test-harness = ["rusteron-java-test-support"]

[lib]
name = "rusteron_cluster"
path = "src/lib.rs"

[dev-dependencies]
serial_test = { version = "3", features = ["file_locks"] }
```

- [ ] **Write `rusteron-cluster/src/lib.rs`**

```rust
//! # Rusteron Cluster Client
//!
//! ⚠️ **TEMPORARY PROTOTYPE.** This is a handwritten Rust reimplementation
//! of the [Aeron Cluster](https://github.com/real-logic/aeron) *client*
//! (no C bindings). It is heavily LLM-assisted, lightly human-reviewed,
//! and less tested than the Java reference.
//!
//! **Delete this crate when official Aeron Cluster C bindings become
//! available.** Bugs in Rusteron's pub/sub layer OR in this
//! reimplementation may cause undefined behaviour, segfaults, or data
//! loss.

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_compiles() {
        assert!(true);
    }
}
```

- [ ] **Write `rusteron-cluster/build.rs`** (minimal — validates codecs exist)

```rust
fn main() {
    let codecs_dir =
        std::path::Path::new("src/codecs/generated");
    if codecs_dir.exists() {
        println!("cargo::rerun-if-changed=src/codecs/generated");
    }
    // Codecs are committed; build.rs does NOT regenerate.
    // Use `just generate-cluster-codecs` for regeneration.
}
```

- [ ] **Write `rusteron-cluster/README.md`**

```markdown
# rusteron-cluster

⚠️ **TEMPORARY PROTOTYPE.** This is a handwritten Rust reimplementation of
the Aeron Cluster *client* (no C bindings). It is heavily LLM-assisted,
lightly human-reviewed, and less tested than the Java reference.

**Delete this crate when official Aeron Cluster C bindings become
available.** Bugs in Rusteron's pub/sub layer OR in this reimplementation
may cause undefined behaviour, segfaults, or data loss.

## Overview

Pure-Rust Aeron Cluster client protocol implementation on top of
`rusteron-client` transport. Mirrors the Java
`io.aeron.cluster.client.AeronCluster` API and SBE session protocol.

## Features

- `test-harness` — enables `rusteron-java-test-support` dependency for
  integration tests (requires Java 17+).
```

- [ ] **Compile to verify**

```bash
cargo check -p rusteron-cluster
```

Expected: compiles clean (only rusteron-client + rusteron-code-gen deps; no harness).

- [ ] **Run scaffold test**

```bash
cargo test -p rusteron-cluster --lib
```

Expected: `scaffold_compiles` passes.

- [ ] **Commit**

```bash
git add rusteron-cluster/
git commit -m "feat: scaffold rusteron-cluster crate"
```

---

## Task 1.2: Create `rusteron-java-test-support` crate

**Files:**
- Create: `rusteron-java-test-support/Cargo.toml`
- Create: `rusteron-java-test-support/src/lib.rs`
- Create: `rusteron-java-test-support/build.rs`

- [ ] **Write `rusteron-java-test-support/Cargo.toml`**

```toml
[package]
name = "rusteron-java-test-support"
version = "0.1.0"
edition = "2024"
description = "Java test harness for rusteron-cluster integration tests"
license = "Apache-2.0"

[dependencies]
rusteron-client = { path = "../rusteron-client" }
rusteron-archive = { path = "../rusteron-archive" }

[lib]
name = "rusteron_java_test_support"
path = "src/lib.rs"

[dev-dependencies]
serial_test = { version = "3", features = ["file_locks"] }
```

- [ ] **Write `rusteron-java-test-support/src/lib.rs`**

```rust
//! Java test harness for rusteron-cluster integration tests.
//!
//! Spawns the official Java Aeron consensus module and archive driver
//! for integration testing. Requires Java 17+ and Gradle-built jars.
//!
//! ⚠️ **Test-only.** Not for production use. PIDs are tracked and
//! processes are killed on drop, but process safety is best-effort.

#[cfg(test)]
mod tests {
    #[test]
    fn scaffold_compiles() {
        assert!(true);
    }
}
```

- [ ] **Write `rusteron-java-test-support/build.rs`** (minimal — Gradle will be added in harness phase)

```rust
fn main() {
    // Gradle jar building will be added in the harness implementation
    // phase. For now, this is a placeholder.
    println!("cargo::rerun-if-changed=build.rs");
}
```

- [ ] **Compile to verify**

```bash
cargo check -p rusteron-java-test-support
```

- [ ] **Compile cluster with harness feature**

```bash
cargo check -p rusteron-cluster --features test-harness
```

- [ ] **Commit**

```bash
git add rusteron-java-test-support/
git commit -m "feat: scaffold rusteron-java-test-support crate"
```

---

## Task 1.3: Wire into workspace + justfile

**Files:**
- Modify: `Cargo.toml`
- Modify: `.justfile`

- [ ] **Add new crates to workspace `Cargo.toml` members**

Open `Cargo.toml` and add to the `members` array:

```toml
members = [
    "rusteron-code-gen",
    "rusteron-media-driver",
    "rusteron-client",
    "rusteron-archive",
    "rusteron-cluster",
    "rusteron-java-test-support",
]
```

- [ ] **Add `just` recipes to `.justfile`**

Append to `.justfile`:

```makefile
# --- rusteron-cluster ---

# Check cluster crate (check + clippy + fmt)
check-cluster:
    cargo clippy --all-features -p rusteron-cluster -- -D warnings
    cargo fmt --check -p rusteron-cluster
    cargo test --all-features -p rusteron-cluster --lib

# Generate cluster SBE codecs from pinned schemas
generate-cluster-codecs:
    #!/usr/bin/env bash
    set -euo pipefail
    echo "=== Generating Rust Cluster SBE codecs ==="
    SBE_JAR=$(find ~/.gradle/caches -name 'sbe-tool-1.39.0*.jar' 2>/dev/null | head -1)
    if [ -z "$SBE_JAR" ]; then
      echo "ERROR: sbe-tool-1.39.0.jar not found in ~/.gradle/caches" >&2
      echo "Run the Gradle build once: cd rusteron-client/aeron && ./gradlew :aeron-cluster:generateCodecs" >&2
      exit 1
    fi
    OUT_DIR="rusteron-cluster/src/codecs/generated"
    mkdir -p "$OUT_DIR"
    SCHEMA_DIR="rusteron-client/aeron/aeron-cluster/src/main/resources/cluster"
    java -jar "$SBE_JAR" \
      -Dsbe.target.language=Rust \
      -Dsbe.output.dir="$OUT_DIR" \
      -Dsbe.xinclude.aware=true \
      "$SCHEMA_DIR/aeron-cluster-codecs.xml"
    java -jar "$SBE_JAR" \
      -Dsbe.target.language=Rust \
      -Dsbe.output.dir="$OUT_DIR" \
      -Dsbe.xinclude.aware=true \
      "$SCHEMA_DIR/aeron-cluster-mark-codecs.xml"
    echo "=== Codecs generated in $OUT_DIR ==="
    # Record checksum
    find "$OUT_DIR" -type f -name '*.rs' -exec sha256sum {} \; | sort > "$OUT_DIR/.checksum"
    echo "=== Checksum saved to $OUT_DIR/.checksum ==="

# Check for codec drift
check-cluster-codec-drift:
    #!/usr/bin/env bash
    set -euo pipefail
    just generate-cluster-codecs
    if ! git diff --exit-code rusteron-cluster/src/codecs/generated/; then
      echo "ERROR: Codec drift detected! Generated codecs differ from committed." >&2
      echo "Run 'just generate-cluster-codecs' and commit the changes." >&2
      exit 1
    fi
    echo "OK: Generated codecs match committed."
```

- [ ] **Verify workspace builds end-to-end**

```bash
just build
```

- [ ] **Commit**

```bash
git add Cargo.toml .justfile
git commit -m "feat: wire rusteron-cluster + rusteron-java-test-support into workspace"
```

---

## Task 1.4: Verify gate

```bash
just build && cargo test -p rusteron-cluster --lib
```

Expected: both crates compile; scaffold test passes. Phase 1 complete.

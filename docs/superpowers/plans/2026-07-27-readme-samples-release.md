# README, Samples, and 0.1.1 Release Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship a CI-checked feature README, current samples, version 0.1.1, and one post-0.1.0 release commit.

**Architecture:** `docs_validation_test` supplies a generated fixture module to
each README snippet crate, turning bare Rust fences into compile-time API
contracts. Repository samples remain the complete runnable examples and are
verified by the root `just` gates. Release metadata is controlled by the
workspace version and Cargo lockfiles.

**Tech Stack:** Rust 1.95, Cargo, `syn`, `rustfmt`, Criterion, just, Git.

## Global Constraints

- Message-header multi-byte fields follow the schema’s `byteOrder`.
- Generated sbe-tool references remain reproducible from submodule commit
  `15f2b9c2380b9814d1d0f5ec2ef42e6baf01d78e`.
- Bare `rust` README fences must compile; schematic fragments must be labelled
  `rust,ignore`.
- Product crates are version 0.1.1; samples remain unpublished.
- Do not push.
- Do not create intermediate commits; the user requested one final squashed
  commit after `initial release 0.1.0`.

---

### Task 1: Canonical golden comparison

**Files:**
- Modify: `sbe/tests/stability_test.rs`
- Test: `sbe/tests/stability_test.rs`

**Interfaces:**
- Consumes: generated module source and the checked-in golden Rust file.
- Produces: `canonical_rust(&str) -> Result<String, Box<dyn Error>>`.

- [ ] **Step 1: Preserve the observed formatter failure**

Run:

```sh
cargo test -p ergo-sbe --test stability_test generated_output_matches_golden
```

Expected before the fix: failure after `cargo fmt` despite equivalent syntax.

- [ ] **Step 2: Canonicalize both sides**

Parse the source, run the project’s required formatter through stdin/stdout,
and compare the canonical results:

```rust
fn canonical_rust(source: &str) -> Result<String, Box<dyn Error>> {
    let _ = syn::parse_file(source)?;
    // Run `rustfmt --emit stdout --edition 2024` with `source` on stdin.
    // Return its stdout, or an error when parsing/formatting fails.
}
```

Compare `canonical_rust(&output)?` with `canonical_rust(&golden)?`.

- [ ] **Step 3: Verify format and stability**

Run:

```sh
cargo fmt --all --check
cargo test -p ergo-sbe --test stability_test
```

Expected: format and stability both pass.

### Task 2: Compile generated-API README fences

**Files:**
- Modify: `sbe/tests/docs_validation_test.rs`
- Modify: `sbe/README.md`
- Test: `sbe/tests/docs_validation_test.rs`

**Interfaces:**
- Consumes: `docs_schema_xml()` and `Generator`.
- Produces: a generated `docs_codec.rs` module available to every compiled
  README snippet as `use docs_codec::*`.

- [ ] **Step 1: Extend the snippet harness**

Generate the fixture once in `readme_rust_fences_compile`, pass its source to
`compile_snippet`, write `src/docs_codec.rs`, and prepend:

```rust
mod docs_codec;
use docs_codec::*;
```

to each snippet crate.

- [ ] **Step 2: Add feature examples**

Use actual fixture types and signatures:

```rust
let mut buf = [0u8; HeartbeatEncoder::ENCODED_LENGTH];
let mut enc = HeartbeatEncoder::try_wrap_and_apply_header(&mut buf, 0)?;
enc.seq(7);
let dec = HeartbeatDecoder::try_from(buf.as_slice())?;
assert_eq!(dec.seq(), 7);
```

For dynamic messages, encode `Quote` with `legs` followed by `note`, then
decode and consume the same order. Include fixed-array helpers, domain DTO
round-trip, `AnyMessage`, and XSD/generator configuration examples.

- [ ] **Step 3: Make fence accounting strict**

Assert a meaningful minimum number of bare runnable fences and report the
README line for every compile failure.

- [ ] **Step 4: Verify docs**

Run:

```sh
cargo test -p ergo-sbe --test docs_validation_test -- --test-threads=1
```

Expected: every bare Rust fence compiles and the generated API smoke runs.

### Task 3: Sample review and modernization

**Files:**
- Modify if stale: `samples/README.md`
- Modify if stale: `sbe/README.md`
- Modify if stale: sample source and manifests under `samples/`

**Interfaces:**
- Consumes: current generated APIs.
- Produces: correct sample dependency-pattern documentation and service-free
  builds/tests.

- [ ] **Step 1: Audit manifests and README claims**

Verify whether each sample is build-only, runtime-only, or build+runtime.
Correct the main README table so `sbe-feature-tour` is build-only.

- [ ] **Step 2: Compile and test samples**

Run:

```sh
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml
cargo test --manifest-path samples/l3-book/Cargo.toml
cargo test --manifest-path samples/exchange-example/Cargo.toml
cargo test --manifest-path samples/cluster-ha-orderbook/Cargo.toml --lib --test ha_offline_pipeline
cargo build --manifest-path samples/cluster-rfq/Cargo.toml --examples
cargo check --manifest-path samples/cluster-tutorial/Cargo.toml --all-targets
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example flyweight
cargo run --manifest-path samples/sbe-codegen-examples/Cargo.toml --example domain_objects
```

- [ ] **Step 3: Link complete examples**

Reference the exact feature-tour functions and the focused l3/exchange samples
from the corresponding README feature sections.

### Task 4: Version 0.1.1

**Files:**
- Modify: `Cargo.toml`
- Modify: `cluster/Cargo.toml`
- Modify: tracked `Cargo.lock` files through Cargo
- Modify: README dependency snippets

**Interfaces:**
- Consumes: workspace package version.
- Produces: publishable `ergo-sbe` and `ergo-aeron-cluster` 0.1.1 metadata.

- [ ] **Step 1: Update manifest versions**

Set:

```toml
[workspace.package]
version = "0.1.1"
```

and update version-qualified internal `ergo-sbe` dependencies to `0.1.1`.

- [ ] **Step 2: Update README dependency examples**

Use `ergo-sbe = "0.1.1"` in copyable Cargo snippets.

- [ ] **Step 3: Refresh lockfiles**

Run Cargo metadata/build commands for the workspace and excluded samples so
all tracked lockfiles resolve local product packages as 0.1.1.

### Task 5: Final verification and history rewrite

**Files:**
- Verify: entire repository
- Rewrite: Git commits after `b462e8e`

**Interfaces:**
- Consumes: final tested tree.
- Produces: exactly three commits on `main`.

- [ ] **Step 1: Run reproducibility and performance gates**

Run:

```sh
./scripts/regenerate-sbe-tool-reference.sh
cargo test -p ergo-sbe --test sbe_tool_multi_schema_wire_parity_test --test sbe_tool_wire_parity_test
cargo bench -p ergo-sbe-benchmarks
cargo bench -p ergo-aeron-cluster
./scripts/check-bench-gate.sh target/criterion
```

- [ ] **Step 2: Run release gates**

Run in order:

```sh
just build
just fix
just test
git diff --check
```

- [ ] **Step 3: Squash the authorized range**

Confirm `b462e8e` is `initial release 0.1.0`, then:

```sh
git reset --soft b462e8e
git add -A
git commit -m "release: 0.1.1 parity fixes and comprehensive tests"
```

- [ ] **Step 4: Verify final history and tree**

Run:

```sh
git log --oneline --decorate -4
git status --short
git show --stat --oneline HEAD
```

Expected: clean tree and exactly three commits on `main`, with the 0.1.1
release commit above `initial release 0.1.0` and `Initial ErgoSBE scaffold`.

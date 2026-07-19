# Contributing

ErgoSBE is intentionally strict about **official-SBE wire compatibility** and
generated-code quality. Preserve the priority ladder in the root README
(compat → maintained performance → zero-cost safety → simplicity).

## Local gates

Prefer the justfile (same shape as CI hygiene):

```sh
just check
```

Manual equivalent (note the cluster exclude):

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features --exclude ergo-aeron-cluster -- -D warnings
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
cargo test --workspace --all-features --exclude ergo-aeron-cluster -- --test-threads=1
cargo test -p ergo-aeron-cluster --lib
```

**Never** run `cargo … --workspace --all-features` without
`--exclude ergo-aeron-cluster` — the `test-harness` feature pulls Java/Gradle
test-support.

Cluster Java integration (optional, slow):

```sh
just build-aeron-jars
just test-aeron-cluster-harness
```

Docs compile (main crates):

```sh
cargo doc -p ergo-sbe --no-deps
cargo doc -p ergo-clickhouse-persist --no-deps
cargo doc -p ergo-aeron-cluster --no-deps
```

## Generator / wire changes

When adding generator behavior, include tests that cover:

- schema input
- normalized IR / resolve
- generated Rust surface (golden / compile-fail where relevant)

For wire-shape or hot-path changes, also cover:

- byte-exact fixture parity
- optional/null semantics
- versioned field absence
- configured `headerType` / `dimensionType`
- external framing
- zero allocations on generated decode/encode hot paths
- maintained benchmark ratios when the scenario is already in the ledger

## Git hygiene

- Stage paths explicitly; **never** `git add -A` (dirty
  `simple-binary-encoding` submodule and `aeron-cluster-[0-9]/` runtime dirs).
- Do not rename pillar directories (`sbe`, `persist`, `cluster`, `samples`).
- Commit messages: one sentence, conventional prefix (`feat:`, `fix:`, `docs:`, …).

## Publish

See [`docs/PUBLISH.md`](docs/PUBLISH.md) for crates.io order and the pre-publish
checklist (`just check`, package lists, no `test-harness` on published cluster).

## Errors and `Result`

- **Library / public API:** crate-specific errors only (`ClusterError`,
  `SinkError`, SBE schema/codegen errors via `thiserror`). Prefer
  `ClusterResult<T>` (= `Result<T, ClusterError>`) at the cluster boundary.
- **Never** expose `Box<dyn std::error::Error>` (or `anyhow`) from a published
  library function. Application code converts into its own type with `?` /
  `From`.
- **Unit/integration tests:** `Result<(), Box<dyn std::error::Error>>` so bodies
  use `?` instead of `.unwrap()` / `.expect()` on fallible calls (keep
  `assert!` / `assert_eq!` for assertions).
- **`fn main()`** (binaries, examples): same `Box<dyn Error>` return (or app
  error). Prefer `?` over unwrap.
- **Channels:** app logic uses `&str` / `channel_uri`; rusteron FFI uses
  `CString` / `&CStr` (`channel_cstr`). Converting `&str` → C string is not
  free (NUL terminator + no interior NUL).
- `build.rs` may stay panic-oriented (Cargo convention).
- `#[should_panic]` tests must return `()` (Rust forbids `Result` there).
- `proptest!` strategy-parameter tests may stay as `()` when `Result` confuses
  the macro.

## Module docs

Each shippable crate has a short README (template: purpose, status, build/test,
layout, non-goals). Prefer pointing at `sbe/design/DECISIONS.md` and living
plans under `docs/superpowers/` rather than duplicating design history.

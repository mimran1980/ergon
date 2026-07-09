# CI, docs, and release automation

**Blocked by:** none (infrastructure, independent of code)

Mirror the rusteron project's automation setup. ErgoSBE is simpler (no Java, no
Docker, no Gradle, no C deps), so the setup is leaner.

Ref: `https://github.com/mimran1980/rusteron_gsr/tree/v2-redesign`
**Status: ACTIVE / RELEASE INFRA**

**Decision after deferred recheck (2026-07-08):** unpark the CI and release
gates needed for a publishable crate. Bors/release automation can remain later,
but fmt, clippy, tests, docs, build matrix, MSRV, and local `just` parity are
release infrastructure, not post-v1 work.


## rust-toolchain.toml

Pin the Rust version so every developer and CI uses the same compiler:

```toml
[toolchain]
channel = "1.88.0"
components = ["rustfmt", "clippy", "rust-analyzer"]
profile = "minimal"
```

- [x] `rust-toolchain.toml` at workspace root
- [ ] Channel = current stable (1.88 at time of writing)
- [x] `rustfmt` + `clippy` + `rust-analyzer` components

## CI workflow (`.github/workflows/ci.yml`)

Three jobs, ubuntu + macos matrix, `[skip ci]` support:

### Job: Lint
- [x] `cargo fmt --all -- --check`
- [x] `cargo clippy --workspace --all-targets -- -D warnings`
- [x] `cargo doc --no-deps --workspace` (verify docs compile)

### Job: Test (matrix)
- [ ] Matrix: `os: [ubuntu-latest, macos-latest]`
- [x] `cargo test --workspace --all-targets`
- [x] `cargo test --doc`
- [ ] `fail-fast: false`

### Job: Build (release)
- [x] `cargo build --release --workspace`
- [ ] Upload release artifacts (1-day retention)

### Job: Bench (non-blocking, informational)
- [x] `cargo bench --workspace` with `continue-on-error: true`

### CI-wide
- [x] Trigger: `push`, `pull_request`, `workflow_dispatch`
- [x] `[skip ci]` in commit message skips all jobs
- [x] Concurrency group by ref, cancel-in-progress
- [x] `RUST_BACKTRACE: 1`, `CARGO_TERM_COLOR: always`

## Release workflow (`.github/workflows/release.yml`)

- [ ] Trigger: `workflow_dispatch` with `release_type` input (major/minor/patch, default: patch)
- [ ] Pin Rust toolchain
- [ ] Run full test suite before releasing
- [ ] `cargo install cargo-release` + `cargo release $type --workspace --execute --no-confirm`
- [ ] Push to main
- [ ] Create GitHub release with version tag
- [ ] Upload build artifacts to release
- [ ] Permissions: `contents: write`

## Documentation

- [ ] `docs.rs` auto-publishes on crates.io push (no config needed — just `#![doc]` attributes)
- [ ] Module-level docs in `lib.rs` with quick-start example
- [ ] `#![warn(missing_docs)]` on the generator crate
- [ ] `cargo doc --open` produces a useful, navigable site
- [ ] CONTRIBUTING.md already exists — verify it's up to date

## justfile

A `just` command runner with common tasks (like rusteron's but Rust-only):

- [x] `just check` — `cargo check --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
- [x] `just fix` — `cargo fmt --all && cargo clippy --workspace --fix --allow-dirty`
- [x] `just test` — `cargo test --workspace --all-targets -- --nocapture && cargo test --doc`
- [x] `just bench` — `cargo bench --workspace` (once benchmarks exist)
- [x] `just bench-hft` — `cargo bench --workspace -- decode/hft/` (HFT-specific benchmarks)
- [x] `just ci` — runs `check test docs bench` locally (mirrors CI pipeline)
- [x] `just docs` — `cargo doc --no-deps --workspace --open`
- [x] `just clean` — `cargo clean`
- [x] `just deps` — check for unused deps, duplicates

## bors.toml

- [ ] `bors.toml` with CI status checks
- [ ] `delete_merged_branches = true`

## Acceptance criteria

- [x] `rust-toolchain.toml` sets compiler version
- [ ] CI runs on push/PR: lint passes, test matrix passes (ubuntu + macos), release build passes
- [x] `[skip ci]` in commit message skips CI
- [ ] Release workflow accepts major/minor/patch, runs tests, publishes to crates.io, creates GitHub release
- [x] `just check` / `just test` / `just fix` work locally
- [ ] `cargo doc` produces docs with no warnings
- [ ] docs.rs shows API docs after first crates.io publish


## Verification / Unit Testing
- [ ] Verify that the build pipeline triggers and completes successfully on pull requests and branch merges.
- [ ] MSRV CI Gate: Add a job in the CI workflow that installs the minimum supported Rust version (e.g. 1.81.0) and validates that the library, generated tests, and baseline modules compile without errors.

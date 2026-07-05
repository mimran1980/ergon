# CI, docs, and release automation

**Blocked by:** none (infrastructure, independent of code)

Mirror the rusteron project's automation setup. ErgoSBE is simpler (no Java, no
Docker, no Gradle, no C deps), so the setup is leaner.

Ref: `https://github.com/mimran1980/rusteron_gsr/tree/v2-redesign`

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
- [ ] `rustfmt` + `clippy` + `rust-analyzer` components

## CI workflow (`.github/workflows/ci.yml`)

Three jobs, ubuntu + macos matrix, `[skip ci]` support:

### Job: Lint
- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo doc --no-deps --workspace` (verify docs compile)

### Job: Test (matrix)
- [ ] Matrix: `os: [ubuntu-latest, macos-latest]`
- [ ] `cargo test --workspace --all-targets`
- [ ] `cargo test --doc`
- [ ] `fail-fast: false`

### Job: Build (release)
- [ ] `cargo build --release --workspace`
- [ ] Upload release artifacts (1-day retention)

### CI-wide
- [ ] Trigger: `push`, `pull_request`, `workflow_dispatch`
- [ ] `[skip ci]` in commit message skips all jobs
- [ ] Concurrency group by ref, cancel-in-progress
- [ ] `RUST_BACKTRACE: 1`, `CARGO_TERM_COLOR: always`

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

- [ ] `just check` — `cargo check --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `just fix` — `cargo fmt --all && cargo clippy --workspace --fix --allow-dirty`
- [ ] `just test` — `cargo test --workspace --all-targets -- --nocapture && cargo test --doc`
- [ ] `just bench` — `cargo bench --workspace` (once benchmarks exist)
- [ ] `just docs` — `cargo doc --no-deps --workspace --open`
- [ ] `just clean` — `cargo clean`
- [ ] `just deps` — check for unused deps, duplicates

## bors.toml

- [ ] `bors.toml` with CI status checks
- [ ] `delete_merged_branches = true`

## Acceptance criteria

- [ ] `rust-toolchain.toml` sets compiler version
- [ ] CI runs on push/PR: lint passes, test matrix passes (ubuntu + macos), release build passes
- [ ] `[skip ci]` in commit message skips CI
- [ ] Release workflow accepts major/minor/patch, runs tests, publishes to crates.io, creates GitHub release
- [ ] `just check` / `just test` / `just fix` work locally
- [ ] `cargo doc` produces docs with no warnings
- [ ] docs.rs shows API docs after first crates.io publish

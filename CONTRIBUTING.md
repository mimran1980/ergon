# Contributing

This is an experimental hobby repository. Contributions must preserve official
SBE wire compatibility and must not turn prototype behaviour into a production
claim. The current release-readiness design and backlog are in
[`.scratch/release-readiness/spec.md`](.scratch/release-readiness/spec.md).

## Work test-first

For generator or protocol behaviour:

1. Add a behavioural or compile-fail test that demonstrates the missing contract.
2. Run it and confirm it fails for the expected reason.
3. Implement the smallest coherent change.
4. Run the focused test, then the affected crate's complete tests and Clippy.
5. Run equal-work allocation and benchmark checks for a hot-path change.

Generated-source substring assertions may supplement a behavioural test, but do
not prove type-state safety, error propagation, allocation behaviour, or wire
correctness on their own.

Wire-shape changes must cover byte-exact reference parity, optional/null values,
acting-version behaviour, configured headers and group dimensions, nested tails,
and malformed input. Generated interface restrictions belong in compile-pass and
compile-fail tests.

## Local checks

Run the repository gate when possible:

```sh
just check
```

Product-crate checks:

```sh
cargo fmt --all -- --check
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
cargo test -p ergo-sbe --all-features -- --test-threads=1
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
cargo test -p ergo-aeron-cluster --all-targets
```

Laboratory compatibility checks:

```sh
cargo test -p ergo-clickhouse-persist --all-features
(cd samples/exchange-example && cargo check --all-targets)
(cd samples/cluster-ha-orderbook && cargo check --all-targets)
```

Some all-feature Cluster tests require Java 17+ and locally built Aeron jars.
Persist and sample integration tests may require ClickHouse, Docker, or a running
Cluster. Do not convert a missing external dependency into a product success
claim.

## Performance rules

For a generated hot-path change, run:

```sh
just bench
cargo bench -p ergo-aeron-cluster
```

Comparisons must perform byte-identical work and apply equivalent validation.
Run three sessions before accepting a material result. A maintained runtime
median regression above 3%, or generator-time regression above 5%, remains open
work. Borrowed decoding, fixed-width conversion, fixed encoding, and Cluster
offering must remain allocation-free where the implementation plan requires it.

Docs-only changes do not require benchmarks.

## Errors and examples

- Published interfaces use crate-specific error types and `Result`.
- Do not expose `Box<dyn std::error::Error>` or `anyhow` from a library interface.
- Tests and binaries may return `Result<(), Box<dyn std::error::Error>>` to use
  `?` cleanly.
- Avoid `unwrap` and `expect` where `?` can preserve useful context.
- Cluster examples use typed `offer`; `offer_raw` is an internal low-level tool,
  not an example interface.
- Dynamic C strings are created privately at the FFI seam. Do not add shallow
  public CString formatting helpers.

## Documentation rules

- Track specs and implementation issues under `.scratch/<feature-slug>/` using
  the conventions in `docs/agents/issue-tracker.md`.
- Do not create standalone per-task todo Markdown files, goal files, completion
  reports, or archived plan trees.
- Update the relevant crate README only for current, verified behaviour. Keep
  unfinished design in the relevant `.scratch/<feature-slug>/spec.md`.
- Persist and Samples must always be labelled unpublished laboratories and never
  described as reference implementations.
- Ergo Aeron Cluster must always be labelled a hobby project unsuitable for
  production.

## Package checks

Only ergo-sbe and ergo-aeron-cluster are publication candidates:

```sh
cargo package -p ergo-sbe --list --allow-dirty
cargo package -p ergo-aeron-cluster --list --allow-dirty
cargo publish -p ergo-sbe --dry-run --allow-dirty
cargo publish -p ergo-aeron-cluster --dry-run --allow-dirty
```

Inspect the package lists. They must not include historical plans, test fixture
inventories, the Java harness, application protocols, RFQ/auction material, or
reference codecs.

## Git hygiene

- Preserve unrelated changes and dirty submodules.
- Stage paths explicitly; do not use `git add -A` in this repository.
- Do not rename the `sbe`, `cluster`, `persist`, or `samples` directories.
- Use a one-line conventional commit subject such as `docs:`, `fix:`, or `feat:`.

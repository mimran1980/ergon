# ergon

> **Experimental hobby repository. Do not use any crate in this repository in
> production.** Interfaces are deliberately opinionated, incomplete, and subject
> to breaking changes while the design is explored.

This repository has two prototype projects and two internal laboratories.

| Directory | Status | Purpose |
|---|---|---|
| [`sbe/`](sbe/) | Primary prototype; intended for an eventual `0.x` crates.io release | Explore ergonomic, safe, and very fast Rust code generation for Simple Binary Encoding |
| [`cluster/`](cluster/) | Hobby prototype; intended for an eventual `0.x` crates.io release | Experiment with an Aeron Cluster client in Rust and exercise ergon against Aeron's protocol |
| [`persist/`](persist/) | Unpublished laboratory | Exercise ergon domain objects and ClickHouse-oriented mappings |
| [`samples/`](samples/) | Unpublished playground | Try APIs end to end; low-quality code that must not be treated as reference material |

ergo-sbe is the main focus and the most tested part of the repository. Its purpose
is fast prototyping: find generated Rust interfaces that are pleasant to use,
hard to misuse, and at least as fast as the maintained Aeron SBE comparison. In
the best case, useful ideas can later be adapted for the official Java SBE Rust
generator so the wider SBE community benefits.

Ergo Aeron Cluster is intentionally a hobby experiment. The preferable long-term
solution is for Aeron to provide official Cluster C client bindings and for
rusteron to expose them. This crate is not a substitute for that work.

## Current state

The repository is preparing for its first prototype release; it is not release
ready today. The corrected design and all verified-open work live in the single
[`release-readiness spec`](.scratch/release-readiness/spec.md). Checked historical
todo files were removed because several described only partial implementations.

Important known gaps include:

- the generic converter registry is only partially emitted;
- latest-version fixed-field encoding does not yet prove all required fields;
- generated domain mappings are incomplete and can hide malformed tails;
- schema-declared text variable data is not consistently exposed as fallible
  zero-copy strings;
- the Cluster client still exposes internal protocol surface and has incomplete
  error/reconnect handling;
- samples and package contents still need release cleanup.

Do not infer completion from generated method names or passing baseline tests.

## Repository map

- [ergo-sbe README](sbe/README.md)
- [Ergo Aeron Cluster README](cluster/README.md)
- [Persist laboratory README](persist/README.md)
- [Samples laboratory README](samples/README.md)
- [Implementation plan and design](.scratch/release-readiness/spec.md)
- [Contributing and verification](CONTRIBUTING.md)
- [Changelog](CHANGELOG.md)

The repository also vendors two upstream submodules:

- `simple-binary-encoding/`: official SBE reference implementation and tooling.
- `aeron/`: Aeron source, schemas, and Java test infrastructure.

Initialise them with:

```sh
git submodule update --init --recursive
```

## Development checks

The convenient local entry point is:

```sh
just check
```

The product-crate checks can also be run directly:

```sh
cargo fmt --all -- --check
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
cargo test -p ergo-sbe --all-features -- --test-threads=1
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
cargo test -p ergo-aeron-cluster --lib
```

Some Cluster integration tests need Java 17+, built Aeron jars, and a local
Aeron environment. Persist and sample live tests can require ClickHouse, Docker,
network services, or a multi-node Java Cluster. See the laboratory READMEs before
running those checks.

## Release posture

Only these crates are candidates for prototype publication:

1. `ergo-sbe`
2. `ergo-aeron-cluster`

Persist, its derive crate, benchmarks, and samples are not publication targets.
No publication should happen until every release item and acceptance command in
the implementation plan passes and the package file lists have been inspected.

## Design priorities

1. Preserve official SBE wire compatibility.
2. Do not regress maintained hot-path performance or allocation behaviour.
3. Make the Rust interface easier and safer when the improvement is zero-cost on
   the hot path or deliberately outside it.
4. Prefer a small, coherent generated interface over many shallow convenience
   methods.
5. Surface malformed protocol data as errors; do not silently substitute default,
   empty, or lossy values.

## License

Apache-2.0.

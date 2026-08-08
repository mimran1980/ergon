# Cluster Client

`ergo-aeron-cluster` is an experimental Rust client for Aeron Cluster. It uses
rusteron for transport and ergo-sbe-generated codecs for Aeron's Cluster
protocol.

> Hobby project. Do not use it as a production substitute for official Aeron
> Cluster client support.

## Scope

The crate implements client-side operations:

- synchronous and poll-driven connection;
- ingress `offer` and explicit `try_claim`;
- regular and controlled egress polling;
- authentication challenges and credentials;
- keep-alives and close requests;
- session events, administrative responses, and leader changes.

The Java Aeron process still provides the media driver, archive, consensus
module, clustered services, election, recovery, and operational tooling. This
crate implements none of those server-side components.

## Features and harness

Default features use the Rust client library only. The `test-harness` feature
adds repository-only Java Cluster launch support and requires Java 17 or newer
plus locally built Aeron artifacts:

```sh
just build-aeron-jars
just test-aeron-cluster-harness
```

The harness, examples, integration tests, reference codecs, and benchmarks are
excluded from the published crate package.

## Verify the crate

```sh
cargo test -p ergo-aeron-cluster --lib
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
cargo bench -p ergo-aeron-cluster --no-run
cargo package -p ergo-aeron-cluster --list --allow-dirty
```

Run `just bench-cluster` for the maintained codec comparisons. See
[sbe/BENCHMARKS.md](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md)
for the common benchmark rules.

## Limitations

The current limitations are documented in the [Compatibility](./compatibility.md)
page, which also lists supported Aeron/rusteron versions, schema identities,
failure modes, and the CI multi-node test matrix.
- Java interoperability depends on the local Aeron harness and environment.

Apache-2.0.

# Contributing

ergon is experimental, but changes must still be reproducible, wire-compatible,
and honest about what has been verified.

## Work from behavior

For generator or protocol changes:

1. Add a focused behavioral, compile-fail, wire-parity, or allocation test.
2. Run it and confirm the expected failure.
3. Make the smallest coherent implementation change.
4. Run the focused test and the affected crate's complete checks.
5. Run the maintained benchmark gate when a generated or Cluster hot path
   changes.

Generated-source substring tests can supplement behavior tests, but do not prove
wire correctness, type-state ordering, error propagation, or allocation
behavior by themselves.

## Product checks

```sh
just check-products
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-sbe --all-features --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
```

`just check` adds the repository hygiene and established sample checks. Some
Cluster integration paths require Java 17+, built Aeron artifacts, and a local
Aeron environment:

```sh
just build-aeron-jars
just test-aeron-cluster-harness
```

Do not report a skipped external integration as a passing test.

## Performance

Run the SBE parity gate after any change that can affect generated hot paths:

```sh
just bench
```

Run the Cluster gate after session codec, offer, claim, or egress hot-path
changes:

```sh
just bench-cluster
```

Every maintained ergo-sbe/reference ratio must be at most `1.00` under
equal-work inputs. Record fresh measurements instead of copying old benchmark
numbers into documentation. Documentation-only changes do not require a
benchmark run.

## Error and example style

- Public library APIs use crate-specific typed errors.
- Tests and binaries may return `Result<(), Box<dyn std::error::Error>>`.
- Prefer `?` to avoidable `unwrap` or `expect`.
- Treat schema-declared text strictly; keep binary fields as bytes.
- Size generated-message buffers from generated constants or length helpers.
- Use the high-level Cluster facade in consumer examples.

## Documentation

- Keep the root README focused on repository orientation.
- Put crate usage in that crate's README.
- Keep one sample inventory in `samples/README.md`; do not add per-sample
  READMEs.
- Describe current, verified behavior. Do not commit dated implementation
  plans, completion ledgers, or archived task trees.
- Use Git history as the archive for superseded design material.
- Keep benchmark results out of permanent docs unless a release record needs an
  immutable, reproducible snapshot.

## Package boundaries

Only `ergo-sbe` and `ergo-aeron-cluster` are publication candidates. Inspect
their payloads before any release:

```sh
cargo package -p ergo-sbe --list --allow-dirty
cargo package -p ergo-aeron-cluster --list --allow-dirty
```

Packages must exclude repository tests, fixtures not required at build time,
benchmarks, Java harness code, application protocols, samples, and internal
plans.

Publishing, tagging, and announcing a release require explicit maintainer
authorization.

## Git hygiene

- Preserve unrelated working-tree changes and dirty submodules.
- Stage paths explicitly; do not use `git add -A`.
- Do not rename the `sbe`, `cluster`, or `samples` directories.
- Use a short one-line conventional commit subject.

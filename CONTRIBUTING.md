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
just policy
just check-products
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-sbe --all-features --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
```

`test-lanes.tsv` assigns every tracked test-bearing Rust source to exactly one
executable lane. `just policy` self-tests that enforcement and rejects ignored
tests, ignored Rust fences, runtime `SKIP` reporting, test-selection bypasses,
conditional test execution, failure-to-success wrappers, and custom skip-CI
conditions.

A failure observed while changing the repository is not a pass because it
appears pre-existing or unrelated. Reproduce and fix it, or stop the change
with the failure recorded as a blocker. Never make the lane green by filtering,
ignoring, conditionally bypassing, or merely logging the failed case.

`just check` adds repository hygiene and established sample checks. The
complete required suite is:

```sh
just test
```

It builds the Aeron Java artifacts and runs the Java lifecycle/recovery and HA
sample lanes. A missing dependency is a failure, not a passing partial run.
Use `just test-all` to add Miri and deterministic fuzz replay.

Quality ratchets are explicit commands, not test-count targets:

```sh
just check-coverage
just check-mutation
```

Coverage runs on every pull request and may not fall below the checked-in
region/function/line baseline. Mutation testing runs weekly over parser,
resolver, sizing, and dynamic-tail code; missing or empty mutation output is a
failure. Nightly CI runs every fuzz target for ten minutes and executes the
LE/BE/nested fixture crate under Miri. Pull requests also execute codec library
tests on 32-bit x86 and big-endian s390x through `cross`/QEMU.

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

Shared GitHub runners execute both profiles and publish Criterion diagnostics,
but noisy wall-clock ratios are not merge gates there. Run the strict ratio
gate locally or on a dedicated stable benchmark runner.

`fairness_policy_test` mechanically requires the maintained SBE and Cluster
parity suites to use `std::hint::black_box`, assert correctness before timing,
and preserve the sceptical benchmark disclosure. Exact wire/value assertions
remain part of each benchmark setup; a large result is presumed suspect until
the benchmark is re-audited.

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

Packages must exclude repository integration tests, fixtures not required at
build time, benchmarks, Java harness code, application protocols, samples, and
internal plans.

Publishing, tagging, and announcing a release require explicit maintainer
authorization.

Before an authorised release, run `just release-check`. The release workflow
runs the same command before `cargo release`; it cannot substitute a partial
workspace-only test command.

## Git hygiene

- Preserve unrelated working-tree changes and dirty submodules.
- Stage paths explicitly; do not use `git add -A`.
- Do not rename the `sbe`, `cluster`, or `samples` directories.
- Use a short one-line conventional commit subject.

# Contributing

Changes must be reproducible, wire-compatible and honest about what was
verified.

## Work from behaviour

For a generator or protocol change:

1. Add a focused behavioural, compile-fail, wire-parity or allocation test.
2. Run it and see it fail.
3. Make the smallest coherent change.
4. Run the focused test and the affected crate's full checks.
5. Run the benchmark gate when a generated or cluster hot path changes.

A substring test on generated source can supplement a behaviour test. It does
not prove wire correctness, type-state order, error propagation or allocation
behaviour.

## Checks

| Command | What it runs |
|---|---|
| `just policy` | test-lane ownership and the suppression rules below, plus their self-tests |
| `just preflight` | fmt, clippy, policy self-tests, book fences, golden and reference `--check` |
| `just check-products` | `ergo-sbe` and `ergo-aeron-cluster` only |
| `just check-local` | repository hygiene, fmt, clippy, the tests that need no Java, and the `exchange-example` sample |
| `just test` | the full suite: builds the Aeron Java jars and runs the Java cluster lifecycle, recovery and HA lanes. A missing dependency fails |
| `just test-all` | `just test` plus Miri and fuzz replay |
| `just check-coverage` | coverage against the checked-in baseline |
| `just check-mutation` | mutation testing over parser, resolver, sizing and dynamic-tail code |

```sh
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-sbe --all-features --no-deps
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-aeron-cluster --no-deps
```

### Test policy

`test-lanes.tsv` assigns every tracked test-bearing Rust file to exactly one
lane. `scripts/check-test-policy.sh` rejects:

| Pattern | Where it looks |
|---|---|
| `#[ignore]` | `.rs` files |
| an `ignore` doc fence | `.rs` doc comments (Markdown `rust,ignore` is allowed) |
| a test printing `SKIP` or `Skipped:` | `.rs` files |
| `--skip`, `--ignored`, `--include-ignored` | `justfile`, `just/*.just`, `scripts/*.sh`, workflows |
| `if … cargo test/bench` | the same control files |
| `cargo test … \|\| true`, `; exit 0` | the same control files |
| `[skip ci]`-style markers, `continue-on-error: true`, any `if:` other than `always()` | workflows |

The policy cannot see a test that returns `Ok` early without printing those
words. The RLValidator leg is not such a test. It is compiled only with
`--cfg ergon_rlvalidator` (`scripts/run-fix-sbe-conformance.sh`). With that
cfg, a missing suite, class directory or classpath fails the test. `just test`
does not pass the cfg, so the leg is absent from the default suite.

A failure seen while changing the repository is not a pass because it looks
pre-existing. Fix it, or stop and report it as a blocker. Never make a lane
green by filtering, ignoring, conditionally bypassing or only logging the
failure.

### CI

| Workflow | What it runs |
|---|---|
| `ci.yml` | policy; lint, test and DPDK jobs on `ubuntu-24.04` (x86_64) and `ubuntu-24.04-arm` (arm64); Rust 1.99.0; Java 25 for the Aeron jobs; the coverage ratchet; the fixture crate as ordinary tests; benches compiled with `--no-run`, none run |
| `cluster-compatibility.yml` | the cluster harness against Aeron 1.46.8 on Java 21, when `cluster/` or `sbe/src/` changes |
| `nightly.yml` | weekly (Sunday): every fuzz target for 600 s, and the fixture crate under Miri |
| `release.yml` | `just release-check`, then `just bench` and `just bench-cluster` on `ubuntu-latest`, blocking |

Mutation testing is manual; CI checks only its configuration, and the ratchet
fails when mutation output is missing or empty. 32-bit x86 and big-endian
s390x codec tests are not run in CI.

## Performance

```sh
just bench           # after a change that can affect generated hot paths
just bench-cluster   # after a cluster session codec, offer, claim or egress change
```

The target is ergon / sbe-tool ≤ `1.00` on equal work. Ceilings, the gate's
pass/inconclusive/fail rules and the fairness rules:
[`sbe/BENCHMARKS.md`](sbe/BENCHMARKS.md). Do not raise a ceiling. Do not copy
benchmark numbers into docs. A docs-only change needs no benchmark run.

`fairness_policy_test` requires the SBE and cluster parity suites to use
`std::hint::black_box`, assert correctness before timing, and keep the
benchmark disclosure. Treat a large result as suspect until the benchmark is
re-audited.

## Error and example style

- Public library APIs use crate-specific typed errors.
- Tests and binaries may return `Result<(), Box<dyn std::error::Error>>`.
- Prefer `?` to `unwrap` or `expect`.
- Treat schema-declared text strictly; keep binary fields as bytes.
- Size message buffers from generated constants or length helpers.
- Use the high-level cluster facade in consumer examples.

## Documentation

- The root README orients; crate usage goes in that crate's README.
- One sample inventory: `samples/README.md`. No per-sample READMEs.
- A page that describes how the software behaves describes current, verified
  behaviour. A superseded number is not the current result.
- Keep the record in the tree. The AI-assistance account, benchmark round
  write-ups, screenshots and release notes stay. Git history is not a
  substitute for that record. Dated plans and scratch task trees do not
  belong in the tree.
- One page owns a fact; other pages link to it.

## Package boundaries

The publication candidates are `ergo-sbe`, `ergo-aeron-cluster`, `ergon-tune`,
`ergon-io` and `ergon-md-dpdk` (`api/public-api-baseline.toml`). `ergon-io` and
`ergon-md-dpdk` depend on `ergon-tune`, so the release publishes it first.
`scripts/check-publish-set.sh` (in `just policy`) fails when a published crate
depends on an unpublished one, on a path without a version, or on an older
version than a published sibling's, or when the release recipe, workflow, dry
runs or public-API gate leave a crate out or publish it before its
dependencies. A version bump therefore updates every `version` beside a
workspace `path` with `[workspace.package]`: `ergo-sbe` in
`[workspace.dependencies]` and the `ergon-tune` lines in `md-dpdk/` and `io/`.

Inspect the payloads before a release:

```sh
cargo package -p ergo-sbe --list --allow-dirty
cargo package -p ergo-aeron-cluster --list --allow-dirty
cargo package -p ergon-tune --list --allow-dirty
cargo package -p ergon-io --list --allow-dirty
cargo package -p ergon-md-dpdk --list --allow-dirty
```

Packages exclude integration tests, fixtures not needed at build time,
benchmarks, Java harness code, application protocols, samples and internal
plans. `ergon-tune` and `ergon-io` ship their library sources and README only.
`ergon-md-dpdk` ships its native C, including the test fakes its build
compiles, but not its harness, device plugin or deployment assets.

Publishing, tagging and announcing a release need explicit maintainer
authorisation. Before an authorised release, run `just release-check`; the
release workflow runs the same command before `cargo release`.

## Git hygiene

- Preserve unrelated working-tree changes and dirty submodules.
- Stage paths explicitly; do not use `git add -A`.
- Do not rename the `sbe`, `cluster` or `samples` directories.
- Use a short one-line conventional commit subject.

# README, Samples, and 0.1.1 Release Design

## Goal

Make `sbe/README.md` a trustworthy, approachable map of ergo-sbe’s generated
API, prove its runnable examples in CI, keep the repository samples aligned
with the current API, release both product crates as 0.1.1, and leave `main`
with one post-0.1.0 commit.

## Documentation contract

Bare `rust` fences in `sbe/README.md` are runnable contracts. The
`readme_rust_fences_compile` integration test generates a small multi-template
codec containing fixed fields, arrays, a group, var-data, domain objects, and
dispatch support. Each bare fence is compiled in a temporary crate with that
generated module in scope.

Illustrative fragments that cannot be complete programs—such as abbreviated
generated type definitions—remain `rust,ignore` and are explicitly described
as schematic. User-facing workflows use bare `rust` fences.

The README feature path is:

1. generator configuration;
2. fixed-message encode/decode;
3. checked validation and dispatch;
4. fixed arrays;
5. ordered groups and var-data;
6. exact encoded length;
7. owned domain DTOs;
8. configuration and conversion selectors;
9. links to complete repository samples.

## Sample contract

Every service-free sample is compiled or tested by `just test`. The README
describes each sample’s actual dependency pattern. Build-only samples use
generated codecs without a runtime `ergo-sbe` dependency. Generator-as-library
samples depend on `ergo-sbe` at runtime. Aeron-backed examples are compiled
where local Java artifacts permit and otherwise remain explicitly gated.

The feature tour is the canonical generated-API teaching sample. `l3-book`
demonstrates nested/ragged books and concrete domain types.
`exchange-example` demonstrates generic conversions and multi-schema IPC.
`sbe-codegen-examples` demonstrates the generator library directly.

## Stability and release

The golden-output test parses both inputs, then canonicalizes both with the
same required `rustfmt` tool before comparison. Formatter-only rewrites do not
create false failures, while generated API/code changes still fail the test.

The workspace package version becomes 0.1.1. Internal path dependencies that
also declare a crates.io version become 0.1.1, and every tracked lockfile is
regenerated through Cargo.

Before history rewriting:

- all 37 sbe-tool Rust reference crates regenerate from the pinned submodule;
- dual parity and malformed-input tests pass;
- the SBE and Cluster benchmark gates pass;
- `just build`, `just fix`, and `just test` pass from the final tree.

Finally, commits after `b462e8e` (`initial release 0.1.0`) plus the final
working-tree changes are squashed into one 0.1.1 commit. The two older commits
remain unchanged. Nothing is pushed.

# ErgoSBE Pre-Release Reset Implementation Plan

> **For agentic workers:** Use `superpowers:subagent-driven-development` or
> `superpowers:executing-plans`, track these checkboxes, work test-first for
> behavior changes, and verify before marking tasks complete.

**Goal:** Prepare ErgoSBE and Ergo Aeron Cluster as honest, packageable
prototype crates while reducing APIs, examples, documentation, and internal
project clutter.

**Architecture:** ErgoSBE is the primary opinionated SBE API-research project.
Ergo Aeron Cluster is a high-level hobby client using private
ErgoSBE-generated protocol codecs. Persist and Samples remain unpublished
API-testing labs and are excluded from release gates.

**Baseline:** Review range `bd3f7ce...946ae3a`. On 2026-07-20,
`cargo test -p ergo-sbe --all-features -- --test-threads=1` and
`cargo test -p ergo-aeron-cluster --lib` passed. Preserve the pre-existing
dirty `simple-binary-encoding` submodule.

## Public API decisions

### ErgoSBE

- `Generator::generate` and `Generator::generate_multi` return
  `Result<GeneratedModuleSet, GenerateError>`; remove `try_generate`.
- Make `GenerationConfig` fields private. Retain builders for domain objects,
  shared modules, external runtime, error conversions, decimal converters,
  and benchmark-only unchecked companions.
- Remove `CompatibilityMode`, `checked_accessors`, `SchemaSource`,
  `Schema::new`, `Generator::config`, and public `GeneratedModuleSet::push`.
- Generated domain mapping uses `TryFrom<Decoder, Error = DecodeError>` and
  never drops malformed groups or var-data.

### Ergo Aeron Cluster

- Export only the high-level client/session/claim API, egress listeners,
  credentials, states, errors, publication failures, and required event enums.
- Keep generated codecs, URI construction, endpoint parsing, transport,
  polling, and connection machinery private.
- Listener callbacks return `ClusterResult`; panics at Aeron callbacks become
  `ClusterError::ListenerPanicked`.
- Remove public C-string helpers. Use `c"..."` for static private strings and
  private `cformat!` conversion for validated dynamic strings.

## Tasks

- [ ] **1. ErgoSBE fallible API:** add failing tests; implement fallible
  generation, private builder configuration, fallible domain mapping, real
  issue-schema generation, and correct `sinceVersion = 0` emission. Localize
  lint exemptions and prefer `?` at fallible boundaries.
- [ ] **2. Intentional SBE examples:** replace debug generators with one owned
  domain-object example and one explicitly zero-copy flyweight example backed
  by a single regeneration-checked generated fixture. Enable domain objects in
  Persist and test owned round trips without making Persist a reference app.
- [ ] **3. Cluster egress hardening:** test and implement filtering for every
  session-bearing event, callback error/panic containment, surfaced decode
  failures, fallible keep-alive, and atomic retryable leader transitions that
  construct a publication plus two assemblers before swapping state.
- [ ] **4. High-level cluster surface:** delete shallow URI/idle/decode/session
  wrappers and RFQ/Mark/auction application protocols; retain three distinct
  high-level examples using `offer`, `try_claim`, `ClusterResult`, and `?`.
  Move the Java harness into integration-test support.
- [ ] **5. Fragmentation and performance proofs:** send deterministic 16 KiB
  payloads with MTU 1408; cover regular, controlled, foreign-session,
  callback-error, and leader/image behavior. Hide benchmark internals, retain
  only the six required sbe-tool reference codecs, and restore all maintained
  benchmark comparisons with a 0.5% tolerance and missing-estimate failures.
- [ ] **6. Packaging:** make both prototype crates self-contained, vendor the
  pinned Aeron session schema under `cluster/schemas`, add a version to the
  ErgoSBE path dependency, use crate-local READMEs and package allowlists, and
  set Persist plus Persist Derive to `publish = false`.
- [ ] **7. Documentation:** keep concise root/product/lab READMEs,
  `sbe/DESIGN.md`, `sbe/GUIDE.md`, `docs/ROADMAP.md`, and
  `docs/PUBLISHING.md`. Delete historical TODO ledgers, goals, plans/specs,
  legacy SBE benches, per-sample READMEs, `package-lock.json`, `bors.toml`, and
  `ci-monitor.sh` after migrating still-relevant facts.
- [ ] **8. Gates and release:** make `just check`/`check-products` product-only,
  add a separate `check-labs`, and make `release-check` verify only the two
  prototype crates. Fix stale CI package/sample names and publish ErgoSBE
  before Cluster; never publish the workspace wholesale.
- [ ] **9. Final review:** run formatting, strict Clippy, all product/lab tests,
  Java harness, maintained benchmarks, package checks, dry-run ErgoSBE publish,
  and a fresh two-axis review against `bd3f7ce...HEAD`.

## Documentation posture

- Root: two publishable prototypes and two unpublished labs.
- ErgoSBE: experimental, opinionated API research and the most tested project
  here, without claiming production readiness.
- Cluster: “Hobby experiment — do not use in production.” The preferred
  long-term solution is official Aeron C cluster bindings plus rusteron.
- Persist and Samples: internal, low-quality test beds; unpublished and not
  reference implementations.

## Final verification

```bash
git diff --check
just check-products
just check-labs
just test-aeron-cluster-harness
just bench
just bench-cluster
just release-check
cargo publish -p ergo-sbe --dry-run --allow-dirty
```

Inspect both package file lists. They must contain no TODO archives, historical
plans, Persist/Samples code, Java harness, or external-path assets. Re-run the
formal review and close Standards and Specification findings separately.

After every checkbox is complete, move genuinely unfinished work into
`docs/ROADMAP.md`, update `CHANGELOG.md`, and delete this active plan so it does
not become another historical ledger.


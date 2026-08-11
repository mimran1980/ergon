# Review tickets

Hand-off for a fresh LLM working from `feat/0.1.17` at `c3e8aab0`.
These are implementation tickets, ordered by leverage and risk rather than by
source-tree location.

For every change under `sbe/`, preserve official SBE wire compatibility,
regenerate and check affected goldens, and apply the benchmark policy in
`CLAUDE.md:63-105`. Generated hot-path changes must stay at or below the
maintained ergon/sbe-tool `1.00` ratio in both LTO profiles. Cluster hot-path
changes use `just bench-cluster` (`CLAUDE.md:107-108`).

Audit guardrail: there is intentionally no standalone PERF ticket because no
measured regression with a defensible mechanism was verified. In particular,
generated enum `raw`/`from_raw` methods are deliberately not inlined after a
measured no-LTO regression (`sbe/src/codegen/runtime.rs:1039-1047`). Do not
reopen that choice without new measurements; the grounded `#[must_use]` gap is
T-20. Hot-path changes below carry mechanism-specific evidence plans.

# 1. Quick wins — 0.1.17

These are S-effort, high-value changes that do not require redesigning the
steady-state encode/decode hot path.

| ID | Ticket | Type | Priority |
|---|---|---|---|
| T-1 | Gate fixed-stride group proof APIs | API | P0 |
| T-2 | Reject malformed layout numerics | CORRECTNESS | P0 |
| T-3 | Validate the complete multi-schema output plan | CORRECTNESS | P1 |
| T-4 | Restore the promised non-exhaustive cluster error contract | API | P1 |
| T-5 | Redact credentials from diagnostics | CORRECTNESS | P1 |
| T-6 | Reject duplicate ingress member IDs | CORRECTNESS | P1 |
| T-7 | Validate timeouts before narrowing to milliseconds | CORRECTNESS | P1 |
| T-8 | Preserve the distinction between malformed XML and invalid SBE | CORRECTNESS | P1 |
| T-9 | Reconcile the published release and performance status | DOCS | P1 |
| T-10 | Put schema field descriptions on encoder APIs | DOCS | P2 |
| T-11 | Preserve UTF-8 decode causes in the error chain | API | P2 |

## T-1: Gate fixed-stride group proof APIs

- Type: API
- Stage: 0.1.17
- Priority: P0 · Effort: S
- Symptom: `sbe/src/codegen/group_encoder.rs:205-242` emits
  `add_checked` and `start_entry` for every repeating group, while
  `sbe/src/codegen/group_encoder.rs:458-463` emits `complete`
  unconditionally. Dynamic entries can still have a var-data tail
  (`sbe/src/codegen/group_encoder.rs:680-725`), so a fixed-block-only slice
  cannot prove entry completion. The generated FuelFigures API nevertheless
  exposes that false proof (`sbe/tests/golden/car_example.rs:7055-7124`)
  and permits `complete` before `usage_description`
  (`sbe/tests/golden/car_example.rs:7158-7230`).
- Change: Emit `add_checked`, `start_entry`, `complete`, and the
  `EntryComplete` proof only when `g.has_fixed_stride()` is true. Dynamic
  groups retain the existing flexible `add` API until T-19 supplies real
  entry typestate. SBE-pattern rationale: a checked fixed-stride group may
  prove completion from its block length; an entry with ordered tails cannot.
- What breaks (API only), what it buys: Calls to the currently unsound proof
  methods on dynamic groups stop compiling. Users no longer get an API that
  claims a partial dynamic entry is complete or silently emits a malformed
  entry.
- Acceptance criteria: Add compile-pass coverage for fixed-stride groups and
  compile-fail coverage for `start_entry`/`complete` on a group with
  var-data or nested-group tails. Update
  `sbe/tests/group_proof_state_test.rs`,
  `sbe/tests/soundness_hostile_constructors_test.rs`, and
  `sbe/tests/golden/car_example.rs`.
- Verification plan: Run `cargo test -p ergo-sbe`, `just check-golden`,
  `just bench-groups`, and the mandatory `just bench` LTO/no-LTO gate.
  The mechanism is API omission only; generated fixed-stride instructions and
  all maintained benchmark ratios must remain unchanged.

## T-2: Reject malformed layout numerics

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P0 · Effort: S
- Symptom: A present but invalid field `offset` is silently treated as absent
  at `sbe/src/xml/message.rs:243-247`; an invalid group `blockLength`
  likewise becomes `None` at `sbe/src/xml/message.rs:269-276`. Ordinary
  `parse` is a documented entry point independent of XSD validation
  (`sbe/src/xml/mod.rs:3-8`), so these declarations can change the computed
  wire layout instead of failing. Fallible, span-aware numeric helpers already
  exist at `sbe/src/xml/attr.rs:172-199`.
- Change: Parse each present layout attribute exactly once through the
  fallible helpers and return a span-bearing `Fault` for malformed, negative,
  or overflowing values. Apply the same rule to any duplicate pre-validation
  parse of field IDs at `sbe/src/xml/message.rs:89-112`.
  SBE-pattern rationale: offsets and block lengths are positional wire declarations, not
  optional hints once supplied.
- What breaks (API only), what it buys: No Rust API break; schemas that were
  accidentally accepted with ignored garbage now fail early. Valid schemas
  cannot acquire an unintended packed layout.
- Acceptance criteria: Add cases to `sbe/src/xml/tests.rs` for malformed,
  negative, and overflowing `offset` and `blockLength`, plus valid
  explicit-padding controls. Assert the diagnostic names the attribute and
  source span. Update `sbe/tests/golden/car_example.rs` only if a valid fixture
  exposes an existing parser bug.
- Verification plan: Run `cargo test -p ergo-sbe`, `just check-golden`,
  and `just bench`. Compare generated output for the valid-schema corpus
  byte-for-byte; benchmark ratios must be identical because the change is
  confined to cold schema parsing.

## T-3: Validate the complete multi-schema output plan

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P1 · Effort: S
- Symptom: `sbe/src/codegen/mod.rs:459-490` validates the configured shared
  module name, but `sbe/src/codegen/mod.rs:576-590` accepts arbitrary
  per-schema module names and `sbe/src/codegen/mod.rs:621-659` uses them to
  construct output paths and imports. The documented first-entry ownership
  rule is at `book/src/sbe/getting-started/multi-schema.md:24-36`, and the
  generated files
  are commonly joined directly to `OUT_DIR`
  (`book/src/sbe/getting-started/multi-schema.md:89-99`).
- Change: Before generating any file, require every per-schema module name to
  be one non-empty Rust identifier, require names to be unique, validate the
  shared module name under the same rule, and require it to identify the
  declared first-entry owner. Return `GenerateError::InvalidConfiguration`
  with the offending entry and reason. SBE-pattern rationale: a multi-schema
  generator has one atomic module graph; validate that graph before emitting
  any codec or shared type.
- What breaks (API only), what it buys: No Rust API break; invalid generation
  configurations now fail before partial output. It removes path traversal,
  overwrite, duplicate-module, and self-import failure modes from build
  scripts.
- Acceptance criteria: Extend
  `sbe/tests/multi_schema_versioning_test.rs` with empty, keyword,
  punctuation/path, duplicate, and mismatched-owner cases; assert no files are
  emitted after a failed plan. Preserve the valid multi-schema golden.
- Verification plan: Run the multi-schema test targets,
  `cargo test -p ergo-sbe`, `just check-golden`, and `just bench`.
  Confirm valid generated Rust is byte-identical, which is also the
  non-regression proof for generated hot paths.

## T-4: Restore the promised non-exhaustive cluster error contract

- Type: API
- Stage: 0.1.17
- Priority: P1 · Effort: S
- Symptom: `CHANGELOG.md:31-36` says 0.1.14 made `ClusterError`
  non-exhaustive, but its declaration at `cluster/src/error.rs:120-124`
  lacks `#[non_exhaustive]`. The intended form is already used on
  `ConnectStep` at `cluster/src/client.rs:1083-1087`.
- Change: Add `#[non_exhaustive]` to `ClusterError` and add an external
  consumer fixture that documents wildcard matching. SBE-pattern rationale:
  the typed protocol/trust-boundary error taxonomy must be able to grow as new
  wire and cluster failure modes become observable.
- What breaks (API only), what it buys: Downstream exhaustive matches must add
  a wildcard arm, which is the contract already announced in 0.1.14. Users
  gain additive future error variants instead of another semver break.
- Acceptance criteria: Add `cluster/tests/non_exhaustive_api.rs` with trybuild
  fixtures under `cluster/tests/non_exhaustive/`: one external match with `_`
  passes and one exhaustive match without it fails. Run
  `scripts/check-public-api.sh` and update any `ClusterError` rustdoc example
  that matches the enum.
- Verification plan: Run `cargo test -p ergo-aeron-cluster` and the public
  API check. Compile cluster benchmark targets with `cargo bench
  -p ergo-aeron-cluster --no-run`; the attribute produces no executable or
  benchmark change.

## T-5: Redact credentials from diagnostics

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P1 · Effort: S
- Symptom: `StaticCredentials` derives `Debug` directly over its
  `Vec<u8>` at `cluster/src/credentials.rs:36-45`; those bytes are the
  actual connect/challenge secret used at
  `cluster/src/credentials.rs:48-69`. `SessionBuilder` contains the
  credential provider and other useful configuration but has no safe
  diagnostic representation (`cluster/src/config.rs:36-65`).
- Change: Replace the derived credential `Debug` with a manual
  implementation that shows only `<redacted>` and, if useful, byte length.
  Add a manual `SessionBuilder::Debug` that prints channels, IDs, timeouts,
  and feature flags while rendering credentials and idle strategy only as
  `<configured>`. SBE-pattern rationale: credentials are opaque var-data at
  a protocol boundary; diagnostics may expose metadata, never payload bytes.
- What breaks (API only), what it buys: No API break; exact `Debug` text
  changes. Users can log connection configuration without leaking reusable
  credentials.
- Acceptance criteria: Add the credential redaction assertion to
  `cluster/src/credentials.rs` and the safe builder diagnostic assertion to
  `cluster/src/config.rs`. Format both around a recognizable secret and assert
  that neither its text nor byte sequence appears while safe connection fields
  remain useful.
- Verification plan: Run `cargo test -p ergo-aeron-cluster` and compile its
  benchmark targets. This is cold formatting code; prove benchmark
  non-regression by confirming the session poll/offer code and benchmark
  binaries have no changed hot-path call sites.

## T-6: Reject duplicate ingress member IDs

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P1 · Effort: S
- Symptom: `parse_ingress_endpoints` collects and sorts entries without
  detecting duplicate member IDs at `cluster/src/endpoints.rs:17-44`.
  Redirect resolution then selects the first matching ID at
  `cluster/src/poller.rs:128-146`, making the chosen endpoint depend on
  input order. Builder validation relies on this parser
  (`cluster/src/config.rs:216-241`).
- Change: Detect duplicate IDs during parsing and return `ConnectFailed`
  naming the ID and both conflicting endpoint declarations.
  SBE-pattern rationale: the ingress list is a logical map keyed by cluster member ID;
  ambiguity must be rejected before entering the leader state machine.
- What breaks (API only), what it buys: No API break; ambiguous configurations
  that previously made an order-dependent choice now fail. Users get a
  deterministic configuration error before connecting.
- Acceptance criteria: Extend `cluster/src/endpoints.rs` tests with duplicates
  before and after sorting, whitespace, repeated identical endpoints,
  conflicting endpoints, and a non-duplicate control. Assert the error
  identifies the member ID.
- Verification plan: Run `cargo test -p ergo-aeron-cluster` and compile its
  benchmarks. The parser is a cold builder seam; verify no poll-loop code or
  benchmark instruction path changes.

## T-7: Validate timeouts before narrowing to milliseconds

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P1 · Effort: S
- Symptom: Builder setters convert with unchecked
  `timeout.as_millis() as u64` at `cluster/src/config.rs:135-149`.
  Sub-millisecond nonzero durations become zero and `Duration::MAX` wraps.
  `validate` does not inspect either timeout
  (`cluster/src/config.rs:216-253`), despite `InvalidTimeout` promising
  zero/overflow rejection at `cluster/src/error.rs:224-233`.
  `checked_deadline` only sees the already-narrowed value
  (`cluster/src/client.rs:30-45`).
- Change: Retain `Duration` until validation or centralize a checked
  `u64::try_from(timeout.as_millis())` conversion; reject zero,
  sub-millisecond, and greater-than-`u64` millisecond values before transport
  setup. Use the same helper for sync, async, and new-leader paths.
  SBE-pattern rationale: validate cold configuration once so impossible timing
  states cannot enter the protocol poll state machine.
- What breaks (API only), what it buys: No signature break; formerly truncated
  durations return the documented error. Users no longer see an immediate or
  wrapped timeout that contradicts what they configured.
- Acceptance criteria: Extend `cluster/src/config.rs` tests with 0, 1ns,
  999999ns, 1ms, the `u64`-millisecond boundary, and `Duration::MAX`; add
  sync/async parity assertions in `cluster/src/client.rs`. Both paths must
  report the same error.
- Verification plan: Run `cargo test -p ergo-aeron-cluster`. Compile
  benchmarks and inspect the diff to confirm conversion remains a one-time
  builder operation; run `just bench-cluster` only if the stored
  representation reaches a measured polling path.

## T-8: Preserve the distinction between malformed XML and invalid SBE

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P1 · Effort: S
- Symptom: `sbe/src/xml/entry.rs:56-74` maps every
  `XsdValidationError` to `ParseError::MalformedXml`, even though
  `sbe/src/xsd.rs:20-63` distinguishes malformed XML from a well-formed
  document that violates the SBE schema. The public parser error taxonomy
  already has separate `MalformedXml` and `Invalid` variants
  (`sbe/src/xml/error.rs:10-51`).
- Change: Map only the malformed-XML source case to `MalformedXml`; map bad
  roots, elements, attributes, and values in well-formed XML to `Invalid`
  while preserving the underlying diagnostic and span. SBE-pattern rationale:
  syntax failure and wire-schema contract failure are separate trust-boundary
  stages and need actionable errors.
- What breaks (API only), what it buys: No API shape break; callers matching
  variants receive the accurate existing variant. Users can distinguish a
  broken XML document from a syntactically valid but non-SBE schema.
- Acceptance criteria: Add table-driven cases to `sbe/src/xml/tests.rs` and
  `sbe/tests/error_validation_test.rs` for malformed XML, wrong root, unknown
  element, invalid attribute, and valid schema. Assert both variant and a
  stable diagnostic fragment/source chain.
- Verification plan: Run `cargo test -p ergo-sbe` and `just bench`.
  Confirm no valid fixture or generated file changes; parser-only code cannot
  affect maintained encode/decode benchmark instructions.

## T-9: Reconcile the published release and performance status

- Type: DOCS
- Stage: 0.1.17
- Priority: P1 · Effort: S
- Symptom: `book/src/project/road-to-1.0.md:60-69` describes 0.1.15 as
  still needed, while
  `book/src/project/performance-release-ledger.md:23-37` calls 0.1.14
  pending and 0.1.15 future. The ledger also links parity to roadmap criterion
  4 at `book/src/project/performance-release-ledger.md:4`, although parity
  is criterion 2 at `book/src/project/road-to-1.0.md:14-17`. A live
  `gh api repos/mimran1980/ergon/releases` check on 2026-08-11 found
  published 0.1.15 and 0.1.16 releases with zero assets and no 0.1.14 release;
  historical benchmark evidence therefore cannot be verified from the
  repository or published release assets.
- Change: Update the release table in
  `book/src/project/road-to-1.0.md` and the complete status table in
  `book/src/project/performance-release-ledger.md`. List tags/releases
  separately from performance proof; mark every benchmark cell
  `unverified — artifact not represented` unless it names an immutable run
  ID, commit, profile, and asset. Correct the criterion link.
  SBE-pattern rationale: performance parity is part of this codec's public contract, so
  its evidence must be tied to the generated-code version and benchmark
  profile.
- What breaks (API only), what it buys: No API break. A fresh maintainer sees
  the actual evidence gap instead of treating an undocumented checkmark as a
  completed 1.0 criterion.
- Acceptance criteria: Neither page contradicts `CHANGELOG.md:3-36`, local
  tags, or published releases; no green cell lacks provenance; both pages
  explain how T-15 will produce future evidence. `mdbook build book` passes
  without broken links.
- Verification plan: Re-run the GitHub release-asset query and
  `mdbook build book`. No benchmark run is needed for a docs-only correction;
  the definition of performance non-regression is that no Rust, schema,
  fixture, or benchmark source changes in this ticket.

## T-10: Put schema field descriptions on encoder APIs

- Type: DOCS
- Stage: 0.1.17
- Priority: P2 · Effort: S
- Symptom: Decoder field generation emits schema descriptions at
  `sbe/src/codegen/message_decoder.rs:591-599` and
  `sbe/src/codegen/message_decoder.rs:710-713`. The encoder imports the same
  doc helper but its setter loop at
  `sbe/src/codegen/message_encoder.rs:481-664` does not apply
  `f.description`; the generated setter is bare at
  `sbe/tests/golden/car_example.rs:6134-6144`. The `FixedFields` rustdoc
  also says optional “or versioned” fields use `Option`
  (`sbe/src/codegen/message_encoder.rs:706-708`), while the emitted type
  does that only for optional presence
  (`sbe/src/codegen/message_encoder.rs:692-703`).
- Change: Emit `doc_attr_tokens(f.description)` immediately above every
  generated public message-encoder setter variant: primitive/conversion,
  array, enum, set, and composite. Correct `FixedFields` rustdoc to say only
  presence-optional fields are `Option`; `sinceVersion` fields remain
  concrete. SBE-pattern rationale: schema XML is the single semantic source
  for both read and write flyweight surfaces.
- What breaks (API only), what it buys: No API or wire break. Encoder users
  see the same field semantics as decoder users, and no longer infer false
  optionality from generated docs.
- Acceptance criteria: Extend
  `sbe/tests/schema_docs_provenance_test.rs:1-14` to assert descriptions on
  decoder and encoder methods with no sibling leakage. Update the schema-doc
  fixture golden and run rustdoc with warnings denied.
- Verification plan: Run the provenance tests, `cargo doc -p ergo-sbe
  --no-deps`, `just check-golden`, and `just bench`. Diff generated
  methods after stripping doc attributes; executable tokens and benchmark
  ratios must be identical.

## T-11: Preserve UTF-8 decode causes in the error chain

- Type: API
- Stage: 0.1.17
- Priority: P2 · Effort: S
- Symptom: Generated `DecodeError::InvalidUtf8` stores
  `core::str::Utf8Error` at `sbe/src/codegen/runtime.rs:14-38`, but its
  `std::error::Error` implementation exposes no source
  (`sbe/src/codegen/runtime.rs:59`). Generated `EncodeError` and
  `VerifyError` correctly return nested causes at
  `sbe/src/codegen/runtime.rs:106-113` and
  `sbe/src/codegen/runtime.rs:172-178`.
- Change: Implement `DecodeError::source` and return the stored UTF-8 error
  for `InvalidUtf8`; return `None` for variants without a cause.
  SBE-pattern rationale: keep the typed byte-level cause at the text
  var-data trust boundary rather than flattening it into display text.
- What breaks (API only), what it buys: This is additive behavior with no
  signature break. `anyhow`, `miette`, and ordinary error-chain
  diagnostics can reach the exact UTF-8 failure.
- Acceptance criteria: Extend `sbe/tests/generated_runtime_api_test.rs` to
  decode invalid UTF-8 and assert `std::error::Error::source` is the expected
  `Utf8Error`; assert source-less variants still return `None`. Update
  `sbe/tests/golden/car_example.rs`.
- Verification plan: Run `cargo test -p ergo-sbe`, `just check-golden`,
  and `just bench`. The new match is reachable only while reporting an
  existing cold error; successful decode instructions and ratios must not
  change.

# 2. Main tickets — 0.1.17

## T-12: Reject wire-incompatible shared type reuse

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P0 · Effort: M
- Symptom: Multi-schema generation records only the first schema's type names
  at `sbe/src/codegen/mod.rs:623-641`, then skips every later type with the
  same name at `sbe/src/codegen/mod.rs:650-655`. It never compares the
  definitions or byte order, so a later schema can silently compile against a
  wire-incompatible enum, set, or composite owned by the first schema.
- Change: Compute a canonical wire fingerprint for every shared enum, set, and
  composite before emission. Include token order, primitive encoding,
  offsets/lengths, presence, constants/null/min/max, discriminants/choices,
  `sinceVersion`, and schema byte order. Compare all name collisions and
  return an additive
  `GenerateError::IncompatibleSharedType { name, owner_module,
  consumer_module, difference }` before writing files.
  SBE-pattern rationale: a type name is not wire identity; shared codecs are safe only
  when their complete encoded layouts agree.
- What breaks (API only), what it buys: No public Rust signature is removed;
  previously accepted incompatible schema sets now fail generation. Users
  cannot silently decode later-schema bytes with the first schema's layout.
- Acceptance criteria: Extend `sbe/tests/multi_schema_versioning_test.rs` with
  mismatched primitive type, enum discriminant, set bit, composite
  order/offset, presence/default, version metadata, and byte order; identical
  duplicates must still share one module. Name both owners and the first
  differing property in the error. Update
  `book/src/sbe/getting-started/multi-schema.md:34-36`.
- Verification plan: Run the multi-schema suite, generated public API checks,
  `just check-golden`, SBE Tool multi-schema parity, and `just bench`.
  Valid shared definitions must produce byte-identical generated code and
  unchanged LTO/no-LTO ratios.

## T-13: Make FixedFields None write the schema null image

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P0 · Effort: M
- Symptom: The generator represents optional fixed fields as `Option` at
  `sbe/src/codegen/message_encoder.rs:680-718`, but `fixed()` skips every
  `None` at `sbe/src/codegen/message_encoder.rs:741-748`. Wrap docs warn
  that existing bytes are not nullified
  (`sbe/src/codegen/message_encoder.rs:356-361`), and nullification is a
  separate opt-in path (`sbe/src/codegen/message_encoder.rs:445-475`).
  The book explicitly admits that stale bytes can ship
  (`book/src/sbe/design-notes/nullval.md:50-57`). Null-image generation
  currently declines composites
  (`sbe/src/codegen/nullification.rs:47-63`), although
  `sbe/tests/fixtures/schemas/issue972.xml:16-24` contains a top-level
  optional composite.
- Change: Make `fixed(&FixedFields)` write every fixed field. `Some`
  writes the value and `None` writes the exact schema null wire image,
  recursively for optional composites. If a legal null image cannot be
  derived, reject the schema/generation instead of preserving buffer history.
  Keep `apply_nulls` only as a raw/per-field convenience.
  SBE-pattern rationale: an exhaustive fixed-field snapshot must fully determine the
  fixed block; prior buffer contents are not message state.
- What breaks (API only), what it buys: No signature break. Behavior changes
  only for callers relying on `None` to retain stale bytes, which violates
  whole-message semantics. Users can safely reuse dirty buffers without
  optional values leaking from the previous message.
- Acceptance criteria: Extend
  `sbe/tests/sbe_tool_multi_schema_wire_parity_test.rs` and
  `sbe/tests/issue_regression_test.rs`: reuse one nonzero buffer to encode
  `Some`, then `None`, and prove the second message decodes `None`. Cover
  primitive, signed/unsigned, float, enum, set, big-endian, and nested optional
  composite null images. Compare bytes with SBE Tool; remove the admitted
  zero-buffer discrepancy at
  `sbe/tests/sbe_tool_multi_schema_wire_parity_test.rs:1793-1794` and
  `sbe/tests/sbe_tool_multi_schema_wire_parity_test.rs:1842-1854`.
- Verification plan: Add an equal-work optional-field arm to
  `sbe/benchmarks/benches/encode_style_bench.rs` comparing the new
  `fixed` operation with the old correct sequence
  `apply_nulls + fixed`. Run parity/goldens, that focused benchmark, and
  mandatory `just bench` in LTO/no-LTO. The expected cost mechanism is
  sentinel stores on `None`; accept only wire-correct output with maintained
  ratios at or below `1.00`.

## T-14: Enforce the fixed block as a real typestate transition

- Type: API
- Stage: 0.1.17
- Priority: P0 · Effort: L
- Symptom: Repository policy says `fixed` is the only path to tails and
  individual setters live only on `RawFixedWriter`
  (`CLAUDE.md:311-319`). Codegen predeclares an unused fixed encoder name at
  `sbe/src/codegen/message_encoder.rs:88-92`, makes `fixed()` return the
  same root type at `sbe/src/codegen/message_encoder.rs:763-771`, emits a
  methodless raw writer at `sbe/src/codegen/message_encoder.rs:774-808`,
  and puts setters on the root encoder starting at
  `sbe/src/codegen/message_encoder.rs:481`. The golden exposes a hollow
  writer (`sbe/tests/golden/car_example.rs:5854-5860`), root setters and
  `fixed() -> Self` (`sbe/tests/golden/car_example.rs:6134-6271`), and a
  first tail directly on the root encoder
  (`sbe/tests/golden/car_example.rs:6324-6335`). Baseline coverage skips
  fixed initialization at `sbe/tests/baseline_test.rs:2988-3025`, while the
  “raw parity” test never calls `raw_fixed`
  (`sbe/tests/conformance_test.rs:609-637`).
- Change: Constructors and `wrap` return `{Message}UnfixedEncoder`; only
  `.fixed(&{Message}FixedFields) -> {Message}Encoder` grants the first tail
  or terminal byte/length views. For fixed-only messages, completion views
  likewise exist only after the transition. Delete `RawFixedWriter/raw_fixed`
  unless a design can statically prove every required fixed field was written;
  do not add an unchecked finish. Update the API-freeze fixtures,
  `book/src/sbe/getting-started/encode-decode.md:5-22`, and the uncompilable
  raw example at `book/examples/heartbeat-encode.rs:14-20`.
  SBE-pattern rationale: the fixed block is the mandatory first state of an
  SBE message; a deep encoder API should make “tails or complete bytes before
  fixed initialization” unrepresentable.
- What breaks (API only), what it buys: Direct setters on the root encoder,
  raw-writer names, root type annotations, and any tail/byte view before
  `fixed` stop compiling. Users get one clear initialization call and cannot
  publish a message whose required fixed block was skipped.
- Acceptance criteria: Add compile-fail cases to
  `sbe/tests/typestate_budgets_test.rs` for `wrap().first_tail()`,
  `wrap().as_bytes()`, and fixed-only completion before `fixed`; compile the
  normal transition and tails. Repair `sbe/tests/conformance_test.rs`,
  `sbe/tests/baseline_test.rs`, the cited book examples,
  `sbe/tests/golden/car_example.rs`, API fixtures, and SBE Tool wire parity.
- Verification plan: The transition must be a move-only, allocation-free,
  branch-free wrapper change with all public hot methods retaining their
  intended inline attributes. Run compile tests, `encode_style_bench`,
  instruction probes, and mandatory `just bench` in both profiles. Any ratio
  movement requires instruction-level attribution; no maintained benchmark
  may exceed `1.00`.

## T-15: Fix benchmark artifact packaging before publishing

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P0 · Effort: M
- Symptom: The release workflow runs benchmark gates at
  `.github/workflows/release.yml:47-54` but publishes crates at
  `.github/workflows/release.yml:70-108` before packaging evidence. It
  creates only `bench-sbe-lto.tar.gz` at
  `.github/workflows/release.yml:117-120`, then uploads a nonexistent no-LTO
  archive at `.github/workflows/release.yml:159-167`. It creates cluster
  `.tar.gz` files at `.github/workflows/release.yml:122-126` but uploads
  nonexistent `.json` paths at `.github/workflows/release.yml:169-187`,
  labels the SBE tar as JSON at `.github/workflows/release.yml:149-157`, and
  uploads a nonexistent root `run-manifest-sbe.json` at
  `.github/workflows/release.yml:189-197`. The real SBE
  runner writes per-run, per-profile manifests under
  `target/bench-runs/<id>/...`
  (`scripts/run-sbe-bench.sh:12-26,60-91,111-160`), while cluster recipes
  have no equivalent run IDs or manifests (`justfile:326-336`). Live
  release inspection on 2026-08-11 found zero assets on 0.1.15 and 0.1.16.
- Change: Add one fail-closed packaging/check script used by both the GitHub
  workflow and local release recipe. It must discover the fresh SBE run ID,
  stamp cluster LTO/no-LTO results with equivalent commit/profile/toolchain
  provenance, create consistently named gzip archives containing Criterion
  estimates plus manifests, and validate every path, media type, archive
  member, and commit before any `cargo publish`. Remove permissive
  `|| true` handling from the evidence path. SBE-pattern rationale:
  performance evidence is part of the generated-code contract and must stay
  coupled to the exact codec revision and compilation profile.
- What breaks (API only), what it buys: No Rust API break; release jobs now
  fail before publication when evidence is missing or stale. Maintainers and
  users receive reproducible assets rather than an unsubstantiated parity
  claim.
- Acceptance criteria: Add `scripts/test-package-bench-artifacts.sh`; its dry
  run proves every referenced asset exists, each archive expands, both
  profiles contain Criterion estimates and a manifest, manifests match the
  release commit, and a missing/stale file fails before a mocked publish step.
  Make `.github/workflows/release.yml` and the local `just release` path use
  the same checker.
- Verification plan: Execute a release-like `just bench` and
  `just bench-cluster`, run the package checker, inspect archive contents,
  and dry-run the workflow ordering. This ticket changes no benchmarked Rust;
  the newly packaged results themselves must show all maintained ratios
  passing in both profiles.

## T-16: Canonicalize cluster egress decoding and propagate failures

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P1 · Effort: M
- Symptom: `cluster/src/fragment.rs:1-4` claims one canonical egress decode
  path, and `Fragment::decode` rejects short or malformed input at
  `cluster/src/fragment.rs:57-81`. The connect poller independently decodes
  frames at `cluster/src/poller.rs:56-125`, with a test that treats a
  four-byte frame as no event at `cluster/src/poller.rs:153-158`. The sync
  handshake discards both Aeron poll and decode errors at
  `cluster/src/client.rs:311-322`; async still discards the poll result at
  `cluster/src/client.rs:1379-1398`. Missing redirect leaders silently loop
  at `cluster/src/client.rs:345-355` and
  `cluster/src/client.rs:1239-1260`, despite regular polling showing the
  intended propagation pattern at `cluster/src/client.rs:766-775` and
  `cluster/src/client.rs:804-812`.
- Change: Derive an owned connection `EgressEvent` projection from the
  canonical `Fragment::decode`; add one poll-once helper that maps Aeron
  poll failures and callback decode failures; use it in sync and async
  handshakes. Resolve redirects through validated ingress endpoints and return
  `ReconnectFailed` immediately for a missing or malformed leader.
  SBE-pattern rationale: the egress frame needs exactly one fail-closed decode
  at the wire trust seam, followed by state-machine-specific projection.
- What breaks (API only), what it buys: No public signature needs to break.
  Users receive the actual malformed-frame, transport, or redirect error
  instead of a misleading timeout after the cause was swallowed.
- Acceptance criteria: Extend `cluster/src/poller.rs`,
  `cluster/tests/egress_fragmentation.rs`, and `cluster/tests/failover.rs` with
  sync/async parity for short headers, truncated blocks, invalid text, unknown
  templates, injected poll failure where existing seams allow, and redirects
  to missing/malformed leaders. Delete the duplicate decoder or reduce it to
  the canonical projection.
- Verification plan: Run `cargo test -p ergo-aeron-cluster` and
  `just bench-cluster`. Inspect allocation counts: the hot borrowed
  `Fragment` decode path must gain no allocation; owned strings are allowed
  only in the cold connection-handshake projection. Poll-loop latency must not
  regress beyond existing noise thresholds.

## T-17: Validate numeric schema metadata instead of silently dropping it

- Type: CORRECTNESS
- Stage: 0.1.17
- Priority: P1 · Effort: M
- Symptom: The vendored XSD defines `deprecated` as a nonnegative schema
  version at `sbe/src/xsd/sbe.xsd:310-331`, but parsers turn mere attribute
  presence into a boolean for messages, fields, groups, and data at
  `sbe/src/xml/message.rs:25-40,156-170,266-284,322-348`, and for type
  declarations at
  `sbe/src/xml/types.rs:152-180,230-250,590-605,758-768`. Tests even
  institutionalize invalid `deprecated="true"` at
  `sbe/src/xml/tests.rs:2147-2170` and
  `sbe/src/xml/tests.rs:2324-2379`. Invalid `nullValue`, `minValue`, and
  `maxValue` text can also become `None` through
  `parse_u64_val` at `sbe/src/xml/registry.rs:90-119`.
- Change: Introduce central fallible `opt_deprecated_attr` and
  `opt_wire_value_attr` helpers. In 0.1.17, retain the boolean IR but accept
  only a valid nonnegative version number as “deprecated”; reject strings,
  negatives, and overflow with a span. Reject, rather than default, malformed
  null/min/max declarations across primitive, signed, float, enum, set, and
  composite contexts. T-102 can preserve the exact version in 1.0.
  SBE-pattern rationale: version and sentinel metadata directly control
  generated wire semantics and must be parsed once, explicitly, and
  fail-closed.
- What breaks (API only), what it buys: No Rust API break; invalid schemas that
  previously inherited a default now fail. Users cannot unknowingly generate
  codecs with different version or sentinel behavior from the schema text.
- Acceptance criteria: Extend `sbe/src/xml/tests.rs` and
  `sbe/tests/null_min_max_test.rs` with every supported element kind for
  absent, 0, positive, nonnumeric, negative, and overflowing `deprecated`,
  plus signed/unsigned/float null/min/max. Repair invalid fixtures, assert
  span-bearing diagnostics, and prove valid generated code is unchanged.
- Verification plan: Run XML/parser suites, schema corpus generation,
  SBE Tool parity, `just check-golden`, and mandatory `just bench`.
  Byte-identical output for valid fixtures and unchanged LTO/no-LTO ratios are
  required.

## T-18: Keep structural bounds fallible in unchecked text accessors

- Type: API
- Stage: 0.1.17
- Priority: P1 · Effort: M
- Symptom: Generated `*_as_str_unchecked` documents only a character
  encoding precondition but calls a fallible bounds accessor with
  `.unwrap()` at `sbe/src/codegen/tail_stages.rs:302-318`. Equivalent
  decoder generation does the same at
  `sbe/src/codegen/message_decoder.rs:1307-1321` and
  `sbe/src/codegen/message_decoder.rs:1344-1359`. A caller can uphold the
  UTF-8/ASCII safety precondition and still panic on a truncated or overflowing
  var-data length.
- Change: Let `unsafe` skip only character validation. Return
  `Result<&str, DecodeError>` for terminal accessors and
  `Result<(&str, NextStage), DecodeError>` for staged accessors so structural
  extent checks stay fallible. Do not replace the panic with
  `unwrap_unchecked`; a future verified-frame token can optimize that only
  if it proves extent separately. SBE-pattern rationale: structural frame
  validity and text encoding validity are independent proofs at a var-data
  boundary.
- What breaks (API only), what it buys: The unsafe accessor return type gains
  `Result`, so callers add `?` or handle the error. In exchange, honoring
  the documented unsafe precondition is sufficient to avoid a panic on
  malformed frame extent.
- Acceptance criteria: Extend `sbe/tests/checked_unchecked_parity_test.rs` and
  `sbe/tests/ordered_decoder_stages_test.rs` with valid unchecked text,
  truncated payload, overflowing length, and invalid text with the caller
  precondition separated. Add a migration compile fixture to
  `sbe/tests/stability_test.rs`; update rustdoc, golden, and API baseline.
- Verification plan: Run decoder/tail soundness tests, `just check-golden`,
  instruction probes, and `just bench` LTO/no-LTO. The bounds check already
  exists; the success-path mechanism changes only error propagation. Require
  unchanged maintained ratios and attribute any instruction delta.

## T-19: Generate typestate for dynamic group entries

- Type: API
- Stage: 0.1.17
- Priority: P1 · Effort: L
- Symptom: After T-1 removes false completion proofs, dynamic group entries
  still expose nested groups and var-data setters as repeated mutations on the
  same `EntryEncoder` at `sbe/src/codegen/group_encoder.rs:592-725`.
  Nothing in the type system prevents skipping, reordering, or repeating
  positional entry tails.
- Change: Generate named entry-tail stages parallel to message-tail stages:
  an entry fixed-block encoder consumes itself into the first nested-group or
  var-data stage, each stage consumes into the next, and the last returns
  `EntryComplete`. Re-enable `add_checked` for dynamic groups only when its
  closure returns that real proof; retain `add` as the explicitly flexible
  path. SBE-pattern rationale: repeating-group entries are miniature SBE
  messages with the same fixed-then-ordered-tail grammar.
- What breaks (API only), what it buys: Checked dynamic entry closures gain
  named stages and consuming calls; code that mutates tails out of order stops
  compiling. Users get compile-time completeness and ordering for nested
  entry payloads.
- Acceptance criteria: Add `sbe/tests/group_entry_typestate_test.rs` with
  compile-fail skip, reorder, repeat, and early-completion cases across
  var-data and nested groups, plus compile-pass zero/one/many and
  version-gated entries. Update generated goldens, API fixtures, book group
  examples, and SBE Tool wire parity.
- Verification plan: Generated transitions must be allocation-free and
  branch-free. Run group soundness tests, `just bench-groups`, instruction
  probes, and mandatory `just bench` in both profiles; maintained ratios
  must remain at or below `1.00`.

## T-20: Mark generated pure observers as must-use

- Type: API
- Stage: 0.1.17
- Priority: P2 · Effort: M
- Symptom: Representative generated primitive and enum getters return
  discardable scalar values with only an inline attribute at
  `sbe/tests/golden/car_example.rs:1645-1765`; bit-set raw values and
  predicates do the same at `sbe/tests/golden/car_example.rs:646-674`.
  Array getters and copy-count observers are generated at
  `sbe/src/codegen/message_decoder.rs:640-691` without a systematic
  must-use classification.
- Change: Add `#[must_use]` to generated pure decoder/composite/header
  getters, raw enum/set values, set predicates, metadata position/length
  queries, and pure encoded-length functions. Exclude mutating setters,
  cursor transitions, and `Result`/`Option` returns whose standard type is
  already must-use. Add a generator-level classification helper rather than
  scattered attributes. SBE-pattern rationale: flyweight observers do not
  advance a cursor; discarding their value cannot perform decode work and is
  almost always a caller mistake.
- What breaks (API only), what it buys: Code built with
  `deny(unused_must_use)` may need `let _ = ...` for intentionally ignored
  values. Users get a compiler warning for no-op calls such as
  `decoder.serial_number();` or `extras.is_sun_roof();`.
- Acceptance criteria: Add `sbe/tests/must_use_generated_api_test.rs` with
  AST-based coverage for every observer category and exclusion, representative
  golden assertions, and a downstream fixture under
  `deny(unused_must_use)`. A discarded observer must fail while
  setters/transitions remain unaffected.
- Verification plan: Run generator tests, public API/golden checks, and
  `just bench`. Attributes must not alter executable generated tokens or
  LTO/no-LTO benchmark ratios.

# 3. 1.0-only tickets

Do not pull these into 0.1.17: each changes a broad public representation or
requires release-baseline infrastructure first.

## T-100: Implement the generated-fixture public API gate

- Type: API
- Stage: 1.0
- Priority: P0 · Effort: L
- Symptom: `api/public-api-baseline.toml:1-3` promises that generated
  fixtures are compiled, their API extracted, and differences classified; it
  enumerates fixtures/profiles/multi-schema/cluster at
  `api/public-api-baseline.toml:5-40` and a baseline tag at lines 42-44.
  `scripts/check-public-api.sh:59-62` explicitly says fixture extraction is
  future work, while CI presents the script as an enforced public-API policy
  at `.github/workflows/ci.yml:54-61`.
- Change: For every manifest entry, generate at the baseline tag and at HEAD
  in isolated temporary crates, compile both, extract rustdoc JSON or
  `cargo-public-api`, canonicalize paths/noise, and classify the diff. Fail
  closed on a missing fixture/profile/config, save the full diff artifact, and
  permit baseline replacement only through an explicit release command.
  SBE-pattern rationale: schema-specific generated Rust—not merely the
  generator crate—is the codec's real public flyweight and typestate API.
- What breaks (API only), what it buys: The checker itself removes no API, but
  CI will begin rejecting unclassified generated breaks. Consumers gain a
  trustworthy warning before generated method or stage changes reach a
  release.
- Acceptance criteria: Add `scripts/test-check-public-api-fixtures.sh` with
  rename, removal, signature-change, and addition mutations. Exercise every
  manifest profile, multi-schema ownership, and cluster fixtures; prove
  baseline and HEAD are isolated and reproducible. Document the explicit
  baseline-update command in `api/public-api-baseline.toml` comments and the
  release book chapter.
- Verification plan: Run the new gate twice from clean checkouts and compare
  canonical output; run workspace CI/release checks. The gate changes no
  generated hot path, so benchmark non-regression is established by a clean
  executable diff; when it surfaces a codegen fix, that fix must run its own
  mandatory benchmark gate.

## T-101: Make Schema wire identity single-source

- Type: API
- Stage: 1.0
- Priority: P1 · Effort: M
- Symptom: Public `Schema` duplicates `package`, `id`, and `version`
  beside a public `Ir` at `sbe/src/schema.rs:27-40`, cloning them once in
  `Schema::from_ir` at `sbe/src/schema.rs:67-88`. Consumers can mutate the
  two identities independently. Generator context/header code reads
  `schema.ir` at `sbe/src/codegen/mod.rs:261-277`, header validation reads
  outer fields at `sbe/src/codegen/mod.rs:358-360`, and generated
  provenance/hash reads outer identity at
  `sbe/src/codegen/mod.rs:893-897` and
  `sbe/src/codegen/mod.rs:1120-1123`.
- Change: Store only private `ir: Ir` in `Schema`; expose
  `package()`, `id()`, `version()`, `ir()`, and `into_ir()`
  accessors derived from that source. Make generator code consume those
  accessors consistently. SBE-pattern rationale: package, schema ID, and
  version are one wire identity and must have one authoritative
  representation.
- What breaks (API only), what it buys: Direct field mutation and struct
  literals stop compiling; callers use accessors or mutate/build `Ir`
  explicitly. It becomes impossible for encoded header validation,
  provenance, and generated hash to describe different schema identities.
- Acceptance criteria: First add the deliberate outer-versus-IR divergence to
  `sbe/tests/stability_test.rs`, then make it unrepresentable. Update public API
  baselines, construction call sites, rustdoc, and add before/after accessors
  to `book/src/sbe/getting-started/migration-1.0.md` plus `book/src/SUMMARY.md`.
- Verification plan: Run schema/codegen tests, generated API and golden checks,
  SBE Tool parity, and `just bench`. The refactor is cold metadata access;
  generated codec output and maintained benchmark ratios must be unchanged.

## T-102: Preserve the schema version that deprecated each item

- Type: API
- Stage: 1.0
- Priority: P2 · Effort: M
- Symptom: The XSD defines `deprecated` as the schema version at
  `sbe/src/xsd/sbe.xsd:323-330`, but public IR collapses it to a boolean at
  `sbe/src/ir.rs:137-140`; structured IR does the same at
  `sbe/src/structured_ir.rs:142` and
  `sbe/src/structured_ir.rs:163`. The public generation hook exposes only
  that boolean around `sbe/src/config.rs:183-205`, discarding information
  migration tooling needs.
- Change: After T-17 validates the syntax, carry
  `deprecated_since: Option<u16>` through token encoding, IR, structured
  message/group/value fields, and `FieldInfo` hooks. Generate rustdoc that
  says “Deprecated since schema version N.” SBE-pattern rationale: SBE
  evolution is version-indexed; presence of deprecation without its version
  is a lossy model of the schema.
- What breaks (API only), what it buys: Public boolean fields and hook pattern
  matches change to `Option<u16>`. Migration and compatibility tools can
  distinguish long-deprecated fields from fields deprecated in the current
  schema version.
- Acceptance criteria: Extend `sbe/src/xml/tests.rs` and
  `sbe/tests/hook_metadata_test.rs` to preserve 0, positive, absent, and
  boundary versions through IR, structured IR, hooks, generated docs, and
  round trips. Update API baselines and
  `book/src/sbe/getting-started/migration-1.0.md` with boolean-to-`is_some()`
  and version-comparison examples.
- Verification plan: Run parser/IR round-trip tests, hook tests, rustdoc,
  generated public API/golden checks, SBE Tool parity, and `just bench`.
  Valid wire layouts and maintained benchmark ratios must remain unchanged;
  only metadata and docs should differ.

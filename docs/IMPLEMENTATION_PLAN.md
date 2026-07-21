# ErgoSBE Release-Readiness Implementation Plan

This is the repository's single design record, living backlog, and implementation
handoff. Historical todo files, phase plans, and completion reports have been
removed because their checked boxes did not reliably describe the implementation.

Do not mark a task complete because generated source contains a method name. A task
is complete only when its behavioural, compile-fail, allocation, performance, and
release acceptance criteria pass where applicable.

## Project posture

- **ErgoSBE** is the primary project: an experimental, opinionated Rust SBE
  generator used to explore safer and faster generated interfaces.
- **Ergo Aeron Cluster** is a hobby experiment. Do not use it in production. The
  preferred long-term solution is official Aeron Cluster C client bindings with
  support in rusteron.
- **Persist** and **Samples** are unpublished, low-quality laboratories used to
  exercise ErgoSBE, domain objects, and transport integrations. They are not
  reference implementations.
- ErgoSBE and Ergo Aeron Cluster may eventually be published to crates.io as
  prototype `0.x` crates. Breaking interface changes are acceptable before the
  first release.
- Official SBE wire compatibility comes first. Maintained hot paths must be at
  least as fast as the equivalent Aeron SBE path and must not allocate.

## Verified baseline and corrected status

The following observations were verified during the 2026-07-21 review. They are
baseline facts, not completed implementation tasks:

- ErgoSBE tests pass with all features.
- ErgoSBE and Ergo Aeron Cluster pass their strict Clippy commands.
- Ergo Aeron Cluster's 75 library tests pass.
- Persist tests pass with all features.
- Benchmark targets compile.
- Workspace formatting does not pass.
- `samples/advanced-bitget` does not compile against the current converter
  interface.
- `samples/cluster-ha-orderbook` checks with warnings that fail under strict
  Clippy because generated version-zero comparisons are useless.
- The ErgoSBE and Cluster package file lists contain substantial test, fixture,
  reference-codec, application-protocol, and harness material.

Areas previously marked complete must be treated as follows:

| Area | Correct status | Evidence still required |
|---|---|---|
| Converter registry | Partial | All documented selectors, validation, precedence, collisions, and all emission sites |
| Latest-version safe encoder | Incomplete | Required-field proof, null writes, group-entry proof, and compile-fail ordering |
| Domain mapper | Not implemented | Configured types in generated domains and lossless fallible recursion |
| Composite symmetry | Partial | Named values, versioned members, direct encoders, and nested symmetry |
| Text variable data | Incorrect | Schema-driven text detection, ordered string methods, and strict errors |
| Cluster hardening | Partial | Error propagation, atomic reconnect, private codecs, and allocation-free offer |
| Examples and samples | Incomplete | No RFQ/auction reference examples, no raw offers, no avoidable unwraps, both labs compiling |
| Performance proof | Incomplete | Equal-work benchmarks and allocation assertions for the new interfaces |
| Documentation and packaging | Incomplete | Honest READMEs, minimal Markdown inventory, and minimal crate packages |

## Canonical design

### 1. Generic conversion seam

Remove all decimal-specific generator knowledge. Decimal, boolean, timestamp,
enum, newtype, composite, and domain conversions use the same two traits in the
shared generated `sbe_rt` module:

```rust
pub trait TryFromSbe<Wire>: Sized {
    type Error: core::fmt::Debug + core::fmt::Display;

    fn try_from_sbe(wire: Wire) -> Result<Self, Self::Error>;
}

pub trait TryToSbe<Wire> {
    type Error: core::fmt::Debug + core::fmt::Display;

    fn try_to_sbe(&self) -> Result<Wire, Self::Error>;
}
```

The generator resolves configuration once into a private `ConversionBinding`
model. Decoder, encoder, fixed-field, composite, group-entry, and domain emitters
all consume that same model. Generated hot paths use static dispatch only: no
trait objects, boxing, heap-backed registries, or runtime selector lookups.

Supported selectors:

1. Exact field path, including recursively nested group paths.
2. SBE `semanticType`.
3. Named primitive alias, enum, set, fixed array, or composite.

That order is also the deterministic precedence. Repeating an identical mapping
is idempotent. Conflicting targets, invalid Rust types, ineligible selectors,
unmatched selectors, or generated method collisions are generation errors. Rust
type strings are parsed as `syn::Type`.

Mapped decoder fields expose `field_as::<T>()`; mapped encoders expose
`field_from(&T)`. Primitive-like wire values are copied. Composite decoding passes
a flyweight view so conversion does not first copy an owned composite. The same
seam is used internally for `BooleanType` to and from `bool`. Decimal and timestamp
types are examples supplied by applications, not dependencies known by ErgoSBE.

### 2. Schema-driven text variable data

Variable data is binary unless the effective payload member declares
`characterEncoding`. Composite names such as `varStringEncoding` are not semantic
evidence by themselves.

The supported built-in text encodings are `UTF-8`/`UTF8` and
`US-ASCII`/`ASCII`, matched case-insensitively. Other encodings remain byte
oriented and may use a configured converter.

The ordered decoder always retains its zero-copy raw method:

```rust
let (detail_bytes, next) = decoder.into_detail()?;
```

For schema-declared supported text it also emits:

```rust
let (detail, next) = decoder.into_detail_as_str()?;
```

This returns `Result<(&str, NextStage), DecodeError>` and allocates nothing. A
non-consuming decoder interface that exposes `detail()` also exposes
`detail_as_str()`. Binary credentials, principals, challenges, application
payloads, and nested SBE payloads do not receive string helpers.

The raw encoder continues accepting `&[u8]`. Supported text fields additionally
receive `detail_str(&str)`. UTF-8 input needs no validation; ASCII input is
validated before any tail bytes are committed. Decoding uses field-aware
`InvalidUtf8` and `InvalidAscii` errors. Do not generate lossy string methods or
replace corrupt input with a sentinel.

Owned generated domains use `String` for supported text variable data and
`Vec<u8>` for binary variable data. Invalid text makes domain conversion fail.
The same rules apply inside repeating-group entries.

Ergo Aeron Cluster's high-level `SessionEventView.detail` and
`NewLeaderEventView.ingress_endpoints` are borrowed `&str`. Credentials and
application payloads remain borrowed bytes.

### 3. Latest-version safe encoding

Encoders target only the schema's latest version. Decoders remain acting-version
aware.

Every non-constant, latest-version required fixed field, including required
`sinceVersion` fields, appears as a required member of `<Message>FixedFields`.
Only fields declared `presence="optional"` use `Option`. `fixed(&fields)` performs
all fallible conversions before writing, writes schema null sentinels for `None`,
and returns the first ordered tail stage.

The initial encoder exposes only `fixed(&fields)` and `raw_fixed(self)`. The raw
transition returns a dedicated consuming writer with individual fixed setters.
Completing that writer returns the same ordered tail stage. The initial stage has
no completion bypass and no `finish_unchecked`.

Concrete consuming stages enforce group and variable-data order. Only the final
complete stage exposes the complete encoded bytes or length. Apply the same fixed
phase to repeating-group entries. For a fixed-only group, adding an entry accepts
its fixed struct; entries with tails transition from fixed fields into their first
tail stage.

Fixed structs derive `Debug`, `Clone`, and `PartialEq`, and derive `Copy` only if
all members are `Copy`. Do not generate `Default` where it could hide missing
required data.

### 4. Composite and domain symmetry

Generate a named latest-version owned value for every composite, plus an
acting-version-aware flyweight decoder and direct encoder. A composite field
offers a zero-copy flyweight and an explicit owned-value accessor. Nested
composites follow the same rule. Members absent from the acting version produce
`FieldNotInVersion` rather than fabricated values.

Configured domain mappings apply recursively to message fields, composites,
groups, and variable data. Domain decoding uses `TryFrom<Decoder>` and a concrete
generated message error. It never uses `filter_map`, `unwrap_or_default`, empty
strings, or empty collections to hide malformed input. Domain encoding travels
through the same latest-version safe encoder and conversion bindings.

Borrowed and fixed-width conversions remain allocation-free unless the selected
domain type inherently allocates.

### 5. Ergo Aeron Cluster interface

Generated Aeron protocol codecs and implementation modules are private. Only
deliberate high-level types are re-exported. Public `channel_cstr` and
`udp_endpoint_cstr` helpers are removed; constant FFI inputs use C literals and
dynamic values are converted locally and fallibly at the FFI seam.

High-level `offer` returns the typed Cluster result and uses rusteron's
scatter/gather `offer_parts` with a stack session header, avoiding the current
per-call `Vec`. Raw Aeron status handling remains private. `try_claim` stays the
explicit zero-copy path.

Listener callbacks, polling, controlled polling, keep-alive, and lifecycle
transitions return `ClusterResult`. Malformed frames, invalid text, callback
errors, and panics at an FFI callback seam are surfaced rather than converted to
`Ok(false)`, `Continue`, or placeholder strings. Session filtering applies to
every session-bearing event.

A leader transition first constructs the endpoint, publication, and both fragment
assemblers. Only after all fallible preparation succeeds does it replace the
client's term, member, publication, assemblers, and state.

RFQ, auction, topic routing, order workflows, and application schemas do not
belong in this generic crate. Retain only connect/echo, controlled-polling, and
failover examples. They use the high-level client, SBE or domain values, typed
`offer`, `Result`, and `?`; they do not use public generated protocol codecs or
`offer_raw`.

Generic lifecycle, reconnect, validation, and observability ideas may be learned
from `reverb-sys/aeron-cluster-client-cpp`, but application protocols are not
copied into the crate.

## Implementation backlog

### A. Lock the contracts with tests

- [ ] Add compile-fail tests proving required latest-version fixed fields cannot
  be omitted and tail methods are unavailable before fixed completion.
- [ ] Add behavioural tests proving optional `None` writes the schema null value
  even when wrapping a buffer containing non-zero bytes.
- [ ] Cover the same fixed-field proof for fixed-only and tailed group entries.
- [ ] Cover selector precedence, every eligible wire kind, nested paths,
  duplicates, conflicts, invalid Rust types, unmatched selectors, and method
  collisions.
- [x] Cover valid and invalid UTF-8 and ASCII, binary fields without string
  methods, text fields inside groups, and encoder validation before writes.
- [ ] Cover acting-version composite members, nested composite encoding, domain
  conversion errors, malformed groups, and malformed variable data.
- [ ] Cover Cluster listener errors, invalid text, session filtering, keep-alive
  failures, callback panics, and reconnect rollback.

### B. Complete ErgoSBE generation

- [x] Build the resolved conversion and character-encoding model once after
  schema normalization and use it from every emitter.
- [x] Remove decimal-specific configuration and generated traits, repeated
  selector scans, unused generation helpers, stale comments, and silent `u8`
  fallbacks.
- [ ] Emit the complete generic conversion interface for primitives, aliases,
  enums, sets, arrays, composites, messages, and recursive group entries.
- [x] Emit schema-driven raw and text variable-data methods with strict,
  field-aware errors and no lossy helpers.
- [ ] Complete the latest-version fixed-field and ordered-tail state machine for
  messages and group entries.
- [ ] Complete owned/flyweight/direct-encoder composite symmetry.
- [ ] Complete recursive fallible domain decoding and encoding.
- [x] Remove generated comparisons such as `acting_version < 0`.

### C. Harden Ergo Aeron Cluster

- [x] Make protocol codecs and internal modules private and expose only the
  high-level client interface.
- [x] Change textual high-level views and callbacks to borrowed `&str`, while
  preserving bytes for binary protocol fields.
- [x] Propagate decoding, text, listener, polling, keep-alive, and controlled
  callback errors through `ClusterResult`.
- [ ] Make leader reconnect state replacement atomic and recreate both fragment
  assemblers.
- [ ] Replace the allocating high-level offer buffer with stack header plus
  `offer_parts`.
- [ ] Remove shallow public CString helpers, RFQ/auction protocols, their codecs,
  schemas, examples, and reference-only public exports.
- [ ] Retain and simplify only connect/echo, controlled-polling, and failover
  examples.

### D. Keep laboratories honest

- [ ] Update Persist only where required to exercise the final converter, text,
  and domain interfaces. Keep it unpublished and outside release claims.
- [ ] Update `advanced-bitget` and `cluster-ha-orderbook` to compile against the
  final interface and use `Result`/`?` instead of avoidable unwraps.
- [ ] Use domain objects where they make the laboratory clearer; retain raw
  flyweights where allocation-free behaviour is what the test exercises.
- [ ] Do not describe either sample as an example to copy or a reference
  implementation.
- [ ] Remove sample and Persist code that no longer exercises a unique product
  interface.

### E. Performance evidence

- [ ] Add equal-work Criterion cases for raw setters versus fixed structs,
  primitive and composite converters, byte versus borrowed-string variable data,
  domain mapping, ordered group entries, and Cluster `offer_parts`.
- [ ] Add allocation assertions for fixed encoding, converter helpers, composite
  flyweights, borrowed text, fixed-width domain round trips, and Cluster offer.
- [ ] Compare generated direct access, safe fixed access, and the maintained Aeron
  reference using byte-identical work.
- [ ] Run three benchmark sessions. No maintained runtime median may regress by
  more than 3%; generator time may not regress by more than 5%. Investigate and
  fix any larger regression before marking this section complete.

### F. Documentation and packaging

- [x] Keep this file as the only design record, backlog, and implementation
  tracker; do not recreate per-task Markdown todos or archived plan trees.
- [x] Keep the root and crate READMEs aligned with the four project postures and
  clearly separate current behaviour from unfinished design.
- [ ] Add a documentation hygiene gate that rejects new todo directories,
  `*-goal.md` files, archived plan trees, or a second active tracker.
- [ ] Repoint the two Rust source-documentation references from the temporary
  `sbe/design/DECISIONS.md` compatibility pointer to this file, then delete the
  pointer without changing the canonical design location.
- [ ] Restrict the ErgoSBE package to its manifest, README, and required source.
- [ ] Restrict the Cluster package to its manifest, README, build script,
  required Aeron schemas, source, and three supported examples.
- [ ] Move Java harness support into the unpublished laboratory area and remove
  tests, reference codecs, RFQ schemas, and application protocols from the
  published Cluster package.
- [ ] Complete crates.io metadata for ErgoSBE and Cluster. Persist, its derive
  crate, benchmarks, and samples remain unpublished.
- [ ] Add a root changelog and release check that packages and dry-runs both
  publishable crates.

## Acceptance criteria

Text-specific acceptance:

- Valid UTF-8 and ASCII return a borrowed `&str` and the correct next stage.
- Invalid UTF-8 and non-ASCII data return field-specific errors.
- Binary variable data has no generated `into_*_as_str` method.
- Text and binary variable data follow the same rules inside group entries.
- ASCII encoder rejection occurs before tail bytes are modified.
- Cluster event views borrow strings; credentials and payloads borrow bytes.
- Borrowed text decoding performs zero allocations.

Release commands:

```sh
cargo fmt --all -- --check
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
cargo test -p ergo-sbe --all-features -- --test-threads=1
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
cargo test -p ergo-aeron-cluster --all-targets
cargo test -p ergo-clickhouse-persist --all-features
cargo bench -p ergosbe-benchmarks --no-run
(cd samples/advanced-bitget && cargo check --all-targets)
(cd samples/cluster-ha-orderbook && cargo check --all-targets)
cargo package -p ergo-sbe --allow-dirty
cargo package -p ergo-aeron-cluster --allow-dirty
cargo publish -p ergo-sbe --dry-run --allow-dirty
cargo publish -p ergo-aeron-cluster --dry-run --allow-dirty
```

Before release, inspect both package file lists and confirm that they contain no
historical plans, fixture inventories, Java harness, RFQ/auction material, or
reference codecs. Preserve the existing dirty `simple-binary-encoding` submodule
untouched.

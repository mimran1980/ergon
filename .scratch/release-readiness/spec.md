# ergon Release-Readiness Implementation Plan

This is the release-readiness effort's design record, backlog, and implementation
handoff. It is stored as the effort's local-tracker spec. Historical todo files,
phase plans, and completion reports were removed because their checked boxes did
not reliably describe the implementation.

Do not mark a task complete because generated source contains a method name. A task
is complete only when its behavioural, compile-fail, allocation, performance, and
release acceptance criteria pass where applicable.

## Project posture

- **ergon** is the primary project: an experimental, opinionated Rust SBE
  generator used to explore safer and faster generated interfaces.
- **Ergo Aeron Cluster** is a hobby experiment. Do not use it in production. The
  preferred long-term solution is official Aeron Cluster C client bindings with
  support in rusteron.
- **Persist** and **Samples** are unpublished, low-quality laboratories used to
  exercise ergon, domain objects, and transport integrations. They are not
  reference implementations.
- ergo-sbe and ergo-aeron-cluster may eventually be published to crates.io as
  prototype `0.x` crates. Breaking interface changes are acceptable before the
  first release.
- Official SBE wire compatibility comes first. Maintained hot paths must be at
  least as fast as the equivalent Aeron SBE path and must not allocate.

## Verified baseline and corrected status

The following observations were verified during the 2026-07-21 review. They are
baseline facts, not completed implementation tasks:

- ergo-sbe tests pass with all features.
- ergo-sbe and ergo-aeron-cluster pass their strict Clippy commands.
- Ergo Aeron Cluster's 75 library tests pass.
- Persist tests pass with all features.
- Benchmark targets compile.
- Workspace formatting does not pass.
- `samples/exchange-example` does not compile against the current converter
  interface.
- `samples/cluster-ha-orderbook` checks with warnings that fail under strict
  Clippy because generated version-zero comparisons are useless.
- The ergon and Cluster package file lists contain substantial test, fixture,
  reference-codec, application-protocol, and harness material.
- The first text-variable-data pass preserves `characterEncoding` and adds an
  ordered UTF-8 accessor, but the non-consuming accessor is still emitted for
  binary fields, ASCII is not validated, encoder string methods are absent, and
  owned domains do not yet distinguish `String` from `Vec<u8>`.
- Cluster's generated codec namespace is still public: hiding `pub mod codecs`
  from rustdoc (or placing the same types under a public `proto` re-export) does
  not remove those types from the crate interface. Its connect poller still uses
  lossy text and drops decode errors through `Option`.
- The latest raw fixed-field change creates a consuming writer without generated
  setters or a validated completion transition; optional `None` still skips the
  write instead of writing the schema null sentinel. Checked-in golden output
  also still contains version-zero comparisons.

Areas previously marked complete must be treated as follows:

| Area | Correct status | Evidence still required |
|---|---|---|
| Converter registry | Partial | All documented selectors, validation, precedence, collisions, and all emission sites |
| Latest-version safe encoder | Incomplete | Required-field proof, null writes, group-entry proof, and compile-fail ordering |
| Domain mapper | Not implemented | Configured types in generated domains and lossless fallible recursion |
| Composite symmetry | Partial | Named values, versioned members, direct encoders, and nested symmetry |
| Text variable data | Partial | Gate every accessor by schema metadata, validate ASCII, add encoder/domain support, and prove strict errors |
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
types are examples supplied by applications, not dependencies known by ergon.

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

### 5. Downstream-proven ergon conveniences

Promote a helper into ergon only when it expresses SBE mechanics, removes the
same boilerplate from at least two consumers, and can remain allocation-free on
the hot path. The public interface should use inherent methods and concrete
errors; do not introduce a generic transport trait until multiple independent
transports need one.

#### Exact owned-message encoding

Every generated owned latest-version message exposes:

```rust
let len = value.encoded_length_with_header()?;
let encoded = value.encode_into(&mut destination[..len])?;
```

`encoded_length_with_header` validates length and count bounds and recursively
measures nested groups, composites, and variable data without allocating.
`encode_into` uses the safe ordered encoder, includes the message header, and
returns only the exact encoded prefix as `&[u8]`. It never allocates a `Vec` or
silently truncates a count. Existing group-entry domain values retain their
`encode_into(&mut EntryEncoder)` method.

This is the reusable part of Persist's `compute_encoded_length`/`record_into`
shape and lets Cluster and samples encode directly into caller-owned buffers or
Aeron claims. Persist's runtime heterogeneous row validation remains
Persist-specific.

#### One header-inspection interface

Replace `peek_header`, `peek_template_id`, `peek_for_schema`, and the free
`schema_id_from_header` function with one checked operation:

```rust
let header = MessageHeader::try_from_prefix(frame)?;
let template_id = header.template_id();
let schema_id = header.schema_id();
```

The returned fixed-size value also exposes block length and version. It performs
no allocation, reports a short header as a typed decode error, and respects the
schema byte order and header layout. Consumers compare its values with generated
constants instead of reading byte offsets or collapsing malformed and
wrong-schema frames into the same `None`.

#### One runtime for multiple schemas

Add an explicit `with_shared_runtime("sbe_rt")` generation option.
`generate_multi` then emits one `sbe_rt.rs` module and makes each otherwise
independent schema module re-export that runtime. This is separate from
`with_shared_module`, which shares schema types. Reject output-name collisions
and incompatible per-schema runtime configuration at generation time.

Keep `with_external_sbe_rt(path)` for a runtime owned by another module or crate.
Remove `enable_error_from_impls`: it stringifies typed errors and assumes an
application `From<String>` implementation. With one runtime, downstream errors
can transparently wrap the same concrete `sbe_rt::DecodeError` and
`sbe_rt::EncodeError`.

#### Adopt before adding

Persist, Cluster, and Samples should use existing generated
`into_*_as_str`, `into_*_as_message`, `payload_with`, `ENCODED_LENGTH`,
`after_this_message`, ordered tail stages, and conversion/domain APIs wherever
their schemas support them. Delete local aliases and byte-offset parsing rather
than generating a second spelling for the same operation. Domain group decoding
preallocates from the declared entry count and propagates every entry error.

Do not promote ClickHouse type tags, SQL formatting, symbol-table layout,
registry or retry policy, Aeron URI/C-string handling, endpoint selection,
publication status, claims, reconnect policy, fragment assembly, order books,
exchange parsing, or market decimal types. Those are persistence, transport, or
application policy rather than SBE mechanics.

### 6. Ergo Aeron Cluster interface

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

## Execution order

Another agent should take the unchecked items in this dependency order:

1. Add the contract and compile-fail tests in A without weakening existing
   assertions.
2. Build the resolved conversion/text model, shared runtime, and single header
   interface before changing downstream crates.
3. Finish latest-version ordered encoding, composite symmetry, fallible domains,
   and exact caller-buffer encoding; regenerate and compile every golden codec.
4. Migrate Persist and Samples to the completed APIs, deleting only duplicated
   helpers and keeping their application policy local.
5. Finish Cluster privacy, error propagation, reconnect, and allocation-free
   offer work against the stable ergon interface.
6. Run equal-work performance gates, then complete documentation, package-file,
   and crates.io dry-run gates. Do not mark a phase complete while a listed
   acceptance command fails.

## Implementation backlog

### A. Lock the contracts with tests

- [x] Add compile-fail tests proving required latest-version fixed fields cannot
  be omitted and tail methods are unavailable before fixed completion.
- [x] Add behavioural tests proving optional `None` writes the schema null value
  even when wrapping a buffer containing non-zero bytes.
- [x] Cover the same fixed-field proof for fixed-only and tailed group entries.
- [x] Cover selector precedence, every eligible wire kind, nested paths,
  duplicates, conflicts, invalid Rust types, unmatched selectors, and method
  collisions.
- [x] Cover valid and invalid UTF-8 and ASCII, binary fields without string
  methods, text fields inside groups, and encoder validation before writes.
- [x] Cover acting-version composite members, nested composite encoding, domain
  conversion errors, malformed groups, and malformed variable data.
- [x] Prove owned-message measured length equals the exact prefix returned by
  `encode_into` for fixed messages, nested groups, composites, text, and binary
  variable data; cover overflow, count, and short-buffer errors.
- [x] Cover the single header parser for short input, both byte orders, custom
  header layouts, and all four standard header values. Add compile-fail coverage
  proving the retired peek/free-function interfaces are absent.
- [x] Generate two independent schemas with one shared runtime and prove they
  use identical error and conversion-trait types while emitting each runtime
  item exactly once.
- [x] Cover Cluster listener errors, invalid text, session filtering, keep-alive
  failures, callback panics, and reconnect rollback.

### B. Complete ergon generation

- [x] Build the resolved conversion and character-encoding model once after
  schema normalization and use it from every emitter.
- [x] Remove decimal-specific configuration and generated traits, repeated
  selector scans, unused generation helpers, stale comments, and silent `u8`
  fallbacks.
- [x] Emit the complete generic conversion interface for primitives, aliases,
  enums, sets, arrays, composites, messages, and recursive group entries.
- [x] Emit schema-driven raw and text variable-data methods with strict,
  field-aware errors and no lossy helpers.
- [x] Complete the latest-version fixed-field and ordered-tail state machine for
  messages and group entries.
- [x] Complete owned/flyweight/direct-encoder composite symmetry.
- [x] Complete recursive fallible domain decoding and encoding.
- [x] Add allocation-free `encoded_length_with_header` and `encode_into`
  inherent methods to owned latest-version messages. Preallocate decoded domain
  group vectors from their exact entry counts and preserve every conversion
  error.
- [x] Consolidate all header peeking into
  `MessageHeader::try_from_prefix`; remove raw-offset and overlapping Option
  helpers after migrating consumers.
- [x] Add explicit shared-runtime output for `generate_multi`, independent of
  shared schema types. Retain external-runtime support and remove stringifying
  generated application-error conversions.
- [x] Remove generated comparisons such as `acting_version < 0`.and regenerate
  every checked-in golden codec before treating the warning as fixed.

### C. Harden Ergo Aeron Cluster

- [x] Make protocol codecs and internal modules private and expose only the
  high-level client interface; a doc-hidden public re-export does not satisfy
  this requirement.
- [x] Change textual high-level views and callbacks to borrowed `&str`, while
  preserving bytes for binary protocol fields.
- [x] Propagate decoding, text, listener, polling, keep-alive, and controlled
  callback errors through `ClusterResult`.
- [x] Make leader reconnect state replacement atomic and recreate both fragment
  assemblers.
- [x] Replace the allocating high-level offer buffer with stack header plus
  `offer_parts`.
- [x] Remove shallow public CString helpers, RFQ/auction protocols, their codecs,
  schemas, examples, and reference-only public exports.
- [x] Retain and simplify only connect/echo, controlled-polling, and failover
  examples.
- [x] Replace manual header offsets and overlapping peek helpers with
  `MessageHeader::try_from_prefix`, and use generated exact lengths, completed
  encoded slices, and nested-message helpers throughout Cluster.

### D. Keep laboratories honest

- [x] Update Persist to use schema-declared text accessors for table names,
  metadata, column names, and string values, while keeping the
  application-defined symbol table binary. Generate its v1/v2 schemas against
  one shared runtime.
- [x] Retain `DynamicRecorder`'s runtime row validation and ClickHouse encoding
  policy in Persist, but remove wrapper code duplicated by generated exact-length
  and caller-buffer encoding APIs.
- [x] Update `exchange-example` and `cluster-ha-orderbook` to compile against the
  final interface and use `Result`/`?` instead of avoidable unwraps.
- [x] Replace local `WireDec`/`WireDecimal`, manual UTF-8 conversion, header-byte
  reads, and nested payload wrappers with configured conversions, generated
  composite/domain values, schema text methods, and existing nested-message
  helpers where applicable.
- [x] Use domain objects where they make the laboratories clearer; retain raw
  flyweights where allocation-free behaviour is what the test exercises.
- [x] Do not describe either sample as an example to copy or a reference
  implementation.
- [x] Remove sample and Persist code that no longer exercises a unique product
  interface.

### E. Performance evidence

- [x] Add equal-work Criterion cases for raw setters versus fixed structs,
  primitive and composite converters, byte versus borrowed-string variable data,
  domain mapping, owned `encode_into`, header inspection, ordered group entries,
  shared versus duplicated runtime generation, and Cluster `offer_parts`.
- [x] Add allocation assertions for fixed encoding, converter helpers, composite
  flyweights, borrowed text, owned measured encoding, fixed-width domain round
  trips, header inspection, and Cluster offer.
- [x] Compare generated direct access, safe fixed access, and the maintained Aeron
  reference using byte-identical work.
- [x] Run three benchmark sessions. No maintained runtime median may regress by
  more than 3%; generator time may not regress by more than 5%. Investigate and
  fix any larger regression before marking this section complete.

### F. Documentation and packaging

- [x] Keep this release-readiness effort in one spec; do not recreate per-task
  Markdown todos or archived plan trees.
- [x] Keep the root and crate READMEs aligned with the four project postures and
  clearly separate current behaviour from unfinished design.
- [x] Add a documentation hygiene gate that rejects new todo directories,
  `*-goal.md` files, archived plan trees, or a second active tracker.
- [x] Repoint the two Rust source-documentation references from the temporary
  `sbe/design/DECISIONS.md` compatibility pointer to this file, then delete the
  pointer without changing the canonical design location.
- [x] Restrict the ergon package to its manifest, README, and required source.
- [x] Restrict the Cluster package to its manifest, README, build script,
  required Aeron schemas, source, and three supported examples.
- [x] Move Java harness support into the unpublished laboratory area and remove
  tests, reference codecs, RFQ schemas, and application protocols from the
  published Cluster package.
- [x] Complete crates.io metadata for ergon and Cluster. Persist, its derive
  crate, benchmarks, and samples remain unpublished.
- [x] Add a root changelog and release check that packages and dry-runs both
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
- Owned message sizing and caller-buffer encoding perform zero allocations and
  return an exact header-inclusive prefix.
- Multi-schema generation can share one runtime without forcing schemas to share
  wire types.

Release commands:

```sh
cargo fmt --all -- --check
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
cargo test -p ergo-sbe --all-features -- --test-threads=1
cargo clippy -p ergo-aeron-cluster --all-targets -- -D warnings
cargo test -p ergo-aeron-cluster --all-targets
cargo test -p ergo-clickhouse-persist --all-features
cargo bench -p ergo-sbe-benchmarks --no-run
(cd samples/exchange-example && cargo check --all-targets)
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

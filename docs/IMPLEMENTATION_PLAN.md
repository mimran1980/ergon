# ErgoSBE Pre-Release Reset Implementation Plan

> **For agentic workers:** Use `superpowers:subagent-driven-development` or
> `superpowers:executing-plans`, track these checkboxes, work test-first for
> behavior changes, and verify before marking tasks complete.

**Goal:** Prepare ErgoSBE and Ergo Aeron Cluster as honest, packageable
prototype crates while making the generated SBE interface easier and safer by
default without regressing raw-path latency, throughput, or allocation behavior.

**Architecture:** ErgoSBE is the primary opinionated SBE API-research project.
Ergo Aeron Cluster is a high-level hobby client using private
ErgoSBE-generated protocol codecs. Persist and Samples remain unpublished
API-testing labs and are excluded from release gates. ErgoSBE encoders target
only the latest schema version: complete fixed values protect required-field
initialization, composite value/flyweight pairs cover owned and zero-copy use,
and consuming stages enforce the only physical ordering constraint—groups and
var-data tails.

**Tech stack:** Rust 1.95, edition 2024, generated allocation-free SBE
flyweights and owned values, Criterion same-session benchmarks, and the
workspace's existing strict Clippy/test/release gates.

**Baseline:** Review range `bd3f7ce...946ae3a`. On 2026-07-20,
`cargo test -p ergo-sbe --all-features -- --test-threads=1` and
`cargo test -p ergo-aeron-cluster --lib` passed. Preserve the pre-existing
dirty `simple-binary-encoding` submodule.

## Public API decisions

### ErgoSBE

- `Generator::generate` and `Generator::generate_multi` return
  `Result<GeneratedModuleSet, GenerateError>`; remove `try_generate`.
- Make `GenerationConfig` fields private. Retain builders for domain objects,
  shared modules, external runtime, type-directed conversions, concrete domain
  type mappings, error conversions, and benchmark-only unchecked companions.
- Remove `CompatibilityMode`, `checked_accessors`, `SchemaSource`,
  `Schema::new`, `Generator::config`, and public `GeneratedModuleSet::push`.
- Generated domain mapping uses `TryFrom<Decoder>` and never drops malformed
  groups or var-data. Unmapped messages retain the normal wire errors; mapped
  messages use a generated, typed `<Message>DomainError`.

### Type-directed conversion seam

Remove the decimal-specific `decimal_composites` configuration,
`enable_decimal_converters`, generated `SbeDecimal` trait, and all generator
knowledge of `mantissa`, `exponent`, or `rust_decimal`. Decimal, Boolean,
timestamp, enum, and domain-newtype conversions all use the same interface.

When at least one conversion is enabled, the generated schema runtime
(`sbe_rt`) exposes two small traits and the generated module re-exports them:

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

Generate the traits once in `sbe_rt`, rather than once per message or codec.
When `external_sbe_rt` is configured, use and re-export that runtime's trait
definitions instead of generating a duplicate copy. Because the wire type is
local to the generated schema, an application may implement the traits for an
external type such as `rust_decimal::Decimal`, `time::OffsetDateTime`,
`chrono::DateTime<Utc>`, or its own newtype without violating Rust's orphan
rules. ErgoSBE does not depend on any conversion library and does not know the
target type's layout.

For a conversion-enabled fixed field named `price`, generate additive methods
beside the canonical wire accessor/setter:

```rust
#[must_use]
pub fn price_as<T>(&self) -> Result<T, T::Error>
where
    T: TryFromSbe<Decimal>;

pub fn price_from<T>(&mut self, value: &T) -> Result<&mut Self, T::Error>
where
    T: TryToSbe<Decimal>;
```

Encoder methods on consuming type-state stages return the exact next stage
rather than `&mut Self`; the example above represents a non-consuming fixed
field. Calls normally rely on type inference:

```rust
let price: rust_decimal::Decimal = decoder.price_as()?;
encoder.price_from(&price)?;

let timestamp: time::OffsetDateTime = decoder.timestamp_as()?;
encoder.timestamp_from(&timestamp)?;
```

Optional and versioned fixed fields compose with the same API rather than
inventing another converter shape:

```rust
#[must_use]
pub fn timestamp_as<T>(&self) -> Result<Option<T>, T::Error>
where
    T: TryFromSbe<u64>;
```

Use `#[must_use]` on decoder conversion results and on consuming encoder
stages. Conversion methods return the concrete associated error directly; do
not box it or add `Send`, `Sync`, or `'static` bounds that static dispatch does
not require.

The raw wire method names, value types, byte semantics, and generated code
remain available. Once the safe fixed-value encoder design below lands,
message-level fixed setters live on the explicit raw fixed writer rather than
on the safe initial encoder stage:

```rust
let raw_price: Decimal = decoder.price();
encoder.price(raw_price);
```

This is a compile-time seam: no runtime registry, hash lookup, trait object,
dynamic dispatch, reflection, or converter allocation is permitted. Generated
calls are statically dispatched and marked `#[inline]`; unused generic methods
are not monomorphized. The raw flyweight path contains no conversion branch and
must remain byte-for-byte and benchmark equivalent when conversion support is
unused.

#### Enabling conversion methods

Boolean conversion is enabled automatically for SBE Boolean enums. ErgoSBE
implements `TryFromSbe<BooleanType> for bool` and
`TryToSbe<BooleanType> for bool` internally, and existing convenience methods
such as `available_bool()` delegate through that implementation. The public
Boolean convenience methods remain infallible and compile to the same direct
comparison/write as today. Preserve the current compatibility semantics
exactly: every non-zero wire value, including the enum's null value, reads as
`true`; `false` writes the schema's false value and `true` writes its true
value. Any future strict/null-aware Boolean interpretation must be a separate,
explicit API change.

Other fixed-value conversions are enabled by a selector only; the generator
does not need an adapter path, method suffix, target type, or error type:

```rust
let config = GenerationConfig::new("messages")
    .with_conversion(ConversionSelector::semantic_type("UTCTimestamp"))
    .with_conversion(ConversionSelector::named_type("Decimal"));
```

Selectors have deterministic precedence:

1. Exact `Message.field` path.
2. SBE `semanticType`.
3. Named primitive alias, enum, set, composite, or fixed array type.

Duplicate/conflicting selectors, selectors matching no field, and generated
method collisions are generation errors. A concrete domain mapping implicitly
enables conversion methods for the same selector, so normal users do not need
both calls.

The first version deliberately supports fixed primitives, aliases, enums,
sets, composites, fixed arrays, and fixed fields within repeating-group
entries. Whole groups and var-data are excluded: their borrowing/ownership
requirements would force a substantially more complicated trait. Their
existing raw and owned-domain APIs remain available. Add a separate var-data
conversion seam only after a measured use case justifies it.

#### Domain-object integration

Domain objects need a concrete Rust field type, so generation accepts a small
type map—not a converter registry:

```rust
let config = GenerationConfig::new("messages")
    .with_domain_objects()
    .with_domain_type(
        ConversionSelector::semantic_type("UTCTimestamp"),
        "time::OffsetDateTime",
    )
    .with_domain_type(
        ConversionSelector::named_type("Decimal"),
        "rust_decimal::Decimal",
    );
```

Use `ConversionSelector` in both public builders; do not introduce a parallel
`DomainTypeMapping` selector hierarchy. `with_domain_type(selector, rust_type)`
contains only the selector and Rust type path, and implicitly enables
conversion methods for that selector. It does not implicitly enable all domain
objects: callers must still opt in with `with_domain_objects()`. The map
contains no conversion function, adapter type, method suffix, global error, or
runtime object. Generated domain `TryFrom<Decoder>` calls that type's
`TryFromSbe` implementation; domain `encode` calls its `TryToSbe`
implementation. Optional and versioned fields remain `Option<T>`, and
group-entry domain objects use the same mapping recursively.

For each mapped message, generate one concrete `<Message>DomainError` enum. It
wraps wire `DecodeError`/`EncodeError` plus the exact associated error type for
each distinct mapping. This preserves typed errors without requiring callers to
invent a registry-wide error or use `Box<dyn Error>`. Domain value types must
implement the derives enabled for the domain object (`Debug`, `Clone`,
`PartialEq`, and optional serde traits).

If a schema has no Boolean fields, conversion selectors, or domain mappings,
no conversion traits, methods, domain-error variants, dependencies, or runtime
behavior are emitted.

#### Fallible text access

Remove generated lossy text helpers that hide malformed UTF-8 behind a
sentinel or replacement value. Keep the explicit fallible accessor as the
generated default:

```rust
let symbol: &str = decoder.symbol_as_str()?;
```

Migrate repository call sites to propagate the error with `?`. A consumer that
deliberately wants lossy decoding must say so at its application boundary with
`String::from_utf8_lossy`; ErgoSBE must not imply that policy. Because these are
prototype crates preparing for publication, remove the shallow lossy methods
instead of carrying a long deprecation cycle.

### Safe latest-version encoding

Encoders support only the latest schema version. They always write the
generated `SCHEMA_VERSION`, latest message `BLOCK_LENGTH`, and latest group
entry block lengths. Do not add an acting-version parameter, runtime version
switch, older-version encoder, or version-generic encoder type. Decoders remain
acting-version aware and continue to read older wire versions.

Fixed fields have schema-defined offsets, so their call order has no effect on
the wire. Enforcing a type-state stage for every fixed field would add a large,
shallow interface without improving correctness. The safety requirement is
that every required latest-version fixed field receives a value before a group
or var-data tail begins.

Generate one named input value per message containing every non-constant fixed
field in the latest schema:

```rust
pub struct OrderFixedFields {
    pub sequence: u64,
    pub timestamp: time::OffsetDateTime,
    pub instrument_id: u64,
    pub price: rust_decimal::Decimal,
    pub quantity: u64,
    pub flags: OrderFlags,
}
```

`OrderFixedFields` is concrete and non-generic. A configured concrete domain
mapping changes the corresponding member to that mapped Rust type; otherwise
the member uses its generated wire/value type. Required fields are ordinary
members, SBE optional fields use `Option<T>`, constants are omitted, and all
fields introduced by `sinceVersion` are present because encoding is
latest-version only. Derive `Debug`, `Clone`, and `PartialEq`; derive `Copy`
only when every member is `Copy`. Do not implement `Default` when any required
field lacks a schema-defined default/null value, because struct-literal
completeness is the compile-time required-field check.

The initial encoder accepts the complete fixed value and consumes itself:

```rust
let encoder = OrderEncoder::wrap_and_apply_header(&mut buffer, 0)?;
let encoder = encoder.fixed(&OrderFixedFields {
    sequence: 42,
    timestamp,
    instrument_id: 7,
    price,
    quantity: 100,
    flags,
})?;
let encoder = encoder.legs(2, |legs| {
    // Entries remain ordered by the generated group stages.
})?;
let complete = encoder.symbol(b"EURUSD")?;
```

`fixed(&OrderFixedFields)` writes the fixed block at constant offsets and
returns the exact first-tail stage, or `OrderComplete` for a fixed-only
message. It returns a generated concrete `OrderFixedError` wrapping only the
associated errors for mapped conversions used by the fixed value. After
`wrap` has reserved the fixed block, no per-field buffer-length checks are
needed. Convert fallible mapped members before exposing the next stage; on
error the consumed encoder is not returned and the partially written buffer is
not a valid message.

Groups and var-data keep the current consuming type-state interface. They are
the only message elements whose physical order changes the wire cursor, so the
compiler must continue to make later tails unavailable until earlier tails are
finished. Complete-message byte and length access remains available only on
the terminal stage.

Keep individual fixed-field setters as an explicitly low-level alternative,
not the documented default. Move them behind a generated raw fixed writer:

```rust
let mut fixed = encoder.raw_fixed();
fixed.sequence(42).instrument_id(7).quantity(100);
let encoder = fixed.finish_unchecked();
```

`finish_unchecked` is safe Rust but deliberately does not prove semantic
required-field initialization. Its name must make that trade-off visible. It
performs no bitmask writes, runtime required-field scan, allocation, or branch.
Tail methods are not available on the raw writer, so even this expert path
cannot violate group/var-data ordering. Remove the shallow `try_fixed` closure
once all call sites use either the complete fixed value or the explicit raw
writer.

### Composite value and flyweight symmetry

Generate two deliberate representations for each fixed-width composite:

1. A named, owned latest-version value such as `Engine`.
2. Zero-copy `EngineDecoder<'a>` and direct-write `EngineEncoder<'a>`
   flyweights over a message buffer.

The owned value uses named public members rather than an opaque public byte
array or a positional constructor:

```rust
pub struct Engine {
    pub capacity: u16,
    pub num_cylinders: u8,
    pub manufacturer_code: [u8; 3],
    pub efficiency: i8,
    pub booster_enabled: bool,
    pub booster: Booster,
}
```

Constant composite members remain associated constants/accessors and are not
stored. Optional members use `Option<T>`. Apply concrete domain mappings and
automatic Boolean conversion to the owned value exactly as for message fixed
values. Do not expose wire padding or Rust layout as part of the interface;
encode/decode members at generated constant offsets so endianness and padding
remain correct.

Composite message fields have symmetric methods. Decode methods live on the
message decoder; direct write methods live on the explicit raw fixed writer,
while the safe `<Message>FixedFields` path stores the complete owned composite:

```rust
let view: EngineDecoder<'_> = decoder.engine();
let capacity = view.capacity();
let value: Engine = decoder.engine_value()?;

fixed.engine(&value);
let mut view = fixed.engine_mut();
view.capacity(2_000).num_cylinders(4);
```

Use `*_value`, not `*_as_struct`, for owned reads. The complete owned value is
the safe/default write path; `*_mut` is the explicit flyweight path for direct
member writes without an intermediate value. Nested composites expose the same
value/view pair recursively.

Every composite decoder carries the parent `acting_version`. A member added in
a later schema version returns `Option<T>` from the flyweight. Converting a
flyweight from an older frame into the complete latest owned value returns
`DecodeError::FieldNotInVersion` when a required latest member is absent; it
must not invent a zero, null, or default value. A composite field added in a
later version retains the outer `Option`, so its owned convenience method is
`Result<Option<Composite>, DecodeError>`; a version-zero field uses
`Result<Composite, DecodeError>`.

For conversion-enabled composite fields, decoder conversion methods pass the
zero-copy composite decoder to the trait:

```rust
impl<'wire> TryFromSbe<DecimalDecoder<'wire>> for rust_decimal::Decimal {
    type Error = DecimalError;

    fn try_from_sbe(wire: DecimalDecoder<'wire>) -> Result<Self, Self::Error> {
        // Read members directly from the message buffer.
    }
}

impl TryToSbe<Decimal> for rust_decimal::Decimal {
    type Error = DecimalError;

    fn try_to_sbe(&self) -> Result<Decimal, Self::Error> {
        // Return the complete named latest-version composite value.
    }
}
```

Primitive, enum, set, and fixed-array conversions continue to receive their
small wire value by copy. Composite decode conversion must not eagerly copy the
whole composite before calling `TryFromSbe`. All paths remain statically
dispatched, allocation-free, and `#[inline]`.

### Ergo Aeron Cluster

- Export only the high-level client/session/claim API, egress listeners,
  credentials, states, errors, publication failures, and required event enums.
- Keep generated codecs, URI construction, endpoint parsing, transport,
  polling, and connection machinery private.
- Listener callbacks return `ClusterResult`; panics at Aeron callbacks become
  `ClusterError::ListenerPanicked`.
- Remove public C-string helpers. Use `c"..."` for static private strings and
  private `cformat!` conversion for validated dynamic strings.

## Conversion adoption boundaries

### ErgoSBE core and generated API

- In `sbe/src/config.rs`, replace the decimal flags and any parallel domain
  mapping selector with `ConversionSelector` plus
  `with_conversion(selector)` and `with_domain_type(selector, rust_type)`.
- In `sbe/src/codegen.rs`, emit the shared runtime traits, additive
  `*_as`/`*_from` methods, optional/versioned composition, concrete domain
  errors, Boolean delegation, and no conversion code when configuration is
  empty. Remove every `SbeDecimal`, mantissa, exponent, and `rust_decimal`
  branch.
- In `sbe/src/lib.rs`, export only the small configuration surface needed by
  consumers. Generated modules re-export the runtime traits; the crate does not
  acquire a conversion-library dependency.
- Use `sbe/tests/fixtures/schemas/decimal-converter-schema.xml`,
  `sbe/tests/fixtures/schemas/bool-semantic-schema.xml`, and focused new
  fixtures only where existing schemas cannot prove selector precedence,
  optional/versioned values, fixed arrays, or group-entry fields.
- Migrate `sbe/tests/baseline_test.rs` and
  `sbe/tests/comprehensive_test.rs` from hard-coded decimal/Boolean scaffolding
  to consumer implementations of `TryFromSbe`/`TryToSbe`. Keep raw-access
  assertions next to converted-access assertions so byte identity is visible.

### Persist laboratory

- In `persist/build.rs`, replace decimal-specific generation flags with
  `ConversionSelector::named_type("Decimal")` and the concrete Persist domain
  type mapping where generated domain objects use decimal values.
- In `persist/src/sbe.rs` and the owned round-trip tests, use generated domain
  `TryFrom`/encode plus the same conversion traits. Keep manual wire access
  wherever it is testing raw SBE behavior.
- Treat this as an internal integration proof only. Do not expose Persist APIs
  from ErgoSBE, add Persist to product release gates, or present Persist as
  reference-quality usage.

### Samples laboratory

- In `samples/advanced-bitget/src/decimal.rs`, replace the macro implementing
  generated `SbeDecimal` with consumer implementations of
  `TryFromSbe<Decimal>` and `TryToSbe<Decimal>` for
  `rust_decimal::Decimal`.
- Update `samples/advanced-bitget/build.rs` and
  `samples/advanced-bitget/src/lib.rs` to select Decimal conversion and use
  `price_as`/`price_from` where it makes the test-bed flow clearer.
- Update `samples/cluster-ha-orderbook/build.rs` only for application-schema
  fields with a genuine domain representation. Do not introduce conversion
  solely to demonstrate the feature.
- Keep `samples/README.md` explicit that samples are low-quality API
  experiments, are unpublished, and are not reference implementations.

### Ergo Aeron Cluster

- In `cluster/build.rs` and `cluster/src/codecs`, keep Aeron session and cluster
  protocol codecs on their raw primitive/composite API. Protocol timestamps,
  IDs, enum values, and opaque application payloads must remain visibly tied
  to the pinned Aeron schema unless that schema explicitly supplies a semantic
  type and a real client-facing domain use case exists.
- Reuse automatic Boolean conversion only if a generated Aeron field has SBE
  Boolean semantics. Do not create Cluster-specific adapters or leak generated
  conversion traits through the public high-level client API.
- Add a regression proving empty conversion configuration leaves generated
  Cluster protocol source and benchmark behavior unchanged.

### Benchmarks and documentation

- In `ergosbe-benchmarks`, compare `*_as`/`*_from` against equivalent
  hand-written Boolean and decimal/timestamp conversion with equal work.
  Require parity within the repository's existing noise tolerance; do not
  compare conversion against a raw accessor that performs less work.
- Compare `MessageEncoder::fixed(&MessageFixedFields)` against the current
  hand-written sequence of fixed setters using identical values and output
  bytes. Compare owned composite encode/decode against manual member-by-member
  materialization, and compare composite flyweights against the existing raw
  flyweight path. Benchmark construction inside the timed loop so stack-value
  creation cannot be hidden from the result.
- Preserve the existing raw-codec parity benchmark and prove an empty
  conversion configuration adds no branch, allocation, or measurable cost.
- In `sbe/GUIDE.md` and `sbe/DESIGN.md`, explain the three deliberate layers:
  raw wire access, opt-in typed conversion, and concrete generated domain
  objects. In `cluster/README.md`, state that Aeron protocol codecs remain raw
  by design. Persist and Samples documentation describes their migration only
  as internal API testing.

## Tasks

- [x] **1. Lock the conversion contract with failing tests:** in
  `sbe/tests/baseline_test.rs`, `sbe/tests/comprehensive_test.rs`, and focused
  fixtures under `sbe/tests/fixtures/schemas`, specify the exact generated
  signatures, selector precedence, selector/method-collision diagnostics,
  optional/versioned behavior, group-entry behavior, Boolean compatibility,
  domain errors, external runtime reuse, and absence of conversion output when
  disabled. Add compile-fail coverage for a missing trait implementation and
  bad Rust type path. Run the focused `ergo-sbe` tests and confirm each new
  assertion fails for the intended missing API before generator work begins.
- [x] **2. Implement the ErgoSBE conversion seam:** update
  `sbe/src/config.rs`, `sbe/src/codegen.rs`, and `sbe/src/lib.rs` to implement
  the shared `sbe_rt` traits, the single `ConversionSelector` vocabulary,
  `with_domain_type(selector, rust_type)`, additive `*_as`/`*_from` methods,
  concrete domain errors, and optional/versioned composition. Remove
  `decimal_composites`, `enable_decimal_converters`, `SbeDecimal`, and all
  decimal-layout/library knowledge. Route existing Boolean helpers through the
  traits without changing their wire semantics. Remove generated lossy string
  helpers and migrate core call sites to the fallible text accessor with `?`.
  Run the focused tests until they pass, then run the full `ergo-sbe` suite.
- [x] **3. Lock the safe encoder and composite contract with failing tests:**
  extend `sbe/tests/baseline_test.rs`, `sbe/tests/comprehensive_test.rs`,
  `sbe/tests/domain_objects_test.rs`, and
  `sbe/tests/schema_edge_cases_test.rs`. Prove that a generated
  `CarFixedFields` struct literal requires every latest required fixed field,
  constants are absent, optional fields are `Option<T>`, and `Default` is not
  available when it would bypass required-field initialization. Prove
  `CarEncoder::fixed` is the only safe transition to the first tail stage,
  later tails do not compile on the initial encoder or raw writer, and
  `raw_fixed().finish_unchecked()` is the explicit low-level transition. Prove
  the header and block length always use the latest schema version. Add
  byte-for-byte round trips for `engine()`, `engine_value()`, `engine(&value)`,
  and `engine_mut()`, including nested and versioned composite members from
  `sbe/tests/fixtures/schemas/since-version-filter-schema.xml`. Confirm an
  older acting version returns `Option` through the flyweight and
  `FieldNotInVersion` when materializing an incomplete latest owned value.
  Run each focused test target and verify the new assertions fail for the
  intended missing interface before implementation. Use:

  ```bash
  cargo test -p ergo-sbe --test baseline_test fixed_fields -- --test-threads=1
  cargo test -p ergo-sbe --test comprehensive_test composite -- --test-threads=1
  cargo test -p ergo-sbe --test domain_objects_test fixed_fields -- --test-threads=1
  cargo test -p ergo-sbe --test schema_edge_cases_test version -- --test-threads=1
  ```

  Before Task 4, each new filtered test must fail because the planned generated
  type/method or version check is absent—not because its fixture cannot compile
  for an unrelated reason.
- [x] **4. Implement latest-version fixed values and composite symmetry:** `FixedFields`, `fixed()`, `raw_fixed()`, `finish_unchecked()`, composite `*_value` rename, `field_type_ident` helper. All tests pass.
  update `sbe/src/codegen.rs` and the generated public surface in
  `sbe/src/lib.rs`. Generate concrete `<Message>FixedFields` and
  `<Message>FixedError`, consuming `fixed`, `raw_fixed`, and
  `finish_unchecked`; expose tail methods only on the returned post-fixed
  stage. Apply configured domain types and converter errors without runtime
  dispatch. Remove `try_fixed`. Replace opaque/positional composite values with
  named latest-version values; generate acting-version-aware composite
  decoders and direct-write composite encoders; rename owned reads from
  `*_as_struct` to `*_value`; and recursively support nested composites.
  Preserve the existing group/var-data type-state machine and terminal-only
  complete-message access. Regenerate the checked fixture used by tests, then
  run:

  ```bash
  cargo fmt --all -- --check
  cargo test -p ergo-sbe --all-features -- --test-threads=1
  cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
  ```

  Expected: all commands exit zero; the tests added in Task 3 pass without
  weakening their compile-fail or byte-equivalence assertions.
- [x] **5. Add equal-work performance and allocation gates:** update
  `ergosbe-benchmarks/build.rs` and
  `ergosbe-benchmarks/benches/perf_parity_bench.rs` with same-session Criterion
  cases for manual fixed setters versus `fixed(&FixedFields)`, manual owned
  composite materialization versus `*_value`, raw composite flyweight access
  versus the new flyweight, and manual conversion versus `*_as`/`*_from`.
  Every pair must perform identical field reads/writes and produce identical
  bytes; value construction stays inside the timed loop. Extend
  `sbe/tests/allocation_count_test.rs` to require zero allocations for fixed
  values, composite values/views, and conversions. Run the checked and
  `bound-check-disabled` baselines in the same Criterion workflow and apply the
  existing 0.5% regression gate. A measured regression blocks adoption: first
  inspect generated code/assembly, retain the raw path, and do not relax the
  tolerance to make the new interface pass. Use:

  ```bash
  cargo test -p ergo-sbe --test allocation_count_test -- --test-threads=1
  just bench
  ```

  Expected: allocation tests report zero for every new path; both Criterion
  sessions complete; `scripts/check-bench-gate.sh` reports no comparison over
  0.5% and no missing estimate.
- [x] **6. Adopt conversions and fixed values in the unpublished laboratories:** migrate
  `persist/build.rs` and `persist/src/sbe.rs` to selector-based concrete domain
  types, complete fixed values, and owned fallible round trips. Replace the Advanced Bitget
  `SbeDecimal` macro in `samples/advanced-bitget/src/decimal.rs`, update its
  build configuration and call sites, and update the HA order-book build only
  where its application schema has a useful concrete domain type. Propagate
  fallible conversion and text errors with `?`; do not add `unwrap()` or
  `expect()` outside tests. Run Persist and affected sample tests independently
  and keep their READMEs explicit that they are unpublished test beds.
- [x] **7. Prove Cluster remains a raw-protocol consumer:** keep
  `cluster/build.rs` conversion configuration empty for Aeron protocol codecs,
  except automatic schema-defined Boolean convenience where applicable. Add
  source/golden and benchmark regressions showing no conversion branches or
  public conversion types enter `cluster/src/codecs` or the high-level Cluster
  API. Application payloads remain opaque; application-schema conversions stay
  in the unpublished HA sample rather than `ergo-aeron-cluster`.
- [x] **8. Intentional SBE examples:** replace debug generators with one owned
  domain-object example and one explicitly zero-copy flyweight example backed
  by a single regeneration-checked generated fixture. Enable domain objects in
  Persist and test owned round trips without making Persist a reference app.
  The owned example implements conversions for at least two unrelated types
  (for example a decimal and a timestamp/newtype) to prove the mechanism is
  generic rather than a renamed decimal special case. Show the raw and
  converted APIs side by side, demonstrate `FixedFields` plus composite
  value/flyweight symmetry, and use only fallible UTF-8 access.
- [x] **9. Cluster egress hardening:** test and implement filtering for every
  session-bearing event, callback error/panic containment, surfaced decode
  failures, fallible keep-alive, and atomic retryable leader transitions that
  construct a publication plus two assemblers before swapping state.
- [x] **10. High-level cluster surface:** delete shallow URI/idle/decode/session
  wrappers and RFQ/Mark/auction application protocols; retain three distinct
  high-level examples using `offer`, `try_claim`, `ClusterResult`, and `?`.
  Move the Java harness into integration-test support.
- [x] **11. Fragmentation and Cluster performance proofs:** send deterministic 16 KiB
  payloads with MTU 1408; cover regular, controlled, foreign-session,
  callback-error, and leader/image behavior. Hide benchmark internals, retain
  only the six required sbe-tool reference codecs, and restore all maintained
  benchmark comparisons with a 0.5% tolerance and missing-estimate failures.
  Add equal-work manual-versus-generated Boolean and decimal/timestamp
  conversion cases, and retain the raw empty-configuration parity gate.
- [x] **12. Packaging:** make both prototype crates self-contained, vendor the
  pinned Aeron session schema under `cluster/schemas`, add a version to the
  ErgoSBE path dependency, use crate-local READMEs and package allowlists, and
  set Persist plus Persist Derive to `publish = false`.
- [x] **13. Documentation:** keep concise root/product/lab READMEs,
  `sbe/DESIGN.md`, `sbe/GUIDE.md`, `docs/ROADMAP.md`, and
  `docs/PUBLISHING.md`. Delete historical TODO ledgers, goals, plans/specs,
  legacy SBE benches, per-sample READMEs, `package-lock.json`, `bors.toml`, and
  `ci-monitor.sh` after migrating still-relevant facts. Document raw wire,
  opt-in conversion, complete fixed values, composite value/flyweight access,
  and domain-object layers; explain that Cluster intentionally keeps its Aeron
  protocol representation raw and that encoders support only the latest schema
  version.
- [x] **14. Gates and release:** make `just check`/`check-products` product-only,
  add a separate `check-labs`, and make `release-check` verify only the two
  prototype crates. Fix stale CI package/sample names and publish ErgoSBE
  before Cluster; never publish the workspace wholesale.
- [x] **15. Final review:** run formatting, strict Clippy, all product/lab tests,
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

Conversion acceptance additionally requires:

- Compile/run proofs for Boolean, primitive/alias, enum/set, composite, fixed
  array, optional/versioned, and group-entry conversions.
- Decoder, encoder, and domain-object round trips through the same trait
  implementations.
- At least two unrelated target libraries/types with no converter-library
  dependency in `ergo-sbe`.
- Compile-fail or generation-error coverage for missing trait implementations,
  bad selectors/type paths, duplicate mappings, and method collisions.
- Generated-source checks proving no trait objects, dynamic dispatch, or
  runtime registry exists.
- Golden-output and benchmark proof that empty conversion configuration leaves
  raw field names, wire types, byte semantics, and hot-path implementation
  unchanged, and that Boolean conversion is equivalent to the current
  handwritten implementation. The safe encoder redesign may intentionally
  relocate message fixed setters behind `raw_fixed`, but must not alter their
  operation once that writer is selected.

Latest-version encoder and composite acceptance additionally requires:

- Compile-fail proof that omitting a required member from
  `<Message>FixedFields` fails at the struct literal and that a group/var-data
  tail is unavailable before the fixed phase is consumed.
- Compile/run proof that constants are omitted from fixed input values,
  optional fields are `Option<T>`, mapped fields use their concrete configured
  types, and every `sinceVersion` field is required by the latest-version
  encoder.
- Header and group-dimension fixtures proving encoders always write the latest
  schema version and matching latest block lengths, with no encoder acting
  version in generated state or method parameters.
- Byte-identical output from complete fixed values and the existing manual
  fixed setter sequence, plus byte-identical owned-composite and direct
  composite-flyweight writes.
- Older-version decode proofs for a whole composite introduced later and for
  individual members introduced later: flyweights expose absence as `Option`,
  while latest owned-value materialization fails with
  `DecodeError::FieldNotInVersion` rather than inventing data.
- Generated-source checks that tail ordering remains consuming type-state,
  `finish_unchecked` is confined to the explicit raw fixed writer,
  `try_fixed`/`*_as_struct` are absent, and complete-message byte access exists
  only on the terminal encoder stage.
- Allocation-count proof of zero allocations for fixed-value writes,
  composite value/view reads and writes, and converter methods.
- Same-session, equal-work Criterion proof that fixed-value and composite-value
  convenience methods stay within the existing 0.5% regression tolerance of
  their manual equivalents. Preserve the raw methods if optimizer output does
  not meet this gate; never weaken the gate to accept a regression.

After every checkbox is complete, move genuinely unfinished work into
`docs/ROADMAP.md`, update `CHANGELOG.md`, and delete this active plan so it does
not become another historical ledger.

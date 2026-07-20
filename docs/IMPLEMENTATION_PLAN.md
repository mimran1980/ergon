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

The generated runtime exposes two small traits:

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

The traits live in the generated module, so an application may implement them
for an external type such as `rust_decimal::Decimal`, `time::OffsetDateTime`,
`chrono::DateTime<Utc>`, or its own newtype without violating Rust's orphan
rules. ErgoSBE does not depend on any conversion library and does not know the
target type's layout.

For a conversion-enabled fixed field named `price`, generate additive methods
beside the canonical wire accessor/setter:

```rust
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

The wire methods remain available and unchanged:

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
comparison/write as today.

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
        DomainTypeMapping::semantic_type(
            "UTCTimestamp",
            "time::OffsetDateTime",
        ),
    )
    .with_domain_type(
        DomainTypeMapping::named_type(
            "Decimal",
            "rust_decimal::Decimal",
        ),
    );
```

The map contains only a selector and the Rust type path. It contains no
conversion function, adapter type, method suffix, global error, or runtime
object. Generated domain `TryFrom<Decoder>` calls that type's `TryFromSbe`
implementation; domain `encode` calls its `TryToSbe` implementation. Optional
and versioned fields remain `Option<T>`, and group-entry domain objects use the
same mapping recursively.

For each mapped message, generate one concrete `<Message>DomainError` enum. It
wraps wire `DecodeError`/`EncodeError` plus the exact associated error type for
each distinct mapping. This preserves typed errors without requiring callers to
invent a registry-wide error or use `Box<dyn Error>`. Domain value types must
implement the derives enabled for the domain object (`Debug`, `Clone`,
`PartialEq`, and optional serde traits).

If a schema has no Boolean fields, conversion selectors, or domain mappings,
no conversion traits, methods, domain-error variants, dependencies, or runtime
behavior are emitted.

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

- [ ] **1. ErgoSBE fallible API and type-directed conversions:** add failing
  tests;
  implement fallible generation, private builder configuration, the generic
  conversion seam and concrete domain type map described above, fallible domain
  mapping, real issue-schema generation, and correct `sinceVersion = 0`
  emission. Remove all decimal-specific generator branches; implement existing
  Boolean conversion through the same traits; and migrate current decimal
  behavior to consumer trait implementations in tests/Persist. Localize lint
  exemptions and prefer `?` at fallible boundaries.
- [ ] **2. Intentional SBE examples:** replace debug generators with one owned
  domain-object example and one explicitly zero-copy flyweight example backed
  by a single regeneration-checked generated fixture. Enable domain objects in
  Persist and test owned round trips without making Persist a reference app.
  The owned example implements conversions for at least two unrelated types
  (for example a decimal and a timestamp/newtype) to prove the mechanism is
  generic rather than a renamed decimal special case.
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
  the existing wire API and hot path unchanged, and that Boolean conversion is
  equivalent to the current handwritten implementation.

After every checkbox is complete, move genuinely unfinished work into
`docs/ROADMAP.md`, update `CHANGELOG.md`, and delete this active plan so it does
not become another historical ledger.

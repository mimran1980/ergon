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
  shared modules, external runtime, generic converter registration, error
  conversions, and benchmark-only unchecked companions.
- Remove `CompatibilityMode`, `checked_accessors`, `SchemaSource`,
  `Schema::new`, `Generator::config`, and public `GeneratedModuleSet::push`.
- Generated domain mapping uses `TryFrom<Decoder>` and never drops malformed
  groups or var-data. Its error is `DecodeError` without converters and the
  registry-wide conversion error when domain converters are enabled.

### Generic converter registry

Remove the decimal-specific `decimal_composites` configuration,
`enable_decimal_converters`, generated `SbeDecimal` trait, and all generator
knowledge of `mantissa`, `exponent`, or `rust_decimal`. Decimal conversion is
only one consumer-defined use of the generic mechanism.

The public configuration shape is:

```rust
let converters = ConverterRegistry::new("crate::ConversionError")
    .register(
        ConverterRegistration::for_type("Decimal", "crate::DecimalConverter")
            .method_suffix("decimal")
            .use_in_domain_objects(),
    )
    .register(
        ConverterRegistration::for_semantic_type(
            "UTCTimestamp",
            "crate::TimestampConverter",
        )
        .method_suffix("datetime")
        .use_in_domain_objects(),
    );

let config = GenerationConfig::new("messages").with_converters(converters);
```

`ConverterRegistration` supports three selectors, in this precedence order:

1. Exact field path: `Message.field`.
2. SBE `semanticType`.
3. Named SBE type: primitive alias, enum, set, composite, fixed array, or
   var-data encoding.

An exact selector conflict, invalid Rust adapter path, invalid method suffix,
missing selector target, or generated method collision is a generation error.
The registry preserves insertion order for deterministic output, but selector
precedence—not insertion order—chooses the applicable registration.

ErgoSBE emits a generic adapter contract only when at least one converter is
registered:

```rust
pub trait SbeConverter {
    type Value;
    type WireView<'a>
    where
        Self: 'a;
    type WireOwned;

    fn decode<'a>(
        wire: Self::WireView<'a>,
    ) -> Result<Self::Value, ConversionError>;

    fn encode(
        value: &Self::Value,
    ) -> Result<Self::WireOwned, ConversionError>;
}
```

`ConversionError` is the registry-wide error path supplied by the consumer.
It must implement `From<DecodeError>` and `From<EncodeError>`, allowing raw SBE
failures and adapter failures to propagate through one typed domain-mapping
error. The adapter type is user-owned, so consumers can implement the generated
trait without orphan-rule problems. ErgoSBE takes no dependency on conversion
libraries.

For a field named `price` with suffix `decimal`, generate additive methods
alongside the canonical wire accessors:

```rust
decoder.price_as_decimal() -> Result<Adapter::Value, ConversionError>
encoder.price_from_decimal(&Adapter::Value) -> Result<NextStage, ConversionError>
```

The converter methods call the existing wire accessor/setter and then the
registered adapter. They do not replace or slow the normal flyweight API.
Optional and versioned fields preserve `Option`; conversion runs only when the
wire value is present. Var-data converters receive a borrowed slice when
decoding and return an owned wire value when encoding. Fixed primitives,
arrays, enums, sets, and composites use their existing generated wire/value
types. Group-entry fields receive the same converter methods as message fields.

When `.use_in_domain_objects()` is selected, the corresponding domain field is
`<Adapter as SbeConverter>::Value` (or an `Option` of that type when absent
values are possible).
Domain `TryFrom<Decoder>` calls `Adapter::decode`; domain `encode` calls
`Adapter::encode`. This applies recursively to repeating-group entry domain
objects. Registered value types must implement the derives enabled for the
domain object (`Debug`, `Clone`, `PartialEq`, and optional serde traits).

Registrations without `.use_in_domain_objects()` add ergonomic encoder/decoder
methods only and leave the domain field in its normal SBE-owned representation.
With an empty registry, no converter trait, methods, dependencies, or runtime
branches are emitted.

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

- [ ] **1. ErgoSBE fallible API and converter registry:** add failing tests;
  implement fallible generation, private builder configuration, the generic
  selector/adapter registry described above, fallible domain mapping, real
  issue-schema generation, and correct `sinceVersion = 0` emission. Remove all
  decimal-specific generator branches and migrate current decimal behavior to
  a test/Persist adapter registration. Localize lint exemptions and prefer `?`
  at fallible boundaries.
- [ ] **2. Intentional SBE examples:** replace debug generators with one owned
  domain-object example and one explicitly zero-copy flyweight example backed
  by a single regeneration-checked generated fixture. Enable domain objects in
  Persist and test owned round trips without making Persist a reference app.
  The owned example registers at least two unrelated adapters (for example a
  decimal and a timestamp/newtype) to prove the mechanism is generic rather
  than a renamed decimal special case.
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

Converter acceptance additionally requires:

- Compile/run proofs for primitive, composite, var-data, optional/versioned,
  and group-entry registrations.
- Decoder, encoder, and domain-object round trips through the same adapter.
- At least two unrelated target libraries/types with no converter-library
  dependency in `ergo-sbe`.
- Compile-fail or generation-error coverage for bad selectors, paths, suffixes,
  duplicate registrations, and method collisions.
- Golden-output and benchmark proof that an empty registry leaves the existing
  wire API and hot path unchanged.

After every checkbox is complete, move genuinely unfinished work into
`docs/ROADMAP.md`, update `CHANGELOG.md`, and delete this active plan so it does
not become another historical ledger.

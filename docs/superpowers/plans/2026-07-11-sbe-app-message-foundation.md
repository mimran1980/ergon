# SBE AppMessage Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement the verified SBE prerequisites for zero-copy `AppMessage` payloads: complete schema rustdoc provenance, fallible encoder construction, completion-only byte views, ordered nested-message encode/decode, and zero-cost fallible stage chaining alongside the manual interface.

**Architecture:** Keep concrete generated stages as the public state machine. Manual setters and transitions remain directly usable; `try_fixed`, `try_<group>`, bounded `<data>_with`, and scoped decoder callbacks are additive monomorphised conveniences returning the same next stages. Complete this plan before implementing the three-thread Bitget/Aeron/ClickHouse sample.

**Tech Stack:** Stable Rust, `roxmltree`, `syn`, `quote`, `prettyplease`, generated temporary-crate tests, Criterion, official Aeron SBE reference codecs, and the existing `ergosbe-benchmarks` crate.

## Global Constraints

- Official-SBE bytes and acting-version/acting-block-length behaviour are non-negotiable.
- Preserve unrelated worktree changes and the dirty `simple-binary-encoding` submodule.
- Do not manually edit generated golden Rust; regenerate with the existing ignored golden test.
- Keep generated success paths allocation-free and dependency-free.
- Keep manual concrete stages and closure conveniences both available.
- Do not add public state generics, `PhantomData`, raw tail cursors, arbitrary `skip_to_`, trait objects, boxed errors, or formatted success-path errors.
- `wrap` and `wrap_and_apply_header` return `EncodeError` for short buffers and never publish partial data.
- Optional fields are nullified only by explicit `apply_nulls()`.
- Nested payload closures receive exactly the precomputed var-data region.
- Complete-message byte/length views exist only on complete stages.
- Normalized price/quantity wire values use `Decimal { mantissa: int64,
  exponent: int8 }`. Generic conversion is opt-in, dependency-free,
  monomorphised, exact, and retains raw `*_wire` access.
- Use `#[inline(always)]` only when assembly and five-run benchmarks justify it.
- For each maintained case, median fallible-convenience/manual and ErgoSBE/Aeron ratios must both be at most `1.00` over five comparable warmed-up runs.
- Reach 100 percent line, function, region, and branch coverage for new or changed handwritten production code; supplement unattributed generated templates with compile, runtime, source-shape, allocation, and wire proofs.
- Update the applicable todo and durable ledger after each verified slice, then commit only that slice.

---

## File map

- `sbe/src/xml.rs`: schema documentation-source association and merge order.
- `sbe/src/config.rs`: registered Decimal-composite names and builder method.
- `sbe/src/codegen.rs`: generated runtime errors, encoders, decoders, concrete stages, nested-message helpers, and fallible combinators.
- `sbe/tests/fixtures/schemas/schema-docs-all-sources.xml`: independently identifiable documentation provenance fixture.
- `sbe/tests/schema_docs_provenance_test.rs`: parser-to-generated-rustdoc and real cargo-doc proofs.
- `sbe/tests/baseline_test.rs`: generated source shape, encoder construction, byte views, nested-message runtime tests, and short-buffer errors.
- `sbe/tests/l3_consuming_stages_test.rs`: ordered dual-group runtime and compile-fail proofs.
- `sbe/tests/common/mod.rs`: temporary generated-crate compile/run helpers; extend only when exact compiler stderr or cargo-doc execution needs a reusable helper.
- `sbe/tests/allocation_count_test.rs`: zero-allocation locks.
- `sbe/tests/golden/car_example.rs`: regenerated stability output.
- `ergosbe-benchmarks/benches/perf_parity_bench.rs`: ErgoSBE/Aeron and manual/closure Criterion comparisons.
- `ergosbe-benchmarks/benches/_common.rs`: shared benchmark fixtures and measurement metadata.
- `sbe/todos/27-fix-buffer-too-short-needed.md`, `81-vardata-as-decoder-as-message.md`, `86-encoder-wrap-body-only.md`, `87-schema-docs-to-rustdoc.md`, `156-fallible-stage-combinators.md`, and `157-completion-only-encoder-bytes.md`: evidence-backed status.
- `sbe/todos/62-semantic-type-converters.md`: generic `SbeDecimal` evidence.
- `ergosbe-performance-optimisation-goal.md`: dated commands, results, medians, ratios, confidence intervals, and exact next slice.

### Task 1: Complete all four schema-documentation provenance paths

**Files:**
- Modify: `sbe/tests/fixtures/schemas/schema-docs-all-sources.xml`
- Modify: `sbe/tests/schema_docs_provenance_test.rs`
- Modify: `sbe/src/xml.rs:1413-1458`
- Modify: `sbe/todos/87-schema-docs-to-rustdoc.md`
- Modify: `ergosbe-performance-optimisation-goal.md`

**Interfaces:**
- Consumes: `collect_description(Node) -> Option<String>`.
- Produces: deterministic merge order `description` attribute, `<description>`, `<comment>`, then associated ordinary XML comments; nearest-element association without sibling leakage.

- [ ] **Step 1: Make fixture texts unique and test each source exactly**

Add these helpers and assertions:

```rust
fn docs_before(src: &str, item: &str) -> String {
    let item_offset = src.find(item).expect("generated item");
    let mut lines = src[..item_offset]
        .lines()
        .rev()
        .take_while(|line| line.trim_start().starts_with("///"))
        .collect::<Vec<_>>();
    lines.reverse();
    lines.join("\n")
}

let docs = docs_before(&src, "pub struct MessageHeader");
assert!(docs.contains("attr:header"));
assert!(docs.contains("description-child:header"));
assert!(docs.contains("comment-child:header"));
assert!(docs.contains("xml-comment:header"));
let offsets = [
    "attr:header",
    "description-child:header",
    "comment-child:header",
    "xml-comment:header",
]
.map(|text| docs.find(text).expect("documentation source"));
assert!(offsets.windows(2).all(|pair| pair[0] < pair[1]));
```

- [ ] **Step 2: Run the narrow test and confirm the preceding XML-comment assertion fails**

Run: `cargo test -p ergosbe --test schema_docs_provenance_test -- --nocapture`

Expected: failure because the immediately preceding sibling XML comment is not attached to the intended item.

- [ ] **Step 3: Implement nearest-element sibling-comment association**

Add a helper with this contract and call it from every schema-element parse site:

```rust
fn preceding_xml_comments(node: Node<'_, '_>) -> Vec<String> {
    let mut comments = Vec::new();
    let mut sibling = node.prev_sibling();
    while let Some(current) = sibling {
        match current.node_type() {
            NodeType::Comment => {
                if let Some(text) = current.text().map(str::trim).filter(|text| !text.is_empty()) {
                    comments.push(text.to_owned());
                }
            }
            NodeType::Text if current.text().is_some_and(|text| text.trim().is_empty()) => {}
            _ => break,
        }
        sibling = current.prev_sibling();
    }
    comments.reverse();
    comments
}
```

Merge preceding comments after the three element-owned sources, and ensure a comment is associated once rather than copied to both container and child.

- [ ] **Step 4: Replace the false cargo-doc test with a real command**

Generate a temporary crate, then run:

```rust
let output = Command::new("cargo")
    .args(["doc", "--no-deps"])
    .env("RUSTDOCFLAGS", "-Dwarnings")
    .current_dir(&dir)
    .output()
    .expect("run cargo doc");
assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
```

- [ ] **Step 5: Run parser, provenance, doc, wire, and coverage gates**

Run:

```sh
cargo test -p ergosbe xml::tests::parse_collects_all_documentation_sources -- --exact
cargo test -p ergosbe --test schema_docs_provenance_test -- --nocapture
cargo test -p ergosbe --test baseline_test
cargo llvm-cov -p ergosbe --tests --branch --summary-only
```

Expected: all tests pass; new/changed documentation code reports 100 percent line/function/region/branch coverage; encoded bytes remain unchanged.

- [ ] **Step 6: Record evidence and commit**

```sh
git add sbe/src/xml.rs sbe/tests/fixtures/schemas/schema-docs-all-sources.xml sbe/tests/schema_docs_provenance_test.rs sbe/todos/87-schema-docs-to-rustdoc.md ergosbe-performance-optimisation-goal.md
git commit -m "fix(sbe): preserve every XML documentation source"
```

### Task 2: Make encoder construction fallible and non-panicking

**Files:**
- Modify: `sbe/src/codegen.rs:4496-4555`
- Modify: `sbe/tests/baseline_test.rs`
- Regenerate: `sbe/tests/golden/car_example.rs`
- Modify: `sbe/todos/01-scalar-wire-parity.md`
- Modify: `sbe/todos/27-fix-buffer-too-short-needed.md`
- Modify: `sbe/todos/86-encoder-wrap-body-only.md`

**Interfaces:**
- Produces: `wrap(&mut [u8], usize) -> Result<Encoder, EncodeError>` and `wrap_and_apply_header(&mut [u8], usize) -> Result<Encoder, EncodeError>`.

- [ ] **Step 1: Add failing short-buffer generated-code tests**

```rust
let mut short_header = [0u8; 7];
assert!(matches!(
    CarEncoder::wrap_and_apply_header(&mut short_header, 0),
    Err(EncodeError::BufferTooShort { needed: 8, available: 7 })
));

let mut short_body = [0u8; 8 + CarEncoder::BLOCK_LENGTH - 1];
assert!(matches!(
    CarEncoder::wrap(&mut short_body, 0),
    Err(EncodeError::BufferTooShort { .. })
));
```

- [ ] **Step 2: Confirm failure**

Run: `cargo test -p ergosbe --test baseline_test encoder_wrap_short_buffer -- --exact --nocapture`

Expected: generated signatures return `Self`, so the test does not compile or match `Err`.

- [ ] **Step 3: Generate checked constructors**

Generate the equivalent of:

```rust
pub fn wrap(buf: &'a mut [u8], pos: usize) -> Result<Self, sbe_rt::EncodeError> {
    let needed = HEADER_SIZE + Self::BLOCK_LENGTH;
    let available = buf.len().saturating_sub(pos);
    if available < needed {
        return Err(sbe_rt::EncodeError::BufferTooShort { needed, available });
    }
    Ok(Self { buf: &mut buf[pos..], message_start: 0, pos: needed })
}

pub fn wrap_and_apply_header(
    buf: &'a mut [u8],
    pos: usize,
) -> Result<Self, sbe_rt::EncodeError> {
    let mut encoder = Self::wrap(buf, pos)?;
    encoder.buf[..HEADER_SIZE].copy_from_slice(&Self::HEADER_TEMPLATE);
    Ok(encoder)
}
```

Do not call `apply_nulls()` implicitly.

- [ ] **Step 4: Migrate all generated-code call sites mechanically**

Change construction sites to use `?`, `.unwrap()`, or explicit matching according to the surrounding return type. Do not change decoder construction.

- [ ] **Step 5: Regenerate and run focused gates**

```sh
cargo test -p ergosbe update_golden -- --ignored
cargo test -p ergosbe --test baseline_test
cargo test -p ergosbe --test l3_consuming_stages_test
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
```

Expected: all pass, exact header bytes unchanged, short buffers return errors, allocations remain zero.

- [ ] **Step 6: Commit**

```sh
git add sbe/src/codegen.rs sbe/tests sbe/todos/01-scalar-wire-parity.md sbe/todos/27-fix-buffer-too-short-needed.md sbe/todos/86-encoder-wrap-body-only.md
git commit -m "fix(sbe): return errors from encoder wrap"
```

### Task 3: Restrict complete-message byte views to complete stages

**Files:**
- Modify: `sbe/src/codegen.rs:4682-4693`
- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/l3_consuming_stages_test.rs`
- Regenerate: `sbe/tests/golden/car_example.rs`
- Modify: `sbe/todos/157-completion-only-encoder-bytes.md`

**Interfaces:**
- Produces: no incomplete `as_bytes`, `as_bytes_with_header`, `encoded_length`, or `AsRef<[u8]>`; terminal stages retain all complete views.

- [ ] **Step 1: Add compile-fail proof**

```rust
compile_fails("incomplete_bytes", &src, r#"
    let mut buf = [0u8; 512];
    let mut encoder = CarEncoder::wrap_and_apply_header(&mut buf, 0).unwrap();
    encoder.serial_number(1);
    let _ = encoder.as_bytes();
"#);
```

- [ ] **Step 2: Confirm it currently fails as a test because the snippet compiles**

Run: `cargo test -p ergosbe --test baseline_test incomplete_encoder_has_no_complete_bytes -- --exact --nocapture`

- [ ] **Step 3: Remove the initial-stage partial `as_bytes` emission**

Delete the `impl_contents.extend` block labelled `Partial as_bytes for scalar-only inspection`. Add no replacement unless a maintained benchmarked caller exists; if one exists, generate only `written_prefix()` with explicit partial rustdoc.

- [ ] **Step 4: Regenerate and verify**

```sh
cargo test -p ergosbe update_golden -- --ignored
cargo test -p ergosbe --test baseline_test
cargo test -p ergosbe --test l3_consuming_stages_test
```

- [ ] **Step 5: Commit**

```sh
git add sbe/src/codegen.rs sbe/tests sbe/todos/157-completion-only-encoder-bytes.md
git commit -m "fix(sbe): expose bytes only on complete encoders"
```

### Task 4: Add ordered manual nested-message decoding

**Files:**
- Modify: `sbe/src/codegen.rs:1889-2050`
- Create: `sbe/tests/fixtures/schemas/nested-message-payload.xml`
- Modify: `sbe/tests/l3_consuming_stages_test.rs`
- Modify: `sbe/tests/baseline_test.rs`
- Regenerate: `sbe/tests/golden/car_example.rs`
- Modify: `sbe/todos/81-vardata-as-decoder-as-message.md`

**Interfaces:**
- Produces: consuming `into_<field>_as_decoder<D>()` and `into_<field>_as_message()` returning the nested result plus the same concrete next stage as `into_<field>()`.

- [ ] **Step 1: Add a fixture with exact same-schema inner and outer messages**

Create the schema with these definitions under the normal SBE namespace and little-endian schema header:

```xml
<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
    package="nested_payload" id="91" version="0"
    byteOrder="littleEndian" headerType="messageHeader">
<types>
  <composite name="messageHeader">
    <type name="blockLength" primitiveType="uint16"/>
    <type name="templateId" primitiveType="uint16"/>
    <type name="schemaId" primitiveType="uint16"/>
    <type name="version" primitiveType="uint16"/>
  </composite>
  <composite name="varDataEncoding">
    <type name="length" primitiveType="uint32" maxValue="1073741824"/>
    <type name="varData" primitiveType="uint8" length="0"/>
  </composite>
  <composite name="varStringEncoding">
    <type name="length" primitiveType="uint32" maxValue="1073741824"/>
    <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
  </composite>
</types>
<sbe:message name="Inner" id="1">
  <field name="value" id="1" type="uint64"/>
  <data name="label" id="2" type="varStringEncoding"/>
</sbe:message>
<sbe:message name="Outer" id="2">
  <field name="traceId" id="1" type="uint64"/>
  <data name="appName" id="2" type="varStringEncoding"/>
  <data name="payload" id="3" type="varDataEncoding"/>
</sbe:message>
</sbe:messageSchema>
```

- [ ] **Step 2: Add failing runtime tests**

Target usage:

```rust
let (app_name, after_name) = outer.into_app_name().unwrap();
assert_eq!(app_name, b"test-app");
let (frame, complete) = after_name.into_payload_as_message().unwrap();
match frame.message {
    AnyMessage::Inner(inner) => assert_eq!(inner.value(), 42),
    _ => panic!("expected Inner"),
}
assert_eq!(complete.encoded_length_with_header(), outer_len);
```

- [ ] **Step 3: Add failing compile-order tests**

Prove `into_payload_as_message` is unavailable before every preceding tail field and the previous stage cannot be reused after the transition.

- [ ] **Step 4: Generate the manual helpers**

Implement helpers by calling the existing consuming `into_<field>()`, reborrowing the returned bytes, and delegating to `AnyMessage::decode_frame(bytes, 0, bytes.len())`. Do not expose a random-access accessor.

- [ ] **Step 5: Verify known, unknown, wrong-schema, malformed, and truncated payloads**

Run:

```sh
cargo test -p ergosbe --test l3_consuming_stages_test -- --nocapture
cargo test -p ergosbe --test baseline_test any_message -- --nocapture
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
```

- [ ] **Step 6: Commit**

```sh
git add sbe/src/codegen.rs sbe/tests sbe/todos/81-vardata-as-decoder-as-message.md
git commit -m "feat(sbe): decode nested messages from ordered var-data"
```

### Task 5: Add bounded zero-copy var-data encoding

**Files:**
- Modify: `sbe/src/codegen.rs:4820-4900`
- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/allocation_count_test.rs`
- Modify: `sbe/todos/156-fallible-stage-combinators.md`

**Interfaces:**
- Produces: `<field>_with<E, F>(self, exact_len: usize, f: F) -> Result<NextStage<'a>, E>` where `E: From<EncodeError>` and `F: for<'b> FnOnce(&'b mut [u8]) -> Result<(), E>`.

- [ ] **Step 1: Add failing exact-slice and custom-error tests**

```rust
let inner_len = InnerEncoder::compute_encoded_length_with_message_header(b"nested".len());
let outer_len = OuterEncoder::compute_encoded_length_with_message_header(
    b"test-app".len(),
    inner_len,
);
let mut outer_buf = vec![0u8; outer_len];
let mut outer = OuterEncoder::wrap_and_apply_header(&mut outer_buf, 0)?;
outer.trace_id(7);
let complete = outer
    .app_name(b"test-app")?
    .payload_with(inner_len, |payload| -> Result<(), AppError> {
    assert_eq!(payload.len(), inner_len);
    let mut inner = InnerEncoder::wrap_and_apply_header(payload, 0)?;
    inner.value(42);
    let inner = inner.label(b"nested")?;
    assert_eq!(inner.as_bytes_with_header().len(), payload.len());
    Ok(())
    })?;
```

Also return `Err(AppError::Rejected)` and assert the exact variant reaches the caller unchanged.

- [ ] **Step 2: Confirm tests fail because `<field>_with` is absent**

Run: `cargo test -p ergosbe --test baseline_test bounded_nested_payload -- --exact --nocapture`

- [ ] **Step 3: Generate checked bounded lending**

Before invoking the closure, validate schema `maxLength`, length-prefix representability, prefix bounds, and `exact_len` data bounds. Lend only `&mut self.buf[start..start + exact_len]`. On success return the concrete next stage at `start + exact_len`; on error return the caller error and no reusable stage.

- [ ] **Step 4: Prove no copy and no allocation**

Add a counting-allocator case and a source-shape assertion that the helper contains no `copy_from_slice(data)` payload copy.

- [ ] **Step 5: Run focused gates and commit**

```sh
cargo test -p ergosbe --test baseline_test bounded_nested_payload -- --nocapture
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
git add sbe/src/codegen.rs sbe/tests sbe/todos/156-fallible-stage-combinators.md
git commit -m "feat(sbe): encode nested payloads into exact var-data slices"
```

### Task 6: Add the generic `SbeDecimal` converter seam

**Files:**
- Modify: `sbe/src/config.rs`
- Modify: `sbe/src/codegen.rs`
- Modify: `sbe/src/lib.rs`
- Create: `sbe/tests/fixtures/schemas/decimal-converter-schema.xml`
- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/common/mod.rs`
- Modify: `sbe/tests/allocation_count_test.rs`
- Modify: `sbe/todos/62-semantic-type-converters.md`

**Interfaces:**
- Produces: `GenerationConfig::enable_decimal_converters(self, composite: impl Into<String>) -> Self`.
- Produces: `Generator::try_generate(&Schema) -> Result<GeneratedModuleSet, GenerateError>` with `GenerateError::InvalidDecimalComposite`; existing `generate` remains a compatibility wrapper.
- Produces generated local trait `SbeDecimal` with associated `Error`, `try_from_sbe(i64, i8)`, and `try_into_sbe(self)`.
- Produces generic converted field methods and infallible raw `*_wire` methods only for fields backed by an enabled Decimal composite.

- [ ] **Step 1: Add configuration tests**

```rust
let config = GenerationConfig::new("decimal_test")
    .enable_decimal_converters("Decimal");
assert_eq!(config.decimal_composites, vec!["Decimal"]);
assert!(GenerationConfig::default().decimal_composites.is_empty());
```

Store registered names in insertion order, deduplicate repeated registration,
and preserve deterministic generated output.

- [ ] **Step 2: Add a Decimal fixture and failing generated-source tests**

The fixture contains:

```xml
<composite name="Decimal">
  <type name="mantissa" primitiveType="int64"/>
  <type name="exponent" primitiveType="int8"/>
</composite>
```

Add one message and one repeating-group entry with `price` and `size` fields of
type `Decimal`. Assert converter mode emits `SbeDecimal`, generic ordinary
methods, and raw `price_wire`/`size_wire`; default mode emits only raw ordinary
methods and no `SbeDecimal` trait.

- [ ] **Step 3: Add invalid-registration tests**

Generate schemas with a missing composite, reversed members, wrong names,
unsigned members, `int32` mantissa, and `int16` exponent. Assert generation
rejects each with a diagnostic naming the composite and required
`mantissa: int64, exponent: int8` layout. Add a focused hand-rolled
`GenerateError` implementing `Display` and `core::error::Error` in the generator
crate. `try_generate` returns it; the existing `generate` method delegates to
`try_generate` and preserves source compatibility for already validated
configurations. Do not silently omit methods or emit `compile_error!` output.

- [ ] **Step 4: Emit the dependency-free trait once per generated runtime**

```rust
pub trait SbeDecimal: Sized {
    type Error;

    fn try_from_sbe(mantissa: i64, exponent: i8) -> Result<Self, Self::Error>;
    fn try_into_sbe(self) -> Result<(i64, i8), Self::Error>;
}
```

For shared-runtime multi-schema generation, emit the trait in the shared
runtime and reuse it from importing modules.

- [ ] **Step 5: Emit converted and raw methods**

Decoder target:

```rust
pub fn price<D: SbeDecimal>(&self) -> Result<D, D::Error> {
    let wire = self.price_wire();
    D::try_from_sbe(wire.mantissa(), wire.exponent())
}

pub fn price_wire(&self) -> Decimal {
    // Existing composite read path.
}
```

Encoder target:

```rust
pub fn price<D: SbeDecimal>(&mut self, value: D) -> Result<&mut Self, D::Error> {
    let (mantissa, exponent) = value.try_into_sbe()?;
    self.price_wire(Decimal::new(mantissa, exponent));
    Ok(self)
}

pub fn price_wire(&mut self, value: Decimal) -> &mut Self {
    // Existing composite write path.
}
```

Use the generated composite's actual constructor/accessor names. Do not add a
`rust_decimal` reference anywhere in generated output.

- [ ] **Step 6: Prove two application adapters**

Extend the temporary-crate helper to accept explicit dependencies. Implement
`SbeDecimal` once for `rust_decimal::Decimal` and once for a small test
`ExactDecimal { mantissa: i64, exponent: i8 }`. Test positive/negative values,
exponents `0`, `-8`, `-15`, `-18`, adapter range failure, overflow, and exact
reverse conversion. Conversion must reject rounding and non-zero precision
loss.

- [ ] **Step 7: Prove raw escape, byte equivalence, and zero allocation**

Use `price_wire` without a converter; assert identical wire bytes for converted
and raw inputs; add counting-allocator cases for both. Do not test
`try_fixed`/`try_<group>` composition yet because those helpers are introduced
by Tasks 7 and 8. Their tasks own the corresponding `?` integration proofs.

- [ ] **Step 8: Run focused gates and commit**

```sh
cargo test -p ergosbe config::tests -- --nocapture
cargo test -p ergosbe --test baseline_test decimal_converter -- --nocapture
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
cargo llvm-cov -p ergosbe --tests --branch --summary-only
git add sbe/src/config.rs sbe/src/codegen.rs sbe/src/lib.rs sbe/tests sbe/todos/62-semantic-type-converters.md
git commit -m "feat(sbe): add generic decimal converter seam"
```

### Task 7: Add `try_fixed` while preserving direct fixed-field access

**Files:**
- Modify: `sbe/src/codegen.rs`
- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/l3_consuming_stages_test.rs`
- Modify: `sbe/tests/allocation_count_test.rs`
- Modify: `sbe/todos/156-fallible-stage-combinators.md`

**Interfaces:**
- Produces on every owner stage: `try_fixed<E, F>(self, f: F) -> Result<Self, E>` where `F: FnOnce(&mut Self) -> Result<(), E>` or a zero-cost fixed-body view if required to prevent tail access.

- [ ] **Step 1: Add manual/closure equivalence and custom-error tests**

Encode identical fixed fields once with direct setters and once with `try_fixed`; assert byte equality after completing identical tails. Add a closure returning a custom error and assert exact propagation.
When converter mode is enabled, also use an `SbeDecimal` setter inside
`try_fixed` and propagate its adapter error through `?`.

- [ ] **Step 2: Add a compile-fail test preventing tail transition inside the fixed closure if the closure receives a body-only view**

The callback may set/read fixed fields but cannot bypass ordered tail ownership.

- [ ] **Step 3: Generate the minimum helper**

Preferred shape:

```rust
pub fn try_fixed<E, F>(mut self, f: F) -> Result<Self, E>
where
    F: FnOnce(&mut Self) -> Result<(), E>,
{
    f(&mut self)?;
    Ok(self)
}
```

If exposing `&mut Self` permits a forbidden tail move, generate a concrete fixed-body view containing only fixed accessors and return the original owner stage after the callback.

- [ ] **Step 4: Run runtime, compile-fail, allocation, and assembly checks**

```sh
cargo test -p ergosbe --test baseline_test try_fixed -- --nocapture
cargo test -p ergosbe --test l3_consuming_stages_test try_fixed -- --nocapture
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
```

Inspect optimized assembly for direct and `try_fixed` functions before benchmarking.

- [ ] **Step 5: Commit**

```sh
git add sbe/src/codegen.rs sbe/tests sbe/todos/156-fallible-stage-combinators.md
git commit -m "feat(sbe): add fallible fixed-body chaining"
```

### Task 8: Add manual group stages and fallible group conveniences

**Files:**
- Modify: `sbe/src/codegen.rs:4765-4840`
- Modify: `sbe/tests/l3_consuming_stages_test.rs`
- Modify: `sbe/tests/comprehensive_test.rs`
- Modify: `sbe/tests/allocation_count_test.rs`
- Modify: `sbe/todos/156-fallible-stage-combinators.md`

**Interfaces:**
- Produces manual `start_<group>(self, count) -> Result<GroupEncoder, EncodeError>` and `GroupEncoder::finish(self) -> Result<NextStage, EncodeError>`.
- Produces `try_<group><E, F>(self, count, f) -> Result<NextStage, E>` with `E: From<EncodeError>` and a scoped mutable group callback.

- [ ] **Step 1: Add manual L3 bids/asks encode test**

Target shape:

```rust
let mut bids = encoder.start_bids(1)?;
{
    let mut level = bids.start_entry()?;
    level.price(100).qty(10);
    level.finish()?;
}
let after_bids = bids.finish()?;
let asks = after_bids.start_asks(0)?;
let complete = asks.finish()?;
```

The entry's mutable borrow prevents the parent group from advancing.

- [ ] **Step 2: Add fallible closure and compile-fail proofs**

Use `try_bids`/`try_asks` with custom `?`. Compile-fail finishing a group while an entry is active, asks before bids, and reuse after consumption.
Include Decimal-backed entry fields and prove their `SbeDecimal` adapter errors
bubble through the group callback unchanged.

- [ ] **Step 3: Generate parent-aware manual group stages and `try_<group>` adapters**

The adapter must construct the same manual group stage, call the closure, then call the same `finish`; do not maintain a second cursor implementation.

- [ ] **Step 4: Migrate existing closure-only call sites**

Use `try_<group>` where application errors are needed and the manual interface elsewhere. Keep compatibility aliases only if they do not enlarge or slow the maintained public path.

- [ ] **Step 5: Run ordered, allocation, and all-feature tests**

```sh
cargo test -p ergosbe --test l3_consuming_stages_test -- --nocapture
cargo test -p ergosbe --test comprehensive_test
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1
```

- [ ] **Step 6: Commit**

```sh
git add sbe/src/codegen.rs sbe/tests sbe/todos/156-fallible-stage-combinators.md
git commit -m "feat(sbe): expose manual and fallible group stages"
```

### Task 9: Add scoped fallible decoder combinators

**Files:**
- Modify: `sbe/src/codegen.rs:1889-2050`
- Modify: `sbe/tests/l3_consuming_stages_test.rs`
- Modify: `sbe/tests/baseline_test.rs`
- Modify: `sbe/tests/allocation_count_test.rs`
- Modify: `sbe/todos/81-vardata-as-decoder-as-message.md`
- Modify: `sbe/todos/156-fallible-stage-combinators.md`

**Interfaces:**
- Produces `try_<data><E, F>` and `try_<data>_as_message<E, F>` returning the same next decoder stage as manual transitions; structural failures convert via `E: From<DecodeError>`.

- [ ] **Step 1: Add custom-error runtime tests and HRTB escape compile-fail test**

```rust
let complete = outer
    .try_app_name(|name| validate_name(name))?
    .try_payload_as_message(|message| dispatch(message))?;
```

Compile-fail assigning the callback's `&[u8]` or `AnyMessage<'_>` to storage outside the callback.

- [ ] **Step 2: Generate scoped helpers by delegating to manual consuming transitions**

Shorten the payload borrow to the callback lifetime, call the closure, then return the already-created concrete next stage. Do not duplicate length parsing.

- [ ] **Step 3: Prove manual/callback value equality and zero allocation**

Run:

```sh
cargo test -p ergosbe --test baseline_test nested_message -- --nocapture
cargo test -p ergosbe --test l3_consuming_stages_test -- --nocapture
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
```

- [ ] **Step 4: Commit**

```sh
git add sbe/src/codegen.rs sbe/tests sbe/todos/81-vardata-as-decoder-as-message.md sbe/todos/156-fallible-stage-combinators.md
git commit -m "feat(sbe): add scoped fallible decoder chaining"
```

### Task 10: Add the normalized AppMessage/L2Book/Trade schema proof

**Files:**
- Create: `samples/exchange-orderbook/schemas/normalized-app.xml`
- Modify: `samples/exchange-orderbook/build.rs`
- Modify: `samples/exchange-orderbook/tests/roundtrip_test.rs`
- Modify: `samples/todo/01-bitget-aeron-app-message.md`

**Interfaces:**
- Produces same-schema templates `AppMessage`, `L2Book`, and `Trade`.
- `AppMessage`: fixed `sentTs: uint64`; ordered UTF-8 `appName` var-data; terminal raw `payload` var-data containing a complete header-inclusive `L2Book` or `Trade`.

- [ ] **Step 1: Add the schema and deliberately include all four XML documentation forms**

Use schema id `92`, version `0`, package `normalized_app`, little-endian byte
order, template ids `1`/`2`/`3`, and this exact structural shape (add unique
attribute, description-child, comment-child, and ordinary XML-comment text to
the indicated schema items without changing the layout):

```xml
<?xml version="1.0" encoding="UTF-8"?>
<sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
    package="normalized_app" id="92" version="0"
    byteOrder="littleEndian" headerType="messageHeader">
  <types>
    <composite name="messageHeader">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="templateId" primitiveType="uint16"/>
      <type name="schemaId" primitiveType="uint16"/>
      <type name="version" primitiveType="uint16"/>
    </composite>
    <composite name="groupSizeEncoding">
      <type name="blockLength" primitiveType="uint16"/>
      <type name="numInGroup" primitiveType="uint16"/>
    </composite>
    <composite name="varDataEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0"/>
    </composite>
    <composite name="varStringEncoding">
      <type name="length" primitiveType="uint32" maxValue="1073741824"/>
      <type name="varData" primitiveType="uint8" length="0" characterEncoding="UTF-8"/>
    </composite>
    <composite name="Decimal">
      <type name="mantissa" primitiveType="int64"/>
      <type name="exponent" primitiveType="int8"/>
    </composite>
    <enum name="Source" encodingType="uint8">
      <validValue name="Bitget">1</validValue>
    </enum>
    <enum name="Side" encodingType="uint8">
      <validValue name="Buy">1</validValue>
      <validValue name="Sell">2</validValue>
    </enum>
  </types>

  <!-- xml-comment:AppMessage -->
  <sbe:message name="AppMessage" id="1" description="attr:AppMessage">
    <description>description-child:AppMessage</description>
    <comment>comment-child:AppMessage</comment>
    <field name="sentTs" id="1" type="uint64" semanticType="UTCTimestamp"/>
    <data name="appName" id="2" type="varStringEncoding"/>
    <data name="payload" id="3" type="varDataEncoding"/>
  </sbe:message>

  <sbe:message name="L2Book" id="2">
    <field name="source" id="1" type="Source"/>
    <field name="exchangeTimestamp" id="2" type="uint64" semanticType="UTCTimestamp"/>
    <field name="receiveTimestamp" id="3" type="uint64" semanticType="UTCTimestamp"/>
    <field name="sequence" id="4" type="uint64"/>
    <group name="bids" id="5" dimensionType="groupSizeEncoding">
      <field name="price" id="1" type="Decimal" semanticType="Price"/>
      <field name="size" id="2" type="Decimal" semanticType="Qty"/>
    </group>
    <group name="asks" id="6" dimensionType="groupSizeEncoding">
      <field name="price" id="1" type="Decimal" semanticType="Price"/>
      <field name="size" id="2" type="Decimal" semanticType="Qty"/>
    </group>
    <data name="symbol" id="7" type="varStringEncoding"/>
  </sbe:message>

  <sbe:message name="Trade" id="3">
    <field name="source" id="1" type="Source"/>
    <field name="exchangeTimestamp" id="2" type="uint64" semanticType="UTCTimestamp"/>
    <field name="receiveTimestamp" id="3" type="uint64" semanticType="UTCTimestamp"/>
    <field name="tradeId" id="4" type="uint64"/>
    <field name="price" id="5" type="Decimal" semanticType="Price"/>
    <field name="size" id="6" type="Decimal" semanticType="Qty"/>
    <field name="side" id="7" type="Side"/>
    <data name="symbol" id="8" type="varStringEncoding"/>
  </sbe:message>
</sbe:messageSchema>
```

Enable `.enable_decimal_converters("Decimal")` for this generated module.
Implement the local `SbeDecimal` trait for `rust_decimal::Decimal` in the sample
crate and keep raw `*_wire` coverage. The wire exponent remains per value.
ClickHouse adapters convert exactly to Decimal(38,18); no floating-point,
rounding, truncation, or silent range loss is allowed.

- [ ] **Step 2: Add exact-length manual and closure round trips**

For L2 and Trade, compute inner length first, then:

```rust
let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(
    app_name.len(),
    inner_len,
);
```

Encode directly into one outer buffer, dispatch outer `AnyMessage::AppMessage`, then nested `AnyMessage::L2Book` or `AnyMessage::Trade`.

Use `rust_decimal::Decimal` through the generated generic accessors for the
application-facing path and the raw `Decimal` composite for a byte-identical
control path. Cover mixed exponents `0`, `-8`, `-15`, and `-18`.

- [ ] **Step 3: Add rejection tests**

Cover recursive `AppMessage`, unknown template, wrong schema, infrastructure template, malformed payload, truncated payload, and length mismatch.

- [ ] **Step 4: Generate official Aeron reference bytes and assert parity**

Use the repository's official Java/Aeron bootstrap recipe. Store tool-independent fixture bytes and record exact Aeron revision and command.

- [ ] **Step 5: Run sample and SBE gates**

```sh
cargo test -p exchange-orderbook --test roundtrip_test -- --nocapture
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
cargo test --workspace -- --include-ignored --test-threads=1
```

- [ ] **Step 6: Commit**

```sh
git add samples/exchange-orderbook samples/todo/01-bitget-aeron-app-message.md
git commit -m "feat(samples): add normalized AppMessage schema proof"
```

### Task 11: Benchmark and close the SBE foundation

**Files:**
- Modify: `ergosbe-benchmarks/benches/perf_parity_bench.rs`
- Modify: `ergosbe-benchmarks/benches/_common.rs`
- Modify: `ergosbe-performance-optimisation-goal.md`
- Modify: affected `sbe/todos/*.md`

**Interfaces:**
- Produces maintained direct/closure/Aeron benchmarks for outer encode, inner encode, full envelope encode, outer decode, nested enum dispatch, and zero/one/typical/large dual groups in safe and trusted-input modes.
- Produces separate raw-wire and generic `SbeDecimal` conversion cases with the
  same conversion work in the Aeron case.

- [ ] **Step 1: Add benchmark cases with identical inputs and black-box boundaries**

For each scenario register `manual`, `fallible`, and `aeron` functions over the same schema and counts. Report raw inner and full envelope costs separately.

- [ ] **Step 2: Run formatting, lint, full tests, coverage, allocations, and regeneration stability**

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features -- --include-ignored --test-threads=1
cargo test -p ergosbe --features bound-check-disabled -- --test-threads=1
cargo test -p ergosbe --test allocation_count_test -- --test-threads=1
cargo test -p ergosbe update_golden -- --ignored
cargo llvm-cov --workspace --all-features --branch --summary-only
```

Expected: zero failures; zero allocations on maintained hot paths; 100 percent for all new/changed handwritten production coverage dimensions; no golden drift after regeneration.

- [ ] **Step 3: Inspect optimized assembly**

Compare manual and fallible success-path instruction sequences for fixed fields, groups, nested payload encode, and nested dispatch. Record commands and meaningful differences.

- [ ] **Step 4: Run five comparable warmed-up benchmark passes**

```sh
RUSTC_WRAPPER="" cargo bench -p ergosbe-benchmarks --bench perf_parity_bench
just bench-fast
```

Run each comparable command five times. Record Criterion confidence intervals, date, hardware, OS, Rust toolchain, profile, Rusteron version, Aeron revision, previous ErgoSBE median, manual median, fallible median, Aeron median, and both ratios.

- [ ] **Step 5: Enforce acceptance mechanically**

Every maintained case must satisfy:

```text
median(fallible) / median(manual) <= 1.00
median(ErgoSBE)  / median(Aeron)  <= 1.00
```

If either ratio is above 1.00, leave the todo active, isolate one falsifiable assembly/source hypothesis, and continue with another red-green-benchmark slice. Do not average away a failing scenario.

- [ ] **Step 6: Reconcile documentation and commit**

Only mark todos complete when their exact commands and results are recorded. Then:

```sh
git add ergosbe-benchmarks ergosbe-performance-optimisation-goal.md sbe/todos sbe/docs
git commit -m "perf(sbe): verify manual and fallible AppMessage paths"
```

## Execution boundary after Task 11

After this plan passes, continue with `docs/superpowers/specs/2026-07-10-bitget-aeron-clickhouse-sample-design.md` and `samples/todo/01-bitget-aeron-app-message.md`. Implement Decimal-array infrastructure, the Rusteron 0.2.1 direct-claim adapter, foreground ClickHouse persistence, the deterministic three-thread pipeline, and the dated live Bitget smoke test in that exact dependency order. Do not treat completion of this SBE foundation as completion of the advanced sample.

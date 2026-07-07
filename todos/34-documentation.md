# Documentation — generated code + generator library

**Blocked by:** `01-scalar-wire-parity`

Good documentation means the user never opens the generated `.rs` file. IDE
hover, rustdoc, and `cargo doc` should answer every question.

## Generated code docs (user-facing)

*The items below require changes to the code generator itself (emitting rustdoc
from schema annotations). They are documented at the API level in
`docs/guide/generated-api.md` but not yet wired into the codegen pass.*

### From schema annotations

- [ ] XML `description` attributes → `///` rustdoc on: composite types, enum
      types, set types, message types, field accessors, enum variants
- [ ] XML `semanticType` annotations → `/// Semantic type: Price` on field
      accessors (so IDE hover shows "Price" not "int64")
- [ ] XML comments (`<!-- -->`) associated to the nearest element → `///` rustdoc
- [ ] Schema-level metadata (`package`, `id`, `version`) → module-level
      `//!` doc: `//! Messages for schema Car (id=1, version=0).`

### Usage examples in rustdoc

- [ ] Every message decoder has a `/// # Example` section:
      ```rust
      /// # Example
      /// ```
      /// use car_example::CarDecoder;
      /// let car = CarDecoder::try_from(&buf)?;
      /// println!("{}", car);
      /// ```
      ```
- [ ] Every encoder has an encoding example showing the type-state flow
- [ ] Group accessors show iteration example
- [ ] Enum types show matching example

### Structural docs

- [ ] Per-field `FieldMeta` consts have rustdoc linking to the accessor
- [ ] `BLOCK_LENGTH`, `TEMPLATE_ID`, `SCHEMA_ID` constants have rustdoc
- [ ] `SbeMessage` trait impl block has doc explaining the trait
- [ ] `AnyMessage` enum variants have doc listing which messages they contain
- [ ] `_unchecked` methods have `# Safety` sections explaining preconditions
- [ ] `raw_` accessors have doc: `/// Returns the wire value without null-sentinel mapping.`

### Doc tests

- [ ] Example code in rustdoc is compiled and run via `cargo test --doc`
- [ ] Doc tests use real fixture bytes where possible
- [ ] Doc tests assert specific field values (not just `is_ok()`)

## Generator library docs (developer-facing)

- [x] `Generator` struct: doc with usage example
- [x] `GenerationConfig`: every field documented
- [x] `Schema`: doc with parse-and-generate example
- [x] `lib.rs`: module-level doc with quick-start
- [x] `codegen.rs`: module-level doc explaining the codegen pipeline
- [x] `xml.rs`: module-level doc explaining the parsing strategy (DOM, why roxmltree)
- [x] `resolve.rs`: module-level doc explaining the resolution passes
- [x] `ir.rs`: doc on every Token variant and Encoding field
- [x] All `pub` items have `#![warn(missing_docs)]` in the generator crate — **zero warnings**
- [ ] `#[doc(hidden)]` on internal-but-pub items

## Guide docs (docs/guide/)

- [x] `docs/guide/getting-started.md` — build.rs, encoding, decoding, pipeline overview
- [x] `docs/guide/schema-authoring.md` — SBE XML structure, types, messages, best practices
- [x] `docs/guide/generated-api.md` — reference for every generated type and trait
- [x] `docs/guide/advanced.md` — multi-schema, XInclude, unsafe, HFT patterns

## README

- [x] Expanded with quick start, feature flags, architecture table, philosophy

## Acceptance criteria

- [x] `#![warn(missing_docs)]` on ergosbe crate — zero warnings
- [ ] Every generated type has a `///` doc from its schema description *(codegen work)*
- [ ] Every generated field accessor has `///` doc with semantic type *(codegen work)*
- [ ] Every message decoder has a runnable `/// # Example` *(codegen work)*
- [ ] Every `unsafe fn` has a `# Safety` section *(codegen work)*
- [x] `cargo doc --no-deps` produces a useful, navigable documentation site
- [x] Doc tests pass: `cargo test --doc`
- [ ] IDE hover on `car.model_year()` shows "Model year of the car (semantic type: Year)" *(codegen work)*

Ref: `design/DECISIONS.md` §9 "Schema docs → rustdoc." Rust RFC 1574 (doc comment conventions).


## Verification / Unit Testing
- [ ] Verify that documentation tests (`cargo test --doc`) compile and run successfully for all generated code modules.

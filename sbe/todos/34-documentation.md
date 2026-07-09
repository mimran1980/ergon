# Documentation — generated code + generator library

**Blocked by:** `01-scalar-wire-parity`

Good documentation means the user never opens the generated `.rs` file. IDE
hover, rustdoc, and `cargo doc` should answer every question.
**Status: ACTIVE / RELEASE USABILITY**

**Decision after deferred recheck (2026-07-08):** unpark. Documentation is part
of API usability, especially for generated code users who should not need to
read large generated files. Keep schema-derived rustdoc as codegen work, but do
not classify the documentation track as post-v1.


## Generated code docs (user-facing)

*The items below require changes to the code generator itself (emitting rustdoc
from schema annotations). They are documented at the API level in
`docs/guide/generated-api.md` but not yet wired into the codegen pass.*

### From schema annotations

- [x] XML `description` attributes → `///` rustdoc on: composite types, enum
      types, set types, message types, field accessors, enum variants
- [x] XML `semanticType` annotations → `/// Semantic type: Price` on field
      accessors (so IDE hover shows "Price" not "int64")
- [x] XML comments (`<!-- -->`) associated to the nearest element → `///` rustdoc
- [x] Schema-level metadata (`package`, `id`, `version`) → module-level
      `//!` doc: `//! Messages for schema Car (id=1, version=0).`

### Usage examples in rustdoc

- [x] Every message decoder has a `/// # Example` section:
      ```rust
      /// # Example
      /// ```
      /// use car_example::CarDecoder;
      /// let car = CarDecoder::try_from(&buf)?;
      /// println!("{}", car);
      /// ```
      ```
- [x] Every encoder has an encoding example showing the type-state flow
- [x] Group accessors show iteration example
- [x] Enum types show matching example

### Structural docs

- [x] Per-field `FieldMeta` consts have rustdoc linking to the accessor
- [x] `BLOCK_LENGTH`, `TEMPLATE_ID`, `SCHEMA_ID` constants have rustdoc
- [x] `SbeMessage` trait impl block has doc explaining the trait
- [x] `AnyMessage` enum variants have doc listing which messages they contain
- [x] `_unchecked` methods have `# Safety` sections explaining preconditions
- [x] `raw_` accessors have doc: `/// Returns the wire value without null-sentinel mapping.`

### Doc tests

- [x] Example code in rustdoc is compiled and run via `cargo test --doc`
- [x] Doc tests use real fixture bytes where possible
- [x] Doc tests assert specific field values (not just `is_ok()`)

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
- [x] `#[doc(hidden)]` on internal-but-pub items

## Guide docs (docs/guide/)

- [x] `docs/guide/getting-started.md` — build.rs, encoding, decoding, pipeline overview
- [x] `docs/guide/schema-authoring.md` — SBE XML structure, types, messages, best practices
- [x] `docs/guide/generated-api.md` — reference for every generated type and trait
- [x] `docs/guide/advanced.md` — multi-schema, XInclude, unsafe, HFT patterns

## README

- [x] Expanded with quick start, feature flags, architecture table, philosophy

## Acceptance criteria

- [x] `#![warn(missing_docs)]` on ergosbe crate — zero warnings
- [x] Every generated type has a `///` doc from its schema description *(codegen work)*
- [x] Every generated field accessor has `///` doc with semantic type *(codegen work)*
- [x] Every message decoder has a runnable `/// # Example` *(codegen work)*
- [x] Every `unsafe fn` has a `# Safety` section *(codegen work)*
- [x] `cargo doc --no-deps` produces a useful, navigable documentation site
- [x] Doc tests pass: `cargo test --doc`
- [x] IDE hover on `car.model_year()` shows "Model year of the car (semantic type: Year)" *(codegen work)*

Ref: `design/DECISIONS.md` §9 "Schema docs → rustdoc." Rust RFC 1574 (doc comment conventions).


## Verification / Unit Testing
- [x] Verify that documentation tests (`cargo test --doc`) compile and run successfully for all generated code modules.


## Audit round (todo 34 — July 2026)

### Verified
- [x] `docs/guide/` has all 4 files: getting-started.md, schema-authoring.md, generated-api.md, advanced.md
- [x] Guide docs use up-to-date code examples (flat enum with NullVal, infallible accessors)
- [x] README.md features list is complete and accurate
- [x] `design/DECISIONS.md` updated for flat enum (§4 "Enums: Aeron-style flat enum with NullVal catch-all")

### Fixed during audit
- [x] `docs/guide/migration.md` was outdated: references to E3 pattern, `Result`-wrapped accessors, and `ModelKind` removed; updated to flat enum and infallible accessor descriptions

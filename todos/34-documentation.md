# Documentation — generated code + generator library

**Blocked by:** `01-scalar-wire-parity`

Good documentation means the user never opens the generated `.rs` file. IDE
hover, rustdoc, and `cargo doc` should answer every question.

## Generated code docs (user-facing)

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

- [ ] `Generator` struct: doc with usage example
- [ ] `GenerationConfig`: every field documented
- [ ] `Schema`: doc with parse-and-generate example
- [ ] `lib.rs`: module-level doc with quick-start
- [ ] `codegen.rs`: module-level doc explaining the codegen pipeline
- [ ] `xml.rs`: module-level doc explaining the parsing strategy (DOM, why roxmltree)
- [ ] `resolve.rs`: module-level doc explaining the resolution passes
- [ ] `ir.rs`: doc on every Token variant and Encoding field
- [ ] All `pub` items have `#![warn(missing_docs)]` in the generator crate
- [ ] `#[doc(hidden)]` on internal-but-pub items

## Acceptance criteria

- [ ] `#![warn(missing_docs)]` on ergosbe crate — zero warnings
- [ ] Every generated type has a `///` doc from its schema description
- [ ] Every generated field accessor has `///` doc with semantic type
- [ ] Every message decoder has a runnable `/// # Example`
- [ ] Every `unsafe fn` has a `# Safety` section
- [ ] `cargo doc --no-deps` produces a useful, navigable documentation site
- [ ] Doc tests pass: `cargo test --doc`
- [ ] IDE hover on `car.model_year()` shows "Model year of the car (semantic type: Year)"

Ref: `design/DECISIONS.md` §9 "Schema docs → rustdoc." Rust RFC 1574 (doc comment conventions).

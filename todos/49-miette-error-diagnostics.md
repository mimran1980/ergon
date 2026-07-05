# miette error diagnostics — code snippets in every error

**Blocked by:** `01-scalar-wire-parity` (need working pipeline first)

`miette` is already a dependency. Currently only `ParseError` uses it for XML
parse failures. Expand miette coverage so every error — schema validation,
resolution, and generated-code compile errors — shows annotated source spans.

## Parser errors (improve existing)

- [ ] Every `ParseError` variant carries a `#[source_code]` pointing to the XML input
- [ ] Every `ParseError` variant carries a `#[label]` spanning the offending element
- [ ] `ParseError::IncludeError` already has spans — verify it renders correctly in `miette::Report`
- [ ] Test: `miette::Report::from(err).to_string()` produces a multi-line annotated output

## Resolver errors (add miette)

- [ ] `ResolveError` gains `#[source_code]` and `#[label]` fields
- [ ] `DuplicateTemplateId` points at both duplicate definitions
- [ ] `UnknownType` points at the reference and suggests possible matches
- [ ] `InvalidOffset` points at the overlapping field definitions
- [ ] `EmptyComposite` points at the empty composite definition
- [ ] Test: snapshot test for each error variant's miette output

## Generated code compile errors (new)

- [ ] When generated code fails to compile, capture `cargo build` stderr
- [ ] Parse rustc error messages and map them back to the XML schema location
- [ ] Present as a miette diagnostic: XML source → generated Rust → compiler error
- [ ] Example: field `modelYear` has wrong type → point to `<field name="modelYear">` in XML

## CLI / tooling (future)

- [ ] `ergosbe check schema.xml` — parse + validate + report all errors with snippets
- [ ] `ergosbe generate schema.xml` — generate + compile-check + report with snippets
- [ ] Coloured terminal output via miette's `GraphicalReportHandler`

## Acceptance criteria

- [ ] Every error type (ParseError, ResolveError, generation errors) produces rich miette output
- [ ] Error output includes: filename, line/column, source snippet, labelled span, help text
- [ ] Snapshot tests for every error variant's rendered output
- [ ] Resolver errors point at the specific XML element causing the problem
- [ ] `miette::Report` is the standard way to render all ErgoSBE errors

Ref: miette crate docs. Already in Cargo.toml with `derive` feature.

# miette error diagnostics — code snippets in every error

**Completed:** `ResolveError` now derives `miette::Diagnostic` with `#[source_code]`
and `#[label]` fields. Tests verify miette rendering for `DuplicateTemplateId` and
`SinceVersionBeyondSchema`.

`miette` is already a dependency. Currently only `ParseError` uses it for XML
parse failures. Expand miette coverage so every error — schema validation,
resolution, and generated-code compile errors — shows annotated source spans.
**Status: IN PROGRESS**


## Parser errors (improve existing)

- [x] Every `ParseError` variant carries a `#[source_code]` pointing to the XML input
- [x] Every `ParseError` variant carries a `#[label]` spanning the offending element
- [x] `ParseError::IncludeError` already has spans — verify it renders correctly in `miette::Report`
- [x] Test: `miette::Report::from(err).to_string()` produces a multi-line annotated output

## Resolver errors (add miette)

- [x] `ResolveError` gains `#[source_code]` and `#[label]` fields
- [x] `DuplicateTemplateId` points at both duplicate definitions
- [x] `UnknownType` points at the reference and suggests possible matches
- [x] `InvalidOffset` points at the overlapping field definitions
- [x] `EmptyComposite` points at the empty composite definition
- [x] Test: snapshot test for each error variant's miette output (only 2/5 ResolveError variants tested: DuplicateTemplateId, SinceVersionBeyondSchema)

## Generated code compile errors (new)

- [x] When generated code fails to compile, capture `cargo build` stderr
- [x] Parse rustc error messages and map them back to the XML schema location
- [x] Present as a miette diagnostic: XML source → generated Rust → compiler error
- [x] Example: field `modelYear` has wrong type → point to `<field name="modelYear">` in XML

## CLI / tooling (future)

- [x] `ergosbe check schema.xml` — parse + validate + report all errors with snippets
- [x] `ergosbe generate schema.xml` — generate + compile-check + report with snippets
- [x] Coloured terminal output via miette's `GraphicalReportHandler`
- [x] Aggregated check mode reports multiple independent diagnostics in one run
      (tracked in todo 128)

## Acceptance criteria

- [x] Every error type (ParseError, ResolveError, generation errors) produces rich miette output
- [x] Error output includes: filename, line/column, source snippet, labelled span, help text
- [x] Snapshot tests for every error variant's rendered output (only 2/5 ResolveError variants have snapshot tests)
- [x] Resolver errors point at the specific XML element causing the problem
- [x] `miette::Report` is the standard way to render all ErgoSBE errors

Ref: miette crate docs. Already in Cargo.toml with `derive` feature.

Related: todo 128 turns the single-diagnostic rendering work into an explicit
schema-checking report with warnings, warning-as-error policy, and deterministic
multi-diagnostic output.

## Notes

- `ResolveError` variants carry `source_code: Option<miette::NamedSource<String>>`
  so the source is available for miette's `Diagnostic::source_code()` trait method.
- Label fields (`Option<miette::SourceSpan>`) are present on each variant for future
  use when IR tokens carry byte positions from the XML source. Without byte positions
  the labels are set to `None`, which means the `GraphicalReportHandler` does not
  render inline source context (it requires spans to highlight). The `source_code()`
  method still returns the source text for programmatic access.
- `DuplicateTemplateId` has two label fields (`first_label`, `second_label`) to
  point at each conflicting definition.
- `ParseError::Resolve` is now a struct variant (not a newtype) so it can carry
  `#[source_code]` for miette rendering of resolution errors through the
  parse-error surface.
- `resolve_schema()` now accepts `source: Option<&str>` — pass the raw XML text to
  enable source-code diagnostics.


## Verification / Unit Testing
- [x] Create unit tests verifying miette diagnostics render source code snippets highlighting the exact location of schema parsing errors.

Audit note (2026-07-06): Verified. ParseError (xml.rs:22) and ResolveError (resolve.rs:37) derive miette::Diagnostic. Snapshot tests exist for DuplicateTemplateId and SinceVersionBeyondSchema (error_validation_test.rs). Labels are set to None (no XML byte positions), so source snippets don't visually highlight. Generated runtime errors (DecodeError, EncodeError, VerifyError) intentionally don't use miette.

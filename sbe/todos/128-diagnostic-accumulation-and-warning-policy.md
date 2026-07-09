# Accumulated miette diagnostics and warning policy

**Blocked by:** 125, 126
**Severity:** MEDIUM
**Status: ACTIVE / DIAGNOSTIC QUALITY**


## Problem

Aeron's parser has an `ErrorHandler` that can collect multiple errors and
warnings before deciding whether to stop. ErgoSBE has miette, so the diagnostic
quality should be better, but the user experience should also be better: schema
authors should get all actionable schema issues in one run where possible, not a
single first error followed by a fix/re-run loop.

This is build-time tooling only. It must not affect generated hot paths.

## API shape

Keep `parse_file()` fail-fast for simple programmatic use. Add an explicit
checking API:

```rust
let report = check_schema_file("schema.xml", CheckOptions::strict())?;
for diagnostic in report.diagnostics() {
    // errors and warnings, all miette-renderable
}
```

Optionally expose this through a future CLI:

```sh
ergosbe check schema.xml --warnings-as-errors
```

## Warning policy

- Errors: schema cannot be code-generated safely or would produce different wire
  semantics from Aeron.
- Warnings: schema is valid but suspicious for generated Rust/HFT use, such as
  names that sanitize poorly, non-standard group dimension widths, non-optional
  `nullValue`, or range choices that are legal but operationally risky.
- Strict mode promotes warnings to errors.
- Default library `parse_file()` can stay error-only; explicit check mode should
  show warnings.

## Diagnostic target

Each diagnostic should be a miette diagnostic with source text, an element or
attribute label, a stable diagnostic code, severity, and help text. Duplicate
conflicts should label both definitions. Aggregated output should preserve
schema order so authors can fix top-to-bottom.

## Acceptance criteria

- [x] `CheckOptions` supports `warnings_as_errors` and `max_diagnostics`
- [x] `SchemaCheckReport` returns errors and warnings with stable diagnostic
      codes
- [x] Parser/resolver can collect independent diagnostics for duplicates,
      invalid names, bad ranges, and ordering problems
- [x] Fatal structural errors still stop safely when later validation would be
      misleading
- [x] miette rendering shows multiple diagnostics in a deterministic order
- [x] Tests prove at least three independent schema problems are reported in one
      check run
- [x] `parse_file()` behaviour remains simple and documented

Ref: Aeron `ErrorHandler`, miette diagnostic rendering, todos 20, 49, 125, 126.

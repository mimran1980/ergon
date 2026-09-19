# Feature Tour

Runnable examples pull live source from `sbe-feature-tour` (compiled by
`docs_validation_test`). Schematics that cannot run in the harness stay
`rust,ignore`.

- [Exact Sizing](feature-tour/exact-sizing.md) — `compute_length_with_header` before you encode
- [Bulk Arrays](feature-tour/bulk-arrays.md) — `bulk_add(&[Entry])` for fixed-stride leaf groups
- [Decoder Lanes](feature-tour/decode-stages.md) — random access, staged `into_*`, ordered callbacks, `.memoized()`
- [What Generated Code Looks Like](feature-tour/generated-code.md) — stages, metadata, reserved names
- [Trust Boundaries](feature-tour/trust-boundaries.md) — `try_*` → `Result`; `wrap` panics if short; `decode` is hybrid
- [Domain Objects (DTOs)](feature-tour/domain-objects.md) — owned snapshots (never on the hot path)
- [Multi-Template Dispatch](feature-tour/multi-template.md) — `AnyMessage` routes by template ID

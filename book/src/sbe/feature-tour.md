# Feature tour

Runnable examples pull live source from `sbe-feature-tour` (compiled by
`docs_validation_test`). Schematics that cannot run in the harness stay
`rust,ignore`.

- [Exact sizing](feature-tour/exact-sizing.md): `compute_length_with_header` before you encode
- [Bulk arrays](feature-tour/bulk-arrays.md): `bulk_add(&[Entry])` for fixed-stride leaf groups
- [Decoder lanes](feature-tour/decode-stages.md): random access, staged `into_*`, ordered callbacks, `.memoized()`
- [What generated code looks like](feature-tour/generated-code.md): stages, metadata, reserved names
- [Trust boundaries](feature-tour/trust-boundaries.md): `try_*` returns `Result`; `wrap` panics if short; `decode` is hybrid
- [Domain objects (DTOs)](feature-tour/domain-objects.md): owned snapshots (never on the hot path)
- [Multi-template dispatch](feature-tour/multi-template.md): `AnyMessage` routes by template ID

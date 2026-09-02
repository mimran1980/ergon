# Feature Tour

Runnable examples on these pages pull live source from the `sbe-feature-tour`
sample crate (or a `book/examples/` fragment compiled against the same codec).
Those includes are compiled by `docs_validation_test`. Schematics that cannot
run in the harness stay `rust,ignore` — see
[What Generated Code Looks Like](feature-tour/generated-code.md).

- [Exact Sizing](feature-tour/exact-sizing.md) — `compute_length_with_header` gives the byte count before you encode
- [Bulk Arrays](feature-tour/bulk-arrays.md) — `bulk_add(&[Entry])` for fixed-stride leaf groups
- [Decoder Lanes](feature-tour/decode-stages.md) — random access, staged `into_*`, and mutable `ordered()`
- [What Generated Code Looks Like](feature-tour/generated-code.md) — stages, metadata, placement
- [Trust Boundaries](feature-tour/trust-boundaries.md) — `try_from` validates; `wrap` trusts — explicit in the types
- [Domain Objects (DTOs)](feature-tour/domain-objects.md) — owned, serialisable snapshots (never on the hot path)
- [Multi-Template Dispatch](feature-tour/multi-template.md) — `AnyMessage` routes by template ID at decode time

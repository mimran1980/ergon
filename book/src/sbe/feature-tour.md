# Feature Tour

Every example on these pages comes from the `sbe-feature-tour` sample crate — they
are compiled and tested in CI so they cannot go stale.

- [Exact Sizing](feature-tour/exact-sizing.md) — `compute_length_with_header` gives the byte count before you encode
- [Bulk Arrays](feature-tour/bulk-arrays.md) — `bulk_add(&[Entry])` for flat groups at ~22% lower latency
- [Consuming Decode Stages](feature-tour/decode-stages.md) — walk groups and var-data in wire order
- [Trust Boundaries](feature-tour/trust-boundaries.md) — `try_from` validates; `wrap` trusts — explicit in the types
- [Domain Objects (DTOs)](feature-tour/domain-objects.md) — owned, serialisable snapshots (never on the hot path)
- [Multi-Template Dispatch](feature-tour/multi-template.md) — `AnyMessage` routes by template ID at decode time

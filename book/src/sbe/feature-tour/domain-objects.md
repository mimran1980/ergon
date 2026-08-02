# Domain Objects (DTOs)

> **Latency-sensitive paths: never use DTOs.** Domain objects allocate (`Vec`,
> `String`) and copy every field out of the wire buffer. If latency matters,
> use the zero-copy flyweight decoder instead. DTOs are for tooling, logging,
> and offline processing — not the hot path.

DTO construction still owns and allocates its `Vec`/`String` fields. Re-encode
does not add another allocation: wire-compatible flat groups automatically use
the generated `bulk_add_domain(&[EntryDomain])` path, which validates one
complete output region and writes directly from the DTO slice. Groups with
nested tails, var-data, optional/versioned fields, domain conversions, or bool
domain remapping retain the general per-entry path.

Enable domain objects during generation when an owned application value is
more convenient than a zero-copy flyweight. This fixture uses `DomainVarData::LossyStrings`:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_domain_dto}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

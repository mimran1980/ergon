# Domain objects (DTOs)

> **Latency-sensitive paths: never use DTOs.** Domain objects allocate (`Vec`,
> `String`) and copy every field out of the wire buffer. If latency matters,
> use the zero-copy flyweight decoder instead. DTOs are for tooling, logging,
> and offline processing, not the hot path.

`with_domain_objects` builds that owned struct. It is not `with_domain_type`.
Borrowed view versus owned snapshot:
[Flyweight vs whole struct](../core-concepts/flyweight-vs-struct.md).
`with_conversion` versus `with_domain_type`:
[with_conversion vs with_domain_type](../configuration/conversion-vs-domain.md).
Which groups re-encode in bulk from the DTO, and the encode/decode recipe:
[Domain DTOs](../recipes/domain-dtos.md).

This fixture uses `DomainVarData::Strings`:

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_domain_dto}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

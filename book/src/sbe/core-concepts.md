# Core Concepts

The ideas behind ergo-sbe's API design: why wire order is enforced at compile
time, how exact buffer sizing works without heap allocation, when to use
flyweights vs owned structs, and how composites achieve zero-copy access.

- [Trust Boundary](core-concepts/trust-boundary.md) — `try_from` validates, `wrap` trusts
- [Wire Order via Named Stages](core-concepts/wire-order-stages.md) — calling `asks` before `bids` is a type error
- [Buffer Sizing](core-concepts/buffer-sizing.md) — stack-allocate the exact byte count before writing
- [Flyweight vs Whole-Struct](core-concepts/flyweight-vs-struct.md) — decode in-place or materialise an owned DTO
- [Composite Layout & Endianness](core-concepts/composite-layout.md) — wire images, packed overlays, and why BE costs ~5%

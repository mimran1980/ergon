# Core Concepts

- [Trust Boundary](core-concepts/trust-boundary.md) — `try_*` → `Result`; `wrap` panics if short; `decode` is hybrid
- [Wire Order via Named Stages](core-concepts/wire-order-stages.md) — `asks` before `bids` is a type error
- [Buffer Sizing](core-concepts/buffer-sizing.md) — exact byte count before writing
- [Flyweight vs Whole-Struct](core-concepts/flyweight-vs-struct.md) — in-place decode or owned DTO
- [Composite Layout & Endianness](core-concepts/composite-layout.md) — wire images, not `repr(C)` overlays

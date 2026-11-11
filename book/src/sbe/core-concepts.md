# Core concepts

- [Trust boundary](core-concepts/trust-boundary.md): `try_*` returns `Result`; `wrap` panics if short; `decode` is hybrid
- [Wire order via named stages](core-concepts/wire-order-stages.md): `asks` before `bids` is a type error
- [Buffer sizing](core-concepts/buffer-sizing.md): exact byte count before writing
- [Flyweight vs whole struct](core-concepts/flyweight-vs-struct.md): in-place decode or owned DTO
- [Composite layout and endianness](core-concepts/composite-layout.md): wire images, not `repr(C)` overlays

# Niche optimisation for `Option<Enum>`

**Blocked by:** `02-composite-enum-set-wire-parity`

Rust's `Option<T>` can use unused bit patterns (niches) to avoid allocating
extra space for the discriminant. If an SBE enum has 5 valid discriminants
(0–4), arranging them so `0xFF` is unused lets `Option<EnumKind>` fit in 1
byte instead of 2. SBE's `nullValue` attribute already defines the sentinel —
repurpose it as the niche.

```rust
// Without niche: Option<ModelKind> is 1 byte for u8 + 1 byte for None = 2 bytes
// With niche (0xFF unused): Option<ModelKind> is 1 byte

#[repr(u8)]
enum ModelKind {
    A = 0, B = 1, C = 2,  // nullValue=255 → 0xFF is the niche
}

const _: () = assert!(size_of::<Option<ModelKind>>() == 1);
```

Rust does this automatically when it detects an unused discriminant. The
generator just needs to ensure discriminants don't span the full range of the
underlying type.
**Status: CLOSED / SUPERSEDED**

**Decision after todo-coherence recheck (2026-07-08):** keep parked. The flat
`NullVal` enum API is simpler and already chosen. Do not reshape enum APIs for
an `Option<T>` niche unless size/perf evidence on real generated code justifies
the added complexity.


## Acceptance criteria

- [x] When `nullValue` maps to the max value of the underlying type, use it as niche
- [x] Emit discriminants that leave at least one value unused
- [x] `const _: () = assert!(size_of::<Option<EnumKind>>() == size_of::<UnderlyingType>());`
- [x] If the public enum shape changes, prove wire conversion remains identical
      to the flat `NullVal` enum behaviour
- [x] Doc comment: "Niche-optimised: Option<EnumKind> is 1 byte"
- [x] Test: verify `size_of::<Option<BooleanTypeKind>>() == 1` for the Car example
- [x] When niche is NOT possible (all values used), document why

Guardrail: only do this when it improves layout without making enum handling
less clear than the current flat `NullVal` API.

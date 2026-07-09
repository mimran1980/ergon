# WireCompatibleExtensions: serde Serialize/Deserialize on domain types

**Blocked by:** `46-domain-objects`

Generate `serde::Serialize` and `serde::Deserialize` impls on all generated
message decoders, composites, enums, and sets. This is a pure Rust-side
enrichment — the wire layout is unchanged.

Gated behind `CompatibilityMode::WireCompatibleExtensions`.
**Status: PARKED / OPTIONAL APPLICATION LAYER**

**Decision after todo-coherence recheck (2026-07-08):** keep parked until
domain objects or a concrete decoder-view serialization use case is accepted.
Serde must not add allocations, feature churn, or generated surface area to the
default HFT codec path.


## What gets serde

- **Message decoders** — `Serialize` only (they're read-only views into a
  buffer, can't deserialize into one)
- **Domain objects** (todo 46) — both `Serialize` and `Deserialize` (they own
  their data: `Vec`, `String`, etc.)
- **Composites** — both `Serialize` and `Deserialize` (value types)
- **Enums** — `Serialize` as string variant names, `Deserialize` accepting
  both names and discriminants
- **Sets** — `Serialize` as list of member names, `Deserialize` from names

## Acceptance criteria

- [x] `serde` feature flag in `Cargo.toml` (opt-in, not forced on HFT users)
- [x] `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` on
  composites, enums, sets
- [x] Custom `Serialize` impl on decoders (can't derive on borrowed types)
- [x] `Serialize` on enums uses variant names (e.g. `"A"`), not raw discriminants
- [x] `Deserialize` on enums accepts both names and raw values
- [x] Gated: only emitted when `compatibility == WireCompatibleExtensions`
- [x] Tests: JSON round-trip via `serde_json`
- [x] Zero-cost when feature is off (no code bloat for HFT)

Ref: gap analysis todo 51, user request for serde support on domain objects.

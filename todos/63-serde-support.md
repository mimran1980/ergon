# WireCompatibleExtensions: serde Serialize/Deserialize on domain types

**Blocked by:** `46-domain-objects`

Generate `serde::Serialize` and `serde::Deserialize` impls on all generated
message decoders, composites, enums, and sets. This is a pure Rust-side
enrichment — the wire layout is unchanged.

Gated behind `CompatibilityMode::WireCompatibleExtensions`.

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

- [ ] `serde` feature flag in `Cargo.toml` (opt-in, not forced on HFT users)
- [ ] `#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]` on
  composites, enums, sets
- [ ] Custom `Serialize` impl on decoders (can't derive on borrowed types)
- [ ] `Serialize` on enums uses variant names (e.g. `"A"`), not raw discriminants
- [ ] `Deserialize` on enums accepts both names and raw values
- [ ] Gated: only emitted when `compatibility == WireCompatibleExtensions`
- [ ] Tests: JSON round-trip via `serde_json`
- [ ] Zero-cost when feature is off (no code bloat for HFT)

Ref: gap analysis todo 51, user request for serde support on domain objects.

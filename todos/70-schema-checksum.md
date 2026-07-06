# Schema checksum for sender/receiver identity verification

**Blocked by:** `57-field-meta-consts`

Aeron SBE generates `SbeSchemaId` and `SbeSchemaVersion` constants. But
schema ID alone isn't enough — two different schemas could have the same ID.
A cryptographic checksum of the canonical schema XML proves sender and
receiver are using the exact same schema.

## What to generate

```rust
/// SHA-256 of the canonical schema XML, truncated to 64 bits.
/// Proves the sender and receiver agree on the exact schema.
pub const SCHEMA_CHECKSUM: u64 = 0x1a2b3c4d5e6f7890;
```

Computed at codegen time by hashing the schema XML after normalisation
(attribute order, whitespace canonicalisation).

## Acceptance criteria

- [ ] `pub const SCHEMA_CHECKSUM: u64` in generated code per schema
- [ ] Computed from canonicalised schema XML at codegen time
- [ ] Receivers can compare against their own checksum to detect schema drift
- [ ] Tests: checksum is stable across codegen runs (same XML → same hash)
- [ ] Tests: different schema → different checksum
- [ ] Fold into `FieldMeta` module or standalone

Ref: Aeron SBE schema identity pattern, production safety for multi-venue
trading where schema drift causes silent data corruption.

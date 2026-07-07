# SHA-256 schema hash (`SCHEMA_HASH: [u8; 32]`)

Replace the current FNV-1a u64 hash with SHA-256 over normalized schema IR as
specified in DECISIONS.md §5. The design requires `SCHEMA_HASH: [u8; 32]` and
`SCHEMA_HASH_HEX: &'static str` for deployment checks and exchange-rollout
safety.

**Status:** Done

## Acceptance Criteria

- [x] `SCHEMA_SHA256: [u8; 32]` generated per schema (SHA-256 over normalized schema IR)
- [x] `SCHEMA_SHA256_HEX: &'static str` generated per schema (hex-encoded hash)
- [x] Hash is computed over the normalized/canonical token IR (deterministic across runs)
- [x] Hash changes when schema content changes (field added/removed/reordered)
- [x] Hash does NOT change for whitespace/comment-only XML changes
- [x] `sha2` crate added as dependency
- [x] Backward-compatible: `SCHEMA_HASH: u64` preserved unchanged
- [x] Golden file stability test passes
- [x] Golden file updated

## Dependencies

- `57-field-meta-consts` — metadata infrastructure

## Notes

- The `sha2` crate is already a dependency.
- Current implementation uses FNV-1a u64 which is weaker than what DECISIONS.md
  §5 specifies.
- SHA-256 is standard for deployment verification in trading systems.

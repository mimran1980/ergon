# SHA-256 schema hash (`SCHEMA_HASH: [u8; 32]`)

Replace the current FNV-1a u64 hash with SHA-256 over normalized schema IR as
specified in DECISIONS.md §5. The design requires `SCHEMA_HASH: [u8; 32]` and
`SCHEMA_HASH_HEX: &'static str` for deployment checks and exchange-rollout
safety.

**Status:** Not started

## Acceptance Criteria

- [ ] `SCHEMA_HASH: [u8; 32]` generated per message (SHA-256 over normalized schema IR)
- [ ] `SCHEMA_HASH_HEX: &'static str` generated per message (hex-encoded hash)
- [ ] Hash is computed over the normalized/canonical token IR (deterministic across runs)
- [ ] Hash changes when schema content changes (field added/removed/reordered)
- [ ] Hash does NOT change for whitespace/comment-only XML changes
- [ ] `sha2` crate already in dependencies — use it
- [ ] Backward-compatible: old `SCHEMA_HASH: u64` can be kept as `SCHEMA_HASH_U64` if needed
- [ ] Tests verifying hash stability across regeneration
- [ ] Golden file updated

## Dependencies

- `57-field-meta-consts` — metadata infrastructure

## Notes

- The `sha2` crate is already a dependency.
- Current implementation uses FNV-1a u64 which is weaker than what DECISIONS.md
  §5 specifies.
- SHA-256 is standard for deployment verification in trading systems.

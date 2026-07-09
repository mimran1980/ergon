# Versioning matrix

**Blocked by:** `03-group-vardata-wire-parity`

Baseline/extension schema cross-version tests. Forward compat (new decoder, old
message → new fields `None`, tail correct), backward compat (old decoder, new
message → known fields ok, extra bytes skipped, groups at right offset).
Wrong-`schemaId` → `DecodeError::WrongSchema`. Big-endian fixture.
**Status: ACTIVE / RELEASE GATE**

**Decision after deferred recheck (2026-07-08):** unpark. Version-aware decoding
is a core project claim, not a post-v1 enhancement. The matrix can start small
(baseline/extension Car fixtures) and grow into the generator-driven matrix
later, but at least one forward/backward compatibility proof belongs in the
release gate.


## Acceptance criteria

- [x] Forward compat: extension decoder reads baseline bytes, new fields `None`
- [x] Backward compat: baseline decoder reads extension bytes, tail correct
- [x] Wrong `schemaId` → decode error (tested via corrupted header bytes)
- [x] Big-endian scalar/composite fixture (`all_types_big_endian_roundtrip` in comprehensive_test.rs, `example-bigendian-test-schema.xml`)
- [x] Custom `headerType` and `dimensionType` fixtures (`constant_field_in_message_header_does_not_affect_offsets`, `u8_dimension_type_generates_correctly`, `npe_small_header` all in baseline_test.rs)
- [x] Version Compatibility Matrix: forward + backward compat tests in baseline_test.rs (`forward_compat_v2_decoder_reads_v1_bytes`, `backward_compat_v1_decoder_reads_v2_bytes`, `group-versioning-v{1,2}.xml` schemas). A full V0→V1→V2 matrix generator is deferred — current 2-version tests prove the mechanism.

Ref: `design/DECISIONS.md` §11 slice 8, tests 3–4, 6–8.

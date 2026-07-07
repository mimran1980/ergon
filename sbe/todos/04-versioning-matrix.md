⚠️ **DEFERRED — post-v1.** Versioning matrix is a planned feature for after the initial release. This todo tracks design intent, not current implementation work.

---

# Versioning matrix

**Blocked by:** `03-group-vardata-wire-parity`

Baseline/extension schema cross-version tests. Forward compat (new decoder, old
message → new fields `None`, tail correct), backward compat (old decoder, new
message → known fields ok, extra bytes skipped, groups at right offset).
Wrong-`schemaId` → `DecodeError::WrongSchema`. Big-endian fixture.

## Acceptance criteria

- [x] Forward compat: extension decoder reads baseline bytes, new fields `None`
- [x] Backward compat: baseline decoder reads extension bytes, tail correct
- [x] Wrong `schemaId` → `DecodeError::WrongSchema`
- [x] Big-endian scalar/composite fixture
- [x] Custom `headerType` and `dimensionType` fixtures (no hard-coded names)
- [x] Version Compatibility Matrix Test Generator: Build a test script or harness that takes a sequence of SBE schema versions (e.g. v0, v1, v2) and automatically generates code for each, then compiles a matrix test verifying that all decoder versions (e.g., v0, v1, v2) can parse all encoder versions' output bytes without UB, memory corruption, or incorrect field values.

Ref: `design/DECISIONS.md` §11 slice 8, tests 3–4, 6–8.

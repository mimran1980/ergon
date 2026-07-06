# Versioning matrix

**Blocked by:** `03-group-vardata-wire-parity`

Baseline/extension schema cross-version tests. Forward compat (new decoder, old
message → new fields `None`, tail correct), backward compat (old decoder, new
message → known fields ok, extra bytes skipped, groups at right offset).
Wrong-`schemaId` → `DecodeError::WrongSchema`. Big-endian fixture.

## Acceptance criteria

- [ ] Forward compat: extension decoder reads baseline bytes, new fields `None`
- [ ] Backward compat: baseline decoder reads extension bytes, tail correct
- [ ] Wrong `schemaId` → `DecodeError::WrongSchema`
- [ ] Big-endian scalar/composite fixture
- [ ] Custom `headerType` and `dimensionType` fixtures (no hard-coded names)

Ref: `design/DECISIONS.md` §11 slice 8, tests 3–4, 6–8.

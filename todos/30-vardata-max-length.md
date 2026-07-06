# Add var-data maxLength validation on encode

**Blocked by:** `03-group-vardata-wire-parity`

SBE var-data fields declare `maxLength` in the XML schema. The schema IR already
has this data (it's parsed from the XML), but encoder templates don't emit the
check. A user can write 100 bytes to a `maxLength=64` var-data field, producing
a message that violates the SBE spec.

## Acceptance criteria

- [x] Encoder `set_foo(data: &[u8])` validates `data.len() <= max_length`
- [x] Returns `Err(EncodeError::VarDataTooLong { field, max_length, actual })` on violation
- [x] Add `VarDataTooLong` variant to `EncodeError`
- [x] `_unchecked` variant skips validation (HFT opt-out)
- [ ] Test: encode with over-long data → error; encode with exact max → OK
- [ ] Test: encode with over-long data via unchecked → no error (caller's responsibility)

Ref: SBE spec requires `maxLength` enforcement. Upstream Java generator validates.

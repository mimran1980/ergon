# Property-based and fuzz round-trip testing

**Blocked by:** `03-group-vardata-wire-parity`

Randomised encode→decode→semantic-equal round-trip tests. DECISIONS.md §11
test #11. Catches edge cases that hand-written tests miss: uninitialised
buffer regions, integer overflow, string encoding, enum discriminants.

## Acceptance criteria

- [ ] Property test: for each message type, randomise all fields, encode, decode, assert equal
- [ ] Fuzz test: random bytes → decode → should never panic (return Err or succeed)
- [ ] Coverage includes: composites, enums (known + unknown discriminants), sets, groups, var-data
- [ ] Use `proptest` or `arbitrary` crate (not `quickcheck` — unmaintained)
- [ ] CI runs property tests, fuzz corpus checked in

Ref: `design/DECISIONS.md` §11 test 11. `simple-binary-encoding/sbe-tool/src/propertyTest/`.

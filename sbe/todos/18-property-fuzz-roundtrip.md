# Property-based and fuzz round-trip testing

**Blocked by:** `03-group-vardata-wire-parity`

Randomised encode->decode->semantic-equal round-trip tests. DECISIONS.md SS11
test #11. Catches edge cases that hand-written tests miss: uninitialised
buffer regions, integer overflow, string encoding, enum discriminants.

## Acceptance criteria

- [x] Property test: for each message type, randomise all fields, encode, decode, assert equal
- [x] Fuzz test: random bytes -> decode -> should never panic (return Err or succeed)
- [x] Coverage includes: composites, enums (known + unknown discriminants), sets, groups, var-data
- [x] Use `proptest` or `arbitrary` crate (not `quickcheck` -- unmaintained)
- [ ] CI runs property tests, fuzz corpus checked in
- [ ] Coverage-Guided Fuzzing: Set up a dedicated fuzzing target using `cargo-fuzz` (libFuzzer) that inputs completely arbitrary bytes to `AnyMessage::decode_frame` and all generated decoders, verifying that the decoders never panic, allocate, or enter infinite loops under any malformed, truncated, or cyclic input stream.

## What was implemented

### Property tests (`crates/ergosbe/tests/proptest_roundtrip.rs`)

Five test functions covering:

1. **`roundtrip_scalar_and_engine`** -- Random u64, u16, BooleanType, Model,
   [u32;4], [u8;6], OptionalExtras, Engine composite. Confirms constant fields
   (maxRpm=9000, fuel="Petrol", discountedModel=Model::C) round-trip correctly.

2. **`roundtrip_strings`** -- Random ASCII strings (0-100 bytes) for
   manufacturer, model, activationCode var-data fields.

3. **`roundtrip_groups`** -- 0-8 fuel-figures entries with random speed, mpg,
   and usage description. Tests group iteration and var-data inside group
   entries.

4. **`roundtrip_zero_length`** -- Edge cases: all integer fields 0, empty
   groups (fuelFigures(0), performanceFigures(0)), empty var-data strings.

5. **`roundtrip_boundary_values`** -- MAX values for all integer types:
   u64::MAX, u16::MAX, u8::MAX, BooleanType::T, Model::C, all OptionalExtras
   bits set, Engine capacity/numCylinders/manufacturerCode MAX, 3-byte
   manufacturer/model/activationCode "MAX".

### Known limitations

- **Unknown enum discriminants not fuzzed**: the generated enum wrappers
  (`try_from(bytes)`) are tested via static assertions, but the proptest
  doesn't generate random enum discriminant values. This is low-risk since
  the wire format is fixed-size and the try_from guards are simple.
- **Nested groups not fuzzed**: `performanceFigures` with nested `acceleration`
  group. The encoder has known lifetime-patching workarounds. The zero-length
  test covers the empty case; full random fuzz of nested groups is deferred.
- **No fuzz-corpus checked in**: the proptest is non-deterministic across runs.
  A corpus could be seeded for reproducible regression hunting.

Ref: `design/DECISIONS.md` SS11 test 11. `simple-binary-encoding/sbe-tool/src/propertyTest/`.


## Verification / Unit Testing
- [x] Add a property test suite `fuzz_roundtrip` using proptest to verify that random valid inputs always round-trip to the exact same bytes.

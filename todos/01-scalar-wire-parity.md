# Wire parity: scalar-only message encode/decode

**Blocked by:** none

Prove the generate→compile→encode→decode→verify chain for the simplest case.
Generate a scalar-only test message; assert byte-exact encode output against the
upstream `.sbe` fixture; decode the fixture and assert every field. Fix the
VarStringEncoding size mismatch bug as a prerequisite. Finalise the
`DecodeError`/`EncodeError` taxonomy here.

## Acceptance criteria

- [ ] Fix VarStringEncoding tail-offset size mismatch (tracked in `16-varstring-encoding-fix`)
- [ ] Generate a scalar-only test message that compiles cleanly
- [ ] Encode: assert byte-exact equality with upstream `.sbe` fixture
- [ ] Decode: read official fixture, assert every field
- [ ] Round-trip: encode→decode→semantic-equal
- [ ] `TryFrom<&'a [u8]>` impl on decoders (idiomatic entrypoint)
- [ ] `acting_version()` and `acting_block_length()` exposed on decoders
- [ ] `#[must_use]` on encoder types (dropped encoder = lost message)
- [ ] `const fn` on all primitive scalar accessors
- [ ] Nullify-on-wrap: `wrap_and_apply_header` writes null sentinels at optional field offsets
- [ ] `wrap_and_apply_header` returns `Result` (buffer-too-short, not panic)

Ref: `design/DECISIONS.md` §2–3, §11 slices 2a, tests 1–2, 10.

## Verification strategy (byte-perfect against upstream)

Golden reference: `simple-binary-encoding/rust/car_example_baseline_data.sbe`
(Java `sbe-tool` output — canonical bytes).

| Step | What | How |
|------|------|-----|
| 1. Decode fixture | Read `.sbe` with ErgoSBE decoder, assert every field = known value | `CarDecoder::try_from(&fixture_bytes)?.model_year() == 2013` |
| 2. Encode → byte compare | Hard-code known values, encode, `assert_eq!(our_bytes, fixture_bytes)` | Headline: ErgoSBE writes canonical wire (upstream Rust only checks semantic) |
| 3. Cross-tool round-trip | Decode fixture with upstream decoder, re-encode with ErgoSBE, compare | Or: encode with ErgoSBE, decode with upstream, assert fields match |
| 4. Self round-trip | ErgoSBE encode → ErgoSBE decode → semantic equal | Internal consistency |

# Domain DTOs

Use when you want **owned** values (`Vec` groups, owned tails) and simple
structs — **not** the zero-copy hot path. Flyweights stay faster for
low-latency applications.

For re-encode, eligible flat groups are bulk-written directly from
`&[EntryDomain]`: no temporary `Vec<Entry>` and no encode-time allocation.
Eligibility requires fixed-size entries whose domain fields have the same wire
representation; nested groups, var-data, optional/versioned fields, configured
domain conversions, and bool remapping use the general `add` path. Integer
min/max checks are preserved in both paths.

On the audited Apple M4 1,000-entry fixture, automatic DTO bulk encode measured
509 ns versus 1.336 µs for the exact previous per-entry path with LTO, and
509 ns versus 1.998 µs without LTO. This is a DTO-to-DTO diagnostic, not an
ergon/sbe-tool fairness ratio.

```rust,no_run
// build.rs — DomainVarData picks the DTO field type for var-data:
// .with_domain_objects(DomainVarData::Strings) // String (invalid UTF-8 → InvalidUtf8 error)
// .with_domain_objects(DomainVarData::Bytes)        // Vec<u8> (byte-exact)
```

**Generated shape** (illustrative — your names follow your schema):

```text
pub struct QuoteDomain {
    pub seq: u32,
    pub some_numbers: [u32; 4],
    pub vehicle_code: [u8; 6],
    pub qty: u32,
    pub legs: Vec<QuoteLegsEntryDomain>,
    pub note: Vec<u8>,              // Bytes|String per DomainVarData
}
impl QuoteDomain {
    // Named methods, not TryFrom/From: two fallible sources (decoder vs framed
    // slice+offset), and materialisation is never infallible.
    pub fn try_from_decoder(dec: QuoteDecoder<'_>) -> Result<Self, DecodeError>;
    pub fn try_from_slice_with_header(buf: &[u8], offset: usize) -> Result<Self, DecodeError>;
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, EncodeError>;
    pub fn encoded_length_with_header(&self) -> Result<usize, EncodeError>;
}
```

**Wire → DTO → wire round-trip** (the docs fixture uses `DomainVarData::Bytes`):

```rust,no_run
  // Encode a message first (the usual flyweight path)
  let mut buf = [0u8; QuoteEncoder::compute_length_with_header(1, 2)];
  let len = QuoteEncoder::wrap_and_apply_header(&mut buf, 0)
      .fixed(&QuoteFixedFields {
          seq: 1,
          some_numbers: [1, 2, 3, 4],
          vehicle_code: *b"ABCDEF",
          qty: 10,
      })
      .legs(1, |legs| {
          legs.add(|leg| { leg.value(99); Ok(()) })?;
          Ok(())
      })?
      .note(b"hi")?
      .encoded_length_with_header();
  // Decode → owned DTO (allocates — not for the hot path)
  let dec = QuoteDecoder::try_from(&buf[..len])?;
  let mut dto = QuoteDomain::try_from_decoder(dec)?;
  assert_eq!(dto.seq, 1);
  assert_eq!(&dto.note, b"hi");
  dto.qty = 500;
  // Re-encode (integer min/max checked; eligible groups use bulk write)
  let n = dto.encode(&mut buf)?;
  assert_eq!(n, len);
```

### `with_domain_objects(DomainVarData)`

SBE `<data>` is length-prefixed **bytes**. The enum picks the DTO field type:

| Call | Field type | Invalid UTF-8 | When to use |
|------|------------|---------------|-------------|
| `.with_domain_objects(DomainVarData::Strings)` | `String` | **`InvalidUtf8` error** (strict; 0.1.10) | Text schemas when validity is known |
| `.with_domain_objects(DomainVarData::Bytes)` | `Vec<u8>` | n/a (raw copy) | Binary tails or **byte-exact** re-encode |

**`LossyStrings` rejects invalid UTF-8.** Materialise returns `InvalidUtf8`
for bad bytes; there is no silent empty-string fallback. Use `Bytes` (or stay
on flyweights) when you need audit / replay fidelity of non-UTF-8 tails.

Runnable demo (text path):
[sbe-feature-tour](https://github.com/mimran1980/ergon/tree/main/samples/sbe-feature-tour)
uses `DomainVarData::Strings`. Flyweight path is unchanged: with schema
`characterEncoding="UTF-8"` you still get `into_manufacturer_as_str()` without
a DTO.

[demo_car_domain_dto](https://github.com/mimran1980/ergon/blob/main/samples/sbe-feature-tour/src/lib.rs) ·
[domain_objects_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs).

# Migration guide: 0.1.x → 0.1.10

## Why a minor-as-major bump

Pre-1.0 Cargo treats the **minor** component as the compatibility boundary.
0.1.10 changes safe constructor semantics (fallible by default) and removes
`try_wrap*` aliases. That is intentionally a **breaking** release.

## API renames

| 0.1.x | 0.1.10 |
|-------|--------|
| `Encoder::try_wrap` | `Encoder::wrap` → `Result` |
| `Encoder::try_wrap_and_apply_header` | `Encoder::wrap_and_apply_header` → `Result` |
| `Encoder::wrap` (infallible, unsafe extent) | Safe `Encoder::wrap` → `Result` (zero-check core is **private** until HFT-008 keep=true) |
| `Encoder::wrap_and_apply_header` (infallible) | Safe `Encoder::wrap_and_apply_header` → `Result` (private `*_unchecked` core) |
| `Decoder::try_wrap_and_apply_header` | `Decoder::decode` |
| `Decoder::wrap` (infallible) | `Decoder::wrap` → `Result` (version-aware min fixed extent) |
| `Domain::from(decoder)` / `.into()` | `Domain::try_from_decoder(decoder)?` (or `try_from_slice_with_header` from framed bytes). Named methods, not `TryFrom`/`From`: two fallible sources; conversion is never infallible. |
| Concrete domain getters `fn price(&self) -> Decimal` | `fn try_price(&self) -> Result<…>` |

## Pattern updates

```rust
// Encode (checked)
let len = CarEncoder::wrap_and_apply_header(&mut buf, 0)?
    .fixed(&fields)
    .bids(0, |_| Ok(()))?
    .asks(0, |_| Ok(()))?
    .symbol(b"IBM")?
    .encoded_length_with_header();

// Decode (checked)
let car = CarDecoder::decode(&buf[..len], 0)?;
// or
let car = CarDecoder::try_from(&buf[..len])?;

// Trusted zero-check twins are module-private until HFT-008 records keep=true
// in docs/evidence/unchecked-keep-manifest.json (instruction + multi-run proof).
// Call the checked constructors; size the claim with EncodedLength so the cold
// extent check is a single predicted-not-taken branch on the hot path.
let enc = CarEncoder::wrap_and_apply_header(claim, 0)?;
```

## Behaviour changes

- Safe constructors **never** perform unchecked OOB pointer I/O.
- Optional null writes use exact primitive width in schema endianness (group
  optionals no longer panic on 1/2/4-byte fields).
- Domain string materialisation (`LossyStrings`) **rejects** invalid UTF-8 with
  `InvalidUtf8` instead of inventing `""`.
- Public safe `read_bytes_unchecked` / `write_bytes_unchecked` are gone (private
  `unsafe` only).

## Docs

- Compatibility claim: [`docs/SBE_COMPATIBILITY.md`](SBE_COMPATIBILITY.md)
- Book: [ergo-sbe book](https://mimran1980.github.io/ergon/)

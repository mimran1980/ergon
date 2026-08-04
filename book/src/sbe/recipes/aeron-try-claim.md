# Aeron try_claim Integration

ergo-sbe's exact buffer sizing is designed to work directly with Aeron's
`try_claim` — size the message first, claim exactly that many bytes from the
publication, and encode straight into the claimed buffer. No oversize scratch
buffer, no copy.

## Pattern

```rust,ignore
use ergo_sbe::HeartbeatEncoder;

// 1. Exact size before claiming — const, no allocation.
const HB_LEN: usize = HeartbeatEncoder::compute_length_with_header();
let header_len = 8; // Aeron message framing header

// 2. Claim exactly enough space.
let claim = publication
    .try_claim(header_len + HB_LEN as i32)
    .expect("claim failed");

// 3. Encode into an exact-length claim slice.
// Fixed-only messages also get `wrap_into_claim` when the buffer is *exactly*
// ENCODED_LENGTH (no Aeron framing prefix in the slice).
let slot = &mut claim.buffer_mut()[header_len..header_len + HB_LEN];
HeartbeatEncoder::try_wrap_and_apply_header(slot, 0)?
    .fixed(&HeartbeatFixedFields { sequence: 7, timestamp: 0 });

// 4. Commit — Aeron sends it.
claim.commit()?;
```

For **variable-length** messages (groups / var-data), there is **no**
`wrap_into_claim` — that helper is fixed-only. Size with the staged
`EncodedLength` builder, claim that exact length, then
`try_wrap_and_apply_header` on a slice of length `len`:

```rust,ignore
let len = CarEncoder::compute_length()
    .fuel_figures_ragged(2, |ff| {
        ff.add()?.usage_description(5)?;  // "Urban"
        ff.add()?.usage_description(7)?;  // "Highway"
        Ok(())
    })?
    .performance_figures_ragged(0, |_| Ok(()))?
    .manufacturer(5)?
    .model(9)?
    .activation_code(6)?
    .encoded_length_with_header();

let claim = publication.try_claim(header_len + len as i32)?;
let slot = &mut claim.buffer_mut()[header_len..header_len + len];
assert_eq!(slot.len(), len); // claim boundary == EncodedLength
let written = CarEncoder::try_wrap_and_apply_header(slot, 0)?
    .fixed(&fields)
    // ... groups/var-data ...
    .manufacturer(b"Honda")?
    .encoded_length_with_header();
debug_assert_eq!(written, len);
claim.commit()?;
```

## Why this works

- `compute_length_with_header()` and the staged `EncodedLength` builder give the
  exact byte count before any byte is written — no guesswork, no oversized
  scratch `vec![0u8; 4096]`.
- `wrap_into_claim` (fixed messages only) requires `buf.len() == ENCODED_LENGTH`
  and returns `ClaimLengthMismatch` otherwise.
- For ragged messages, **you** own the claim length from EncodedLength; the
  encoder still validates capacity via `try_wrap_*`.
- The encoder writes directly into the slice you hand it — the claim buffer IS
  the encode buffer.
- The `encoded_length_with_header()` return value on the terminal encoder stage
  is a diagnostic assertion that the claimed size matches the actual written
  size — use it in tests or debug builds.

The [cluster-ha-orderbook sample](https://github.com/mimran1980/ergon/tree/main/samples/cluster-ha-orderbook)
demonstrates this pattern in the context of a real Aeron Cluster publication
loop.

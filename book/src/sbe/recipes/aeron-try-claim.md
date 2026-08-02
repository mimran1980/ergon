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

// 3. Encode directly into the claimed buffer.
HeartbeatEncoder::wrap_and_apply_header(&mut claim.buffer_mut()[header_len..], 0)
    .fixed(&HeartbeatFixedFields { sequence: 7, timestamp: 0 });

// 4. Commit — Aeron sends it.
claim.commit()?;
```

For variable-length messages, size first with the staged `EncodedLength` builder:

```rust,ignore
let len = CarEncoder::compute_length()
    .fuel_figures_ragged(2, |ff| {
        ff.add()?.usage_description(5)?;  // "Urban"
        ff.add()?.usage_description(7)?;  // "Highway"
        Ok(())
    })?
    .manufacturer(5)?
    .activation_code(6)?
    .encoded_length_with_header();

let claim = publication.try_claim(header_len + len as i32)?;
CarEncoder::wrap_and_apply_header(&mut claim.buffer_mut()[header_len..], 0)?
    .fixed(&fields)
    // ...
    .manufacturer(b"Honda")?
    .encoded_length_with_header();
claim.commit()?;
```

## Why this works

- `compute_length_with_header()` and the staged `EncodedLength` builder give the
  exact byte count before any byte is written — no guesswork, no oversized
  scratch `vec![0u8; 4096]`.
- The encoder writes directly into the slice you hand it — the claim buffer IS
  the encode buffer.
- The `encoded_length_with_header()` return value on the terminal encoder stage
  is a diagnostic assertion that the claimed size matches the actual written
  size — use it in tests or debug builds.

The [cluster-ha-orderbook sample](https://github.com/mimran1980/ergon/tree/main/samples/cluster-ha-orderbook)
demonstrates this pattern in the context of a real Aeron Cluster publication
loop.

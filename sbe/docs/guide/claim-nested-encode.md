# Claim-shaped encode and nested SBE messages

Canonical hot-path pattern used by the IPC and cluster samples:
**exact length → claim → encode in place → commit**. No intermediate heap buffer
on the success path.

## Session framing (cluster)

`SessionMessageHeader` is a fixed message. Use generated constants:

```rust
use ergo_aeron_cluster::codecs::session::SessionMessageHeaderEncoder;

const HDR: usize = SessionMessageHeaderEncoder::ENCODED_LENGTH; // 32
// Application payload after the session header:
let app = SessionMessageHeaderEncoder::after_this_message(frame)?;
```

`AeronCluster::try_claim(payload_len)` writes the session header into the claim
and returns a writable slice of length `payload_len` for the app payload.

## Nested AppMessage → L2Book (samples)

```rust
let inner_len = L2BookEncoder::compute_encoded_length_with_message_header(
    bids.len(), asks.len(), symbol.len(),
);
let outer_len = AppMessageEncoder::compute_encoded_length_with_message_header(
    app_name.len(), inner_len,
);

// claim `outer_len` bytes, then:
let mut app = AppMessageEncoder::wrap_and_apply_header(buf, 0)?;
let after = app.app_name(app_name.as_bytes())?;
after.payload_with(inner_len, |payload| {
    let mut book = L2BookEncoder::wrap_and_apply_header(payload, 0)?;
    // fixed fields …
    // Closures may return `()` or `Result` — one method name (`bids` / `add`).
    book.bids(n, |g| {
        for level in bids {
            g.add(|e| {
                let _ = e.price_wire(dec).size_wire(sz);
                Ok::<(), sbe_rt::EncodeError>(())
            })?;
        }
        Ok::<(), sbe_rt::EncodeError>(())
    })?;
    // asks + symbol …
    Ok(())
})?;
```

### Rules

1. **Length first.** Use `compute_encoded_length_with_message_header` for every
   nested message; never guess claim size.
2. **`payload_with(exact_len, …)`** for nested SBE (writes var-data length +
   lends the slice). Same idea as decoder `into_*` / `into_*_as_message`.
3. **Group `add` / `bids` / …** — closures may return `()` or `Result` (no
   parallel `try_*` names).
4. **Decimals:** with `enable_decimal_converters("Decimal")`, wire setters are
   `price_wire(Decimal)` and generic converters are `price::<D: SbeDecimal>(…)`.
5. **Flyweight vs eager composites:** `engine()` is zero-copy; `engine_as_struct()`
   copies `[u8; N]`. Prefer flyweight on the hot path.

## Shared `sbe_rt` across schemas

By default each generated file inlines `sbe_rt`. For multi-schema crates that
need **one** `EncodeError` type:

1. Generate a small shared module with only types + runtime (or first schema).
2. Subsequent modules:  
   `GenerationConfig::new("other").with_external_sbe_rt("crate::shared::sbe_rt")`.

`generate_multi` with `shared_module` already emits `sbe_rt` once in the first
module and re-exports it for siblings.

## Reference code

| Path | Role |
|------|------|
| `samples/cluster-ha-orderbook/src/publish.rs` | Cluster try_claim + nested AppMessage |
| `samples/advanced-bitget/src/publication.rs` | IPC claim + nested AppMessage |
| `cluster/src/client.rs` | `AeronCluster::try_claim` |

## See also

- [generated-api.md](generated-api.md) — stages, `as_bytes`, errors
- [advanced.md](advanced.md) — multi-schema, decimal converters
- [DECISIONS.md](../../design/DECISIONS.md) — wire order, complete-stage APIs

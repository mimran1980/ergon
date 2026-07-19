# ergosbe (`sbe/`)

SBE XML → idiomatic Rust codec generator. Core pillar of the ErgoSBE umbrella.

## Status

**Experimental product crate.** Maintained ErgoSBE vs Aeron SBE matrix is green
(10/10 ≤ 1.00 as of 2026-07-18). Not a universal “HFT-ready” claim beyond that set.

Verified-open items only: [`../docs/LIVING_BACKLOG.md`](../docs/LIVING_BACKLOG.md).

## Depends on

- Rust MSRV **1.95** (workspace)
- Official SBE semantics / wire shape (see design authority below)

## Build / test

```sh
cargo test -p ergosbe --lib
cargo test -p ergosbe --test baseline_test
cargo bench -p ergosbe-benchmarks --no-run   # from repo root
just bench                                   # Aeron parity matrix
```

## Public entry points

- `parse` / `parse_file` → IR
- `Schema::from_ir`
- `GenerationConfig` + `Generator::try_generate` / `generate` / `generate_multi`
- `GenerationConfig::enable_decimal_converters` / `with_external_sbe_rt`
- Typical consumer: call from **your** `build.rs`, `include!` from `OUT_DIR`

## Claim + nested encode (sample shape)

```rust
// Fixed session header framing
let hdr = SessionMessageHeaderEncoder::ENCODED_LENGTH;
let app = SessionMessageHeaderEncoder::after_this_message(frame)?;

// Nested AppMessage → L2Book into a claimed buffer
let inner = L2BookEncoder::compute_encoded_length_with_message_header(n_b, n_a, sym_len);
let outer = AppMessageEncoder::compute_encoded_length_with_message_header(name_len, inner);
// claim `outer` bytes, then:
let mut app = AppMessageEncoder::wrap_and_apply_header(buf, 0)?;
let after = app.app_name(name)?;
after.payload_with(inner, |p| {
    let mut book = L2BookEncoder::wrap_and_apply_header(p, 0)?;
    book.bids(n_b as u16, |g| {
        for level in bids {
            g.add(|e| { let _ = e.price_wire(px).size_wire(sz); Ok::<(), _>(()) })?;
        }
        Ok::<(), _>(())
    })?;
    Ok(())
})?;
```

Full recipe: [`docs/guide/claim-nested-encode.md`](docs/guide/claim-nested-encode.md).

## Layout

| Path | Role |
|------|------|
| `src/xml.rs`, `schema.rs` | Parse / validate SBE XML |
| `src/ir.rs`, `resolve.rs` | Intermediate representation + offsets |
| `src/config.rs` | Generation options |
| `src/codegen.rs` | Rust source generation (`syn` / `quote` / `prettyplease`) |
| `design/DECISIONS.md` | Canonical design authority |
| `docs/guide/` | Getting started, schema authoring, generated API, claim/nested |
| `tests/` | Wire, golden, compile-fail, allocation proofs |

## Where truth lives

- Design: [`design/DECISIONS.md`](design/DECISIONS.md)
- Guide: [`docs/guide/getting-started.md`](docs/guide/getting-started.md)
- Claim / nested: [`docs/guide/claim-nested-encode.md`](docs/guide/claim-nested-encode.md)
- Perf ledger: [`../ergosbe-performance-optimisation-goal.md`](../ergosbe-performance-optimisation-goal.md)
- Crate rustdoc: `cargo doc -p ergosbe --open`

## Non-goals

- Nightly-only APIs, speculative SIMD bulk copy, broad per-field unchecked families
- Transmute / native-endian casts from wire buffers
- Hand-editing generated sample codecs instead of regenerating from XML

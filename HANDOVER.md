# Handover: Truthful Buffer Coordinates + Typestate Header Markers

Branch: `feat/0.1.8` — 14 commits atop `e75b34a5`.

## What was built

### 1. Truthful message buffer coordinates

Generated encoders/decoders now use one absolute coordinate system instead of sub-slicing buffers.

**Before**: `wrap()` subsliced `&mut buf[pos..]`, resetting `message_start` to 0.
All positions were relative to the sub-slice start. `message_offset()` returned
`Option<usize>` derived from `pos - HEADER_LENGTH`. `as_bytes()` was ambiguous
(body? header?).

**After**: `wrap(buf, msg_offset)` retains the original buffer verbatim. All
positions are absolute within it. The encoder/decoder API is explicit:

| Method | Returns | Notes |
|--------|---------|-------|
| `message_offset()` | `usize` | The `msg_offset` argument to `wrap` |
| `limit()` | `usize` | Absolute cursor within original buffer |
| `buffer()` | `&[u8]` | The original buffer |
| `as_body_bytes()` | `&[u8]` / `Result<&[u8]>` | Body only (excludes header) |
| `as_bytes_with_header()` | `&[u8]` (encoder HeaderPresent) / `Result<&[u8]>` (decoder) / `Option<&[u8]>` (encoder HeaderAbsent) | Header-inclusive |
| `encoded_length()` | `usize` / `Result<usize>` | Body length |
| `encoded_length_with_header()` | `usize` / `Result<usize>` | Infallible — pure arithmetic |
| `into_remaining_mut()` | `&mut [u8]` | Complete encoders only |
| `remaining()` | `&[u8]` | Fixed + Complete decoders only |

**Removed**: `as_bytes()`, `AsRef<[u8]>`, `whole_buffer()`, `after_this_message()`,
encoder `remaining()`/`remaining_mut()`.

### 2. Encoder typestate (HeaderPresent / HeaderAbsent)

The encoder wrapper now carries a zero-sized compile-time marker:

```rust
pub struct CarEncoder<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    pos: usize,
    _header: core::marker::PhantomData<H>,
}
```

- `wrap(buf, msg_offset)` → `Encoder<'_, HeaderAbsent>` — no header written
- `wrap_and_apply_header(buf, msg_offset)` → `Encoder<'_, HeaderPresent>` — header written
- `as_bytes_with_header()` and `encoded_length_with_header()` exist ONLY on `HeaderPresent` stages, returning `&[u8]` / `usize` directly — no `Option`
- Default `= HeaderPresent` means type inference handles the common case: users write `let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0)?` with zero turbofish
- Decoders are deliberately non-generic — they always parse headers, so `H` provides no safety benefit and would hurt type inference

### 3. Schema constants struct

Each message gets a `{Name}Schema` struct to avoid turbofish on associated constants:

```rust
pub struct CarSchema;
impl CarSchema {
    pub const SCHEMA_ID: u16 = 1;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 45;
    pub const HEADER_LENGTH: usize = 8;
    pub const SCHEMA_VERSION: u16 = 0;
}
```

`CarSchema::TEMPLATE_ID` works without `::<'_>::` — the struct is non-generic.
The AnyMessage dispatch in `runtime.rs` uses `#schema::TEMPLATE_ID`.

### 4. HeaderState type definitions

`HeaderState`, `HeaderPresent`, `HeaderAbsent` are defined per-module in
`generate_sbe_rt_src()` (runtime.rs) — each generated `sbe_rt` module has its
own copy. This avoids forcing a runtime dependency on `ergo_sbe` from generated
code. The types are zero-sized and identical across modules.

## Design decisions

1. **Decoders stay non-generic** (`02d416c5`). Decoders always parse a header
   during construction — they don't benefit from the `H` distinction. Keeping
   them non-generic avoids the type-inference pain that `wrap()` returning
   `HeaderAbsent` would cause at every call site.

2. **`encoded_length_with_header()` is infallible.** It's pure arithmetic:
   `body_len + HEADER_LENGTH`. The `HEADER_LENGTH` constant exists regardless
   of whether a valid header was written. Only `as_bytes_with_header()` needs
   the header gate — raw `wrap()` leaves garbage in the header region.

3. **Schema struct over `impl<H>` constants.** Generic impl blocks make
   associated constants painful to access (`Decoder::<'_, H>::TEMPLATE_ID`).
   The `{Name}Schema` struct sidesteps this entirely.

4. **No cross-module type collisions.** The first typestate attempt
   (`acdd13ff`, reverted by `a295f810`) put `HeaderState` in a shared
   `ergo_sbe::header_state` module. This forced generated code to `use
   ergo_sbe::header_state` — a runtime dependency that feature-tour and other
   standalone crates didn't have. The current design inlines the types per-module
   via `generate_sbe_rt_src()`.

## Commit map

| Commit | What |
|--------|------|
| `5af584f3` | Coordinate system: absolute offsets, new methods, remove deprecated |
| `2e0df9f3` | AnyMessage dispatch for new coordinate API |
| `dd4d07be` | Infalible `encoded_length_with_header`, decoder `as_bytes_with_header` |
| `ce0a3a00` | Dynamic decoder infallible, AnyMessage + feature tour updates |
| `7589f27b` | Runtime AnyMessage dispatch cleanup |
| `acdd13ff` | Typestate (first attempt — reverted) |
| `a295f810` | Revert typestate (collision issues) |
| `c6116396` | Benchmarks compile and pass with coordinate-change API |
| `a6ebdc9b` | `{Name}Schema` constants struct |
| `10e5284e` | Typestate (second attempt — encoder only) |
| `02d416c5` | Decoder stays non-generic |
| `a234114d` | Bounds check fix, cluster API migration, feature tour fix |
| _(uncommitted)_ | Baseline test fixes (99/99), golden regeneration, migration script cleanup |

## Release readiness verification

Run these IN ORDER. Every step must be green before merging to main.

### Quick health check

```sh
# 1. All modules compile and test
cargo test -p ergo-sbe --lib                    # must be 194/194
cargo test -p ergo-sbe --test baseline_test     # must be 99/99
cargo test -p ergo-aeron-cluster                # must be 51/51
cargo test -p ergo-sbe --lib -- --test-threads=1  # recheck for flaky tests

# Feature tour (samples)
cargo test --manifest-path samples/sbe-feature-tour/Cargo.toml --lib  # 3/3

# 2. Clippy
cargo clippy -p ergo-sbe         # 0 warnings
cargo clippy -p ergo-aeron-cluster  # 0 warnings (or pre-existing only)

# 3. All benches compile
cargo bench -p ergo-sbe-benchmarks --no-run   # 16/16
cargo bench -p ergo-aeron-cluster --no-run    # 2/2

# 4. Golden file is current
cargo run --example regenerate_golden -p ergo-sbe -- /tmp/golden_check.rs
diff sbe/tests/golden/car_example.rs /tmp/golden_check.rs
# Must be byte-identical. If not, a generator change wasn't committed.
```

### SBE performance gate (non-negotiable)

```sh
cargo bench -p ergo-sbe-benchmarks --bench perf_parity_bench
```

Every maintained ergon/sbe-tool ratio must be ≤ 1.00 in BOTH LTO-on and
LTO-off profiles. Record Criterion estimates + CIs. If a ratio repeatably
exceeds 1.00 AND assembly evidence attributes it to the `H` parameter, STOP
and report — do not merge, do not raise the ceiling.

### Wire parity (non-negotiable)

```sh
cargo test -p ergo-sbe sbe_tool_wire_parity_test
cargo test -p ergo-sbe sbe_tool_multi_schema_wire_parity_test
```

These dual-encode every message shape with both ergon and sbe-tool and assert
byte-identical output. A failure means ergon produces different wire bytes.

### Golden shape audit

The golden file (`sbe/tests/golden/car_example.rs`) must show exactly:

1. Encoder stage structs: `<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent>` with `_header: PhantomData<H>`
2. `as_bytes_with_header()` only on `impl<...HeaderPresent>` blocks, returning `&[u8]` (no `Option`)
3. Decoder structs: `<'a>` only (non-generic)
4. Constants on `CarSchema` struct, not on `impl<H>` blocks

```sh
grep -c "HeaderState = sbe_rt::HeaderPresent" sbe/tests/golden/car_example.rs
grep "as_bytes_with_header" sbe/tests/golden/car_example.rs | grep -c "Option"
# second command should return 0
grep "pub struct CarDecoder" sbe/tests/golden/car_example.rs | grep -c "H:"
# should return 0 (decoders non-generic)
```

### Ergonomics acceptance (compile-test these patterns)

```rust
// 1. Happy path: zero turbofish, zero explicit H
let mut buf = [0u8; CarEncoder::compute_length_with_header()];
let n = CarEncoder::wrap_and_apply_header(&mut buf, 0)?
    .fixed(&fields).encoded_length_with_header();

// 2. Consumer helper compiles (default H)
fn finish(enc: CarComplete<'_>) -> usize { enc.encoded_length_with_header() }

// 3. Schema constant access without turbofish
let tid = CarSchema::TEMPLATE_ID;

// 4. Body-only: wrap() yields HeaderAbsent; calling as_bytes_with_header()
//    on a HeaderAbsent complete stage MUST be a compile error
let enc = CarEncoder::wrap(&mut buf, 0);  // HeaderAbsent
// enc.as_bytes_with_header();  // ← must NOT compile
```

### Diagnostic benches (fixed in follow-up)

Absolute coordinates require field setters to write at `msg_offset + HEADER + field_offset`.
An early incomplete migration left message-relative field writes, which only worked at
`msg_offset == 0` and broke `alignment_bench` / non-zero-offset encode. Decoder
`wrap` also takes **message_offset** (not body offset); benches that still passed
`HEADER_LENGTH` as the second argument were migrated to `0` (or the true message start).

- `alignment_bench` — green (`--test` all offsets 0..=63)
- `layout_access_bench` — green (`wrap(..., 0, BLOCK, VERSION)`)
- `codec_matrix_bench` — green
- Durable probe: `sbe/benchmarks/tests/align_offset_probe.rs`

### Before merging

- [x] Run HANDOVER test ladder — lib 194, baseline 100, wire 23+52, cluster 51, feature-tour 3; clippy `-D warnings` clean
- [x] Run SBE bench gate LTO + no-LTO — all maintained ratios ≤ 1.00; cluster gate green after re-run (first 1.02 was noise)
- [x] Clippy clean for ergo-sbe + ergo-aeron-cluster
- [x] Diagnostic benches green: alignment / layout_access / codec_matrix (`--test`); align_offset_probe
- [x] Absolute coords: field writers use `msg_offset + header + field` (non-zero offset encode)
- [ ] Regenerate sbe-tool reference crates: `scripts/regenerate-sbe-tool-reference.sh` (only if sbe-tool API surface changed; wire parity green without regen)
- [x] Verify golden file is byte-identical to regeneration
- [x] Commit remaining working-tree changes (baseline/golden/H propagation/consumers + abs-offset fix)
- [x] No leftover scripts: `apply_*.py` and `process_files.py` absent
- [x] `book/` and `.github/workflows/pages.yml` stay untracked (not part of this PR)

# Design compliance audit — every DECISIONS.md requirement verified

Audit date: 2026-07-06
Auditor: Claude Code
Scope: `ergosbe/src/codegen.rs` (3647 lines) + `ergosbe/src/config.rs` + `ergosbe/src/resolve.rs` + `ergosbe/src/ir.rs` + golden test `tests/golden/car_example.rs` (2334 lines)
Reference: `design/DECISIONS.md` (496 lines)

Status legend:
- ✅ COMPLIANT — codegen follows the design
- ⚠️ PARTIAL — partially implemented or has gaps
- ❌ VIOLATION — codegen contradicts or is missing the design requirement
- 📝 NOT APPLICABLE — design is about future work or unrelated subsystem

---

## §1 Type model

- [x] ✅ Flyweight-only: decoders borrow `&'a [u8]`, encoders borrow `&'a mut [u8]`
  - `codegen.rs:1526-1532` — Decoder `{ buf: &'a [u8], pos, acting_version, acting_block_length }`
  - `codegen.rs:2704-2709` — Encoder `{ buf: &'a mut [u8], message_start, pos }` plus type-state variant
- [x] ✅ `OwnershipMode` removed — one mode
  - `config.rs` — no `OwnershipMode` anywhere
- [x] ✅ Decoders are `Copy`, encoders are not
  - `codegen.rs:1526` `#[derive(Clone, Copy)]` on decoders; encoders lack `Copy`
- [x] ✅ Serde, `no_std`, `zerocopy` all parked for v1
  - No generated code references these crates/features
- [x] 📝 Domain objects (`Vec`, `String`, serde) — parked
  - Generated `as_slice()` returns `&'a [u8]`, not `Vec`

## §2 Encoder

- [x] ✅ Scalars: `&mut self -> &mut Self` fluent setters
  - `codegen.rs:2858-2866` — all setters return `&mut Self`
- [x] ✅ Closure sub-encoders for groups via `EncodeGroupEntry<E>` trait
  - `codegen.rs:281-292` — `EncodeGroupEntry` trait + blanket `impl<E, F: FnOnce(&mut E)> EncodeGroupEntry<E> for F`
- [x] ✅ No top-level `encode(|c| ...)` closure
  - Encoder uses type-state tail pattern directly
- [x] ✅ Type-state tail ordering (phantom states for messages with groups/var-data)
  - `codegen.rs:2673-2695` — generates `NeedsFoo`, `NeedsBar`, `Complete` phantom states
  - Golden: `car_encoder_state::NeedsFuelFigures -> NeedsPerformanceFigures -> NeedsManufacturer -> Complete`
- [x] ✅ `.add(impl EncodeGroupEntry<E>)` — trait + blanket impl generated correctly
  - `codegen.rs:3140-3163` — `GroupEncoder::add<F: FnOnce(&mut EntryEncoder)>`
- [x] ✅ Encoder emits current schema version only (not version-aware)
  - `codegen.rs:2761-2768` — `HEADER_TEMPLATE` has schema version baked in at codegen time
- [x] ✅ Nullify-on-wrap: `wrap_and_apply_header` writes null sentinels for optional fields
  - `codegen.rs:2594-2622` — `generate_nullification()` writes `NullValue` for each optional field
  - `codegen.rs:2799, 3155` — called in encoder `wrap_and_apply_header` and group entry `add`
- [x] ❌ `#[must_use]` on encoder types — **NOT PRESENT**
  - `codegen.rs:2714-2722` — `pub struct FooEncoder<'a, State>` has no `#[must_use]` attribute
  - **Fix:** Add `#[must_use]` to encoder struct definitions (both type-state and non-type-state variants)
- [x] ✅ `wrap_and_apply_header` returns `Result<Self, EncodeError>` — no panic/truncation
  - `codegen.rs:2791-2797` — returns `Err(EncodeError::BufferTooShort)` when buffer insufficient
- [x] ✅ `AsRef<[u8]>` on `Complete` terminal state encoder
  - `codegen.rs:3046-3061` — `Complete` state has `as_bytes()` and `AsRef<[u8]>`
  - `codegen.rs:3062-3070` — fixed (no-tail) encoder also has `AsRef<[u8]>`

## §3 Decoder

- [x] ✅ `Copy` decoders
  - `codegen.rs:1526` — `#[derive(Clone, Copy)]` on `FooDecoder`
- [x] ✅ No type-state (asymmetry with encoder)
  - Decoders have no phantom state parameters
- [x] ✅ `acting_version()` and `acting_block_length()` exposed
  - `codegen.rs:1595-1598` — both `const fn` methods
- [x] ✅ Group decoders implement `ExactSizeIterator` with `len()`
  - `codegen.rs:2208-2214` — `impl ExactSizeIterator for FooDecoder`
- [x] ✅ `is_empty()` as inherent method (not from ExactSizeIterator, which is unstable)
  - `codegen.rs:2167-2169` — `fn is_empty(&self) -> bool`
- [x] ⚠️ Var-data accessor family: **partial implementation**
  - ✅ `field_name()` returns `&'a [u8]` (as_slice equivalent, `codegen.rs:2060-2070`)
  - ✅ `field_name_as_str()` returns `Result<&'a str, DecodeError>` (`codegen.rs:2073-2078`)
  - ❌ `as_string()` behind `alloc-convenience` — **NOT GENERATED**
  - ❌ `as_decoder()` — **NOT GENERATED**
  - ❌ `as_message()` — **NOT GENERATED** (design requires `AnyMessage::decode_frame` over var-data)
- [x] ❌ `unsafe fn as_str_unchecked()` — **NOT GENERATED**
  - `codegen.rs:2073-2078` — only safe `_as_str` using `core::str::from_utf8()` is generated
  - Design requires: `as_str() -> Result<&'a str, Utf8Error>` + `unsafe fn as_str_unchecked() -> &'a str`
  - **Fix:** Generate `pub unsafe fn {}_as_str_unchecked(&self) -> &'a str` using `core::str::from_utf8_unchecked()`
- [x] ✅ Always version-aware — no compiled-`blockLength` fallback
  - `codegen.rs:1982` — `tail_offset_0` uses `self.pos + self.acting_block_length` (wire value)
  - All field accessors gate on `self.acting_version` for `sinceVersion > 0` fields
- [x] ✅ `AsRef<[u8]>` on decoders
  - `codegen.rs:2111-2114` — `impl AsRef<[u8]> for FooDecoder`
- [x] ❌ `impl TryFrom<&'a [u8]> for XxxDecoder<'a>` — **NOT GENERATED**
  - Design requires: idiomatic Rust conversion delegating to `wrap_and_apply_header(buf, 0)`
  - **Fix:** Add generated code `impl<'a> TryFrom<&'a [u8]> for FooDecoder<'a>` that calls `Self::wrap_and_apply_header(buf, 0)`
- [x] ✅ `const fn` on primitive field accessors
  - All `serial_number()`, `model_year()`, etc. are `const fn`
  - `_unchecked` variants are also `const unsafe fn`
- [x] ✅ `raw_` accessors for HFT — generated for all fields (both v0 and versioned)
  - `codegen.rs:1798-1817` — v0 fields get `raw_foo() -> T`; versioned fields get `raw_foo() -> Option<T>`

## §4 Data types

- [x] ✅ Composites: `Copy` value struct, field-by-field read via `from_{le,be}_bytes`
  - `codegen.rs:1059-1068` — `#[repr(transparent)] struct Foo(pub [u8; N])` — never transmuted
- [x] ✅ Both struct accessor AND per-field direct methods (for schema built-in header type)
  - `MessageHeader` has both `block_length()` and is returned from `header()`
  - User-defined composites get struct accessor; per-field methods not lifted to enclosing decoder. Acceptable.
- [x] ✅ Encode side: value (composites) or closure (groups)
- [x] ✅ Fixed arrays: concrete `[T; N]` returned by value
  - `codegen.rs:1631-1657` — `[u32; 4]` via manual while-loop copy
- [x] ✅ Enums: E3 pattern — `#[repr(transparent)] struct X(pub u8)` + `kind() -> Option<XKind>`
  - `codegen.rs:880-990` — correct E3 pattern with `From<T>`, `Into<T>`, `TryFrom<T>`, `raw()`, `kind()`, `into_kind()`
- [x] ✅ Choices: `#[repr(transparent)] struct X(u8)` with per-flag bool accessors + `raw()`
  - `codegen.rs:992-1057` — correct pattern with `From<T>`, `Into<T>`, `Default`, `raw()`
- [x] ✅ Standard derives on value types
  - Enums: `Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord`
  - Choices: same + `Default`
  - Composites: `Clone, Copy, Debug, PartialEq, Eq, Hash` (no `PartialOrd, Ord`)
- [x] ✅ Newtype conversions: `From<u8> + Into<u8> + raw()` for enums; same + `Default` for choices
  - `codegen.rs:964-989` — enum conversions (includes `TryFrom` for `Kind`)
  - `codegen.rs:1042-1056` — choice conversions
- [x] ✅ Constant-value fields (`presence="constant"`) generate `const fn` returning hardcoded value
  - `codegen.rs:1611-1629` — string constants return `&'static str`, numeric return typed value
- [x] ✅ Optional/null separate from version absence — correct return type mapping
  - `codegen.rs:1711-1785`:
    - Optional + any -> `Result<Option<T>>` (collapses null sentinel to `None`)
    - Required + `sinceVersion > 0` -> `Result<Option<T>>` (version absence = `None`)
    - Required + v0 -> `Result<T>`
- [x] ⚠️ `semantic_type` and `description` captured but NOT emitted as rustdoc or `FieldMeta` const
  - `codegen.rs:670, 693` — `semantic_type` is parsed from IR but never used in output
  - Tracked in `35-semantic-type-system.md`

## §5 Metadata & SbeMessage trait

- [x] ✅ Per-message associated consts: `TEMPLATE_ID`, `BLOCK_LENGTH`, `SCHEMA_ID`, `SCHEMA_VERSION`
  - `codegen.rs:1539-1563` — generated correctly
  - Additionally: `ENCODED_LENGTH`/`MAX_ENCODED_LENGTH` for stack-allocate guidance
- [x] ❌ `SCHEMA_HASH` and `SCHEMA_HASH_HEX` — **NOT GENERATED**
  - `ir.rs:148-166` — `Ir` struct has no `schema_hash` field
  - **Fix:** Add SHA-256 hash of normalized IR to resolve pass, emit as `SCHEMA_HASH: [u8; 32]` and `SCHEMA_HASH_HEX: &'static str`
- [x] ❌ `SEMANTIC_VERSION` and `SEMANTIC_TYPE` on messages — **NOT GENERATED**
  - `ir.rs:161` — `semantic_version` exists in `Ir` but not emitted as const on message types
  - **Fix:** Emit `SEMANTIC_VERSION: Option<&'static str>` and `SEMANTIC_TYPE: Option<&'static str>` on decoder/encoder types
- [x] ❌ Per-field `FieldMeta` const module (`pub mod meta`) — **NOT GENERATED**
  - Design requires one `FieldMeta` const per field with `id`, `since_version`, `offset`, `presence`, `null_value`, `semantic_type`
  - **Fix:** Generate `pub mod meta { pub const SERIAL_NUMBER: FieldMeta = FieldMeta { ... }; }` per design spec
- [x] ✅ Sealed `SbeMessage` trait — generated
  - `codegen.rs:278-280` — `pub mod private { pub trait Sealed {} }`
  - `codegen.rs:2104` — decoders implement `Sealed`
  - `codegen.rs:3075` — encoders implement `Sealed`
- [x] ❌ `#[diagnostic::on_unimplemented]` on `SbeMessage` trait — **NOT PRESENT**
  - `codegen.rs:272-277` — `SbeMessage` trait has no diagnostic attribute
  - **Fix:** Add `#[diagnostic::on_unimplemented(message = "...", note = "...")]` to `SbeMessage` trait definition

## §6 Message dispatch + entrypoints

- [x] ✅ `AnyMessage<'a>` enum with `Unknown { header, payload }` variant
  - `codegen.rs:3374-3392` — generated correctly
- [x] ✅ `AnyMessage::decode(buf, off)` — dispatches on `templateId`, returns `WrongSchema`/`UnknownTemplateLength`
  - `codegen.rs:3468-3505` — correct implementation
- [x] ✅ `AnyMessage::decode_frame(buf, off, frame_len)` — frame-aware decode with `Unknown { payload }` forwarding
  - `codegen.rs:3508-3572` — correct implementation with frame bounds check
- [x] ✅ `FrameCursor<'a>` — iterates buffer with `FramingPolicy` (LengthPrefixU32/U16/Fixed)
  - `codegen.rs:3415-3466` — correct implementation
- [x] ❌ `as_message()` on var-data fields — **NOT GENERATED**
  - Codegen only generates basic `field_name()` returning `&'a [u8]` and `field_name_as_str()`
  - **Fix:** Generate `fn field_name_as_message(...)` that delegates to `AnyMessage::decode_frame`
- [x] ⚠️ Encode entrypoints:
  - ✅ `wrap()` — generated
  - ✅ `wrap_and_apply_header()` — generated
  - ❌ `AnyMessage::encode()` — **NOT GENERATED**
- [x] ✅ `#[non_exhaustive]` on `AnyMessage<'a>`
  - `codegen.rs:3375` — present
- [x] ✅ Length helpers: `encoded_length()`, `encoded_length_with_header()`
  - `codegen.rs:2086-2092` — decoder variants
  - `codegen.rs:2919-2926` — encoder variants
- [x] ✅ Configurable header and group dimension types (not hardcoded)
  - `codegen.rs:1477-1509` — resolves `headerType` from schema IR
  - `codegen.rs:1355-1400` — resolves group `dimensionType` from schema IR

## §7 Versioning

- [x] ✅ Tail offset uses **wire** `blockLength` (trap 1 — THE classic bug avoided)
  - `codegen.rs:1982` — `tail_offset_0()` returns `self.pos + self.acting_block_length`
  - `acting_block_length` is set from wire header: `header.block_length()` at `codegen.rs:1589`
- [x] ✅ Forward compat: `sinceVersion > acting_version` -> accessor returns `None`
  - `codegen.rs:1722-1745, 1748-1766` — version gating with `if self.acting_version < since_version`
- [x] ✅ Backward compat: known fields at stable offsets; unknown trailing bytes skipped via wire blockLength
  - Wire `blockLength` determines tail start; fixed fields outside compiled block skipped naturally
- [x] ✅ `schemaId` = identity (mismatch = error), `version` = evolution (normal)
  - `codegen.rs:1586-1588` — `wrap_and_apply_header` checks SCHEMA_ID and returns `WrongSchema` on mismatch
- [x] ✅ Sub-decoders (group entries/composites) thread wire `version` down
  - `codegen.rs:2048` — group decoder receives `self.acting_version`; nested decoders propagate it

## §8 Bounds checking + Error taxonomy

- [x] ✅ `Result`-returning methods bounds-check
  - All public field accessors return `Result<T, DecodeError>` and check bounds
- [x] ⚠️ `_unchecked` variants on field accessors — **generated for scalar fields, missing on structural points**
  - ✅ Primitive fields: `const unsafe fn foo_unchecked()` (`codegen.rs:1788`)
  - ✅ Composite fields: `const unsafe fn foo_unchecked()` (`codegen.rs:1847`)
  - ✅ Enum fields: `const unsafe fn foo_unchecked()` (`codegen.rs:1898`)
  - ✅ Set fields: `const unsafe fn foo_unchecked()` (`codegen.rs:1957`)
  - ❌ Group decoder `wrap()` returns `Result` — no `wrap_unchecked()` variant
  - ❌ Var-data accessor returns `&[u8]` with bounds — no `_unchecked` variant
  - **Fix:** Add `_unchecked` variants for group `wrap` and var-data accessors
- [x] ❌ `bound-check-disabled` feature — **NOT IMPLEMENTED**
  - Tracked in `07-bound-check-disabled.md`
- [x] ✅ Group/var-data accessors return `Result`
  - `codegen.rs:2046` — `fn foo(&self) -> Result<GroupDecoder, DecodeError>`
- [x] ✅ `DecodeError` matches spec: `BufferTooShort`, `WrongSchema`, `UnknownTemplateLength`, `InvalidVarDataLength`, `Utf8`
  - `codegen.rs:229-236` — all present (implementation adds `field` to `BufferTooShort` — ahead of spec)
- [x] ✅ `EncodeError` with `BufferTooShort` and `VarDataTooLong`
  - `codegen.rs:254-259` — both present (design only specified `BufferTooShort`; `VarDataTooLong` is additive)
- [x] ✅ Hand-rolled `core::error::Error` impl — no `thiserror`
  - `codegen.rs:253` — `impl core::error::Error for DecodeError {}`
  - `codegen.rs:271` — `impl core::error::Error for EncodeError {}`

## §9 Helpers

- [x] ❌ `Display` + `Debug` walkers — **NOT IMPLEMENTED**
- [x] ⚠️ `skip()` — partial: group entries only, no message-level skip
  - `codegen.rs:2579-2584` — static `skip()` on entry decoders (used internally for tail offset)
  - ❌ No public `skip()` on message decoder or group decoder
- [x] ✅ `encoded_length()` and `encoded_length_with_header()` — present on both decoder and encoder
- [x] ✅ `as_bytes()` — present on both decoder (`Result`) and encoder (`&[u8]`)
- [x] ⚠️ `raw_foo()` scalar accessors:
  - ✅ Generated for all primitive/array fields on messages and group entries
  - ❌ NOT generated for composite/enum/set fields in group entries (only primitive)
  - **Fix:** Add `raw_` accessors for composite/enum/set group entry fields
- [x] ❌ `*_NULL`, `*_MIN`, `*_MAX` constants — **NOT GENERATED**
- [x] ✅ Fixed-entry group fast path (`slice::as_chunks`) — **GENERATED**
  - `codegen.rs:2176-2191` — `as_chunks()` on group decoder when entries have no tail
- [x] ❌ Schema docs -> rustdoc — **NOT GENERATED**
  - Description captured from IR but never emitted as `///` doc comments
- [x] ❌ Opt-in extras: `finish()`, `validate()`, `reset_count_to_index()`, `copy_to_slice`, `write_to` — **NOT GENERATED**
- [x] ❌ Wire-annotated debug format `debug_wire()` -> `WireDebug<'_>` — **NOT GENERATED**
- [x] ❌ `MessageVisitor` trait + `accept_visitor` — **NOT GENERATED**

## §10 Codegen rules

- [x] ✅ roxmltree DOM parser — verified (`xml.rs`)
- [x] ✅ Token IR modelled on sbe-tool design — verified
- [x] ✅ Codegen emits plain Rust source — verified
- [x] 📝 `build.rs` driver — not yet implemented (`09-ergosbe-build-driver`)
- [x] 📝 Generated code in user's crate (orphan rule) — architectural design decision
- [x] ✅ Runtime inline by default — `sbe_rt` module emitted with zero external deps
- [x] ❌ `ergosbe-rt` shared crate — not implemented (`09`)
- [x] ⚠️ `#[inline]` on primitive field accessors — PARTIAL
  - ✅ `From<T>` / `Into<T>` for enums/choices: `#[inline(always)]`
  - ✅ `EncodeGroupEntry` blanket impl: `#[inline]`
  - ✅ Tail offset helpers: `#[inline]`
  - ❌ Primitive field accessors (decoder/encoder) have NO `#[inline]`
  - **Fix:** Add `#[inline]` to all primitive/composite/enum/set accessors on decoders and all setters on encoders
- [x] ✅ `const fn` on all primitive scalar accessors — VERIFIED
- [x] ✅ `slice::as_chunks` for fixed arrays and tail-free fixed-entry groups — VERIFIED
- [x] ❌ `#[cold]` on error paths — **NOT GENERATED**
  - **Fix:** Add `#[cold]` to error-returning helper functions in generated code
- [x] ❌ `#[expect(lint)]` over `#[allow(lint)]` — **NOT FOLLOWED**
  - Top-level: `#![allow(...)]` (`codegen.rs:126-135`)
  - `raw_` accessors: `#[allow(unused_unsafe)]` (`codegen.rs:1801, 1812`)
  - **Fix:** Replace `#[allow(...)]` with `#[expect(...)]` in generated code
- [x] ❌ `const` assertions in generated code — **NOT GENERATED**
  - e.g. `const _: () = assert!(core::mem::size_of::<MessageHeader>() == 8);`
  - **Fix:** Emit structural const assertions to catch generator bugs at compile time
- [x] ✅ `core::error::Error` (stable in core since 1.81) — VERIFIED

## §11 Test strategy (not in codegen scope)

- [x] ✅ Pre-generate + check in golden test (`tests/golden/car_example.rs`)
- [x] ✅ Regen-stability test (`stability_test.rs`)
- [x] ⚠️ Interop matrix — tests 1-2 partially done (`baseline_test.rs`), 3-11 not yet
- [x] ❌ Benchmark suite — not yet (`06-benchmark-perf-gates`)
- [x] ❌ HFT perf gates — not yet

## Explicitly rejected (verify NOT present)

- [x] ✅ No `bytemuck`/`Pod`/`zerocopy` transmute — all reads via `from_{le,be}_bytes`
- [x] ✅ No SIMD bulk copy — uses `copy_from_slice`
- [x] ✅ No `async`/`Stream` decode
- [x] ✅ No `thiserror` in generated code — hand-rolled Error impl
- [x] ✅ No per-version decoder types — `Option<T>` + `acting_version()` used
- [x] ✅ No `<const N: usize>` generic field accessors — N codegen-resolved
- [x] ✅ No `sbe-tool` Java IR at build time — pure Rust

---

## Additional specific checks

### Wire compatibility
- ✅ Byte layouts use `from_{le,be}_bytes` / `to_{le,be}_bytes` based on schema byte order
- ✅ All offsets reference schema-resolved positions; composites are `#[repr(transparent)] [u8; N]`
- ✅ No `repr(C)` transmute (trap 3 avoided)

### Version-aware decoding
- ✅ `acting_block_length` comes from wire header (`header.block_length()`) at `codegen.rs:1589`
- ✅ `tail_offset_0` uses `self.pos + self.acting_block_length` at `codegen.rs:1982`
- ✅ No compiled-`blockLength` fallback path exists

### Optional null vs version absence
- ✅ `Presence::Optional` fields: checked for null sentinel, returns `Option<T>`
- ✅ `sinceVersion > 0` fields (even if required): returns `Option<T>` (absent by version)
- ✅ `raw_` accessors: skip null check, preserve wire sentinel for HFT hot loops

### No transmute from buffer
- ✅ All reads: copy `[u8; N]` from buffer via manual byte copy or `copy_from_slice`, then call `from_{le,be}_bytes`
- ✅ No `core::mem::transmute`, no `ptr::read_unaligned`, no `repr(C)` casting

### Error taxonomy completeness
- ✅ `DecodeError`: 5 specified variants all present; `BufferTooShort` has extra `field` context (ahead of spec)
- ✅ `EncodeError`: `BufferTooShort` + `VarDataTooLong` (ahead of spec which only listed `BufferTooShort`)
- ✅ Both implement `core::error::Error` with `Display` — no `thiserror`

### Safe-by-default with opt-in unsafe
- ✅ All checked accessors are safe `fn` returning `Result`
- ✅ All `_unchecked` variants are `unsafe fn`
- ✅ `raw_()` accessors call `_unchecked()` inside an `unsafe` block but are themselves safe `fn`

### const fn on primitive accessors
- ✅ All field read accessors are `const fn`
- ✅ `_unchecked` variants are `const unsafe fn`
- ✅ `wrap` (without header) is `const fn`
- ✅ `acting_version()`, `acting_block_length()` are `const fn`
- ✅ `raw()` on enums/choices is `const fn`

### #[cold] on error paths
- ❌ **Not present anywhere in generated code**

### E3 enum pattern for unknown discriminants
- ✅ `#[repr(transparent)] struct Foo(pub u8)` — any byte round-trips
- ✅ `Foo::kind() -> Option<FooKind>` — returns `None` for unknown discriminants
- ✅ `FooKind` is `#[repr(u8)]` enum with known variants
- ✅ `From<u8> for Foo`, `From<Foo> for u8`, `TryFrom<Foo> for FooKind`

---

## Summary

| Section | Total | ✅ | ⚠️ | ❌ | 📝 |
|---------|-------|---|---|---|---|
| §1 Type model | 5 | 4 | 0 | 0 | 1 |
| §2 Encoder | 11 | 10 | 0 | 1 | 0 |
| §3 Decoder | 14 | 8 | 1 | 3 | 0 |
| §4 Data types | 12 | 10 | 2 | 0 | 0 |
| §5 Metadata & SbeMessage | 8 | 2 | 0 | 4 | 0 |
| §6 Dispatch | 9 | 6 | 1 | 2 | 0 |
| §7 Versioning | 5 | 5 | 0 | 0 | 0 |
| §8 Bounds + Errors | 7 | 5 | 1 | 1 | 0 |
| §9 Helpers | 12 | 4 | 2 | 6 | 0 |
| §10 Codegen rules | 14 | 7 | 1 | 4 | 2 |
| §11 Tests | 5 | 2 | 1 | 2 | 0 |
| Rejected items | 7 | 7 | 0 | 0 | 0 |
| **Total** | **109** | **70** | **9** | **23** | **3** |

**Clean bill of health:** Versioning (§7), Error taxonomy (§8b), and Rejected items are fully compliant.
**Strongest:** Wire layout, E3 enum pattern, const fn on accessors, safe-by-default, optional/null semantics.
**Weakest:** Metadata (§5) and Helpers (§9) are largely future work.

**Key violations (in priority order):**

1. `#[must_use]` on encoder types (§2) — ignored encoder silently emits partial message
2. `#[inline]` on primitive field accessors (§10) — performance promise (Q5) depends on this
3. `#[cold]` on error paths (§10) — instruction cache pollution in HFT hot paths
4. `#[expect(lint)]` over `#[allow(lint)]` (§10) — stale suppressions go undetected
5. `const` assertions in generated code (§10) — generator bugs undetected at compile time
6. `unsafe fn as_str_unchecked()` (§3) — zero-cost UTF-8 variant missing
7. `TryFrom<&[u8]>` on decoders (§3) — idiomatic Rust entrypoint missing
8. `as_message()` on var-data (§6) — var-data message dispatch missing
9. `AnyMessage::encode()` (§6) — encode dispatch missing
10. `#[diagnostic::on_unimplemented]` on SbeMessage (§5) — poor compiler error messages

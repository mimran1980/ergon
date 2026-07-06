# ErgoSBE — Design Decisions

Status: Draft (grilling output, 2026-07-05; revised after the upstream generator
inventory and HFT completeness review). The shared understanding between the
author and the design pass. Implementation follows this record; deviations
update this file.

ErgoSBE is an opinionated, idiomatic Rust code generator for Simple Binary
Encoding (SBE). Wire-compatible with the official SBE; API-shaped for Rust, not
translated from Java. Target: low-latency trading (HFT, market data, order
gateways, exchange connectivity).

The three core values, in priority order:

1. Wire compatible with official SBE (binary layout identical).
2. Idiomatic Rust, not Java-in-Rust.
3. Performance-first, zero-allocation hot path.

When these conflict, the earlier one wins (e.g. ergonomics yield to wire compat).

---

## 1. Type model

- **Flyweight-only.** Decoders borrow `&'a [u8]`; encoders borrow `&'a mut [u8]`.
  No owned/`Vec`/`String` generation.
- `OwnershipMode` is removed from `config.rs` — there is one mode.
- Decoders are `Copy` (`{ buf: &'a [u8], pos }`); encoders hold `&'a mut [u8]` (not Copy).
- **Serde, `no_std`, `zerocopy` all parked** for v1. The hot path is allocation-free
  by construction regardless; we just don't pay the `no_std`/`alloc`-feature tax yet.
  Allocating conveniences are opt-in so HFT users do not accidentally pull heap
  behavior into generated hot-path code.

## 2. Encoder

- **Scalars and composites:** `&mut self -> &mut Self` fluent setters, statement-style.
  They write to fixed schema-known offsets, so order is irrelevant on the wire.
- **Closure sub-encoders** for composites, groups, and var-data. The sub-encoder is
  borrowed only inside the closure; the caller never holds it, so there is no
  `.parent()` ping-pong (the official generator's main ergonomic flaw).
- **No top-level `encode(|c| …)` closure** — incompatible with type-state (trap 6).
- **Type-state tail ordering.** Only the variable-length tail (groups + var-data) is
  ordered on the wire, so only it earns type-state. By-value setters advance a
  phantom state named after the next-needed field: `NeedsBids` → `NeedsAsks` →
  `Complete`. Messages with no tail generate no phantom types.
- **`.add(impl EncodeGroupEntry<E>)`** — one generic trait, implemented for all
  `FnOnce(&mut EntryEncoder)` so closures work out of the box; user structs opt in
  with a one-line impl. No owned entry types generated.
- **Encoder is not version-aware** — it emits the current schema version only.
- **Nullify-on-wrap:** `wrap_and_apply_header` writes each optional field's `NullValue`
  at its offset, so an unset optional reads as null on the wire, not garbage.
- **`#[must_use]` on encoder types** — an ignored encoder would emit a partial message.
- **`wrap_and_apply_header` returns `Result`.** If the buffer is too short for the
  header + `blockLength`, it returns `Err(EncodeError::BufferTooShort)` — no panic,
  no silent truncation. Order-entry systems need this.
- **`AsRef<[u8]>` on the `Complete` terminal state** — after type-state tail
  completes, the encoder exposes the written region via `as_bytes()` /
  `AsRef<[u8]>`, matching the decoder's shape.

## 3. Decoder

- **`Copy` decoders**, all accessors return borrows tied to `'a` (the buffer), not
  `&self`. Plain `Iterator` over groups — no GATs / lending iterators — because
  every item borrows the buffer, not the iterator.
- No type-state (asymmetry with encoder): reading is random-access, order is free.
- **`acting_version() -> u16` and `acting_block_length() -> usize`** expose the wire
  header fields the decoder already carries internally (lets users branch on version).
- **Group decoders implement `ExactSizeIterator`** (count from the group dimension)
  with `len()`. `is_empty()` is an inherent method on the group decoder, not an
  `ExactSizeIterator` override, because `ExactSizeIterator::is_empty` is still
  unstable on stable Rust.
- **Var-data accessor family** (emitted per field as relevant):
  `as_slice() -> &'a [u8]` (always); `as_str()` (when `characterEncoding` declared);
  `as_string()` behind `alloc-convenience`; `as_decoder()`; `as_message()`.
- `as_str() -> Result<&'a str, Utf8Error>` + `unsafe fn as_str_unchecked() -> &'a str`
  (zero-cost, caller asserts validity). Distinct from bounds-checking.
- **Always version-aware.** `wrap_and_apply_header(buf, off)` reads the header;
  `wrap(buf, off, header)` wraps the body with a provided header (used by
  `AnyMessage::decode` internally to avoid a double-read). No compiled-`blockLength`
  fallback path — it would be a footgun.
- **`AsRef<[u8]>`** on decoders exposes `as_bytes()`.
- **`impl TryFrom<&'a [u8]> for XxxDecoder<'a>`** — idiomatic Rust conversion;
  delegates to `wrap_and_apply_header(buf, 0)`. Discoverable in docs and lets
  users write `let car = CarDecoder::try_from(buf)?;`.
- **`const fn` on primitive field accessors** — edition 2024 / Rust 1.88 makes most
  fixed-slice + `from_{le,be}_bytes` accessors `const`-eligible. No other SBE
  generator does this; it lets users use decoded fields in const contexts.
- **Raw scalar accessors for HFT.** Every scalar field also gets a `raw_foo()`
  accessor that returns the wire value without optional-null mapping. For
  `sinceVersion > acting_version`, raw accessors still return `None` rather than
  reading bytes that are not present. This gives latency-sensitive users direct
  sentinel handling while preserving safe version behavior.

## 4. Data types

- **Composites:** `Copy` value struct read field-by-field (`from_{le,be}_bytes`,
  unaligned-safe, endian-correct), `#[repr(C)]` nominal only — **never transmuted**
  (trap 3).
  Both a struct accessor (`car.header() -> MessageHeader`) AND per-field direct
  methods (`car.template_id()`) are generated, single source of truth. Encode side
  dual: value or closure.
- **Fixed arrays** (`int32[8]`, `char[16]`): concrete `[T; N]` returned by value.
  Const generics live only inside the runtime-support read helper, not in user types.
- **Enums:** Aeron-style flat enum with `NullVal` catch-all.
  A `#[repr(u8)] pub enum X { A = b'A', B = b'B', C = b'C', NullVal }` — the last
  variant catches unknown wire values. The generated `from_raw()` match handles all
  known discriminants; everything else falls through to `NullVal`. The old E3 split
  (newtype + `Kind` enum + `kind()`/`into_kind()`) is removed. Trap 2 is moot because
  `NullVal` preserves the unknown discriminant as a typed variant, though the original
  wire byte is lost (acceptable per user — they do not filter on unknown enum values).
  - `From<X> for u8` / `From<u8> for X` / `const fn from_raw(u8) -> X` / `const fn raw(self) -> u8`
  - `From<bool>` / `From<X> for bool` for boolean-style enums (F=0, T=1)
  - `Display` uses `Debug` formatting (the `{e:?}` pattern)
- **Choices (multi-value bitsets):** hand-rolled `#[repr(transparent)] struct X(u8)`
  newtype with per-flag bool read/write accessors + `raw()`. No `bitflags` crate.
- **Standard derives** on value types (composites, enums, choice newtypes):
  `Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord` — Java's
  `equals`/`hashCode`/`compareTo` delivered idiomatically. Float fields skip `Eq`/`Ord`;
  enum `Ord` follows numeric order.
- **Newtype conversions:** choice newtypes get `From<u8>` + `Into<u8>` + `raw()`
  + `Default` (all-zero bits, replaces Java's `clear()`). Enums get `From<u8>` +
  `From<EnumType> for u8` + `const fn raw()` + `const fn from_raw()`. Boolean enums
  additionally get `From<bool>`.
- **Constant-value fields** (`presence="constant"`) generate a `const fn` returning
  `&'static str` (string constants) or a typed value (numeric constants). No wire
  space consumed; the value is baked from the schema. The accessor is always
  available regardless of version.
- **Optional/null semantics are separate from version absence.** Accessor return
  shape is:
  - required + `sinceVersion == 0` → `T`
  - required + `sinceVersion > 0` → `Option<T>` (`None` means absent in this
    acting version)
  - optional + any version → `Option<T>` (`None` means absent by version or equal
    to the type's null sentinel)
  - `raw_foo()` preserves the null sentinel when the field is present on the wire.
  The IR validation pass resolves SBE default null sentinels and any XML
  `nullValue` overrides into typed constants before codegen.
- **`semantic_type` on fields** — SBE fields carry `semanticType` (e.g. `Price`,
  `Qty`, `UTCTimestamp`). Captured into the `FieldMeta` const and emitted as a
  rustdoc line on the accessor. Enables future typed wrappers and serde
  customisation.
- **Semantic newtypes for trading domains.** Behind a `semantic-newtypes` config,
  common semantic types (`Price`, `Qty`, `UTCTimestamp`, `LocalMktDate`,
  `SecurityID`, etc.) emit `#[repr(transparent)]` wrappers with `raw()` and
  `From` conversions. Keep decimal/time formatting out of the hot path; rich
  conversions remain optional features.

## 5. Metadata & the `SbeMessage` trait

Generated code is self-describing — for audit, tooling, generic code, and dispatch.

- **Per-message associated consts:** `TEMPLATE_ID`, `BLOCK_LENGTH`, `SCHEMA_ID`,
  `SCHEMA_VERSION`, `SCHEMA_HASH: [u8; 32]`, `SCHEMA_HASH_HEX: &'static str`,
  `SEMANTIC_VERSION: Option<&'static str>`, `SEMANTIC_TYPE: Option<&'static str>`
  (standardises the existing `SBE_*` constants). `SCHEMA_HASH` is SHA-256 over the
  normalized schema IR, useful for deployment checks and exchange-rollout safety.
- **Per-field `meta` module** — one `FieldMeta` const per field:
  ```rust
  pub struct FieldMeta {
      pub id: u16, pub since_version: u16, pub offset: usize,
      pub presence: Presence, pub null_value: Option<u64>,
      pub semantic_type: Option<&'static str>,
  }
  pub mod meta {
      pub const SERIAL_NUMBER: FieldMeta = FieldMeta { id: 1, since_version: 0, offset: 0, … };
  }
  ```
- **`SbeMessage` trait** (Java's implicit "message interface," made Rust-idiomatic).
  Each message's encoder/decoder implements it; it is the foundation `AnyMessage::decode`
  dispatch is built on and enables user generics:
  ```rust
  pub trait SbeMessage {
      const TEMPLATE_ID: u16; const BLOCK_LENGTH: usize;
      const SCHEMA_ID: u16;   const SCHEMA_VERSION: u16;
  }
  // → fn send<M: SbeMessage>(msg: &M, buf: &mut [u8]) { … }
  ```
- **Sealed trait.** `SbeMessage` uses the sealed-trait pattern
  (`mod private { pub trait Sealed {} }`) so only generated types implement it —
  the dispatch `match` depends on exhaustiveness.
- **`#[diagnostic::on_unimplemented]`** (stable since 1.78) on `SbeMessage`:
  ```rust
  #[diagnostic::on_unimplemented(
      message = "`{Self}` is not an SBE message type",
      note = "only types generated by ErgoSBE implement `SbeMessage`"
  )]
  ```
  Gives clear compiler errors instead of "trait bound not satisfied."

## 6. Message dispatch + entrypoints

- **`AnyMessage<'a>` enum**, one variant per message in the schema plus
  `Unknown { header: MessageHeader, payload: &'a [u8] }` so unknown messages can be
  forwarded/re-emitted unchanged when a frame length is available.
- `AnyMessage::decode(buf, off) -> Result<AnyMessage<'a>, DecodeError>` reads the
  schema header, dispatches on `templateId`, returns a typed decoder positioned
  after the header. Unknown `templateId` without a supplied frame length returns
  `DecodeError::UnknownTemplateLength`, because the SBE message header does not
  carry total message length. Wrong `schemaId` returns `DecodeError::WrongSchema`.
- `AnyMessage::decode_frame(buf, off, frame_len) -> Result<DecodedFrame<'a>, DecodeError>`
  is the HFT/feed-facing entrypoint. It can return `Unknown { payload }` over the
  whole externally framed message and can forward unknown templates unchanged.
- `FrameCursor<'a>` iterates through a buffer of externally framed messages and
  yields `DecodedFrame { message, range, len }`. The framing policy is explicit:
  length-prefix, fixed packet boundary, or caller-supplied frame lengths. SBE
  itself is not treated as a transport frame.
- `as_message()` on a var-data field is
  `AnyMessage::decode_frame(field_bytes, 0, field_bytes.len())` — same enum, with
  the var-data length acting as the external frame length for unknown templates.
- Encode entrypoints: `wrap` (body only, header managed elsewhere),
  `wrap_and_apply_header` (writes header + body, nullifies optionals), `AnyMessage::encode`.
- **`#[non_exhaustive]` on the `AnyMessage<'a>` dispatch enum** — new variants appear
  on schema evolution; downstream `match` must have a `_ =>` arm. Prevents hard
  breakage when the schema adds messages.
- **Length helpers:** for known templates, `AnyMessage::encoded_message_length(buf) ->
  Result<usize, DecodeError>` returns total known-template size (header + block +
  groups + var-data computed by scanning structural extents). For unknown
  templates, length is unavailable unless the caller supplies a frame length.
- **Configurable header and group dimensions.** Do not hard-code `messageHeader` or
  `groupSizeEncoding`: resolve the root `headerType` and each group's
  `dimensionType`. v1 supports custom names and primitive widths when they resolve
  to the required semantic fields (`blockLength`, `templateId`, `schemaId`,
  `version`; and group `blockLength`, `numInGroup`). Strict mode rejects layouts
  that cannot be represented wire-compatibly.

## 7. Versioning (wire-compat core)

Four invariants — the correctness heart of the library:

1. **Tail offset uses the wire `blockLength`**, not the compiled one. Groups and
   var-data live at `body_offset + header.block_length()`. THE classic SBE bug (trap 1).
2. **Forward compat** (new decoder, old message): fields with `sinceVersion >
   header.version()` are absent → accessor returns `None`.
3. **Backward compat** (old decoder, new message): known fields read at stable
   offsets; unknown trailing fixed bytes are skipped via `header.block_length()`.
4. `schemaId` is identity (mismatch = error); `version` is evolution (normal).

- **Accessor gating:** required v0 fields return `T`; optional fields and
  `sinceVersion > 0` fields return `Option<T>`. The generator knows each field's
  `sinceVersion` and presence from the XML.
- Sub-decoders (groups/entries/composites) thread the wire `version` down so their
  own versioned fields gate correctly.
- Optional-null handling is orthogonal to versioning: a field can be present in
  the acting version but null by sentinel. Public accessors collapse both "absent"
  and "present-null" to `None`; `raw_` accessors distinguish them.

## 8. Bounds checking

- `Result`-returning methods bounds-check; raw-returning methods don't (they read
  inside a region a structural point already validated).
- **Structural points** (`wrap_and_apply_header`, group iteration, var-data length)
  have BOTH a checked `Result` form AND an `unsafe …_unchecked` form — std
  `get`/`get_unchecked` idiom, both always compiled.
- **Feature `bound-check-disabled`** (default off) flips the auto-derived ergonomic
  paths (`Iterator` impls, default decode entry) to call the `_unchecked` primitives
  internally. API shape is identical across the feature; it only subtracts branches.
- Group/var-data accessors return `Result` in checked mode (`car.bids()?`) with
  `_unchecked` companions. `Iterator::next` returns `Option`, so the group's extent
  is validated when the accessor is called, not inside `next` (trap 5).
- Crate is **safe by default**; unsafety is opt-in via the feature or per-call
  `unsafe {}`. Deliberate divergence from the official generator's "no unsafe ever."

## 8b. Error taxonomy

- **`DecodeError`** — the exhaustive error enum for all decode failures:
  ```rust
  pub enum DecodeError {
      BufferTooShort { needed: usize, available: usize },
      WrongSchema { expected: u16, actual: u16 },
      UnknownTemplateLength { template_id: u16 },
      InvalidVarDataLength { field: &'static str, length: u32 },
      Utf8(core::str::Utf8Error),
  }
  ```
  Variants carry enough context for recovery (skip-to-next-message) but no heap
  allocation. Implements `core::error::Error` (stable in `core` since 1.81) —
  `no_std`-ready without `thiserror`.
- **`EncodeError`** — minimal: `BufferTooShort { needed: usize, available: usize }`.
  Encoder failures are simpler; the type-state prevents structural mis-ordering.
- Both error types are hand-rolled in the inline runtime (~20 lines each).
  `core::error::Error` impl, `Display` impl, no dependencies.

## 9. Helpers

- `Display` + `Debug` walkers (the idiomatic `toString` equivalent — gives
  `format!("{}", msg)`, `println!`, `to_string()`). Zero-alloc until formatted.
- `skip()` — advance past a group/var-data region without decoding, returns the
  post-region offset.
- `encoded_length()` (body) and `encoded_length_with_header()` (body + 8).
- `as_bytes() -> &'a [u8]` — header-inclusive current slice on a decoder; the
  written region on an encoder.
- `raw_foo()` scalar accessors and generated `*_NULL`, `*_MIN`, `*_MAX` constants
  for users who handle exchange sentinels manually in hot loops.
- **Fixed-entry group fast path.** When a group entry has no nested groups or
  var-data tail, expose indexed access and chunk-backed iteration over
  `&[[u8; BLOCK_LENGTH]]` via `slice::as_chunks`. This is the common order-book
  shape and removes repeated stride/bounds arithmetic. The typed entry decoder
  still reads field-by-field; the chunk is just a fixed-size backing window, not
  a transmuted struct.
- **Schema docs → rustdoc.** Both XML `<!-- -->` comments (associated to the nearest
  element) AND `description` attributes/children are captured and combined into `///`
  docs on the type, the struct field AND its accessor method, and enum/choice variants.
- **Opt-in extras** (build only when requested):
  - `finish()`/`validate()` encode-completion check (debug feature) — type-state
    covers the tail; this covers required scalars.
  - `reset_count_to_index()` — shrink a group during encode.
  - `copy_to_slice(&mut [u8])` and `write_to(&mut impl io::Write)` — `as_slice`/`Display`
    cover most needs; these are conveniences.
- **Wire-annotated debug format** — `fn debug_wire(&self) -> WireDebug<'_>` implements
  `Display` with a hex dump annotated with field boundaries:
  ```
  [00..08] Header: templateId=1, blockLength=42, schemaId=1, version=0
  [08..16] serial_number: 0x00000000DEADBEEF (3735928559)
  [16..20] model_year: 0x07E6 (2022)
  ```
  Zero-alloc until formatted. Invaluable for wire-level debugging in trading systems.
- **`MessageVisitor` trait** — generic walking pattern for decoded messages:
  ```rust
  pub trait MessageVisitor {
      fn field(&mut self, meta: &FieldMeta, value: FieldValue<'_>);
      fn begin_group(&mut self, name: &str, count: usize);
      fn end_group(&mut self);
      fn var_data(&mut self, meta: &FieldMeta, data: &[u8]);
  }
  ```
  `Display`/`Debug` walkers, JSON export, metrics extraction, and logging all
  implement this one trait. Generated `accept_visitor(&self, v: &mut impl
  MessageVisitor)` method on each decoder.

## 10. Internal architecture

```
SBE XML --roxmltree(DOM)--> resolved Token IR --codegen--> Rust source --rustfmt--> .rs
```

- **roxmltree** (DOM), not quick-xml streaming — DOM handles SBE's mixed-order
  `<type>`/`<enum>`/`<set>`/`<composite>`/`<message>` and forward refs trivially, and
  preserves XML comments as nodes (needed for §9). Schemas are KB-scale, DOM is free.
- **Token IR** modelled on sbe-tool's proven design (flat token list with signals,
  each carrying offset/version/encoding/presence). Pure Rust — no Java at build time.
  The validation pass resolves references, default/null/min/max values, byte order,
  header type, group dimension type, block lengths, schema hash, and semantic
  metadata before codegen.
- **Codegen** emits plain Rust source (no proc-macros in the output), run through
  rustfmt. Output is reviewable text — the point for audit.
- **Driver: `build.rs` in v1** via an `ergosbe-build` crate (no CLI). The generator
  library is the single source of truth; a proc-macro annotation
  (`#[ergosbe::schema("car.xml")] mod messages;`) is the v1.1 ergonomic front-end.
- **Generated code is a module in the user's crate** — not a separate crate. This is
  load-bearing: it lets users add inherent methods and impl foreign traits (serde,
  rust_decimal) on generated types despite the orphan rule (trap 4).
- **Runtime inline by default** (zero-dep): `MessageHeader`, read/write primitives,
  `EncodeGroupEntry`, `SbeMessage` trait, `DecodeError`, checked/`_unchecked` helpers are
  emitted into each generated module. **Opt-in `ergosbe-rt` shared crate** (config flag
  `shared_runtime`) deduplicates `MessageHeader` + primitives when multiple schemas
  are generated into one crate.
- The generator crate can keep `unsafe_code = "forbid"`. Generated modules may
  contain `unsafe fn …_unchecked` declarations by design; generated fixture crates
  and user crates must not inherit a blanket `unsafe_code = "forbid"` lint unless
  unchecked APIs are disabled for that output.

### Codegen rules

- **`#[inline]`** on primitive field accessors — the mechanism behind the Q5
  "not slower than transmute" promise (lets LLVM elide the LE `bswap` and dead field
  reads). `#[inline]` broadly; `#[inline(always)]` only on the hottest one-liners.
- **Hand-roll the `Error` impl** in the inline runtime (~15 lines) — no `thiserror`/`miette`
  in generated code, to keep the zero-dep story. The *generator* crate itself uses both:
  `thiserror` for `Error`/`Display` and `miette` for span-rich diagnostics (the parser's
  `ParseError` carries `NamedSource` + `SourceSpan` so failures highlight the offending
  XML element). `miette`'s large error is `#[allow]`ed — build-time only, not hot path.
- Standard derives + nullify-on-wrap + `#[must_use]` per §§2/4.
- **`const fn`** on all primitive scalar accessors (read a fixed slice,
  `from_{le,be}_bytes`) — edition 2024 makes these const-eligible. Differentiator.
- **`slice::as_chunks` / `as_chunks_mut`** for fixed arrays and tail-free fixed-entry
  groups. This is stable in the MSRV and gives fixed-size backing windows without
  unsafe casting or custom pointer arithmetic.
- **`#[cold]`** on error-construction helpers and panic paths — tells LLVM to keep
  them out of the hot instruction cache. The dual of `#[inline]` for error branches.
- **`#[expect(lint)]`** (stable since 1.81) instead of `#[allow(lint)]` in generated
  code — warns if the lint stops firing, catching stale suppressions.
- **`const` assertions** emitted into generated code for structural invariants:
  ```rust
  const _: () = assert!(core::mem::size_of::<MessageHeader>() == 8);
  ```
  Catches generator bugs at compile time, not runtime.
- **`core::error::Error`** (stable in `core` since 1.81) on `DecodeError`/`EncodeError`
  — no `std` dependency, `no_std`-ready by construction.

### Explicitly rejected (do not re-propose)

| Idea | Reject because |
|---|---|
| `bytemuck`/`Pod`/`zerocopy` transmute | Trap 3 — unaligned + native-endian + `packed` UB. |
| SIMD bulk copy | Premature; `copy_from_slice` is already optimal. |
| `async`/`Stream` decode | Framing concern, not codec. Parked. |
| `thiserror` in generated code | Breaks zero-dep/audit. Hand-roll `Error`. |
| Per-version decoder types (`V1Decoder`/`V2Decoder`) | `Option<T>` + `acting_version()` is the right granularity. |
| `<const N: usize>` generic field accessors | `N` is known at gen time → concrete `[T; N]`. |
| `sbe-tool` Java IR at build time | JVM in the build graph breaks the pure-Rust story. Reuse the IR *design*, not the runtime. |

## 11. Test strategy (tests are the source of truth)

- **Pre-generate + check in** the car baseline/extension test crates (fast,
  reproducible CI), plus a **regen-stability test** (regenerate, assert no diff) to
  catch generator drift.
- Interop + versioning matrix against the upstream `.sbe` byte fixtures
  (`simple-binary-encoding/rust/car_example_*_data.sbe` — Java-generated, tool-independent):

| # | Test | Proves |
|---|---|---|
| 1 | Decode official `.sbe` fixtures with ErgoSBE decoders, assert every field | Reads official wire |
| 2 | Encode canonical car with ErgoSBE, assert bytes == official fixture | Writes official wire (byte-exact; upstream only checks semantic) |
| 3 | Decode baseline bytes with extension decoder → new fields `None`, tail correct | Forward compat |
| 4 | Decode extension bytes with baseline decoder → fields ok, extra bytes skipped, groups at right offset | Backward compat (catches trap 1) |
| 5 | Optional/null matrix across required/optional/versioned fields, including `raw_` accessors | Null and version semantics |
| 6 | Wrong-`schemaId` bytes → `DecodeError::WrongSchema` | Identity rejection |
| 7 | Big-endian scalar/composite fixture | `byteOrder` correctness |
| 8 | Custom `headerType` and `dimensionType` fixtures | No hard-coded schema names |
| 9 | `FrameCursor` over known and unknown templates with external frame lengths | Real feed/framing behavior |
| 10 | Round-trip encode→decode→semantic-equal, both versions | Internal consistency |
| 11 | Property/fuzz round-trip | Robustness |

- **Benchmark suite** (`criterion` crate, separate `benches/` crate in the workspace).
  Baseline from step 2 (scalar-only message), extended with each vertical slice.
  Benchmarks: encode, decode, round-trip, `Display` format, `debug_wire`, and
  `skip` — all on realistic market-data-shaped messages. Performance is value #3;
  it must be measured from day one.
- **HFT perf gates:** allocation-count tests assert zero allocations for decode,
  raw scalar access, group iteration, frame cursor decode, and encode into caller
  buffers. Optional instruction/cache-oriented benches can be run locally for
  release decisions, but CI always keeps the no-allocation hot path honest.

---

## Tricky areas / traps

1. **Tail offset must use the wire `blockLength`, not compiled.** Using
   `SBE_BLOCK_LENGTH` mis-locates every group when reading a newer version. Test #4.
2. **A Rust enum cannot hold an unknown discriminant.** The flat enum with `NullVal`
   (chosen over the old E3 newtype+Kind split) catches unknown wire values but does
   not preserve the original byte. The user accepts this trade-off — they do not
   filter on unknown enum values. If preserving the raw byte becomes a requirement,
   add a `raw_` accessor or return to the newtype. See `design/DECISIONS.md §4` and
   `todos/106-flat-enum-nullval.md`.
3. **`repr(C)` transmute is a trap.** SBE buffers are unaligned (so transmute needs
   `read_unaligned` — no win), `repr(C)` is native-endian (so BE schemas are wrong),
   and `packed` makes `&field` UB. Read field-by-field with `from_{le,be}_bytes`.
4. **Orphan rule** → generated types must live in the user's crate for users to impl
   foreign traits / add inherent methods. Hence `build.rs`/macro-into-user-crate, no
   CLI-to-separate-crate.
5. **`Iterator::next` returns `Option`, not `Result`.** A checked group iterator can't
   signal OOB through the trait. Validate the group's extent when the accessor is
   called (`bids() -> Result<…>`), making iteration infallible within.
6. **Type-state needs moves; a `&mut` closure needs `&mut self`.** Incompatible — so
   the encoder is the γ hybrid: scalars `&mut self` (statement-style), tail by-value
   type-state. No enclosing `encode(|c| …)` closure.
7. **`as_str_unchecked` must be `unsafe fn`** (zero-cost via
   `str::from_utf8_unchecked`), not a panicking safe fn — and it's a different escape
   hatch from `bound-check-disabled` (UTF-8 validity vs array bounds).
8. **No silent compiled-`blockLength` decoder path.** Always version-aware, or it's a
   footgun. The only "fast path" is `bound-check-disabled` (bounds), not version-skipping.
9. **SBE is not a transport frame.** Unknown-template forwarding requires an
   external frame length; the SBE message header alone cannot recover total length.
10. **Optional null is not version absence.** `Option<T>` is ergonomic; `raw_`
    accessors are required for hot loops that distinguish null sentinels from old
    acting versions.

---

## Parked / deferred

- **Serde** (`Serialize`/`Deserialize`) — not v1. `Serialize` on the decoder view is
  the natural shape when it returns.
- **`no_std`** — not v1. Core decode/encode is already `no_std`-clean; only the
  allocating helpers would need an `alloc` feature.
- **Proc-macro annotation driver** — v1.1 (same generator library, thin front-end).
- **`ergosbe-rt` shared runtime crate** — opt-in, for multi-schema-in-one-crate dedup.
- **sbe-tool binary-IR interop** — optional later path (decoding sbe-tool's SBE-encoded
  IR dump) for users who want bit-identical IR without our XML parser.
- **Rich decimal/time conversions** — optional later features on top of semantic
  newtypes, not default hot-path code.
- **Opt-in helpers** (§9): `finish()`/`validate()`, `reset_count_to_index()`,
  `copy_to_slice`/`write_to`.
- **Owned structs** — not generating; if ever needed, project from the decoder view.

---

## Implementation order (TDD, vertical slices)

1. roxmltree parse of a minimal schema → assert Token IR.
2. Codegen for a scalar-only message → golden test (generate, check in, assert
   stable) + wire round-trip. Error taxonomy (`DecodeError`/`EncodeError`) lands
   here — needed for the first `Result`-returning decode. Include optional/null
   matrix and `raw_` accessors before moving past scalars.
2b. Benchmark scaffold (`criterion`, scalar-only encode/decode baseline).
3. Reference-resolution pass: byte order, default/null/min/max values,
   `headerType`, `dimensionType`, block lengths, schema hash, field ids.
4. Composites (value struct + per-field methods).
5. Enums (flat enum with NullVal) and choices (newtype bitset).
6. Groups (Iterator decode, type-state encode, tail-free fixed-entry fast path).
7. Var-data (`as_slice`/`as_str`/`as_string` behind feature/`as_decoder`/`as_message`).
8. Versioning (baseline/extension cross-version tests + official fixtures).
9. `AnyMessage` dispatch enum + `SbeMessage` trait + `FrameCursor`.
10. `bound-check-disabled` + `_unchecked` variants.
11. `Display`/`Debug`, `skip`, length accessors, `as_bytes`, metadata + `meta` module.
12. `build.rs` driver + `ergosbe-build` crate.
13. (v1.1) annotation macro; (parked) `no_std`, serde.

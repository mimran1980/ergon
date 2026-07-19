# ErgoSBE — Design Decisions

Status: Canonical design authority (created 2026-07-05; canonical priority and
ordered-tail decision revised 2026-07-10; fallible stage-combinator and nested
application-envelope decision revised 2026-07-11). Implementation, goal files, guides,
and todos defer to this record. Historical measurements and decisions remain in
their dated documents, but a conflict is resolved in favour of this file.

ErgoSBE is an opinionated, idiomatic Rust code generator for Simple Binary
Encoding (SBE). Wire-compatible with the official SBE; API-shaped for Rust, not
translated from Java. Target: low-latency trading (HFT, market data, order
gateways, exchange connectivity).

The canonical priority order is:

1. **Official-SBE wire compatibility is non-negotiable.** Binary layout and
   version-aware decoder behaviour must match official SBE.
2. **ErgoSBE must be equal to or faster than Aeron SBE in every maintained,
   measured scenario.** A measured regression remains unfinished.
3. **The Rust API should be easier or safer than Aeron** whenever that is
   zero-cost or the work stays outside the hot path.
4. **No safety check, abstraction, branch, or ergonomic wrapper may slow a
   benchmarked hot path** unless it is an explicit opt-in.
5. **Simplicity decides only when compatibility, performance, and safety are
   equal.**

When these priorities conflict, the earlier item wins. Performance claims are
always scoped to the named benchmark, hardware, toolchain, profile, and date.

**Generated code is never checked in.** ErgoSBE code is always generated
on-the-fly from schemas (via `build.rs` or equivalent). Benchmarks, tests, and
samples all generate code dynamically — never from a manually-maintained
checked-in file. The golden file is the stability target, generated via
`cargo test update_golden -- --ignored`.

---

## 1. Type model

- **Flyweight-only.** Decoders borrow `&'a [u8]`; encoders borrow `&'a mut [u8]`.
  No owned/`Vec`/`String` generation.
- `OwnershipMode` is removed from `config.rs` — there is one mode.
- Ordered decoder stages are consuming and are not `Copy`. A generated
  fixed-block body view may be `Copy` because it has no tail cursor and cannot
  be used to advance tail state. Encoders hold `&'a mut [u8]` and are not Copy.
- **Serde, `no_std`, `zerocopy` all parked** for v1. The hot path is allocation-free
  by construction regardless; we just don't pay the `no_std`/`alloc`-feature tax yet.
  Allocating conveniences are opt-in so HFT users do not accidentally pull heap
  behavior into generated hot-path code.

## 2. Encoder

- **Scalars and composites:** `&mut self -> &mut Self` fluent setters,
  statement-style. They write to fixed schema-known offsets, so order is
  irrelevant on the wire. A zero-cost body view may keep these accessors
  available while tail stages advance.
- **Concrete generated tail stages.** Generate a distinct public struct for
  each legal position in the ordered groups/var-data tail. Do not use public
  state generics, `PhantomData`, const-state indices, or APIs that require
  turbofish when concrete generated names suffice.
- **Consuming transitions.** Starting or skipping a tail component consumes the
  current stage. The moved stage cannot be reused. A group entry stage owns the
  right to return to its parent group, so the parent cannot advance while an
  entry, nested group, or variable-data stage is active.
- **No closure-only tail abstraction.** Entry closures may exist only as an
  optional convenience if measurement proves they are zero-cost. The generated
  concrete stages are the contract and must remain directly usable.
- **Manual and fallible-closure models are both first-class.** Callers may set
  every fixed field and drive every concrete stage directly. Additive helpers
  such as `try_fixed(...)`, `try_<group>(...)`, and
  `payload_with(exact_len, ...)` may return the same concrete next stage while
  allowing a caller-selected error to propagate with `?`. A closure helper
  must not replace, hide, or weaken the manual stage interface.
- **Caller errors remain monomorphised.** A fallible helper that can itself
  encounter an encode failure returns the caller's `E` with
  `E: From<EncodeError>`; `try_fixed` needs only the closure's `E` after a
  successful wrap. Do not box errors, use trait objects, allocate, or format
  strings on the generated success path.
- **Zero allocation.** Generated stage transitions, group entry handling, nested
  tails, and variable data allocate no heap memory.
- **Encoder is not version-aware** — it emits the current schema version only.
- **Opt-in nullify-on-wrap:** `wrap_and_apply_header` does NOT nullify optional fields
  by default (matching Aeron's behaviour). A generated `apply_nulls()` method writes
  each optional field's schema-defined `NullValue` when called. Aeron does not nullify
  on wrap either — unset optional fields retain whatever was in the buffer, so
  calling `apply_nulls()` is the explicit opt-in for deterministic wire output.
- **`#[must_use]` on encoder types** — an ignored encoder would emit a partial message.
- **`wrap_and_apply_header` returns `Result`.** If the buffer is too short for the
  header + `blockLength`, it returns `Err(EncodeError::BufferTooShort)` — no panic,
  no silent truncation. Order-entry systems need this.
- **Completion-only bytes and length.** `encoded_length()`, complete-message
  `as_bytes()`, and `AsRef<[u8]>` exist only on the terminal complete stage.
  Incomplete stages must not expose an `as_bytes()` that looks like a complete
  message. If partial inspection is genuinely needed, use an explicit name such
  as `written_prefix()` or `partial_bytes()` and benchmark any hot-path effect.
- **Exact pre-encoding length.** A variable-length encoder exposes
  `compute_encoded_length(...)` for the body and
  `compute_encoded_length_with_message_header(...)` for the standard SBE
  header plus body. Inputs must describe every runtime group count, nested
  count, and variable-data length needed for an exact result. Callers must not
  replace the header-inclusive helper with a hand-written `+ 8`.
- **Bounded nested-message encoding.** A var-data encoder may expose
  `payload_with(exact_len, |buf| -> Result<(), E> { ... })`. It writes the
  declared prefix, lends exactly `exact_len` bytes (never the rest of the outer
  buffer), and advances only after the closure succeeds. Nested SBE payloads
  include their own standard message header. Maintained callers must retain the
  nested complete stage and prove its header-inclusive length equals the lent
  slice before returning `Ok(())`.
- **Explicit header-inclusive completion view.** A complete encoder produced by
  `wrap_and_apply_header` exposes `as_bytes_with_header()` for the exact SBE
  header-plus-body region. It remains completion-only. This explicit view is
  used when the caller pre-sizes an external frame or encodes directly into an
  Aeron claim.
- **L3Book sample proof.** The maintained L3Book example pre-sizes with
  `compute_encoded_length_with_message_header(...)`, writes through fluent
  fixed-field setters followed by consuming nested tail stages, and ends with
  `as_bytes_with_header()` (or the equivalent complete `as_bytes()` view where
  its header inclusion is explicit). The final slice length must equal the
  computed length.
- **Required-field proof without scalar-order state explosion.** Fixed-block
  fields remain order-free because their offsets are schema-known, but generated
  strict builders/proxies can prove all required fixed fields were set before
  the final `finish()`/`as_bytes()` capability is exposed. Use a compact generated
  proof/bitset or grouped proxy state, not one type-state transition per scalar.
  Optional fields are already nullified on wrap, so omission stays explicit and
  cheap.

For a message whose tail is `bids` followed by `asks`, the generated shape is:

```text
OrderBookEncoder
  -> BidsEncoder
  -> OrderBookAfterBids
  -> AsksEncoder
  -> OrderBookComplete
```

`asks()` exists only on `OrderBookAfterBids`. Starting `bids` consumes
`OrderBookEncoder`; finishing the group consumes `BidsEncoder` and returns
`OrderBookAfterBids`. An explicit zero-count/skip transition must still write
the correct SBE group dimension and move sequentially to
`OrderBookAfterBids`.

## 3. Decoder

- **Concrete generated tail stages are the only tail traversal contract.**
  Scalar, enum, set, composite, and fixed-array fields in the fixed block remain
  direct and version-aware. Groups and var-data are sequential on the wire, so
  each legal tail position is a distinct consuming decoder stage.
- **No raw or random-access tail cursor.** Do not expose a cursor, offset, or
  earlier-stage convenience accessor that permits an arbitrary later group or
  var-data field to be read out of order.
- **Body access stays zero-cost.** A fixed-block body view may be borrowed or
  copied from each stage. Reading fixed fields never advances the tail cursor.
- **Entry ownership enforces nesting.** Starting a group consumes its parent
  stage. Taking an entry consumes or exclusively borrows the group stage so the
  parent group cannot advance until the entry and any nested group/var-data tail
  have been completed or explicitly skipped.
- **`acting_version() -> u16` and `acting_block_length() -> usize`** expose the wire
  header fields the decoder already carries internally (lets users branch on version).
- **Group decoders implement `ExactSizeIterator`** (count from the group dimension)
  with `len()`. `is_empty()` is an inherent method on the group decoder, not an
  `ExactSizeIterator` override, because `ExactSizeIterator::is_empty` is still
  unstable on stable Rust.
- **Var-data accessor family** (emitted per field as relevant):
  base accessor `field() -> &'a [u8]`; `field_as_str()` when
  `characterEncoding` is declared; `as_decoder()`/`as_message()` for nested SBE
  payloads. Do not emit allocation helpers or UTF-8 unchecked helpers by
  default; users can call `.to_string()` or `core::str::from_utf8_unchecked`
  explicitly if profiling justifies it.
- **Always version-aware.** `wrap_and_apply_header(buf, off)` reads the header;
  `wrap(buf, off, header)` wraps the body with a provided header (used by
  `AnyMessage::decode` internally to avoid a double-read). No compiled-`blockLength`
  fallback path — it would be a footgun.
- **`AsRef<[u8]>`** on decoders exposes `as_bytes()`.
- **`impl TryFrom<&'a [u8]> for XxxDecoder<'a>`** — idiomatic Rust conversion;
  delegates to `wrap_and_apply_header(buf, 0)`. Discoverable in docs and lets
  users write `let car = CarDecoder::try_from(buf)?;`.
- **Verified-frame proof mode.** Structural verification should be able to return
  a proof token, not only `Result<()>`: e.g. `VerifiedFrame<'a, Car>` or
  `XxxDecoder<'a, Verified>`. Normal decoders stay `Checked`; the `Verified`
  fast path is constructible only by generated verification code and can avoid
  repeated structural scans/bounds checks where the proof covers the extents.
  This is the safe Rust version of "validate once, read many times" for feed
  loops.
- **Runtime accessors optimise for the hot path, not `const fn`.** Field
  accessors that read `&[u8]` use the fastest clear runtime path
  (`try_into`/slice copies or the verified/unchecked fast path). Do not keep
  byte-by-byte loops just to preserve `const fn`; real feed buffers are runtime
  data, and Aeron does not pay for const-evaluable accessors either.
- **No broad scalar `raw_foo()` aliases.** The primary accessor is the
  zero-allocation hot-path API. Keep `raw()` only where it has different
  semantics (for example enum/set/composite value wrappers that expose the
  underlying representation). For sentinel-sensitive workflows, rely on
  generated null/min/max constants and metadata rather than duplicating every
  field accessor.
- **Sequential finish, skip, and rewind.** `finish(self)` scans past unread
  entries and nested tails in wire order and returns the next concrete stage.
  `skip_remaining(self)` may be provided as an explicit spelling of that intent.
  `rewind(self)` consumes any current stage and returns a fresh initial decoder;
  no stale stage remains usable after the move.
- **Runtime counts are separate from compile-time order.** Types prove that
  `bids` precedes `asks`; ordinary runtime state validates and tracks the count
  encoded in each group dimension header.
- **Fallible decoder combinators are additive.** The manual consuming methods
  (`into_<group>`, `into_<data>`, `finish`, and `skip_remaining`) remain the
  canonical escape hatch. Additive `try_fixed`, `try_<data>`, and
  `try_<data>_as_message` helpers may run caller closures and return the same
  concrete next stage. Helpers that decode structure require
  `E: From<DecodeError>` and use higher-ranked callback lifetimes so borrowed
  slices, entries, and message views cannot escape the callback.

For the same order-book schema, the decoder shape is:

```text
OrderBookDecoder
  -> BidsDecoder
  -> OrderBookAfterBids
  -> AsksDecoder
  -> OrderBookComplete
```

`asks()` exists only on `OrderBookAfterBids`. Empty groups, fully-read groups,
`finish()`, and `skip_remaining()` all reach that stage through the same
wire-order transition. Nested groups and variable data use equivalent entry
stages.

## 4. Data types

- **Composites:** `Copy` value struct read field-by-field (`from_{le,be}_bytes`,
  unaligned-safe, endian-correct), `#[repr(C)]` nominal only — **never transmuted**
  (trap 3).
  Both a struct accessor (`car.header() -> MessageHeader`) AND per-field direct
  methods (`car.template_id()`) are generated, single source of truth. Encode side
  dual: value or closure.
- **Variable-exponent Decimal composite.** Market prices and quantities that
  cannot share one schema-wide scale use an ordinary SBE composite with
  `mantissa: int64` and `exponent: int8`, representing
  `mantissa * 10^exponent`. The exponent is present per value; do not bake scale
  8 into the normalized `L2Book`/`Trade` wire schema. Generated access remains a
  zero-allocation `Copy` composite. Fixed-scale persistence conversion is an
  explicit checked adapter outside the codec hot path.
- **Generic decimal-converter seam.** A caller opts a structurally valid wire
  composite into conversion with
  `GenerationConfig::enable_decimal_converters("Decimal")`. The generator
  validates signed `int64` mantissa plus signed `int8` exponent and emits a
  local `SbeDecimal` trait with fallible `try_from_sbe`/`try_into_sbe` methods.
  Any application type may implement that local trait, including
  `rust_decimal::Decimal`; generated code does not depend on `rust_decimal`.
- **Converted and raw decimal access coexist.** In converter mode, ordinary
  price/quantity methods are generic over `D: SbeDecimal`, are monomorphised,
  and return conversion errors without allocation. Infallible `*_wire()` raw
  accessors/setters remain available. Without converter mode, the ordinary
  methods continue to use the generated wire composite directly. This is a
  legitimate generic seam because the application adapter type is not known at
  generation time; it does not weaken the concrete tail-stage rule.
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
  - generated null/min/max constants and field metadata preserve the exact
    sentinel information for code that needs to distinguish present-null from
    version absence.
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
- **Type-level scale/unit markers for semantic newtypes.** When the schema gives
  enough information, prefer zero-sized type parameters or const generics over
  runtime metadata: `Price<const SCALE: i32, Ccy>`, `Qty<Unit>`,
  `Timestamp<TimeUnit>`. The raw representation stays the SBE primitive, but the
  compiler can reject mixing ticks, lots, shares, millis, nanos, USD, and JPY.
  Generated aliases keep the public API readable, e.g. `type BidPx = Price<4, Usd>`.

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
      type Decoder<'a>: TryFrom<&'a [u8], Error = DecodeError>;
      type Encoder<'a>;
      type Schema: SchemaIdentity;
      const TEMPLATE_ID: u16; const BLOCK_LENGTH: usize;
      const SCHEMA_ID: u16;   const SCHEMA_VERSION: u16;
  }
  // → fn send<M: SbeMessage>(msg: &M, buf: &mut [u8]) { … }
  ```
  Associated codec types let generic code name the generated decoder/encoder
  without falling back to dynamic `AnyMessage` dispatch. Keep the trait sealed so
  user code cannot forge message identity.
- **Sealed trait.** `SbeMessage` uses the sealed-trait pattern
  (`mod private { pub trait Sealed {} }`) so only generated types implement it —
  the dispatch `match` depends on exhaustiveness.
- **Schema identity as a type.** Each generated schema emits a sealed marker type
  that carries `SCHEMA_ID`, `SCHEMA_VERSION`, and `SCHEMA_HASH`. Frames,
  dispatchers, proxies, and adapters can be parameterised by that marker so two
  exchanges or two schema generations cannot be accidentally mixed in generic
  code just because their wire primitives look similar.
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
- `FrameCursor<'a, P>` iterates through a buffer of externally framed messages
  and yields `DecodedFrame<'a, Schema> { message, range, len }`. The framing
  policy is explicit and typed: length-prefix, fixed packet boundary, or
  caller-supplied frame lengths. SBE itself is not treated as a transport frame.
- **Typed frame policy.** Framing is part of the API contract, not a runtime enum
  users can accidentally mismatch. A cursor over `LengthPrefixed` frames should
  not expose constructors or unknown-forwarding behaviour that only make sense
  for `FixedPacket<N>` or caller-supplied frame lengths.
- **Scoped callback dispatch.** Generated adapters use higher-ranked callback
  lifetimes (`for<'a>`) so decoded views borrow exactly the input frame and
  cannot escape into long-lived handler state. This preserves the flyweight
  zero-copy API while making the safe path natural for feed handlers.
- `as_message()` on a var-data field is
  `AnyMessage::decode_frame(field_bytes, 0, field_bytes.len())` — same enum, with
  the var-data length acting as the external frame length for unknown templates.
- **Nested application-message envelope.** The advanced Bitget sample places
  `AppMessage`, `L2Book`, and `Trade` in one normalized application schema.
  `AppMessage` contains fixed `sentTs: uint64` Unix-epoch nanoseconds followed
  by UTF-8 `appName` var-data and terminal `payload` var-data. The payload is a
  complete same-schema `L2Book` or `Trade`, including its SBE header, and is
  dispatched through the generated `AnyMessage` enum. Recursive envelopes,
  unknown/wrong-schema payloads, and infrastructure templates are rejected by
  the sample. `DynamicSchema` and `DynamicRow` are platform messages and remain
  unwrapped on their separate IPC stream.
- Encode entrypoints: `wrap` (body only, header managed elsewhere),
  `wrap_and_apply_header` (writes header + body without nullifying optionals),
  explicit `apply_nulls()`, and `AnyMessage::encode`.
- **`#[non_exhaustive]` on the `AnyMessage<'a>` dispatch enum** — new variants appear
  on schema evolution; downstream `match` must have a `_ =>` arm. Prevents hard
  breakage when the schema adds messages.
- **Length helpers:** for known templates, `AnyMessage::encoded_message_length(buf) ->
  Result<usize, DecodeError>` returns total known-template size (header + block +
  groups + var-data computed by scanning structural extents). For unknown
  templates, length is unavailable unless the caller supplies a frame length.
- **Ordered decode helpers:** `AnyMessage` and generated adapters return the
  initial concrete decoder stage. Adapters may drive those stages internally,
  but cannot bypass the schema-ordered transitions.
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
  the acting version but null by sentinel. Public accessors collapse both
  "absent" and "present-null" to `None`; generated null/min/max constants,
  metadata, and value-type `raw()` methods are the supported escape hatches.

## 8. Bounds checking and trust boundaries

- Checked constructors and verification APIs validate structural extents before
  returning borrowed views. Fixed-field accessors are infallible where the
  schema and acting version allow them to be present.
- **Feature `bound-check-disabled`** (default off) is the canonical fast-path
  switch. It keeps the public API identical and routes generated internals
  through unchecked reads/writes where the surrounding structural validation or
  caller contract makes that acceptable.
- Describe omitted validation only as a **trusted-input fast path**. Do not
  present "validation stripping to match Aeron" as a general design rule.
  Checked construction or validation may remain outside the benchmarked hot
  loop, and the trust preconditions must be explicit and tested.
- Avoid per-field `_unchecked` methods in the default public surface. They bloat
  generated code and duplicate the feature flag. Add an explicit unsafe method
  only when it exposes a genuinely different operation, such as a var-data
  encoder length check bypass that has been intentionally retained.
- Group/var-data accessors return `Result` in checked mode (`car.bids()?`).
  `Iterator::next` returns `Option`, so the group's extent is validated when the
  accessor is called, not inside `next` (trap 5).
- Crate is **safe by default**; unsafety is opt-in by feature/config or by a
  small number of explicitly justified unsafe APIs. Deliberate divergence from
  the official generator's "no unsafe ever."

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
- Fallible closure conveniences do not add a third generated wrapper error.
  They are generic over the caller's `E`; codec failures convert through
  `From`, while custom closure failures propagate unchanged.

## 9. Helpers

- `Display` + `Debug` walkers (the idiomatic `toString` equivalent — gives
  `format!("{}", msg)`, `println!`, `to_string()`). Zero-alloc until formatted.
- Sequential `finish()` / `skip_remaining()` transitions advance past the
  current group or var-data component without exposing an arbitrary raw offset.
- Decoder length helpers report body and header-inclusive extents after
  sequential structural scanning. Encoder `encoded_length()` is terminal-stage
  only.
- `as_bytes() -> &'a [u8]` may expose the header-inclusive message slice on a
  decoder; on an encoder it exists only on the complete stage.
- `raw_foo()` scalar accessors and generated `*_NULL`, `*_MIN`, `*_MAX` constants
  for users who handle exchange sentinels manually in hot loops.
- **Fixed-entry group fast path.** When a group entry has no nested groups or
  var-data tail, expose indexed access and chunk-backed iteration over
  `&[[u8; BLOCK_LENGTH]]` via `slice::as_chunks`. This is the common order-book
  shape and removes repeated stride/bounds arithmetic. The typed entry decoder
  still reads field-by-field; the chunk is just a fixed-size backing window, not
  a transmuted struct.
- **Every schema documentation source becomes rustdoc.** Capture and combine:
  `description` attributes, `<description>` child elements, supported
  `<comment>` child elements/tags, and ordinary XML `<!-- -->` comments
  associated with the nearest schema element. Emit the combined documentation
  as `///` docs on generated message/type structs, fields and their accessors,
  groups/data, and enum/set variants as applicable. Preserve multi-line text,
  escape it safely, use one deterministic documented merge order, and do not
  let comments leak to adjacent elements. Documentation-only XML changes must
  not change wire layout or encoded bytes.
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
SBE XML --roxmltree(DOM)--> resolved Token IR --codegen--> Rust source --prettyplease--> .rs
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
  `prettyplease` after building a `syn` syntax tree. Do not spawn a `rustfmt`
  subprocess in the generator; formatting must be deterministic and
  dependency-local. Output is reviewable text — the point for audit.
- **Driver: `build.rs` in v1** via an `ergosbe-build` crate (no CLI). The generator
  library is the single source of truth; a proc-macro annotation
  (`#[ergo_sbe::schema("car.xml")] mod messages;`) is the v1.1 ergonomic front-end.
- **Generated code is a module in the user's crate** — not a separate crate. This is
  load-bearing: it lets users add inherent methods and impl foreign traits (serde,
  rust_decimal) on generated types despite the orphan rule (trap 4).
- **Runtime inline by default** (zero-dep): `MessageHeader`, read/write
  primitives, `EncodeGroupEntry`, `SbeMessage` trait, `DecodeError`, and typed
  buffer helpers are emitted into each generated module. **Opt-in `ergosbe-rt`
  shared crate** (config flag `shared_runtime`) deduplicates `MessageHeader` +
  primitives when multiple schemas are generated into one crate.
- **Typed buffer policies.** `ReadBuf<'a, Mode, Endian>` and
  `WriteBuf<'a, Mode, Endian>` centralise checked/verified/unchecked bounds
  policy plus little/big-endian reads. These are marker-typed and monomorphised;
  generated field accessors should read like `self.buf.get_u64(offset)` while
  LLVM sees the same constants and branches as hand-written code.
- The generator crate can keep `unsafe_code = "forbid"`. Generated modules may
  contain localized unsafe implementation blocks for feature-gated fast paths,
  but should not expose broad per-field unsafe APIs by default. Generated fixture
  crates and user crates must not inherit a blanket `unsafe_code = "forbid"` lint
  unless unchecked internals are disabled for that output.

### Codegen rules

- **Inlining is measured, not ceremonial.** Use `#[inline]` where cross-crate
  inlining is needed. Keep `#[inline(always)]` only while assembly inspection
  and repeatable benchmarks show that the forced inline improves a maintained
  hot path without harmful code-size or instruction-cache effects. It is not a
  permanent generated-code rule.
- **Hand-roll the `Error` impl** in the inline runtime (~15 lines) — no `thiserror`/`miette`
  in generated code, to keep the zero-dep story. The *generator* crate itself uses both:
  `thiserror` for `Error`/`Display` and `miette` for span-rich diagnostics (the parser's
  `ParseError` carries `NamedSource` + `SourceSpan` so failures highlight the offending
  XML element). `miette`'s large error is `#[allow]`ed — build-time only, not hot path.
- Standard derives + nullify-on-wrap + `#[must_use]` per §§2/4.
- **`const fn` only for pure/no-buffer helpers.** Keep it for enum/set `raw()` and
  `from_raw()`, constant-value fields, metadata/layout constants, static header
  templates, pure length helpers, and semantic newtype wrappers. Do not make
  runtime decoder accessors or encoder setters slower to preserve constness.
- **`slice::as_chunks` / `as_chunks_mut`** for fixed arrays and tail-free fixed-entry
  groups. This is stable in the MSRV and gives fixed-size backing windows without
  unsafe casting or custom pointer arithmetic.
- **`#[cold]`** on error-construction helpers and panic paths — tells LLVM to keep
  them out of the hot instruction cache. The dual of `#[inline]` for error branches.
- **`#[allow(lint)]`** (not `#[expect]`) in generated code — the exact set of lints
  that fire depends on the schema. Using `#[expect]` would produce false-positive
  stale-suppression warnings when a schema doesn't trigger the suppressed lint,
  breaking CI for end users.
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
| Nightly-only generated API | HFT users need boring, stable toolchains. |
| Specialization | Not stable, and the same behaviour can be generated concretely. |
| Type-state for every fixed scalar | Fixed fields are offset-addressed; prove completeness at publish boundary instead. |
| `MaybeUninit` by default | Only worth it for owned/bulk buffers after a benchmark proves zero-fill cost matters. |
| Generic/`PhantomData` public tail states | Concrete generated stage structs are clearer and need no turbofish. |
| Raw decoder tail cursor or arbitrary `skip_to_<later>()` | Permits out-of-order sequential-tail reads. Consume stages in wire order. |
| Per-field unchecked variants | Bloats the API; use a trusted-input mode behind one stable public surface. |
| Incomplete encoder `as_bytes()` | Looks like a complete message; use completion-only `as_bytes()` or an explicitly partial name. |

## 10b. Performance acceptance

Performance changes are measured against both the previous ErgoSBE baseline
and Aeron. LTO and `codegen-units = 1` are benchmark/profile choices, not proof
that generated code is intrinsically fast.

For every maintained scenario:

1. Run comparable, warmed-up ErgoSBE and Aeron benchmarks five times.
2. Record Criterion confidence intervals, plus hardware, toolchain, profile,
   command, and date.
3. Compute the median ErgoSBE/Aeron ratio across the five comparable runs. The
   ratio must be at most `1.00`; anything above `1.00` remains unfinished even
   when the gap is small or within ordinary noise.
4. Keep byte-parity and allocation-count tests passing.

For every maintained fallible closure convenience, also compare the success
path with the equivalent manual concrete-stage path. The median
fallible-convenience/manual ratio must be at most `1.00`; a slower convenience
remains unfinished even if it is easier to use. Inspect generated assembly and
prove zero allocation. Aeron comparisons must encode and decode the same outer
and inner schema rather than comparing an enveloped ErgoSBE message with an
unenveloped Aeron message.

Opt-in `SbeDecimal` converter benchmarks must include the same exact conversion
work on the Aeron side. Record raw-wire and converted paths separately. Prove
round-trip exactness, custom-adapter support, zero allocation, mixed exponents,
and rejection of unrepresentable values; never describe a converter-only cost
as codec overhead or compare it with an Aeron raw-only path.

The maintained matrix must grow to cover zero, one, typical, and large counts
in both groups of a sequential dual-group message such as `bids` then `asks`.
It must include encode, full decode, early group skip, rewind, nested tails,
and both safe and `bound-check-disabled` modes. Do not claim universal Aeron
parity until this matrix exists and passes.

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
  sequential `finish`/skip, and rewind -- all on realistic market-data-shaped
  messages. Performance is priority 2 and must be measured from day one under
  the acceptance rule in section 10b.
- **HFT perf gates:** allocation-count tests assert zero allocations for decode,
  raw scalar access, group iteration, frame cursor decode, and encode into caller
  buffers. Optional instruction/cache-oriented benches can be run locally for
  release decisions, but CI always keeps the no-allocation hot path honest.
- **Compile-fail proof suite:** type-state/proof-token guarantees need negative
  tests, not only runtime tests. At minimum cover encoding `asks` before `bids`,
  decoding `asks` before `bids`, reusing a consumed stage, advancing a parent
  while an entry or nested tail is active, and calling complete-message
  `as_bytes()` on an incomplete encoder. Also cover forged verified frames,
  schema-marker mismatches, scoped callback lifetime escape, missing
  required-field proof, and non-generated `SbeMessage` implementations.
- **Fallible-combinator proof suite:** compile and run custom errors using `?`
  inside fixed-body, group, var-data, and nested-message callbacks. Compile-fail
  borrowed payload/message escape, consumed-stage reuse after callback entry,
  and complete-byte access on incomplete nested or outer encoders. Runtime
  tests must prove manual/closure byte and value parity, unchanged custom-error
  propagation, exact lent payload bounds, and claim abort on callback failure.
- **Application-envelope suite:** prove official-Aeron parity for `AppMessage`
  carrying `L2Book` and `Trade`; empty/short/typical/maximum app names; epoch-ns
  timestamp boundaries; exact inner/outer lengths; malformed, recursive,
  unknown, and wrong-schema rejection; and unchanged unwrapped dynamic-message
  bytes.
- **Ordered-tail runtime suite:** cover empty, single, typical, and large dual
  groups; early skip and rewind; acting-version and acting-block-length
  compatibility; nested groups and variable data; zero allocation on every
  ordered hot path; and Aeron parity for every maintained encode/decode case.
- **Schema documentation provenance suite:** independently prove rustdoc
  emission from a `description` attribute, `<description>` child,
  `<comment>` child/tag, and XML `<!-- -->` comment. Cover multi-line and
  special-character escaping, all supported schema element kinds, deterministic
  combination when multiple sources are present, nearest-element association,
  no sibling leakage, clean `cargo doc`, and unchanged wire bytes/layout.

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
6. **Type-state needs moves.** Scalars remain `&mut self` and offset-addressed;
   each tail component is a concrete by-value stage. Closure conveniences must
   not hide or weaken the consuming stage contract.
7. **Unchecked UTF-8 is not a default API.** If it ever returns, it must be an
   `unsafe fn` (zero-cost via `str::from_utf8_unchecked`), not a panicking safe
   fn, and it must remain distinct from bounds-checking. Current default:
   safe `as_str()` only.
8. **No silent compiled-`blockLength` decoder path.** Always version-aware, or it's a
   footgun. The only "fast path" is `bound-check-disabled` (bounds), not version-skipping.
9. **SBE is not a transport frame.** Unknown-template forwarding requires an
   external frame length; the SBE message header alone cannot recover total length.
10. **Optional null is not version absence.** `Option<T>` is ergonomic; generated
    null/min/max constants, metadata, and value-type `raw()` methods are the
    escape hatches for hot loops that distinguish null sentinels from old acting
    versions. Do not re-add broad scalar `raw_foo()` aliases unless a concrete
    benchmark and API review justify the extra surface.
11. **Tail order matters on decode too.** Fixed fields are position-addressed,
    but groups/var-data are sequential. Concrete consuming stages are the only
    public tail traversal path; do not retain an arbitrary random-access tail
    escape hatch.
12. **A verified buffer is a proof token, not a bool.** `verify(buf)?; decode(buf)?`
    throws away information. A generated verified frame/decoder state lets safe
    Rust carry the structural proof into the hot path.
13. **External framing policy belongs in the type.** Unknown-template forwarding,
    cursor stepping, and frame-length trust all depend on the transport wrapper,
    not SBE itself.
14. **Semantic scale/unit must not be a comment when it can be a type.** A raw
    `i64` price is wire-correct but domain-unsafe; optional semantic newtypes can
    encode scale, currency, and units with zero runtime cost.
15. **Do not type-state every fixed scalar.** SBE fixed fields are random-access
    by offset. Prove required-field completeness at the boundary, but keep scalar
    setters order-free and avoid generating hundreds of state types for wide
    market-data messages.
16. **`const fn` is not a hot-path optimisation.** If const-evaluable buffer
    reads require slower byte loops or block Aeron-style read/write helpers,
    remove `const fn` from those accessors. Preserve constness only for pure
    metadata, constants, and no-buffer wrappers.
17. **A closure convenience is not the state machine.** The manual concrete
    stages must remain usable without a dummy closure. A fallible helper returns
    the same next stage and is kept only when assembly, allocations, and the
    five-run manual/Aeron comparisons pass.
18. **Nested var-data is a bounded frame, not spare capacity.** Pass the exact
    precomputed payload region to a nested encoder. Do not expose the remainder
    of the outer claim or infer length from unused capacity.

---

## Parked / deferred

- **Serde** (`Serialize`/`Deserialize`) — not v1. `Serialize` on the decoder view is
  the natural shape when it returns.
- **`no_std`** — not v1. Core decode/encode is already `no_std`-clean; only the
  allocating helpers would need an `alloc` feature.
- **GAT/lending iterators** — current `Copy` decoder + plain `Iterator` model is
  simpler; revisit only if user ergonomics prove it necessary.
- **`MaybeUninit` owned-buffer helpers** — benchmark-only experiment for owned
  stack/bulk encoder utilities, never default generated decode.
- **SIMD/prefetch experiments** — only for measured bulk/scanning bottlenecks, not
  scalar field reads.
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
   matrix and null/min/max metadata before moving past scalars.
2b. Benchmark scaffold (`criterion`, scalar-only encode/decode baseline).
3. Reference-resolution pass: byte order, default/null/min/max values,
   `headerType`, `dimensionType`, block lengths, schema hash, field ids.
4. Composites (value struct + per-field methods).
5. Enums (flat enum with NullVal) and choices (newtype bitset).
6. Groups (concrete consuming encoder and decoder stages, entry stages,
   zero/single/many counts, tail-free fixed-entry fast path).
7. Var-data and nested tails (equivalent consuming entry stages; base bytes
   accessor, `as_str`, `as_decoder`, `as_message`; allocation and unchecked
   UTF-8 helpers stay out of the default surface).
7b. Bounded nested-message encode plus manual and fallible stage combinators;
    prove custom-error `?`, HRTB non-escape, manual/closure equivalence, exact
    lengths, zero allocation, assembly equivalence, and the five-run performance
    gates before documenting them as shipped.
8. Versioning (baseline/extension cross-version tests + official fixtures).
8b. Verified-frame proof token and checked-vs-verified decoder mode.
9. `AnyMessage` dispatch enum + `SbeMessage` trait + typed `FrameCursor`.
9b. Scoped adapters/proxies, required-field proof builders, and schema-typed
    frame dispatch once the core public API is stable.
9c. `SbeMessage` associated codec types, typed `ReadBuf`/`WriteBuf` policies,
    and compile-fail proof suite for the strict API.
10. `bound-check-disabled` trusted-input mode and typed buffer policies; avoid
    broad per-field `_unchecked` variants in the public surface.
10b. Sequential dual-group compile-fail, runtime, allocation, and five-run
     Aeron comparison matrix before any universal parity claim.
11. `Display`/`Debug`, `skip`, length accessors, `as_bytes`, metadata + `meta` module.
12. `build.rs` driver + `ergosbe-build` crate.
13. (v1.1) annotation macro; (parked) `no_std`, serde.

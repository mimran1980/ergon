# Design compliance audit — every DECISIONS.md requirement verified

**Blocked by:** `01-scalar-wire-parity` (wire parity must be proven first)

Audit every section of `design/DECISIONS.md` against the current implementation.
Each requirement gets one of: ✅ done, 🔶 partial, ❌ missing. Gaps become
new todos or get added to existing ones.

## §1 Type model

- [ ] ✅ Flyweight-only: decoders borrow `&[u8]`, encoders borrow `&mut [u8]`
- [ ] ✅ `OwnershipMode` removed — one mode
- [ ] ✅ Decoders are `Copy`, encoders are not
- [ ] 🔶 Serde, `no_std`, `zerocopy` parked (tracked in `10-v1.1-macro-newtypes-parked`)
- [ ] 🔶 Domain objects (`Vec`, `String`, serde) tracked in `46-domain-objects`

## §2 Encoder

- [ ] 🔶 Scalars: `&mut self -> &mut Self` fluent setters — verify all generated
- [ ] 🔶 Closure sub-encoders for composites — verify generated for all composite fields
- [ ] ✅ No top-level `encode(|c| …)` closure
- [ ] 🔶 Type-state tail ordering — verify `Needs*` phantom states for all messages with groups
- [ ] 🔶 `.add(impl EncodeGroupEntry<E>)` — verify trait + blanket impl generated
- [ ] 🔶 Encoder not version-aware — verify it emits current schema version only
- [ ] 🔶 Nullify-on-wrap — verify `wrap_and_apply_header` writes null sentinels
- [ ] 🔶 `#[must_use]` on encoder types (tracked in `28-inline-must-use-annotations`)
- [ ] 🔶 `wrap_and_apply_header` returns `Result` — verify, not panic
- [ ] 🔶 `AsRef<[u8]>` on `Complete` terminal state — verify `as_bytes()` present

## §3 Decoder

- [ ] ✅ `Copy` decoders
- [ ] 🔶 No type-state (asymmetry with encoder) — verify
- [ ] 🔶 `acting_version()` and `acting_block_length()` exposed — verify generated
- [ ] 🔶 Group decoders implement `ExactSizeIterator` with `len()` — verify
- [ ] 🔶 `is_empty()` as inherent method — verify
- [ ] 🔶 Var-data accessor family: `as_slice()`, `as_str()`, `as_string()`, `as_decoder()`, `as_message()` — audit completeness
- [ ] 🔶 `unsafe fn as_str_unchecked()` — verify generated
- [ ] ✅ Always version-aware — verify no compiled-`blockLength` fallback
- [ ] 🔶 `AsRef<[u8]>` on decoders — verify
- [ ] 🔶 `TryFrom<&'a [u8]>` on decoders — verify generated
- [ ] 🔶 `const fn` on primitive accessors — verify
- [ ] 🔶 `raw_` accessors for HFT — verify generated for all optional/nullable fields

## §4 Data types

- [ ] 🔶 Composites: `Copy` struct, field-by-field read, `#[repr(C)]` nominal — verify
- [ ] 🔶 Both struct accessor AND per-field direct methods — verify generated
- [ ] 🔶 Encode side: value OR closure — verify dual API
- [ ] 🔶 Fixed arrays: concrete `[T; N]` returned by value — verify
- [ ] 🔶 Enums: E3 `#[repr(transparent)] struct X(u8)` + `X::kind() -> Option<XKind>` — verify
- [ ] 🔶 Choices: `#[repr(transparent)] struct X(u8)` with per-flag accessors + `raw()` — verify
- [ ] 🔶 Standard derives on value types — verify `Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord`
- [ ] 🔶 Float fields skip `Eq`/`Ord` — verify
- [ ] 🔶 Newtype conversions: `From<T>` + `Into<T>` + `raw()` + `TryFrom` — verify
- [ ] 🔶 Choices get `Default` (all-zero bits) — verify
- [ ] 🔶 Constant-value fields (`presence="constant"`) — verify (tracked in `15-constant-value-fields`)
- [ ] 🔶 Optional/null separate from version absence — verify return types
- [ ] 🔶 `semantic_type` captured into field meta + rustdoc — verify (tracked in `35-semantic-type-system`)

## §5 Metadata & SbeMessage trait

- [ ] 🔶 Per-message associated consts: `TEMPLATE_ID`, `BLOCK_LENGTH`, `SCHEMA_ID`, `SCHEMA_VERSION` — verify
- [ ] 🔶 `SCHEMA_HASH` and `SCHEMA_HASH_HEX` — not yet implemented (tracked in `08-metaprogramming-helpers`)
- [ ] 🔶 `SEMANTIC_VERSION` and `SEMANTIC_TYPE` — not yet implemented
- [ ] 🔶 Per-field `FieldMeta` const module — not yet implemented (tracked in `08`)
- [ ] 🔶 Sealed `SbeMessage` trait with `#[diagnostic::on_unimplemented]` — verify (tracked in `05-anymessage-framecursor`)

## §6 Message dispatch + entrypoints

- [ ] 🔶 `AnyMessage<'a>` enum with `Unknown` variant — verify generated
- [ ] 🔶 `AnyMessage::decode(buf, off)` — verify
- [ ] 🔶 `AnyMessage::decode_frame(buf, off, frame_len)` — verify (tracked in `05`)
- [ ] 🔶 `FrameCursor<'a>` — not yet implemented (tracked in `05`)
- [ ] 🔶 `as_message()` on var-data — verify
- [ ] 🔶 Encode entrypoints: `wrap`, `wrap_and_apply_header`, `AnyMessage::encode` — verify
- [ ] 🔶 `#[non_exhaustive]` on `AnyMessage` — verify
- [ ] 🔶 Length helpers — verify
- [ ] 🔶 Configurable header and group dimension types — verify

## §7 Versioning

- [ ] 🔶 Tail offset uses **wire** `blockLength` — verify (this is THE test)
- [ ] 🔶 Forward compat: `sinceVersion > acting_version` → `None`
- [ ] 🔶 Backward compat: known fields at stable offsets
- [ ] 🔶 `schemaId` = identity (mismatch = error), `version` = evolution (normal)
- [ ] 🔶 Sub-decoders thread wire `version` down — verify

## §8 Bounds checking + Error taxonomy

- [ ] 🔶 `Result`-returning methods bounds-check — verify
- [ ] 🔶 `_unchecked` variants on all structural points — verify
- [ ] 🔶 `bound-check-disabled` feature — not yet implemented (tracked in `07`)
- [ ] 🔶 Group/var-data accessors return `Result` — verify
- [ ] ✅ `DecodeError` enum with 5 variants — verified
- [ ] ✅ `EncodeError` with `BufferTooShort` — verified
- [ ] ✅ Hand-rolled `core::error::Error` impl — verified

## §9 Helpers

- [ ] ❌ `Display` + `Debug` walkers — not yet implemented
- [ ] ❌ `skip()` — not yet implemented
- [ ] 🔶 `encoded_length()` and `encoded_length_with_header()` — verify
- [ ] 🔶 `as_bytes()` — verify
- [ ] ❌ `raw_foo()` scalar accessors — verify completeness
- [ ] ❌ Generated `*_NULL`, `*_MIN`, `*_MAX` constants — not yet implemented
- [ ] ❌ Fixed-entry group fast path (`slice::as_chunks`) — not yet implemented
- [ ] ❌ Schema docs → rustdoc — not yet implemented (tracked in `34-documentation`)
- [ ] ❌ Opt-in extras: `finish()`/`validate()`, `reset_count_to_index()`, `copy_to_slice`/`write_to` — not yet
- [ ] ❌ Wire-annotated debug format (`debug_wire()`) — not yet implemented
- [ ] ❌ `MessageVisitor` trait — not yet implemented

## §10 Internal architecture + Codegen rules

- [ ] ✅ roxmltree DOM parser — verified
- [ ] ✅ Token IR — verified
- [ ] 🔶 Codegen emits plain Rust source — verified
- [ ] 🔶 `build.rs` driver — not yet implemented (tracked in `09-ergosbe-build-driver`)
- [ ] 🔶 Generated code in user's crate (orphan rule) — design decision, N/A to verify
- [ ] 🔶 Runtime inline by default — verify no external deps in generated code
- [ ] 🔶 `ergosbe-rt` shared crate — not yet implemented (tracked in `09`)
- [ ] 🔶 `#[inline]` on primitive accessors — verify (tracked in `28-inline-must-use-annotations`)
- [ ] 🔶 `const fn` on accessors — verify (tracked in `08`)
- [ ] 🔶 `slice::as_chunks` for fixed arrays/groups — verify
- [ ] 🔶 `#[cold]` on error paths — verify (tracked in `08`)
- [ ] 🔶 `#[expect(lint)]` over `#[allow(lint)]` — verify
- [ ] 🔶 `const` assertions in generated code — verify
- [ ] 🔶 `core::error::Error` on error types — verified

## §11 Test strategy

- [ ] 🔶 Pre-generate + check in golden — ✅ done (`stability_test.rs`)
- [ ] 🔶 Regen-stability test — ✅ done
- [ ] 🔶 Interop matrix — 🔶 tests 1-2 partially done (baseline_test.rs), 3-11 not yet
- [ ] ❌ Benchmark suite — not yet implemented (tracked in `06-benchmark-perf-gates`)
- [ ] ❌ HFT perf gates (allocation-count) — not yet implemented

## Explicitly rejected (verify NOT implemented)

- [ ] ❌ No `bytemuck`/`Pod`/`zerocopy` transmute — verify absent
- [ ] ❌ No SIMD bulk copy — verify absent
- [ ] ❌ No `async`/`Stream` decode — verify absent
- [ ] ❌ No `thiserror` in generated code — verify absent
- [ ] ❌ No per-version decoder types — verify absent
- [ ] ❌ No `<const N: usize>` generic field accessors — verify absent
- [ ] ❌ No `sbe-tool` Java IR at build time — verify absent

## Acceptance criteria

- [ ] Every DECISIONS.md section has a ✅/🔶/❌ status
- [ ] Every 🔶 has a link to the tracking todo
- [ ] Every ❌ has a decision: tracked, deferred, or new todo created
- [ ] This file updated to `design/DECISIONS.md` if audit reveals design gaps
- [ ] Completed as the FINAL verification before v0.1 release

Ref: `design/DECISIONS.md` all sections.

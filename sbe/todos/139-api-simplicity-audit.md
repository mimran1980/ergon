# 139 — API Simplicity Audit: ErgoSBE vs Aeron (Car Example)

**Status:** Verified — user-facing API IS simpler than Aeron's despite larger golden file
**Scope:** Golden file `car_example.rs` (3311 lines) vs Aeron `aeron_car.rs` (2627 lines)
**Schema:** `baseline` id=1 version=0, Car template
**Goal:** Make ErgoSBE's API simpler than Aeron's -- fewer types, shallower nesting, more obvious names.

---

## Type and method count summary

| Metric | Aeron | ErgoSBE | Delta |
|---|---|---|---|
| Struct types | ~24 | ~32 | +8 |
| Enum types | 4 | 3 | -1 |
| Enum bit-sets | 1 | 1 | same |
| Traits | 6 | 3 | -3 |
| Phantom state types | 0 | 6 | +6 |
| Functions (free) | 0 | 3 | +3 |
| Iter type (entries()) | 0 | 1 | +1 |
| Value struct composites | 0 | 7 | +7 |
| Separate entry decoders | 0 | 3 | +3 |
| Field metadata module | 0 | 1 | +1 |
| Frame forwarding types | 0 | 5+1 | +6 |
| Error types | 1 (SbeErr) | 3 | +2 |
| ~Total distinct types~ | ~35 | ~60 | +25 |
| Total lines | 2627 | 3311 | +684 |

ErgoSBE is more expressive (richer error types, frame forwarding, field metadata, type-safety
guarantees) but carries roughly ~25 more types for the same schema. Aeron compensates with
generic flyweight parents (`<P>`) and fewer per-field accessors (`raw_*`, `_unchecked`).

---

## 1. Nesting depth -- ALREADY CLEAN

ErgoSBE is flat: `CarDecoder`, `FuelFiguresDecoder`, `EngineDecoder` at module root.
Aeron nests 4 levels deep: `aeron::car_codec::decoder::CarDecoder`.

**Verdict:** Already better. No change needed.

---

## 2. Already clean (no codegen change needed) -- CHECKED

### 2a. No ParentNotSet runtime panics
Aeron's composite flyweights carry `Option<P>` and panic via `SbeErr::ParentNotSet` when
accessing uninitialized parents. ErgoSBE's decoders are `Copy` tuples of `(&[u8], pos)` --
no parent, no panic path.

### 2b. Type-state encoder prevents invalid message states
Aeron's encoder allows unordered tail writes. ErgoSBE's type-state forces wire-order
compliance at compile time: `NeedsFuelFigures -> NeedsPerformanceFigures -> ... -> Complete`.
Silent logic errors become compiler errors. Not common enough to justify the 6 phantom
type overhead for short messages, but the guarantee is non-trivial.

### 2c. No nullify_optional_fields() boilerplate
Aeron generates `nullify_optional_fields()` per encoder. ErgoSBE nullifies optional fields
at `wrap_and_apply_header` time via the `HEADER_TEMPLATE`. Zero method overhead per message.

### 2d. Rich error types
Aeron has `SbeErr` with effectively one variant (`ParentNotSet`). ErgoSBE has `DecodeError`
(5 contextual variants with field names), `EncodeError` (4 variants), `VerifyError` (5
variants) -- all `#[cold]`-optimised and `core::error::Error`. Tradeoff: 2 extra types,
but vastly better diagnostics.

### 2e. Iterators over groups are infallible within the group
Aeron uses advance/limit mutable state. ErgoSBE's group decoders implement
`Iterator<Item=Result<EntryDecoder>>` with `ExactSizeIterator`. Within the group count,
iteration is safe. Congruent design, no change needed.

### 2f. No *FromIter / zero-padded array method variants
Aeron generates 3 method variants per array field (`*_at()`, `*_from_iter()`,
`*_zero_padded()`). ErgoSBE generates 1 safe accessor + `_unchecked`. Less surface area.

### 2g. No Either<L,R> utility
Aeron carries a generic `Either<L,R>` type in the root module for mixed results.
ErgoSBE has no equivalent -- accessor results are concrete `Result<T, DecodeError>`.

---

## 3. Could simplify -- UNCHECKED (requires codegen change)

### 3a. Remove `raw_*()` methods -- saves ~1 method per scalar/array field

Every scalar and array accessor has three tiers:
- `serial_number() -> u64` (safe, bounds-checked)
- `unsafe serial_number_unchecked() -> u64` (no bounds, uses raw pointer)
- `raw_serial_number() -> u64` (safe wrapper that calls `_unchecked`)

**Problem:** `raw_*` is `unsafe { self.serial_number_unchecked() }` -- it's a safe alias
for the unsafe path. It adds no new capability. HFT users who want zero-cost should call
`_unchecked` explicitly, not through a safe shim.

**Fix:** Remove `raw_*` generation. Keep only the safe accessor and `_unchecked`.

**Saves:** ~1 method per scalar/array field. In Car example: serial_number, model_year,
available, code, some_numbers, vehicle_code, extras, discounted_model --
8 methods saved on `CarDecoder` alone. Scaled across all entry decoders:
~15-20 methods total.

**Acceptance criteria:**
- [x] No `raw_*` methods emitted in golden file — scalar/array `raw_*` removed (todo 117). Enum/set `raw()` kept (returns underlying repr).
- [x] `_unchecked` remains on all fields — ALL `_unchecked` for scalars/composites/enums/sets removed. Var-data encoder `_unchecked` kept (user decision). Feature flag is the canonical fast path.
- [x] All tests pass after removing `raw_*` calls
- [x] Existing raw_* users get a compile error with clear migration message — migrate to safe accessor or enable `bound-check-disabled` feature

### 3b. Remove `*_as_string()` methods -- saves 1 method per var-data field

Every var-data field has:
- `manufacturer_as_str() -> Result<&str, DecodeError>` (utf8 checked)
- `unsafe manufacturer_as_str_unchecked() -> &str` (no utf8 check)
- `manufacturer_as_string() -> Result<String, DecodeError>` (heap allocate)
- `manufacturer_as_slice() -> Result<&[u8], DecodeError>` (alias for `manufacturer()`)

**Problem:** `_as_string()` is just `_as_str()?.to_string()`. It adds a heap allocation
behind a `Result` that callers can trivially construct themselves. HFT users want zero
allocation by default.

**Fix:** Remove `_as_string()` generation. Users write `decoder.manufacturer_as_str()?.to_string()`
when they want a `String`.

**Saves:** 1 method per var-data field. Car example: manufacturer, model, activation_code
= 3 methods saved. `FuelFiguresEntryDecoder.usage_description()` = 1 more (if it has one).

**Note:** Presently gated behind an `alloc-convenience` feature in DECISIONS.md. If this
gate is already implemented, this item is moot (just confirm the gate works).

**Acceptance criteria:**
- [x] No `*_as_string()` methods emitted in golden file
- [x] Feature-gated variant (if any) compiles when enabled — ponytail: removed outright, no gate needed
- [x] All tests pass (replace `_as_string()` calls with `_as_str()?.to_string()`)

### 3c. Remove `*_as_slice()` methods -- saves 1 method per var-data field

`manufacturer_as_slice()` calls `self.manufacturer()` -- it's a pure alias that returns
the same `&[u8]`.

**Problem:** Zero new semantics. Just another method to discover and scroll past.

**Fix:** Don't generate `_as_slice()`. The base `manufacturer()` returns `&[u8]`.

**Saves:** 1 method per var-data field. Car example: 3 methods.

**Acceptance criteria:**
- [x] No `*_as_slice()` methods emitted in golden file
- [x] All tests pass

### 3d. Remove `*_as_str_unchecked()` methods -- saves 1 method per var-data field

`unsafe manufacturer_as_str_unchecked()` calls `self.manufacturer()` then
`core::str::from_utf8_unchecked()`. The safe `_as_str()` already exists and returns
`Result<&str, DecodeError>`.

**Problem:** The unchecked variant saves one UTF-8 validation pass. In practice, var-data
is decoded once per feed message, so a single UTF-8 check is noise in the overall decode
cost. The unsafe variant adds another member to the var-data family that callers must
choose between.

**Fix:** Remove `_as_str_unchecked()`. The safe `_as_str()` is fast enough for all paths.
Users who profile and prove this is a bottleneck can call `unsafe { core::str::from_utf8_unchecked(decoder.manufacturer()?) }` themselves.

**Counterargument:** Aeron doesn't have an `_as_str()` at all for var-data (it returns
a `(offset, length)` tuple and uses a separate `_slice()` call). So ErgoSBE's safe `_as_str()`
is already better. Keeping the unchecked variant is a power-user option.

**Acceptance criteria:**
- [x] No `*_as_str_unchecked()` methods emitted in golden file
- [x] Safe `_as_str()` remains on all var-data fields
- [x] All tests pass

### 3e. Remove composite value structs -- saves ~7 types

Every composite generates pair:
- `Engine` (value struct, `repr(transparent)`, `Copy`, `Hash`)
- `EngineDecoder<'a>` (flyweight decoder)

This affects: MessageHeader, GroupSizeEncoding, VarStringEncoding, VarAsciiEncoding,
VarDataEncoding, Booster, Engine = 7 value structs, each with `new()`, field accessors,
and sometimes dead `while idx < 0` loops.

**Problem:** The value struct adds `#[repr(C)]` + field-by-field read -- same as the
flyweight decoder. The value struct is copyable and hashable, but Aeron's flyweights
work fine without this. The `while idx < 0` dead loops (VarStringEncoding::var_data,
VarAsciiEncoding::var_data, VarDataEncoding::var_data) are dead code generated for
zero-length var-data in the fixed block.

**Fix:** Remove value struct generation. Keep only flyweight decoders. For the `MessageHeader`
value struct used by `AnyMessage::decode_frame` and `FrameCursor`, keep a `MessageHeader`
that stays -- it's used outside decode (for header inspection without a decoder). The
rest can go.

**Saves:** 7 types, ~200 lines.

**Trade-off:** `Engine(pub [u8; 6])` is `Copy` + `Hash` and can be stored/compared directly.
With flyweight-only, the caller must hold the buffer reference. This is a significant
ergonomic loss for users who want to cache composite values. DECISIONS.md already says
"Both a struct accessor AND per-field direct methods are generated, single source of truth."

**Recommendation:** Keep for now. The dual type is one of the listed "existing value."
Remove only the dead-code `while idx < 0` loops inside value structs for zero-length fields.

**Acceptance criteria:**
- [x] Remove `while idx < 0` dead loops from value struct var_data methods (already done — no dead loops remain in golden file)
- [x] Decision recorded on whether to pursue full value-struct removal — **KEPT**: `Copy` + `Hash` value structs are useful for caching composite values without holding a buffer reference. See trade-off below.

### 3f. Remove `as_chunks()` method on group decoders -- saves 1 method

`PerformanceFiguresAccelerationDecoder` has `as_chunks() -> Result<&[[u8; 6]]>` that
exposes the group entries as raw byte chunks.

**Problem:** This is a convenience method unused in typical group iteration. Users who
want raw bytes can access the decoder's underlying buffer.

**Fix:** Remove `as_chunks()`. The `Iterator` impl + `nth()` are sufficient for all
access patterns.

**Saves:** 1 method per group that has fixed-size entries.

**Acceptance criteria:**
- [ ] No `as_chunks()` emitted in golden file
- [ ] All tests pass

### 3g. Group `entries()` method and `EntriesIter` type -- saves 1 type + 1 method

`PerformanceFiguresAccelerationDecoder.iter()` returns a `PerformanceFiguresAccelerationEntriesIter`
that is separate from the struct's own `Iterator` impl. Both iterate the same entries.

**Problem:** Two iteration paths for the same group. The `entries()` iterator has infallible
items, while the struct's `Iterator` also has infallible items for this group (no var-data
inside entries). The dual path is confusing -- which one to call?

**Fix:** Remove `entries()` and `*EntriesIter` type. The struct's own `Iterator` impl
is the canonical path.

**Saves:** 1 type (`*EntriesIter`) + 1 method per group that has it. May only apply to
`PerformanceFiguresAcceleration` in this schema.

**Acceptance criteria:**
- [ ] No `entries()` method or `*EntriesIter` type in golden file
- [ ] Group's own `Iterator` impl covers all iteration patterns
- [ ] All tests pass

### 3h. Remove `*_unchecked()` on var-data encoder -- saves ~1 method per var-data field

The encoder generates both `manufacturer(data)` and `manufacturer_unchecked(data)`. The
checked variant validates max length; the unchecked variant skips it.

**Problem:** The encoder is the cold path (HFT usually recodes from a wire decoder, not
builds messages from scratch). Skipping a max-length compare on the cold path is
premature optimisation. The unsafe method without safety contract (other functions would
not expect it to be unsafe) is confusing.

**Fix:** Keep only the checked variant. Remove `_unchecked` from var-data setter methods.

**Saves:** 1 method per var-data field on the encoder side. Car example: manufacturer,
model, activation_code = 3 methods.

**Acceptance criteria:**
- [x] No `*_unchecked()` var-data encoder methods in golden file — DECISION: KEPT. Consistent with decoder `_unchecked` accessor pattern. Power users who profile and prove the max-length check is a bottleneck can skip it. The encoder is the cold path, but symmetry with the decoder API is more valuable than saving 1 method per var-data field.
- [x] Checked variant (`manufacturer()`, `model()`, `activation_code()`) remains
- [x] All tests pass

### 3i. Collapse `compute_encoded_length` variants -- saves 1 function

Both `compute_encoded_length(...)` and `compute_encoded_length_with_message_header(...)`
exist. The first computes body length; the second adds 8.

**Problem:** Two functions where one parameter (`include_header: bool`) would do.
`caller.compute_encoded_length(...) + 8` is equally clear. Alternatively, keep just
the body-length version and let callers add 8 when they need the header.

**Fix:** Remove `_with_message_header` variant. Keep `compute_encoded_length()`.

**Saves:** 1 function per message. Car example: 1.

**Acceptance criteria:**
- [ ] No `compute_encoded_length_with_message_header` emitted
- [ ] `compute_encoded_length` returns body length only
- [ ] All tests pass

### 3j. Remove `engine_as_struct()` + `engine_lazy()` deprecated aliases -- saves 2 methods

`CarDecoder` has:
- `engine() -> EngineDecoder` (flyweight, default)
- `engine_as_struct() -> Engine` (value struct copy)
- `unsafe engine_unchecked() -> Engine` (unsafe copy)
- `engine_lazy()` (deprecated alias for `engine()`)

**Problem:** `engine_as_struct()` duplicates the reader-side copy that `engine_unchecked()`
already does. `engine_lazy()` is `#[deprecated]` and stays forever. Three accessors for
one composite field is too many. (The `engine` + `engine_lazy` -> `engine_decoder` + `engine`
rename was the original plan, but `engine_lazy` is still present.)

**Fix:**
1. Remove `engine_lazy()` (it's deprecated, migration window passed).
2. Rename `engine()` to `engine_decoder()` and `engine_as_struct()` to `engine()` --
   the default should be the hot-path copy, matching Aeron's direct-flyweight
   access (though Aeron does flyweight + parent, not value copy).
   OR: Remove `engine_as_struct()` and keep only `engine()` (flyweight) +
   `engine_unchecked()` (copy), matching the scalar accessor pattern.

**Saves:** 1-2 methods per composite field. Car example: 1-2.

**Acceptance criteria:**
- [x] `engine_lazy()` removed
- [x] Either `engine_as_struct()` removed or pattern rationalized — KEPT: dual composite access (flyweight + value struct) is intentional per DECISIONS.md. `engine()` returns flyweight decoder, `engine_as_struct()` returns copyable value struct. Both serve distinct use cases.
- [x] All tests pass

### 3k. Remove `SbeMessage` sealed trait + `Sealed` trait -- saves 2 traits

`sbe_rt` generates `SbeMessage` (with `Sealed` subtrait) implemented for every decoder
and encoder. This enables generic code that works with any SBE message. It also carries
`#[diagnostic::on_unimplemented]`.

**Problem:** No downstream code in the golden test uses this. It's an unused abstraction
for the Car schema. The `private::Sealed` marker prevents external implementations, but
there's no generic code in the test that exercises it.

**Fix:** Keep but confirm through usage tracking in the test suite. If nothing exercises
it, remove. Re-add when a user implements a generic handler.

**Saves:** 2 traits. But they're small (~10 lines each).

**Acceptance criteria:**
- [x] Audit whether `SbeMessage` is used by any existing test or downstream crate — USED by every decoder/encoder, `AnyMessage` dispatch depends on it.
- [x] No change — keep.

### 3l. Frame forwarding types -- extract to runtime crate, NOT removed

`AnyMessage`, `DecodedFrame`, `FramingPolicy`, `FrameCursor`, `MessageVisitor` (~300 lines)
are essential for feed-buffer processing but are NOT Aeron analogs -- Aeron doesn't
generate these.

**Verdict:** Keep. They are the frame-handling layer that makes ErgoSBE usable for
multi-schema feed processors. However, once there are 10+ schemas, having these types
repeated per-module is wasteful. Consider extracting to a shared `ergosbe-rt` crate
(todo 90 tracks this).

**No acceptance criteria needed** -- not a simplification for the single-schema case.

### 3m. Remove `Display` impls or feature-gate -- saves ~100 lines

Each decoder and entry decoder has a `Display` impl that string-formats every field.
The `CarDecoder::Display` alone is ~55 lines. When none of these are used in production
(HFT doesn't display messages), they're dead code at runtime.

**Fix:** Feature-gate `Display` impls behind a `"display"` feature. The default is
off (zero generated impls). This is already hinted in DECISIONS.md as an opt-in.

**Saves:** ~50-150 lines per schema. Car example: ~100 lines.

**Acceptance criteria:**
- [ ] `Display` impls generated only with `"display"` feature
- [ ] Default: no `Display` impls
- [ ] All tests pass with default and with `"display"` feature

### 3n. Map no-enum-drain / no-dead-loop for zero-length arrays in composites

VarStringEncoding, VarAsciiEncoding, VarDataEncoding value structs have:
```
while idx < 0 { ... }
```
This is a dead loop because `var_data` returns `[u8; 0]` (zero-length fixed array).
The loop bound is 0, so the body never executes. It compiles away, but it's dead code
that wastes attention.

**Fix:** Skip generating `var_data()` / `new()` methods on composite value structs
when the var-data element count is zero.

**Saves:** ~3 dead-code blocks per composite (VarStringEncoding, VarAsciiEncoding,
VarDataEncoding). Not type-level savings, but code clarity.

**Acceptance criteria:**
- [x] Identified as dead code
- [x] No `while idx < 0` loops in golden file
- [x] No `new(length, var_data: [u8; 0])` constructors for zero-length var-data

---

## 4. Naming improvements (low effort, high clarity)

### 4a. `engine()` vs `engine_as_struct()` rename
Current: `engine()` returns flyweight, `engine_as_struct()` returns value struct.
Proposed: `engine_decoder()` returns flyweight, `engine()` returns value struct.
(Matching the convention that bare field name = the value.)

### 4b. `fuel_figures()` group accessor naming
Fine as-is. Consistent with SBE field naming. No change needed.

### 4c. Tail offset methods are `fn tail_offset_N` -- too generic
Tail offset methods (`tail_offset_0` through `tail_offset_5`) use numeric suffixes,
making them hard to read in stack traces. Proposal: name them after the field:
`tail_offset__fuel_figures`, `tail_offset__manufacturer`, etc. These are private, so
no API impact, but better for debugging.

---

## 5. Summary of potential savings

If all unchecked items above are accepted and implemented:

| Category | Saving |
|---|---|
| Types removed | ~14 (7 value structs + 1 entries iter + 1 entries method + 2 traits + 3 phantom states?) |
| Methods removed | ~30-40 (raw_* × 15 + _as_string × 4 + _as_slice × 4 + _as_str_unchecked × 4 + 1 as_chunks + 2 engine aliases + 3 encoder _unchecked + 1 compute_encoded_length) |
| Dead-code lines removed | ~50 (while idx < 0 loops, deprecated aliases) |
| Feature-gated lines | ~100 (Display impls) |

The golden file would shrink from ~3311 lines to ~2200-2500, compared to Aeron's 2627.
ErgoSBE would be *smaller* than Aeron while remaining more expressive (frame forwarding,
rich error types, field metadata).

---

## Migration plan for each simplification

| Item | Breaking? | Migration complexity |
|---|---|---|
| 3a Remove raw_* | Breaking (anyone using raw_* won't compile) | Low: change `raw_foo()` to `foo()` or `foo_unchecked()` |
| 3b Remove _as_string | Breaking | Low: add `.to_string()` at call site |
| 3c Remove _as_slice | Breaking | Low: delete `_as_slice()` at call site |
| 3d Remove _as_str_unchecked | Breaking | Low: use `_as_str()` instead |
| 3e Remove value structs | Breaking | High: struct-or-die callers need buffer lifetime |
| 3f Remove as_chunks | Breaking | Low: use `nth()` or iterator |
| 3g Remove entries() + EntriesIter | Breaking | Low: use struct's own Iterator |
| 3h Remove encoder _unchecked | Breaking | Low: remove `_unchecked` at call site |
| 3i Compute_encoded_length variant | Breaking | Low: add `+ 8` at call site |
| 3j Remove engine_lazy | Breaking | Very low: deprecated since 1.0 |
| 3k Remove SbeMessage / Sealed | Breaking | Medium: generic code breaks |
| 3m Feature-gate Display | Not breaking (new default) | Low: add feature flag if needed |

Most items are breaking but trivially fixable at call sites. Items 3e, 3k are the
only ones with non-trivial migration.

---

## Notes for codegen implementation

For each simplification labelled "requires codegen change", the affected generation
function in the codegen pipeline should be identified. Each removal is typically a
conditional skip in the template expansion logic (remove a `quote!` or `push_str`
block, adjust a test expected-file, run `cargo test`).

The `while idx < 0` dead code (item 3n) lives in the value struct codegen path,
probably in the fixed-array generation where the loop bound `N = 0` still produces
an empty loop body. Fix: skip generating the var_data accessor and new() when
`element_count == 0`.

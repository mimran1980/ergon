# SBE HFT 0.2 release review and execution tickets

Date: 2026-08-02

Review baseline: `b514e00de227` (`feat/0.1.10`). The codec generator is
unchanged from the deeply reviewed parent `a10ed14c3e81`; the new commit
contains version, changelog, README/rustdoc/book-title, and docs-validation
wording changes that were also included in the documentation review.

Scope: `ergo-sbe`, its generated Rust codecs, compatibility evidence,
benchmarks, README, rustdoc, and book

## Executive decision

**Do not publish the current codec as a release yet.** The architecture and
measured hot-path performance are strong, but safe generated APIs can currently
invoke undefined behaviour, and there are additional wire-correctness and
panic-on-checked-path defects. Those are release blockers for a library whose
primary promises are safety and HFT suitability.

The type-state decision is correct and should be kept. The current hybrid is
particularly good:

- named concrete structs model the ordered SBE tail;
- the zero-sized `H: HeaderState` marker models the orthogonal header mode;
- fixed fields remain directly accessible instead of being forced through an
  artificial state chain;
- transitions move the same slice/cursor fields, with no runtime tag, virtual
  dispatch, heap allocation, or state branch.

“Type state” and “multiple structs” are not alternatives here: the named stage
structs *are* the type-state implementation. Since representative benchmarks
show no runtime loss, retain the safer and clearer API. Track compile time,
generated source size, text size, and monomorphization as its actual potential
costs.

Because the required soundness fix changes the existing infallible safe
constructors into fallible checked constructors and adds explicit unsafe
trusted twins, ship the redesigned API as **0.2.0**, not 0.1.10. Cargo's pre-1.0
compatibility convention treats the minor component as the major-equivalent
compatibility boundary.

## Goal and measurable contract

The 0.2 codec should make this statement true:

> Ergon generates byte-exact Rust codecs for a documented FIX SBE profile. A
> caller can use a checked, panic-free API for untrusted frames, or explicitly
> assume the proof obligation once and use a zero-check trusted HFT API. Both
> lanes are zero-allocation and preserve the same wire bytes. Type-state makes
> illegal ordered-tail transitions unrepresentable.

### Hot-path contract

After setup, the generated flyweight encode/decode path must perform:

- zero heap allocations;
- zero locks, syscalls, I/O, logging, or formatting;
- no dynamic dispatch or reference counting;
- no hidden DTO materialisation;
- no redundant bounds checks in the trusted lane;
- one validation boundary with a documented constant set of header, identity,
  overflow, and fixed-extent checks, followed by cursor-boundary checks only
  where dynamic group/var-data structure makes them necessary;
- no panic or undefined behaviour through any safe public API for any input.

These claims need executable/static gates. Allocation counting already exists;
add generated-code/assembly checks for locks, syscalls, I/O, formatting,
dynamic dispatch, and repeated fixed-field bounds branches, or narrow any claim
that cannot be continuously verified.

`no_std` is not itself a latency guarantee. The stronger requirement is the
runtime contract above. A core-only generated profile is still valuable as a
dependency and portability guarantee and should have its own compile gate.

### Proof-first release policy

No safety, compatibility, latency, allocation, panic, wire-parity, code-size,
or ergonomics claim is accepted because it sounds correct or follows from a
plausible design. Every claim must name a reproducible proof artifact: a test,
property, fuzz/Miri target, cross-codec fixture, benchmark, instruction count,
assembly assertion, size report, or compiled documentation example.

In particular, a public `_unchecked` method is not justified merely because it
removes a check in source or produces different assembly. It ships only when a
paired, production-representative benchmark demonstrates a repeatable,
practically meaningful latency improvement and instruction/branch evidence
identifies the removed work. If that proof fails, expose only the safe checked
unsuffixed method. A private unsafe core may still exist behind it to keep the
implementation sound and local without expanding the public interface.

The release must archive a machine-readable evidence manifest for the exact
release commit. Every normative claim in README, book, rustdoc, release notes,
and this specification must link to one or more manifest entries. A stale,
missing, skipped, or wrong-commit artifact is a failed claim, not neutral
evidence.

### Compatibility contract

Do not claim unqualified “SBE binary compatibility.” Publish a precise profile
that names:

- the FIX SBE specification revision/family and XML namespace(s);
- the pinned Real Logic SBE revision used as the reference;
- supported header/dimension layouts, byte orders, field shapes, schema
  evolution, and framing assumptions;
- explicitly unsupported or partially supported features;
- which features have byte-for-byte encode and cross-decode evidence in both
  directions.

The current 37 pinned official-reference crates, two Java fixtures, property
tests, hostile-input tests, golden generation, and full Car cross-decode are an
excellent foundation. They do not yet prove every supported feature in both
directions; several reference cases validate generated shape or header layout
rather than the complete payload.

## Required public API shape

### Chosen checked/unchecked convention

Every memory-extent seam that benefits from a trusted HFT path has two
predictable names:

- the unsuffixed method is safe, performs the minimum required validation once,
  returns `Result`, and then enters the same unchecked implementation;
- the `_unchecked` twin is `unsafe`, performs no buffer bounds or overflow
  validation, and documents the caller's exact memory-safety proof in a
  `# Safety` section.

This naming is final for 0.2. Do not retain `try_wrap*` aliases: they enlarge the
interface and make the unsuffixed name ambiguous. `unchecked` means that the
method omits memory-extent validation. It does not promise to omit template
dispatch or every protocol/domain check; any remaining validation and error
must be documented and benchmarked.

`unsafe` is the correct Rust spelling for a caller-supplied memory-safety proof.
It has no runtime cost. Verify the actual implementation in disassembly; the
keyword alone is not performance evidence.

```rust,ignore
// Safe lane: one minimum bounds/overflow validation boundary, then the same
// fixed-field implementation as the trusted lane.
let enc = CarEncoder::wrap_and_apply_header(&mut frame, 0)?;
let dec = CarDecoder::decode(&frame, 0)?;
let any = AnyMessage::decode(&frame, 0)?;

// Trusted HFT lane: no construction-time bounds/overflow check.
// SAFETY: encoded_length was used for this exact schema/message shape and the
// transport claim exposes at least that many writable bytes.
let enc = unsafe {
    CarEncoder::wrap_and_apply_header_unchecked(claim.buffer_mut(), 0)
};

// SAFETY: the upstream framing boundary guarantees that the header and the
// version-readable fixed extent for the selected template are in bounds.
let dec = unsafe { CarDecoder::decode_unchecked(frame, 0) }?;
let any = unsafe { AnyMessage::decode_unchecked(frame, 0) }?;
```

Required names for candidates that pass the proof-first public-method gate:

The table fixes spelling and semantics; it does not force a public unsafe twin
to exist. If its paired benchmark fails HFT-008, expose only the safe method in
the left column and keep any shared unsafe core private.

| Operation | Checked safe API | Zero-check trusted API |
|---|---|---|
| Encode frame and write header | `wrap_and_apply_header(...) -> Result<_, EncodeError>` | `unsafe wrap_and_apply_header_unchecked(...) -> _` |
| Encode body with header absent/reserved | `wrap(...) -> Result<_, EncodeError>` | `unsafe wrap_unchecked(...) -> _` |
| Decode a concrete framed message | `decode(...) -> Result<_, DecodeError>` | `unsafe decode_unchecked(...) -> Result<_, DecodeError>` |
| Decode a concrete body with external metadata | `wrap(...) -> Result<_, DecodeError>` | `unsafe wrap_unchecked(...) -> _` |
| Dispatch a known/unknown template | `AnyMessage::decode(...) -> Result<_, DecodeError>` | `unsafe AnyMessage::decode_unchecked(...) -> Result<_, DecodeError>` |
| Dispatch and validate a bounded frame | `AnyMessage::decode_frame(...) -> Result<_, DecodeError>` | `unsafe AnyMessage::decode_frame_unchecked(...) -> Result<_, DecodeError>` |

Decoder construction is named `decode`, not `wrap_and_apply_header`: decoding
reads and validates a header; it does not apply one.

### Shared implementation shape

For a pair that passes HFT-008, the checked encoder must make one cold check and
then call the public unsafe twin. If the twin does not pass, it calls the same
core privately. The bodies must not drift:

```rust,ignore
#[inline]
pub fn wrap_and_apply_header(
    buf: &mut [u8],
    message_offset: usize,
) -> Result<CarEncoder<'_>, EncodeError> {
    ensure_extent(
        buf.len(),
        message_offset,
        Self::HEADER_LENGTH + Self::BLOCK_LENGTH,
    )?;
    // SAFETY: ensure_extent proved the complete header + fixed write extent.
    Ok(unsafe { Self::wrap_and_apply_header_unchecked(buf, message_offset) })
}

/// # Safety
/// `message_offset + HEADER_LENGTH + BLOCK_LENGTH` must not overflow and must
/// be at most `buf.len()` for the lifetime of the returned encoder. Each later
/// unchecked dynamic write additionally requires its own complete extent.
#[inline]
pub unsafe fn wrap_and_apply_header_unchecked(
    buf: &mut [u8],
    message_offset: usize,
) -> CarEncoder<'_> {
    // One generated implementation of header write and state construction.
}
```

`AnyMessage::decode` must validate header readability, non-overflowing offsets,
schema policy, and the version-aware minimum fixed extent for the selected
template before entering the shared unchecked dispatch implementation.
`decode_unchecked` may still return `UnknownTemplateLength`, schema-policy, or
other non-memory protocol errors. It must not perform a buffer bounds check.
If literally calling `decode_unchecked` would reread the header and LLVM does
not eliminate that work, both public methods may call one private unsafe
`decode_parsed_unchecked` core instead. The public contract and paired names do
not change; optimized checked code must not parse the header twice.

The unsafe `AnyMessage::decode_unchecked` proof covers the header and the
version-readable fixed extent of whichever recognized template the header
selects. It does not silently make malformed dynamic groups or var-data valid.
Dynamic access remains checked unless the caller separately chooses an unsafe
dynamic-tail method. `decode_frame_unchecked` has the stronger obligation that
the complete declared frame, including every traversed dynamic extent, is
structurally present in `buf`.

### Where the twin pattern applies

Apply it in 0.2 to:

- root message encoder `wrap` and `wrap_and_apply_header`;
- concrete decoder `decode` and external-metadata `wrap`;
- `AnyMessage::decode` and `AnyMessage::decode_frame`;
- destination-capacity-sensitive `AnyMessage::encode` if it remains a public
  hot path (`encode`/`unsafe encode_unchecked`);
- generated group bulk writes and dynamic var-data writes only when the
  unchecked version actually removes a bounds branch in optimized code;
- a trusted framed-stream cursor only if benchmarks justify it. Use a separate
  `TrustedFrameCursor` or zero-sized state, constructed by one unsafe function,
  so `Iterator::next` does not branch on a runtime mode flag.

Do not expose twins mechanically everywhere:

- make raw byte helpers private `unsafe fn`, with no public raw-helper pair;
- make group, nested-group, and entry invariant constructors private. Parent
  stages call private unsafe constructors after establishing their invariants;
- replace the current safe `wrap_trusted` group method with a private unsafe
  constructor;
- do not add `_unchecked` to metadata access, exact-length arithmetic, or pure
  type-state transitions that perform no buffer access;
- keep UTF-8 methods such as `*_as_str_unchecked` separately unsafe: their
  proof is byte validity, not buffer capacity, and their `# Safety` docs must
  say so explicitly;
- do not generate an unchecked twin that merely removes an explicit check but
  leaves an equivalent slice-index panic/check in the implementation. It must
  satisfy the proof-first public-method gate or be deleted.

Rules:

- A safe constructor establishes the invariant at one validation boundary.
  Fixed getters/setters may then use internal unchecked loads/stores without
  repeated checks.
- Every public unsafe constructor has a complete `# Safety` section containing
  only UB-critical obligations not already guaranteed by `&[u8]`/`&mut [u8]`:
  sufficient non-overflowing extent, plus structural dynamic-tail validity only
  when later traversal is also unchecked. Wrong schema/template/version is a
  protocol contract, not a memory-safety precondition unless the
  implementation genuinely relies on it for extent safety.
- Internal raw byte helpers are private `unsafe fn`; they are never safe public
  utilities.
- Use checked arithmetic at safe boundaries. An offset addition must not occur
  before overflow/bounds validation.
- `debug_assert!` is acceptable inside trusted constructors; the optimized
  trusted lane must contain no bounds branch.
- Decoder checked construction must validate enough declared fixed bytes for
  every field readable at the acting version without requiring the current
  full `BLOCK_LENGTH` from an older message. Fields introduced later must never
  touch bytes when absent. Encoder checked construction requires the full
  current fixed block that its setters can write.
- Checked dynamic writes validate capacity before mutation. Unsafe dynamic
  twins require the complete prefix + payload extent and any wire-length
  representability precondition that the implementation relies on.
- Safe and unsafe twins must produce byte-identical output or identical decoded
  values for every input satisfying the unsafe method's contract.

The type system cannot prove the length of a runtime transport claim merely
because the application called an encoded-length function. A safe wrapper can
check the claim once; a literally zero-check wrapper must be unsafe unless a
separate proof-carrying buffer type establishes the invariant. Do not hide this
proof obligation behind a safe method.

Defer a proof-carrying safe third entry style until after the two-lane 0.2
interface is measured. If added later, a private, non-fabricable
`VerifiedFrame`/`VerifiedClaim` or exact encode-plan token must pay or inherit
validation once, own/borrow the buffer so it cannot outlive, alias, or be
invalidated independently, and optimize to the same code as the unsafe lane.

## Findings summary

| Area | Verdict | Evidence |
|---|---|---|
| Architecture | Strong | Build-time generator is separated from dependency-free generated hot code; borrowed flyweights, explicit endian loads, exact sizing, and ordered stages are the right seams. |
| Type-state | Keep | Zero-sized/static state, no runtime dispatch; fresh parity benchmarks show no tax. Named stages improve errors and rustdoc. |
| Hot-path throughput | Strong but not complete | Fresh isolated no-LTO and LTO runs passed all 10 maintained ratios. Ratios are Ergon mean ns/op divided by sbe-tool mean ns/op: 0.4866x-0.9538x without LTO and 0.5786x-0.9864x with LTO. This proves the maintained central-tendency cases on this host, not production tail latency. |
| Memory safety | Release blocked | Safe public raw helpers and safe zero-check constructors can reach unchecked out-of-bounds pointer reads/writes. |
| Wire correctness | Release blocked | Optional group null initialization and big-endian nullification are wrong; group var-data maximums and some exact-length/count cases are inconsistent. |
| Checked-path behaviour | Release blocked | Some checked/domain conversion paths panic or manufacture lossy/default values. |
| Compatibility evidence | Strong partial | Broad pinned reference corpus and real cross-decode tests, but the public claim is not tied to an exact supported profile/conformance matrix. |
| Ease of use | Good foundation | Type-state/closures/exact sizing are good; constructor naming and checked/trusted semantics need one coherent redesign. |
| Documentation | Broad, not release-grade yet | Good book structure and many compiled includes, but safety/compatibility claims conflict with code and 17 `rust,ignore` fences are outside compilation. |

## Release-blocking tickets

### HFT-001 — Restore soundness and make trust explicit

Priority: P0

Dependencies: none

Problem:

- `sbe/src/codegen/mod.rs:1017` and `:1027` emit safe public
  `read_bytes_unchecked`/`write_bytes_unchecked` functions around raw pointer
  operations. Safe callers can immediately cause out-of-bounds UB.
- `sbe/src/codegen/message_decoder.rs:294` emits safe `wrap` without validating
  offsets or the base fixed extent. `try_wrap_and_apply_header` validates that
  the declared block fits the slice, but accepts an undersized declared fixed
  block. A valid Car header declaring block length zero is accepted; the safe
  `serial_number()` accessor then reads beyond the eight-byte frame.
- `sbe/src/codegen/message_encoder.rs:1234` and `:1253` emit safe zero-check
  constructors. Their safe setters use internal unchecked writes. A zero-length
  buffer passed to `CarEncoder::wrap` can therefore produce UB from safe Rust.
- `sbe/src/codegen/runtime.rs:1875` emits `AnyMessage::decode`, which checks that
  the header is readable but constructs a concrete decoder without validating
  that template's version-readable fixed extent. A later safe fixed-field
  accessor can therefore reach the same unchecked out-of-bounds read.
- Public group/entry invariant-bearing `wrap` methods need the same audit.

Implementation:

- Make unsuffixed `wrap`, `wrap_and_apply_header`, `decode`, `decode_frame`, and
  any approved dynamic twins the safe checked `Result` methods. Remove
  `try_wrap*` aliases. Implement and measure the explicit unsafe `_unchecked`
  candidates in an internal benchmark surface, but expose each candidate
  publicly only in HFT-008 after it passes that ticket's gate; otherwise keep
  the zero-check core private.
- Implement checked encoder twins as one cold validation followed by a
  documented internal call to their public unsafe twin. Give decoder/dispatch
  pairs one shared unsafe core and prove optimized checked dispatch does not
  parse a header twice.
- Implement and measure `AnyMessage::decode_unchecked` and
  `decode_frame_unchecked` candidates with the distinct fixed-extent and
  complete-frame safety obligations defined above; publish them only if each
  passes HFT-008.
- Generate a version-aware decoder minimum readable fixed extent and require
  the full current fixed extent for encoder checked construction.
- Make raw byte helpers private and unsafe; keep unsafe blocks tiny and locally
  documented.
- Replace unchecked safe-boundary addition with `checked_add` or
  subtraction-first comparisons.
- Audit all generated safe constructors, group/entry constructors, cursor
  advances, byte/string views, and dispatch/frame helpers against the same
  invariant.

Acceptance criteria:

- [ ] `#![deny(unsafe_op_in_unsafe_fn)]` and undocumented-unsafe lint policy pass
      for a strict generated consumer fixture.
- [ ] No public safe function has a caller-owned memory-safety precondition.
- [ ] Property/fuzz tests walk complete legal state sequences for arbitrary
      bytes, including nested/ragged tails and every truncation boundary;
      `catch_unwind` observes no panic and Miri reports no UB.
- [ ] Regression tests cover zero bytes, header-only bytes, declared block
      lengths 0..minimum, `usize::MAX` offsets, every truncation boundary, and
      old acting versions, plus schema/template/header-layout rejection.
- [ ] A Miri regression reports no UB for the former all-safe Car decoder and
      encoder cases, which now return an error or require an unsafe call.
- [ ] Generated-source tests prove each checked encoder calls the same unsafe
      core (public twin only when HFT-008 passes), and each checked
      decoder/dispatch method enters the same private unsafe core after
      validation rather than maintaining a second codec body.
- [ ] Safe/unsafe differential properties prove byte/value equivalence for all
      inputs satisfying the unsafe preconditions.
- [ ] No unchecked candidate is part of the normal public generated interface
      before HFT-008 records a passing keep decision; failed candidates remain
      private and cannot leak through the prelude or generated rustdoc.

### HFT-002 — Correct null sentinels for groups, widths, signs, and endianness

Priority: P0

Dependencies: HFT-001 for final hostile-path verification

Problem:

- `sbe/src/codegen/group_encoder.rs:95-98` always produces an eight-byte `u64`
  null representation and copies it into a field-width destination. Optional
  1/2/4-byte group fields panic on `copy_from_slice`.
- `sbe/src/codegen/nullification.rs:39-42` selects the first `size` bytes of a
  `u64` representation. That selects low-order bytes for little endian, but
  high-order bytes (commonly zero) for small big-endian fields.
- The representation must also respect signed and floating null bit patterns,
  not only an IR `u64` convenience representation.

Implementation:

- Centralise primitive-to-wire-byte encoding for schema constants/nulls.
- Produce exactly the declared primitive width in the schema byte order.
- Reuse one implementation for message, group, nested group, bulk, and DTO
  paths.

Acceptance criteria:

- [ ] Matrix covers every primitive width, signed/unsigned, float/double bit
      sentinel, optional message field, optional group field, nested group,
      little endian, and big endian.
- [ ] Every result is byte-compared with pinned sbe-tool output.
- [ ] Encoding an omitted optional value never panics and writes the exact
      declared null bytes.

### HFT-003 — Make all fallible/materialisation paths genuinely fallible

Priority: P0

Dependencies: HFT-001

Problem:

- `sbe/src/codegen/domain_cluster.rs:443` and `:451` map group entries through
  `EntryDomain::from`; that `From` delegates to `try_from_decoder(...).expect`
  at `:628-632`.
- Configured concrete conversions generate `expect` in decoder/encoder methods
  at `sbe/src/codegen/converter_impls.rs:64-81` and `:144-160`.
- `DomainVarData::LossyStrings` converts invalid UTF-8 to empty text, so a
  checked decode accepts corrupt bytes and re-encodes different bytes.
- Decimal conversion can overflow on exponent `i8::MIN` and saturates positive
  exponent arithmetic; timestamp conversion casts negative nanoseconds to
  `u64`.

Implementation:

- Use `TryFrom`/`try_from_decoder` throughout checked domain materialisation.
- Generate fallible concrete conversion methods (for example `try_price`) and
  reserve infallible methods only for conversions whose trait contract is
  actually infallible.
- Replace `LossyStrings` with raw bytes plus a fallible UTF-8 view, or a strict
  string mode returning an explicit error. Do not manufacture empty/default
  data.
- Use checked decimal/timestamp arithmetic and range conversion.
- If panicking convenience APIs remain, make the panic explicit in their name
  and `# Panics` docs and never call them from a checked path.

Acceptance criteria:

- [ ] Hostile/truncated nested DTO materialisation returns a typed error and
      never panics.
- [ ] Invalid UTF-8 round-trips losslessly as bytes or returns `InvalidUtf8`.
- [ ] Decimal exponent and mantissa boundary matrix has no panic/saturation.
- [ ] Pre-epoch and maximum timestamps either encode exactly or return a range
      error.
- [ ] `catch_unwind` hostile corpus and Miri cover all generated checked entry
      points.

### HFT-004 — Make exact sizing, counts, and schema maximums one invariant

Priority: P0

Dependencies: HFT-002

Problem:

- Domain message length uses a tight field span at
  `sbe/src/codegen/domain_cluster.rs:811-818`, while the encoder uses the
  declared padded block length. “Compute exact length, allocate exactly, then
  encode” can consequently fail.
- Domain group counts use `len() as count_ty` at `:467-477`, allowing
  truncation before the encoder detects a mismatch/partial write.
- Group entry var-data at `sbe/src/codegen/group_encoder.rs:590-611` validates
  buffer and wire integer width but not schema `maxLength`.
- `MAX_ENCODED_LENGTH` counts one group entry rather than the maximum count,
  omits nested tails, treats unbounded var-data as zero, and is capped while
  documented as an upper bound.
- `start_entry` advances without first checking fixed entry capacity.

Implementation:

- Define one resolved effective block length used by every encoder, DTO,
  length builder, and constant.
- Validate group count representability before mutation.
- Apply the same schema max-length rule to message and every group depth.
- Either compute a true finite maximum or emit `Option<usize>`/no constant for
  an unbounded schema. Never call a capped diagnostic value an upper bound.
- Make exact-length calculation transactional: if it succeeds for a DTO/plan,
  encoding the same value into that exact-sized buffer must not fail for a
  size/count reason or partially mutate before a predictable validation error.

Acceptance criteria:

- [ ] Property: for every generated schema/value, successful
      `encoded_length_with_header()` equals bytes written.
- [ ] Property includes declared padding, count-type maximum boundaries,
      nested groups, ragged entries, empty groups, and every var-data maximum.
- [ ] One-byte-short always errors before out-of-range mutation; exact length
      always succeeds.
- [ ] sbe-tool byte parity covers padded DTO, group max-length, and nested
      count-boundary fixtures.

### HFT-005 — Restore every release gate to green

Priority: P0

Dependencies: HFT-001 through HFT-004 and HFT-006 through HFT-010

Current failures observed during this review:

- `cargo fmt --all --check` fails at `sbe/src/xml/mod.rs:51` under the pinned
  1.95 formatter.
- `cargo deny check` fails for `RUSTSEC-2026-0204` in
  `crossbeam-epoch 0.9.18` through Criterion and for unmaintained benchmark-only
  dependencies through `iai-callgrind`.
- Generated documentation sample crates compile with many ordinary Rust
  warnings; the harness globally allows several warnings and does not enforce
  a clean consumer experience.

Implementation/acceptance criteria:

- [ ] Format check passes with the pinned toolchain.
- [ ] `cargo deny check` and `cargo audit` pass without blanket advisory
      exceptions; upgrade/remove/replace the offending dev dependencies.
- [ ] All generated representative crates compile warning-free under an
      explicit supported lint set.
- [ ] Full `just test-all`, reference regeneration, coverage/mutation gates,
      benchmark compile, rustdoc `-D warnings`, book validation, and publish dry
      run pass from a clean checkout.
- [ ] The checked/unchecked benchmark manifest is complete, all paired
      benchmarks and assembly/instruction assertions run on fresh artifacts,
      and benchmark docs are generated or validated from that exact run.
- [ ] A release-evidence verifier confirms every manifest artifact exists,
      passed, names the release commit/toolchain/environment, records its
      reproducible command and content hash, and is referenced by each
      normative documentation claim.
- [ ] Release automation refuses to publish if any gate or required external
      tool is missing.

## 0.2 proof and quality tickets

### HFT-006 — Lock in the type-state architecture with non-runtime budgets

Priority: P1

Dependencies: HFT-001

- Keep named wire-order stages, statically monomorphized for the zero-sized
  orthogonal header state.
- Add compile-fail coverage for wrong group order, reused consumed stages,
  escaped child borrows, header-absent completion, and fabricated private
  states.
- Assert `size_of` equality between representative stages and a plain
  slice/cursor carrier; assert marker size zero and intended `Send`/`Sync`,
  variance, and drop behaviour.
- Set concrete compile-time, generated-source, release-text, and
  monomorphization-count budgets for a pinned toolchain and
  small/large/multi-schema corpus, with a repeatability/noise policy.
- Keep a benchmark comparing direct single-struct code and named stages, but
  treat a tie as expected. Fail only on a repeatable, controlled regression.
- Document the complexity of convenience random-access tail accessors. The
  consuming staged traversal is the canonical linear HFT path; accessors that
  rescan preceding tails must say `O(previous tail bytes/entries)` and should
  not be presented as the fastest path.

### HFT-007 — Publish and enforce the SBE conformance profile

Priority: P1

Dependencies: HFT-001 through HFT-004

- Add `docs/SBE_COMPATIBILITY.md` with the exact profile and an explicit
  supported/partial/unsupported table.
- Pin and display the Real Logic revision in the generated provenance and
  release notes.
- For each claimed feature, check both directions: Ergon bytes decoded by the
  reference and reference bytes decoded by Ergon. Self-round-trip is supporting
  evidence only.
- Integrate the official FIX SBE Conformance suite as a release/CI lane.
- Add schema evolution matrices for old decoder/new bytes and new decoder/old
  bytes, including appended fixed fields, groups/data, unknown enum values,
  and custom headers.
- Separate “XML accepted by the parser” from “wire behaviour proven against a
  reference.”

Acceptance criteria:

- [ ] Every README/rustdoc/book compatibility claim links to the profile.
- [ ] No feature is called compatible without a bidirectional byte/cross-decode
      fixture or an explicit qualification.
- [ ] The pinned 37-crate manifest cannot gain a header-only row while claiming
      full payload parity.
- [ ] The pinned official FIX conformance suite passes for the exact declared
      profile; archive its plan/results and pin the normative FIX revision/XSD
      plus Real Logic commit/hash.
- [ ] Unknown enum values never use transmute/invalid discriminants and either
      preserve the raw value or return a typed application-policy error.

### HFT-008 — Add HFT-grade latency and instruction evidence

Priority: P0

Dependencies: HFT-001, HFT-004

The current Criterion comparisons are useful and the fresh no-LTO results are
excellent. Batched Criterion samples and batch-duration division do not prove
per-message p99/p99.9 latency.

- Keep the existing fair sbe-tool ratio gate and correctness preflight.
- Add a mandatory paired benchmark for every checked/unchecked candidate using
  the internal benchmark surface from HFT-001. Public exposure is the output of
  this ticket, not an assumption. Both sides must use the same buffer, offset,
  header/template, field writes or reads, payload shape, `black_box` placement,
  compiler profile, and batching. The only permitted difference is the method
  selected and the safety comment at the unsafe call site.
- Run each constructor pair in two caller-knowledge modes: an exact array or
  slice whose length LLVM can see, and an opaque runtime `&[u8]`/`&mut [u8]`
  matching an Aeron/transport claim. Prevent constant propagation without
  adding unequal work. If the compiler already removes the checked method's
  validation in the real caller shape, that is evidence to keep only the safe
  method, not a reason to preserve a redundant unsafe twin.
- At minimum, benchmark this matrix when the corresponding twin ships:

  | Pair | Fixed message | Dynamic message | Required variants |
  |---|---:|---:|---|
  | `wrap` / `wrap_unchecked` | yes | yes | exact and oversized buffer; offset 0 and non-zero |
  | `wrap_and_apply_header` / `wrap_and_apply_header_unchecked` | yes | yes | constructor-only and full encode |
  | concrete `decode` / `decode_unchecked` | yes | yes | constructor-only and fixed-field reads |
  | concrete decoder `wrap` / `wrap_unchecked` | yes | yes | external acting metadata |
  | `AnyMessage::decode` / `decode_unchecked` | yes | yes | single known template and mixed-template dispatch |
  | `AnyMessage::decode_frame` / `decode_frame_unchecked` | yes | yes | complete traversal and unknown-template frame |
  | `AnyMessage::encode` / `encode_unchecked` | if emitted | if emitted | copy-only and end-to-end dispatch |
  | checked/unchecked var-data write | n/a | yes | empty, median, declared maximum |
  | checked/unchecked group bulk write | n/a | yes | empty, one, typical batch, count maximum edge |
  | `FrameCursor` / trusted cursor | yes | yes | one frame and a mixed multi-frame stream |

- Report each side's absolute ns/op and instructions, plus
  `checked - unchecked` for nanoseconds, instructions, conditional branches,
  and branch mispredictions. Also publish the checked/unchecked ratio with its
  numerator and denominator stated. A ratio alone can hide noise at sub-ns
  scale.
- Pre-register the keep/remove rule before measuring. For 0.2, keep a public
  `_unchecked` twin only when all of these hold on at least one declared
  production caller shape:

  1. optimized instruction evidence shows fewer retired instructions or
     conditional branches and identifies the exact removed validation;
  2. across at least ten fresh isolated process runs, at least eight runs favour
     `_unchecked` and the aggregate 95% confidence interval for the latency
     ratio lies wholly below `1.0`;
  3. the median latency improvement is at least 2%, after subtracting measured
     harness/timer overhead, in either the constructor-only or end-to-end case;
  4. no paired end-to-end p99 or p99.9 result regresses by more than the same 2%
     practical threshold.

  If any condition fails, remove the public `_unchecked` twin and its docs,
  examples, benchmark row, and migration surface; retain only safe `xxx`.
  Record the negative result so the method is not repeatedly proposed without
  new compiler, hardware, or workload evidence.
- Inspect optimized assembly for each constructor-only pair. The unchecked
  side must contain no slice-length, capacity, or overflow branch. The checked
  side must contain exactly the documented validation set, use an out-of-line
  cold error path, enter the same codec core, and avoid duplicate header reads
  after inlining.
- Run paired benchmarks under both the release profile used by normal users
  and the documented HFT profile (`target-cpu`, LTO, codegen units). Pin the
  generated schema and archive source/assembly so a generator change cannot
  silently change the work being compared.
- Add a per-observation latency harness for encode, decode, and end-to-end
  framed flows; publish p50, p90, p99, p99.9, max, throughput, sample count, and
  raw histogram. Record timer source/resolution, measurement overhead, and a
  repeated-run acceptance policy.
- Run on a dedicated/pinned core with documented CPU model, governor/frequency,
  thermal state, affinity, background-load controls, compiler, target CPU, LTO,
  codegen units, and schema/payload.
- Add Linux `iai-callgrind` instruction-count gates rather than compile-only
  coverage.
- Compare current release with the previous release tag automatically, not only
  with sbe-tool. Fail statistically meaningful regressions against both the
  project baseline and the reference ceiling.
- Add a freshness marker/run ID to benchmark artifacts as defence in depth;
  the current Criterion path is correct, but a removed/renamed arm must not be
  satisfied by an old estimate left in the directory.

Acceptance criteria:

- [ ] Every candidate appears in a machine-readable benchmark manifest with a
      keep/remove decision linked to a Criterion/per-observation benchmark and
      an instruction/assembly assertion; CI fails if a generated public twin
      lacks a passing decision.
- [ ] Trusted lane has zero bounds/overflow checks in optimized disassembly.
- [ ] Safe lane shows exactly the documented constructor validation set and no
      per-fixed-field bounds check.
- [ ] Checked and unchecked lanes produce identical bytes/values in benchmark
      correctness preflight before timing begins.
- [ ] Each public `_unchecked` method passes all four pre-registered proof
      conditions on at least one declared production workload. If it does not,
      the release exposes only safe `xxx`; a shared unsafe implementation core
      remains private.
- [ ] Allocation-count gates cover every representative encode/decode API.
- [ ] Published ratios state numerator, denominator, metric, confidence
      interval, repetitions, hardware, raw artifacts, and reproducible command.

### HFT-009 — Add an intentional lean generated-code profile

Priority: P1

Dependencies: HFT-006

The default Car golden is roughly thousands of lines/hundreds of KiB because
Display/Debug, metadata, dispatch, DTOs, conversions, and runtime support share
one generated module. Existing boolean knobs are useful but difficult to
reason about as a product contract.

Introduce a small profile enum/preset, for example:

```rust,ignore
GenerationConfig::new("feed").profile(GenerationProfile::HftLean)
GenerationConfig::new("feed").profile(GenerationProfile::Full)
```

`HftLean` should generate only the byte codec, typed stages, required errors,
and exact sizing. `Full` can add Display/Debug, metadata, dispatch, DTOs, and
conversion conveniences. Individual overrides may remain, but invalid or
surprising combinations should be rejected.

- Compile a core-only/no-allocation generated consumer for `HftLean`.
- Test additive feature configurations: `--no-default-features`, `alloc`,
  positive `std`, and all-features. Do not use a negative `no_std` feature.
- Prefer one external shared runtime for multi-schema generation to avoid
  duplicated support code.
- Record source/compile/text-size budgets per profile.
- Decide whether public IR/token types and codegen hooks are a stable product
  surface. If not, move them behind explicitly experimental features before
  1.0; their current internal representation is a large SemVer commitment.

### HFT-010 — Make the documentation executable and contract-first

Priority: P1

Dependencies: HFT-001 through HFT-004 and HFT-006 through HFT-009

Strengths to retain:

- the book has a good task/concept/design-note structure;
- many examples include real sample source and compile in CI;
- compatibility, allocation, fuzz, Miri, mutation, and benchmark methodology
  are documented more deeply than in most 0.x libraries;
- type-state and migration explanations are clear.

Required corrections:

- Replace unqualified “binary-compatible” language with the conformance
  profile link.
- Rewrite trust-boundary/API-freeze pages: current text says malformed
  unchecked access is garbage “not UB,” which is false for out-of-bounds raw
  pointer access.
- Make the checked/unchecked convention a single documentation contract across
  every surface: the unsuffixed method is safe and checked; `_unchecked` is
  unsafe and omits buffer-extent validation; the safe method reaches the same
  unchecked codec core after proving its invariant. Never call the unchecked
  path “safe because HFT developers are advanced.”
- Update every affected call site from `try_wrap*` to the unsuffixed checked
  name and add a local `// SAFETY:` proof to every `_unchecked` example. For an
  exact-sized transport claim, the proof must connect the exact encoded-length
  calculation, the same message shape, the requested claim length, the
  returned slice length, and the message offset.
- Document `AnyMessage::decode`/`decode_unchecked` and
  `decode_frame`/`decode_frame_unchecked` separately. State that unchecked
  dispatch may retain template/schema policy errors, that plain `decode` only
  establishes the recognized template's version-readable fixed extent, and
  that dynamic tails remain checked unless separately opted out.
- Give every public unsafe function a rendered `# Safety` section and at least
  one positive example. Give every safe twin its exact checks, error cases,
  panic guarantee, allocation behaviour, and expected branch/instruction
  difference from its unsafe twin.
- Correct `with_unchecked_companions`: its rustdoc promises decoder fixed-field
  companion methods that are not generated, while ordinary fixed getters are
  already internally unchecked. Redefine it only for optional high-cardinality
  dynamic candidates. Root constructor/decode candidates are always measured,
  but public twins are emitted only when they pass the proof-first gate.
- Rename/explain decoder construction: a decoder does not “apply” a header.
- Correct `verify` docs and examples. The implementation walks dynamic tails;
  it is not merely a cheap header-only check, and `verify` is associated rather
  than `car.verify()` in the generated API.
- Fix stale ignored examples: multi-schema iteration, set serde setters, exact
  Car stage order/length, Aeron `try_claim`, and cluster chained decode.
- The crate README needs one complete generated encode/decode example plus
  direct links to safety, compatibility, schema evolution, feature/MSRV, and
  benchmark contracts.

Documentation migration inventory:

- top-level/package/release policy: `README.md`, `sbe/README.md`,
  `cluster/README.md`, `samples/README.md`, `SECURITY.md`, `CHANGELOG.md`, and
  any safety/performance statements in `AI-ASSISTANCE.md`;
- book concepts and design: both trust-boundary chapters, `api-freeze.md`,
  `generated-code.md`, `generation-config.md`, the feature matrix, and the
  core-concepts/design-note indexes;
- book task pages and examples: encode/decode, method chaining, migration from
  sbe-tool, bulk arrays, flyweight-vs-struct, recipes, Aeron `try_claim`, domain
  DTOs, chained decoding, and every included source file they reference;
- performance material: `sbe/BENCHMARKS.md`, `sbe/benchmarks/README.md`, both
  benchmark book chapters, benchmark module docs, and command/output labels;
- generated rustdoc and generator/config rustdoc: constructor/decode methods,
  `AnyMessage`, `FrameCursor`, `with_unchecked_companions`, generated module
  prelude, error types, and unsafe dynamic/UTF-8 companions;
- samples, tests, doctests, golden files, and compile fixtures whose code or
  comments teach the old names or old safety model.

This inventory is a minimum, not an allowlist. Search every tracked Markdown
file and every public rustdoc/comment after regeneration. Unrelated prose does
not need artificial checked/unchecked text, but no old name or contradictory
safety claim may remain outside an explicitly labelled 0.1 migration example.

CI/docs acceptance criteria:

- [ ] Replace the no-op `book_fences_no_ignored` test with a real inventory.
      Every `rust,ignore` fence must compile in a dedicated fixture or be on a
      small allowlist with owner, reason, and executable external test.
- [ ] Prefer `{{#include}}` from tested examples over duplicated snippets.
- [ ] Deny broken intra-doc links and missing safety/panic docs.
- [ ] Add a strict generated consumer fixture for public documentation/lints;
      `RUSTDOCFLAGS=-D warnings` alone does not measure missing docs.
- [ ] Add `compile_fail` doctests for illegal type-state sequences.
- [ ] Run a link checker and validate versioned book/docs.rs links.
- [ ] Generated code examples compile without blanket warning suppression.
- [ ] A repository-wide stale-interface check rejects `try_wrap`,
      `try_wrap_and_apply_header`, decoder “apply header” wording, safe
      zero-check constructor claims, and unchecked “garbage but not UB” claims,
      except in an allowlisted migration/changelog context.
- [ ] A generated-interface inventory checks that every public `unsafe fn` is
      mentioned in generated rustdoc, the safety chapter, and the paired
      benchmark manifest, and that every unsuffixed checked twin links to it.
- [ ] All README/book/rustdoc examples use the same names, return handling, and
      safety proofs; CI compiles both checked and unchecked examples.
- [ ] Benchmark documentation contains the complete checked/unchecked matrix,
      absolute results, deltas, ratios, assembly/instruction evidence, and
      reproducible commands for the release commit.

### HFT-011 — Release as 0.2.0 with a migration guide

Priority: P1

Dependencies: all P0 tickets, HFT-006 through HFT-010

- Publish a 0.1-to-0.2 migration table that explicitly maps
  `try_wrap_and_apply_header` to checked `wrap_and_apply_header`, old safe
  zero-check `wrap_and_apply_header` to unsafe
  `wrap_and_apply_header_unchecked`, concrete decoder construction to
  `decode`/`decode_unchecked`, and `AnyMessage`/frame variants, as well as
  strict strings, fallible conversions, and sizing changes.
- Include explicit examples for callers that compute exact length and want no
  bounds checks: the migration is a one-line unsafe constructor plus a local
  `// SAFETY:` proof tying the calculation to the claim, not repeated per-field
  validation.
- Include a “do not mechanically choose unchecked” decision table: use checked
  at network/file/external seams, unchecked only after a capacity/framing proof,
  and keep dynamic traversal checked unless its complete structure is proven.
- Run `cargo semver-checks` against the latest published crate and review every
  intentional break.
- Inspect the packaged crate contents and make all release evidence refer to
  the exact tag/commit, toolchain, SBE reference revision, and raw benchmark
  run.
- Do not tag/publish while the worktree is dirty or a release gate is using a
  previous run's artifact.

## Recommended ticket order

```text
HFT-001 sound trust boundary
   |
   +--> HFT-002 null/wire fixes --> HFT-004 exact sizing --> HFT-008 unchecked proof gate
   |
   +--> HFT-003 fallible paths
   |
   +--> HFT-006 typestate budgets --> HFT-009 lean profile

HFT-001..004 --> HFT-007 conformance profile
HFT-001..009 --> HFT-010 executable docs
all above    --> HFT-005 clean release gate --> HFT-011 release 0.2.0
```

## Verification performed during this review

Passed:

- `cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings`
- `cargo test -p ergo-sbe --all-features -- --test-threads=1`
  (all unit, integration, parity, property, hostile-input, allocation, and
  doctest suites passed)
- `cargo check --manifest-path sbe/fuzz/Cargo.toml --bins`
- native and nightly-Miri `sbe/miri-fixtures` tests
- 37-reference/two-fixture pinned sbe-tool regeneration check
- generated golden regeneration check
- strict rustdoc build, book validation/build, benchmark-crate tests, and
  `cargo publish -p ergo-sbe --dry-run --allow-dirty`
- fresh isolated no-LTO and LTO parity benchmarks: all 10 maintained ratios
  passed the 1.00 + 0.005 gate in each profile; observed ratios were
  0.4866x-0.9538x (no LTO) and 0.5786x-0.9864x (LTO) sbe-tool

Failed:

- format check (one pinned-rustfmt difference)
- dependency policy/audit (one current transitive vulnerability plus
  unmaintained benchmark-only dependency chain)

Important limitation: the existing Miri and hostile-input tests exercise happy
paths or call accessors after `verify`; they do not exercise the currently
unsafe all-safe constructor/accessor reproductions. A green existing suite
therefore does not invalidate HFT-001.

## Definition of “best-in-class” for 1.0

Do not call the project best-in-class based on benchmark wins alone. The 1.0
bar is all of the following, continuously enforced:

1. Soundness: no safe API can cause UB, and unsafe proof obligations are small,
   explicit, tested, and documented.
2. Wire fidelity: an exact SBE profile passes official conformance and
   bidirectional reference vectors.
3. Predictability: zero allocations/locks/I/O on hot paths, exact sizing, no
   hidden panics, and measured instruction/tail-latency budgets.
4. Ergonomics: named type-state stages make order errors impossible without
   obscuring fixed-field access; checked and trusted lanes are unmistakable.
5. Smallness: a lean codec profile has controlled generated source, compile,
   and binary size.
6. Documentation: every claim is linked to an executable test or reproducible
   artifact; every unsafe/panic/error/complexity contract is visible where the
   user makes the decision.

## Primary-source design basis

The detailed, link-verified research note is
[`research/sbe-hft-architecture-primary-sources.md`](research/sbe-hft-architecture-primary-sources.md).
It covers FIX SBE structure/evolution/conformance, Real Logic reference design,
Rust layout/unsafe/typestate rules, Cargo SemVer/no-std guidance, Criterion
limits, and rustdoc expectations.

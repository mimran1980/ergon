# SBE/HFT architecture: primary-source research notes

Date: 2026-08-02

Scope: evidence and release criteria for a Rust SBE codec that claims wire
compatibility, safety, ease of use, and HFT-grade performance. This note does
not assess the current implementation line by line; it supplies the standards
and engineering bar against which that assessment should be made.

## Bottom line

Keeping typestate is the right default if representative benchmarks show no
regression. Rust's official Embedded Book uses typestate as its worked example
of a zero-cost abstraction: zero-sized marker states have no runtime storage,
and the generated code can reduce to the same machine operations as direct
access. Rust monomorphizes generics, so generic state parameters do not require
runtime dispatch. The tradeoff is compile time and binary size from additional
monomorphizations, not per-message state checks. This supports the benchmark
result, while making illegal cursor/state transitions unrepresentable.
([Embedded Rust Book: typestate and zero-cost abstractions](https://doc.rust-lang.org/stable/embedded-book/static-guarantees/zero-cost-abstractions.html),
[Rust Book: monomorphization](https://doc.rust-lang.org/book/ch10-01-syntax.html#performance-of-code-using-generics),
[rustc development guide: monomorphization costs](https://rustc-dev-guide.rust-lang.org/backend/monomorph.html))

That conclusion is conditional: marker fields must actually be zero-sized,
transitions must be statically dispatched and inlineable, and the public API
must not add allocation, dynamic dispatch, locks, reference counting, or a
runtime state enum around the typestate layer. `PhantomData` itself occupies no
space but affects variance, drop checking, and auto traits, so its exact form
must be chosen deliberately and the intended `Send`/`Sync` behavior tested.
([`PhantomData` documentation](https://doc.rust-lang.org/core/marker/struct.PhantomData.html),
[Rustonomicon `PhantomData` table](https://doc.rust-lang.org/nomicon/phantom-data.html))

The larger release risk is not typestate. It is making an unqualified “SBE
binary compatible” claim without naming the SBE version/profile and proving
interoperability, plus ensuring every safe entry point handles malformed or
truncated input without undefined behavior or an undocumented panic.

## What “SBE binary compatible” must mean

SBE is a presentation-layer wire format designed for high-performance trading,
low-latency encode/decode, positional access, native binary datatypes, and a
preference for fixed positions and lengths. It is schema-driven and not
self-describing; both sides need the schema out of band.
([FIX SBE objectives and design principles](https://github.com/FIXTradingCommunity/fix-simple-binary-encoding/blob/master/v1-0-STANDARD/doc/01Introduction.md),
[FIX SBE field metadata and presence](https://github.com/FIXTradingCommunity/fix-simple-binary-encoding/blob/master/v1-0-STANDARD/doc/02FieldEncoding.md))

The compatibility claim must state all of the following:

- Supported FIX SBE specification family and revision, XML namespace(s), and
  schema features. FIX identifies SBE 1.0 as the final technical specification
  and the 2.0 line as release candidates; FIX also states that 2.0 RC1 has
  minor wire-format changes and is not interoperable with 1.0. A claim such as
  “SBE-compatible” without a version/profile is therefore ambiguous.
  ([FIX specification repository: versions and lifecycle](https://github.com/FIXTradingCommunity/fix-simple-binary-encoding#versions))
- The accepted message-header and group-dimension composites, primitive widths,
  byte order, explicit offsets/padding, and framing convention. SBE messages
  have no delimiter; FIX recommends external framing for streaming transports
  or persisted streams because an internal walk depends on a correct start and
  non-malformed message. SOFH is separate from the SBE payload.
  ([FIX SBE 1.0 with November 2020 errata, message framing and structure](https://www.fixtrading.org/wp-content/uploads/download-manager-files/Simple_Binary_Encoding_V1.0_with_Errata_November_2020.pdf))
- Full field semantics: all integer and floating encodings; schema byte order;
  fixed arrays; constants that consume no wire bytes; optional/null values;
  min/max handling; enums; sets; composites and references; explicit field
  offsets; root padding; repeating groups including empty and nested groups;
  and variable data. The required body order is fixed fields, then groups, then
  variable data; nested groups are depth-first and must be walked sequentially.
  ([FIX field encoding](https://github.com/FIXTradingCommunity/fix-simple-binary-encoding/blob/master/v1-0-STANDARD/doc/02FieldEncoding.md),
  [FIX message structure](https://github.com/FIXTradingCommunity/fix-simple-binary-encoding/blob/master/v1-0-STANDARD/doc/03MessageStructure.md))
- Schema-evolution behavior in both directions. Compatible changes append
  fixed fields to the end of a block, do not move or change existing fields,
  append new groups/data in their permitted locations, and do not change the
  message-header encoding. Older decoders can see unknown enum values; the FIX
  specification leaves the application response to the application layer.
  ([FIX schema-extension constraints and compatibility strategy](https://github.com/FIXTradingCommunity/fix-simple-binary-encoding/blob/master/v1-0-STANDARD/doc/05SchemaExtensionMechanism.md))

The release gate should be byte-for-byte cross-implementation tests, not only
round trips through this library (a symmetric bug can pass a self round trip).
For every supported feature, decode vectors emitted by a pinned Real Logic SBE
version and have that implementation decode this library's output. Add the FIX
SBE Conformance suite: it exists specifically to verify interoperability, uses
the Real Logic Java implementation as injector/validator, and permits the
implementation under test to be written in any language.
([FIX SBE Conformance project](https://github.com/FIXTradingCommunity/fix-sbe-conformance),
[Real Logic/Aeron SBE reference implementation](https://github.com/aeron-io/simple-binary-encoding))

## Zero-copy flyweights: the safety/performance boundary

A zero-copy API should mean that decoders borrow `&[u8]`, encoders borrow
`&mut [u8]`, and variable/fixed byte fields are returned as subslices rather
than allocated objects. This design lets Rust lifetimes prevent a view from
outliving its packet and prevents encoder aliasing. The pinned Real Logic Rust
generator follows this broad model, emits `#![forbid(unsafe_code)]`, and builds
read/write buffers over borrowed slices, demonstrating that a reference codec
can use safe Rust throughout.
([Real Logic Rust generator `LibRsDef.java`, pinned commit](https://github.com/aeron-io/simple-binary-encoding/blob/44b04492d67aff2cd1fab19da77f365860b1e8c8/sbe-tool/src/main/java/uk/co/real_logic/sbe/generation/rust/LibRsDef.java))

Safe Rust is not automatically panic-free. The same reference generator uses
slice indexing and `expect`, which panic on short buffers. For a safe public API
aimed at untrusted network input, constructors/cursor advances should use
checked arithmetic and return a small, allocation-free error containing useful
context such as offset, needed bytes, and remaining bytes. `offset + length`
must be checked for integer overflow before checking it against the buffer.
The hot path can still check once per block/cursor transition rather than once
per byte load.
([slice `get`/indexing behavior](https://doc.rust-lang.org/stable/core/primitive.slice.html),
[Cargo profiles and overflow checks](https://doc.rust-lang.org/cargo/reference/profiles.html#overflow-checks))

Do not cast wire bytes directly to ordinary Rust structs. Default Rust layout
does not guarantee declaration order and may change between compilations; SBE
has schema-defined offsets, byte order, and normally no padding. A byte buffer
also need not satisfy integer alignment. Safe `from_le_bytes`/`from_be_bytes`
loads are the simple baseline. If measured evidence justifies unsafe unaligned
loads, isolate them in a tiny internal module after a checked bounds proof and
document each invariant.
([Rust Reference: type-layout guarantees](https://doc.rust-lang.org/reference/type-layout.html),
[`ptr::read_unaligned` safety](https://doc.rust-lang.org/std/ptr/fn.read_unaligned.html))

Unchecked indexing is not merely “less safe”: an out-of-bounds
`get_unchecked` is undefined behavior even if the result is never used. Raw
slice construction additionally requires one live allocation, valid alignment,
initialized contents, non-wrapping size, and a correctly bounded lifetime.
Therefore no safe method may let malformed input reach unchecked access.
([slice `get_unchecked` safety](https://doc.rust-lang.org/stable/core/primitive.slice.html#method.get_unchecked),
[`slice::from_raw_parts` safety](https://doc.rust-lang.org/std/slice/fn.from_raw_parts.html))

Decode enum/set storage as its integer primitive first. Do not transmute an
arbitrary wire integer into a Rust enum: producing an enum with an invalid
discriminant is immediate undefined behavior, and schema evolution explicitly
allows an older decoder to encounter unknown enum values. Preserve the raw
unknown value in the result/error API so the application can apply policy.
([Rust Reference: invalid enum values](https://doc.rust-lang.org/reference/behavior-considered-undefined.html#invalid-values),
[FIX schema evolution and unknown enum values](https://github.com/FIXTradingCommunity/fix-simple-binary-encoding/blob/master/v1-0-STANDARD/doc/05SchemaExtensionMechanism.md#compatibility-strategy))

Likewise, SBE character data is not universally UTF-8. Raw bytes should be the
lossless zero-copy representation; expose a checked text view only when the
schema's character encoding and the bytes justify it.
([FIX field encoding: character and string encodings](https://github.com/FIXTradingCommunity/fix-simple-binary-encoding/blob/master/v1-0-STANDARD/doc/02FieldEncoding.md),
[`str::from_utf8`-based guidance](https://doc.rust-lang.org/std/str/fn.from_raw_parts.html))

## Typestate API criteria

Typestate is especially appropriate for SBE because groups and variable data
form a streaming state machine. Real Logic's user guide warns that elements
must be processed in schema order and groups must be completed before later
groups or variable data. Encoding/decoding a child cursor should therefore
borrow or consume the parent so safe code cannot advance both concurrently.
([Real Logic SBE C++ user guide](https://github.com/aeron-io/simple-binary-encoding/wiki/Cpp-User-Guide))

The API is successful when:

- states and their constructors are private/sealed, so users cannot fabricate a
  state;
- transitions consume `self` or return a child tied to the parent's borrow;
- finishing a group/variable field returns the exact next legal state;
- fixed-field reads that SBE permits in any order are not needlessly forced
  through a verbose state chain;
- malformed input remains a `Result` concern—typestate proves API order, not
  wire validity;
- compile-fail tests prove illegal sequences do not compile, and examples show
  the shortest legal encode/decode flow;
- layout/code-size checks accompany timing benchmarks so a runtime tie does not
  conceal monomorphization bloat.

## `no_std`, allocation, and latency

`#![no_std]` replaces the standard-library prelude/link with `core`; `core` has
no heap allocation, I/O, or OS integration. Heap-backed collections can still
be used by explicitly linking `alloc` and supplying an allocator. Consequently,
`no_std` is a portability/dependency property, not proof that a hosted build has
lower latency, and it is not by itself proof of an allocation-free hot path.
([Rust Reference: `no_std`](https://doc.rust-lang.org/reference/names/preludes.html#the-no_std-attribute),
[`alloc` crate](https://doc.rust-lang.org/stable/alloc/))

For HFT, the stronger runtime contract is: after caller-controlled setup, encode
and decode perform zero heap allocations, take no locks, do no I/O, and do not
format strings. Dynamic collections can reallocate and make worst-case
execution time depend on allocator/capacity state; fixed-capacity/caller-owned
storage has predictable capacity failures instead.
([Embedded Rust Book: allocation, collections, and worst-case execution time](https://doc.rust-lang.org/stable/embedded-book/collections/index.html))

Make the codec/runtime layer `no_std` and allocation-free where practical, with
an additive `std` feature for error integration/tooling and a separate optional
`alloc` layer only where ownership is genuinely useful. Cargo explicitly
recommends a positive `std` feature rather than a negative `no_std` feature and
states that features should be additive. Test `--no-default-features`, `alloc`,
`std`, and all-features configurations in CI.
([Cargo features: `no_std` and additivity](https://doc.rust-lang.org/cargo/reference/features.html))

## Performance proof expected for an HFT claim

The benchmark matrix should cover encode and decode separately; fixed-only,
groups, nested groups, variable data, and versioned messages; small/typical/max
payloads; little- and big-endian schemas; checked error paths; and comparison
with the pinned Real Logic Rust/C++/Java output where meaningful. Include bytes
per second as well as time/message, compiler and target CPU, exact profile and
flags, schema/data, sample counts, and raw results.

Criterion is useful for regression detection, confidence intervals, warm-up,
and outlier reporting, but its standard measurement records batches of many
iterations rather than each individual iteration. Therefore Criterion means or
slopes alone do not establish HFT tail latency. Keep microbenchmarks for code
generation regressions and add a separate end-to-end harness that records
individual observations and publishes median plus p99/p99.9/max under a quiet,
controlled machine configuration.
([Criterion analysis process](https://bheisler.github.io/criterion.rs/book/analysis.html),
[Criterion command-line output and outlier guidance](https://bheisler.github.io/criterion.rs/book/user_guide/command_line_output.html))

Benchmark setup must be outside the timed loop and values/results must be made
opaque enough that the optimizer cannot remove the work. Profile experiments
must be reported, not assumed: Cargo warns that optimization level 3 can be
slower than 2, more codegen units can produce slower code, and LTO trades link
time for possible whole-program gains. A library's own dependency manifest
cannot force the consumer's profile because Cargo reads profiles only from the
workspace root.
([Rust benchmark guidance and optimizer pitfalls](https://doc.rust-lang.org/unstable-book/library-features/test.html),
[Cargo profiles](https://doc.rust-lang.org/cargo/reference/profiles.html))

## Documentation and pre-release bar

Rustdoc's own guidance says public items should be documented and recommends
`#![deny(missing_docs)]` for a library. Deny broken intra-doc links, run doctests,
and use `compile_fail` doctests for typestate misuse. Document `# Panics` for
every reachable panic and `# Safety` for every unsafe public contract. README
examples can be included into doctests so they cannot silently rot.
([rustdoc: what to include](https://doc.rust-lang.org/rustdoc/write-documentation/what-to-include.html),
[rustdoc lints](https://doc.rust-lang.org/rustdoc/lints.html),
[rustdoc documentation tests](https://doc.rust-lang.org/rustdoc/documentation-tests.html),
[rustdoc writing guidance](https://doc.rust-lang.org/rustdoc/how-to-write-documentation.html))

For this library, the crate front page/README/book should additionally make the
following contracts impossible to miss: supported SBE revision/profile and
known gaps; schema-evolution behavior; header/framing ownership; exact
zero-copy/lifetime model; legal cursor order; allocation and panic guarantees;
checked versus any unchecked API; error recovery; feature/MSRV matrix; a full
encode/decode example; performance methodology with reproducible commands; and
the interoperability/conformance evidence behind compatibility claims.

Set `package.rust-version` and test it. Before publishing, use `cargo publish
--dry-run`/`cargo package`, inspect packaged files, and curate a changelog/tag.
For `0.y.z`, Cargo treats a change in `y` as the incompatible/major-equivalent
release; breaking a published `0.1.x` API should therefore ship as `0.2.0`, not
as a `0.1` patch.
([Cargo `rust-version`](https://doc.rust-lang.org/cargo/reference/rust-version.html),
[Cargo publishing checklist](https://doc.rust-lang.org/cargo/reference/publishing.html),
[Cargo SemVer compatibility](https://doc.rust-lang.org/cargo/reference/semver.html))

## Highest-value release tickets implied by the sources

1. Define and document an exact SBE conformance profile; remove any unqualified
   compatibility claim.
2. Add pinned Real Logic cross-language golden vectors and the official FIX SBE
   Conformance suite as CI/release gates.
3. Audit every length/offset/count calculation and safe entry point for
   truncation, overflow, invalid enum values, panic, aliasing, and schema-version
   handling; fuzz malformed cursors and run Miri over any unsafe core.
4. Keep typestate, but add compile-fail API tests plus representative assembly,
   type-size, binary-size, and runtime comparisons against the alternative.
5. Make the hot path demonstrably allocation-free and feature-test the
   `core`/`alloc`/`std` matrix.
6. Publish a reproducible performance corpus that distinguishes throughput,
   microbenchmark central tendency, and end-to-end tail latency.
7. Turn documentation requirements into lints/doctests and add the missing SBE
   profile, safety, framing, schema evolution, performance, and migration pages.

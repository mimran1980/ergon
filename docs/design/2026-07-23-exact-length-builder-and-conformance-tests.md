# Exact-Length Builder and Codec Conformance Tests

**Date:** 2026-07-23

> **Superseded (2026-07-24):** The "generate a builder for every dynamic tail"
> interface choice in this document is superseded by the three-tier
> strategy classification in
> [`2026-07-24-simplified-encoded-length-api-implementation-plan.md`](2026-07-24-simplified-encoded-length-api-implementation-plan.md).
> The correctness, checked arithmetic, domain-length, conformance, and
> performance requirements from this document remain in force.

## Context

`ergo-sbe` generates staged encoders and decoders for fixed fields, repeating
groups, nested repeating groups, and variable-length data. The encoder supports
both declared group counts (`bids(5, |bids| { ... })`) and back-patched counts
(`bids_unknown_size(|bids| { ... })`), as well as fixed-field structs,
individual fixed-field setters, per-entry setters, and `add_struct` for
pure-fixed group entries.

The current `compute_encoded_length` implementation accepts only top-level
group counts and message-level varData lengths. It therefore cannot calculate
an exact length when an entry contains a nested group or entry-level varData.
The current L3 integration fixture exercises both constructs, but its length
test merely checks that the returned value is positive. Several L3 tests also
use group closure syntax that predates the current `GroupResult` API and fail
to compile.

## Goals

- Calculate the exact body and header-inclusive encoded lengths for arbitrary
  nesting of groups and varData.
- Keep length calculation allocation-free and independent of a wire buffer.
- Make the calculator follow the same ordered tail structure as the generated
  encoder.
- Support declared-count and unknown-size group construction at every nesting
  level.
- Give owned domain objects an exact length operation derived from their actual
  nested values.
- Reject declared group counts that do not match the number of entries written.
- Add broad generated-code tests covering the supported encoder, decoder,
  domain-object, and length-calculation usage styles.
- Preserve official SBE wire compatibility and the maintained performance
  ratios.

## Non-goals

- No generic runtime schema interpreter.
- No heap-allocated tree of length descriptors.
- No relaxation of consuming encoder or decoder tail order.
- No `unsafe` native-endian reads or writes.
- No new unchecked group-count or length-calculation API.
- No attempt to make fixed-field values influence encoded length; their block
  size is determined by the schema.

## Generated Length API

Every message with a dynamic tail generates a staged, zero-allocation length
builder. The entry point is associated with the encoder:

```rust
let length = L3BookEncoder::encoded_length_builder()
    .bids(2, |bids| {
        bids.add(|bid| {
            bid.orders_unknown_size(|orders| {
                orders.add(|order| {
                    order.order_id(7)?;
                    Ok(())
                })?;
                Ok(())
            })?;
            bid.venue(4)?;
            Ok(())
        })?;
        bids.add(|bid| {
            bid.orders(0, |_| Ok(()))?;
            bid.venue(0)?;
            Ok(())
        })?;
        Ok(())
    })?
    .asks_unknown_size(|asks| {
        asks.add(|ask| {
            ask.orders(1, |orders| {
                orders.add(|order| {
                    order.order_id(5)?;
                    Ok(())
                })
            })?;
            ask.venue(3)?;
            Ok(())
        })
    })?
    .symbol(7)?
    .encoded_length_with_header();
```

Names are generated from the schema, following the same scoping convention as
group encoders:

- `L3BookEncodedLength`
- `L3BookEncodedLengthAfterBids`
- `BidsEncodedLength`
- `BidsEntryEncodedLength`
- `BidsOrdersEncodedLength`
- `BidsOrdersEntryEncodedLength`

The initial message builder starts at the compiled message block length. A
group method adds its dimension header. Each `add` adds the entry block length
and runs a closure that adds nested dimensions, nested entries, and entry
varData. A varData method accepts a byte length, validates it against the
schema encoding, and adds its length prefix plus payload length.

Known-size methods accept the schema's generated count type and require the
closure to add exactly that number of entries. Unknown-size methods count
entries and validate that the final count fits the dimension encoding. Both
forms calculate identical lengths for identical logical values.

Only the terminal builder stage exposes:

```rust
pub fn encoded_length(&self) -> usize;
pub fn encoded_length_with_header(&self) -> usize;
```

The first excludes the message header; the second includes the schema's actual
header size. Intermediate stages do not expose complete-message lengths.

Fixed-only and structurally flat messages retain zero-allocation constant
helpers when top-level counts and varData lengths fully describe the wire
shape. A helper that cannot represent a nested dynamic tail is not generated;
callers use the staged builder instead. The builder is generated consistently
for all messages with dynamic tails so flat and nested code can share one
usage model.

## Arithmetic and Error Semantics

Length-builder addition and multiplication use checked arithmetic. Builder
methods return `Result` while work remains and use the generated runtime's
`EncodeError`:

- Existing `GroupFull { declared, attempted }` reports an extra entry in a
  known-size group.
- New `GroupCountMismatch { declared, actual }` reports too few entries when a
  known-size group closure returns.
- New `GroupCountOverflow { maximum, actual }` reports an unknown-size count
  that does not fit `numInGroup`.
- Existing `VarDataTooLong` reports a schema or prefix limit violation.
- New `EncodedLengthOverflow` reports checked `usize` arithmetic failure.

The real encoder gains the same exact-count check after every known-size
top-level or nested group closure. A failed encoder call may leave partial
bytes in the caller-owned buffer, but it never returns the next or complete
stage, so generated complete-stage APIs cannot publish the partial message.
Unknown-size encoders continue to back-patch the number of successfully added
entries.

## Domain Objects

Generated message domain objects gain:

```rust
pub fn encoded_length(&self) -> Result<usize, sbe_rt::EncodeError>;
pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::EncodeError>;
```

The implementation walks the domain object's nested vectors and varData values
through the same generated length-builder operations used by flyweight
callers. It does not allocate additional collections. Its result must equal
the length returned by `encode`, the completed encoder stage, and the decoder.

Entry domain objects get an internal/generated length contribution method when
needed for recursive message-domain calculation. It is not a second public
shape API.

## Test Architecture

### Generator unit tests

Targeted unit tests inspect generated source and generator metadata for:

- fixed-only messages;
- flat groups and message varData;
- nested known/unknown group length methods;
- entry-level and message-level varData length methods;
- staged method order;
- little- and big-endian dimension encodings;
- domain-object length methods;
- omission of misleading flat helpers for nested dynamic messages.

### Generated-code integration suite

A rich conformance schema contains:

- multiple fixed-field primitive kinds plus representative enum, set, array,
  and composite fields;
- sequential `bids` and `asks` top-level groups;
- fixed fields on each level;
- nested `orders` groups;
- pure-fixed nested entries where `add_struct` is available;
- nested entries with UTF-8 or binary varData;
- parent-entry varData after a nested group;
- message-level text and binary varData.

Existing all-types little- and big-endian fixtures remain the broad primitive
coverage source. The conformance fixture concentrates on API composition and
dynamic-tail shape.

The generated source is compiled once per schema family into a temporary test
crate containing individually named inner tests. This avoids one Cargo build
and package-cache lock per combination while preserving precise failure names.

The runtime matrix covers:

1. `fixed(&FixedFields)` and `raw_fixed()` individual setters, producing
   byte-identical messages.
2. Known/known, known/unknown, unknown/known, and unknown/unknown top-level
   `bids`/`asks` combinations.
3. Known and unknown nested-group methods crossed with the outer-group forms.
4. Per-field entry setters, `add_struct` for pure-fixed entries, and manual
   entry creation only where the generated API can safely commit a fixed-only
   entry.
5. Empty, singleton, ragged, and many-entry group shapes.
6. Empty, short, Unicode UTF-8, binary, and varied-length varData at nested
   entry, parent entry, and message levels.
7. `try_from`, offset-aware `try_wrap_and_apply_header`, random-access fixed
   fields, sequential group iteration, `finish`, `skip_remaining`, `rewind`,
   raw varData, string varData, and complete-stage byte access.
8. Flyweight-to-domain conversion, domain length calculation, exact-buffer
   domain encoding, and byte-identical decode/re-encode.
9. Expected failures for too many and too few known entries, unknown count
   overflow, oversized varData, encoded-length overflow, one-byte-short encode
   buffers, and truncated nested tails.

Every successful scenario asserts the following invariants:

```text
builder body length
    == completed encoder body length
    == decoder body length

builder header-inclusive length
    == completed encoder header-inclusive length
    == completed as_bytes().len()
    == decoder header-inclusive length
    == domain header-inclusive length
    == exact caller buffer length
```

Known-size and unknown-size encoders for the same logical value must produce
byte-identical wire output, as must fixed-struct and raw-fixed encoding.

## Existing Test Repair

The stale L3 tests under `sbe/tests` will be updated to the current closure
contract (`Result<(), EncodeError>` via `Ok(())` and `?`). Their weak positive
length assertion will be replaced with exact nested-group and nested-varData
agreements. Existing tests that duplicate the conformance matrix without
covering a distinct contract may be consolidated to keep runtime bounded.

## Performance and Verification

Length builders are used before encoding and allocate no heap memory. Encoder
count validation adds one comparison after each known-size group closure and
does not add a per-field or per-byte branch.

Verification must include:

- focused generator unit tests;
- the generated conformance integration suite;
- repaired L3 and domain-object integration tests;
- the complete `ergo-sbe` test suite;
- formatting and Clippy checks required by the workspace;
- `just bench`, with every maintained ergon/Aeron ratio at or below `1.00`.

If the exact-count validation causes a maintained regression, its
implementation must be revised or reverted without weakening the correctness
contract.

## Acceptance Criteria

- A nested group with entry varData can be sized exactly before allocating its
  encoding buffer.
- All known/unknown group combinations produce correct dimension headers and
  identical bytes for identical logical input.
- Too few known-size entries are rejected rather than silently publishing a
  malformed message.
- Builder, encoder, decoder, byte slice, and domain-object lengths agree for
  every conformance scenario.
- Exact-size buffers succeed and buffers one byte shorter fail cleanly.
- The full suite passes with no newly ignored coverage.
- Official SBE wire compatibility is preserved.
- All maintained benchmark ratios satisfy the repository performance gate.

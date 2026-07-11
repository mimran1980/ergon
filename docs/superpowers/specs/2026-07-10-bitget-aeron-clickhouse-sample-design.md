# Bitget to Aeron IPC to ClickHouse Sample Design

**Status:** Approved for future implementation on 2026-07-10; `AppMessage`
envelope and fallible-chaining revision approved on 2026-07-11

**Current change scope:** Documentation only. Do not implement code as part of
this documentation change.

**Implementation authority:** This document defines the advanced sample. The
canonical SBE rules in `sbe/design/DECISIONS.md` remain authoritative whenever
they impose a stronger compatibility, performance, safety, or API requirement.

## 1. Purpose

Build a production-shaped sample that proves the following path end to end:

```text
Bitget BTCUSDT WebSocket
  -> parse Bitget book and trade messages
  -> build a normalized L2 book
  -> wrap normalized L2Book and Trade payloads in AppMessage
  -> encode both SBE layers directly into Aeron IPC claims
  -> consume typed and dynamic streams
  -> compare both L2 representations
  -> persist typed books, dynamic books, and trades into ClickHouse
```

The sample is deliberately more demanding than the existing
`samples/exchange-orderbook` example. The existing example and its historical
completion record remain valid for their original scope. This design is a
separate advanced successor and must not cause old results to be rewritten as
though they measured this new pipeline.

The sample process owns exactly three long-lived application threads.
ClickHouse already runs in Docker as an external service and is not started or
managed by the sample.

## 2. Priorities and non-negotiable constraints

Apply the repository priority order:

1. Official-SBE wire compatibility is non-negotiable.
2. Maintained ErgoSBE paths must be equal to or faster than their Aeron SBE
   equivalents under the canonical benchmark rules.
3. Prefer an easier or safer Rust API when it is zero-cost or outside the hot
   path.
4. No safety check, abstraction, branch, allocation, or ergonomic wrapper may
   slow the benchmarked hot path unless it is an explicit opt-in.
5. Simplicity decides only when compatibility, performance, and safety are
   equal.

Additional sample constraints:

- Pin `rusteron-client` and `rusteron-media-driver` to exactly `0.2.1`.
- Use Aeron `try_claim`, not `offer`, for every published SBE message.
- Encode directly into the claimed Aeron payload. Do not encode into a
  temporary buffer and copy.
- Use separate stream IDs over the same `aeron:ipc` transport for typed market
  data and dynamic-table messages.
- Drop a data message immediately when its claim fails. Do not retry, block,
  queue, or fall back to `offer`.
- Publish control-plane schema data successfully before live ingestion begins.
- Wrap every normalized `L2Book` and `Trade` in `AppMessage`. Do not wrap
  `DynamicSchema`, `DynamicRow`, or their V2 replacements; they are platform
  infrastructure messages on the separate dynamic stream.
- Do not introduce unbounded queues.
- Do not run code generation as part of a Markdown-only documentation change.

## 3. Dependency contract

The future sample must use exact dependency pins:

```toml
rusteron-client = "=0.2.1"
rusteron-media-driver = "=0.2.1"
```

Do not silently upgrade these crates while implementing this design. A future
version change is a separate measured and reviewed decision.

Use exclusive publications because each publication has exactly one owner on
the producer thread. Both producer and consumer Aeron clients must use the
conductor agent invoker and drive it from their owning application thread so a
client conductor does not create another long-lived thread.

## 4. Three-thread architecture

### 4.1 Thread 1: Bitget ingestion, normalization, and publication

The process main thread is thread 1. It runs a current-thread async runtime and
owns:

- the Bitget BTCUSDT public WebSocket connection;
- subscriptions for L2 book updates and public trades;
- Bitget SBE or JSON decoding required by the live channel in use;
- the normalized in-memory L2 book;
- the typed and dynamic Aeron exclusive publications;
- the producer Aeron client conductor invoker;
- exact-length calculation, claiming, direct encoding, and committing;
- outer `AppMessage` metadata plus zero-copy nested-message encoding;
- producer-side counters and reconnect state.

Bitget parsing is an external-feed boundary. The Aeron payloads are new,
exchange-neutral internal SBE messages. Do not republish or merely rename the
incoming Bitget wire messages.

On a WebSocket disconnect, reconnect with a capped backoff, resubscribe, and do
not publish another book until a fresh valid snapshot has rebuilt the local
state. Trades may resume after their own subscription is valid.

### 4.2 Thread 2: SHARED media driver

Thread 2 owns `rusteron-media-driver` and runs the Aeron media driver in
`SHARED` threading mode. It is the single long-lived media-driver thread and
must not start dedicated sender, receiver, or conductor threads.

Configure IPC MTU and term settings before starting the driver. The IPC maximum
payload must be large enough for the largest maintained typed book, dynamic
book, trade, and schema message. The initial expected IPC MTU is 4096 bytes,
but implementation must derive the real requirement from the final schemas and
verify the selected Aeron-supported value rather than relying on this estimate.

### 4.3 Thread 3: subscription, comparison, and ClickHouse

Thread 3 owns:

- both Aeron subscriptions;
- the consumer Aeron client conductor invoker;
- typed SBE dispatch and decoding;
- dynamic schema registration and dynamic row decoding;
- ordered typed/dynamic L2 matching;
- equality checking;
- foreground ClickHouse batching and writes;
- consumer, comparison, drop, and persistence counters.

The existing ClickHouse sink starts a worker thread. The advanced sample cannot
use that background mode unchanged because it would violate the three-thread
contract. Add or use a foreground mode whose work loop is driven by thread 3
while preserving existing public behaviour for users who select the background
sink.

Use `127.0.0.1` for the default Docker ClickHouse endpoint where practical so
local ClickHouse access does not require a background DNS helper. The live
Bitget DNS/TLS path must use the current-thread runtime without creating an
intentional extra long-lived worker pool.

## 5. Aeron channels, streams, and message dispatch

Use one IPC media channel and two stream IDs:

```text
aeron:ipc + TYPED_STREAM_ID = 1001
  - AppMessage(payload = normalized L2Book)
  - AppMessage(payload = normalized Trade)

aeron:ipc + DYNAMIC_STREAM_ID = 1002
  - dynamic schema registration
  - dynamic L2Book rows
```

Use these stable named constants and values unless a verified Aeron constraint
requires a separately reviewed change. `AppMessage` is an ordinary SBE message,
not a transport-specific or redundant non-SBE discriminator. Dispatch the outer
fragment and the complete nested payload by their standard SBE message headers,
including both schema ID and template ID.

Every typed L2 book and corresponding dynamic row carries the same monotonically
ordered sequence/correlation value. Each stream preserves its own ordering.
Thread 3 performs an ordered merge:

- equal sequence values: decode, compare, and persist both rows;
- smaller typed sequence: count and discard an unmatched typed book;
- smaller dynamic sequence: count and discard an unmatched dynamic book;
- equal sequence but unequal content: count and log the mismatch, then persist
  neither representation.

The merge must use bounded state and must not grow a map or queue without
limit.

## 6. Normalized internal SBE messages

### 6.1 AppMessage application envelope

Place `AppMessage`, `L2Book`, and `Trade` in the same normalized application SBE
schema so the generated `AnyMessage` enum can dispatch both the outer and inner
messages. `AppMessage` contains:

- fixed `sentTs: uint64`, defined as a Unix epoch timestamp in nanoseconds;
- UTF-8 `appName` variable data;
- terminal `payload` variable data containing one complete same-schema SBE
  message, including its standard SBE message header.

The only accepted nested payload variants are `L2Book` and `Trade`. Reject and
count recursive `AppMessage`, unknown templates, wrong schema IDs, malformed or
incomplete nested frames, and infrastructure messages. `DynamicSchema` and
`DynamicRow` families remain unwrapped on `DYNAMIC_STREAM_ID`.

The ordered envelope shape is:

```text
AppMessageEncoder
  -> AppMessageAfterAppName
  -> AppMessageComplete

AppMessageDecoder
  -> AppMessageAfterAppName
  -> AppMessageComplete
```

The final transition is the terminal `payload` field. Manual consuming methods
and fallible closure conveniences must return the same concrete stages.

### 6.2 L2Book

Generate a new exchange-neutral `L2Book` schema. It contains at least:

- source/exchange identifier;
- exchange timestamp;
- local receive timestamp;
- sequence/correlation ID;
- ordered `bids` repeating group;
- ordered `asks` repeating group;
- trailing symbol variable data.

Each level contains `price` and `size` as a variable-exponent SBE `Decimal`
composite:

```text
Decimal
  mantissa: int64
  exponent: int8
```

The represented value is `mantissa * 10^exponent`. The exponent is encoded per
value so instruments requiring 15 or more fractional decimal places remain
exact. Do not assume or bake scale 8 into the normalized wire message.

The sample enables the generator's generic converter seam:

```rust
let config = GenerationConfig::new("normalized_app")
    .enable_decimal_converters("Decimal");
```

The generator validates the composite shape and emits a local `SbeDecimal`
trait. The sample implements that trait for `rust_decimal::Decimal`, so ordinary
generated price/size accessors and setters use `rust_decimal::Decimal` directly
with fallible exact conversion. Generated code must not depend on
`rust_decimal`; any application decimal type may implement the trait. Raw
infallible `price_wire()`/`size_wire()` accessors and setters preserve the
generated composite escape hatch. Without converter mode, ordinary methods use
the raw generated composite.

```rust
pub trait SbeDecimal: Sized {
    type Error;

    fn try_from_sbe(mantissa: i64, exponent: i8)
        -> Result<Self, Self::Error>;
    fn try_into_sbe(self) -> Result<(i64, i8), Self::Error>;
}
```

Generated setters use a named inferred type parameter such as
`fn price<D: SbeDecimal>(..., value: D) -> Result<..., D::Error>`. This accepts
any implementing type without a turbofish while allowing the return type to
name that adapter's error.

`SbeDecimal` conversion must be allocation-free and exact. Reject exponent,
mantissa, scale, or range combinations that `rust_decimal` cannot represent;
reverse conversion must reproduce the same numeric value without rounding or
truncation. Conversion errors compose with the approved caller-error `?`
closures.
The tail order is exactly:

```text
L2BookEncoder
  -> BidsEncoder
  -> L2BookAfterBids
  -> AsksEncoder
  -> L2BookAfterAsks
  -> SymbolEncoder
  -> L2BookComplete
```

The decoder uses equivalent consuming stages. It must be impossible to encode
or decode asks before completing or explicitly skipping bids. An active group
entry prevents its parent from advancing.

### 6.3 Trade

Generate a normalized singular `Trade` message for each public trade entry.
It contains at least:

- source/exchange identifier;
- exchange and receive timestamps;
- trade/execution ID;
- variable-exponent `Decimal` composites for price and size;
- side enum;
- trailing symbol variable data.

If one incoming Bitget frame contains several trades, attempt one independent
claim per normalized trade. Failure to claim one trade drops only that trade.

### 6.4 Schema documentation

The new schemas must intentionally exercise all supported schema documentation
sources:

- `description="..."` attributes;
- `<description>...</description>` child elements;
- supported `<comment>...</comment>` child elements or tags;
- ordinary XML `<!-- ... -->` comments.

Generated rustdoc must combine and associate these sources according to the
canonical schema-documentation rules without changing wire layout or bytes.

## 7. Exact sizing and zero-copy claim lifecycle

Every generated complete message used by this sample must support exact sizing
before encoding:

```text
compute_encoded_length(...)
compute_encoded_length_with_message_header(...)
```

For nested groups, variable data, or arrays, the length input must describe all
runtime counts and data lengths needed for an exact result. A helper that knows
only top-level group counts is not exact and does not satisfy this design.

The header-inclusive helper returns the SBE message header plus SBE body. It
does not include the Aeron data-frame header.

For an application message, calculate the header-inclusive `L2Book` or `Trade`
length first. Then calculate the outer `AppMessage` length from
`app_name.len()` and that exact inner length. The inner length is the declared
length of the terminal payload var-data field.

The producer lifecycle is:

1. Validate and normalize source values before claiming.
2. Compute the exact header-inclusive inner message length.
3. Compute the exact header-inclusive outer `AppMessage` length.
4. Verify the outer length is no greater than the publication maximum payload.
5. Call `try_claim_owned(outer_length)` on the typed exclusive publication.
6. If claiming fails, increment the relevant drop counter and return
   immediately.
7. Wrap `AppMessageEncoder` directly around the claim's writable data slice.
8. Encode `sentTs` and `appName`, then call
   `payload_with(inner_length, |payload| -> Result<(), E> { ... })`.
9. The closure receives exactly the inner payload region. Wrap and complete the
   inner encoder directly in that slice; do not expose the rest of the claim.
10. Assert the inner complete header-inclusive view equals `inner_length` and
    the outer complete header-inclusive view equals `outer_length`.
11. End both encoder borrows and commit the claim.

An owned claim that leaves scope before commit must abort through its RAII
lifecycle. Unexpected encoding failure after a successful exact claim is an
invariant failure; do not commit partial data.

There must be no use of:

- `offer` as a normal path or fallback;
- an intermediate encoded `Vec` or fixed buffer;
- `copy_from_slice` from an encoded message into a claim;
- Aeron fragmentation as a substitute for a correct claim size;
- incomplete-stage `as_bytes()` masquerading as a complete message.

### 7.1 Dual manual and fallible-closure interfaces

Concrete consuming stages remain the canonical interface. A caller can set
every fixed field directly, then drive every group, nested tail, and var-data
transition manually. No dummy closure is required.

The maintained sample includes the manual model:

```rust
let mut app = AppMessageEncoder::wrap_and_apply_header(claim.data_mut(), 0)?;
app.sent_ts(epoch_ns);

let complete = app
    .app_name(app_name.as_bytes())?
    .payload_with(inner_len, |payload| -> Result<(), AppError> {
        let inner = encode_l2book_manually(payload)?;
        debug_assert_eq!(inner.as_bytes_with_header().len(), payload.len());
        Ok(())
    })?;
```

The `Trade` publication uses the same outer sequence with its own concrete
inner complete stage.

Additive fallible helpers return the same concrete next stages:

```rust
let complete = AppMessageEncoder::wrap_and_apply_header(claim.data_mut(), 0)?
    .try_fixed(|app| -> Result<(), AppError> {
        validate_timestamp(epoch_ns)?;
        app.sent_ts(epoch_ns);
        Ok(())
    })?
    .app_name(app_name.as_bytes())?
    .payload_with(inner_len, |payload| -> Result<(), AppError> {
        let inner = encode_l2book_fallibly(payload)?;
        debug_assert_eq!(inner.as_bytes_with_header().len(), payload.len());
        Ok(())
    })?;
```

Equivalent decoder conveniences are `try_fixed`, `try_app_name`, and
`try_payload_as_message`. The last helper reborrows the payload for a scoped
same-schema `AnyMessage` dispatch and accepts only `L2Book` or `Trade`.

The maintained decoder examples also show both models. Manual decoding uses
`sent_ts()`, `into_app_name()`, `into_payload()`, and explicit
`AnyMessage::decode_frame(...)`. Fallible chaining uses:

```rust
let complete = AppMessageDecoder::try_from(fragment)?
    .try_fixed(|app| -> Result<(), AppError> {
        validate_sent_ts(app.sent_ts())?;
        Ok(())
    })?
    .try_app_name(|name| {
        validate_app_name(name)?;
        Ok(())
    })?
    .try_payload_as_message(|message| {
        match message {
            AnyMessage::L2Book(book) => decode_book(book)?,
            AnyMessage::Trade(trade) => decode_trade(trade)?,
            _ => return Err(AppError::UnsupportedPayload),
        }
        Ok(())
    })?;
```

Fallible helpers are generic over the caller's error. Helpers that can produce
codec failures require `E: From<EncodeError>` or `E: From<DecodeError>`;
custom closure failures propagate unchanged. Use higher-ranked callback
lifetimes so borrowed slices, entries, and decoders cannot escape. Do not add a
boxed error, trait object, allocation, or formatted error on the success path.

The maintained manual and closure examples must produce identical bytes and
decoded values. A closure helper is retained only after assembly, allocation,
and five-run benchmarks prove it does not slow the equivalent manual hot path.

## 8. Dynamic Decimal array support

The L2 book must also be represented as one dynamic row per complete snapshot.
Its ClickHouse columns are:

```text
source               String
symbol               String
exchange_timestamp   UInt64
receive_timestamp    UInt64
sequence              UInt64
bid_prices            Array(Decimal(38,18))
bid_sizes             Array(Decimal(38,18))
ask_prices            Array(Decimal(38,18))
ask_sizes             Array(Decimal(38,18))
```

Do not represent these arrays as JSON, strings, opaque byte columns, one row per
level, or lossy floating-point values.

The current dynamic recorder does not support Decimal or Array values. Add
Decimal-array support as an independently verified prerequisite before wiring
the E2E sample.

Dynamic schema and row messages are platform infrastructure. Publish their
standard SBE messages directly on `DYNAMIC_STREAM_ID`; never put them inside
`AppMessage`.

Preserve the existing version-0 `DynamicSchema` and `DynamicRow` template IDs
1 and 2, bytes, and decoding behaviour. Do not insert a new variable tail
before the existing terminal symbol table in a way that old template decoders
misinterpret.

Add array support with schema version 1 and two new templates:

```text
template 3: DynamicSchemaV2
template 4: DynamicRowV2
```

`DynamicSchemaV2` has column descriptors for outer type, element type,
precision, and scale. `DynamicRowV2` retains the scalar field groups, then adds
an ordered `decimalArrayFields` group whose entries contain `fieldId` followed
by a nested `values` group of the same variable-exponent `Decimal` composite
(`mantissa: int64`, `exponent: int8`). Its symbol table remains the terminal
variable data. `SchemaRegistry` and `RowDecoder` accept both the version-0 and
V2 template families. Independently validate the new templates against
official SBE tooling before keeping the implementation.

The array-capable schema communicates:

- the outer Array type;
- Decimal element type;
- precision 38;
- ClickHouse target scale 18.

Each dynamic row preserves the original mantissa and exponent for every value.
The dynamic schema owns the ClickHouse precision and target scale metadata;
the transport does not destroy the per-value exponent. Nested repeating groups
are preferred over an undocumented binary blob so official SBE tooling can
describe and decode the layout.

Add a borrowed recording API, such as `DynamicValueRef<'a>`, so strings and
Decimal arrays can be supplied without constructing owned values on every
record. Add exact header-inclusive size calculation and a `record_into`-style
API that writes directly into the caller-provided Aeron claim. Preserve the
existing owning and reusable-buffer APIs for compatibility.

Conversion to ClickHouse `Decimal(38,18)` is exact and checked. For wire value
`mantissa * 10^exponent`, the stored scaled integer is
`mantissa * 10^(exponent + 18)`. When `exponent + 18` is negative, division is
allowed only if every discarded digit is zero. Reject non-zero precision loss,
checked-integer overflow, values outside Decimal(38,18), and any value the
selected Rust adapter cannot represent. Never round or silently truncate.

## 9. ClickHouse persistence

Assume ClickHouse is already running in Docker. The sample does not start,
stop, recreate, or otherwise manage the container.

Thread 3 creates or verifies these tables before live ingestion:

```text
l2book_typed
l2book_dynamic
trades
```

The typed and dynamic L2 tables use equivalent user columns, including all four
`Array(Decimal(38,18))` columns. They may contain the normal persistence metadata
columns added by the persistence crate. Both rows from an equal matched pair
are persisted so ClickHouse can be queried for row-for-row equivalence.

Normalized trades are decoded from the typed stream and persisted independently
to `trades`.

ClickHouse batching occurs on thread 3. It may allocate outside the producer
hot path, but it must be bounded. If a foreground batch insert fails, count and
log the failure, discard that failed batch, and continue live processing. Table
creation or compatibility failure during startup is fatal.

## 10. Backpressure, errors, and observability

Aeron data backpressure is intentionally simple:

- call `try_claim_owned` once;
- on any unsuccessful data claim, count and drop the current message;
- do not retry or sleep;
- do not block WebSocket ingestion;
- do not enqueue the data for later publication.

A retryable Aeron status changes only the drop counters. A fatal publication
status, such as a closed publication or maximum position, also requests an
orderly process shutdown after dropping the current message; otherwise the
sample would silently drop every subsequent input forever.

Schema registration is control-plane data. Establish publication/subscription
connectivity and publish the schema successfully before reading live market
data. If that cannot be done within the documented startup deadline, fail
startup rather than sending undecodable rows.

Maintain at least these counters:

- Bitget book and trade inputs;
- valid normalized books and trades;
- typed book, trade, dynamic schema, and dynamic row claim successes;
- claim drops by message type and Aeron error;
- typed and dynamic decode failures;
- unmatched typed and dynamic book sequences;
- equal and unequal pairs;
- ClickHouse rows and batches persisted;
- ClickHouse batches dropped;
- WebSocket reconnects;
- invariant and shutdown errors;
- rejected recursive, unknown, wrong-schema, infrastructure, and malformed
  `AppMessage` payloads;
- custom encode/decode callback failures and aborted claims.

Counters must be observable at shutdown and periodically without adding
per-message logging to the hot path.

## 11. Startup and shutdown

Startup order:

1. Thread 2 starts the SHARED driver and signals readiness.
2. Thread 3 connects to ClickHouse, verifies tables, creates subscriptions, and
   signals readiness.
3. Thread 1 creates publications and waits for connected subscribers.
4. Thread 1 publishes the dynamic schema successfully.
5. Thread 1 connects to Bitget and starts live subscriptions.

If ClickHouse is unavailable, exit with a clear message that the external
Docker service must already be running. Do not auto-start it.

On Ctrl-C or another requested shutdown:

1. Set a shared atomic shutdown flag.
2. Thread 1 stops WebSocket ingestion and closes publications.
3. Thread 3 drains available fragments, flushes its final foreground
   ClickHouse batch, and exits.
4. Thread 2 stops last so IPC remains available during the drain.
5. Thread 1 joins threads 3 and 2 and prints final counters.

## 12. Verification requirements

### 12.1 Compile-time and API proofs

Add compile-fail tests proving at least:

- asks cannot be encoded before bids;
- asks cannot be decoded before bids;
- a consumed stage cannot be reused;
- a parent cannot advance while a group entry or nested tail is active;
- incomplete encoders cannot call complete-message byte views;
- complete encoders expose the explicit header-inclusive byte view used by the
  claim-length assertion.
- borrowed payload slices, nested decoders, and group entries cannot escape a
  fallible callback;
- manual and closure paths return the same concrete next-stage types;
- custom error types can use `?` inside fixed-body, group, var-data, and nested
  message callbacks;
- calling the completion-only byte/length view used by the maintained payload
  closure on an incomplete nested encoder fails to compile; runtime tests prove
  the sample performs the exact-length assertion before returning `Ok(())`.

### 12.2 Length, wire, allocation, and runtime proofs

Test at least:

- empty, one-level, typical, and 50-by-50 L2 books;
- zero, one, and batched incoming trades;
- exact length for typed books, trades, schema messages, and Decimal-array
  dynamic rows;
- exact inner `L2Book`/`Trade` and outer `AppMessage` header-inclusive lengths;
- claimed length equal to final header-inclusive encoded length;
- message length no greater than actual publication maximum payload;
- independent official-Aeron SBE byte parity;
- version-0 dynamic scalar compatibility;
- array-capable dynamic schema/row compatibility and acting-version behaviour;
- Decimal positive, negative, minimum, maximum, rescale, overflow, and
  precision-loss cases;
- mixed wire exponents including `0`, `-8`, `-15`, `-18`, exact values below
  `-18`, inexact values below `-18`, and values outside `rust_decimal` or
  ClickHouse Decimal(38,18) range;
- generic `SbeDecimal` round trips for `rust_decimal::Decimal` and a second
  test adapter, plus infallible raw `*_wire` access;
- zero heap allocation for warmed-up size, claim, direct encode, and commit of
  each maintained producer message;
- manual/fallible-closure byte equality and decoded-value equality;
- unchanged custom-error propagation, closure-error claim abort, and no commit
  of a partial inner or outer message;
- `AppMessage` parity for empty, short, typical, and maximum app names and
  epoch-nanosecond timestamp boundary values;
- rejection of recursive envelopes, unknown or wrong-schema inner headers,
  infrastructure payloads, malformed payloads, and declared-length mismatch;
- unchanged unwrapped version-0 and V2 dynamic-message bytes;
- real IPC round trips through a SHARED Rusteron 0.2.1 driver;
- immediate claim-drop behaviour;
- asymmetric typed/dynamic drops and ordered unmatched handling;
- equal and unequal pair handling;
- malformed and wrong-schema messages;
- startup failure without ClickHouse;
- final drain and foreground flush.

Tests must prove direct claim encoding. A test that encodes elsewhere and then
copies into a claim is invalid even if the final bytes match.

### 12.3 Deterministic and live E2E

The automated E2E gate uses deterministic captured Bitget fixtures and a real
local Aeron IPC driver. Against a running Docker ClickHouse instance it must:

1. replay book and trade fixtures;
2. build normalized state;
3. publish typed and dynamic messages by direct claims;
4. consume and compare matched books;
5. persist all three table shapes;
6. query ClickHouse;
7. prove typed and dynamic books are equal row for row;
8. prove expected trades are present.

The live Bitget BTCUSDT run is an additional smoke test. Record its date,
duration, event counts, drops, mismatches, persistence results, and any current
exchange protocol assumptions. Network availability does not replace the
deterministic gate.

## 13. Performance and coverage gates

Benchmark:

- exact length calculation;
- manual and fallible-closure `AppMessage` plus nested L2 direct-claim encode
  for 0, 1, typical, and 50-by-50 counts;
- dynamic Decimal-array direct-claim encode for the same shapes;
- manual and fallible-closure `AppMessage` plus nested Trade direct-claim encode;
- manual and fallible-closure outer decode plus nested enum dispatch;
- raw inner-message and complete-envelope costs reported separately;
- raw Decimal-composite and converted `SbeDecimal` paths reported separately,
  with equivalent conversion work in the Aeron comparison;
- typed and dynamic decode;
- pair comparison;
- IPC claim/commit throughput;
- deterministic three-thread E2E throughput and latency.

For each maintained ErgoSBE/Aeron comparison, run five comparable warmed-up
runs. The median ErgoSBE/Aeron ratio must be at most 1.00. Record Criterion
confidence intervals, the previous ErgoSBE baseline, exact commands, date,
hardware, OS, Rust toolchain, Rusteron version, Aeron revision, and profile.

For each fallible helper, also run five comparable warmed-up measurements
against the equivalent manual concrete-stage path. The median
fallible-convenience/manual ratio must be at most 1.00. Anything above 1.00 is
unfinished even when close or inside ordinary noise. Compare Aeron against the
same outer and inner schemas; an unenveloped Aeron message is not comparable.
Inspect generated assembly for both success paths and record zero-allocation
evidence. LTO and profile settings are not proof of zero-cost abstraction.

Reach 100 percent line, function, region, and branch coverage for all new or
changed handwritten production code. Report generated codecs and external FFI
separately with explicit, justified filters. Existing workspace coverage must
not regress. Do not use ignored or auto-skipped tests as evidence that an
external integration passed.

Final verification includes formatting, clippy with warnings denied, all
features, the trusted-input mode where applicable, complete tests, compile-fail
tests, official wire parity, allocation tests, coverage, Docker ClickHouse E2E,
benchmarks, documentation, and a clean regeneration stability check.

## 14. Incremental implementation order

Implement one small, measured vertical slice at a time. Each slice starts with
a failing proof, makes the minimum change, runs narrow and neighbouring gates,
updates the durable ledger, and commits a focused verified change.

Recommended dependency order:

1. Exact `compute_encoded_length_with_message_header` and complete
   header-inclusive byte-view contracts.
2. Reconcile the actual nested-var-data surface: implement and prove
   `as_decoder`/`as_message`, bounded `payload_with`, manual stages, fallible
   combinators, custom-error propagation, and completion-only byte views.
3. Add the normalized application schema containing `AppMessage`, `L2Book`, and
   `Trade`, with official parity and exact inner/outer length proofs.
4. Independent Decimal-array dynamic schema, recorder, decoder, and foreground
   persistence support.
5. Rusteron 0.2.1 direct-claim adapter and maximum-payload proofs.
6. Foreground ClickHouse execution without an extra long-lived thread.
7. Deterministic three-thread fixture pipeline.
8. Live Bitget BTCUSDT integration and reconnect behaviour.
9. Full allocation, coverage, parity, performance, and final quality gates.

Do not start with the live WebSocket and leave the prerequisites as stubs. Each
slice must be independently useful, tested, benchmarked where performance
sensitive, and buildable before proceeding.

Do not stop merely because the work is large, Docker/Java/Aeron tooling is not
initially available, or the live network is unavailable. Install authorised
local dependencies, start authorised services for verification, record genuine
external blockers, and continue independent work.

## 15. Definition of done

This advanced sample is complete only when all of the following are true in the
same final worktree:

- the process has the approved three-thread ownership model;
- Rusteron dependencies are pinned exactly to 0.2.1;
- Bitget BTCUSDT books and trades feed normalized internal messages;
- every normalized `L2Book` and `Trade` is a complete same-schema SBE payload
  inside `AppMessage` with `sentTs` epoch nanoseconds and UTF-8 `appName`;
- dynamic schema and row messages remain direct, unwrapped infrastructure
  messages on their separate stream;
- typed and dynamic messages use separate IPC stream IDs;
- every publication uses exact-length direct `try_claim` encoding;
- manual concrete stages and fallible closure chaining are both implemented,
  tested, documented, allocation-free, and within both median performance gates;
- normalized price/quantity wire fields use per-value mantissa/exponent Decimal
  composites; the sample uses the generic `SbeDecimal` seam with
  `rust_decimal::Decimal` while retaining raw `*_wire` access;
- no `offer`, encoded temporary buffer, copy fallback, or fragmentation is
  present;
- full 50-by-50 books fit the configured and verified IPC maximum payload;
- dynamic rows use the four required `Array(Decimal(38,18))` columns;
- version-0 dynamic messages retain their documented compatibility;
- typed and dynamic books are matched, compared, and persisted to separate
  tables;
- normalized trades are persisted;
- claim failure drops immediately and is observable;
- deterministic Aeron plus ClickHouse E2E passes;
- the live Bitget smoke test has current dated evidence;
- official-SBE byte parity, zero-allocation proofs, coverage, benchmarks, and
  all repository quality gates pass;
- documentation matches the actual API and runtime behaviour;
- no unrelated user changes are overwritten or included.

Anything missing, skipped, copied, auto-skipped, unmeasured, or merely assumed
means the work is not complete.

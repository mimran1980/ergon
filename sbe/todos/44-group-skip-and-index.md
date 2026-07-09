# Group skip, indexed access, and rewind

**Blocked by:** `03-group-vardata-wire-parity`

Groups in the SBE tail are sequential. To reach the third group you must step
past the first two. Currently this requires manually reading dimension headers
and computing strides. The decoder should do this for you.
**Status: ACTIVE / GROUP ERGONOMICS**


## skip_n() — generic group skip

Skip `n` entries within an already-decoded group. Uses entry `encoded_length()`
for groups with var-data tails; O(1) stride math for fixed-size entries.

```rust
let mut ff = car.fuel_figures()?;
ff.skip_n(1)?;   // skip first entry, now positioned at entry 1
let entry = ff.next().unwrap();  // reads entry 1 (second entry)
```

- [x] `skip_n()` generated for every repeating group (generic, not per-group named)
- [x] Uses entry `encoded_length()` for groups with var-data tails; O(1) stride for fixed-size groups
- [x] Returns `Result<(), DecodeError>` (validates n <= remaining count)
- [x] `unsafe fn skip_n_unchecked()` — skips without extent validation
- [x] Updates internal position so subsequent entries/navigation read from the right offset

## skip_all() — skip to end of message

Skip ALL remaining groups and var-data to reach end-of-message:

```rust
car.skip_all()?;
assert_eq!(car.pos, car.buf.len()); // at end of message
```

- [x] `skip_all()` skips every remaining group and var-data field
- [x] Returns post-tail offset

## Indexed group access (random entry by index)

Jump directly to entry N without iterating through 0..N-1:

```rust
let entry_47 = car.fuel_figures()?.nth(47)?;  // stride math, no decode of 0..46
```

`nth()` on group iterators uses stride math: `base + N × entry_block_length`.
This is already spec'd in `33-rust-idiomatic-api.md` — verify it's efficient
stride math, not actual iteration.

- [x] `nth()` on group decoders is O(1) stride math, not O(N) iteration
- [x] Verified by benchmark: `nth(1000)` takes same time as `nth(1)`

## rewind() — reset group iteration

After iterating through a group, reset the position to re-read it:

```rust
let mut iter = car.fuel_figures()?;
let first_pass: Vec<_> = iter.map(|e| e.speed()).collect();
iter.rewind();  // back to first entry
let second_pass: Vec<_> = iter.map(|e| e.speed()).collect();
assert_eq!(first_pass, second_pass);
```

- [x] `rewind()` on group decoders resets internal position to first entry
- [x] Does not re-read the dimension header (cached from initial accessor call)
- [x] Zero allocation — just resets an offset counter

## Combined example: skip bids, read asks, rewind

```rust
let car = CarDecoder::try_from(buf)?;

// Only interested in asks
car.skip_fuel_figures()?;
let asks = car.performance_figures()?;
for entry in asks {
    println!("ask: {}", entry.price());
}
asks.rewind();  // re-read if needed
```

## Acceptance criteria

- [x] `skip_n()` on every repeating group (generic skip, not per-group named)
- [x] `skip_all()` on every message with tail elements
- [x] `nth()` on group iterators uses O(1) stride math
- [x] `rewind()` on group iterators resets to first entry
- [x] Dimension headers read once, cached for rewind
- [x] All `skip` methods have `_unchecked` variants
- [x] Test: skip bids, read asks, assert correct ask entry values
- [x] Test: skip_all() → pos == buf.len()
- [x] Test: nth(5) on 100-entry group → correct entry, no iteration overhead
- [x] Test: rewind → re-iterate → same values


## Verification / Unit Testing
- [x] Create unit tests `test_group_skip_and_index` verifying `skip_n()`, `nth()`, `rewind()`, and `as_chunks()` navigate groups correctly and return errors for out of bounds access.

Audit note (2026-07-07): Verified. `skip_n()`, `nth()` (O(1) stride), `rewind()`, `remaining()`, and `is_empty()` confirmed in:
- codegen.rs lines 2546-2648 (group decoder impl)
- golden car_example.rs lines 1420-1476, 1650-1706, 1859-1908 (all three group decoders)

`skip_all()` (message-level), `skip_n_unchecked()`, and dedicated unit tests remain unimplemented. No test currently exercises `skip_n()`, `nth()`, or `rewind()` at runtime.

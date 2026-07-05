# Group skip, indexed access, and rewind

**Blocked by:** `03-group-vardata-wire-parity`

Groups in the SBE tail are sequential. To reach the third group you must step
past the first two. Currently this requires manually reading dimension headers
and computing strides. The decoder should do this for you.

## skip_<group>()

Skip an entire group without decoding it. Reads the dimension header, computes
the region size, advances the decoder position past it:

```rust
let car = CarDecoder::try_from(buf)?;
car.skip_fuel_figures()?;  // read dim header, advance past all fuel figure entries
// car.performance_figures() now reads from the correct position
```

- [ ] `skip_<group>()` generated for every repeating group
- [ ] Reads the dimension header (blockLength + numInGroup)
- [ ] Returns `Result<(), DecodeError>` (validates extent fits in buffer)
- [ ] `unsafe fn skip_<group>_unchecked()` — skips without extent validation
- [ ] Updates internal position so subsequent tail accessors read from the right offset

## skip_all() — skip to end of message

Skip ALL remaining groups and var-data to reach end-of-message:

```rust
car.skip_all()?;
assert_eq!(car.pos, car.buf.len()); // at end of message
```

- [ ] `skip_all()` skips every remaining group and var-data field
- [ ] Returns post-tail offset

## Indexed group access (random entry by index)

Jump directly to entry N without iterating through 0..N-1:

```rust
let entry_47 = car.fuel_figures()?.nth(47)?;  // stride math, no decode of 0..46
```

`nth()` on group iterators uses stride math: `base + N × entry_block_length`.
This is already spec'd in `33-rust-idiomatic-api.md` — verify it's efficient
stride math, not actual iteration.

- [ ] `nth()` on group decoders is O(1) stride math, not O(N) iteration
- [ ] Verified by benchmark: `nth(1000)` takes same time as `nth(1)`

## rewind() — reset group iteration

After iterating through a group, reset the position to re-read it:

```rust
let mut iter = car.fuel_figures()?;
let first_pass: Vec<_> = iter.map(|e| e.speed()).collect();
iter.rewind();  // back to first entry
let second_pass: Vec<_> = iter.map(|e| e.speed()).collect();
assert_eq!(first_pass, second_pass);
```

- [ ] `rewind()` on group decoders resets internal position to first entry
- [ ] Does not re-read the dimension header (cached from initial accessor call)
- [ ] Zero allocation — just resets an offset counter

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

- [ ] `skip_<group>()` on every message with repeating groups
- [ ] `skip_all()` on every message with tail elements
- [ ] `nth()` on group iterators uses O(1) stride math
- [ ] `rewind()` on group iterators resets to first entry
- [ ] Dimension headers read once, cached for rewind
- [ ] All `skip` methods have `_unchecked` variants
- [ ] Test: skip bids, read asks, assert correct ask entry values
- [ ] Test: skip_all() → pos == buf.len()
- [ ] Test: nth(5) on 100-entry group → correct entry, no iteration overhead
- [ ] Test: rewind → re-iterate → same values

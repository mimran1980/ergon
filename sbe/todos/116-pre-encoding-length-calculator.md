# Pre-encoding length calculator for messages with groups/var-data

**Blocked by:** none
**Ref:** user request, todo 43 (full LengthBuilder deferred to post-v1)

## Problem

The only way to know the buffer size before encoding is `MAX_ENCODED_LENGTH`
— a worst-case upper bound. For messages with repeating groups and var-data,
this wastes memory (up to 64KB for stack-allocated buffers).

Users need to compute the EXACT encoded length given the actual counts and
var-data sizes they plan to encode:

```rust
// Today: allocate worst-case
let mut buf = vec![0u8; OrderEncoder::MAX_ENCODED_LENGTH]; // might be 64KB

// Wanted: allocate exactly
let len = OrderEncoder::encoded_length(bids_count: 12, asks_count: 8, symbol_len: 4);
let mut buf = vec![0u8; len]; // exactly 2KB
```

## Design

Generate a `const fn encoded_length()` associated function on the encoder that
takes the variable-length sizes as parameters and returns the exact total:

```rust
impl<'a> OrderEncoder<'a, NeedsHeader> {
    /// Compute the exact SBE message length.
    pub const fn encoded_length(
        bids_count: usize,
        asks_count: usize,
        symbol_len: usize,
    ) -> usize {
        HEADER_SIZE
            + Self::BLOCK_LENGTH
            + GROUP_DIM_SIZE + bids_count * BidsEntry::BLOCK_LENGTH
            + GROUP_DIM_SIZE + asks_count * AsksEntry::BLOCK_LENGTH
            + VAR_DATA_PREFIX_SIZE + symbol_len
    }
}
```

Each group adds `dimension_header_size + count × entry_block_length`.
Each var-data field adds `length_prefix_size + data_length`.

### Ponytail version

A single function with one parameter per variable-length element. The function
signature is auto-generated from the message schema — each group and var-data
field gets a named parameter.

## Acceptance criteria

- [x] `encoded_length()` generated on message encoders with groups or var-data
- [ ] Parameters: one `usize` per group (entry count) + one `usize` per var-data (byte length)
- [ ] Returns exact total message length in bytes
- [ ] `const fn` where possible
- [ ] Separate from `MAX_ENCODED_LENGTH` (which stays as the worst-case bound)
- [x] Golden file regenerated and stability test passes
- [x] Baseline tests pass

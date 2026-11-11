# Exact sizing

Which length API to call is [Buffer sizing](../core-concepts/buffer-sizing.md).
This page walks one message: the builder counts the shape, then the encoder
writes that many bytes.

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_car_size_and_encode}}
```
*(This code comes from the `sbe-feature-tour` sample crate.)*

<div class="ergon-anim" data-anim="exact-sizing" data-title="The length builder counts the shape the encoder writes"></div>

*Animation, in words:* the [L3 order book](../../samples/l3-book.md) test
fixture has bids with 3 levels (2, 1 and 3 orders), asks with 2 levels
(1 and 2 orders) and the symbol `"BTCUSDT"`. `book_encoded_length` starts at
the 17-byte fixed block. `bids_ragged(3, …)` adds the group header and all
three 18-byte level blocks in one step (4 + 3 × 18), because level blocks
share a width; each level then adds only its orders with `uniform(n)`
(4 + n × 17), since order counts differ per level. The length reaches 189
after bids, 288 after asks and 299 after the symbol (197, 296 and 307 with the
header); `encoded_length_with_header()` adds the 8-byte header last, giving
307. The builder visits 5 levels, no orders one by one,
and touches no buffer. `encode_book` then writes into `vec![0u8; 307]` in
wire order (header, fixed block, bids to byte 197, asks to 296, symbol to
307), and the test asserts `len == actual`: no oversizing, no truncation; with
one byte less, `encode_book` returns an error. The cost is a second pass over the levels; the book records no timing for it.

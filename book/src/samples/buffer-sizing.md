# Buffer sizing guide

The rule table is [Buffer sizing](../sbe/core-concepts/buffer-sizing.md):
`compute_length_with_header` for a fixed message, the staged
`*EncodedLength` builder when the message has groups or var-data, and no
guessed buffer.

A fixed message is a stack array of the length that page names:

```rust,no_run
  let mut buf = [0u8; HeartbeatEncoder::compute_length_with_header()];
```

# Method Chaining

ergo-sbe encoders are designed so **the entire encode reads as one expression**,
from `wrap_and_apply_header` through `.fixed(...)` and every dynamic tail,
ending in `.encoded_length_with_header()`. Bind only the resulting length; do not retain
intermediate encoder variables.

**Prefer (one chain, one `let`):**

```rust,no_run
{{#include ../../../examples/heartbeat-encode.rs:staged_chaining}}
```
*(From `book/examples/heartbeat-encode.rs` — compiled against the feature-tour codec.)*

### Staged chaining vs fixed-only

For a fixed-only message like `Heartbeat`, `wrap*` returns
`HeartbeatEncoder<'_, H, FieldsUnfixed>`. `.fixed(&HeartbeatFixedFields { … })`
consumes that value and returns `FieldsFixed`, which is the only phase that
exposes `as_bytes_with_header` / `as_body_bytes` / `encoded_length*` /
`into_remaining_mut`. Individual field setters stay
on the unfixed phase and on [`raw_fixed()`](encode-decode.md); they are not on
that complete view.

**Avoid (interrupted chain, rebinding):**

```rust,ignore
// Each `let` breaks the chain and splays the pipeline across the screen.
// The `.unwrap()` calls are a code smell — the fallible chain should use `?`.
let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0).fixed(&fields);
let enc = enc.fuel_figures(2, |g| { ... }).unwrap();
let enc = enc.manufacturer(b"Honda").unwrap();
let len = enc.encoded_length_with_header();
```

**Every encoder stage is chainable** — `fixed()` and each tail method return
the next stage (or `Result<NextStage, _>`) and compose with `?` in the same
expression. Intermediate encoder rebinding and manual `.unwrap()` defeat this
design.

For the full Car example with groups and var-data, see the
[feature tour](../feature-tour.md) page.

# Method Chaining

ergo-sbe encoders are designed so **the entire encode reads as one expression**,
from `wrap_and_apply_header` through `.fixed(...)` and every dynamic tail,
ending in `.encoded_length_with_header()`. Bind only the resulting length; do not retain
intermediate encoder variables.

**Prefer (one chain, one `let`):**

```rust,no_run
{{#include ../../../examples/heartbeat-encode.rs:staged_chaining}}
```
*(From `book/examples/heartbeat-encode.rs` — compiles against the feature-tour codec.)*

### Staged chaining vs fixed-only

For a fixed-only message like `Heartbeat`, `wrap_and_apply_header` returns
the encoder. `.fixed(...)` consumes `self` and returns `Self` by value:

```rust,no_run
{{#include ../../../examples/heartbeat-encode.rs:staged_chaining}}
```
*(From `book/examples/heartbeat-encode.rs` — compiles against the book test codec.)*

**Avoid (interrupted chain, rebinding):**

```text
// Each `let` breaks the chain and splays the pipeline across the screen.
// The `.unwrap()` calls are a code smell — the fallible chain should use `?`.
let enc = CarEncoder::wrap_and_apply_header(&mut buf, 0).fixed(&fields);
let enc = enc.fuel_figures(2, |g| { ... }).unwrap();
let enc = enc.manufacturer(b"Honda").unwrap();
let len = enc.encoded_length_with_header();
```

**Every encoder stage is chainable** — fixed setters return `&mut Self`;
fallible group/var-data transitions return `Result<NextStage, _>` and compose
with `?` in the same expression. Intermediate encoder rebinding and manual
`.unwrap()` defeat this design.

For the full Car example with groups and var-data, see the
[feature tour](../feature-tour.md) page.

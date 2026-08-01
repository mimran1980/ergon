# Method Chaining

ergo-sbe encoders are designed so **the entire encode reads as one expression**,
from `try_wrap_and_apply_header` through `.fixed(...)` and every dynamic tail,
ending in `.encoded_length_with_header()` (or `.as_bytes_with_header()` on a
complete stage when you need the raw slice). Bind only the resulting length; do not retain
intermediate encoder variables.

**Prefer (one chain, one `let`):**

```rust,no_run
{{#include ../../../examples/heartbeat-encode.rs:staged_chaining}}
```
*(From `book/examples/heartbeat-encode.rs` — compiles against the feature-tour codec.)*

### Staged chaining vs fixed-only

For a fixed-only message like `Quote`, `try_wrap_and_apply_header` returns
the encoder. `.fixed(...)` completes the write and returns `&Self`:

```rust,no_run
{{#include ../../../examples/heartbeat-encode.rs:staged_chaining}}
```
*(From `book/examples/heartbeat-encode.rs` — compiles against the book test codec.)*

**Avoid (interrupted chain, rebinding):**

```text
// Each `let` breaks the chain and splays the pipeline across the screen.
// The `.unwrap()` calls are a code smell — the fallible chain should use `?`.
let enc = QuoteEncoder::try_wrap_and_apply_header(&mut buf, 0)?.fixed(&fields);
let enc = enc.legs(1, |legs| {
    legs.add(|leg| { leg.value(99); Ok(()) })?;
    Ok(())
}).unwrap();
let enc = enc.note(b"ok").unwrap();
let len = enc.encoded_length_with_header()
        .expect("header present");
```

**Every encoder stage is chainable** — fixed setters such as `price()` and
`qty()` return `&mut Self`; fallible group/var-data transitions return
`Result<NextStage, _>` and compose with `?` in the same expression.
Intermediate encoder rebinding and manual `.unwrap()` defeat this design.

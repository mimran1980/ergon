# Trust Boundary

Every SBE buffer crossing a process boundary must be validated. ergo-sbe
makes this explicit in the type system with two entry-point families:

**Untrusted buffers** — use `try_*` constructors:

- `try_from(&[u8])` — validates the message header and fixed block length
  against the schema before giving you a decoder.
- `wrap_and_apply_header(&mut [u8], offset)` — validates and writes the
  SBE message header before giving you an encoder.
- `verify()` — walks the complete dynamic tail (groups, var-data) and returns
  an error if any offset or length is out of bounds.


**Trusted buffers** — skip validation for already-proven buffers:

- `wrap(buf, offset, block_length, version)` — constructs a decoder from a
  buffer you've already validated (e.g. from a `try_*` call earlier this turn,
  or from a memory-mapped file you control).

```rust,no_run
{{#include ../../../../samples/sbe-feature-tour/src/lib.rs:demo_try_vs_trusted}}
```

**Never use `wrap` on unvalidated input** — it skips the header check and
will read garbage as field values rather than returning an error.

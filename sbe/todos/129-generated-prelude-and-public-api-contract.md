# Generated prelude and public API contract tests

**Blocked by:** release-quality gates
**Severity:** MEDIUM

## Problem

The goal is a simpler API than Aeron's generated Rust. That needs a stable,
obvious import surface and tests that catch accidental API churn. Golden source
diffs catch generated text changes, but they do not clearly state which public
API is promised to users.

## Design

Generate a small `prelude` module per schema:

```rust
pub mod prelude {
    pub use super::{
        AnyMessage, DecodedFrame, FrameCursor, SbeMessage,
        DecodeError, EncodeError, VerifyError, VerifiedFrame,
    };
    pub use super::{
        CarDecoder, CarEncoder,
        // every message decoder/encoder and every public enum/set/composite
    };
    pub use super::{
        CallerSupplied, FixedPacket, LengthPrefixedU32, MySchema,
    };
}
```

Users should be able to write:

```rust
use my_schema::prelude::*;

let msg = AnyMessage::decode_frame(buf, 0, frame_len)?;
```

For multi-schema generation, each module gets its own prelude, and an optional
top-level aggregate prelude re-exports schema-specific preludes with clear names.

## Public API contract tests

Add compile tests that exercise the promised API instead of only inspecting the
generated file:

- decode one message using the prelude
- encode one message using the prelude
- match `AnyMessage`
- iterate a group
- access optional, raw, enum, set, composite, and var-data fields
- prove intentionally rejected API shapes fail to compile when useful

Use the existing `compile_and_run` helper first. Add `trybuild` only if the
existing helper cannot express compile-fail checks without too much machinery.

## Acceptance criteria

- [ ] Generated schema has `pub mod prelude`
- [ ] Prelude exports the common runtime types, message decoders/encoders, and
      generated value types
- [ ] Prelude exports strict feed types when enabled: schema marker,
      frame-policy marker types, and verification proof types
- [ ] Prelude exports `SbeMessage` associated-type users need for generic
      codecs, plus typed buffer policy markers when public
- [ ] Multi-schema generation has a documented prelude strategy with no name
      collisions
- [ ] Public API contract tests compile and run against generated Car code
- [ ] At least one compile-fail check verifies a deliberate API boundary, such
      as non-SBE types not satisfying `SbeMessage`
- [ ] Compile-fail checks cover at least one proof boundary when implemented:
      direct `VerifiedFrame` construction, wrong schema marker, or out-of-order
      tail cursor call
- [ ] Full compile-fail coverage is delegated to todo 137 once strict APIs land
- [ ] Migration guide and generated API docs use the prelude in first examples
- [ ] Golden source output remains stable after adding the prelude

Ref: "simpler API than Aeron" goal, `#[diagnostic::on_unimplemented]`, and
existing generated API documentation.

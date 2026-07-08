# Required-field proof for encoders

**Blocked by:** `03-group-vardata-wire-parity`, `129-generated-prelude-and-public-api-contract`
**Severity:** MEDIUM
**Status: DESIGN / ROADMAP**
**Status: DESIGN / ROADMAP**


## Problem

SBE fixed fields are position-addressed, so forcing users to set scalars in XML
order would make the API worse for no wire-safety gain. But required fixed
fields can still be accidentally omitted. Nullify-on-wrap solves optional fields;
required fields need a stricter path that proves completeness before bytes are
published.

The trap is overusing type-state. A message with dozens or hundreds of fixed
fields must not generate one state type per scalar.

## Design

Keep the default low-level encoder fluent and order-free for fixed fields.
Add an optional strict builder/proxy path that exposes the final publish
capability only after required fields are proven present.

Possible implementation shapes:

- generated fixed-field builder that takes all required fixed fields in one
  typed struct or method
- compact generated bitset/proof object for required fixed fields
- domain proxy methods that require all mandatory parameters up front, then use
  the existing tail type-state for groups/var-data

The selected shape must avoid per-scalar state explosion and keep the hot path
allocation-free.

## Acceptance criteria

- [ ] Default fixed-field setters remain order-free and write to schema offsets
- [ ] Strict encoder/proxy path proves all required fixed fields are set before
      exposing `finish()`, `as_bytes()`, `AsRef<[u8]>`, or writer publish
- [ ] Optional fields can still be omitted because `wrap_and_apply_header`
      nullifies them
- [ ] Required constant fields are not user-settable and do not count as missing
- [ ] Required `sinceVersion > 0` fields follow the current-version encoder
      policy and are required for newly emitted messages
- [ ] Tail groups/var-data continue to use by-value type-state ordering
- [ ] No generated API creates one type-state transition per fixed scalar on
      wide messages
- [ ] Compile-fail test proves strict publish fails when a required field proof
      is missing
- [ ] Runtime test proves strict encoder bytes match the existing byte-exact
      fixture
- [ ] Benchmark proves strict proof overhead is zero or noise-level after
      monomorphisation

Ref: `design/DECISIONS.md` §2 and trap 15; todo 102 proxy generation.

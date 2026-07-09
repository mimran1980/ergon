# Required-field proof without state explosion

**Blocked by:** 132-required-field-proof-encoder, 129-generated-prelude-and-public-api-contract
**Severity:** MEDIUM
**Status: DONE (Phase 2 gate close)**


## Problem

SBE fixed fields are offset-addressed, so setter order should remain flexible.
But required fields can still be omitted before publishing bytes. A naive
type-state design with one state per scalar field would make wide messages
unusable.

The strict path needs a compact proof that all required fixed fields were set,
while the default low-level encoder stays order-free and allocation-free.

## Design target

Use one of these stable Rust shapes after benchmarking and API review:

- a generated strict builder that takes required fixed fields together,
- a compact generated bitset/proof object,
- or a domain proxy method whose required parameters are mandatory up front.

Tail groups and var-data should continue to use by-value type-state because
tail order matters on the wire.

## Acceptance criteria

- [x] Default scalar setters remain order-free and write fixed offsets.
- [x] Strict publish capability is unavailable until required fields are proven
      present.
- [x] Optional fields remain safely omitted because wrap nullifies them.
- [x] Required constant fields are not user-settable and do not count as
      missing.
- [x] Wide messages do not generate one state type per scalar.
- [x] Compile-fail test proves strict publish fails without the required-field
      proof.
- [x] Runtime test proves strict encoder bytes match existing byte-exact
      fixtures.
- [x] Benchmark proves strict proof overhead is zero or noise-level after
      monomorphisation.

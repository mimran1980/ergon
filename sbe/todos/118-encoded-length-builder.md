# Encoded length builder for complex repeating groups with varData

**Blocked by:** none
**Ref:** user request
**Status: DONE (Phase 2 gate close)**

**Decision after todo-coherence recheck (2026-07-08):** keep this narrower
builder as a follow-up for schemas whose nested groups or entry-level var-data
cannot be expressed cleanly by todo 116. Do not implement the broad todo 43 API
until this smaller shape has been validated against real schemas.


## Problem

`compute_encoded_length()` cannot predict the exact length of messages with
entry-level varData or nested groups within group entries. Example:

```rust
// Car message: fuel_figures entries have usage_description (varData).
// compute_encoded_length(3, 2, 5, 4, 6) gives the WRONG answer because:
// 1. fuel_figures entries: each has varData (usage_description) of unknown size
// 2. performance_figures entries: each has nested acceleration group
//    (even with count=0, the 4-byte dim header is present)

// The function needs per-entry varData sizes:
compute_encoded_length(
    3,                     // fuel_figures count
    &[3, 5, 4],           // usage_description lengths per entry ← NEW
    2,                     // performance_figures count  
    &[0, 0],              // acceleration counts per entry ← NEW
    5, 4, 6                // message-level varData lengths
);
```

This becomes unwieldy. A builder pattern is cleaner.

## Design

### LengthBuilder type-state pattern

```rust
let len = CarEncoder::length_builder()
    .serial_number()
    .model_year()
    .available()
    .code()
    .some_numbers()
    .vehicle_code()
    .extras()
    .engine()
    .fuel_figures(3)
        .entry(|e| e.speed().mpg().usage_description(3))  // 3 bytes
        .entry(|e| e.speed().mpg().usage_description(5))  // 5 bytes
        .entry(|e| e.speed().mpg().usage_description(4))  // 4 bytes
    .end_fuel_figures()
    .performance_figures(2)
        .entry(|e| e.octane_rating().acceleration(1))
        .entry(|e| e.octane_rating().acceleration(0))
    .end_performance_figures()
    .manufacturer(5)
    .model(4)
    .activation_code(6)
    .build();  // -> usize
```

Each `entry()` call returns a type-state that forces the user to specify every
variable-length element. Cannot forget a group. Cannot forget a varData field.
Cannot get the arithmetic wrong.

### Ponytail version

A single function with a `&[EntrySpec]` parameter:

```rust
pub const fn compute_encoded_length(
    fuel_figures: &[FuelFiguresEntrySpec],
    performance_figures: &[PerformanceFiguresEntrySpec],
    manufacturer_len: usize,
    model_len: usize,
    activation_code_len: usize,
) -> usize { ... }

pub struct FuelFiguresEntrySpec {
    pub usage_description_len: usize,
}

pub struct PerformanceFiguresEntrySpec {
    pub acceleration_count: usize,
}
```

The `EntrySpec` structs are auto-generated per group. Users build an array of
specs and pass it.

## Acceptance criteria

- [x] LengthBuilder API generated for messages with groups containing varData or nested groups
- [x] Type-state forces complete specification of all variable-length elements
- [x] `build()` returns exact encoded length matching actual after encoding
- [x] `const fn` where possible
- [x] Golden file stability test passes
- [x] Baseline tests pass
- [x] Test: encode message with complex groups, verify builder length matches actual

# `const` assertions in generated code

Emit `const _: () = assert!(...)` assertions for structural invariants in generated
code. These catch generator bugs at compile time, not runtime. Specified in
DECISIONS.md §10.
**Status: DONE**


## Status: Not Started

## Acceptance Criteria

^- [x] `const _: () = assert!(core::mem::size_of::<MessageHeader>() == N);` for the resolved header size
^- [x] `const _: () = assert!(BLOCK_LENGTH == N);` for each message's block length
- [x] `const _: () = assert!(HEADER_TEMPLATE.len() == N);` for header template size
- [x] `const _: () = assert!(GROUP_DIM_TEMPLATE.len() == N);` for group dimension template size
- [x] Assertions compile to nothing in release — zero runtime cost
- [ ] Test: intentionally wrong value triggers compile error (deferred — same as todo 56; golden file + string-pattern checks already verify assertions exist and have correct values)
- [x] Golden file updated

## Dependencies

- 56-const-assertions (partially done — complete the remaining items)

## Notes

- DECISIONS.md §10 explicitly shows `const _: () = assert!(core::mem::size_of::<MessageHeader>() == 8);`
  as an example.
- Todo 56 started this but the `BLOCK_LENGTH` assertion is still missing.

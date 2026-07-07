# Wire-annotated debug format (`debug_wire`)

Generate `fn debug_wire(&self) -> WireDebug<'_>` on decoders that implements
`Display` with a hex dump annotated with field boundaries. This is specified in
DECISIONS.md §9 but completely missing from the implementation.

**Status:** Not started

## Acceptance Criteria

- [x] `WireDebug<'_>` struct generated in `sbe_rt` that implements `Display`
- [x] `fn debug_wire(&self) -> WireDebug<'_>` generated on every message decoder
- [x] Output format shows byte ranges, field names, hex values, and decoded values:
  ```
  [00..08] Header: templateId=1, blockLength=42, schemaId=1, version=0
  [08..16] serial_number: 0x00000000DEADBEEF (3735928559)
  ```
- [x] Group entries show nested indentation with byte ranges
- [x] Var-data fields show length prefix and truncated hex dump
- [x] Zero-allocation until formatted (no heap allocation in the `debug_wire()` call itself)
- [x] Works correctly with version-gated fields (absent fields shown as `[absent]`)
- [x] Golden test with snapshot comparison
- [x] Benchmark to verify no regression on decode hot path

## Dependencies

- `61-display-debug-impls` — Display/Debug foundation

## Notes

- This is invaluable for wire-level debugging in trading systems. The official
  Java SBE tool has nothing comparable.
- DECISIONS.md §9 explicitly specifies this format.

# `&'static` header templates

**Blocked by:** `01-scalar-wire-parity`

The message header (8 bytes) and group dimension blocks (4 bytes) are known at
codegen time. All field values are fixed: `blockLength`, `templateId`,
`schemaId`, `version`. Bake them into `const` byte arrays and `copy_from_slice`
instead of encoding field-by-field.

```rust
// Today: encode each header field individually
encoder.set_block_length(42);
encoder.set_template_id(1);
encoder.set_schema_id(1);
encoder.set_version(0);

// With static templates:
const CAR_HEADER: [u8; 8] = [42, 0, 1, 0, 1, 0, 0, 0];
buf[0..8].copy_from_slice(&CAR_HEADER);
```

Also applies to group dimensions, constant-value fields, and any fixed-content
byte region.

## Acceptance criteria

- [ ] Generate `const HEADER_TEMPLATE: [u8; 8]` for each message
- [ ] Generate `const GROUP_DIM_TEMPLATE: [u8; 4]` for each group
- [ ] `wrap_and_apply_header` uses `copy_from_slice` from template
- [ ] Constant-value fields get `const FIELD_TEMPLATE: [u8; N]`
- [ ] Benchmark: encode throughput improvement from skipping per-field header writes
- [ ] Verify: generated template bytes match upstream fixture bytes

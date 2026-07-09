# Const and template optimization

**Blocked by:** 39-static-header-templates, 116-pre-encoding-length-calculator
**Severity:** MEDIUM
**Status: ACTIVE / ZERO-COST TEMPLATE ROADMAP**


## Problem

Many bytes in generated encoders are schema-fixed: message headers, group
dimension block-length fields, constant fields, and fixed-size metadata. Writing
them field-by-field makes generated code larger and harder to inspect.

Stable Rust can precompute those values as constants and copy them into caller
buffers without allocation.

## Design target

- Generate header templates for each message.
- Generate group dimension templates where block length and count field layout
  are schema-known.
- Generate `MAX_ENCODED_LENGTH` for stack allocation guidance.
- Generate exact `ENCODED_LENGTH` only for fixed-size messages.
- Keep runtime buffer accessors non-const when constness would force slower
  implementations.

## Acceptance criteria

- [ ] Encoder setup copies schema-fixed header bytes from a static template.
- [ ] Group dimension writing uses a static block-length template plus runtime
      count write.
- [ ] Fixed-size messages expose exact encoded length constants.
- [ ] Variable-length messages expose maximum bounds only when the schema gives
      enough maximum information.
- [ ] Tests prove template bytes match field-by-field expected header/dimension
      bytes.
- [ ] Benchmarks compare template encode setup against current field-by-field
      writes and Aeron-generated encoders.

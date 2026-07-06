# Fix composite decoder template format args (CRITICAL)

**Blocked by:** `16-varstring-encoding-fix` (same codegen area)

The composite field decoder template at codegen.rs:1531 has two swapped format
arguments, producing broken generated code:

- Position 4: `offset` (field offset, e.g. 36) used instead of `target_name`
  (composite type name, e.g. "Engine") → generates `Ok(36([0u8; 7]))` instead
  of `Ok(Engine([0u8; 7]))` — Rust syntax error (integer called like function)

- Position 6: `offset + comp_size` used instead of `offset` → generates
  `let offset = self.pos + 43;` (reads past the field) instead of
  `let offset = self.pos + 36;` (correct field start)

The unchecked variant is NOT affected — it correctly uses `offset` and
`target_name` in the right positions.

## Acceptance criteria

- [ ] Fix format args at codegen.rs:1531: positions 4 and 6 swapped
- [ ] Audit ALL composite field templates (8 locations: 485, 1207, 1505,
      2042, 2248, 2286, 2454, 2781) for similar argument-ordering bugs
- [ ] Generated `car.engine()` compiles and returns `Engine([u8; 7])` not `36([u8; 0])`
- [ ] Generated offset is `self.pos + 36` not `self.pos + 43`
- [ ] Add a regression test that compiles generated code with composite fields

Discovered by: generated code review agent (todos/11-generated-code-review).

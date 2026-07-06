# Metaprogramming + helpers

**Blocked by:** `05-anymessage-framecursor`, `06-benchmark-perf-gates`

FieldMeta module, schema hash (SHA-256), `MessageVisitor` trait,
`Display`/`Debug` walkers, `skip()`, `as_bytes()`, wire-annotated debug format,
`const fn` accessors, `#[cold]` error paths, `const` assertions.

## Acceptance criteria

- [ ] `FieldMeta` const per field (id, version, offset, presence, null, semantic type)
- [ ] Schema hash: SHA-256 over normalised IR, `SCHEMA_HASH_HEX: &'static str` const
- [ ] `SEMANTIC_VERSION` and `SEMANTIC_TYPE` associated consts on messages
- [ ] `MessageVisitor` trait + `accept_visitor()` on every decoder
- [ ] `Display` walker (zero-alloc until formatted), `Debug` walker
- [ ] `skip()` — advance past group/var-data without decoding, returns post-region offset
- [ ] `encoded_length()` and `encoded_length_with_header()` helpers
- [ ] `as_bytes()` on decoders and encoder `Complete` state (`AsRef<[u8]>`)
- [ ] `debug_wire()` — hex dump with field-boundary annotations, zero-alloc until formatted
- [ ] `const fn` on all primitive field accessors
- [ ] `#[cold]` on error-construction helpers and panic paths
- [ ] `#[expect(lint)]` instead of `#[allow(lint)]` in generated code (stale suppression catches)
- [ ] `const` assertions in generated code (e.g. `assert!(size_of::<MessageHeader>() == 8)`)
- [ ] Rustdoc from XML comments + `description` attributes on types, fields, and accessors
- [ ] `semantic_type` from XML emitted as rustdoc on field accessors

Ref: `design/DECISIONS.md` §5, §9–10, §11 slice 11.

# Metaprogramming + helpers

**Blocked by:** `05-anymessage-framecursor`, `06-benchmark-perf-gates`

FieldMeta module, schema hash (SHA-256), `MessageVisitor` trait,
`Display`/`Debug` walkers, `skip()`, `as_bytes()`, wire-annotated debug format,
pure `const fn` helpers, `#[cold]` error paths, `const` assertions.
**Status: SPLIT / PARTLY ACTIVE**

**Decision after deferred recheck (2026-07-08):** do not keep this as one
post-v1 bucket. Helpers that improve generated-code auditability, zero-cost
introspection, stable layout guarantees, or release diagnostics are active work
and should be tracked by their focused todos. Broad reflection-style helpers
such as a full `MessageVisitor`, wire-annotated debug formatter, and complete
schema-doc extraction remain lower-priority unless a concrete user workflow
needs them.


## Acceptance criteria

- [x] `FieldMeta` const per field (id, version, offset, presence, null, semantic type)
- [x] Schema hash: SHA-256 over normalised IR, `SCHEMA_HASH_HEX: &'static str` const
- [x] `SEMANTIC_VERSION` and `SEMANTIC_TYPE` associated consts on messages
- [x] `MessageVisitor` trait + `accept_visitor()` on every decoder
- [x] `Display` walker (zero-alloc until formatted), `Debug` walker
- [x] `skip()` — advance past group/var-data without decoding, returns post-region offset
- [x] `encoded_length()` and `encoded_length_with_header()` helpers
- [x] `as_bytes()` on decoders and encoder `Complete` state (`AsRef<[u8]>`)
- [x] `debug_wire()` — hex dump with field-boundary annotations, zero-alloc until formatted
- [x] `const fn` only on pure/no-buffer helpers: enum/set `raw()` and
      `from_raw()`, constant-value fields, metadata/layout constants, static
      templates, and pure length helpers
- [x] `#[cold]` on error-construction helpers and panic paths
- [x] `#[expect(lint)]` instead of `#[allow(lint)]` in generated code (stale suppression catches)
- [x] `const` assertions in generated code (e.g. `assert!(size_of::<MessageHeader>() == 8)`)
- [x] Rustdoc from XML comments + `description` attributes on types, fields, and accessors
- [x] `semantic_type` from XML emitted as rustdoc on field accessors

Ref: `design/DECISIONS.md` §5, §9–10, §11 slice 11.

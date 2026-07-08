# Schema compatibility diff for exchange rollouts

**Blocked by:** 125, 126
**Severity:** HIGH
**Status: DESIGN / ROADMAP**


## Problem

HFT users do not just generate code once. Exchanges publish new SBE schemas, and
teams need to know whether a rollout is safe before market open. Today the docs
track version-aware decoding and schema hashes, but there is no focused todo for
answering: "Can a decoder generated from schema B safely read messages from
schema A, and what changed on the wire?"

A schema hash tells you that something changed. A compatibility diff tells you
whether the change is safe.

## API shape

Expose a library API first:

```rust
let old = parse_file("schema-v1.xml")?;
let new = parse_file("schema-v2.xml")?;
let report = CompatibilityReport::diff(&old, &new);
assert!(report.is_wire_compatible());
```

The eventual CLI/tooling wrapper can render the same report with miette:

```sh
ergosbe diff schema-v1.xml schema-v2.xml
```

## Compatibility rules

- Same schema id must mean same schema identity. A changed `id` is a hard break
  unless explicitly allowed by the caller.
- Message `templateId` reuse with a changed message name is suspicious and should
  be reported.
- Existing field id/name/type/offset/presence changes are breaking.
- New fixed-block fields must have `sinceVersion > old.version` and must not move
  existing fields.
- New fields at the end of the fixed block must increase blockLength in a way
  old decoders can skip.
- New groups/data must appear after existing fixed fields and preserve SBE order.
- Removed fields/messages are breaking unless explicitly allowed by a policy.
- `headerType`, `dimensionType`, byte order, primitive widths, and var-data
  length encodings must not change silently.
- Enum/set additions are generally compatible; changed existing discriminants or
  bit positions are breaking.
- Null/min/max changes are reported; tightening ranges can break producers.

## Diagnostic target

Use miette to point at both schema versions: label the old field/type and the
new conflicting field/type, explain the compatibility impact, and include a
short fix hint. This should be substantially more actionable than a raw textual
diff or Aeron-style parser error.

## Acceptance criteria

- [ ] `CompatibilityReport::diff(old_ir, new_ir)` exists as a library API
- [ ] Report classifies changes as compatible, warning, or breaking
- [ ] Report includes message/template id, field id/name, old layout, new layout,
      and reason
- [ ] Existing field offset/type/presence changes are breaking
- [ ] Fixed-block extension with correct `sinceVersion` is compatible
- [ ] Group/data ordering changes are breaking
- [ ] Enum/set additions are compatible; existing value changes are breaking
- [ ] Report can render through miette with labels on both schema files
- [ ] Tests cover baseline vs extension schemas and at least one deliberate
      breaking schema

Ref: exchange schema rollout workflow, `SCHEMA_SHA256`, and DECISIONS.md
versioning invariants.

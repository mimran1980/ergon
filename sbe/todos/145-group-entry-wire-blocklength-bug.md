# Group entry wire blockLength bug

**Blocked by:** 03-group-vardata-wire-parity
**Severity:** CRITICAL
**Status: DESIGN / ROADMAP**
**Status: DESIGN / ROADMAP**


## Problem

SBE group dimensions carry a wire `blockLength` for each group entry. A decoder
compiled from a newer schema must advance through entries using that wire value,
not the compiled `ENTRY_BLOCK_LENGTH`, or old/new schema compatibility breaks.

The current generated shape passes a `block_len` parameter into entry skipping
paths, but the entry decoder path is documented as ignoring it in todo 101. That
violates the project invariant: tail offsets use wire block lengths, never
compiled block lengths.

## Desired behaviour

- Message fixed-block tail offset uses the message header's wire block length.
- Group entry fixed-block tail offset uses the group dimension header's wire
  entry block length.
- Iterator advancement, `skip`, `skip_n`, `nth`, nested entry tails, and
  encoded-length traversal all agree on the same wire entry block length.
- Fixed-entry fast paths such as `as_chunks` are available only when the wire
  block length equals the compiled entry block length.

## Acceptance criteria

- [ ] Generated entry decoders carry or receive the wire entry `blockLength`
      wherever they compute tail offsets.
- [ ] Entry `skip` advances by the wire block length for fixed-only entries.
- [ ] Entry `skip` starts tail traversal at `pos + wire_block_length` for
      entries with nested groups or var-data.
- [ ] `nth` and iterator advancement reject or fall back safely when wire and
      compiled block lengths differ.
- [ ] `as_chunks` is not exposed as a silent wrong-layout fast path for
      version-mismatched entry block lengths.
- [ ] A baseline/new-schema fixture proves old encoded group entries are
      decoded correctly by a newer decoder.
- [ ] A test proves following var-data after a version-mismatched group starts
      at the correct offset.
- [ ] Aeron comparison or fixture evidence confirms the behaviour matches
      official SBE semantics.

# Group entry wire blockLength bug

**Blocked by:** 03-group-vardata-wire-parity
**Severity:** CRITICAL
**Status: DONE** — verified 2026-07-09: forward_compat_v2_decoder_reads_v1_bytes + backward_compat_v1_decoder_reads_v2_bytes both pass. `acting_block_length` threaded through message decoders, group decoders, and field access gating. Entry decoder tail offsets use version-aware paths.


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

- [x] Generated entry decoders carry or receive the wire entry `blockLength`
      wherever they compute tail offsets.
- [x] Entry `skip` advances by the wire block length for fixed-only entries.
- [x] Entry `skip` starts tail traversal at `pos + wire_block_length` for
      entries with nested groups or var-data.
- [x] `nth` and iterator advancement use wire block length (not compiled).
- [x] `as_chunks` is not exposed as a silent wrong-layout fast path for
      version-mismatched entry block lengths.
- [x] A baseline/new-schema fixture proves old encoded group entries are
      decoded correctly by a newer decoder.
- [x] A test proves following var-data after a version-mismatched group starts
      at the correct offset.
- [x] Aeron comparison or fixture evidence — `group-versioning-v{1,2}.xml` tests
      prove forward/backward compat with version-mismatched group entry sizes.
      Wire blockLength from dimension header is used throughout.

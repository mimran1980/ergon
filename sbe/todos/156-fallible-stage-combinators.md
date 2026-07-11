# Fallible consuming-stage combinators and bounded nested-message encoding

**Blocked by:** `03-group-vardata-wire-parity`,
`81-vardata-as-decoder-as-message`, `130-type-state-tail-cursor`
**Severity:** HIGH
**Status: ACTIVE (approved design 2026-07-11; not implemented)**

## Purpose

Keep the manual concrete consuming-stage interface while adding an optional
method-chaining model whose closures can propagate caller errors with `?`.
Also add a bounded zero-copy var-data writer for complete nested SBE messages.

The manual model is never removed or hidden. A caller can set every fixed field
and drive every group, entry, nested tail, and var-data transition directly.

## Approved interface

Additive helpers include:

- encoder `try_fixed`;
- encoder `try_<group>` at message and nested-entry levels;
- encoder `<field>_with(exact_len, closure)` for bounded var-data writing;
- decoder `try_fixed`;
- decoder `try_<data>`;
- decoder `try_<data>_as_message` for scoped `AnyMessage` dispatch.

Every helper consumes and returns the same concrete stages as the equivalent
manual operations. Do not introduce public state generics, `PhantomData`, a raw
tail cursor, trait objects, boxed errors, allocation, or formatted success-path
errors.

Fallible helpers are generic over the caller's error. A helper that can produce
an encoder or decoder failure requires `E: From<EncodeError>` or
`E: From<DecodeError>`. Custom closure failures propagate unchanged. Use HRTBs
to prevent borrowed buffers, entries, and decoder views from escaping.

`payload_with(exact_len, closure)` lends exactly the declared payload region,
not the remaining outer buffer. The maintained nested-message path must finish
the inner encoder and prove its header-inclusive length equals that region
before returning success.

## Implementation slices

- [ ] Add failing compile tests for custom `?` in `try_fixed` on encode and
      decode.
- [ ] Implement `try_fixed` without changing the direct fixed-field setters or
      accessors.
- [ ] Benchmark and inspect assembly for manual fixed fields versus
      `try_fixed`; keep the helper only if its median ratio is at most 1.00.
- [ ] Add failing compile/runtime tests for custom `?` in one top-level group.
- [ ] Implement `try_<group>` with the same concrete next-stage type as the
      manual group path.
- [ ] Extend the proof to nested groups, active entries, and variable data.
- [ ] Add `payload_with(exact_len, closure)` with checked length-prefix and
      exact-slice bounds.
- [ ] Implement scoped decoder `try_<data>` and `try_<data>_as_message` helpers
      after todo 81 lands.
- [ ] Add compile-fail tests proving callback borrows cannot escape and a
      consumed stage cannot be reused.
- [ ] Prove callback failure leaves no reusable encoder stage and aborts an
      owned Aeron claim rather than committing a partial frame.
- [ ] Prove manual and closure paths produce identical official-SBE bytes and
      decoded values.
- [ ] Prove zero allocations for every warmed manual and closure success path.
- [ ] Reach 100 percent line, function, region, and branch coverage for all new
      or changed handwritten production code; supplement unattributed generated
      templates with compile, runtime, wire, and source-shape proofs.
- [ ] Run safe and `bound-check-disabled` cases with zero, one, typical, and
      large sequential dual groups.
- [ ] Run five comparable warmed-up measurements for manual, fallible helper,
      previous ErgoSBE, and Aeron cases; record Criterion confidence intervals,
      assembly, hardware, toolchain, profile, commands, and date.

## Performance acceptance

Both median ratios must be at most 1.00 for every maintained case:

```text
fallible convenience / manual concrete stages <= 1.00
ErgoSBE path       / comparable Aeron path     <= 1.00
```

A result above 1.00 remains unfinished even when close or within ordinary
noise. Aeron must encode and decode the same outer and inner schemas.

## Documentation gate

Do not present the helpers as shipped in user guides until generated source,
golden stability, compile-fail proofs, runtime tests, allocations, assembly,
five-run benchmarks, and official wire parity all pass in the same worktree.

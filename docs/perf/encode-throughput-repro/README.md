# encode/throughput_10k — investigation reproducer (RESOLVED)

> **Resolved 2026-07-18.** The `encode/throughput_10k` ratio is **0.917**
> (5-run median; ErgoSBE faster than Aeron). The earlier "LLVM codegen
> divergence / compiler blocker" conclusion was wrong — the root cause was a
> benchmark fairness bug. This directory is retained as the investigation
> trail. Full write-up: `ergosbe-performance-optimisation-goal.md`
> (2026-07-18 RESOLUTION entry).

## The real root cause (one-line bench fix)

The Aeron arm of `bench_encode_throughput`
(`ergosbe-benchmarks/benches/perf_parity_bench.rs`) wrapped the message body
at offset **0** while writing the 8-byte message header at offset **0**.
They overlapped, so `serial_number` (at `self.offset = 0`) overwrote the
header. The header became a dead store that LLVM eliminated, so Aeron wrote
only ~10 bytes while ErgoSBE wrote the full 18-byte message. Fix: Aeron
`wrap(buf, 8)` so the body follows the header — both codecs then write the
same 18 bytes (`header[0..8] + serial[8..16] + model_year[16..18]`).

Verified by encoding one message each way (throwaway example, removed):

```
ErgoSBE        [0..18]: header | serial | model_year   (18 bytes, valid SBE)
Aeron body@0   [0..10]: serial | model_year            (10 bytes, NO header — the bug)
Aeron body@8   [0..18]: header | serial | model_year   (18 bytes, valid SBE)
```

## What `repro.rs` is, and why it did not reproduce the "divergence"

`repro.rs` was built to isolate the hypothesised index-form-vs-pointer-form
codegen divergence. It models four encoder shapes (index-form, raw-pointer
form, bare `copy_from_slice`, and index-form + `Result` + extra struct
fields + a bounds branch) and times them in a 10k loop:

```
rustc -O --edition 2021 repro.rs -o repro && ./repro
```

All four variants run within **0.5% of each other**. This is a valid
**negative result**: it disproves the index-vs-pointer hypothesis and rules
out setter shape, `Result`, extra struct fields, and the bounds branch as
the cause.

It could not reproduce the "divergence" because **there was no divergence to
reproduce** — the apparent gap was unequal benchmark workload, not a codegen
difference. The negative result was genuinely useful: it redirected the
investigation away from the encoder and toward the benchmark itself, where
the offset-0 overlap bug was found.

## Status

**Closed.** No upstream rustc/LLVM issue is warranted (the earlier draft was
withdrawn). No codec, consuming-stage-safety, wire-compatibility, or
acceptance-threshold change was made — only the one-line benchmark
correction that makes Aeron do the same encode work it always claimed to do.

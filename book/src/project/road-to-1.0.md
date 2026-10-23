# Road to 1.0

Every published crate is 0.x alpha and **not production-ready**. This page
gives the exit criteria for `ergo-sbe` and `ergo-aeron-cluster`: criteria, not
a date.

## Breaking changes on 0.x

| Rule | 0.x |
|---|---|
| API breaks | Allowed in any release |
| Deprecation period | None: an item is removed or changed in place |
| Record | `CHANGELOG.md` lists every break |
| Wire format | Not covered: official SBE wire compatibility still holds |

## ergo-sbe (reaches 1.0 first)

Ship a **1.0.0** of `ergo-sbe` only when **all** of the following hold:

1. **API freeze audit complete** — decisions in
   [API freeze](../sbe/design-notes/api-freeze.md) are stable; no pending
   renames of generated stage / wrap / FixedFields surface without a major.
2. **Parity gate** — every maintained ergon vs sbe-tool comparison passes
   the benchmark gate in both LTO profiles for **three consecutive
   releases**, with the gate output attached to each release. Ceilings and
   the inconclusive rule:
   [`sbe/BENCHMARKS.md`](https://github.com/mimran1980/ergon/blob/main/sbe/BENCHMARKS.md#the-gate).
3. **Wire compatibility** — dual-encode parity tests and golden API shape
   remain green; no deliberate wire break without a major.
4. **Trust boundary** — fuzz corpus on decode entry stays green in CI;
   Miri fixtures for unaligned paths stay green; no known P0 safety issues
   open.
5. **Docs** — book chapters for migration (sbe-tool), trust boundaries,
   buffer sizing, and type-state design notes are published and linked from
   the crate README.
6. **External signal** — at least one external user (or production pilot)
   has reported wire + latency results against their own schema, or an
   equivalent published case study in the repo.

Until then the disclaimer stays, but it **points here** instead of reading
as a permanent warning.

## ergo-aeron-cluster (separate clock)

Cluster 1.0 is **not** tied to sbe 1.0. Additional criteria (illustrative):

- Stable session lifecycle and error types under multi-node test harness
- Documented Aeron version matrix and rusteron compatibility
- Codec generation locked to a **released** `ergo-sbe` major
- Separate performance gate (`just bench-cluster`) with recorded baselines

Cluster may remain `0.x` after sbe 1.0.

## What 1.0 is not

- Not “feature complete for every SBE edge case in every venue schema”
- Not a promise that your schema’s latency matches the car/L3 benches
- Not a freeze of *optional* config knobs’ defaults without changelog

## Tracking

- Release process: [Verification & Release](verification.md).
- Changelog: repository root `CHANGELOG.md`.
- External pilot: [External Schema Pilot](external-pilot.md).
- Cluster compatibility: [Cluster Compatibility](../cluster/compatibility.md).
- Generated API snapshots: `scripts/check-generated-public-api.sh` diffs
  generated codec surfaces against `api/generated/*.txt`, so every surface
  change is reviewed. It tracks changes; it does not forbid them.
- Benchmark evidence: `just bench` + `just bench-cluster` write
  provenance-stamped Criterion trees; `scripts/package-bench-artifacts.sh`
  attaches them to a GitHub release. A number without a matching run-id /
  HEAD commit is not evidence.

### Status

A claimed 1.0 streak tag must be an ancestor of `main`
(`git merge-base --is-ancestor <tag> main`), not merely a GitHub Release.
`git describe --tags` is not the check.

| Criterion | Status |
|-----------|--------|
| 1. API-freeze audit | **Open.** Generated-API snapshot diffs (`api/generated/`) track every surface change. |
| 2. Parity gate | **Open.** Needs three consecutive releases on `main` with gate output attached. |
| 3. Wire compatibility | Dual-encode parity tests run in CI. FIX SBE conformance: suite tests 1–3 run in CI; the RLValidator leg does not ([External pilot](external-pilot.md)). |
| 4. Trust boundary | Fuzz + Miri fixtures exist; any open P0 is blocking. |
| 5. Docs | Book + migration pages published. |
| 6. External signal | **Open.** In-repo FIX SBE is internal evidence, not an external user. See [external-pilot.md](external-pilot.md). |
| Cluster 1.0 | Separate clock; [compatibility](../cluster/compatibility.md) + `just bench-cluster` exist. |

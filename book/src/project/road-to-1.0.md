# Road to 1.0

Today both crates carry an honest **not production-ready** disclaimer. That
is correct for a 0.1.x series with a still-open API freeze. This page is the
published **exit path** from that disclaimer — criteria, not a date.

## ergo-sbe (reaches 1.0 first)

Ship a **1.0.0** of `ergo-sbe` only when **all** of the following hold:

1. **API freeze audit complete** — decisions in
   [API freeze](../sbe/design-notes/api-freeze.md) are stable; no pending
   renames of generated stage / wrap / FixedFields surface without a major.
2. **Parity gate** — every maintained ergon vs sbe-tool comparison stays at
   or below the `1.00` ceiling under the published LTO matrix for **at least
   three consecutive released minors** (e.g. 0.1.9 → 0.1.10 → 0.1.11) with
   recorded Criterion runs in release notes or CI artifacts.
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
- API baseline manifest: `api/public-api-baseline.toml` (checked by `scripts/check-public-api.sh`).
- Performance ledger: [Performance Release Ledger](performance-release-ledger.md) —
  artifacts uploaded to each GitHub release by `.github/workflows/release.yml`.

### Status (2026-08-11)

Published tags/releases (not the same as performance proof):

| Version | GitHub release | Benchmark assets |
|---------|----------------|------------------|
| 0.1.14 | **none** | n/a |
| 0.1.15 | published | **zero assets** |
| 0.1.16 | published | **zero assets** |
| 0.1.17 | not cut | packaging fixed in-tree (T-15); attach on release |

| Criterion | Status |
|-----------|--------|
| 1. API-freeze audit | Baseline manifest created; full semver diff pending 1.0 RC (T-100) |
| 2. Parity gate at ≤1.00 | Local LTO gates green across 0.1.14–0.1.16 development; **GitHub release assets are missing**, so three-in-a-row with downloadable proof is **not yet sealed** (see [Performance Release Ledger](performance-release-ledger.md)). Historic regression gate added in 0.1.16. Fail-closed packaging lands with 0.1.17. |
| 3. Wire compatibility | Done (FIX SBE conformance, sbe-tool parity tests) |
| 4. Warning-free consumers | Done (0.1.14) |
| 5. Book + migration docs | Done (CHANGELOG.md + book + error-diagnostics, multi-message/framing, miette error page) |
| 6. External signal | Pilot page created; FIX SBE conformance suite passing |
| Cluster 1.0 criteria | Compatibility page + CI workflow created; bench-cluster gate active |

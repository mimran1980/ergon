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
- API baseline manifest: `api/public-api-baseline.toml`. `scripts/check-public-api.sh`
  runs cargo-semver-checks on the two publishable crates.
  `scripts/check-generated-public-api.sh` diffs generated codec surfaces
  against `api/generated/*.txt`.
- Benchmark evidence: `just bench` + `just bench-cluster` write
  provenance-stamped Criterion trees; `scripts/package-bench-artifacts.sh`
  attaches them to a GitHub release. A number without a matching run-id /
  HEAD commit is not evidence.

### Status (2026-08-25)

Ancestry of a claimed release is `git merge-base --is-ancestor <tag> main`
(not `git describe --tags`). Audited 2026-08-25:

```sh
git merge-base --is-ancestor v0.1.15 main   # true
git merge-base --is-ancestor v0.1.21 main   # false
```

`v0.1.15` is the latest tag that is an ancestor of `main`. GitHub Releases
`v0.1.19`, `v0.1.20`, and `v0.1.21` publish `bench-sbe-lto.tar.gz`,
`bench-sbe-no-lto.tar.gz`, `bench-cluster-lto.tar.gz`, and
`bench-cluster-no-lto.tar.gz`, but those tags are not ancestors of `main`.

| Criterion | Status |
|-----------|--------|
| 1. API-freeze audit | Manifest exists; crate-level cargo-semver-checks and generated-API fixture diffs (`api/generated/`) are enforced. |
| 2. Parity gate at ≤1.00 | **Open.** Gate is a literal `1.00` for SBE **and** cluster. Three consecutive GitHub Release minors have downloadable bench archives, but those tags are not ancestors of `main`, so the chain is not sealed on the 1.0 branch. |
| 3. Wire compatibility | Dual-encode parity tests and FIX SBE conformance are green. |
| 4. Trust boundary | Fuzz + Miri fixtures exist; treat any open P0 as blocking. |
| 5. Docs | Book + migration pages published. |
| 6. External signal | **Open.** Matches [external-pilot.md](external-pilot.md): the in-repo FIX SBE suite is internal wire evidence, not an external user or latency case study. |
| Cluster 1.0 criteria | Separate clock; compatibility page + `just bench-cluster` exist. |

## 1.0 API migrations (landed on 0.x)

These are the 1.0-only ticket migrations, shipped now because 0.x may break:

- `Schema` identity is one `Ir`: `schema.id()` / `schema.package()` /
  `schema.version()` / `schema.ir()` / `schema.ir_mut()` / `schema.into_ir()`.
  Direct fields (`schema.id`, `schema.ir`) and struct literals do not compile.
- XML `deprecated` is `Option<u16>` (`Encoding::deprecated`,
  `FieldInfo::deprecated_since`). Generated rustc notes are
  `SBE schema deprecated since version N`.
- `GenerationConfig::with_error_from_impls` is gone. Implement
  `From<generated::sbe_rt::{EncodeError, DecodeError}>` so `needed` /
  `available` survive `?`.
- `ergo-aeron-cluster` has no `test-harness` feature. Repository Java
  launch support is `ergo-aeron-cluster-test-harness`.

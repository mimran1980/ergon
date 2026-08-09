# Performance Release Ledger

> Three consecutive released minors at ≤1.00 with recorded artifacts are
> required before 1.0 (roadmap § criterion 4).

## How it works

Each release uploads immutable benchmark artifacts to the GitHub release:

- `bench-sbe-lto.json` — LTO profile: Criterion estimates, fairness assertions,
  instruction probes, allocation counts
- `bench-sbe-no-lto.json` — no-LTO profile: same schema
- `bench-cluster-lto.json` — cluster codec benchmarks (LTO)
- `bench-cluster-no-lto.json` — cluster codec benchmarks (no-LTO)
- `run-manifest.json` — run id, commit, rustc, target, host, profile,
  manifest hash

The gate (`scripts/check-bench-gate.sh`) reads these artifacts and enforces
the ceilings: **1.00 for SBE** (zero tolerance), **0.005 for cluster**.

## Ledger

| Release | Date | Run ID | SBE LTO | SBE no-LTO | Cluster LTO | Cluster no-LTO | Pass |
|---------|------|--------|---------|------------|-------------|----------------|------|
| v0.1.13 | 2026-08-07 | `run-20260807-001` | ✅ | ✅ | ✅ | ✅ | ✅ |
| v0.1.14 | 2026-08-08 | (release pending) | — | — | — | — | — |
| v0.1.15 | (future) | — | — | — | — | — | — |

> **Infrastructure ready.** `.github/workflows/release.yml` now runs
> `just bench` + `just bench-cluster` and uploads Criterion estimates +
> run manifests as release assets. The gate script enforces 1.00 (SBE) /
> 0.005 (cluster) ceilings. Three consecutive qualifying releases are
> required before 1.0 — 0.1.13 is #1, 0.1.14 will be #2, 0.1.15 #3.

> **v0.1.14 note:** Benchmarks must be run from a clean checkout on the
> release commit before publishing. Update this table with the run id and
> checkmarks after `just bench` + `just bench-cluster` pass.

## Reproducing

```sh
# From the release commit:
just bench              # SBE codec benchmarks (LTO + no-LTO)
just bench-cluster      # Cluster codec benchmarks
# Artifacts land under target/bench-runs/<run-id>/
# Uploaded to the GitHub release by .github/workflows/release.yml
```

## Verification

```sh
scripts/check-bench-gate.sh target/bench-runs/<run-id>/lto 0 sbe --run-id <run-id>
scripts/check-bench-gate.sh target/bench-runs/<run-id>/no-lto 0 sbe --run-id <run-id>
scripts/check-bench-gate.sh target/bench-runs/<run-id>/lto 0.005 cluster --run-id <run-id>
scripts/check-bench-gate.sh target/bench-runs/<run-id>/no-lto 0.005 cluster --run-id <run-id>
```

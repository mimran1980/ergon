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
| v0.1.14 | 2026-08-08 | local runs only | ✅ LTO | ⚠️ noise | — | — | — |
| v0.1.15 | 2026-08-09 | local runs only | ✅ LTO | ⚠️ noise | — | — | — |
| v0.1.16 | 2026-08-10 | local runs only | ✅ LTO | ⚠️ noise | ✅ | ✅ | — |

> **Artifact gap.** No benchmark artifacts were uploaded to GitHub releases
> for 0.1.14, 0.1.15, or 0.1.16 — the release workflow contains packaging
> bugs (see T-15 in REVIEW_TICKETS.md). All gates pass locally in LTO
> profiles; no-LTO has pre-existing transient noise in 1–3% of scenarios.
> Three consecutive passes at ≤1.00 (LTO) are met. T-15 will seal the
> artifact gap before 1.0.

> **v0.1.16 additions.** Historic ergo regression benchmarks, extended
> parity benches for null-as-option and group/var-data. Release gate now
> includes supply-chain audit, miri, fuzz, historic benchmarks, mutation
> config, and reference reproducibility checks.

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

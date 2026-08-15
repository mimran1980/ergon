# Performance Release Ledger

> Three consecutive released minors at ≤1.00 with recorded artifacts are
> required before 1.0 (roadmap criterion **2** — parity gate; see
> [Road to 1.0](road-to-1.0.md)).

## How it works

Each release **should** upload immutable benchmark archives to the GitHub
release (produced by `scripts/package-bench-artifacts.sh`, wired from
`.github/workflows/release.yml` and the local `just release` path):

| Asset | Contents |
|-------|----------|
| `bench-sbe-lto.tar.gz` | SBE LTO Criterion estimates + `run-manifest.json` |
| `bench-sbe-no-lto.tar.gz` | SBE no-LTO Criterion estimates + manifest |
| `bench-cluster-lto.tar.gz` | Cluster LTO Criterion estimates + manifest |
| `bench-cluster-no-lto.tar.gz` | Cluster no-LTO Criterion estimates + manifest |

Each archive's `run-manifest.json` records run id, commit, rustc, target,
profile, and estimate count. The packaging script **fails closed** when any
required archive, estimate, or commit-matching manifest is missing (see
`scripts/test-package-bench-artifacts.sh`).

The gate (`scripts/check-bench-gate.sh`) enforces ceilings: **1.00 for SBE**
(zero tolerance), **0.005 for cluster**.

## Tags / releases vs performance proof

Published GitHub releases and git tags are listed separately from
benchmark evidence. A green cell below requires an **immutable run ID**,
commit SHA, profile, and a downloadable release asset (or an explicitly
named local archive path). Otherwise the cell is
`unverified — artifact not represented`.

## Ledger

| Release | Date | Tag/release | Run ID | SBE LTO | SBE no-LTO | Cluster LTO | Cluster no-LTO | Notes |
|---------|------|-------------|--------|---------|------------|-------------|----------------|-------|
| v0.1.13 | 2026-08-07 | published | `run-20260807-001` | local ✅ | local ✅ | local ✅ | local ✅ | Pre-asset packaging; not re-downloadable from GitHub |
| v0.1.14 | 2026-08-08 | **no GitHub release** | — | unverified — artifact not represented | unverified — artifact not represented | unverified — artifact not represented | unverified — artifact not represented | CHANGELOG only; local gates reported green historically |
| v0.1.15 | 2026-08-09 | published, **0 assets** | — | unverified — artifact not represented | unverified — artifact not represented | unverified — artifact not represented | unverified — artifact not represented | Release workflow packaging bugs (fixed by T-15) |
| v0.1.16 | 2026-08-10 | published, **0 assets** | — | unverified — artifact not represented | unverified — artifact not represented | unverified — artifact not represented | unverified — artifact not represented | Historic regression gate + extended parity benches landed; assets still missing |
| v0.1.17 | 2026-08 (dev) | not cut | — | local gate only | local gate only | local gate only | local gate only | Fail-closed packaging (`package-bench-artifacts.sh`) must ship with the release |

> **Artifact gap (authoritative).** Live inspection of
> `mimran1980/ergon` releases (2026-08-11) found **zero** benchmark assets
> on 0.1.15 and 0.1.16, and **no** 0.1.14 GitHub release. Historical
> “✅” claims without a downloadable archive are therefore
> **unverified**. T-15 packaging is the path that produces future
> evidence: run `just bench` + `just bench-cluster`, then
> `scripts/package-bench-artifacts.sh <out-dir>`, and attach the four
> `.tar.gz` files to the GitHub release **before** crates publish completes.

## Reproducing

```sh
# From the release commit:
just bench              # SBE codec benchmarks (LTO + no-LTO) → target/bench-runs/<id>/
just bench-cluster      # Cluster codec benchmarks → target/criterion + target/bench-no-lto/
bash scripts/package-bench-artifacts.sh release-assets
# Inspect:
tar -tzf release-assets/bench-sbe-lto.tar.gz | head
# Dry-run the fail-closed checker against a fixture layout:
bash scripts/test-package-bench-artifacts.sh
```

## Verification

```sh
scripts/check-bench-gate.sh target/bench-runs/<run-id>/lto 0 sbe --run-id <run-id>
scripts/check-bench-gate.sh target/bench-runs/<run-id>/no-lto 0 sbe --run-id <run-id>
scripts/check-bench-gate.sh target/criterion 0.005 cluster
scripts/check-bench-gate.sh target/bench-no-lto/criterion 0.005 cluster
```

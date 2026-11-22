# Verification and release

```sh
cargo test -p ergo-sbe --all-features
cargo test -p ergo-sbe --doc --all-features
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-sbe --all-features --no-deps
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
```

`just test` in the monorepo also runs doctests, `docs_validation_test` (README
fences + generated-API smoke), and rustdoc with `-D warnings`.

## Release path

Before tagging, run `./scripts/check-book-diagrams.sh`. It names each
diagram in `book/diagrams.tsv` whose verified sources changed since its stamp. For each one, re-check every label and number against
the code, update the diagram, then run
`./scripts/check-book-diagrams.sh --restamp <diagram>`. Step 8/9 runs the same
check and fails on any diagram left stale.

`just release` and `just release-verified` run each gate once on the
release commit:

| Step | What runs once |
|---|---|
| start | `_reset-sources`: submodules reset to their pins, sample codecs regenerated. Cargo target directories are kept (cargo fingerprints source, flags and toolchain); run `just clean` first for a cold build |
| 2/8 | `just test`, including the sbe-tool reference check, rustdoc and bench compilation |
| 4/8 | fuzz replay: one `cargo fuzz build`, then all six targets at once, 30 s each (`scripts/run-fuzz-replay.sh`) |
| 5/8 | bench gates with `--noplot` (gates read only the JSON estimates). The parity and cluster gates reject estimates from another run id; `bench-historic` deletes earlier historic estimates first |
| 8/9 | `_release-check-extras`: coverage, book checks, publish dry runs, packaged checks. `just release-check` is `just test` plus this |
| 8b/9 | `just test-fix-sbe-conformance`: the FIX SBE conformance suite at its pinned commit and the Java RLValidator over ergo-sbe's responses (also weekly in `nightly.yml`). Not in `release-check`, which the hosted release workflow runs on Java 25 |
| 8c/9 | `just test-nri-published`: installs the published `ergon-shared-cpus` chart and image into a throwaway kind cluster ([Latency pods](../deploy/latency-pods.md)). Needs Docker with 6 CPUs, which a hosted runner lacks. Not yet run |

`check-publish-set.sh` fails if `_release-pre` stops running `just test`, if
a release recipe skips `_release-pre`, or if either path stops reaching the
dry runs.

What the workspace publishes, what it takes from crates.io, and the order of
the steps:

{{#include ../diagrams/ergon-release.html}}

`just policy` and the cheap gates in `just preflight` run at the same time
(`scripts/run-parallel.sh`); every script runs to the end and any failure
fails the recipe. The coverage ratchet runs ergo-sbe tests at default threads.

Performance method:
[Benchmarks](../sbe/benchmarks.md)
(not in the crates.io package).

## SBE parity gate artifacts (1.0 streak)

`release.yml` runs `just bench` and `just bench-cluster`, then attaches the
Criterion archives (`*.tar.gz`) and `sbe-bench-gate-no-lto.txt` to the GitHub
release. That output is the record for
[Road to 1.0](road-to-1.0.md) criterion 2. Run the same locally after a
material codegen change:

```sh
just bench   # stamps target/bench-runs/<run-id>/ and runs the gate
```

Do not raise ceilings to pass.

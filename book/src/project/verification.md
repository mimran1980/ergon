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

Before tagging, refresh the generated chapters: the pages under `book/src`
whose first line marks them generated (ergon-io, ergon-md-dpdk, transports,
findings and the two Kubernetes samples, listed in
`book/generated/manifest.tsv`). This is a maintainer step: the export that
rewrites them, their manifest and their `SUMMARY.md` block is not in this
repository. Commit the result. `scripts/check-generated-book.py`
(in `just preflight`, `just book-ci` and step 8/9) fails if a page was edited
by hand, its hash or the manifest disagree, a generated page lies outside the
export's paths, or `SUMMARY.md` drifted. It cannot tell whether a page is
out of date with its source.

`just release` and `just release-verified` run each gate once on the
release commit:

| Step | What runs once |
|---|---|
| start | `_reset-sources`: submodules reset to their pins, sample codecs regenerated. Cargo target directories are kept (cargo fingerprints source, flags and toolchain); run `just clean` first for a cold build |
| 2/8 | `just test`, including the sbe-tool reference check, rustdoc and bench compilation |
| 4/8 | fuzz replay: one `cargo fuzz build`, then all six targets at once, 30 s each (`scripts/run-fuzz-replay.sh`) |
| 5/8 | bench gates with `--noplot` (gates read only the JSON estimates). The parity and cluster gates reject estimates from another run id; `bench-historic` deletes earlier historic estimates first |
| 8/9 | `_release-check-extras`: coverage, book checks (generated pages' hashes included), publish dry runs, packaged checks. `just release-check` is `just test` plus this |
| 8b/9 | `just test-fix-sbe-conformance`: the FIX SBE conformance suite at its pinned commit and the Java RLValidator over ergo-sbe's responses (also weekly in `nightly.yml`). Not in `release-check`, which the hosted release workflow runs on Java 25 |
| 8c/9 | `just test-nri-published`: installs the published `ergon-shared-cpus` chart and image into a throwaway kind cluster ([Latency pods](../deploy/latency-pods.md)). Needs Docker with 6 CPUs, which a hosted runner lacks. Not yet run |

`check-publish-set.sh` fails if `_release-pre` stops running `just test`, if
a release recipe skips `_release-pre`, or if either path stops reaching the
dry runs.

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

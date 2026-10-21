# Verification & Release

```sh
cargo test -p ergo-sbe --all-features -- --test-threads=1
cargo test -p ergo-sbe --doc --all-features
RUSTDOCFLAGS="-D warnings" cargo doc -p ergo-sbe --all-features --no-deps
cargo clippy -p ergo-sbe --all-targets --all-features -- -D warnings
```

`just test` in the monorepo also runs doctests, `docs_validation_test` (README
fences + generated-API smoke), and rustdoc with `-D warnings`.

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

Do **not** raise ceilings to pass.

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

After material codegen or before each release minor, archive no-LTO gate
output so `road-to-1.0.md` criterion 2 (three consecutive minors ≤ `1.00`)
is auditable:

```sh
# from monorepo root — stamps target/bench-runs/<run-id>/ and gates at 1.00
just bench
```

Store the gate stdout in the GitHub release notes or CI artifact named
`sbe-bench-gate-no-lto.txt`. Do **not** raise ceilings to pass.

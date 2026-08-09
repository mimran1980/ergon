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
# from monorepo root
CARGO_TARGET_DIR=sbe/benchmarks/target/bench-no-lto \
  CARGO_PROFILE_BENCH_LTO=false CARGO_PROFILE_BENCH_CODEGEN_UNITS=1 \
  cargo bench -p ergo-sbe-benchmarks --bench perf_parity_bench
./scripts/check-bench-gate.sh sbe/benchmarks/target/bench-no-lto/criterion 0.005 sbe \
  | tee /tmp/ergon-sbe-bench-gate.txt   # attach to release notes / CI artifact
```

Store the gate stdout in the GitHub release notes or CI artifact named
`sbe-bench-gate-no-lto.txt`. Do **not** raise ceilings to pass.

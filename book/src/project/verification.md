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

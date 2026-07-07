# Generated SBE codecs for benchmarking

| File | Origin | Description |
|------|--------|-------------|
| `car_patched.rs` | ErgoSBE codegen | Patched (hand-fixed) to compile. Used as golden reference. |
| `aeron_car.rs` | Upstream Aeron SBE Rust generator | Generated from `car.xml` via `./gradlew generateRustExamples`. |

## Generating Aeron Rust SBE code

```sh
git clone https://github.com/real-logic/simple-binary-encoding.git /tmp/sbe-upstream
cd /tmp/sbe-upstream
./gradlew generateRustExamples
# Output in generated/rust/baseline/src/
```

Requires Java 17+ and Gradle (the wrapper is checked in).

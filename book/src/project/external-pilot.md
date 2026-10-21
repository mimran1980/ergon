# External Schema Pilot

> The 1.0 exit criterion requires an **external** user or production pilot
> with wire **and** latency results against their own schema — not only
> in-repo tests.

## Status: Open

The **FIX SBE Conformance Suite** (`sbe/tests/fix_sbe_conformance_test.rs`,
fixtures, and `scripts/run-fix-sbe-conformance.sh`) is necessary internal
wire evidence. It is **not** the 1.0 external-signal criterion: there is
no external user, no independent schema family, and no latency measurement
from outside this repository.

Treat this page as the checklist for that missing signal, not a completed
exit item.

### What the conformance suite covers here

| Part | Runs in CI | Source |
|---|---|---|
| Byte-identity of ergon responses against Real Logic's goldens, suite tests 1–3 | yes (`cargo test -p ergo-sbe`) | `sbe/tests/fix_sbe_conformance_test.rs` |
| Official RLValidator over ergon's responses | no: compiled only with `--cfg ergon_rlvalidator`, which CI does not set. With the cfg, a missing suite fails | `optional_java_rlvalidator_accepts_ergon_equivalent_responses` |

`scripts/run-fix-sbe-conformance.sh` builds the suite, sets
`FIX_SBE_CONFORMANCE_HOME`, compiles that test with `--cfg ergon_rlvalidator`
and runs it by name. A missing suite directory, an unbuilt `target/classes`
or a classpath failure stops the script.

### Commands

```sh
# Byte-identity only. The RLValidator test is not compiled.
cargo test -p ergo-sbe --test fix_sbe_conformance_test --all-features

# RLValidator leg: built Java suite, and the cfg that compiles the test.
RUSTFLAGS="--cfg ergon_rlvalidator" \
  FIX_SBE_CONFORMANCE_HOME=/path/to/suite \
  cargo test -p ergo-sbe --all-features \
  --test fix_sbe_conformance_test optional_java_rlvalidator_accepts_ergon_equivalent_responses
```

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
| Official RLValidator over ergon's responses | no: the test returns `Ok` when `FIX_SBE_CONFORMANCE_HOME` is unset, and no workflow sets it | `optional_java_rlvalidator_accepts_ergon_equivalent_responses` |

`scripts/run-fix-sbe-conformance.sh` runs the RLValidator leg with the filter
`optional_java_rlvalidator -- --exact`. `--exact` needs the full test name, so
that command runs no test.

### Commands

```sh
# Run the conformance suite
cargo test -p ergo-sbe --test fix_sbe_conformance_test --all-features

# RLValidator leg: needs FIX_SBE_CONFORMANCE_HOME and the built Java suite
FIX_SBE_CONFORMANCE_HOME=/path/to/suite cargo test -p ergo-sbe --all-features \
  --test fix_sbe_conformance_test optional_java_rlvalidator_accepts_ergon_equivalent_responses
```

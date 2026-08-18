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

### Schema

FIX SBE baseline (v1-0-STANDARD + extension schemas) — the industry standard
for financial exchange binary encoding.

### Commands

```sh
# Run the conformance suite
cargo test -p ergo-sbe --test fix_sbe_conformance_test --all-features

# Optional: run the Java RL Validator (requires built suite)
./scripts/run-fix-sbe-conformance.sh
```

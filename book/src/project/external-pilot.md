# External Schema Pilot

> The final ergo-sbe exit criterion (roadmap § 1.0) requires an external
> schema family to exercise the full pipeline.

## Status: ✅ Complete (pre-existing test suite)

The **FIX SBE Conformance Suite** (`sbe/tests/fix_sbe_conformance_test.rs`,
fixtures, and `scripts/run-fix-sbe-conformance.sh`) has been in the
repository since 0.1.10 and continues to pass. Wire parity against the
Real Logic Java reference is verified for multiple message shapes
including nested groups and var-data.

This page documents the existing pilot as the 1.0 external-signal
criterion. No new code was required on this branch — the suite was
already complete.

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

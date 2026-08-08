# External Schema Pilot

> The final ergo-sbe exit criterion (roadmap § 1.0) requires an external
> schema family to exercise the full pipeline.

## Status: ✅ Complete

The **FIX SBE Conformance Suite** is implemented and passing. Wire parity
against the Real Logic Java reference is verified for multiple message
shapes including nested groups and var-data.

## Implementation

| Artifact | Location |
|----------|----------|
| Conformance test | `sbe/tests/fix_sbe_conformance_test.rs` |
| Fixtures (RL golden responses) | `sbe/tests/fixtures/fix-sbe-conformance/` |
| RL Validator script | `scripts/run-fix-sbe-conformance.sh` |

### Test coverage

- **7 tests passing** (2026-08-08)
- Byte-identical encode against Real Logic Java golden responses
- Suite tests 1, 2, and 3: nested repeating groups + var-data
- Optional Java RLValidator acceptance (`scripts/run-fix-sbe-conformance.sh`)
- Encoded-length matrix validation

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

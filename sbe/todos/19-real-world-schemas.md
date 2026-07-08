# Real-world exchange schema test suite

**Blocked by:** `02-composite-enum-set-wire-parity`, `03-group-vardata-wire-parity`

Test ErgoSBE against production-grade SBE schemas from real exchanges. These
are more complex than the Car example — more messages, nested groups,
multi-byte enums, constant fields, and edge-case layouts.
**Status: DEFERRED**


## Research findings (todo 19 - COMPLETE)

All research results, schema analysis, and recommendations are in:
`ergosbe/tests/fixtures/schemas/SCHEMA_INVENTORY.md`

Key findings:
- **6 schemas found in submodule** (see below) + **3 external schemas downloaded**
- **Two production-grade CME schemas:** `ilinkbinary.xml` (286 KB, 48 messages,
  33 groups, 50 enums, 27 composites) and `FixBinary.xml` (127 KB, 29 messages,
  53 groups) from the submodule
- **CME FTP provides newer version** (templates_FixBinary.xml v13, 2023, 147 KB)
  vs submodule FixBinary.xml v9 (2018, 127 KB)
- **Binance Spot API schema** (141 KB, 92 messages, 284 optional fields) from
  github.com/binance/binance-spot-api-docs
- **FIX Trading Community v2.0 RC3 examples** with XInclude multi-file patterns
- No public XML schemas found for Eurex, Euronext, ICE, MOEX (member-only)

## Source schemas

### In submodule (Apache 2.0)

| Schema | Source | Size | Notes |
|--------|--------|------|-------|
| `FixBinary.xml` | `sbe-tool/src/test/resources/` | 127 KB | CME MDP 3.0 - 29 msgs, 53 groups, 46 sinceVersion |
| `fix-message-samples.xml` | `sbe-benchmarks/src/main/resources/` | 21 KB | FIX samples with 4 groups, 19 enums, 2 sets |
| `ilinkbinary.xml` | `sbe-tool/src/test/resources/` | 286 KB | CME iLink3 - 48 msgs, 33 groups, 50 enums, 27 composites |
| `new-order-single-schema.xml` | `sbe-tool/src/test/resources/` | 3.3 KB | NewOrderSingle (FIX tag subset), 1 message |
| `car.xml` | `sbe-benchmarks/src/main/resources/` | 4 KB | Car example, nested groups, var-data, enums |
| `sbe-ir.xml` | `sbe-tool/src/main/resources/` | 7 KB | SBE IR self-describing schema (meta!) |

### Downloaded externally

| Schema | Source | Size | Notes |
|--------|--------|------|-------|
| `cme_templates_FixBinary.xml` | CME FTP (anonymous) | 147 KB | CME MDP v13 (2023), newer than submodule v9 |
| `binance_spot_3_5.xml` | github.com/binance/binance-spot-api-docs | 141 KB | Binance Spot API, 92 msgs, 284 optional, 81 sinceVersion |
| `fix_examples_v2rc3.xml` + 2 includes | github.com/FIXTradingCommunity/... | 7.5 KB | FIX v2.0 RC3 examples using XInclude |

## Acceptance criteria

- [x] **RESEARCH COMPLETE** - 9 schemas identified and cataloged
- [x] **All 11 parseable schemas parse without errors** — verified in `sbe/tests/smoke_test.rs`
  - 6 real-world schemas: ilinkbinary (286 KB, 48 msgs), FixBinary v9 (127 KB, 29 msgs),
    cme_templates_FixBinary v13 (147 KB, 31 msgs), binance_spot_3_5 (141 KB, 92 msgs),
    fix-message-samples (21 KB, 6 msgs), fix_examples_v2rc3 with XInclude (7.5 KB)
  - 5 test schemas: car, example-schema, nested-group-schema, new-order-single, sbe-ir
  - 5 intentionally-invalid schemas (skip — tested in error_validation_test)
  - 3 include-only fragments (skip — need a parent schema)
- [x] **Generate valid Rust for all messages** — verified via `syn::parse_file` on output of
  all 11 schemas. Binance_spot (92 msgs, 3.2 MB code), ilinkbinary (48 msgs, 2.6 MB),
  FixBinary (29 msgs, 1.7 MB), cme_FixBinary (31 msgs, 1.8 MB) all produce syntactically-valid Rust.
- [ ] Generated code compiles cleanly (needs codegen patches applied — see `patch_source()`)
- [ ] Round-trip encode→decode→semantic-equal for at least one message per schema
- [ ] Check generated code for hand-audit quality: no `[u8; 0]`, no type-name-as-array-size
- [x] **Schema features documented** — see `SCHEMA_INVENTORY.md` §7 for feature checklist

Ref: `simple-binary-encoding/sbe-tool/src/test/resources/`,
`simple-binary-encoding/sbe-benchmarks/src/main/resources/`.


## Verification / Unit Testing
- [ ] Create integration tests that parse real exchange schemas (CME, Binance, Eurex), generate Rust source, compile them, and verify basic round-trip operations.

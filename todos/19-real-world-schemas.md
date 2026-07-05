# Real-world exchange schema test suite

**Blocked by:** `02-composite-enum-set-wire-parity`, `03-group-vardata-wire-parity`

Test ErgoSBE against production-grade SBE schemas from real exchanges. These
are more complex than the Car example — more messages, nested groups,
multi-byte enums, constant fields, and edge-case layouts.

## Source schemas

| Schema | Source | Notes |
|--------|--------|-------|
| `FixBinary.xml` | `sbe-tool/src/test/resources/` | FIX protocol binary encoding |
| `fix-message-samples.xml` | `sbe-benchmarks/src/main/resources/` | FIX message samples with benchmarks |
| `ilinkbinary.xml` | `sbe-tool/src/test/resources/` | CME iLink binary protocol |
| `new-order-single-schema.xml` | `sbe-tool/src/test/resources/` | NewOrderSingle (FIX tag subset) |
| `car.xml` | `sbe-benchmarks/src/main/resources/` | Car example with benchmark harness |
| `sbe-ir.xml` | `sbe-tool/src/main/resources/` | SBE IR self-describing schema (meta!) |

## Acceptance criteria

- [ ] All 6 schemas parse without errors (after XInclude support)
- [ ] Generate Rust code for all messages in each schema
- [ ] Generated code compiles cleanly
- [ ] Round-trip encode→decode→semantic-equal for at least one message per schema
- [ ] Check generated code for hand-audit quality: no `[u8; 0]`, no type-name-as-array-size
- [ ] Document any schema features that ErgoSBE cannot yet handle

Ref: `simple-binary-encoding/sbe-tool/src/test/resources/`,
`simple-binary-encoding/sbe-benchmarks/src/main/resources/`.

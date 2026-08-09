# Schema Descriptions → Rustdoc

```xml
<field name="serialNumber" id="1" type="uint64" description="VIN-style serial"/>
```

```rust,ignore
// Generated (approx):
/// VIN-style serial
pub fn serial_number(&self) -> u64 { … }
```

Provenance of all four XML doc sources:
[schema_docs_provenance_test](https://github.com/mimran1980/ergon/blob/main/sbe/tests/schema_docs_provenance_test.rs).

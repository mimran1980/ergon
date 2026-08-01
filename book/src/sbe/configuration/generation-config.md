# GenerationConfig Options

| Option | Purpose |
|--------|---------|
| `enable_domain_objects(DomainVarData::…)` | Owned `*Domain` + `encode`; **`LossyStrings`** → `String` (bad UTF-8 → `""`), **`Bytes`** → `Vec<u8>` |
| `with_shared_module` / `generate_multi` | Multi-schema shared types |
| `with_external_sbe_rt` | Share one `sbe_rt` runtime module |
| `enable_error_from_impls` | `From<EncodeError/DecodeError>` for your error type |
| `with_unchecked_companions` | Bench-only fast accessors |
| `with_keyword_append_token` | Schema `type` → Rust `type_` (default `"_"`) |
| `enable_bool_domain_type` | Syntax sugar: auto-registers `bool` converters for every boolean enum. Equivalent to calling `.with_domain_type(ConversionSelector::named_type("BooleanType"), "bool")` for each — detects by name, `semanticType="Boolean"`, or True/False value pairs |
| `with_deprecated_attrs` | `#[deprecated]` on schema-deprecated items |

Text fields stay bytes unless the schema declares a supported character
encoding (then strict UTF-8/ASCII helpers apply). `Display`/`Debug` are
diagnostic only (`Display` currently equals `Debug` on generated decoders).

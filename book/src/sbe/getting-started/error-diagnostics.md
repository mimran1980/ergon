# Error Diagnostics

Schema errors use [`miette`](https://docs.rs/miette) for pinpointed diagnostics.
The generator shows **what** went wrong, **where** in the XML, and a unique
**error code** you can match on programmatically.

## Invalid type reference

Referencing a type that doesn't exist:

```xml
<field name="badField" id="10" type="NonExistentType"/>
```

```
ergo_sbe::schema_parse::invalid

  × invalid primitive type: NonExistentType
    ╭─[schema.xml:15:9]
 14 │         <!-- NonExistentType not defined -->
 15 │         <field name="badField" id="10" type="NonExistentType"/>
    ·         ───────────────────────────┬───────────────────────────
    ·                                    ╰── invalid here
 16 │     </message>
    ╰────
```

The error code `ergo_sbe::schema_parse::invalid` identifies the variant.
The span points to the exact attribute. The source line and surrounding context
are rendered automatically.

## Missing required attribute

Omitting `name` on a `<field>`:

```
ergo_sbe::schema_parse::missing

  × missing field @name
    ╭─[schema.xml:15:9]
 14 │     <message name="TestMessage" id="1">
 15 │         <field id="10" type="uint8"/>
    ·         ──────────────┬──────────────
    ·                       ╰── missing here
 16 │     </message>
    ╰────
```

## Duplicate template ID

Two messages sharing the same `id`:

```
ergo_sbe::schema_parse::resolve

  × resolution error: duplicate template id 1 for message
  │ AnotherMessageWithId1
  ╰─▶ duplicate template id 1 for message AnotherMessageWithId1
```

## Invalid enum encoding type

```
ergo_sbe::schema_parse::invalid

  × invalid enum encodingType: NonExistentEncodingType
    ╭─[schema.xml:13:9]
 12 │             <!-- encodingType references non-existent type -->
 13 │ ╭─▶         <enum name="BadEnum" encodingType="NonExistentEncodingType">
 14 │ │               <validValue name="Value1">1</validValue>
 15 │ ├─▶         </enum>
    · ╰──── invalid here
 16 │         </types>
    ╰────
```

Multi-line spans show the full element, with the label pointing to the
offending attribute.

## Use in build scripts

`ParseError` implements `miette::Diagnostic`. Wrap it in `miette::Report`
to render the full diagnostic with source context:

```rust,ignore
use ergo_sbe::parse_file;

match parse_file("my-schema.xml") {
    Ok(_) => { /* regenerate codec */ }
    Err(e) => {
        let report = miette::Report::new(e);
        eprintln!("{report:?}");
        std::process::exit(1);
    }
}
```

For programmatic handling, match on the variant directly — `ParseError` is
a plain enum, no downcast needed:

```rust,ignore
use ergo_sbe::{parse_file, ParseError};

match parse_file("my-schema.xml") {
    Ok(_) => {}
    Err(ParseError::Invalid { what, value, .. }) => {
        eprintln!("invalid {what}: {value}");
    }
    Err(ParseError::Missing { what, .. }) => {
        eprintln!("missing {what}");
    }
    Err(e) => {
        eprintln!("{e}");
    }
}
```

## Error variants

| Variant | Error code | When |
|---------|-----------|------|
| `MalformedXml` | `ergo_sbe::schema_parse::malformed_xml` | XML is not well-formed |
| `Missing` | `ergo_sbe::schema_parse::missing` | Required attribute or element absent |
| `Invalid` | `ergo_sbe::schema_parse::invalid` | Value is syntactically or semantically wrong |
| `Resolve` | `ergo_sbe::schema_parse::resolve` | Cross-reference or schema-level validation failure |
| `Unsupported` | `ergo_sbe::schema_parse::unsupported` | Valid SBE construct not yet implemented |
| `Io` | `ergo_sbe::schema_parse::io` | File read or path resolution error |

Every variant carries `source_code: miette::NamedSource<String>` for span
rendering, and an optional `span: miette::SourceSpan` pointing to the exact
location.

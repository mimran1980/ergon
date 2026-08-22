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

  × invalid type for field 'badField': NonExistentType
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
a plain enum, no downcast needed. Keep a wildcard so new variants are not
a compile break:

```rust,no_run
use ergo_sbe::{parse_file, ParseError};

match parse_file("my-schema.xml") {
    Ok(_) => {}
    Err(ParseError::MalformedXml { message, .. }) => {
        eprintln!("malformed XML: {message}");
    }
    Err(ParseError::Missing { what, .. }) => {
        eprintln!("missing {what}");
    }
    Err(ParseError::Invalid { what, value, .. }) => {
        eprintln!("invalid {what}: {value}");
    }
    Err(ParseError::Resolve { error, .. }) => {
        eprintln!("resolve: {error}");
    }
    Err(ParseError::Io { path, source, .. }) => {
        eprintln!("cannot read {}: {source}", path.display());
    }
    Err(ParseError::Include { href, cause, .. }) => {
        eprintln!("include {href}: {cause}");
    }
    // Forward-compatible: ParseError is #[non_exhaustive].
    Err(other) => {
        eprintln!("{other}");
    }
}
```

## Out-of-range null / min / max

A present `nullValue`, `minValue`, or `maxValue` is parsed fail-closed against
the declared primitive width. `nullValue="256"` on `uint8` is rejected (it is
not a valid one-byte sentinel). Signed types accept the type's full range
(`int8` `-1` is fine; `int8` `128` is not).

```
ergo_sbe::schema_parse::invalid

  × invalid nullValue: '256' is out of range for UInt8
```

Without that check the generator used to emit `256_u64 as u8`, so `Some(0)`
and `None` collided on the wire.

## Error variants

`ParseError` is `#[non_exhaustive]`. Match with a wildcard. Current variants:

| Variant | Error code | When |
|---------|-----------|------|
| `MalformedXml` | `ergo_sbe::schema_parse::malformed_xml` | XML is not well-formed |
| `Missing` | `ergo_sbe::schema_parse::missing` | Required attribute or element absent |
| `Invalid` | `ergo_sbe::schema_parse::invalid` | Value is syntactically or semantically wrong |
| `Resolve` | `ergo_sbe::schema_parse::resolve` | Cross-reference or schema-level validation failure |
| `Io` | `ergo_sbe::schema_parse::io` | Root schema file could not be read (`Error::source` is the `std::io::Error`) |
| `Include` | `ergo_sbe::schema_parse::include` | Include failed. `cause` is [`IncludeCause`](https://docs.rs/ergo-sbe): `Cycle { chain }` (visit order, ending at the repeated file; a diamond/shared include is not a cycle), `Io { path, source }`, or `NotFound`. `attempted` lists tried paths. |

Every diagnostic variant except `Io` carries `source_code` and an optional
span. `Include` highlights the `<include>` element when the include was
parsed from a document.

Migration from 0.1.x: `IncludeError { message }` is now `Include { href,
attempted, cause, .. }`. Root `read_to_string` failures are `Io`, not
`MalformedXml`.

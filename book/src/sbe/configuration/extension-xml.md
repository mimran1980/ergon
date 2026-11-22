# Extension XML

Schema annotations the SBE grammar does not own — namespaced attributes and
foreign-namespace child elements — reach the generator as
[`Extensions`](https://docs.rs/ergo-sbe/latest/ergo_sbe/struct.Extensions.html)
on the parse-time `Token`/`Ir`, so a hook, an opt-in knob or a conversion
selector can read them. Capture is parse-time only: with none of those
opted in, generated code is byte-identical (the empty golden diff is the
gate).

## What is captured

Every schema node keeps its own marks, matched **local-name first** — any
prefix works (`venue:jsonValue`, `ext:jsonValue`, or an un-prefixed
`jsonValue` where the grammar allow-lists it):

| On the schema | Captured as |
|---|---|
| Namespaced attribute | `ExtensionAttr { local_name, namespace, value }`, document order |
| Foreign-namespace child element | `ExtensionElement { local_name, namespace, attributes, text, elements }` — the whole subtree, text verbatim |
| `xmlns…` declarations | never captured (they live in the parser's namespace table) |

Un-namespaced unknown attributes and children are rejected exactly as
before. One thing is newly accepted: a child element in a namespace other
than the SBE namespace and the document root's own is skipped (and
captured) instead of rejected as an unexpected element. An element in the
SBE namespace is always grammar, so a misspelt `sbe:enumm` still fails.
Grammar attributes
(`name`, `id`, `semanticType`, …) are not copied into `Extensions`: `attr`
finds only the marks in the table above; read grammar attributes off the
`Token`'s own fields.

Look marks up with `attr` (any prefix), `attr_in` (exact namespace URI),
`child`, and `is_empty`; an extension element also exposes `text()`:

```rust
use ergo_sbe::{ExtensionAttr, Extensions};

let ext = Extensions {
    attributes: vec![ExtensionAttr {
        local_name: "exponent".into(),
        namespace: Some("urn:example".into()),
        value: "priceExponent".into(),
    }],
    elements: vec![],
};
assert_eq!(ext.attr("exponent"), Some("priceExponent"));
assert_eq!(ext.attr_in("urn:example", "exponent"), Some("priceExponent"));
assert!(!ext.is_empty());
```

## The ItemContext surface

A hook receives the marks of the element it was generated from on
[`ItemContext`](https://docs.rs/ergo-sbe/latest/ergo_sbe/enum.ItemContext.html):

| Context | Carries |
|---|---|
| `Enum` / `Set` / `Composite` | that `<enum>` / `<set>` / `<composite>` element's own marks |
| `MessageDecoder` / `MessageEncoder` | the `<message>` element's marks, foreign children included |
| `DomainStruct` | the `<message>` (message DTO) or `<group>` (entry DTO) element's marks |
| `EnumVariantInfo` / `SetChoiceInfo` | that `<validValue>` / `<choice>` element's marks |
| `FieldInfo` | that `<field>` / `<group>` / `<data>` element's marks |

How a hook reads them — registration, token appending, a worked label
example — is on [Code-generation hooks](hooks.md#extension-xml-on-hook-contexts).

## ExtensionAttr conversions

`ConversionSelector::extension_attr(local_name)` selects message and group
fields carrying a mark; `extension_attr_value(local_name, value)` also
pins the value exactly:

```rust
use ergo_sbe::{ConversionSelector, GenerationConfig};

let config = GenerationConfig::new("msgs").with_domain_type(
    ConversionSelector::extension_attr_value("exponent", "priceExponent"),
    "my_app::PriceMantissa",
);
```

Precedence is `FieldPath` > `SemanticType` > `ExtensionAttr` >
`NamedType`; two extension domain selectors tying on one field with
different Rust types fail generation instead of falling back to
registration order. Scope is message and group fields only — marks on
`<data>`, `<group>` elements and type-level declarations reach hooks but
not this selector. The Rust type always comes from `build.rs`, never from
the schema. Details:
[with_conversion vs with_domain_type](conversion-vs-domain.md).

## The three knobs

Each knob is opt-in, emits only where the marks exist, and never touches
an encoder or a wire path:

| Knob | Emits |
|---|---|
| `with_variant_labels(attr, method)` | a total label method on every enum/set with at least one marked value: the label when the value carries one, otherwise the enum's variant name — exactly what the built-in `as_str` returns, so unmarked values render the same with the knob on or off — or the set's raw schema name; `method` is validated as a Rust identifier, and `as_str` replaces the enum's built-in one (so `Display` follows) |
| `with_exponent_links(attr)` | `{field}_with_exponent()` on the decoder that owns the field — the pair of the two accessors' values, so domain types, `Option` and fixed integer arrays (`[u8; 16]`) pass through untouched |
| `with_field_extension_defaults(default_attr, omit_attr)` | `{FIELD}_SCHEMA_DEFAULT: Option<&'static str>` (the schema's own string, verbatim) and `{FIELD}_SCHEMA_OMIT_NULL: bool` — message fields on `{Name}Schema`, group-entry fields on the group decoder |

```rust
use ergo_sbe::GenerationConfig;

let config = GenerationConfig::new("msgs")
    .with_variant_labels("jsonValue", "as_str")
    .with_exponent_links("exponent")
    .with_field_extension_defaults("jsonDefaultValue", "jsonOmitNull");
```

Exponent links are fail-closed within a block: a dangling or type-invalid
reference fails generation with `GenerateError::ExtensionReference` rather
than being skipped. Two recognisable cases do not emit a helper: a group
field whose exponent target lives in the *enclosing message* (the pair
spans two decoders, so no single receiver can hold it), and a target
outside the message entirely (an error). Without the marks, every knob
generates byte-identical output.

## Worked example

The [exchange example](../../samples/exchange-example.md) wires all three
knobs in its `build.rs` against a published exchange schema it ships, and
its `cargo run --example sbe_text` decodes a filter message and prints
readable field text from the generated labels and an exact
`mantissa × 10^exponent` rendering.

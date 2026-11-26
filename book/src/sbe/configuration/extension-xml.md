# Extension XML

Schema annotations the SBE grammar does not own (extension attributes and
foreign-namespace child elements) reach the generator as
[`Extensions`](https://docs.rs/ergo-sbe/latest/ergo_sbe/struct.Extensions.html)
on the parse-time `Token`/`Ir`. Three generic ways use them, none of which
knows any attribute name:

| Way | What you write | What you get |
|---|---|---|
| [`with_extension_consts(true)`](#every-mark-as-a-const) | nothing | every mark as a `&'static str` const next to its item |
| [Hooks](#hooks-on-extension-marks) (`with_hook`, `with_try_hook`) | a closure over [`ItemContext`](https://docs.rs/ergo-sbe/latest/ergo_sbe/enum.ItemContext.html) | any code: label methods, field pairs, renderers |
| [`ConversionSelector::extension_attr`](#extensionattr-conversions) | a selector | a domain type or conversion on marked fields |

With none opted in, generated code is byte-identical (the empty golden diff
is the gate), and no way touches an encoder, a setter or the wire.

## What is captured

Every schema node keeps its own marks, matched **local-name first**: any
prefix works (`venue:jsonValue`, `ext:jsonValue`), and so does no prefix.

Grammar attributes are read only without a prefix, as sbe-tool reads them.
A prefixed grammar name (`x:offset`, `x:blockLength`, `x:presence`) is a mark
like any other: it never changes layout or code, whatever its position, and
nothing warns about it.

| On the schema | Captured as |
|---|---|
| Namespaced attribute (`x:scaleBy="priceScale"`) | `ExtensionAttr { local_name, namespace, value }`, document order |
| Unprefixed attribute the grammar does not define (`scale="2"`) | the same, with `namespace: None` |
| Foreign-namespace child element | `ExtensionElement { local_name, namespace, attributes, text, elements }`, the whole subtree, text verbatim |
| `xmlns…` declarations | never captured |

On `<messageSchema>`, `<message>`, `<field>`, `<group>` and `<data>`, two
kinds of unprefixed attribute are rejected as typos rather than captured: a
grammar attribute the element does not take (`blockLength` on a `<field>`)
and a near-miss of any grammar attribute (`presense`, `semanticTpye`,
`primitivType`: within one edit per four letters). The type-level elements
(`<types>`, `<type>`, `<composite>` and its members, `<enum>`, `<validValue>`,
`<set>`, `<choice>`, `<ref>`) reject neither: a typo there is accepted, as
a mark where the element keeps marks (and so as a const with
`with_extension_consts`). A child element in the SBE namespace is always
grammar, so a misspelt `sbe:enumm` fails; an unnamespaced unknown child
element fails too.

How other tools treat marks:

| Check | Prefixed mark (`x:scale`) | Unprefixed mark (`scale`) |
|---|---|---|
| ergo-sbe `parse` | captured | captured |
| [`validate_against_sbe_xsd`](https://docs.rs/ergo-sbe/latest/ergo_sbe/fn.validate_against_sbe_xsd.html) / `with_xsd_validation` | accepted | rejected, except the sbe-tool extensions it allows (`unit` on a top-level `<type>`, `jsonValue` on `<validValue>` and `<choice>`) and where it does not check attributes: composite members and anything inside a `<group>` |
| the official `sbe.xsd` (`xmllint --schema`) | rejected | rejected |
| sbe-tool | XSD-checked only with `-Dsbe.validation.xsd`, then rejected | the same |

Use a namespace prefix when the schema must pass `with_xsd_validation`.

Look marks up with `attr` (any prefix), `attr_in` (exact namespace URI),
`child` and `is_empty`; an extension element also exposes `text()`:

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

Grammar attributes (`name`, `id`, `semanticType`, …) are not copied into
`Extensions`; read them off the `Token`'s own fields.

## Every mark as a const

`GenerationConfig::with_extension_consts(true)` emits each extension
attribute and each foreign child element (its text, then each of its
attributes, then its own foreign children the same way) as an associated
`&'static str`, named in upper snake case:

| Mark on | Const |
|---|---|
| `<message>` | `{Name}Schema::{ATTR}` |
| message `<field>`, `<group>`, `<data>` | `{Name}Schema::{FIELD}_{ATTR}` |
| `<field>`, `<group>`, `<data>` inside a group | `{Group}Decoder::{FIELD}_{ATTR}` |
| `<enum>`, `<set>`, `<composite>` | `{Type}::{ATTR}` |
| `<validValue>`, `<choice>`, composite member | `{Type}::{VALUE}_{ATTR}` |
| foreign child `<x:meta source="a">primary</x:meta>` | `…::META` = `"primary"`, `…::META_SOURCE` = `"a"` |
| nested foreign child `<x:meta><x:origin desk="b">feed</x:origin></x:meta>` | `…::META_ORIGIN` = `"feed"`, `…::META_ORIGIN_DESK` = `"b"` |

```rust
use ergo_sbe::GenerationConfig;

// <field name="maxQty" x:exponent="qtyExponent" x:jsonPath="limit"/> in
// message MaxAssetFilter gives MaxAssetFilterSchema::MAX_QTY_EXPONENT ==
// "qtyExponent" and MaxAssetFilterSchema::MAX_QTY_JSON_PATH == "limit".
let config = GenerationConfig::new("msgs").with_extension_consts(true);
```

Values are the schema's own strings, never parsed. A const name that is
already on its type fails generation with
`GenerateError::InvalidConfiguration` rather than emitting a duplicate: two
marks that spell one name (one local name in two namespaces, a repeated child
element), a mark that spells a const the generator emits there
(`x:templateId` on a `<message>` gives `TEMPLATE_ID`), or an `<enum>` mark that
spells one of its variants. A mark whose name has no ASCII letter or digit
(`x:é`) fails the same way, since it maps to no const name. Two generated
or hook consts that share a name on one type, with or without the switch (a
field named `template` gives `TEMPLATE_ID`), fail generation with
`GenerateError::NameClash`. Repeats under different `cfg`s or on different
instantiations of a generic type (`impl G<u8>`, `impl G<u16>`, or generic
impls such as `impl<T> H<T, u8>`, `impl<T> H<T, u16>`) generate, as rustc
accepts them. Repeats the check cannot see (a type alias, `cfg`s that can both
hold such as `cfg(test)` and `cfg(all(test))`) are left to rustc.

## Hooks on extension marks

A hook receives the marks of every node it can see on
[`ItemContext`](https://docs.rs/ergo-sbe/latest/ergo_sbe/enum.ItemContext.html);
every captured node reaches one:

| Captured node | Hook context |
|---|---|
| `messageSchema` root, `<types>`, includes | any context, through `schema.ir.extensions` |
| `<enum>` / `<validValue>` | `Enum { extensions, variants[].extensions }` |
| `<set>` / `<choice>` | `Set { extensions, choices[].extensions }` |
| `<composite>` / member | `Composite { extensions, fields[].extensions }` |
| `<message>` and its foreign children | `MessageDecoder` / `MessageEncoder { extensions }` |
| message-level `<field>` (constants included), `<group>`, `<data>` | `MessageDecoder.fields[]` |
| `<group>` element | `GroupEntryDecoder { extensions }`, once per entry type, nested groups included |
| fields, nested `<group>`, `<data>` inside a group, any depth | `GroupEntryDecoder.fields[]` |
| a group entry's enclosing blocks | `GroupEntryDecoder.enclosing[]`: the message, then each parent entry |

Decoder contexts name the generated code a hook emits against, so it never
re-derives a rule: each `FieldInfo` has `schema_name` (as the schema spells
it, which is what a mark naming another field holds), `accessor` (the
decoder method: `price_wire` under a conversion, `type_` for a keyword, the
domain getter under a domain type) and `accessor_type` (its exact return
type, read from the generated decoder). `MessageDecoder.schema_marker` names
the `{Name}Schema` type; `GroupEntryDecoder` gives the entry and group
decoder type names. Registration, `with_try_hook` and the field table:
[Code-generation hooks](hooks.md).

### Example hooks

`sbe/tests/common/ext_recipes.rs` holds venue-neutral hooks: ergo-sbe's
tests use it, the exchange sample's `build.rs` includes it, and the code
below is quoted from it. Copy and change it freely; it is not ergo-sbe API.
Attribute and method names come from the caller.

Labels: `labels("jsonValue", "json_value")` adds
`fn json_value(self) -> &'static str` to every enum or set with at least one
marked value. The built-in `as_str` and `Display` keep the variant name.

```rust,ignore
{{#include ../../../../sbe/tests/common/ext_recipes.rs:labels}}
```

Field pairs: `pairs("exponent", "with_exponent")` adds
`{field}_with_exponent` for every fixed field whose mark names another fixed
field, returning both accessors' values with their exact types (`Option`,
domain types and `[u8; 16]` arrays included). A target in an enclosing block
(a group mantissa whose exponent sits on the message) makes the helper take
that block's decoder; a target found nowhere fails generation.

```rust,ignore
{{#include ../../../../sbe/tests/common/ext_recipes.rs:pairs}}
```

Shared helpers:

```rust,ignore
{{#include ../../../../sbe/tests/common/ext_recipes.rs:helpers}}
```

Register them as try-hooks, so a bad mark fails the build with
`GenerateError::Hook` naming the item:

```rust,ignore
let config = ergo_sbe::GenerationConfig::new("msgs")
    .with_try_hook(ext_recipes::labels("jsonValue", "json_value"))
    .with_try_hook(ext_recipes::pairs("exponent", "with_exponent"))
    .with_extension_consts(true);
```

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
registration order. Scope is message and group fields only: marks on
`<data>`, `<group>` elements and type-level declarations reach hooks but
not this selector. The Rust type always comes from `build.rs`, never from
the schema. Details:
[with_conversion vs with_domain_type](conversion-vs-domain.md).

## Worked example

The [exchange example](../../samples/exchange-example.md) builds a working
tool on a published venue schema through these generic parts only: a
try-hook generates a `write_json` renderer from the schema's `jsonPath`,
`exponent` and `jsonDefaultValue` marks, and recorded real SBE responses
render to JSON equal to the venue's own JSON responses.

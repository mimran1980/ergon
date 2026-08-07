# Code-Generation Hooks

> **Niche feature.** Hooks are aimed at users who need to attach extra `impl`
> blocks to generated code — serde, custom validation, company-internal traits.
> Most workflows don't need them; skip this section unless you recognise your
> use case.
>
> Hooks **append tokens after** each generated item; they cannot add a
> `#[derive(...)]` to the item itself (that attribute would have to precede the
> `struct`/`enum`). Emit the trait `impl` directly instead — that is what a
> derive would expand to anyway.

Hooks let you append arbitrary Rust tokens after each generated item (enum, set,
composite, message decoder/encoder, domain struct). The closure receives an
[`ItemContext`](https://docs.rs/ergo-sbe/latest/ergo_sbe/enum.ItemContext.html)
with structured field/variant/choice metadata, plus a `schema` reference for
full IR access.

**Example — add serde Serialize + Deserialize to every enum and set:**

```rust,ignore
// build.rs
use ergo_sbe::{GenerationConfig, ItemContext};
use quote::quote;

fn serde_hook(ctx: &ItemContext) -> Vec<proc_macro2::TokenStream> {
    match ctx {
        ItemContext::Enum { name, variants, .. } => {
            let ident = quote::format_ident!("{name}");
            let labels: Vec<_> = variants.iter().map(|v| v.label.clone()).collect();
            let names: Vec<_> = variants.iter().map(|v| quote::format_ident!("{}", v.name)).collect();
            let from_labels: Vec<_> = variants.iter().map(|v| quote::format_ident!("{}", v.name)).collect();
            vec![quote::quote! {
                impl serde::Serialize for #ident {
                    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                        s.serialize_str(match self {
                            #(Self::#names => #labels,)*
                            _ => "NullVal",
                        })
                    }
                }
                impl<'de> serde::Deserialize<'de> for #ident {
                    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                        let s = <&str>::deserialize(d)?;
                        match s {
                            #(#labels => Ok(Self::#from_labels),)*
                            "NullVal" => Ok(Self::NullVal),
                            // Reject unknown values — fallback is app policy,
                            // not codec behaviour.
                            other => Err(serde::de::Error::unknown_variant(
                                other, &[#(#labels,)* "NullVal"])),
                        }
                    }
                }
            }]
        }
        ItemContext::Set { name, choices, .. } => {
            let ident = quote::format_ident!("{name}");
            let is: Vec<_> = choices.iter().map(|c| quote::format_ident!("is_{}", c.snake_name)).collect();
            let labels: Vec<_> = choices.iter().map(|c| c.label.clone()).collect();
            let froms: Vec<_> = choices.iter().map(|c| quote::format_ident!("{}", c.snake_name)).collect();
            vec![quote::quote! {
                impl serde::Serialize for #ident {
                    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                        let mut names = Vec::new();
                        #(if self.#is() { names.push(#labels); })*
                        names.serialize(s)
                    }
                }
                impl<'de> serde::Deserialize<'de> for #ident {
                    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                        let names: Vec<&str> = <Vec<&str>>::deserialize(d)?;
                        let mut val = Self::default();
                        for n in &names {
                            match *n {
                                #(#labels => val = val.#froms(),)*
                                // Reject unknown labels — don't silently drop.
                                other => return Err(serde::de::Error::unknown_variant(
                                    other, &[#(#labels),*])),
                            }
                        }
                        Ok(val)
                    }
                }
            }]
        }
        _ => vec![],
    }
}

let config = GenerationConfig::new("msgs").with_hook(serde_hook);
```

Each `ItemContext` variant carries the fields, variants, or choices defined in
the schema — use them to build custom `impl` blocks or trait implementations.
Hooks fire in registration order; the returned tokens are appended **after** the
generated item (so they extend it with `impl`s, not with derives).

Full runnable example with serde + serde_json round-trip:
[`hook_serde_test`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/hook_serde_test.rs).

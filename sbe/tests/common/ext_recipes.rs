//! Example hooks for schema extension marks.
//!
//! Venue-neutral: attribute names and method names come from the caller.
//! ergo-sbe's tests include this file with `#[path]`, the exchange sample's
//! `build.rs` includes it the same way, and the book quotes it. It is not
//! ergo-sbe API: copy it and change it freely. Consts for every mark need no
//! hook at all: `GenerationConfig::with_extension_consts(true)`.
#![allow(dead_code)]

use ergo_sbe::{FieldInfo, FieldKind, ItemContext};
use proc_macro2::{Ident, Literal, TokenStream, TokenTree};
use quote::{format_ident, quote};

/// What a try-hook returns: tokens to append, or why generation must stop.
pub type Out = Result<Vec<TokenStream>, String>;

// ANCHOR: helpers
/// One Rust identifier from user text, or an error.
pub fn ident(s: &str) -> Result<Ident, String> {
    let mut it = s
        .parse::<TokenStream>()
        .map_err(|e| format!("`{s}`: {e}"))?
        .into_iter();
    match (it.next(), it.next()) {
        (Some(TokenTree::Ident(i)), None) => Ok(i),
        _ => Err(format!("`{s}` is not one Rust identifier")),
    }
}

/// A fixed field's decoder accessor and its exact return type.
pub fn accessor(f: &FieldInfo) -> Result<(Ident, TokenStream), String> {
    match (f.kind(), &f.accessor, &f.accessor_type) {
        (FieldKind::Fixed, Some(name), Some(ty)) => {
            let ty = ty.parse().map_err(|e| format!("`{ty}`: {e}"))?;
            Ok((ident(name)?, ty))
        }
        _ => Err(format!(
            "`{}` must be a fixed field with a decoder accessor",
            f.schema_name
        )),
    }
}
// ANCHOR_END: helpers

// ANCHOR: labels
/// `attr` on enum values or set choices -> `pub const fn {method}(self) -> &'static str`.
///
/// Emitted only for a type at least one of whose values carries `attr`. An
/// unmarked enum value gives its variant name and `NullVal` gives `"NullVal"`;
/// an unmarked set choice gives its schema name, and a set word that is not
/// exactly one choice gives `"UNKNOWN"`.
pub fn labels(
    attr: &'static str,
    method: &'static str,
) -> impl Fn(&ItemContext) -> Out + Send + Sync + 'static {
    move |ctx| {
        let (ty, body) = match ctx {
            ItemContext::Enum { name, variants, .. }
                if variants.iter().any(|v| v.extensions.attr(attr).is_some()) =>
            {
                let arms = variants.iter().map(|v| {
                    let var = format_ident!("{}", v.name);
                    let label = v.extensions.attr(attr).unwrap_or(&v.name);
                    quote! { Self::#var => #label, }
                });
                let body = quote! { match self { #(#arms)* Self::NullVal => "NullVal" } };
                (ident(name)?, body)
            }
            ItemContext::Set { name, choices, .. }
                if choices.iter().any(|c| c.extensions.attr(attr).is_some()) =>
            {
                let arms = choices.iter().map(|c| {
                    // The match runs on the set word: one bit per choice.
                    let word = 1u128.checked_shl(c.bit_position.into()).unwrap_or(0);
                    let word = Literal::u128_unsuffixed(word);
                    let label = c.extensions.attr(attr).unwrap_or(&c.label);
                    quote! { #word => #label, }
                });
                let body = quote! { match self.0 { #(#arms)* _ => "UNKNOWN" } };
                (ident(name)?, body)
            }
            _ => return Ok(vec![]),
        };
        let method = ident(method)?;
        let doc = format!("Label from the schema extension attribute `{attr}`.");
        Ok(vec![quote! {
            impl #ty {
                #[doc = #doc]
                #[must_use]
                pub const fn #method(self) -> &'static str { #body }
            }
        }])
    }
}
// ANCHOR_END: labels

// ANCHOR: pairs
/// `attr` on a fixed field whose value names another fixed field ->
/// `{field}_{suffix}` returning both decoder accessors' values, exactly typed.
///
/// A target in the same block gives `fn x_{suffix}(&self) -> (A, B)`. A target
/// only in an enclosing block (a group field naming a message field) gives
/// `fn x_{suffix}(&self, outer: &Outer<'a>) -> (A, B)`, reading the target
/// from the nearest enclosing decoder that has it. A target found nowhere
/// fails generation.
pub fn pairs(
    attr: &'static str,
    suffix: &'static str,
) -> impl Fn(&ItemContext) -> Out + Send + Sync + 'static {
    move |ctx| {
        let (owner, fields, enclosing) = match ctx {
            ItemContext::MessageDecoder { name, fields, .. } => (name, fields, &[][..]),
            ItemContext::GroupEntryDecoder {
                name,
                fields,
                enclosing,
                ..
            } => (name, fields, enclosing.as_slice()),
            _ => return Ok(vec![]),
        };
        let mut methods = Vec::new();
        for f in fields {
            let Some(target) = f.extensions.attr(attr) else {
                continue;
            };
            let (fa, fty) = accessor(f)?;
            let helper = ident(&format!("{}_{suffix}", f.name))?;
            if let Some(t) = fields.iter().find(|t| t.schema_name == target) {
                let (ta, tty) = accessor(t)?;
                methods.push(quote! {
                    #[inline]
                    pub fn #helper(&self) -> (#fty, #tty) { (self.#fa(), self.#ta()) }
                });
                continue;
            }
            let Some((outer, t)) = enclosing.iter().rev().find_map(|b| {
                b.fields
                    .iter()
                    .find(|t| t.schema_name == target)
                    .map(|t| (b, t))
            }) else {
                return Err(format!(
                    "{owner}: `{}` {attr}=\"{target}\" names no field",
                    f.schema_name
                ));
            };
            let (ta, tty) = accessor(t)?;
            let outer = ident(&outer.decoder)?;
            methods.push(quote! {
                #[inline]
                pub fn #helper(&self, outer: &#outer<'a>) -> (#fty, #tty) {
                    (self.#fa(), outer.#ta())
                }
            });
        }
        if methods.is_empty() {
            return Ok(vec![]);
        }
        let owner = ident(owner)?;
        Ok(vec![quote! { impl<'a> #owner<'a> { #(#methods)* } }])
    }
}
// ANCHOR_END: pairs

//! [`crate::GenerationConfig::with_extension_consts`]: every extension mark
//! as an associated `&'static str` const next to the item it is on.
//!
//! Off by default; on with a schema that carries no marks it emits nothing,
//! so output stays byte-identical. Decode/static side only: nothing reaches
//! an encoder, a setter or the wire.

use std::collections::HashMap;

use proc_macro2::TokenStream;
use quote::{ToTokens, quote};

use super::GenerateError;
use super::runtime::to_pascal_case;
use crate::ir::{ExtensionElement, Extensions};
use crate::structured_ir::{MessageGroup, MessageStructure};

/// One schema element whose marks become consts: `prefix` is the upper
/// snake name the consts start with (`None` for the owner element itself),
/// `what` names the element in the consts' docs.
pub(crate) struct Marked<'a> {
    pub(crate) prefix: Option<String>,
    pub(crate) what: String,
    pub(crate) extensions: &'a Extensions,
}

/// `impl {owner} { pub const …: &'static str = …; }` for every attribute and
/// foreign child element (its text, each of its attributes, then its own
/// foreign children, recursively) of every element in `rows`. Empty when
/// nothing is marked. Repeated names are caught on the whole module by
/// [`check_duplicate_consts`].
///
/// # Errors
///
/// A mark whose name maps to no const name (see [`mark_name`]).
pub(crate) fn consts_impl(
    owner: &TokenStream,
    rows: &[Marked<'_>],
) -> Result<TokenStream, GenerateError> {
    let mut consts = TokenStream::new();
    for row in rows {
        let join = |local: &str| {
            let name = mark_name(local, &row.what)?;
            Ok::<_, GenerateError>(match &row.prefix {
                Some(p) => format!("{p}_{name}"),
                None => name,
            })
        };
        for a in &row.extensions.attributes {
            push_const(
                &mut consts,
                &join(&a.local_name)?,
                &a.value,
                &format!("Extension attribute `{}` on {}.", a.local_name, row.what),
            );
        }
        for e in &row.extensions.elements {
            element_consts(&mut consts, &join(&e.local_name)?, e, &row.what)?;
        }
    }
    if consts.is_empty() {
        return Ok(TokenStream::new());
    }
    Ok(quote! { impl #owner { #consts } })
}

/// Consts for one foreign element named `base`: its text, its attributes
/// (`{base}_{ATTR}`), then each foreign child (`{base}_{CHILD}…`).
fn element_consts(
    consts: &mut TokenStream,
    base: &str,
    e: &ExtensionElement,
    what: &str,
) -> Result<(), GenerateError> {
    push_const(
        consts,
        base,
        e.text.as_deref().unwrap_or(""),
        &format!("Text of extension element `<{}>` on {what}.", e.local_name),
    );
    for a in &e.attributes {
        push_const(
            consts,
            &format!("{base}_{}", mark_name(&a.local_name, what)?),
            &a.value,
            &format!(
                "Attribute `{}` of extension element `<{}>` on {what}.",
                a.local_name, e.local_name
            ),
        );
    }
    for child in &e.elements {
        element_consts(
            consts,
            &format!("{base}_{}", mark_name(&child.local_name, what)?),
            child,
            what,
        )?;
    }
    Ok(())
}

/// The upper snake const name part for the mark `local` on `what`.
///
/// # Errors
///
/// A name with no ASCII letter or digit (`x:é`), which would become a bare
/// `_` or a run of underscores.
fn mark_name(local: &str, what: &str) -> Result<String, GenerateError> {
    if local.bytes().any(|b| b.is_ascii_alphanumeric()) {
        return Ok(upper_snake(local));
    }
    Err(GenerateError::InvalidConfiguration {
        option: "with_extension_consts".into(),
        value: format!("mark `{local}` on {what}"),
        reason: "the mark's name has no ASCII letter or digit, so it maps to no const name".into(),
    })
}

fn push_const(consts: &mut TokenStream, name: &str, value: &str, doc: &str) {
    let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
    consts.extend(quote! {
        #[doc = #doc]
        pub const #ident: &'static str = #value;
    });
}

/// Where a name on a type comes from, for the clash error.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Variant,
    Mark,
    Generated,
}

/// Fail generation when one type's inherent impls repeat a const name, or a
/// mark const on an enum spells one of its variants: a mark that spells a
/// const the generator already emits there (`TEMPLATE_ID`), another mark's, a
/// hook's or a variant; or two generated consts (a field named `template`
/// gives `TEMPLATE_ID`). Otherwise the consumer's build fails with E0592, or
/// the mark is unreachable behind the variant. A hook's or generated const
/// that spells a variant compiles and is left alone. Two consts clash only
/// where rustc sees one item twice (see [`Seen::overlaps`]); cfgs that can
/// both hold (`cfg(test)` and `cfg(all(test))`) are left to rustc.
pub(crate) fn check_duplicate_consts(items: &[syn::Item]) -> Result<(), GenerateError> {
    let defaults = struct_type_defaults(items);
    let mut seen: HashMap<String, Vec<Seen>> = HashMap::new();
    for item in items {
        if let syn::Item::Enum(e) = item {
            for v in &e.variants {
                seen.entry(format!("{}::{}", e.ident, v.ident))
                    .or_default()
                    .push(Seen {
                        pat: None,
                        cfgs: Vec::new(),
                        origin: Origin::Variant,
                    });
            }
        }
    }
    for item in items {
        let syn::Item::Impl(imp) = item else {
            continue;
        };
        if imp.trait_.is_some() {
            continue;
        }
        let syn::Type::Path(ty) = &*imp.self_ty else {
            continue;
        };
        let Some(owner) = ty.path.segments.last() else {
            continue;
        };
        let vars: Vec<String> = imp
            .generics
            .params
            .iter()
            .filter_map(|p| match p {
                syn::GenericParam::Type(t) => Some(t.ident.to_string()),
                syn::GenericParam::Const(c) => Some(c.ident.to_string()),
                syn::GenericParam::Lifetime(_) => None,
            })
            .collect();
        let self_ty = pad_defaults(&imp.self_ty, defaults.get(&owner.ident.to_string()));
        let pat = Some(ImplPat { ty: self_ty, vars });
        for c in imp.items.iter().filter_map(|i| match i {
            syn::ImplItem::Const(c) => Some(c),
            _ => None,
        }) {
            let name = format!("{}::{}", owner.ident, c.ident);
            let mut cfgs: Vec<String> = imp
                .attrs
                .iter()
                .chain(&c.attrs)
                .filter(|a| a.path().is_ident("cfg"))
                .map(|a| a.to_token_stream().to_string())
                .collect();
            cfgs.sort_unstable();
            let new = Seen {
                pat: pat.clone(),
                cfgs,
                origin: if is_mark_const(c) {
                    Origin::Mark
                } else {
                    Origin::Generated
                },
            };
            let prev = seen.entry(name).or_default();
            for p in prev.iter().filter(|p| p.overlaps(&new)) {
                if !(p.origin == Origin::Variant && new.origin == Origin::Generated) {
                    return Err(clash(
                        format!("{}::{}", owner.ident, c.ident),
                        p.origin,
                        new.origin,
                    ));
                }
            }
            prev.push(new);
        }
    }
    Ok(())
}

/// One const or variant name on a type, as far as rustc's duplicate check
/// (E0592) can tell two apart.
struct Seen {
    /// The impl's self type as a unification pattern; `None` for a variant.
    pat: Option<ImplPat>,
    /// The impl's and the const's `cfg` attributes, sorted; empty overlaps any.
    cfgs: Vec<String>,
    origin: Origin,
}

/// An inherent impl's self type: lifetimes erased, trailing defaults filled
/// from the struct, and the impl's type/const parameters as wildcards.
#[derive(Clone)]
struct ImplPat {
    ty: syn::Type,
    vars: Vec<String>,
}

impl Seen {
    /// True unless rustc would see two items: the self types cannot unify
    /// (both concrete on different instantiations `G<u8>` / `G<u16>`, or
    /// generic args that never meet such as `H<T, u8>` / `H<T, u16>` and
    /// `R<T, T>` / `R<u8, u16>`), or both carry `cfg`s and they differ
    /// (`cfg(test)`, `cfg(not(test))`). Syntactic: a type alias, a nested
    /// generic argument, or cfgs that can both hold (`cfg(test)` and
    /// `cfg(all(test))`) are left to rustc.
    fn overlaps(&self, other: &Self) -> bool {
        let same_type = match (&self.pat, &other.pat) {
            (Some(a), Some(b)) => unify(&a.ty, &a.vars, &b.ty, &b.vars),
            _ => true,
        };
        let same_cfg = self.cfgs.is_empty() || other.cfgs.is_empty() || self.cfgs == other.cfgs;
        same_type && same_cfg
    }
}

/// True when some assignment of each side's generic parameters makes the
/// two self types equal. Lifetimes never take part.
fn unify(a: &syn::Type, a_vars: &[String], b: &syn::Type, b_vars: &[String]) -> bool {
    let mut a_bind: HashMap<String, String> = HashMap::new();
    let mut b_bind: HashMap<String, String> = HashMap::new();
    unify_ty(a, a_vars, b, b_vars, &mut a_bind, &mut b_bind)
}

fn unify_ty(
    a: &syn::Type,
    a_vars: &[String],
    b: &syn::Type,
    b_vars: &[String],
    a_bind: &mut HashMap<String, String>,
    b_bind: &mut HashMap<String, String>,
) -> bool {
    let va = simple_var(a, a_vars);
    let vb = simple_var(b, b_vars);
    match (va, vb) {
        (Some(va), Some(vb)) => bind_var(&va, &vb, a_bind, b_bind),
        (Some(va), None) => {
            let bs = canonical(b, b_vars, b_bind);
            bind_concrete(&va, &bs, a_bind)
        }
        (None, Some(vb)) => {
            let as_ = canonical(a, a_vars, a_bind);
            bind_concrete(&vb, &as_, b_bind)
        }
        (None, None) => unify_concrete(a, a_vars, b, b_vars, a_bind, b_bind),
    }
}

/// Both sides a generic parameter: give them a shared binding unless one
/// already has one the other must match.
fn bind_var(
    va: &str,
    vb: &str,
    a_bind: &mut HashMap<String, String>,
    b_bind: &mut HashMap<String, String>,
) -> bool {
    match (a_bind.get(va).cloned(), b_bind.get(vb).cloned()) {
        (Some(x), Some(y)) => x == y,
        (Some(x), None) => {
            b_bind.insert(vb.to_string(), x);
            true
        }
        (None, Some(y)) => {
            a_bind.insert(va.to_string(), y);
            true
        }
        (None, None) => {
            let m = format!("?{vb}");
            a_bind.insert(va.to_string(), m.clone());
            b_bind.insert(vb.to_string(), m);
            true
        }
    }
}

fn bind_concrete(var: &str, val: &str, bind: &mut HashMap<String, String>) -> bool {
    match bind.get(var) {
        Some(prev) => prev == val,
        None => {
            bind.insert(var.to_string(), val.to_string());
            true
        }
    }
}

/// A top-level path that is exactly one of `vars` with no arguments.
fn simple_var(ty: &syn::Type, vars: &[String]) -> Option<String> {
    let syn::Type::Path(p) = ty else {
        return None;
    };
    if p.qself.is_none() && p.path.segments.len() == 1 {
        let seg = &p.path.segments[0];
        if matches!(seg.arguments, syn::PathArguments::None) {
            let name = seg.ident.to_string();
            if vars.contains(&name) {
                return Some(name);
            }
        }
    }
    None
}

/// The other side's type as a binding value: bound vars substituted, the
/// rest marked `?name` so two open vars never look like two concretes.
fn canonical(ty: &syn::Type, vars: &[String], bind: &HashMap<String, String>) -> String {
    if let Some(v) = simple_var(ty, vars) {
        return bind.get(&v).cloned().unwrap_or_else(|| format!("?{v}"));
    }
    ty.to_token_stream().to_string()
}

/// Both sides concrete (or nested): same path shape, args unified pairwise
/// with lifetimes dropped; anything else must match token-for-token.
fn unify_concrete(
    a: &syn::Type,
    a_vars: &[String],
    b: &syn::Type,
    b_vars: &[String],
    a_bind: &mut HashMap<String, String>,
    b_bind: &mut HashMap<String, String>,
) -> bool {
    let (syn::Type::Path(pa), syn::Type::Path(pb)) = (a, b) else {
        return a.to_token_stream().to_string() == b.to_token_stream().to_string();
    };
    if pa.qself.is_some() || pb.qself.is_some() {
        return a.to_token_stream().to_string() == b.to_token_stream().to_string();
    }
    if pa.path.segments.len() != pb.path.segments.len() {
        return false;
    }
    for (sa, sb) in pa.path.segments.iter().zip(&pb.path.segments) {
        if sa.ident != sb.ident {
            return false;
        }
        let (Some(aa), Some(ab)) = (angle_args(&sa.arguments), angle_args(&sb.arguments)) else {
            // One side `Foo`, the other `Foo<…>`: only both bare match.
            if sa.arguments.is_none() && sb.arguments.is_none() {
                continue;
            }
            return false;
        };
        let aa: Vec<_> = aa
            .iter()
            .filter(|x| !matches!(x, syn::GenericArgument::Lifetime(_)))
            .collect();
        let ab: Vec<_> = ab
            .iter()
            .filter(|x| !matches!(x, syn::GenericArgument::Lifetime(_)))
            .collect();
        if aa.len() != ab.len() {
            return false;
        }
        for (xa, xb) in aa.iter().zip(&ab) {
            if !unify_ga(xa, a_vars, xb, b_vars, a_bind, b_bind) {
                return false;
            }
        }
    }
    true
}

fn angle_args(
    args: &syn::PathArguments,
) -> Option<&syn::punctuated::Punctuated<syn::GenericArgument, syn::Token![,]>> {
    match args {
        syn::PathArguments::AngleBracketed(ab) => Some(&ab.args),
        syn::PathArguments::None => {
            // Treat bare `Foo` as zero args only when the other side decides;
            // callers pair None with None above, so surface empty here.
            None
        }
        syn::PathArguments::Parenthesized(_) => None,
    }
}

fn unify_ga(
    a: &syn::GenericArgument,
    a_vars: &[String],
    b: &syn::GenericArgument,
    b_vars: &[String],
    a_bind: &mut HashMap<String, String>,
    b_bind: &mut HashMap<String, String>,
) -> bool {
    match (a, b) {
        (syn::GenericArgument::Type(x), syn::GenericArgument::Type(y)) => {
            unify_ty(x, a_vars, y, b_vars, a_bind, b_bind)
        }
        (syn::GenericArgument::Const(x), syn::GenericArgument::Const(y)) => {
            x.to_token_stream().to_string() == y.to_token_stream().to_string()
        }
        (syn::GenericArgument::Lifetime(_), syn::GenericArgument::Lifetime(_)) => true,
        _ => a.to_token_stream().to_string() == b.to_token_stream().to_string(),
    }
}

/// Trailing type/const params a self type left out, filled from the struct
/// defaults so `impl Foo<T>` meets `impl Foo<T, u8>` for `struct Foo<T, U = u8>`.
fn pad_defaults(ty: &syn::Type, defaults: Option<&Vec<Option<String>>>) -> syn::Type {
    let Some(defaults) = defaults else {
        return erase_lifetimes_type(ty);
    };
    let mut ty = ty.clone();
    if let syn::Type::Path(p) = &mut ty {
        if let Some(seg) = p.path.segments.last_mut() {
            let written = match &seg.arguments {
                syn::PathArguments::AngleBracketed(ab) => ab
                    .args
                    .iter()
                    .filter(|a| !matches!(a, syn::GenericArgument::Lifetime(_)))
                    .count(),
                syn::PathArguments::None => 0,
                syn::PathArguments::Parenthesized(_) => return erase_lifetimes_type(&ty),
            };
            let mut filled = Vec::new();
            for d in defaults.iter().skip(written) {
                match d.as_ref().and_then(|s| syn::parse_str::<syn::Type>(s).ok()) {
                    Some(t) => filled.push(syn::GenericArgument::Type(t)),
                    None => break,
                }
            }
            if !filled.is_empty() {
                match &mut seg.arguments {
                    syn::PathArguments::AngleBracketed(ab) => {
                        ab.args.extend(filled);
                    }
                    _ => {
                        seg.arguments = syn::PathArguments::AngleBracketed(
                            syn::AngleBracketedGenericArguments {
                                colon2_token: None,
                                lt_token: Default::default(),
                                args: filled.into_iter().collect(),
                                gt_token: Default::default(),
                            },
                        );
                    }
                }
            }
        }
    }
    erase_lifetimes_type(&ty)
}

/// Lifetimes never tell two impls apart: drop them at every path level.
fn erase_lifetimes_type(ty: &syn::Type) -> syn::Type {
    match ty {
        syn::Type::Path(p) => {
            let mut p = p.clone();
            for seg in &mut p.path.segments {
                if let syn::PathArguments::AngleBracketed(ab) = &mut seg.arguments {
                    for a in &mut ab.args {
                        if let syn::GenericArgument::Type(t) = a {
                            *t = erase_lifetimes_type(t);
                        }
                    }
                    ab.args
                        .retain(|a| !matches!(a, syn::GenericArgument::Lifetime(_)));
                    if ab.args.is_empty() {
                        seg.arguments = syn::PathArguments::None;
                    }
                }
            }
            syn::Type::Path(p)
        }
        other => other.clone(),
    }
}

/// Trailing type/const defaults per struct, by last path segment name.
fn struct_type_defaults(items: &[syn::Item]) -> HashMap<String, Vec<Option<String>>> {
    let mut map = HashMap::new();
    for item in items {
        let syn::Item::Struct(s) = item else {
            continue;
        };
        let defaults = s
            .generics
            .params
            .iter()
            .filter_map(|p| match p {
                syn::GenericParam::Type(t) => Some(
                    t.default
                        .as_ref()
                        .map(|(_, d)| d.to_token_stream().to_string()),
                ),
                syn::GenericParam::Const(c) => Some(
                    c.default
                        .as_ref()
                        .map(|(_, d)| d.to_token_stream().to_string()),
                ),
                syn::GenericParam::Lifetime(_) => None,
            })
            .collect();
        map.insert(s.ident.to_string(), defaults);
    }
    map
}

fn clash(name: String, prev: Origin, origin: Origin) -> GenerateError {
    let mark = |reason: &str| GenerateError::InvalidConfiguration {
        option: "with_extension_consts".into(),
        value: name.clone(),
        reason: reason.into(),
    };
    match (prev, origin) {
        (Origin::Variant, Origin::Mark) => mark(
            "an extension mark spells a variant of this enum, so the path would always \
             mean the variant",
        ),
        (Origin::Mark, _) | (_, Origin::Mark) => mark(
            "an extension mark spells a const this type already has \
             (a generated const, another mark or a hook's)",
        ),
        _ => GenerateError::NameClash {
            name,
            reason: "two schema names map to this const (a field named `template` gives \
                     `TEMPLATE_ID`, which every message has), or a hook emits it again; \
                     rename the field or the hook's const"
                .into(),
        },
    }
}

/// True for a const [`push_const`] emitted: its doc names an extension
/// attribute or element.
fn is_mark_const(c: &syn::ImplItemConst) -> bool {
    c.attrs.iter().any(|a| match &a.meta {
        syn::Meta::NameValue(nv) if nv.path.is_ident("doc") => match &nv.value {
            syn::Expr::Lit(syn::ExprLit {
                lit: syn::Lit::Str(s),
                ..
            }) => {
                let doc = s.value();
                doc.starts_with("Extension attribute `") || doc.contains(" extension element `<")
            }
            _ => false,
        },
        _ => false,
    })
}

/// `jsonDefaultValue` / `max-qty` -> `JSON_DEFAULT_VALUE` / `MAX_QTY`.
pub(crate) fn upper_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    let mut prev_lower = false;
    for c in s.chars() {
        if c.is_ascii_uppercase() && prev_lower {
            out.push('_');
        }
        prev_lower = c.is_ascii_lowercase() || c.is_ascii_digit();
        out.push(if c.is_ascii_alphanumeric() {
            c.to_ascii_uppercase()
        } else {
            '_'
        });
    }
    out
}

/// Consts for one message: the `<message>` element and its fields, groups
/// and var-data on `schema_marker` (the `{Name}Schema` unit), then each group's
/// fields, nested groups and var-data on that group's decoder, recursively.
///
/// # Errors
///
/// A mark whose name maps to no const name.
pub(crate) fn message_consts(
    msg: &MessageStructure,
    schema_marker: &str,
    multi_message: bool,
) -> Result<TokenStream, GenerateError> {
    let marker = syn::Ident::new(schema_marker, proc_macro2::Span::call_site());
    let what = format!("message `{}`", msg.name);
    let mut rows = vec![Marked {
        prefix: None,
        what: what.clone(),
        extensions: &msg.extensions,
    }];
    rows.extend(block_rows(&what, &msg.fields, &msg.groups, &msg.var_data));
    let mut ts = consts_impl(&quote! { #marker }, &rows)?;
    let prefix = if multi_message {
        to_pascal_case(&msg.name)
    } else {
        String::new()
    };
    for g in &msg.groups {
        group_consts(g, &format!("{prefix}{}", to_pascal_case(&g.name)), &mut ts)?;
    }
    Ok(ts)
}

fn group_consts(g: &MessageGroup, scoped: &str, ts: &mut TokenStream) -> Result<(), GenerateError> {
    let decoder = syn::Ident::new(&format!("{scoped}Decoder"), proc_macro2::Span::call_site());
    let what = format!("group `{}`", g.name);
    let rows = block_rows(&what, &g.fields, &g.groups, &g.var_data);
    // The group decoder has a defaulted type parameter: the elided lifetime
    // is enough for the impl header, and `Decoder::CONST` paths still resolve.
    ts.extend(consts_impl(&quote! { #decoder<'_> }, &rows)?);
    for ng in &g.groups {
        group_consts(ng, &format!("{scoped}{}", to_pascal_case(&ng.name)), ts)?;
    }
    Ok(())
}

fn block_rows<'a>(
    owner: &str,
    fields: &'a [crate::structured_ir::MessageField],
    groups: &'a [MessageGroup],
    var_data: &'a [crate::structured_ir::MessageVarData],
) -> Vec<Marked<'a>> {
    let row = |name: &str, kind: &str, extensions: &'a Extensions| Marked {
        prefix: Some(upper_snake(name)),
        what: format!("{kind} `{name}` of {owner}"),
        extensions,
    };
    fields
        .iter()
        .map(|f| row(&f.name, "field", &f.extensions))
        .chain(groups.iter().map(|g| row(&g.name, "group", &g.extensions)))
        .chain(var_data.iter().map(|d| row(&d.name, "data", &d.extensions)))
        .collect()
}

/// Consts for an enum, set or composite: the element itself, then each
/// value, choice or member (`(schema name, marks)`).
///
/// # Errors
///
/// A mark whose name maps to no const name.
pub(crate) fn type_consts(
    type_name: &str,
    own: &Extensions,
    kind: &str,
    members: &[(&str, &Extensions)],
) -> Result<TokenStream, GenerateError> {
    let ident = syn::Ident::new(&to_pascal_case(type_name), proc_macro2::Span::call_site());
    let what = format!("{kind} `{type_name}`");
    let mut rows = vec![Marked {
        prefix: None,
        what: what.clone(),
        extensions: own,
    }];
    rows.extend(members.iter().map(|(name, extensions)| Marked {
        prefix: Some(upper_snake(name)),
        what: format!("`{name}` of {what}"),
        extensions,
    }));
    consts_impl(&quote! { #ident }, &rows)
}

#[cfg(test)]
mod tests {
    use super::upper_snake;

    #[test]
    fn upper_snake_splits_camel_case_and_punctuation() {
        assert_eq!(upper_snake("jsonDefaultValue"), "JSON_DEFAULT_VALUE");
        assert_eq!(upper_snake("maxQty"), "MAX_QTY");
        assert_eq!(upper_snake("max-qty.v2"), "MAX_QTY_V2");
        assert_eq!(upper_snake("MAX_ASSET"), "MAX_ASSET");
        assert_eq!(upper_snake("qtyExponent8"), "QTY_EXPONENT8");
    }
}

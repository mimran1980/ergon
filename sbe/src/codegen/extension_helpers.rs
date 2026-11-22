//! Opt-in built-in generators driven by schema extension attributes.
//!
//! One emission predicate per knob — the knob is configured **and** the item
//! carries a matching attribute — so a schema without the marks generates
//! byte-identical output. Attribute matching is by local name in any
//! namespace, exactly like [`crate::Extensions::attr`]. Decode/static side
//! only: nothing here reaches an encoder, a setter or the wire.

use super::GenerateError;
use super::conversion_helpers::{
    DECODER_RESERVED, domain_accessor_ident, field_has_conversion_free, find_domain_type,
    resolve_field_ident,
};
use super::converter_impls::is_optional_domain_field;
use super::runtime::{to_pascal_case, to_snake_case};
use crate::ir::{Presence, Signal, Token};
use crate::structured_ir::{FieldType, MessageField, MessageGroup, MessageStructure, rust_type};
use proc_macro2::TokenStream;
use quote::quote;

/// Whether any value of `tokens` (an enum's or set's value tokens) carries
/// the extension attribute `attr`. The single predicate behind
/// [`enum_labels_impl`] and [`set_labels_impl`]: a value token without a
/// constant is not a value and never counts.
fn any_value_labels(tokens: &[Token], attr: &str) -> bool {
    tokens
        .iter()
        .filter(|t| t.signal == Signal::Encoding && t.encoding.constant_value.is_some())
        .any(|t| t.extensions.attr(attr).is_some())
}

/// `impl {Enum} { pub const fn {method}(self) -> &'static str { … } }`,
/// one arm per variant: the extension label when the value carries `attr`,
/// its variant name otherwise — byte-for-byte what the built-in `as_str`
/// returns, so enabling the knob never flips an unmarked value's rendering —
/// `"NullVal"` for the unknown variant.
/// `None` when no value carries the attribute (nothing is emitted).
pub(crate) fn enum_labels_impl(tokens: &[Token], attr: &str, method: &str) -> Option<TokenStream> {
    if !any_value_labels(tokens, attr) {
        return None;
    }
    let ident = syn::Ident::new(
        &to_pascal_case(&tokens[0].name),
        proc_macro2::Span::call_site(),
    );
    let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
    let doc = format!("Label from the schema extension attribute `{attr}`.");
    let mut arms: Vec<TokenStream> = Vec::new();
    for t in tokens.iter().filter(|t| t.signal == Signal::Encoding) {
        // Mirror `generate_enum`: a value token without a constant is not a
        // variant, so it gets no arm either.
        if t.encoding.constant_value.is_none() {
            continue;
        }
        let variant = syn::Ident::new(&to_pascal_case(&t.name), proc_macro2::Span::call_site());
        // Fallback = the variant's own name, i.e. `stringify!(#variant)` of
        // the built-in `as_str` — not the raw schema name, which would flip
        // `as_str`/`Display` for every unmarked value the knob touches.
        let label = t
            .extensions
            .attr(attr)
            .map(str::to_string)
            .unwrap_or_else(|| to_pascal_case(&t.name));
        let lit = syn::LitStr::new(&label, proc_macro2::Span::call_site());
        arms.push(quote! { Self::#variant => #lit, });
    }
    arms.push(quote! { Self::NullVal => "NullVal", });
    Some(quote! {
        impl #ident {
            #[doc = #doc]
            #[must_use = "discarding this value is almost always a mistake"]
            pub const fn #method_ident(self) -> &'static str {
                match self { #(#arms)* }
            }
        }
    })
}

/// `impl {Set} { pub const fn {method}(self) -> &'static str { … } }`,
/// one arm per single choice bit: the extension label when the choice
/// carries `attr`, its raw schema name otherwise; any other word (the
/// empty set or a combination of choices) returns `"UNKNOWN"`.
/// `None` when no choice carries the attribute.
pub(crate) fn set_labels_impl(tokens: &[Token], attr: &str, method: &str) -> Option<TokenStream> {
    if !any_value_labels(tokens, attr) {
        return None;
    }
    let ident = syn::Ident::new(
        &to_pascal_case(&tokens[0].name),
        proc_macro2::Span::call_site(),
    );
    let method_ident = syn::Ident::new(method, proc_macro2::Span::call_site());
    let doc = format!("Label from the schema extension attribute `{attr}`.");
    let mut arms: Vec<TokenStream> = Vec::new();
    for t in tokens.iter().filter(|t| t.signal == Signal::Encoding) {
        let Some(val) = t.encoding.constant_value.as_ref() else {
            continue;
        };
        // The schema value of a choice is its bit index; the match runs on
        // the set word, exactly as `generate_set` masks it. An index past
        // 127 would panic the generator, so it yields word 0 and the
        // consumer's duplicate-pattern error names the bad schema.
        let bit: u8 = val.parse().unwrap_or(0);
        let word = 1u128.checked_shl(u32::from(bit)).unwrap_or(0);
        // Unsuffixed: the pattern must take the set's word type, not `u128`.
        let word_lit = proc_macro2::Literal::u128_unsuffixed(word);
        let label = t.extensions.attr(attr).unwrap_or(&t.name);
        let lit = syn::LitStr::new(label, proc_macro2::Span::call_site());
        arms.push(quote! { #word_lit => #lit, });
    }
    Some(quote! {
        impl #ident {
            #[doc = #doc]
            #[must_use = "discarding this value is almost always a mistake"]
            pub const fn #method_ident(self) -> &'static str {
                match self.0 { #(#arms)* _ => "UNKNOWN" }
            }
        }
    })
}

/// Knob state the message walk reads. Built once per message in `gen_schema`
/// so the three locations (message, group entry, nested group entry) share
/// one decision path.
pub(crate) struct KnobWalk<'a> {
    /// [`crate::GenerationConfig::with_exponent_links`] attribute local name.
    pub(crate) exponent_links: Option<&'a str>,
    /// [`crate::GenerationConfig::with_field_extension_defaults`] attribute
    /// local names: `(default, omit-null)`.
    pub(crate) field_defaults: Option<(&'a str, &'a str)>,
    /// Selectors that rename a field's raw accessor (`*_wire`) or retype it.
    pub(crate) conv_sels: &'a [crate::ConversionSelector],
    /// Configured domain types: a matched field's bare accessor returns one.
    pub(crate) domain_types: &'a [(crate::ConversionSelector, String)],
    pub(crate) null_as_option: &'a [crate::ConversionSelector],
    pub(crate) all_enums_as_option: bool,
    /// The schema has more than one message, so message-level group type
    /// names carry the message prefix (the rule `generate_message_decoder`
    /// applies to `group_unique_names`).
    pub(crate) multi_message: bool,
}

/// Emit the extension helpers owned by `msg`: B2 pair helpers on the message
/// decoder and B3 default consts on `schema_marker` (the `{Msg}Schema` unit
/// `generate_message_decoder` chose), plus both knobs on each group's
/// decoder/entry decoder recursively. Empty (and parse-cheap) when no knob
/// is set.
pub(crate) fn message_extension_impls(
    msg: &MessageStructure,
    schema_marker: &str,
    walk: &KnobWalk<'_>,
) -> Result<TokenStream, GenerateError> {
    let mut ts = TokenStream::new();
    if let Some(attr) = walk.exponent_links {
        let owner = syn::Ident::new(
            &format!("{}Decoder", to_pascal_case(&msg.name)),
            proc_macro2::Span::call_site(),
        );
        emit_pair_helpers(
            &msg.fields,
            &msg.fields,
            &msg.name,
            &owner,
            DECODER_RESERVED,
            attr,
            walk,
            &mut ts,
        )?;
    }
    if walk.field_defaults.is_some() {
        let marker = syn::Ident::new(schema_marker, proc_macro2::Span::call_site());
        let owner_ty: syn::Type = syn::parse_quote!(#marker);
        emit_field_default_consts(&msg.fields, &owner_ty, walk, &mut ts);
    }
    let msg_pascal = to_pascal_case(&msg.name);
    for g in &msg.groups {
        let raw = to_pascal_case(&g.name);
        let scoped = if walk.multi_message {
            format!("{msg_pascal}{raw}")
        } else {
            raw
        };
        walk_group(g, &scoped, &msg.fields, walk, &mut ts)?;
    }
    Ok(ts)
}

/// B2/B3 for one group's fields (pairs on its entry decoder — the
/// accessors live there; default consts on the group decoder), then each
/// nested group under its parent-scoped name. `msg_fields` is the enclosing
/// message's field list, used only to recognise cross-block exponent refs.
fn walk_group(
    g: &MessageGroup,
    scoped: &str,
    msg_fields: &[MessageField],
    walk: &KnobWalk<'_>,
    ts: &mut TokenStream,
) -> Result<(), GenerateError> {
    if let Some(attr) = walk.exponent_links {
        let owner = syn::Ident::new(
            &format!("{scoped}EntryDecoder"),
            proc_macro2::Span::call_site(),
        );
        // Group entries do not rename against the static reserved list.
        emit_pair_helpers(&g.fields, msg_fields, scoped, &owner, &[], attr, walk, ts)?;
    }
    if walk.field_defaults.is_some() {
        let decoder = syn::Ident::new(&format!("{scoped}Decoder"), proc_macro2::Span::call_site());
        // The group decoder carries a defaulted type parameter, so the impl
        // header needs the elided lifetime; `Marker::CONST` paths still
        // resolve through the defaults.
        let owner_ty: syn::Type = syn::parse_quote!(#decoder<'_>);
        emit_field_default_consts(&g.fields, &owner_ty, walk, ts);
    }
    for ng in &g.groups {
        let nested = format!("{scoped}{}", to_pascal_case(&ng.name));
        walk_group(ng, &nested, msg_fields, walk, ts)?;
    }
    Ok(())
}

/// One `{field}_with_exponent()` per field of `fields` that carries `attr`.
/// Validation is fail-closed: a dangling or type-invalid reference is
/// [`GenerateError::ExtensionReference`], never a silent skip. The one
/// recognised no-op is a *cross-block* reference — a group field whose
/// target lives in the enclosing message (`msg_fields`): the pair spans two
/// decoders, so no helper is emitted, but the reference exists and is not
/// an error. Everything must still resolve inside the message.
fn emit_pair_helpers(
    fields: &[MessageField],
    msg_fields: &[MessageField],
    item: &str,
    owner: &syn::Ident,
    reserved: &[&str],
    attr: &str,
    walk: &KnobWalk<'_>,
    ts: &mut TokenStream,
) -> Result<(), GenerateError> {
    for f in fields {
        let Some(target_name) = f.extensions.attr(attr) else {
            continue;
        };
        let ext_ref = |reason: String| GenerateError::ExtensionReference {
            item: item.to_string(),
            field: f.name.clone(),
            target: target_name.to_string(),
            reason,
        };
        let Some(target) = fields.iter().find(|t| t.name == target_name) else {
            // Not in this block. A target in the enclosing message is a
            // real cross-block reference (venue schemas declare exponents
            // at message level and mantissas inside groups): skip — the
            // entry decoder cannot reach message-level state. Anything
            // else is dangling.
            if msg_fields.iter().any(|t| t.name == target_name) {
                continue;
            }
            return Err(ext_ref(
                "no field with that name exists in the same block".into(),
            ));
        };
        validate_pair_member(f).map_err(ext_ref)?;
        validate_pair_member(target).map_err(ext_ref)?;
        let (mantissa_ident, mantissa_ty) = pair_member(f, reserved, walk);
        let (exponent_ident, exponent_ty) = pair_member(target, reserved, walk);
        let helper = syn::Ident::new(
            &format!("{}_with_exponent", to_snake_case(&f.name)),
            proc_macro2::Span::call_site(),
        );
        let doc = format!(
            "`{}` with the exponent field its schema extension names.",
            f.name
        );
        ts.extend(quote! {
            impl #owner<'_> {
                #[doc = #doc]
                #[inline]
                pub fn #helper(&self) -> (#mantissa_ty, #exponent_ty) {
                    (self.#mantissa_ident(), self.#exponent_ident())
                }
            }
        });
    }
    Ok(())
}

/// B3: `{FIELD}_SCHEMA_DEFAULT` / `{FIELD}_SCHEMA_OMIT_NULL` associated
/// consts on `owner_ty` for every field of `fields` carrying either
/// attribute. Values are the schema's own strings (no parse); the omit flag
/// is the shared truthiness rule. No field carries either ⇒ no impl block.
fn emit_field_default_consts(
    fields: &[MessageField],
    owner_ty: &syn::Type,
    walk: &KnobWalk<'_>,
    ts: &mut TokenStream,
) {
    let Some((default_attr, omit_attr)) = walk.field_defaults else {
        return;
    };
    let mut consts = TokenStream::new();
    for f in fields {
        let upper = super::runtime::to_upper_snake_case(&f.name);
        let span = proc_macro2::Span::call_site();
        if let Some(value) = f.extensions.attr(default_attr) {
            let ident = syn::Ident::new(&format!("{upper}_SCHEMA_DEFAULT"), span);
            let doc = format!(
                "Schema default for `{}`, extension attribute `{default_attr}`.",
                f.name
            );
            let lit = syn::LitStr::new(value, span);
            consts.extend(quote! {
                #[doc = #doc]
                pub const #ident: Option<&'static str> = Some(#lit);
            });
        }
        if let Some(value) = f.extensions.attr(omit_attr) {
            let ident = syn::Ident::new(&format!("{upper}_SCHEMA_OMIT_NULL"), span);
            let doc = format!(
                "`{}` is omitted rather than sent as null (extension `{omit_attr}`).",
                f.name
            );
            let truthy = crate::structured_ir::is_truthy_name(value);
            consts.extend(quote! {
                #[doc = #doc]
                pub const #ident: bool = #truthy;
            });
        }
    }
    if !consts.is_empty() {
        ts.extend(quote! {
            impl #owner_ty {
                #consts
            }
        });
    }
}

/// A pair member must be a non-constant fixed primitive integer — a scalar
/// or a fixed-size array (the venue's `mantissa128` is a 16-byte
/// little-endian magnitude). Anything else (composite, enum, float, char,
/// constant) is an [`GenerateError::ExtensionReference`] with this reason.
fn validate_pair_member(f: &MessageField) -> Result<(), String> {
    let integer = matches!(
        &f.field_type,
        FieldType::Primitive(p, _) if matches!(
            p,
            crate::ir::PrimitiveType::Int8
                | crate::ir::PrimitiveType::UInt8
                | crate::ir::PrimitiveType::Int16
                | crate::ir::PrimitiveType::UInt16
                | crate::ir::PrimitiveType::Int32
                | crate::ir::PrimitiveType::UInt32
                | crate::ir::PrimitiveType::Int64
                | crate::ir::PrimitiveType::UInt64
        )
    );
    if !integer {
        return Err("must be a fixed primitive integer field (scalar or array)".into());
    }
    if f.presence == Presence::Constant {
        return Err("a constant field cannot take part in an exponent link".into());
    }
    Ok(())
}

/// The accessor a pair helper calls and the tuple element type it yields:
/// exactly what `self.{field}()` returns — `Result`-wrapped, domain-typed and
/// `Option`-wrapped the same way the generated accessor is.
fn pair_member(
    f: &MessageField,
    reserved: &[&str],
    walk: &KnobWalk<'_>,
) -> (syn::Ident, syn::Type) {
    let snake = to_snake_case(&f.name);
    let has_conv = field_has_conversion_free(f, walk.conv_sels);
    if let Some(domain) = find_domain_type(f, walk.domain_types) {
        // The domain getter owns the bare name; the raw one moved to `*_wire`.
        // It returns `Result<Domain, …>` (`Result<Option<Domain>, …>` when
        // optional) — mirror that, or the helper's tuple lies to the caller.
        let ident = domain_accessor_ident(&snake, reserved);
        let base: syn::Type = syn::parse_str(domain)
            .unwrap_or_else(|e| panic!("invalid domain type path {domain}: {e}"));
        let wire = wire_type_of(f);
        let ty = if is_optional_domain_field(f, walk.null_as_option, walk.all_enums_as_option) {
            syn::parse_quote! { Result<Option<#base>, <#base as TryFromSbe<#wire>>::Error> }
        } else {
            syn::parse_quote! { Result<#base, <#base as TryFromSbe<#wire>>::Error> }
        };
        return (ident, ty);
    }
    let wire_name = has_conv.then(|| format!("{snake}_wire"));
    let ident = resolve_field_ident(&snake, &wire_name, reserved);
    let base = raw_accessor_type(f);
    (ident, base)
}

/// The wire encoding a pair member decodes from: `u64`, `[u8; 16]` — never
/// `Option`-wrapped (absence is the decoder's concern, not the wire's), and
/// the `TryFromSbe<Wire>` parameter a domain getter's error type names.
fn wire_type_of(f: &MessageField) -> syn::Type {
    let FieldType::Primitive(p, len) = &f.field_type else {
        unreachable!("validate_pair_member rejected non-primitive fields");
    };
    match len {
        Some(n) => {
            let elem = rust_type(*p);
            let lit = proc_macro2::Literal::usize_unsuffixed(*n);
            syn::parse_str::<syn::Type>(&format!("[{elem}; {lit}]"))
                .unwrap_or_else(|e| panic!("array type [{elem}; {n}]: {e}"))
        }
        None => {
            syn::parse_str(rust_type(*p)).unwrap_or_else(|e| panic!("rust type for {p:?}: {e}"))
        }
    }
}

/// The wire accessor's return type for a non-constant fixed primitive
/// member: `Option<T>` exactly when [`is_optional_domain_field`] says the
/// raw accessor is `Option` (its primitive arm: version-gated or optional).
/// A fixed-size array member yields `[T; N]` — arrays are never `Option`.
fn raw_accessor_type(f: &MessageField) -> syn::Type {
    let base = wire_type_of(f);
    if is_optional_domain_field(f, &[], false) {
        syn::parse_quote! { Option<#base> }
    } else {
        base
    }
}

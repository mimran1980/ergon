//! `TryFromSbe` / `TryToSbe` trait emission and built-in domain-type impls.

use super::runtime::{to_pascal_case, to_snake_case};
use crate::structured_ir::SchemaElements;

/// Emit `TryFromSbe` / `TryToSbe` traits into the generated sbe_rt module.
///
/// `#[diagnostic::on_unimplemented]` (stable since 1.78) replaces the default
/// "the trait bound `T: TryFromSbe<Wire>` is not satisfied" with a message
/// naming the exact missing impl. It fires for both causes: a
/// `with_conversion` caller who forgot the impl for their chosen `T`, and a
/// `DomainImpl::Manual` field — the generated accessor's own doc comment
/// (see `converter_impls.rs`) carries a ready-to-paste snippet for the
/// latter, which the note below points at.
pub(crate) fn emit_conversion_traits(src: &mut String) {
    src.push_str(
        "/// Convert from a wire type to an application type.\n\
         #[diagnostic::on_unimplemented(\n\
             message = \"`{Self}` has no `TryFromSbe<{Wire}>` impl\",\n\
             label = \"missing `impl TryFromSbe<{Wire}> for {Self}`\",\n\
             note = \"if this field uses DomainImpl::Manual, the generated domain \
accessor's doc comment has a ready-to-paste starting point\"\n\
         )]\n\
         pub trait TryFromSbe<Wire>: Sized {\n\
             type Error: core::fmt::Debug + core::fmt::Display;\n\
             fn try_from_sbe(wire: Wire) -> Result<Self, Self::Error>;\n\
         }\n\n\
         /// Convert from an application type to a wire type.\n\
         #[diagnostic::on_unimplemented(\n\
             message = \"`{Self}` has no `TryToSbe<{Wire}>` impl\",\n\
             label = \"missing `impl TryToSbe<{Wire}> for {Self}`\",\n\
             note = \"if this field uses DomainImpl::Manual, the generated domain \
accessor's doc comment has a ready-to-paste starting point\"\n\
         )]\n\
         pub trait TryToSbe<Wire> {\n\
             type Error: core::fmt::Debug + core::fmt::Display;\n\
             fn try_to_sbe(&self) -> Result<Wire, Self::Error>;\n\
         }\n\n",
    );
}

/// `impl TryFromSbe<BoolEnum> for bool` / `impl TryToSbe<BoolEnum> for bool`
/// for one boolean-mapped enum. Shared by the real Generated-mode emission
/// and the Manual-mode doc-comment snippet (same tokens, different sink).
fn bool_impl_tokens(bt_ident: &syn::Ident) -> proc_macro2::TokenStream {
    quote::quote! {
        impl TryFromSbe<#bt_ident> for bool {
            type Error = &'static str;
            #[inline]
            fn try_from_sbe(wire: #bt_ident) -> Result<Self, Self::Error> {
                wire.as_bool().ok_or("null or unknown boolean discriminant")
            }
        }
        impl TryToSbe<#bt_ident> for bool {
            type Error = &'static str;
            #[inline]
            fn try_to_sbe(&self) -> Result<#bt_ident, Self::Error> {
                Ok(#bt_ident::from(*self))
            }
        }
    }
}

/// `impl TryFromSbe<DecComposite> for rust_decimal::Decimal` / `TryToSbe`
/// for one Decimal-shaped composite. Shared by the real Generated-mode
/// emission and the Manual-mode doc-comment snippet.
fn decimal_impl_tokens(
    dec_ident: &syn::Ident,
    exponent_is_constant: bool,
    mantissa_is_optional: bool,
) -> proc_macro2::TokenStream {
    // A constant exponent is not on the wire, so the mantissa must be
    // rescaled to it: value = m × 10^-scale = M × 10^exponent. Exact or an
    // error, never rounded. scale == -exponent (the usual case) is a no-op.
    // Kept inline: moving the rescale out of line measured slower.
    let to_sbe_body: proc_macro2::TokenStream = if exponent_is_constant {
        quote::quote! {
            let shift = -(self.scale() as i32) - i32::from(#dec_ident::new(0).exponent());
            let mantissa = self.mantissa();
            let mantissa = if shift == 0 {
                mantissa
            } else if shift > 0 {
                10i128.checked_pow(shift as u32)
                    .and_then(|pow| mantissa.checked_mul(pow))
                    .ok_or("Decimal mantissa overflow")?
            } else {
                let pow = 10i128.checked_pow(shift.unsigned_abs())
                    .ok_or("Decimal exponent overflow")?;
                if mantissa % pow != 0 {
                    sbe_rt::cold();
                    return Err("Decimal has more digits than the constant exponent allows");
                }
                mantissa / pow
            };
            let mantissa: i64 = mantissa
                .try_into()
                .map_err(|_| "Decimal mantissa overflow i64")?;
            Ok(#dec_ident::new(mantissa))
        }
    } else {
        quote::quote! {
            let mantissa: i64 = self.mantissa()
                .try_into()
                .map_err(|_| "Decimal mantissa overflow i64")?;
            Ok(#dec_ident::new(mantissa, -(self.scale() as i8)))
        }
    };
    // A mantissa with presence="optional" (and a schema nullValue) has a
    // genuine null image — `mantissa()` decodes it as `Option<i64>`. The
    // wire null sentinel is not a valid Decimal, so it fails closed as a
    // typed error rather than silently decoding as a huge/wrong number.
    let mantissa_expr: proc_macro2::TokenStream = if mantissa_is_optional {
        quote::quote! { wire.mantissa().ok_or("null Decimal mantissa")? as i128 }
    } else {
        quote::quote! { wire.mantissa() as i128 }
    };
    quote::quote! {
        impl TryFromSbe<#dec_ident> for rust_decimal::Decimal {
            type Error = &'static str;
            #[inline]
            fn try_from_sbe(wire: #dec_ident) -> Result<Self, Self::Error> {
                let mantissa = #mantissa_expr;
                let exponent = wire.exponent() as i32;
                // SBE Decimal: negative exponent = fractional places (e.g.
                // -2 → scale 2). Positive exponent = magnitude (mantissa ×
                // 10^exp). rust_decimal scale must be a positive u32 ≤ 28.
                let (mantissa, scale) = if exponent < 0 {
                    let scale = exponent.unsigned_abs();
                    (mantissa, scale)
                } else {
                    let pow = 10i128.checked_pow(exponent as u32)
                        .ok_or("Decimal exponent overflow")?;
                    let scaled = mantissa.checked_mul(pow)
                        .ok_or("Decimal mantissa overflow")?;
                    (scaled, 0)
                };
                rust_decimal::Decimal::from_i128_with_scale(mantissa, scale)
                    .try_into()
                    .map_err(|_| "Decimal overflow")
            }
        }
        impl TryToSbe<#dec_ident> for rust_decimal::Decimal {
            type Error = &'static str;
            #[inline]
            fn try_to_sbe(&self) -> Result<#dec_ident, Self::Error> {
                #to_sbe_body
            }
        }
    }
}

/// `impl TryFromSbe<u64> for chrono::DateTime<Utc>` / `TryToSbe`. Shared by
/// the real Generated-mode emission and the Manual-mode doc-comment snippet.
fn chrono_impl_tokens() -> proc_macro2::TokenStream {
    quote::quote! {
        impl TryFromSbe<u64> for chrono::DateTime<chrono::Utc> {
            type Error = &'static str;
            #[inline]
            fn try_from_sbe(wire: u64) -> Result<Self, Self::Error> {
                let secs = (wire / 1_000_000_000) as i64;
                let nsec = (wire % 1_000_000_000) as u32;
                chrono::DateTime::from_timestamp(secs, nsec)
                    .ok_or("timestamp out of range for DateTime<Utc>")
            }
        }
        impl TryToSbe<u64> for chrono::DateTime<chrono::Utc> {
            type Error = &'static str;
            #[inline]
            fn try_to_sbe(&self) -> Result<u64, Self::Error> {
                let total_nanos = self.timestamp_nanos_opt()
                    .ok_or("timestamp_nanos overflow")?;
                u64::try_from(total_nanos)
                    .map_err(|_| "timestamp out of u64 range")
            }
        }
    }
}

/// Pretty-print a token stream of one or more items as standalone Rust
/// source, for embedding in a doc comment. Falls back to the raw
/// (unformatted) token text if it somehow fails to parse as a file — still
/// correct Rust, just ugly, and never worth failing codegen over.
fn pretty_print(ts: &proc_macro2::TokenStream) -> String {
    syn::parse_str::<syn::File>(&ts.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| ts.to_string())
}

/// Emit `TryFromSbe` / `TryToSbe` impls for well-known **domain-type** mappings
/// (bool ↔ BooleanType, rust_decimal ↔ Decimal, chrono ↔ u64/UTCTimestamp).
///
/// These built-in impls are gated on `domain_types`, not bare `conversions`:
/// `with_conversion` alone keeps the seam dependency-free so callers can plug
/// any adapter (see samples/sbe-feature-tour `FixedPrice` / app-side
/// rust_decimal). `with_domain_type` opts into a concrete app type *and* these
/// well-known impls — unless the caller chose [`crate::DomainImpl::Manual`],
/// in which case use [`generate_manual_impl_snippets`] instead.
pub(crate) fn generate_conversion_impl_blocks(
    elements: &SchemaElements,
    _conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    manual_impl_selectors: &[crate::ConversionSelector],
) -> String {
    let mut out = String::new();
    let span = proc_macro2::Span::call_site();

    // Built-ins require an explicit domain-type mapping (not conversion-only).
    // Emit TryFromSbe / TryToSbe for EVERY boolean enum mapped to "bool",
    // not just the one literally named BooleanType. Skip selectors opted into
    // DomainImpl::Manual — the caller supplies this impl.
    for (sel, ty) in domain_types {
        if ty != "bool" || manual_impl_selectors.contains(sel) {
            continue;
        }
        let bt_name = match sel {
            crate::ConversionSelector::NamedType(n) => to_pascal_case(n),
            _ => continue,
        };
        let bt_ident = syn::Ident::new(&bt_name, span);
        out.push_str(&bool_impl_tokens(&bt_ident).to_string());
    }

    let has_chrono_conv = domain_types.iter().any(|(sel, _)| {
        matches!(sel, crate::ConversionSelector::SemanticType(st) if st == "UTCTimestamp")
            && !manual_impl_selectors.contains(sel)
    });

    // Emit rust_decimal TryFromSbe/TryToSbe for EVERY composite mapped to
    // "rust_decimal::Decimal" — not just one literally named "Decimal". A
    // schema with Decimal64 / Decimal128 (or any other decimal composite)
    // gets one impl per composite, keyed to that composite's ident. Skip
    // selectors opted into DomainImpl::Manual.
    for (sel, ty) in domain_types {
        if ty != "rust_decimal::Decimal" || manual_impl_selectors.contains(sel) {
            continue;
        }
        let comp_name = match sel {
            crate::ConversionSelector::NamedType(n) => n.as_str(),
            _ => continue,
        };
        let Some(dec_composite) = elements.composites.iter().find(|c| c[0].name == comp_name)
        else {
            continue;
        };
        let dec_ident = syn::Ident::new(&to_pascal_case(comp_name), span);
        let exponent_is_constant = composite_exponent_is_constant(dec_composite);
        let mantissa_is_optional = composite_mantissa_is_optional(dec_composite);
        let ts = decimal_impl_tokens(&dec_ident, exponent_is_constant, mantissa_is_optional);
        out.push_str(&ts.to_string());
    }

    if has_chrono_conv {
        out.push_str(&chrono_impl_tokens().to_string());
    }

    out
}

fn composite_exponent_is_constant(dec_composite: &[crate::ir::Token]) -> bool {
    dec_composite
        .iter()
        .find(|t| t.name == "exponent")
        .map(|t| t.encoding.presence == crate::ir::Presence::Constant)
        .unwrap_or(false)
}

fn composite_mantissa_is_optional(dec_composite: &[crate::ir::Token]) -> bool {
    dec_composite
        .iter()
        .find(|t| t.name == "mantissa")
        .map(|t| t.encoding.presence == crate::ir::Presence::Optional)
        .unwrap_or(false)
}

/// For every selector opted into [`crate::DomainImpl::Manual`] on one of the
/// three well-known built-in type paths (`bool` / `rust_decimal::Decimal` /
/// `chrono::DateTime<Utc>`), render the exact impl ergo-sbe would otherwise
/// have generated as pretty-printed Rust source — a copy-paste starting
/// point for the doc comment on the field's generated domain accessor (see
/// `converter_impls.rs`).
///
/// A `rust_type` outside those three never had generated-impl logic to offer
/// in the first place, so it produces no snippet — `with_domain_type` never
/// auto-generates an impl for it regardless of `DomainImpl`.
pub(crate) fn generate_manual_impl_snippets(
    elements: &SchemaElements,
    domain_types: &[(crate::ConversionSelector, String)],
    manual_impl_selectors: &[crate::ConversionSelector],
) -> Vec<(crate::ConversionSelector, String)> {
    let span = proc_macro2::Span::call_site();
    let mut out = Vec::new();

    for (sel, ty) in domain_types {
        if !manual_impl_selectors.contains(sel) {
            continue;
        }
        let ts = match ty.as_str() {
            "bool" => {
                let crate::ConversionSelector::NamedType(n) = sel else {
                    continue;
                };
                let bt_ident = syn::Ident::new(&to_pascal_case(n), span);
                bool_impl_tokens(&bt_ident)
            }
            "rust_decimal::Decimal" => {
                let crate::ConversionSelector::NamedType(comp_name) = sel else {
                    continue;
                };
                let Some(dec_composite) =
                    elements.composites.iter().find(|c| &c[0].name == comp_name)
                else {
                    continue;
                };
                let dec_ident = syn::Ident::new(&to_pascal_case(comp_name), span);
                decimal_impl_tokens(
                    &dec_ident,
                    composite_exponent_is_constant(dec_composite),
                    composite_mantissa_is_optional(dec_composite),
                )
            }
            "chrono::DateTime<chrono::Utc>" if matches!(sel, crate::ConversionSelector::SemanticType(st) if st == "UTCTimestamp") => {
                chrono_impl_tokens()
            }
            _ => continue,
        };
        out.push((sel.clone(), pretty_print(&ts)));
    }
    out
}

// ── Generated application-type mappings ─────────────────────────────────
//
// `with_enum_mapping` / `with_composite_mapping`: `TryFromSbe` / `TryToSbe`
// impls written as an exhaustive `match` / struct literal, so the compiler
// checks both types against each other. Both use `sbe_rt::MappingError` and
// neither touches the wire layout: they sit on top of the generated enum's
// variants and the composite's getters and `new`.

/// Domain types with a built-in impl (`generate_conversion_impl_blocks`); a
/// mapping onto one would emit a second, conflicting impl.
const BUILT_IN_DOMAIN_TYPES: &[&str] = &[
    "bool",
    "rust_decimal::Decimal",
    "chrono::DateTime<chrono::Utc>",
];

fn mapping_error(option: &str, value: &str, reason: String) -> crate::GenerateError {
    crate::GenerateError::InvalidConfiguration {
        option: option.to_string(),
        value: value.to_string(),
        reason,
    }
}

/// The application type of a mapping: a plain path without generic
/// arguments, so it can name variants (`app::Side::Bid`) and build struct
/// literals (`app::Leg { .. }`).
fn mapping_path(rust_type: &str) -> Result<syn::Path, String> {
    let path: syn::Path = syn::parse_str(rust_type)
        .map_err(|e| format!("`{rust_type}` is not a valid Rust type path: {e}"))?;
    if path
        .segments
        .iter()
        .any(|segment| !segment.arguments.is_none())
    {
        return Err(format!(
            "`{rust_type}` is not a valid Rust type path: generic arguments are not supported"
        ));
    }
    Ok(path)
}

fn mapping_ident(name: &str) -> Result<syn::Ident, String> {
    syn::parse_str::<syn::Ident>(name).map_err(|_| format!("`{name}` is not a Rust identifier"))
}

/// `(schema name, generated variant)` for every declared variant, in schema
/// order. `NullVal` is not among them: it is never mapped.
fn enum_variants(tokens: &[crate::ir::Token]) -> Vec<(String, String)> {
    tokens
        .iter()
        .filter(|t| t.signal == crate::ir::Signal::Encoding && t.encoding.constant_value.is_some())
        .map(|t| (t.name.clone(), to_pascal_case(&t.name)))
        .collect()
}

/// Application variant for each schema variant, renames applied. A rename
/// may name the variant by its schema or its generated name; the last one
/// given for it wins, whichever spelling it used.
fn enum_targets(
    tokens: &[crate::ir::Token],
    mapping: &crate::EnumMapping,
) -> Vec<(String, String, String)> {
    enum_variants(tokens)
        .into_iter()
        .map(|(schema, generated)| {
            let target = mapping
                .renames
                .iter()
                .rev()
                .find(|(from, _)| *from == schema || *from == generated)
                .map_or_else(|| generated.clone(), |(_, to)| to.clone());
            (schema, generated, target)
        })
        .collect()
}

/// Struct field for each non-constant member, renames applied (the last
/// one given for a member wins, by schema or snake-case name):
/// `(member, field)`.
fn composite_targets(
    members: &[crate::structured_ir::CompositeMember],
    mapping: &crate::CompositeMapping,
) -> Vec<(String, String)> {
    members
        .iter()
        .filter(|m| !is_constant_member(m))
        .map(|m| {
            let snake = to_snake_case(&m.name);
            let field = mapping
                .renames
                .iter()
                .rev()
                .find(|(from, _)| *from == m.name || *from == snake)
                .map_or(snake, |(_, to)| to.clone());
            (m.name.clone(), field)
        })
        .collect()
}

const fn is_constant_member(m: &crate::structured_ir::CompositeMember) -> bool {
    matches!(
        m.member_type,
        crate::structured_ir::MemberType::Primitive {
            presence: crate::ir::Presence::Constant,
            ..
        }
    )
}

/// Reject the first `(name, target)` pair whose target repeats an earlier
/// one's.
fn unique_targets<'a>(pairs: impl Iterator<Item = (&'a str, &'a str)>) -> Result<(), String> {
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for (name, target) in pairs {
        if let Some((first, _)) = seen.iter().find(|(_, t)| *t == target) {
            return Err(format!("`{first}` and `{name}` both map to `{target}`"));
        }
        seen.push((name, target));
    }
    Ok(())
}

/// The first `presence="optional"` field of enum `enum_name` in any message
/// or group, as `Message.group.field` in schema names.
fn optional_field_of_enum(elements: &SchemaElements, enum_name: &str) -> Option<String> {
    fn search(
        path: &str,
        fields: &[crate::structured_ir::MessageField],
        groups: &[crate::structured_ir::MessageGroup],
        enum_name: &str,
    ) -> Option<String> {
        fields
            .iter()
            .find(|f| {
                f.presence == crate::ir::Presence::Optional
                    && matches!(&f.field_type,
                        crate::structured_ir::FieldType::Enum { name, .. } if name == enum_name)
            })
            .map(|f| format!("{path}.{}", f.name))
            .or_else(|| {
                groups.iter().find_map(|g| {
                    search(
                        &format!("{path}.{}", g.name),
                        &g.fields,
                        &g.groups,
                        enum_name,
                    )
                })
            })
    }
    elements.messages.iter().find_map(|tokens| {
        let msg = crate::structured_ir::parse_message_structure(tokens, elements);
        search(&msg.name, &msg.fields, &msg.groups, enum_name)
    })
}

/// Check every configured mapping against the schemas it will be generated
/// for (the union, for multi-schema generation) before any code is emitted.
pub(crate) fn validate_type_mappings(
    config: &crate::GenerationConfig,
    schemas: &[&SchemaElements],
) -> Result<(), crate::GenerateError> {
    let domain_type_of = |name: &str| {
        let selector = crate::ConversionSelector::named_type(name);
        config
            .domain_types
            .iter()
            .find(|(s, _)| *s == selector)
            .map(|(_, ty)| ty.as_str())
    };
    let check_target = |kind: &str, name: &str, rust_type: &str| -> Result<(), String> {
        mapping_path(rust_type)?;
        if BUILT_IN_DOMAIN_TYPES.contains(&rust_type) {
            return Err(format!(
                "`{rust_type}` has a built-in conversion and is not an application {kind}; \
                 use with_domain_type for it"
            ));
        }
        match domain_type_of(name) {
            Some(ty) if ty == rust_type => Ok(()),
            Some(ty) => Err(format!(
                "`{name}` is mapped to `{rust_type}` but its domain type is `{ty}`; \
                 a later with_domain_type replaced the mapping's"
            )),
            None => Err(format!("`{name}` has no domain type for its mapping")),
        }
    };

    for mapping in &config.enum_mappings {
        let name = mapping.sbe_enum.as_str();
        let fail = |reason: String| mapping_error("enum_mapping", name, reason);
        let tokens = schemas
            .iter()
            .find_map(|e| e.enums.iter().find(|t| t[0].name == name))
            .ok_or_else(|| fail(format!("no enum named `{name}` in the schema")))?;
        check_target("enum", name, &mapping.rust_type).map_err(fail)?;
        let variants = enum_variants(tokens);
        for (from, to) in &mapping.renames {
            if !variants.iter().any(|(s, g)| s == from || g == from) {
                let known: Vec<&str> = variants.iter().map(|(s, _)| s.as_str()).collect();
                return Err(fail(format!(
                    "`{name}` has no variant `{from}` (variants: {})",
                    known.join(", ")
                )));
            }
            mapping_ident(to).map_err(fail)?;
        }
        let targets = enum_targets(tokens, mapping);
        unique_targets(targets.iter().map(|(s, _, t)| (s.as_str(), t.as_str()))).map_err(fail)?;
        if !super::conversion_helpers::enum_uses_null_as_option(
            name,
            &config.null_as_option,
            config.all_enums_as_option,
        ) {
            if let Some(field) = schemas
                .iter()
                .find_map(|elements| optional_field_of_enum(elements, name))
            {
                return Err(fail(format!(
                    "`{field}` is optional, so its `NullVal` means absent, which `{name}` \
                     has no application variant for; add \
                     with_null_as_option(ConversionSelector::named_type(\"{name}\")) to \
                     decode it as None"
                )));
            }
        }
    }

    for mapping in &config.composite_mappings {
        let name = mapping.composite.as_str();
        let fail = |reason: String| mapping_error("composite_mapping", name, reason);
        let tokens = schemas
            .iter()
            .find_map(|e| e.composites.iter().find(|t| t[0].name == name))
            .ok_or_else(|| fail(format!("no composite named `{name}` in the schema")))?;
        check_target("struct", name, &mapping.rust_type).map_err(fail)?;
        let members = crate::structured_ir::parse_composite_members(tokens);
        for (from, to) in &mapping.renames {
            let member = members
                .iter()
                .find(|m| m.name == *from || to_snake_case(&m.name) == *from)
                .ok_or_else(|| {
                    let known: Vec<&str> = members.iter().map(|m| m.name.as_str()).collect();
                    fail(format!(
                        "`{name}` has no member `{from}` (members: {})",
                        known.join(", ")
                    ))
                })?;
            if is_constant_member(member) {
                return Err(fail(format!(
                    "`{from}` is constant and has no field: it is not on the wire"
                )));
            }
            mapping_ident(to).map_err(fail)?;
        }
        let targets = composite_targets(&members, mapping);
        unique_targets(targets.iter().map(|(m, f)| (m.as_str(), f.as_str()))).map_err(fail)?;
    }
    Ok(())
}

/// `TryFromSbe` / `TryToSbe` between a generated enum and its mapped
/// application enum: an exhaustive `match` each way. `NullVal` — also what
/// the decoder makes of an undeclared discriminant — is `NullEnum`.
pub(crate) fn enum_mapping_impl_tokens(
    tokens: &[crate::ir::Token],
    mapping: &crate::EnumMapping,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let wire_name = to_pascal_case(&tokens[0].name);
    let wire = syn::Ident::new(&wire_name, span);
    let app = mapping_path(&mapping.rust_type).expect("validated by validate_type_mappings");
    let mut from_arms = proc_macro2::TokenStream::new();
    let mut to_arms = proc_macro2::TokenStream::new();
    for (_, generated, target) in enum_targets(tokens, mapping) {
        let wire_variant = syn::Ident::new(&generated, span);
        let app_variant = mapping_ident(&target).expect("validated by validate_type_mappings");
        from_arms.extend(quote::quote! { #wire::#wire_variant => Ok(#app::#app_variant), });
        to_arms.extend(quote::quote! { #app::#app_variant => #wire::#wire_variant, });
    }
    let wire_lit = syn::LitStr::new(&wire_name, span);
    quote::quote! {
        impl TryFromSbe<#wire> for #app {
            type Error = sbe_rt::MappingError;
            #[inline]
            fn try_from_sbe(wire: #wire) -> Result<Self, Self::Error> {
                match wire {
                    #from_arms
                    #wire::NullVal => Err(sbe_rt::MappingError::NullEnum { enum_name: #wire_lit }),
                }
            }
        }
        impl TryToSbe<#wire> for #app {
            type Error = sbe_rt::MappingError;
            #[inline]
            fn try_to_sbe(&self) -> Result<#wire, Self::Error> {
                Ok(match self {
                    #to_arms
                })
            }
        }
    }
}

/// `TryFromSbe` / `TryToSbe` between a generated composite and its mapped
/// application struct: a struct literal from the getters, and the
/// composite's `new` from the fields. Constant members have no field;
/// optional scalars are `Option`, and `Some(null value)` is refused rather
/// than written; members of a mapped enum or composite go through that
/// mapping.
pub(crate) fn composite_mapping_impl_tokens(
    tokens: &[crate::ir::Token],
    mapping: &crate::CompositeMapping,
    enum_mappings: &[crate::EnumMapping],
    composite_mappings: &[crate::CompositeMapping],
) -> proc_macro2::TokenStream {
    use crate::structured_ir::MemberType;
    let span = proc_macro2::Span::call_site();
    let wire_name = to_pascal_case(&tokens[0].name);
    let wire = syn::Ident::new(&wire_name, span);
    let wire_lit = syn::LitStr::new(&wire_name, span);
    let app = mapping_path(&mapping.rust_type).expect("validated by validate_type_mappings");
    let members = crate::structured_ir::parse_composite_members(tokens);
    let targets = composite_targets(&members, mapping);
    // Application path of a nested enum / composite that has its own mapping.
    let mapped = |name: &str, is_enum: bool| -> Option<syn::Path> {
        let rust_type = if is_enum {
            enum_mappings
                .iter()
                .find(|m| m.sbe_enum == name)
                .map(|m| m.rust_type.as_str())
        } else {
            composite_mappings
                .iter()
                .find(|m| m.composite == name)
                .map(|m| m.rust_type.as_str())
        }?;
        mapping_path(rust_type).ok()
    };

    let mut fields = proc_macro2::TokenStream::new();
    let mut args = proc_macro2::TokenStream::new();
    for member in members.iter().filter(|m| !is_constant_member(m)) {
        let Some((_, field)) = targets.iter().find(|(name, _)| *name == member.name) else {
            continue;
        };
        let field = mapping_ident(field).expect("validated by validate_type_mappings");
        let getter = syn::Ident::new(&to_snake_case(&member.name), span);
        let nested = match &member.member_type {
            MemberType::Enum { name, .. } => mapped(name, true).map(|path| (name, path)),
            MemberType::Composite { name, .. } => mapped(name, false).map(|path| (name, path)),
            MemberType::Primitive { .. } | MemberType::Set { .. } => None,
        };
        if let Some((name, path)) = nested {
            let nested_wire = syn::Ident::new(&to_pascal_case(name), span);
            fields.extend(quote::quote! {
                #field: <#path as TryFromSbe<#nested_wire>>::try_from_sbe(wire.#getter())?,
            });
            args.extend(quote::quote! {
                <#path as TryToSbe<#nested_wire>>::try_to_sbe(&self.#field)?,
            });
            continue;
        }
        fields.extend(quote::quote! { #field: wire.#getter(), });
        match &member.member_type {
            MemberType::Primitive {
                prim,
                length: None,
                presence: crate::ir::Presence::Optional,
                null_value,
                ..
            } => {
                let ty = syn::Ident::new(crate::structured_ir::rust_type(*prim), span);
                let is_null = super::runtime::composite_member_null_check(*prim, *null_value);
                let null: proc_macro2::TokenStream = if matches!(
                    prim,
                    crate::ir::PrimitiveType::Float | crate::ir::PrimitiveType::Double
                ) {
                    match null_value {
                        Some(bits) => syn::parse_str::<syn::Expr>(
                            &super::runtime::field_const_value_expr(*bits, *prim),
                        )
                        .map(|expr| quote::quote! { #expr })
                        .unwrap_or_else(|_| quote::quote! { #ty::NAN }),
                        None => quote::quote! { #ty::NAN },
                    }
                } else {
                    // The getter's own null test: `nullValue`, else 0.
                    let lit = syn::LitInt::new(&format!("{}_u64", null_value.unwrap_or(0)), span);
                    quote::quote! { #lit as #ty }
                };
                let member_lit = syn::LitStr::new(&member.name, span);
                args.extend(quote::quote! {
                    match self.#field {
                        None => #null,
                        Some(val) => {
                            if #is_null {
                                sbe_rt::cold();
                                return Err(sbe_rt::MappingError::SomeNullValue {
                                    composite: #wire_lit,
                                    member: #member_lit,
                                });
                            }
                            val
                        }
                    },
                });
            }
            _ => args.extend(quote::quote! { self.#field, }),
        }
    }
    quote::quote! {
        impl TryFromSbe<#wire> for #app {
            type Error = sbe_rt::MappingError;
            #[inline]
            fn try_from_sbe(wire: #wire) -> Result<Self, Self::Error> {
                Ok(#app { #fields })
            }
        }
        impl TryToSbe<#wire> for #app {
            type Error = sbe_rt::MappingError;
            #[inline]
            fn try_to_sbe(&self) -> Result<#wire, Self::Error> {
                Ok(#wire::new(#args))
            }
        }
    }
}

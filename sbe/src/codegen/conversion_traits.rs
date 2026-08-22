//! `TryFromSbe` / `TryToSbe` trait emission and built-in domain-type impls.

use super::runtime::to_pascal_case;
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
             note = \"if this field uses DomainImpl::Manual, the generated `try_*` \
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
             note = \"if this field uses DomainImpl::Manual, the generated `try_*` \
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
    let dec_new_call: proc_macro2::TokenStream = if exponent_is_constant {
        quote::quote! { #dec_ident::new(mantissa) }
    } else {
        quote::quote! { #dec_ident::new(mantissa, -(self.scale() as i8)) }
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
                let mantissa: i64 = self.mantissa()
                    .try_into()
                    .map_err(|_| "Decimal mantissa overflow i64")?;
                Ok(#dec_new_call)
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
/// point for the doc comment on the field's generated `try_*` accessor (see
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

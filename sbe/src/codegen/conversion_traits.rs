//! `TryFromSbe` / `TryToSbe` trait emission and built-in domain-type impls.

use super::runtime::to_pascal_case;
use crate::structured_ir::SchemaElements;

/// Emit `TryFromSbe` / `TryToSbe` traits into the generated sbe_rt module.
pub(crate) fn emit_conversion_traits(src: &mut String) {
    src.push_str(
        "/// Convert from a wire type to an application type.\n\
         pub trait TryFromSbe<Wire>: Sized {\n\
             type Error: core::fmt::Debug + core::fmt::Display;\n\
             fn try_from_sbe(wire: Wire) -> Result<Self, Self::Error>;\n\
         }\n\n\
         /// Convert from an application type to a wire type.\n\
         pub trait TryToSbe<Wire> {\n\
             type Error: core::fmt::Debug + core::fmt::Display;\n\
             fn try_to_sbe(&self) -> Result<Wire, Self::Error>;\n\
         }\n\n",
    );
}

/// Emit `TryFromSbe` / `TryToSbe` impls for well-known **domain-type** mappings
/// (bool ↔ BooleanType, rust_decimal ↔ Decimal, chrono ↔ u64/UTCTimestamp).
///
/// These built-in impls are gated on `domain_types`, not bare `conversions`:
/// `with_conversion` alone keeps the seam dependency-free so callers can plug
/// any adapter (see samples/sbe-feature-tour `FixedPrice` / app-side
/// rust_decimal). `with_domain_type` opts into a concrete app type *and* these
/// well-known impls.
pub(crate) fn generate_conversion_impl_blocks(
    elements: &SchemaElements,
    _conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
) -> String {
    let mut out = String::new();
    let span = proc_macro2::Span::call_site();

    // Built-ins require an explicit domain-type mapping (not conversion-only).
    // Emit TryFromSbe / TryToSbe for EVERY boolean enum mapped to "bool",
    // not just the one literally named BooleanType.
    for (sel, ty) in domain_types {
        if ty != "bool" {
            continue;
        }
        let bt_name = match sel {
            crate::ConversionSelector::NamedType(n) => to_pascal_case(n),
            _ => continue,
        };
        let bt_ident = syn::Ident::new(&bt_name, span);
        let ts = quote::quote! {
            impl TryFromSbe<#bt_ident> for bool {
                type Error = &'static str;
                #[inline]
                fn try_from_sbe(wire: #bt_ident) -> Result<Self, Self::Error> {
                    Ok(bool::from(wire))
                }
            }
            impl TryToSbe<#bt_ident> for bool {
                type Error = &'static str;
                #[inline]
                fn try_to_sbe(&self) -> Result<#bt_ident, Self::Error> {
                    Ok(#bt_ident::from(*self))
                }
            }
        };
        out.push_str(&ts.to_string());
    }

    let has_decimal_conv = domain_types
        .iter()
        .any(|(sel, _)| matches!(sel, crate::ConversionSelector::NamedType(n) if n == "Decimal"));
    let has_chrono_conv = domain_types.iter().any(|(sel, _)| {
        matches!(sel, crate::ConversionSelector::SemanticType(st) if st == "UTCTimestamp")
    });

    if has_decimal_conv {
        let dec_composite = elements.composites.iter().find(|c| c[0].name == "Decimal");
        let dec_name = dec_composite
            .map(|c| to_pascal_case(&c[0].name))
            .unwrap_or_else(|| "Decimal".to_string());
        let dec_ident = syn::Ident::new(&dec_name, span);
        // Check if the schema's Decimal composite has a constant exponent.
        let exponent_is_constant = dec_composite
            .and_then(|c| c.iter().find(|t| t.name == "exponent"))
            .map(|t| t.encoding.presence == crate::ir::Presence::Constant)
            .unwrap_or(false);
        let dec_new_call: proc_macro2::TokenStream = if exponent_is_constant {
            quote::quote! { #dec_ident::new(mantissa) }
        } else {
            quote::quote! { #dec_ident::new(mantissa, -(self.scale() as i8)) }
        };
        let ts = quote::quote! {
            impl TryFromSbe<#dec_ident> for rust_decimal::Decimal {
                type Error = &'static str;
                #[inline]
                fn try_from_sbe(wire: #dec_ident) -> Result<Self, Self::Error> {
                    let mantissa = wire.mantissa() as i128;
                    let exponent = wire.exponent();
                    // SBE Decimal: negative exponent = fractional places (e.g.
                    // -2 → scale 2). Positive exponent = magnitude (mantissa ×
                    // 10^exp). rust_decimal scale must be a positive u32 ≤ 28.
                    let (mantissa, scale) = if exponent < 0 {
                        (mantissa, (-exponent) as u32)
                    } else {
                        (mantissa.saturating_mul(10i128.saturating_pow(exponent as u32)), 0)
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
        };
        out.push_str(&ts.to_string());
    }

    if has_chrono_conv {
        let ts = quote::quote! {
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
                    Ok(total_nanos as u64)
                }
            }
        };
        out.push_str(&ts.to_string());
    }

    out
}

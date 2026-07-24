//! Encoded-length classification and code generation.
//!
//! Three strategies:
//! - `Fixed`: no groups, no varData → use existing encoder constants.
//! - `Direct`: flat groups + message varData → checked const-fn helpers.
//! - `Staged`: nested groups or entry varData → staged builder types.

use crate::structured_ir::{MessageStructure, MessageGroup, MessageVarData, SchemaElements, get_dimension_info, get_dim_num_layout, get_vardata_info, rust_type};
use proc_macro2::TokenStream;
use quote::format_ident;

/// How the encoded length of a message should be computed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LengthStrategy {
    Fixed,
    Direct,
    Staged,
}

/// Output of encoded-length code generation for one message.
pub(super) struct GeneratedEncodedLength {
    /// Methods on the initial encoder (`impl {Msg}Encoder`).
    pub(super) encoder_impl: TokenStream,
    /// Standalone types appended after encoder stage generation.
    pub(super) standalone: TokenStream,
}

/// Classify a message into one of the three length strategies.
pub(super) fn strategy(message: &MessageStructure) -> LengthStrategy {
    if message.groups.is_empty() && message.var_data.is_empty() {
        return LengthStrategy::Fixed;
    }
    let has_dynamic_entry = message
        .groups
        .iter()
        .any(|group| !group.groups.is_empty() || !group.var_data.is_empty());
    if has_dynamic_entry { LengthStrategy::Staged } else { LengthStrategy::Direct }
}

/// Generate encoded-length support for one message.
pub(super) fn generate(
    message: &MessageStructure,
    block_length: usize,
    header_size: usize,
    elements: &SchemaElements,
) -> GeneratedEncodedLength {
    let s = strategy(message);
    match s {
        LengthStrategy::Fixed => GeneratedEncodedLength {
            encoder_impl: TokenStream::new(),
            standalone: TokenStream::new(),
        },
        LengthStrategy::Direct => generate_direct(message, block_length, header_size, elements),
        LengthStrategy::Staged => {
            // ponytail: return old staged builder for now until Tasks 4-7
            // implement uniform/ragged/unknown_size. The standalone field
            // is filled by the caller in mod.rs via the legacy path.
            GeneratedEncodedLength {
                encoder_impl: TokenStream::new(),
                standalone: TokenStream::new(),
            }
        }
    }
}

/// Generate direct `compute_encoded_length` + checked `try_compute_encoded_length` helpers.
fn generate_direct(
    msg: &MessageStructure,
    block_length: usize,
    header_size: usize,
    elements: &SchemaElements,
) -> GeneratedEncodedLength {
    let span = proc_macro2::Span::call_site();
    let block_len_lit = syn::LitInt::new(&block_length.to_string(), span);
    let header_size_lit = syn::LitInt::new(&header_size.to_string(), span);

    // Compatibility methods (existing pattern)
    let mut compat_param_decls = Vec::new();
    let mut compat_param_names = Vec::new();
    let mut compat_body = Vec::new();

    for g in &msg.groups {
        let g_snake = crate::codegen::to_snake_case(&g.name);
        let param_ident = syn::Ident::new(&format!("{g_snake}_count"), span);
        let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
        let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), span);
        let g_bl = syn::LitInt::new(&g.block_length.to_string(), span);
        compat_body.push(quote::quote! {
            len += #dim_size_lit + #param_ident * #g_bl;
        });
        compat_param_decls.push(quote::quote! { #param_ident: usize });
        compat_param_names.push(param_ident);
    }
    for vd in &msg.var_data {
        let vd_snake = crate::codegen::to_snake_case(&vd.name);
        let param_ident = syn::Ident::new(&format!("{vd_snake}_len"), span);
        let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
        let ps = syn::LitInt::new(&prefix_size.to_string(), span);
        compat_body.push(quote::quote! { len += #ps + #param_ident; });
        compat_param_decls.push(quote::quote! { #param_ident: usize });
        compat_param_names.push(param_ident);
    }

    let compat = quote::quote! {
        /// Compute the exact SBE message body length before encoding.
        /// Parameters: one `usize` per group (entry count) and one `usize`
        /// per var-data field (byte length).
        #[inline]
        pub const fn compute_encoded_length(#(#compat_param_decls),*) -> usize {
            let mut len = #block_len_lit;
            #(#compat_body)*
            len
        }

        /// Compute the exact SBE message length including the standard
        /// message header.
        #[inline]
        pub const fn compute_encoded_length_with_message_header(
            #(#compat_param_decls),*
        ) -> usize {
            #header_size + Self::compute_encoded_length(#(#compat_param_names),*)
        }
    };

    // Checked methods with typed group counts
    let mut checked_param_decls = Vec::new();
    let mut checked_param_names = Vec::new();
    let mut checked_body = Vec::new();

    for g in &msg.groups {
        let g_snake = crate::codegen::to_snake_case(&g.name);
        let param_ident = syn::Ident::new(&format!("{g_snake}_count"), span);
        let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
        let (_, _, num_prim) = get_dim_num_layout(elements, &g.dimension_type);
        let count_ty: syn::Type = syn::parse_str(rust_type(num_prim)).unwrap();
        let ds = syn::LitInt::new(&dim_size.to_string(), span);
        let g_bl = syn::LitInt::new(&g.block_length.to_string(), span);

        checked_param_decls.push(quote::quote! { #param_ident: #count_ty });
        checked_param_names.push(param_ident.clone());

        checked_body.push(quote::quote! {
            let entries_len = match (#g_bl as usize).checked_mul(#param_ident as usize) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
            len = match len.checked_add(#ds as usize) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
            len = match len.checked_add(entries_len) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
        });
    }

    for vd in &msg.var_data {
        let vd_snake = crate::codegen::to_snake_case(&vd.name);
        let param_ident = syn::Ident::new(&format!("{vd_snake}_len"), span);
        let vd_name = &vd.name;
        let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
        let ps = syn::LitInt::new(&prefix_size.to_string(), span);

        let mut max_check = TokenStream::new();
        if let Some(max) = vd.max_length {
            let max_lit = syn::LitInt::new(&max.to_string(), span);
            let pi = param_ident.clone();
            max_check.extend(quote::quote! {
                if #pi > #max_lit {
                    return Err(sbe_rt::EncodeError::VarDataTooLong {
                        field: #vd_name,
                        max_length: #max_lit,
                        actual: #pi,
                    });
                }
            });
        }

        let pi_decl = param_ident.clone();
        checked_param_decls.push(quote::quote! { #pi_decl: usize });
        let pi_name = param_ident.clone();
        checked_param_names.push(pi_name);
        let pi_body = param_ident.clone();

        checked_body.push(quote::quote! {
            #max_check
            len = match len.checked_add(#ps as usize) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
            len = match len.checked_add(#pi_body) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
        });
    }

    let checked = quote::quote! {
        /// Compute the exact SBE message body length with checked arithmetic.
        /// Group counts use the wire type (`u16` or `u8`); var-data lengths
        /// use `usize`.
        #[inline]
        pub const fn try_compute_encoded_length(
            #(#checked_param_decls),*
        ) -> Result<usize, sbe_rt::EncodeError> {
            let mut len: usize = #block_len_lit;
            #(#checked_body)*
            Ok(len)
        }

        /// Compute the exact SBE message length including the header, with
        /// checked arithmetic.
        #[inline]
        pub const fn try_compute_encoded_length_with_header(
            #(#checked_param_decls),*
        ) -> Result<usize, sbe_rt::EncodeError> {
            let body = Self::try_compute_encoded_length(#(#checked_param_names),*)?;
            match body.checked_add(#header_size) {
                Some(v) => Ok(v),
                None => Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            }
        }
    };

    let mut encoder_impl = TokenStream::new();
    encoder_impl.extend(compat);
    encoder_impl.extend(checked);

    GeneratedEncodedLength {
        encoder_impl,
        standalone: TokenStream::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::{strategy, LengthStrategy};
    use crate::structured_ir::{parse_message_structure, partition_tokens};
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("schemas")
            .join(name)
    }

    fn strategy_for(
        path: &std::path::Path,
        message_name: &str,
    ) -> Result<LengthStrategy, Box<dyn std::error::Error>> {
        let ir = crate::parse_file(path)?;
        let elements = partition_tokens(&ir.tokens);
        let message_tokens = elements
            .messages
            .iter()
            .find(|tokens| tokens[0].name == message_name)
            .ok_or_else(|| format!("missing message {message_name}"))?;
        let message = parse_message_structure(message_tokens, &elements);
        Ok(strategy(&message))
    }

    #[test]
    fn classifies_repository_message_shapes() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            strategy_for(&fixture("basic-schema.xml"), "TestMessage50001")?,
            LengthStrategy::Fixed,
        );
        assert_eq!(
            strategy_for(&fixture("basic-variable-length-schema.xml"), "TestMessage1")?,
            LengthStrategy::Direct,
        );
        assert_eq!(
            strategy_for(&fixture("basic-group-schema.xml"), "TestMessage1")?,
            LengthStrategy::Direct,
        );
        assert_eq!(
            strategy_for(&fixture("group-with-data-schema.xml"), "TestMessage1")?,
            LengthStrategy::Staged,
        );
        assert_eq!(
            strategy_for(&fixture("nested-group-schema.xml"), "Top")?,
            LengthStrategy::Staged,
        );
        assert_eq!(
            strategy_for(&fixture("l3-orderbook-schema.xml"), "L3Book")?,
            LengthStrategy::Staged,
        );
        Ok(())
    }
}

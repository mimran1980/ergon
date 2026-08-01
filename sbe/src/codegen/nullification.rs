//! Null-sentinel write generation for optional fields.

use crate::ir::{ByteOrder, Presence};
use crate::structured_ir::MessageField;
use quote::quote;

/// Generate null-sentinel write statements for optional fields.
pub(crate) fn generate_nullification(
    src: &mut String,
    fields: &[MessageField],
    offset_base: &str,
    buf_expr: &str,
    byte_order: ByteOrder,
) {
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };
    let mut stmts = proc_macro2::TokenStream::new();
    for f in fields {
        if f.presence == Presence::Optional {
            if let Some(null_val) = f.null_value {
                let size = f.field_type.size();
                // The null value is stored as a u64 in the IR (matching the
                // XML unsigned-integer attribute). Always render as _u64 and
                // slice to the field's wire size — smaller fields take the
                // low-order bytes, which is correct for little-endian.
                let null_val_expr: syn::Expr = syn::parse_str(&format!("{null_val}_u64")).unwrap();
                let to_method = syn::Ident::new(
                    &format!("to_{order_suffix}_bytes"),
                    proc_macro2::Span::call_site(),
                );
                let offset_base_expr: syn::Expr = syn::parse_str(offset_base).unwrap();
                let buf_expr_ts: syn::Expr = syn::parse_str(buf_expr).unwrap();
                let f_offset = syn::Index::from(f.offset);
                let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());

                stmts.extend(quote! {
                    let null_bytes = #null_val_expr.#to_method();
                    let offset = #offset_base_expr + #f_offset;
                    #buf_expr_ts[offset..offset + #size_lit]
                        .copy_from_slice(&null_bytes[..#size_lit]);
                });
            }
        }
    }
    if !stmts.is_empty() {
        src.push_str(&stmts.to_string());
        src.push('\n');
    }
}

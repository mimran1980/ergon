//! Null-sentinel write generation for optional fields.
//!
//! Wire bytes must be exactly the declared primitive width in the schema
//! byte order (HFT-002). Never copy a full `u64` LE/BE array into a shorter
//! field (panics on group optionals) or take the first `N` LE bytes of a BE
//! encoding (wrong high-order bytes).

use crate::ir::{ByteOrder, Presence};
use crate::structured_ir::{FieldType, MessageField};
use quote::quote;

/// Encode a schema null/constant integer as exactly `size` wire bytes.
///
/// `null_val` is the IR `u64` convenience representation (XML integer attribute
/// or float/double bit pattern). Signed and floating patterns use the same
/// bit-width truncation as the official SBE tools: the low `size` bytes of the
/// value in the schema endianness.
pub(crate) fn null_sentinel_bytes(null_val: u64, size: usize, byte_order: ByteOrder) -> [u8; 8] {
    debug_assert!((1..=8).contains(&size));
    let mut out = [0u8; 8];
    match byte_order {
        ByteOrder::LittleEndian => {
            let full = null_val.to_le_bytes();
            out[..size].copy_from_slice(&full[..size]);
        }
        ByteOrder::BigEndian => {
            // Take the least-significant `size` bytes, then place them at the
            // high end of a big-endian field (standard SBE integer null layout).
            let full = null_val.to_be_bytes();
            out[..size].copy_from_slice(&full[8 - size..]);
        }
    }
    out
}

/// Token-stream form of [`null_sentinel_bytes`] for generated apply_nulls paths.
fn null_bytes_expr(null_val: u64, size: usize, byte_order: ByteOrder) -> proc_macro2::TokenStream {
    let bytes = null_sentinel_bytes(null_val, size, byte_order);
    let lits: Vec<syn::LitInt> = bytes[..size]
        .iter()
        .map(|b| syn::LitInt::new(&b.to_string(), proc_macro2::Span::call_site()))
        .collect();
    let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
    quote! { [#(#lits),*] as [u8; #size_lit] }
}

fn optional_null_size(field: &MessageField) -> Option<usize> {
    match &field.field_type {
        FieldType::Primitive(prim, length) => {
            let n = length.unwrap_or(1);
            if n != 1 {
                // Multi-byte char arrays / fixed arrays use per-element nulls
                // elsewhere; single primitive optionals only here.
                return None;
            }
            Some(prim.size())
        }
        FieldType::Enum { encoding_type, .. } | FieldType::Set { encoding_type, .. } => {
            Some(encoding_type.size())
        }
        FieldType::Composite { .. } => None,
    }
}

/// Generate null-sentinel write statements for optional fields.
pub(crate) fn generate_nullification(
    src: &mut String,
    fields: &[MessageField],
    offset_base: &str,
    buf_expr: &str,
    byte_order: ByteOrder,
) {
    let mut stmts = proc_macro2::TokenStream::new();
    for f in fields {
        if f.presence != Presence::Optional {
            continue;
        }
        let Some(null_val) = f.null_value else {
            continue;
        };
        let Some(size) = optional_null_size(f) else {
            continue;
        };
        if size == 0 || size > 8 {
            continue;
        }
        let offset_base_expr: syn::Expr = syn::parse_str(offset_base).unwrap();
        let buf_expr_ts: syn::Expr = syn::parse_str(buf_expr).unwrap();
        let f_offset = syn::Index::from(f.offset);
        let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
        let null_arr = null_bytes_expr(null_val, size, byte_order);

        stmts.extend(quote! {
            {
                let null_bytes: [u8; #size_lit] = #null_arr;
                let offset = #offset_base_expr + #f_offset;
                #buf_expr_ts[offset..offset + #size_lit].copy_from_slice(&null_bytes);
            }
        });
    }
    if !stmts.is_empty() {
        src.push_str(&stmts.to_string());
        src.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ByteOrder;

    #[test]
    fn le_null_sentinel_takes_low_order_bytes() {
        // 0xFFFF_FFFF as u32 null → ff ff ff ff
        let b = null_sentinel_bytes(0xffff_ffff, 4, ByteOrder::LittleEndian);
        assert_eq!(&b[..4], &[0xff, 0xff, 0xff, 0xff]);
        let b1 = null_sentinel_bytes(0xff, 1, ByteOrder::LittleEndian);
        assert_eq!(&b1[..1], &[0xff]);
        let b2 = null_sentinel_bytes(0xffff, 2, ByteOrder::LittleEndian);
        assert_eq!(&b2[..2], &[0xff, 0xff]);
    }

    #[test]
    fn be_null_sentinel_places_lsb_at_field_end() {
        // Same integer nulls on big-endian: significant bytes at the high end.
        let b = null_sentinel_bytes(0xffff_ffff, 4, ByteOrder::BigEndian);
        assert_eq!(&b[..4], &[0xff, 0xff, 0xff, 0xff]);
        let b1 = null_sentinel_bytes(0xff, 1, ByteOrder::BigEndian);
        assert_eq!(&b1[..1], &[0xff]);
        // Value 0x00FF as u16 BE is 00 ff, not ff 00.
        let b2 = null_sentinel_bytes(0x00ff, 2, ByteOrder::BigEndian);
        assert_eq!(&b2[..2], &[0x00, 0xff]);
        // null 0xFF as u16 BE is 00 ff (low 16 bits), not the high two BE bytes of u64.
        let b2b = null_sentinel_bytes(0xff, 2, ByteOrder::BigEndian);
        assert_eq!(&b2b[..2], &[0x00, 0xff]);
    }

    #[test]
    fn float_null_bit_pattern_width() {
        // IEEE quiet NaN bit pattern commonly used as float null (example).
        let bits = f32::NAN.to_bits() as u64;
        let b = null_sentinel_bytes(bits, 4, ByteOrder::LittleEndian);
        assert_eq!(&b[..4], &bits.to_le_bytes()[..4]);
    }
}

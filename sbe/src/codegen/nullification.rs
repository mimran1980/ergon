//! Null-sentinel write generation for optional fields.
//!
//! Wire bytes must be exactly the declared primitive width in the schema
//! byte order. Never copy a full `u64` LE/BE array into a shorter
//! field (panics on group optionals) or take the first `N` LE bytes of a BE
//! encoding (wrong high-order bytes).

use crate::ir::{ByteOrder, Presence, PrimitiveType, Signal};
use crate::structured_ir::{
    FieldType, MemberType, MessageField, SchemaElements, parse_composite_members,
};
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

/// Emit a single copy of `size` null bytes at `offset_expr` into `buf_expr`.
fn write_null_at(
    buf_expr: &syn::Expr,
    offset_expr: proc_macro2::TokenStream,
    null_val: u64,
    size: usize,
    byte_order: ByteOrder,
) -> proc_macro2::TokenStream {
    let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
    let null_arr = null_bytes_expr(null_val, size, byte_order);
    quote! {
        {
            let null_bytes: [u8; #size_lit] = #null_arr;
            let offset = #offset_expr;
            #buf_expr[offset..offset + #size_lit].copy_from_slice(&null_bytes);
        }
    }
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

/// Null-image statements for one optional message field at `field_abs_offset`.
///
/// Used by both `apply_nulls` and `fixed(&FixedFields)` when a field is `None`.
/// Returns `None` only for constant / non-optional fields (caller should skip).
/// Returns `Err` when an optional field has no derivable null image.
pub(crate) fn null_image_stmts_for_field(
    f: &MessageField,
    field_abs_offset: proc_macro2::TokenStream,
    buf_expr: &syn::Expr,
    byte_order: ByteOrder,
    elements: &SchemaElements,
) -> Result<Option<proc_macro2::TokenStream>, String> {
    if f.presence != Presence::Optional {
        return Ok(None);
    }
    match &f.field_type {
        FieldType::Primitive(prim, length) => {
            let n = length.unwrap_or(1);
            let elem_size = prim.size();
            if elem_size == 0 || elem_size > 8 {
                return Err(format!(
                    "optional field '{}': unsupported primitive width {elem_size}",
                    f.name
                ));
            }
            // Prefer schema nullValue; default element null is 0 for arrays
            // (ASCII/char padding) and the IR null for scalar optionals.
            let null_val = f.null_value.unwrap_or(0);
            if n == 1 {
                return Ok(Some(write_null_at(
                    buf_expr,
                    field_abs_offset,
                    null_val,
                    elem_size,
                    byte_order,
                )));
            }
            // Fixed array: write the element null image into every slot.
            let mut stmts = proc_macro2::TokenStream::new();
            for i in 0..n {
                let off = i * elem_size;
                let off_lit = syn::LitInt::new(&off.to_string(), proc_macro2::Span::call_site());
                stmts.extend(write_null_at(
                    buf_expr,
                    quote! { #field_abs_offset + #off_lit },
                    null_val,
                    elem_size,
                    byte_order,
                ));
            }
            Ok(Some(stmts))
        }
        FieldType::Enum { encoding_type, .. } | FieldType::Set { encoding_type, .. } => {
            let size = encoding_type.size();
            // Prefer schema nullValue; enums default to encoding-type max (from resolve).
            let null_val = f.null_value.unwrap_or(match encoding_type {
                PrimitiveType::UInt8 => 255,
                PrimitiveType::UInt16 => 65535,
                PrimitiveType::UInt32 => 4294967295,
                PrimitiveType::UInt64 => u64::MAX,
                PrimitiveType::Int8 => -128i8 as u64,
                PrimitiveType::Int16 => -32768i16 as u64,
                PrimitiveType::Int32 => i32::MIN as u64,
                PrimitiveType::Int64 => i64::MIN as u64,
                _ => 0,
            });
            Ok(Some(write_null_at(
                buf_expr,
                field_abs_offset,
                null_val,
                size,
                byte_order,
            )))
        }
        FieldType::Composite { name, size } => {
            // Optional composite: zero the whole span, then write nested
            // optional member null sentinels recursively. Nested composite
            // member offsets are relative to that nested start and must be
            // added to the parent field base (not treated as absolute).
            let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());
            let mut stmts = quote! {
                {
                    let offset = #field_abs_offset;
                    #buf_expr[offset..offset + #size_lit].fill(0);
                }
            };
            stmts.extend(composite_optional_null_stmts(
                name,
                field_abs_offset,
                buf_expr,
                byte_order,
                elements,
            )?);
            Ok(Some(stmts))
        }
    }
}

/// Recursively emit null images for optional members of a composite type.
///
/// `base_offset` is an absolute buffer expression for the start of this
/// composite instance.
fn composite_optional_null_stmts(
    type_name: &str,
    base_offset: proc_macro2::TokenStream,
    buf_expr: &syn::Expr,
    byte_order: ByteOrder,
    elements: &SchemaElements,
) -> Result<proc_macro2::TokenStream, String> {
    let Some(comp_tokens) = elements.composites.iter().find(|c| c[0].name == type_name) else {
        return Err(format!(
            "optional composite type '{type_name}' not found in schema"
        ));
    };
    // Prefer structured members (correct nested offsets). Also consult the
    // raw token stream for optional + nullValue on primitive members, since
    // MemberType::Primitive may not always surface nullValue.
    let members = parse_composite_members(comp_tokens);
    let mut stmts = proc_macro2::TokenStream::new();
    for m in &members {
        let mem_off_lit = syn::LitInt::new(&m.offset.to_string(), proc_macro2::Span::call_site());
        let abs = quote! { #base_offset + #mem_off_lit };
        match &m.member_type {
            MemberType::Primitive {
                prim,
                length,
                presence,
                ..
            } => {
                if *presence != Presence::Optional {
                    continue;
                }
                // Look up nullValue from the matching BeginField token when present.
                let null_val = comp_tokens
                    .iter()
                    .find(|t| t.signal == Signal::BeginField && t.name == m.name)
                    .and_then(|t| t.encoding.null_value)
                    .unwrap_or(0);
                let n = length.unwrap_or(1);
                let elem_size = prim.size();
                if !(1..=8).contains(&elem_size) {
                    return Err(format!(
                        "optional composite member '{}': unsupported width {elem_size}",
                        m.name
                    ));
                }
                if n == 1 {
                    stmts.extend(write_null_at(
                        buf_expr, abs, null_val, elem_size, byte_order,
                    ));
                } else {
                    for i in 0..n {
                        let slot = i * elem_size;
                        let slot_lit =
                            syn::LitInt::new(&slot.to_string(), proc_macro2::Span::call_site());
                        stmts.extend(write_null_at(
                            buf_expr,
                            quote! { #abs + #slot_lit },
                            null_val,
                            elem_size,
                            byte_order,
                        ));
                    }
                }
            }
            MemberType::Composite { name, .. } => {
                stmts.extend(composite_optional_null_stmts(
                    name, abs, buf_expr, byte_order, elements,
                )?);
            }
            MemberType::Enum { .. } | MemberType::Set { .. } => {
                // Enum/set refs inside composites: parent span is zeroed; when
                // the BeginField is optional with an explicit nullValue, stamp it.
                if let Some(tok) = comp_tokens
                    .iter()
                    .find(|t| t.signal == Signal::BeginField && t.name == m.name)
                    && tok.encoding.presence == Presence::Optional
                    && let Some(null_val) = tok.encoding.null_value
                {
                    let prim = tok.encoding.primitive_type.unwrap_or(PrimitiveType::UInt8);
                    let mem_size = prim.size();
                    if (1..=8).contains(&mem_size) {
                        stmts.extend(write_null_at(buf_expr, abs, null_val, mem_size, byte_order));
                    }
                }
            }
        }
    }
    Ok(stmts)
}

/// Generate null-sentinel write statements for optional fields.
pub(crate) fn generate_nullification(
    src: &mut String,
    fields: &[MessageField],
    offset_base: &str,
    buf_expr: &str,
    byte_order: ByteOrder,
    elements: &SchemaElements,
) {
    let mut stmts = proc_macro2::TokenStream::new();
    let offset_base_expr: syn::Expr = syn::parse_str(offset_base).unwrap();
    let buf_expr_ts: syn::Expr = syn::parse_str(buf_expr).unwrap();
    for f in fields {
        if f.presence != Presence::Optional {
            continue;
        }
        let f_offset = syn::Index::from(f.offset);
        let abs = quote! { #offset_base_expr + #f_offset };
        match null_image_stmts_for_field(f, abs, &buf_expr_ts, byte_order, elements) {
            Ok(Some(s)) => stmts.extend(s),
            Ok(None) => {}
            // Keep apply_nulls best-effort for legacy callers; fixed() rejects.
            Err(_) => {
                if let (Some(null_val), Some(size)) = (f.null_value, optional_null_size(f)) {
                    if size > 0 && size <= 8 {
                        stmts.extend(write_null_at(
                            &buf_expr_ts,
                            quote! { #offset_base_expr + #f_offset },
                            null_val,
                            size,
                            byte_order,
                        ));
                    }
                }
            }
        }
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

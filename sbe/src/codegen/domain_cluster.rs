//! Domain-object (DTO) code generation.
//!
//! Owned domain structs + `From<Decoder>` conversions for messages and group
//! entries, bulk-encode helpers, and schema min/max range checks. Depends on
//! [`super::conversion_helpers`], [`super::runtime`], and `structured_ir` types.

use super::conversion_helpers::{
    domain_encode_setter_name, field_has_conversion_free, find_domain_type, message_field_infos,
};
use super::runtime::{to_pascal_case, to_snake_case};
use crate::ir::{ByteOrder, Presence, PrimitiveType};
use crate::structured_ir::{
    FieldType, MessageField, MessageGroup, MessageStructure, MessageVarData, SchemaElements,
    get_dim_num_layout, get_dimension_info, get_vardata_info, rust_type,
};
/// Generate owned domain structs + From<Decoder> impls for a message and all
/// its group entries. Groups are `Vec<…EntryDomain>`; var-data follows
/// [`crate::config::DomainVarData`].
pub(crate) fn generate_domain_objects(
    msg: &MessageStructure,
    elements: &SchemaElements,
    msg_name: &str,
    _parent_scope: &str,
    multi_message: bool,
    byte_order: ByteOrder,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    domain_var_data: crate::config::DomainVarData,
    hooks: &crate::config::Hooks,
    schema: &crate::Schema,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let mut ts = proc_macro2::TokenStream::new();
    let _has_conversion = domain_has_conversion(&msg.fields, &msg.groups, &conversions);
    generate_domain_recursive(
        msg_name,
        msg_name,
        &msg.fields,
        &msg.groups,
        &msg.var_data,
        elements,
        byte_order,
        multi_message,
        msg_name,
        msg.block_length,
        conversions,
        domain_types,
        domain_var_data,
        false, // is_entry — this is a message, not a group entry
        hooks,
        schema,
        &mut ts,
        span,
    );
    ts
}

pub(crate) fn domain_entry_can_bulk_encode(
    fields: &[MessageField],
    groups: &[MessageGroup],
    var_data: &[MessageVarData],
    elements: &SchemaElements,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
) -> bool {
    groups.is_empty()
        && var_data.is_empty()
        && fields.iter().all(|f| {
            f.presence == Presence::Constant
                || (f.presence != Presence::Optional
                    && f.since_version == 0
                    && !field_has_conversion_free(f, conversions)
                    && find_domain_type(f, domain_types).is_none()
                    && !matches!(
                        &f.field_type,
                        FieldType::Enum { name, .. } if crate::structured_ir::is_bool_enum(elements, name)
                    ))
        })
}

pub(crate) fn domain_bulk_slot_write_tokens(
    fields: &[MessageField],
    byte_order: ByteOrder,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    let to_endian = match byte_order {
        ByteOrder::LittleEndian => syn::Ident::new("to_le_bytes", span),
        ByteOrder::BigEndian => syn::Ident::new("to_be_bytes", span),
    };
    let mut writes = proc_macro2::TokenStream::new();
    for f in fields {
        if f.presence == Presence::Constant {
            continue;
        }
        let f_name = syn::Ident::new(&to_snake_case(&f.name), span);
        let f_offset = syn::Index::from(f.offset);
        let f_size = syn::LitInt::new(&f.field_type.size().to_string(), span);
        match &f.field_type {
            FieldType::Composite { .. } => {
                writes.extend(quote::quote! {
                    slot[#f_offset..#f_offset + #f_size]
                        .copy_from_slice(&entry.#f_name.0);
                });
            }
            FieldType::Enum { encoding_type, .. } | FieldType::Set { encoding_type, .. } => {
                let r_ty = syn::Ident::new(rust_type(*encoding_type), span);
                writes.extend(quote::quote! {
                    slot[#f_offset..#f_offset + #f_size]
                        .copy_from_slice(&(#r_ty::from(entry.#f_name)).#to_endian());
                });
            }
            FieldType::Primitive(pt, Some(len)) => {
                let len_lit = syn::LitInt::new(&len.to_string(), span);
                let prim_size_lit = syn::LitInt::new(&pt.size().to_string(), span);
                writes.extend(quote::quote! {
                    let mut idx = 0usize;
                    while idx < #len_lit {
                        let offset = #f_offset + idx * #prim_size_lit;
                        slot[offset..offset + #prim_size_lit]
                            .copy_from_slice(&entry.#f_name[idx].#to_endian());
                        idx += 1;
                    }
                });
            }
            FieldType::Primitive(_, None) => {
                writes.extend(quote::quote! {
                    slot[#f_offset..#f_offset + #f_size]
                        .copy_from_slice(&entry.#f_name.#to_endian());
                });
            }
        }
    }
    writes
}

/// Check whether any field, group entry, or nested group under these
/// fields/groups uses a registered decimal composite.
pub(crate) fn domain_has_conversion(
    fields: &[MessageField],
    groups: &[MessageGroup],
    conversions: &[crate::ConversionSelector],
) -> bool {
    for f in fields {
        if let FieldType::Composite { name, .. } = &f.field_type {
            if conversions
                .iter()
                .any(|sel| matches!(sel, crate::ConversionSelector::NamedType(n) if n == name))
            {
                return true;
            }
        }
    }
    for g in groups {
        if domain_has_conversion(&g.fields, &g.groups, conversions) {
            return true;
        }
    }
    false
}

/// Emit a domain-object range check against schema min/max for integer wire types.
/// Floats/doubles are skipped (IEEE null sentinels are not simple min/max ranges).
pub(crate) fn dto_range_check_tokens(
    f: &MessageField,
    prim: PrimitiveType,
    value_expr: proc_macro2::TokenStream,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    if matches!(prim, PrimitiveType::Float | PrimitiveType::Double) {
        return quote::quote! {};
    }
    let (Some(min), Some(max)) = (f.min_value, f.max_value) else {
        return quote::quote! {};
    };
    let to_i128 = |v: u64| -> i128 {
        match prim {
            PrimitiveType::Int8 => (v as i8) as i128,
            PrimitiveType::Int16 => (v as i16) as i128,
            PrimitiveType::Int32 => (v as i32) as i128,
            PrimitiveType::Int64 => (v as i64) as i128,
            PrimitiveType::Char | PrimitiveType::UInt8 => v as u8 as i128,
            PrimitiveType::UInt16 => v as u16 as i128,
            PrimitiveType::UInt32 => v as u32 as i128,
            PrimitiveType::UInt64 => v as i128,
            PrimitiveType::Float | PrimitiveType::Double => 0,
        }
    };
    let min_i = to_i128(min);
    let max_i = to_i128(max);
    // Skip no-op ranges that cover the full native type width.
    if min_i == to_i128(0)
        && max_i >= (i128::from(u64::MAX) - 1)
        && matches!(prim, PrimitiveType::UInt64)
    {
        // still check — MAX is often max-1 for null reserved
    }
    let min_lit = syn::LitInt::new(&format!("{min_i}"), span);
    let max_lit = syn::LitInt::new(&format!("{max_i}"), span);
    let field_lit = syn::LitStr::new(&f.name, span);
    quote::quote! {
        {
            let __v = #value_expr as i128;
            if __v < #min_lit || __v > #max_lit {
                return Err(sbe_rt::EncodeError::ValueOutOfRange {
                    field: #field_lit,
                    min: #min_lit,
                    max: #max_lit,
                    actual: __v,
                });
            }
        }
    }
}

#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
pub(crate) fn generate_domain_recursive(
    struct_prefix: &str,
    decoder_name: &str,
    fields: &[MessageField],
    groups: &[MessageGroup],
    var_data: &[MessageVarData],
    elements: &SchemaElements,
    byte_order: ByteOrder,
    multi_message: bool,
    msg_name: &str,
    message_block_length: usize,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    domain_var_data: crate::config::DomainVarData,
    is_entry: bool,
    hooks: &crate::config::Hooks,
    schema: &crate::Schema,
    ts: &mut proc_macro2::TokenStream,
    span: proc_macro2::Span,
) {
    let domain_ident = syn::Ident::new(&format!("{struct_prefix}Domain"), span);
    let decoder_ident = syn::Ident::new(&format!("{decoder_name}Decoder"), span);

    let mut struct_fields: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut from_exprs: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut encode_stmts: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut group_encode_stmts: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut vardata_encode_stmts: Vec<proc_macro2::TokenStream> = Vec::new();

    for f in fields {
        if f.presence == Presence::Constant {
            continue;
        }
        let f_snake = to_snake_case(&f.name);
        let f_ident = syn::Ident::new(&f_snake, span);
        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_type_str = rust_type(*prim);
                let r_type: syn::Type = syn::parse_str(r_type_str).unwrap();
                // Domain type for primitives with a semantic/named conversion
                // (e.g. u64 UTCTimestamp → chrono::DateTime<Utc>). Only the
                // scalar required case is converted; arrays/optional keep the
                // wire type.
                let scalar_domain = if length.is_none() && f.presence != Presence::Optional {
                    find_domain_type(f, domain_types)
                } else {
                    None
                };
                let scalar_ty: syn::Type = match scalar_domain {
                    Some(dt) => syn::parse_str(dt).unwrap(),
                    None => r_type.clone(),
                };
                let enc_setter = syn::Ident::new(
                    &domain_encode_setter_name(f, conversions, domain_types, &f_snake),
                    span,
                );
                if let Some(len) = length {
                    let len_lit = syn::LitInt::new(&len.to_string(), span);
                    struct_fields.push(quote::quote! { pub #f_ident: [#r_type; #len_lit] });
                    from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                    encode_stmts.push(quote::quote! { enc.#enc_setter(self.#f_ident); });
                } else if f.presence == Presence::Optional {
                    struct_fields.push(quote::quote! { pub #f_ident: Option<#r_type> });
                    from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                    let range_check = dto_range_check_tokens(f, *prim, quote::quote! { v }, span);
                    encode_stmts.push(quote::quote! {
                        if let Some(v) = self.#f_ident {
                            #range_check
                            enc.#enc_setter(v);
                        }
                    });
                } else {
                    // Domain-typed scalars use fallible `try_*`.
                    // Conversion-only renames the raw flyweight getter to *_wire.
                    if f.since_version > 0 {
                        struct_fields.push(quote::quote! { pub #f_ident: Option<#scalar_ty> });
                        if scalar_domain.is_some() {
                            let try_g = syn::Ident::new(&format!("try_{f_snake}"), span);
                            let field_lit = syn::LitStr::new(&f_snake, span);
                            from_exprs.push(quote::quote! {
                                #f_ident: Some(dec.#try_g().map_err(|_| {
                                    sbe_rt::DecodeError::DomainConversionFailed {
                                        field: #field_lit,
                                        reason: "try_* conversion rejected wire value",
                                    }
                                })?)
                            });
                            encode_stmts.push(quote::quote! {
                                if let Some(v) = self.#f_ident {
                                    enc.#enc_setter(v).map_err(|_| {
                                        sbe_rt::EncodeError::DomainConversionFailed {
                                            field: #field_lit,
                                            reason: "try_* conversion rejected domain value",
                                        }
                                    })?;
                                }
                            });
                        } else {
                            let from_getter = if field_has_conversion_free(f, conversions) {
                                syn::Ident::new(&format!("{f_snake}_wire"), span)
                            } else {
                                f_ident.clone()
                            };
                            from_exprs.push(quote::quote! { #f_ident: dec.#from_getter() });
                            let range_check =
                                dto_range_check_tokens(f, *prim, quote::quote! { v }, span);
                            encode_stmts.push(quote::quote! {
                                if let Some(v) = self.#f_ident {
                                    #range_check
                                    enc.#enc_setter(v);
                                }
                            });
                        }
                    } else {
                        struct_fields.push(quote::quote! { pub #f_ident: #scalar_ty });
                        if scalar_domain.is_some() {
                            let try_g = syn::Ident::new(&format!("try_{f_snake}"), span);
                            let field_lit = syn::LitStr::new(&f_snake, span);
                            from_exprs.push(quote::quote! {
                                #f_ident: dec.#try_g().map_err(|_| {
                                    sbe_rt::DecodeError::DomainConversionFailed {
                                        field: #field_lit,
                                        reason: "try_* conversion rejected wire value",
                                    }
                                })?
                            });
                            encode_stmts.push(quote::quote! {
                                enc.#enc_setter(self.#f_ident).map_err(|_| {
                                    sbe_rt::EncodeError::DomainConversionFailed {
                                            field: #field_lit,
                                            reason: "try_* conversion rejected domain value",
                                        }
                                })?;
                            });
                        } else {
                            let from_getter = if field_has_conversion_free(f, conversions) {
                                syn::Ident::new(&format!("{f_snake}_wire"), span)
                            } else {
                                f_ident.clone()
                            };
                            from_exprs.push(quote::quote! { #f_ident: dec.#from_getter() });
                            let range_check = dto_range_check_tokens(
                                f,
                                *prim,
                                quote::quote! { self.#f_ident },
                                span,
                            );
                            encode_stmts.push(quote::quote! {
                                #range_check
                                enc.#enc_setter(self.#f_ident);
                            });
                        }
                    }
                }
            }
            FieldType::Composite {
                name: comp_name, ..
            } => {
                let comp_pascal = to_pascal_case(comp_name);
                let comp_ident = syn::Ident::new(&comp_pascal, span);
                let as_struct_ident = syn::Ident::new(&format!("{f_snake}_value"), span);
                // If a domain type is configured for this composite (e.g.
                // Decimal → rust_decimal::Decimal), the DTO field uses the
                // domain type and reads/writes via the domain accessors.
                let domain_ty = find_domain_type(f, domain_types);
                let enc_setter = syn::Ident::new(
                    &domain_encode_setter_name(f, conversions, domain_types, &f_snake),
                    span,
                );
                let field_ty: proc_macro2::TokenStream = match domain_ty {
                    Some(dt) => {
                        let parsed: syn::Type = syn::parse_str(dt).unwrap();
                        quote::quote! { #parsed }
                    }
                    None => quote::quote! { #comp_ident },
                };
                // Drive-by fix: versioned composites return Option<T> on decoders,
                // so the DTO field must also be optional.
                if f.since_version > 0 {
                    struct_fields.push(quote::quote! { pub #f_ident: Option<#field_ty> });
                    if domain_ty.is_some() {
                        let try_g = syn::Ident::new(&format!("try_{f_snake}"), span);
                        let field_lit = syn::LitStr::new(&f_snake, span);
                        from_exprs.push(quote::quote! {
                            #f_ident: Some(dec.#try_g().map_err(|_| {
                                sbe_rt::DecodeError::DomainConversionFailed {
                                        field: #field_lit,
                                        reason: "try_* conversion rejected wire value",
                                    }
                            })?)
                        });
                        encode_stmts.push(quote::quote! {
                            if let Some(v) = self.#f_ident {
                                enc.#enc_setter(v).map_err(|_| {
                                    sbe_rt::EncodeError::DomainConversionFailed {
                                            field: #field_lit,
                                            reason: "try_* conversion rejected domain value",
                                        }
                                })?;
                            }
                        });
                    } else {
                        from_exprs.push(quote::quote! { #f_ident: dec.#as_struct_ident() });
                        encode_stmts
                            .push(quote::quote! { if let Some(ref v) = self.#f_ident { enc.#enc_setter(*v); } });
                    }
                } else {
                    struct_fields.push(quote::quote! { pub #f_ident: #field_ty });
                    if domain_ty.is_some() {
                        let try_g = syn::Ident::new(&format!("try_{f_snake}"), span);
                        let field_lit = syn::LitStr::new(&f_snake, span);
                        from_exprs.push(quote::quote! {
                            #f_ident: dec.#try_g().map_err(|_| {
                                sbe_rt::DecodeError::DomainConversionFailed {
                                        field: #field_lit,
                                        reason: "try_* conversion rejected wire value",
                                    }
                            })?
                        });
                        encode_stmts.push(quote::quote! {
                            enc.#enc_setter(self.#f_ident).map_err(|_| {
                                sbe_rt::EncodeError::DomainConversionFailed {
                                            field: #field_lit,
                                            reason: "try_* conversion rejected domain value",
                                        }
                            })?;
                        });
                    } else {
                        from_exprs.push(quote::quote! { #f_ident: dec.#as_struct_ident() });
                        encode_stmts.push(quote::quote! { enc.#enc_setter(self.#f_ident); });
                    }
                }
            }
            FieldType::Enum {
                name: enum_name, ..
            } => {
                if crate::structured_ir::is_bool_enum(elements, enum_name) {
                    // bool enums → plain bool in DTO; NullVal rejected at decode time
                    let try_bool_ident = syn::Ident::new(&format!("try_{f_snake}_bool"), span);
                    let opt_bool_ident = syn::Ident::new(&format!("{f_snake}_bool"), span);
                    if f.since_version > 0 {
                        struct_fields.push(quote::quote! { pub #f_ident: Option<bool> });
                        from_exprs.push(quote::quote! { #f_ident: dec.#opt_bool_ident()? });
                        encode_stmts.push(quote::quote! { if let Some(v) = self.#f_ident { enc.#opt_bool_ident(v); } });
                    } else {
                        struct_fields.push(quote::quote! { pub #f_ident: bool });
                        from_exprs.push(quote::quote! { #f_ident: dec.#try_bool_ident().expect("null or invalid bool value") });
                        encode_stmts.push(quote::quote! { enc.#opt_bool_ident(self.#f_ident); });
                    }
                } else {
                    let type_ident = syn::Ident::new(&to_pascal_case(enum_name), span);
                    if f.since_version > 0 {
                        struct_fields.push(quote::quote! { pub #f_ident: Option<#type_ident> });
                        from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                        encode_stmts.push(
                            quote::quote! { if let Some(v) = self.#f_ident { enc.#f_ident(v); } },
                        );
                    } else {
                        struct_fields.push(quote::quote! { pub #f_ident: #type_ident });
                        from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                        encode_stmts.push(quote::quote! { enc.#f_ident(self.#f_ident); });
                    }
                }
            }
            FieldType::Set {
                name: enum_name, ..
            } => {
                let type_ident = syn::Ident::new(&to_pascal_case(enum_name), span);
                if f.since_version > 0 {
                    struct_fields.push(quote::quote! { pub #f_ident: Option<#type_ident> });
                    encode_stmts.push(
                        quote::quote! { if let Some(v) = self.#f_ident { enc.#f_ident(v); } },
                    );
                } else {
                    struct_fields.push(quote::quote! { pub #f_ident: #type_ident });
                    encode_stmts.push(quote::quote! { enc.#f_ident(self.#f_ident); });
                }
                from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
            }
        }
    }

    for g in groups {
        let g_snake = to_snake_case(&g.name);
        let g_pascal = to_pascal_case(&g.name);
        let g_field_ident = syn::Ident::new(&g_snake, span);
        let entry_domain_ident =
            syn::Ident::new(&format!("{struct_prefix}{g_pascal}EntryDomain"), span);

        let g_scoped = if decoder_name.ends_with("Entry") {
            // Nested group: prefix with parent group's scoped name
            let parent_scoped = decoder_name.trim_end_matches("Entry");
            format!("{parent_scoped}{g_pascal}")
        } else if multi_message {
            format!("{msg_name}{g_pascal}")
        } else {
            g_pascal.clone()
        };
        let g_entry_dec_ident = syn::Ident::new(&format!("{g_scoped}EntryDecoder"), span);

        struct_fields.push(quote::quote! { pub #g_field_ident: Vec<#entry_domain_ident> });
        // Fixed-entry groups (no tail) yield entries directly;
        // tailed-entry groups yield Result<EntryDecoder, _>.
        let has_tail = g.has_dynamic_entries();
        if has_tail {
            from_exprs.push(quote::quote! {
                #g_field_ident: dec.#g_field_ident()
                    .map(|g| {
                        g.map(|r| {
                            r.and_then(|entry| #entry_domain_ident::try_from_decoder(entry))
                        })
                        .collect::<Result<Vec<_>, _>>()
                    })
                    .unwrap_or_else(|e| Err(e))?
            });
        } else {
            from_exprs.push(quote::quote! {
                #g_field_ident: dec.#g_field_ident()
                    .map(|g| {
                        g.map(#entry_domain_ident::try_from_decoder)
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .unwrap_or_else(|e| Err(e))?
            });
        }

        let (_, _, count_prim) = get_dim_num_layout(elements, &g.dimension_type);
        let count_ty: syn::Type = syn::parse_str(rust_type(count_prim)).unwrap();
        let can_bulk_encode = domain_entry_can_bulk_encode(
            &g.fields,
            &g.groups,
            &g.var_data,
            elements,
            conversions,
            domain_types,
        );
        if can_bulk_encode {
            group_encode_stmts.push(quote::quote! {
                let count = <#count_ty>::try_from(self.#g_field_ident.len()).map_err(|_| {
                    sbe_rt::EncodeError::ValueOutOfRange {
                        field: "group count",
                        min: 0,
                        max: #count_ty::MAX as i128,
                        actual: self.#g_field_ident.len() as i128,
                    }
                })?;
                let enc = enc.#g_field_ident(
                    count,
                    |g| g.bulk_add_domain(&self.#g_field_ident),
                )?;
            });
        } else {
            group_encode_stmts.push(quote::quote! {
                let count = <#count_ty>::try_from(self.#g_field_ident.len()).map_err(|_| {
                    sbe_rt::EncodeError::ValueOutOfRange {
                        field: "group count",
                        min: 0,
                        max: #count_ty::MAX as i128,
                        actual: self.#g_field_ident.len() as i128,
                    }
                })?;
                let enc = enc.#g_field_ident(
                    count,
                    |g| -> Result<(), sbe_rt::EncodeError> {
                        for e in &self.#g_field_ident {
                            g.add(|entry| -> Result<(), sbe_rt::EncodeError> {
                                e.encode_into(entry)
                            })?;
                        }
                        Ok(())
                    }
                )?;
            });
        }

        let entry_prefix = format!("{struct_prefix}{g_pascal}Entry");
        let entry_decoder_name = format!("{g_scoped}Entry");
        generate_domain_recursive(
            &entry_prefix,
            &entry_decoder_name,
            &g.fields,
            &g.groups,
            &g.var_data,
            elements,
            byte_order,
            multi_message,
            msg_name,
            g.effective_block_length(),
            &conversions,
            domain_types,
            domain_var_data,
            true,
            hooks,
            schema,
            ts,
            span,
        );
        if can_bulk_encode {
            let g_encoder_ident = syn::Ident::new(&format!("{g_scoped}Encoder"), span);
            let mut range_checks = proc_macro2::TokenStream::new();
            for f in &g.fields {
                if let FieldType::Primitive(prim, None) = f.field_type {
                    if f.presence != Presence::Constant {
                        let f_ident = syn::Ident::new(&to_snake_case(&f.name), span);
                        range_checks.extend(dto_range_check_tokens(
                            f,
                            prim,
                            quote::quote! { entry.#f_ident },
                            span,
                        ));
                    }
                }
            }
            let slot_writes = domain_bulk_slot_write_tokens(&g.fields, byte_order, span);
            ts.extend(quote::quote! {
                impl<'a> #g_encoder_ident<'a> {
                    /// Encode flat domain entries with one complete-region bounds check
                    /// and no temporary wire-entry allocation.
                    #[inline]
                    pub fn bulk_add_domain(
                        &mut self,
                        entries: &[#entry_domain_ident],
                    ) -> Result<(), sbe_rt::EncodeError> {
                        self.bulk_add_with(entries, |entry, slot| {
                            #range_checks
                            #slot_writes
                            Ok(())
                        })
                    }
                }
            });
        }
    }

    // Var-data shape from DomainVarData (with_domain_objects argument).
    for vd in var_data {
        let vd_snake = to_snake_case(&vd.name);
        let vd_ident = syn::Ident::new(&vd_snake, span);
        match domain_var_data {
            crate::config::DomainVarData::Strings => {
                // never manufacture empty/default for invalid UTF-8.
                let field_name_lit = syn::LitStr::new(&vd_snake, span);
                struct_fields.push(quote::quote! { pub #vd_ident: String });
                from_exprs.push(quote::quote! {
                    #vd_ident: match dec.#vd_ident() {
                        Ok(data) => match core::str::from_utf8(data) {
                            Ok(s) => s.to_owned(),
                            Err(error) => {
                                return Err(sbe_rt::DecodeError::InvalidUtf8 {
                                    field: #field_name_lit,
                                    error,
                                });
                            }
                        },
                        Err(e) => return Err(e),
                    }
                });
                vardata_encode_stmts.push(quote::quote! {
                    let enc = enc.#vd_ident(self.#vd_ident.as_bytes())?;
                });
            }
            crate::config::DomainVarData::Bytes => {
                struct_fields.push(quote::quote! { pub #vd_ident: Vec<u8> });
                from_exprs.push(quote::quote! {
                    #vd_ident: match dec.#vd_ident() {
                        Ok(data) => data.to_vec(),
                        Err(e) => return Err(e),
                    }
                });
                vardata_encode_stmts.push(quote::quote! {
                    let enc = enc.#vd_ident(&self.#vd_ident)?;
                });
            }
            #[cfg(feature = "compact_str")]
            crate::config::DomainVarData::CompactStrings => {
                let field_name_lit = syn::LitStr::new(&vd_snake, span);
                struct_fields.push(quote::quote! { pub #vd_ident: ergo_sbe::compact_str::CompactString });
                from_exprs.push(quote::quote! {
                    #vd_ident: match dec.#vd_ident() {
                        Ok(data) => match core::str::from_utf8(data) {
                            Ok(s) => ergo_sbe::compact_str::CompactString::new(s),
                            Err(error) => {
                                return Err(sbe_rt::DecodeError::InvalidUtf8 {
                                    field: #field_name_lit,
                                    error,
                                });
                            }
                        },
                        Err(e) => return Err(e),
                    }
                });
                vardata_encode_stmts.push(quote::quote! {
                    let enc = enc.#vd_ident(self.#vd_ident.as_bytes())?;
                });
            }
            #[cfg(feature = "smol_str")]
            crate::config::DomainVarData::SmolStrings => {
                let field_name_lit = syn::LitStr::new(&vd_snake, span);
                struct_fields.push(quote::quote! { pub #vd_ident: ergo_sbe::smol_str::SmolStr });
                from_exprs.push(quote::quote! {
                    #vd_ident: match dec.#vd_ident() {
                        Ok(data) => match core::str::from_utf8(data) {
                            Ok(s) => ergo_sbe::smol_str::SmolStr::new(s),
                            Err(error) => {
                                return Err(sbe_rt::DecodeError::InvalidUtf8 {
                                    field: #field_name_lit,
                                    error,
                                });
                            }
                        },
                        Err(e) => return Err(e),
                    }
                });
                vardata_encode_stmts.push(quote::quote! {
                    let enc = enc.#vd_ident(self.#vd_ident.as_bytes())?;
                });
            }
            #[cfg(feature = "bytes")]
            crate::config::DomainVarData::BytesCrate => {
                struct_fields.push(quote::quote! { pub #vd_ident: ergo_sbe::bytes::Bytes });
                from_exprs.push(quote::quote! {
                    #vd_ident: match dec.#vd_ident() {
                        Ok(data) => ergo_sbe::bytes::Bytes::copy_from_slice(data),
                        Err(e) => return Err(e),
                    }
                });
                vardata_encode_stmts.push(quote::quote! {
                    let enc = enc.#vd_ident(&self.#vd_ident)?;
                });
            }
        }
    }

    let encoder_ident = syn::Ident::new(&format!("{decoder_name}Encoder"), span);

    // Only message-level decoders have decode;
    // entry decoders use wrap() and don't get try_from_slice_with_header.
    //
    // Naming: inherent `try_from_decoder` / `try_from_slice_with_header` instead of
    // `TryFrom`/`From` — two distinct fallible sources (flyweight decoder vs
    // framed byte slice + offset). Std `TryFrom` would collapse both into
    // `try_from` and hide which path validates the message header. No infallible
    // `From`/`from`: materialisation can fail on groups, var-data, and converters.
    let try_from_slice_method: proc_macro2::TokenStream = if !is_entry {
        quote::quote! {
            /// Decode from a framed byte slice: validate the message header, then
            /// materialise the full domain object.
            ///
            /// Distinct from [`Self::try_from_decoder`]: this path owns header
            /// validation + offset; that path starts from an already-wrapped decoder.
            /// Named methods (not `TryFrom`/`From`) keep the two sources obvious.
            #[inline]
            pub fn try_from_slice_with_header(
                buf: &[u8],
                message_offset: usize,
            ) -> Result<Self, sbe_rt::DecodeError> {
                Self::try_from_decoder(
                    #decoder_ident::decode(buf, message_offset)?,
                )
            }
        }
    } else {
        proc_macro2::TokenStream::new()
    };

    let domain_doc: proc_macro2::TokenStream = if is_entry {
        quote::quote! {
            /// Owned domain object — application-layer counterpart to the flyweight decoder.
            ///
            /// Materialise with [`Self::try_from_decoder`] (from a decoder).
            /// This is an inherent method, not `TryFrom`/`From`: conversion is never
            /// infallible (groups, var-data, converters).
        }
    } else {
        quote::quote! {
            /// Materialise with [`Self::try_from_decoder`] (from a decoder) or
            /// [`Self::try_from_slice_with_header`] (from framed bytes).
            /// These are inherent methods, not `TryFrom`/`From`: there are two fallible
            /// sources, and conversion is never infallible (groups, var-data, converters).
        }
    };

    let try_from_decoder_doc: proc_macro2::TokenStream = if is_entry {
        quote::quote! {
            /// Fallible conversion from a flyweight decoder.
            ///
            /// Propagates decode errors from malformed group entries and var-data
            /// instead of panicking. Prefer this over `From`/`TryFrom`.
        }
    } else {
        quote::quote! {
            /// Fallible conversion from a flyweight decoder.
            ///
            /// Propagates decode errors from malformed group entries and var-data
            /// instead of panicking. Prefer this over `From`/`TryFrom`: the companion
            /// entry point is [`Self::try_from_slice_with_header`] (when generated),
            /// and named methods make the two sources unambiguous.
        }
    };

    ts.extend(quote::quote! {
        /// Owned domain object — application-layer counterpart to the flyweight decoder.
        ///
        #domain_doc
        #[derive(Debug, Clone, PartialEq)]
        pub struct #domain_ident {
            #(#struct_fields),*
        }

        impl #domain_ident {
            #try_from_decoder_doc
            #[inline]
            pub fn try_from_decoder(
                dec: #decoder_ident<'_>,
            ) -> Result<Self, sbe_rt::DecodeError> {
                Ok(Self {
                    #(#from_exprs),*
                })
            }

            #try_from_slice_method
        }
    });

    // Fire hooks for this domain struct (message DTO or entry DTO).
    // Build context including group and var-data field info.
    if !hooks.is_empty() {
        let mut ctx_fields = message_field_infos(fields, domain_types, Some(elements));
        // Append synthetic field entries for groups (Vec<EntryDomain>).
        // The generated entry-DTO struct is `{struct_prefix}{Group}EntryDomain`
        // (see the recursion below), so the reported type must carry the same
        // prefix — a bare `Vec<{Group}EntryDomain>` names a type that does not
        // exist.
        for g in groups {
            ctx_fields.push(crate::FieldInfo {
                name: to_snake_case(&g.name),
                rust_type: format!("Vec<{struct_prefix}{}EntryDomain>", to_pascal_case(&g.name)),
                offset: None,
                since_version: g.since_version,
                semantic_type: None,
                presence: "required",
                null_value: None,
                deprecated: false,
                description: g.description.clone(),
            });
        }
        // Append synthetic field entries for var-data
        for vd in var_data {
            let vd_ty = match domain_var_data {
                crate::config::DomainVarData::Bytes => "Vec<u8>",
                crate::config::DomainVarData::Strings => "String",
                #[cfg(feature = "compact_str")]
                crate::config::DomainVarData::CompactStrings => "ergo_sbe::compact_str::CompactString",
                #[cfg(feature = "smol_str")]
                crate::config::DomainVarData::SmolStrings => "ergo_sbe::smol_str::SmolStr",
                #[cfg(feature = "bytes")]
                crate::config::DomainVarData::BytesCrate => "ergo_sbe::bytes::Bytes",
            };
            ctx_fields.push(crate::FieldInfo {
                name: to_snake_case(&vd.name),
                rust_type: vd_ty.to_string(),
                offset: None,
                since_version: vd.since_version,
                semantic_type: None,
                presence: "required",
                null_value: None,
                deprecated: false,
                description: vd.description.clone(),
            });
        }
        let ctx = crate::ItemContext::DomainStruct {
            schema,
            name: struct_prefix.to_string() + "Domain",
            fields: ctx_fields,
        };
        for hook in hooks.iter() {
            for token_stream in hook(&ctx) {
                ts.extend(token_stream);
            }
        }
    }

    if is_entry {
        // Entry domains: encode_into for use inside group closures
        let entry_encoder_ident = syn::Ident::new(&format!("{decoder_name}Encoder"), span);
        let encode_body = if !vardata_encode_stmts.is_empty() || !group_encode_stmts.is_empty() {
            quote::quote! {
                #(#encode_stmts)*
                #(#group_encode_stmts)*
                #(#vardata_encode_stmts)*
                Ok(())
            }
        } else {
            quote::quote! {
                #(#encode_stmts)*
                Ok(())
            }
        };

        let entry_block_len = groups.iter().fold(
            fields.iter().fold(0usize, |acc, f| {
                let size = f.field_type.size();
                acc.max(f.offset + size)
            }),
            |acc, g| acc.max(g.effective_block_length()),
        );
        let entry_bl_lit = syn::LitInt::new(&entry_block_len.to_string(), span);
        let mut len_stmts = quote::quote! {
            let mut len: usize = #entry_bl_lit;
        };
        for ng in groups {
            let ng_snake = syn::Ident::new(&to_snake_case(&ng.name), span);
            let (_, dim_size, _, _) = get_dimension_info(elements, &ng.dimension_type);
            let ds_lit = syn::LitInt::new(&dim_size.to_string(), span);
            len_stmts.extend(quote::quote! {
                len = len.checked_add(#ds_lit).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                for entry in &self.#ng_snake {
                    len = len.checked_add(entry.length_contribution()?).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                }
            });
        }
        for vd in var_data {
            let vd_snake = syn::Ident::new(&to_snake_case(&vd.name), span);
            let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
            let ps_lit = syn::LitInt::new(&prefix_size.to_string(), span);
            if let Some(max) = vd.max_length {
                let max_lit = syn::LitInt::new(&max.to_string(), span);
                let vd_name = &vd.name;
                len_stmts.extend(quote::quote! {
                    if self.#vd_snake.len() > #max_lit {
                        return Err(sbe_rt::EncodeError::VarDataTooLong {
                            field: #vd_name,
                            max_length: #max_lit,
                            actual: self.#vd_snake.len(),
                        });
                    }
                });
            }
            len_stmts.extend(quote::quote! {
                len = len.checked_add(#ps_lit).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                len = len.checked_add(self.#vd_snake.len()).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
            });
        }
        len_stmts.extend(quote::quote! { Ok(len) });

        ts.extend(quote::quote! {
            impl #domain_ident {
                #[inline]
                pub fn encode_into<'a>(
                    &self,
                    enc: &mut #entry_encoder_ident<'a>,
                ) -> Result<(), sbe_rt::EncodeError> {
                    #encode_body
                }

                /// Compute this entry's contribution to the total encoded length
                /// (entry block + nested groups + entry var-data).
                #[inline]
                pub fn length_contribution(&self) -> Result<usize, sbe_rt::EncodeError> {
                    #len_stmts
                }
            }
        });

        // Preserve the explicit domain-to-wire conversion helper for callers
        // that build their own wire-entry slices.
        if domain_entry_can_bulk_encode(
            fields,
            groups,
            var_data,
            elements,
            conversions,
            domain_types,
        ) {
            let wire_entry_ident = syn::Ident::new(decoder_name, span);
            let mut wire_fields = proc_macro2::TokenStream::new();
            for f in fields {
                if f.presence == Presence::Constant {
                    continue;
                }
                let f_ident = syn::Ident::new(&to_snake_case(&f.name), span);
                wire_fields.extend(quote::quote! {
                    #f_ident: self.#f_ident,
                });
            }
            ts.extend(quote::quote! {
                impl #domain_ident {
                    /// Convert to the wire entry struct for bulk encoding.
                    #[inline]
                    pub fn to_wire_entry(&self) -> #wire_entry_ident {
                        #wire_entry_ident {
                            #wire_fields
                        }
                    }
                }
            });
        }
    } else {
        // Message domains: full encode via try_wrap_and_apply_header (checked)
        let has_optional = fields
            .iter()
            .any(|f| f.presence == Presence::Optional && f.null_value.is_some());
        let nullify = if has_optional {
            quote::quote! { enc.apply_nulls(); }
        } else {
            quote::quote! {}
        };
        // Use the schema-declared padded block length, not the last field's
        // extent — the schema may include trailing padding or future-version
        // reservation beyond the last field's offset + size.
        let block_len = message_block_length;
        let bl_lit = syn::LitInt::new(&block_len.to_string(), span);
        let mut msg_len_stmts = quote::quote! {
            let mut len: usize = #bl_lit;
        };
        for g in groups {
            let g_snake = syn::Ident::new(&to_snake_case(&g.name), span);
            let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
            let ds_lit = syn::LitInt::new(&dim_size.to_string(), span);
            msg_len_stmts.extend(quote::quote! {
                len = len.checked_add(#ds_lit).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                for entry in &self.#g_snake {
                    len = len.checked_add(entry.length_contribution()?).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                }
            });
        }
        for vd in var_data {
            let vd_snake = syn::Ident::new(&to_snake_case(&vd.name), span);
            let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
            let ps_lit = syn::LitInt::new(&prefix_size.to_string(), span);
            if let Some(max) = vd.max_length {
                let max_lit = syn::LitInt::new(&max.to_string(), span);
                let vd_name = &vd.name;
                msg_len_stmts.extend(quote::quote! {
                    if self.#vd_snake.len() > #max_lit {
                        return Err(sbe_rt::EncodeError::VarDataTooLong {
                            field: #vd_name,
                            max_length: #max_lit,
                            actual: self.#vd_snake.len(),
                        });
                    }
                });
            }
            msg_len_stmts.extend(quote::quote! {
                len = len.checked_add(#ps_lit).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                len = len.checked_add(self.#vd_snake.len()).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
            });
        }
        msg_len_stmts.extend(quote::quote! { Ok(len) });
        let has_tail = !group_encode_stmts.is_empty() || !vardata_encode_stmts.is_empty();
        let encode_body = if has_tail {
            quote::quote! {
                let mut enc = #encoder_ident::try_wrap_and_apply_header(buf, 0)?;
                #nullify
                #(#encode_stmts)*
                #(#group_encode_stmts)*
                #(#vardata_encode_stmts)*
                Ok(enc.encoded_length() + #encoder_ident::HEADER_LENGTH)
            }
        } else {
            // Fixed-only message: encoder implements AsRef<[u8]>
            quote::quote! {
                let mut enc = #encoder_ident::try_wrap_and_apply_header(buf, 0)?;
                #nullify
                #(#encode_stmts)*
                Ok(enc.encoded_length() + #encoder_ident::HEADER_LENGTH)
            }
        };
        ts.extend(quote::quote! {
            impl #domain_ident {
                #[inline]
                pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
                    #encode_body
                }

                /// Compute the exact SBE message body length from this domain object.
                /// Matches the length returned by [`Self::encode`].
                #[inline]
                pub fn encoded_length(&self) -> Result<usize, sbe_rt::EncodeError> {
                    #msg_len_stmts
                }

                /// Compute the exact SBE message length including the message header.
                /// Matches `encode()` return value for non-fixed messages.
                #[inline]
                pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::EncodeError> {
                    Ok(self.encoded_length()? + #encoder_ident::HEADER_LENGTH)
                }
            }
        });
    }
}

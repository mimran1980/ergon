//! Mutable ordered decoder codegen.
//!
//! `{Name}Decoder::ordered()` yields a cursor that keeps compile-time-free
//! sequential access to groups and var-data, with runtime `OutOfOrder` checks.
//! Group methods return a guard that borrows the parent until it is consumed.

use crate::ir::{ByteOrder, Presence, PrimitiveType};
use crate::structured_ir::{
    FieldType, MessageField, MessageGroup, MessageStructure, MessageVarData, SchemaElements,
    get_dimension_info, get_vardata_info, rust_type,
};

use super::conversion_helpers::{
    DECODER_RESERVED, field_has_conversion_free, find_domain_type, resolve_field_ident,
};
use super::converter_impls::is_optional_domain_field;
use super::runtime::{to_pascal_case, to_snake_case};

pub(crate) fn generate_ordered_decoder(
    msg: &MessageStructure,
    elements: &SchemaElements,
    name: &str,
    _header_size: usize,
    byte_order: ByteOrder,
    _multi_message: bool,
    group_unique_names: &[String],
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    enable_dispatch: bool,
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let decoder_ident = syn::Ident::new(&format!("{name}Decoder"), span);
    let ordered_ident = syn::Ident::new(&format!("{name}OrderedDecoder"), span);
    let metadata_ident = syn::Ident::new(&format!("{name}DecoderMetadata"), span);
    let owner_lit = syn::LitStr::new(name, span);
    let total_tail = msg.groups.len() + msg.var_data.len();
    // A fixed-block message has nothing to order: every field is random-access
    // off the block, and the base decoder already reads them all. Emitting a
    // cursor that only forwards fixed getters would be a second name for the
    // same thing, so the lane simply does not exist for these messages.
    if total_tail == 0 {
        return proc_macro2::TokenStream::new();
    }

    let forwards = forward_fixed_fields(
        &msg.fields,
        conversions,
        domain_types,
        null_as_option,
        all_enums_as_option,
    );

    let mut ts = quote::quote! {
        impl<'a> #decoder_ident<'a> {
            /// Convert this flyweight into a mutable ordered cursor.
            ///
            /// Group and var-data methods must then be called in schema order;
            /// a wrong call returns [`sbe_rt::DecodeError::OutOfOrder`] and
            /// leaves the cursor unchanged. Fixed fields stay random-access.
            #[inline]
            pub fn ordered(self) -> #ordered_ident<'a> {
                let tail_offset = self.offset + self.acting_block_length;
                #ordered_ident {
                    inner: self,
                    tail_offset,
                    next_ordinal: 0,
                }
            }
        }

        /// Mutable ordered decoder — sequential dynamic tails, random-access
        /// fixed fields, runtime order checks.
        #[must_use = "decoder must be read or advanced; dropping is fine only after use"]
        pub struct #ordered_ident<'a> {
            inner: #decoder_ident<'a>,
            tail_offset: usize,
            next_ordinal: u16,
        }

        impl<'a> #ordered_ident<'a> {
            /// Schema version from the message header (or wrap args).
            #[inline]
            pub const fn acting_version(&self) -> u16 {
                self.inner.acting_version()
            }
            /// Block length from the wire header / wrap args.
            #[inline]
            pub const fn acting_block_length(&self) -> usize {
                self.inner.acting_block_length()
            }
            /// Placement utilities. Does not expose random-access dynamic tails.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn get_metadata(&self) -> #metadata_ident<'_, 'a> {
                self.inner.get_metadata()
            }
            #forwards
        }
    };

    let complete_ident = syn::Ident::new(&format!("{name}DecoderComplete"), span);
    let names: Vec<syn::LitStr> = msg
        .groups
        .iter()
        .map(|g| syn::LitStr::new(&g.name, span))
        .chain(msg.var_data.iter().map(|v| syn::LitStr::new(&v.name, span)))
        .collect();
    let n_tails_lit = syn::LitInt::new(&total_tail.to_string(), span);
    let name_array = quote::quote! { [#(#names),*] };

    let expect_fn = quote::quote! {
        #[inline]
        fn expect(&self, ordinal: u16, requested: &'static str) -> Result<(), sbe_rt::DecodeError> {
            const NAMES: &[&str] = &#name_array;
            let expected = if (self.next_ordinal as usize) < NAMES.len() {
                NAMES[self.next_ordinal as usize]
            } else {
                "<complete>"
            };
            if self.next_ordinal != ordinal {
                return Err(sbe_rt::DecodeError::OutOfOrder {
                    owner: #owner_lit,
                    expected,
                    requested,
                });
            }
            Ok(())
        }
    };

    let mut tail_methods = proc_macro2::TokenStream::new();
    let mut finish_arms = proc_macro2::TokenStream::new();
    let mut nested = proc_macro2::TokenStream::new();

    for (gi, g) in msg.groups.iter().enumerate() {
        let unique = &group_unique_names[gi];
        let snake = to_snake_case(&g.name);
        let ident = syn::Ident::new(&snake, span);
        let guard_ident = syn::Ident::new(&format!("{unique}OrderedDecoder"), span);
        let ordinal_lit = syn::LitInt::new(&(gi as u16).to_string(), span);
        let name_lit = syn::LitStr::new(&g.name, span);
        tail_methods.extend(quote::quote! {
            #[inline]
            pub fn #ident(
                &mut self,
            ) -> Result<#guard_ident<'_, 'a>, sbe_rt::DecodeError> {
                self.expect(#ordinal_lit, #name_lit)?;
                #guard_ident::begin(self)
            }
        });
        finish_arms.extend(quote::quote! {
            #ordinal_lit => self.#ident()?.skip_remaining()?,
        });
        nested.extend(generate_group_guard(
            g,
            elements,
            unique,
            quote::quote! { #ordered_ident<'a> },
            byte_order,
            enable_dispatch,
            conversions,
            domain_types,
            null_as_option,
            all_enums_as_option,
        ));
    }

    for (vi, vd) in msg.var_data.iter().enumerate() {
        let i = msg.groups.len() + vi;
        let snake = to_snake_case(&vd.name);
        let ident = syn::Ident::new(&snake, span);
        let ordinal_lit = syn::LitInt::new(&(i as u16).to_string(), span);
        let name_lit = syn::LitStr::new(&vd.name, span);
        let read = var_data_read(vd, elements, byte_order);
        tail_methods.extend(quote::quote! {
            #[inline]
            pub fn #ident(&mut self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                self.expect(#ordinal_lit, #name_lit)?;
                let (data, end) = { #read };
                self.tail_offset = end;
                self.next_ordinal = self.next_ordinal.saturating_add(1);
                Ok(data)
            }
        });
        tail_methods.extend(var_data_as_str_methods(
            vd,
            &ident,
            &ordinal_lit,
            &name_lit,
            &read,
            &msg.fields,
        ));
        if enable_dispatch {
            let as_msg = syn::Ident::new(&format!("{snake}_as_message"), span);
            tail_methods.extend(quote::quote! {
                #[inline]
                pub fn #as_msg(&mut self) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
                    self.expect(#ordinal_lit, #name_lit)?;
                    let (data, end) = { #read };
                    let frame = AnyMessage::decode_frame(data, 0, data.len())?;
                    self.tail_offset = end;
                    self.next_ordinal = self.next_ordinal.saturating_add(1);
                    Ok(frame)
                }
            });
        }
        finish_arms.extend(quote::quote! {
            #ordinal_lit => { let _ = self.#ident()?; }
        });
    }

    ts.extend(quote::quote! {
        impl<'a> #ordered_ident<'a> {
            #expect_fn
            #tail_methods
            /// Skip any unconsumed suffix and return the complete stage.
            #[inline]
            pub fn finish(mut self) -> Result<#complete_ident<'a>, sbe_rt::DecodeError> {
                while (self.next_ordinal as usize) < #n_tails_lit {
                    match self.next_ordinal {
                        #finish_arms
                        _ => break,
                    }
                }
                Ok(#complete_ident {
                    buf: self.inner.buf,
                    offset: self.inner.offset,
                    tail_start: self.tail_offset,
                    acting_version: self.inner.acting_version,
                    acting_block_length: self.inner.acting_block_length,
                })
            }
        }
    });
    ts.extend(nested);
    ts
}

/// `owner_fields` are the fixed fields forwarded onto the same type. A field
/// whose accessor already spells `<vd>_as_str` wins the name: entry fields keep
/// their schema spelling in every entry location, so renaming one here would
/// give the same field different names per location. `note()` still returns
/// the bytes. At message level the field would already have been renamed by
/// `DECODER_RESERVED`, so this never fires there.
fn var_data_as_str_methods(
    vd: &MessageVarData,
    ident: &syn::Ident,
    ordinal_lit: &syn::LitInt,
    name_lit: &syn::LitStr,
    read: &proc_macro2::TokenStream,
    owner_fields: &[MessageField],
) -> proc_macro2::TokenStream {
    let Some(ref enc) = vd.character_encoding else {
        return proc_macro2::TokenStream::new();
    };
    let claimed = format!("{ident}_as_str");
    if owner_fields
        .iter()
        .any(|f| to_snake_case(&f.name) == claimed)
    {
        return proc_macro2::TokenStream::new();
    }
    let is_utf8 = enc.eq_ignore_ascii_case("UTF-8") || enc.eq_ignore_ascii_case("UTF8");
    let is_ascii = enc.eq_ignore_ascii_case("ASCII") || enc.eq_ignore_ascii_case("US-ASCII");
    if !is_utf8 && !is_ascii {
        return proc_macro2::TokenStream::new();
    }
    let span = proc_macro2::Span::call_site();
    let as_str = syn::Ident::new(&format!("{ident}_as_str"), span);
    let _ = ident;
    if is_ascii {
        quote::quote! {
            #[inline]
            pub fn #as_str(&mut self) -> Result<&'a str, sbe_rt::DecodeError> {
                self.expect(#ordinal_lit, #name_lit)?;
                let (data, end) = { #read };
                if !data.is_ascii() {
                    return Err(sbe_rt::DecodeError::InvalidAscii { field: #name_lit });
                }
                self.tail_offset = end;
                self.next_ordinal = self.next_ordinal.saturating_add(1);
                Ok(unsafe { core::str::from_utf8_unchecked(data) })
            }
        }
    } else {
        quote::quote! {
            #[inline]
            pub fn #as_str(&mut self) -> Result<&'a str, sbe_rt::DecodeError> {
                self.expect(#ordinal_lit, #name_lit)?;
                let (data, end) = { #read };
                let s = core::str::from_utf8(data).map_err(|e| {
                    sbe_rt::DecodeError::InvalidUtf8 { field: #name_lit, error: e }
                })?;
                self.tail_offset = end;
                self.next_ordinal = self.next_ordinal.saturating_add(1);
                Ok(s)
            }
        }
    }
}

fn var_data_read(
    vd: &MessageVarData,
    elements: &SchemaElements,
    byte_order: ByteOrder,
) -> proc_macro2::TokenStream {
    var_data_read_from(vd, elements, byte_order, quote::quote! { self.inner })
}

fn var_data_read_from(
    vd: &MessageVarData,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    inner: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let (_, prefix_size, _, len_type) = get_vardata_info(elements, &vd.type_name);
    let prefix_size_lit = syn::LitInt::new(&prefix_size.to_string(), span);
    let len_ty = syn::Ident::new(rust_type(len_type), span);
    let from_endian = syn::Ident::new(
        match byte_order {
            ByteOrder::LittleEndian => "from_le_bytes",
            ByteOrder::BigEndian => "from_be_bytes",
        },
        span,
    );
    let name_lit = syn::LitStr::new(&vd.name, span);
    let since = vd.since_version;
    let mut max_check = proc_macro2::TokenStream::new();
    if let Some(max) = vd.max_length {
        let max_lit = syn::LitInt::new(&max.to_string(), span);
        max_check.extend(quote::quote! {
            if len > #max_lit {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: #name_lit,
                    length: len,
                    max_length: #max_lit as u64,
                });
            }
        });
    }
    let parse = quote::quote! {
        let offset = self.tail_offset;
        if offset + #prefix_size_lit > #inner.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: #name_lit,
                needed: #prefix_size_lit,
                available: #inner.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; #prefix_size_lit] =
            read_bytes::<#prefix_size_lit>(#inner.buf, offset);
        let len = #len_ty::#from_endian(bytes) as u64;
        #max_check
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            #name_lit,
            offset,
            #prefix_size_lit,
            len,
            #inner.buf.len(),
        )?;
        (&#inner.buf[data_start..data_end], data_end)
    };
    if since > 0 {
        let since_lit = syn::LitInt::new(&since.to_string(), span);
        quote::quote! {
            if #inner.acting_version < #since_lit {
                (&[][..], self.tail_offset)
            } else {
                #parse
            }
        }
    } else {
        parse
    }
}

fn generate_group_guard(
    g: &MessageGroup,
    elements: &SchemaElements,
    unique: &str,
    parent_ty: proc_macro2::TokenStream,
    byte_order: ByteOrder,
    enable_dispatch: bool,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
) -> proc_macro2::TokenStream {
    generate_group_guard_inner(
        g,
        elements,
        unique,
        parent_ty,
        byte_order,
        enable_dispatch,
        conversions,
        domain_types,
        null_as_option,
        all_enums_as_option,
        false,
    )
}

fn generate_nested_group_guard(
    g: &MessageGroup,
    elements: &SchemaElements,
    unique: &str,
    parent_ty: proc_macro2::TokenStream,
    byte_order: ByteOrder,
    enable_dispatch: bool,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
) -> proc_macro2::TokenStream {
    generate_group_guard_inner(
        g,
        elements,
        unique,
        parent_ty,
        byte_order,
        enable_dispatch,
        conversions,
        domain_types,
        null_as_option,
        all_enums_as_option,
        true,
    )
}

fn generate_group_guard_inner(
    g: &MessageGroup,
    elements: &SchemaElements,
    unique: &str,
    parent_ty: proc_macro2::TokenStream,
    byte_order: ByteOrder,
    enable_dispatch: bool,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
    nested: bool,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let guard_ident = syn::Ident::new(&format!("{unique}OrderedDecoder"), span);
    let entry_ident = syn::Ident::new(&format!("{unique}EntryDecoder"), span);
    let entry_ord_ident = syn::Ident::new(&format!("{unique}EntryOrderedDecoder"), span);
    let flyweight_ident = syn::Ident::new(&format!("{unique}Decoder"), span);
    let (dim_name, dim_size, bl_field, count_field) =
        get_dimension_info(elements, &g.dimension_type);
    let dim_ident = syn::Ident::new(&dim_name, span);
    let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), span);
    let bl_ident = syn::Ident::new(&bl_field, span);
    let count_ident = syn::Ident::new(&count_field, span);
    let g_name_lit = syn::LitStr::new(&g.name, span);
    let begin_ident = if nested {
        syn::Ident::new("begin_entry", span)
    } else {
        syn::Ident::new("begin", span)
    };
    let since = g.since_version;
    let absent = if since > 0 {
        let since_lit = syn::LitInt::new(&since.to_string(), span);
        quote::quote! {
            if parent.inner.acting_version < #since_lit {
                return Ok(Self {
                    buf: parent.inner.buf,
                    offset: start,
                    count: 0,
                    acting_block_length: 0,
                    acting_version: parent.inner.acting_version,
                    min_entry_extent: 0,
                    parent,
                });
            }
        }
    } else {
        quote::quote! {}
    };
    let dyn_extent = if g.has_dynamic_entries() {
        quote::quote! {
            let min_fixed = <#flyweight_ident<'_, sbe_rt::Detached>>::min_readable_fixed_extent(
                parent.inner.acting_version,
            );
            let min_entry_extent = if block_length > min_fixed { block_length } else { min_fixed };
            if count > 0 && block_length < min_fixed {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: #g_name_lit,
                    needed: min_fixed,
                    available: block_length,
                });
            }
        }
    } else {
        // Fixed stride: two independent things must be proven HERE, because
        // `visit_entries` hands each entry to a caller callback without any
        // per-entry check (there is no per-entry extent to compute).
        //
        // 1. The wire `blockLength` must be able to hold the fixed fields
        //    active at this version. A short stride is malformed: a required
        //    getter reads at a compiled offset inside the entry and would run
        //    past it. This is checked FIRST, so an undersized stride is
        //    rejected as malformed rather than sliding under a
        //    `count * blockLength` region check that a small stride makes
        //    trivially satisfiable.
        // 2. The whole `count * blockLength` region must be in bounds, or a
        //    large `numInGroup` on a truncated buffer reads past the end.
        //
        // `group_decoder.rs` has always validated both; the ordered lane
        // validated neither, then only (2).
        quote::quote! {
            let min_entry_extent = 0usize;
            let min_fixed = <#flyweight_ident<'_, sbe_rt::Detached>>::min_readable_fixed_extent(
                parent.inner.acting_version,
            );
            if count > 0 && block_length < min_fixed {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: #g_name_lit,
                    needed: min_fixed,
                    available: block_length,
                });
            }
            let entries_start = start + #dim_size_lit;
            let available = parent.inner.buf.len().saturating_sub(entries_start);
            let entries_length = count.checked_mul(block_length).ok_or(
                sbe_rt::DecodeError::BufferTooShort {
                    field: #g_name_lit,
                    needed: usize::MAX,
                    available,
                },
            )?;
            if entries_length > available {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: #g_name_lit,
                    needed: entries_length,
                    available,
                });
            }
        }
    };
    let visit_body = if g.has_dynamic_entries() {
        quote::quote! {
            while self.count > 0 {
                let available = self.buf.len().saturating_sub(self.offset);
                if self.min_entry_extent > available {
                    return Err(E::from(sbe_rt::DecodeError::BufferTooShort {
                        field: #g_name_lit,
                        needed: self.min_entry_extent,
                        available,
                    }));
                }
                let mut entry = #entry_ord_ident::at(
                    self.buf,
                    self.offset,
                    self.acting_block_length,
                    self.acting_version,
                );
                visit(&mut entry)?;
                self.offset = entry.finish_unread()?;
                self.count -= 1;
            }
        }
    } else {
        quote::quote! {
            while self.count > 0 {
                let mut entry = #entry_ord_ident::at(
                    self.buf,
                    self.offset,
                    self.acting_block_length,
                    self.acting_version,
                );
                visit(&mut entry)?;
                self.offset += self.acting_block_length;
                self.count -= 1;
            }
        }
    };

    let mut ts = quote::quote! {
        pub struct #guard_ident<'p, 'a> {
            buf: &'a [u8],
            offset: usize,
            count: usize,
            acting_block_length: usize,
            acting_version: u16,
            min_entry_extent: usize,
            parent: &'p mut #parent_ty,
        }
        impl<'p, 'a> #guard_ident<'p, 'a> {
            #[inline]
            fn #begin_ident(parent: &'p mut #parent_ty) -> Result<Self, sbe_rt::DecodeError> {
                let start = parent.tail_offset;
                #absent
                if #dim_size_lit > parent.inner.buf.len().saturating_sub(start) {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: #g_name_lit,
                        needed: #dim_size_lit,
                        available: parent.inner.buf.len().saturating_sub(start),
                    });
                }
                let bytes: [u8; #dim_size_lit] =
                    read_bytes::<#dim_size_lit>(parent.inner.buf, start);
                let header = #dim_ident(bytes);
                let count = sbe_rt::checked_group_count(
                    "numInGroup",
                    header.#count_ident() as u64,
                )?;
                let block_length = sbe_rt::checked_header_usize(
                    "blockLength",
                    header.#bl_ident() as u64,
                )?;
                #dyn_extent
                Ok(Self {
                    buf: parent.inner.buf,
                    offset: start + #dim_size_lit,
                    count,
                    acting_block_length: block_length,
                    acting_version: parent.inner.acting_version,
                    min_entry_extent,
                    parent,
                })
            }
            #[inline]
            pub const fn remaining_entries(&self) -> usize {
                self.count
            }
            #[inline]
            pub const fn is_empty(&self) -> bool {
                self.count == 0
            }
            #[inline]
            pub fn visit_entries<E, F>(mut self, mut visit: F) -> Result<(), E>
            where
                E: From<sbe_rt::DecodeError>,
                F: FnMut(&mut #entry_ord_ident<'a>) -> Result<(), E>,
            {
                #visit_body
                self.commit();
                Ok(())
            }
            #[inline]
            pub fn finish(mut self) -> Result<(), sbe_rt::DecodeError> {
                while self.count > 0 {
                    self.offset = #entry_ident::skip(
                        self.buf,
                        self.offset,
                        self.acting_block_length,
                        self.acting_version,
                    )?;
                    self.count -= 1;
                }
                self.commit();
                Ok(())
            }
            #[inline]
            pub fn skip_remaining(self) -> Result<(), sbe_rt::DecodeError> {
                self.finish()
            }
            #[inline]
            fn commit(self) {
                self.parent.tail_offset = self.offset;
                self.parent.next_ordinal = self.parent.next_ordinal.saturating_add(1);
            }
        }
    };

    ts.extend(generate_entry_ordered(
        g,
        elements,
        unique,
        byte_order,
        enable_dispatch,
        conversions,
        domain_types,
        null_as_option,
        all_enums_as_option,
    ));
    ts
}

fn generate_entry_ordered(
    g: &MessageGroup,
    elements: &SchemaElements,
    unique: &str,
    byte_order: ByteOrder,
    enable_dispatch: bool,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let entry_ord_ident = syn::Ident::new(&format!("{unique}EntryOrderedDecoder"), span);
    let entry_ident = syn::Ident::new(&format!("{unique}EntryDecoder"), span);
    let total_tail = g.groups.len() + g.var_data.len();
    let forwards = forward_entry_fields(
        &g.fields,
        conversions,
        domain_types,
        null_as_option,
        all_enums_as_option,
    );

    if total_tail == 0 {
        return quote::quote! {
            pub struct #entry_ord_ident<'a> {
                inner: #entry_ident<'a>,
            }
            impl<'a> #entry_ord_ident<'a> {
                #[inline]
                fn at(
                    buf: &'a [u8],
                    offset: usize,
                    acting_block_length: usize,
                    acting_version: u16,
                ) -> Self {
                    Self {
                        inner: unsafe {
                            #entry_ident::wrap(buf, offset, acting_block_length, acting_version)
                        },
                    }
                }
                #forwards
            }
        };
    }

    let owner_lit = syn::LitStr::new(unique, span);
    let names: Vec<syn::LitStr> = g
        .groups
        .iter()
        .map(|ng| syn::LitStr::new(&ng.name, span))
        .chain(g.var_data.iter().map(|v| syn::LitStr::new(&v.name, span)))
        .collect();
    let name_array = quote::quote! { [#(#names),*] };
    let n_tails_lit = syn::LitInt::new(&total_tail.to_string(), span);

    let mut tail_methods = proc_macro2::TokenStream::new();
    let mut nested = proc_macro2::TokenStream::new();
    let mut skip_arms = proc_macro2::TokenStream::new();

    for (gi, ng) in g.groups.iter().enumerate() {
        let ng_unique = format!("{}{}", unique, to_pascal_case(&ng.name));
        let snake = to_snake_case(&ng.name);
        let ident = syn::Ident::new(&snake, span);
        let guard_ident = syn::Ident::new(&format!("{ng_unique}OrderedDecoder"), span);
        let ordinal_lit = syn::LitInt::new(&(gi as u16).to_string(), span);
        let name_lit = syn::LitStr::new(&ng.name, span);
        tail_methods.extend(quote::quote! {
            #[inline]
            pub fn #ident(
                &mut self,
            ) -> Result<#guard_ident<'_, 'a>, sbe_rt::DecodeError> {
                self.expect(#ordinal_lit, #name_lit)?;
                #guard_ident::begin_entry(self)
            }
        });
        skip_arms.extend(quote::quote! {
            #ordinal_lit => { self.#ident()?.skip_remaining()?; }
        });
        nested.extend(generate_nested_group_guard(
            ng,
            elements,
            &ng_unique,
            quote::quote! { #entry_ord_ident<'a> },
            byte_order,
            enable_dispatch,
            conversions,
            domain_types,
            null_as_option,
            all_enums_as_option,
        ));
    }

    for (vi, vd) in g.var_data.iter().enumerate() {
        let i = g.groups.len() + vi;
        let snake = to_snake_case(&vd.name);
        let ident = syn::Ident::new(&snake, span);
        let ordinal_lit = syn::LitInt::new(&(i as u16).to_string(), span);
        let name_lit = syn::LitStr::new(&vd.name, span);
        let read = var_data_read_from(vd, elements, byte_order, quote::quote! { self.inner });
        tail_methods.extend(quote::quote! {
            #[inline]
            pub fn #ident(&mut self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                self.expect(#ordinal_lit, #name_lit)?;
                let (data, end) = { #read };
                self.tail_offset = end;
                self.next_ordinal = self.next_ordinal.saturating_add(1);
                Ok(data)
            }
        });
        tail_methods.extend(var_data_as_str_methods(
            vd,
            &ident,
            &ordinal_lit,
            &name_lit,
            &read,
            &g.fields,
        ));
        if enable_dispatch {
            let as_msg = syn::Ident::new(&format!("{snake}_as_message"), span);
            tail_methods.extend(quote::quote! {
                #[inline]
                pub fn #as_msg(&mut self) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
                    self.expect(#ordinal_lit, #name_lit)?;
                    let (data, end) = { #read };
                    let frame = AnyMessage::decode_frame(data, 0, data.len())?;
                    self.tail_offset = end;
                    self.next_ordinal = self.next_ordinal.saturating_add(1);
                    Ok(frame)
                }
            });
        }
        skip_arms.extend(quote::quote! {
            #ordinal_lit => { let _ = self.#ident()?; }
        });
    }

    let mut ts = quote::quote! {
        pub struct #entry_ord_ident<'a> {
            inner: #entry_ident<'a>,
            tail_offset: usize,
            next_ordinal: u16,
        }
        impl<'a> #entry_ord_ident<'a> {
            #[inline]
            fn at(
                buf: &'a [u8],
                offset: usize,
                acting_block_length: usize,
                acting_version: u16,
            ) -> Self {
                Self {
                    inner: unsafe {
                        #entry_ident::wrap(buf, offset, acting_block_length, acting_version)
                    },
                    tail_offset: offset + acting_block_length,
                    next_ordinal: 0,
                }
            }
            #[inline]
            fn expect(&self, ordinal: u16, requested: &'static str) -> Result<(), sbe_rt::DecodeError> {
                const NAMES: &[&str] = &#name_array;
                let expected = if (self.next_ordinal as usize) < NAMES.len() {
                    NAMES[self.next_ordinal as usize]
                } else {
                    "<complete>"
                };
                if self.next_ordinal != ordinal {
                    return Err(sbe_rt::DecodeError::OutOfOrder {
                        owner: #owner_lit,
                        expected,
                        requested,
                    });
                }
                Ok(())
            }
            #forwards
            #tail_methods
            #[inline]
            fn finish_unread(mut self) -> Result<usize, sbe_rt::DecodeError> {
                while (self.next_ordinal as usize) < #n_tails_lit {
                    match self.next_ordinal {
                        #skip_arms
                        _ => break,
                    }
                }
                Ok(self.tail_offset)
            }
        }
    };
    ts.extend(nested);
    ts
}

pub(crate) fn forward_fixed_fields(
    fields: &[MessageField],
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
) -> proc_macro2::TokenStream {
    let mut out = proc_macro2::TokenStream::new();
    for f in fields {
        out.extend(forward_one_field(
            f,
            conversions,
            domain_types,
            null_as_option,
            all_enums_as_option,
            true,
        ));
    }
    out
}

fn forward_entry_fields(
    fields: &[MessageField],
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
) -> proc_macro2::TokenStream {
    let mut out = proc_macro2::TokenStream::new();
    for f in fields {
        out.extend(forward_one_field(
            f,
            conversions,
            domain_types,
            null_as_option,
            all_enums_as_option,
            false,
        ));
    }
    out
}

fn field_wire_type(f: &MessageField) -> syn::Type {
    let span = proc_macro2::Span::call_site();
    match &f.field_type {
        FieldType::Primitive(prim, Some(len)) => {
            let r = syn::Ident::new(rust_type(*prim), span);
            let n = syn::LitInt::new(&len.to_string(), span);
            syn::parse_quote!([#r; #n])
        }
        FieldType::Primitive(prim, None) => {
            let r = syn::Ident::new(rust_type(*prim), span);
            syn::parse_quote!(#r)
        }
        FieldType::Composite { name, .. }
        | FieldType::Enum { name, .. }
        | FieldType::Set { name, .. } => {
            let t = syn::Ident::new(&to_pascal_case(name), span);
            syn::parse_quote!(#t)
        }
    }
}

fn forward_one_field(
    f: &MessageField,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
    reserve_decoder_names: bool,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let snake = to_snake_case(&f.name);
    let wire_name = field_has_conversion_free(f, conversions).then(|| format!("{snake}_wire"));
    let ident = if reserve_decoder_names {
        resolve_field_ident(&snake, &wire_name, DECODER_RESERVED)
    } else {
        let method_name = wire_name.as_deref().unwrap_or(&snake);
        syn::Ident::new(method_name, span)
    };

    let wire_ty = field_wire_type(f);
    let optional = f.presence != Presence::Constant
        && is_optional_domain_field(f, null_as_option, all_enums_as_option);

    let ret: syn::Type = if f.presence == Presence::Constant {
        match &f.field_type {
            FieldType::Primitive(prim, None)
                if *prim == PrimitiveType::Char
                    && f.constant_value.as_ref().is_some_and(|v| v.len() > 1) =>
            {
                syn::parse_quote!(&'static str)
            }
            _ => wire_ty.clone(),
        }
    } else if let FieldType::Composite { name, .. } = &f.field_type {
        let dec = syn::Ident::new(&format!("{}Decoder", to_pascal_case(name)), span);
        if optional {
            syn::parse_quote!(Option<#dec<'_>>)
        } else {
            syn::parse_quote!(#dec<'_>)
        }
    } else if optional {
        syn::parse_quote!(Option<#wire_ty>)
    } else {
        wire_ty.clone()
    };

    let mut ts = quote::quote! {
        #[inline]
        pub fn #ident(&self) -> #ret {
            self.inner.#ident()
        }
    };
    if f.presence != Presence::Constant {
        if let FieldType::Composite { name, .. } = &f.field_type {
            let value_ident = syn::Ident::new(&format!("{snake}_value"), span);
            let value_ty = syn::Ident::new(&to_pascal_case(name), span);
            if optional {
                ts.extend(quote::quote! {
                    #[inline]
                    pub fn #value_ident(&self) -> Option<#value_ty> {
                        self.inner.#value_ident()
                    }
                });
            } else {
                ts.extend(quote::quote! {
                    #[inline]
                    pub fn #value_ident(&self) -> #value_ty {
                        self.inner.#value_ident()
                    }
                });
            }
        }
    }

    if f.presence == Presence::Constant {
        return ts;
    }

    if let Some(dt) = find_domain_type(f, domain_types) {
        if let Ok(dt_ty) = syn::parse_str::<syn::Type>(dt) {
            let try_ident = syn::Ident::new(&format!("try_{snake}"), span);
            if optional {
                ts.extend(quote::quote! {
                    #[inline]
                    pub fn #try_ident(
                        &self,
                    ) -> Result<Option<#dt_ty>, <#dt_ty as TryFromSbe<#wire_ty>>::Error> {
                        self.inner.#try_ident()
                    }
                });
            } else {
                ts.extend(quote::quote! {
                    #[inline]
                    pub fn #try_ident(
                        &self,
                    ) -> Result<#dt_ty, <#dt_ty as TryFromSbe<#wire_ty>>::Error> {
                        self.inner.#try_ident()
                    }
                });
            }
        }
    } else if field_has_conversion_free(f, conversions) {
        let as_ident = syn::Ident::new(&format!("{snake}_as"), span);
        if optional {
            ts.extend(quote::quote! {
                #[inline]
                pub fn #as_ident<T: TryFromSbe<#wire_ty>>(
                    &self,
                ) -> Result<Option<T>, T::Error> {
                    self.inner.#as_ident()
                }
            });
        } else {
            ts.extend(quote::quote! {
                #[inline]
                pub fn #as_ident<T: TryFromSbe<#wire_ty>>(&self) -> Result<T, T::Error> {
                    self.inner.#as_ident()
                }
            });
        }
    }
    ts
}

//! Group encoder codegen.
//!
//! `generate_group_encoder` emits the repeating-group encoder and its entry
//! encoder (recursively, for nested groups) for one `MessageGroup`.

use crate::ir::{ByteOrder, Presence, PrimitiveType};
use crate::structured_ir::{
    FieldType, MessageGroup, SchemaElements, get_dim_block_layout, get_dim_num_layout,
    get_dimension_info, get_vardata_info, rust_type,
};

use super::conversion_helpers::field_has_conversion_free;
use super::field_type::field_type_ident;
use super::runtime::{to_pascal_case, to_snake_case};

pub(crate) fn generate_group_encoder(
    src: &mut String,
    g: &MessageGroup,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    scoped_name: &str,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
) {
    let name = scoped_name.to_string();
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };
    let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
    let (block_offset, block_size, block_prim) = get_dim_block_layout(elements, &g.dimension_type);
    let (_, _, num_prim) = get_dim_num_layout(elements, &g.dimension_type);
    let count_ty: syn::Type = syn::parse_str(rust_type(num_prim)).unwrap();

    let group_block_length = g.effective_block_length();

    assert!(
        dim_size <= 32,
        "group dimension header larger than stack pad: {dim_size}"
    );
    let mut dim_storage = [0u8; 32];
    let dim_tpl = &mut dim_storage[..dim_size];
    assert!(
        block_offset
            .checked_add(block_size)
            .is_some_and(|end| end <= dim_size),
        "group dimension blockLength is outside its composite"
    );
    match block_prim {
        PrimitiveType::UInt8 => {
            dim_tpl[block_offset] = u8::try_from(group_block_length)
                .expect("group blockLength exceeds uint8 dimension field");
        }
        PrimitiveType::UInt16 => {
            let value = u16::try_from(group_block_length)
                .expect("group blockLength exceeds uint16 dimension field");
            let bytes = match byte_order {
                ByteOrder::LittleEndian => value.to_le_bytes(),
                ByteOrder::BigEndian => value.to_be_bytes(),
            };
            dim_tpl[block_offset..block_offset + 2].copy_from_slice(&bytes);
        }
        PrimitiveType::UInt32 => {
            let value = u32::try_from(group_block_length)
                .expect("group blockLength exceeds uint32 dimension field");
            let bytes = match byte_order {
                ByteOrder::LittleEndian => value.to_le_bytes(),
                ByteOrder::BigEndian => value.to_be_bytes(),
            };
            dim_tpl[block_offset..block_offset + 4].copy_from_slice(&bytes);
        }
        _ => panic!("group dimension blockLength must use an unsigned integer type"),
    }

    let span = proc_macro2::Span::call_site();
    let group_enc_ident = syn::Ident::new(&format!("{}Encoder", name), span);
    let entry_enc_ident = syn::Ident::new(&format!("{}EntryEncoder", name), span);
    let block_len_lit = syn::LitInt::new(&group_block_length.to_string(), span);
    let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), span);
    let dim_bytes: Vec<syn::LitInt> = dim_tpl
        .iter()
        .map(|b| syn::LitInt::new(&b.to_string(), span))
        .collect();
    let to_endian = syn::Ident::new(&format!("to_{order_suffix}_bytes"), span);

    let mut null_stmts = proc_macro2::TokenStream::new();
    for f in &g.fields {
        if f.presence != Presence::Optional {
            continue;
        }
        let Some(null_val) = f.null_value else {
            continue;
        };
        let size = f.field_type.size();
        if size == 0 || size > 8 {
            continue;
        }
        // Exact width in schema endianness (HFT-002) — not a full u64 array.
        let null_bytes = super::nullification::null_sentinel_bytes(null_val, size, byte_order);
        let f_offset = syn::Index::from(f.offset);
        let size_lit = syn::LitInt::new(&size.to_string(), span);
        let lits: Vec<syn::LitInt> = null_bytes[..size]
            .iter()
            .map(|b| syn::LitInt::new(&b.to_string(), span))
            .collect();
        null_stmts.extend(quote::quote! {
            {
                let null_bytes: [u8; #size_lit] = [#(#lits),*];
                let offset = self.pos + #f_offset;
                self.buf[offset..offset + #size_lit].copy_from_slice(&null_bytes);
            }
        });
    }

    let mut add_body = quote::quote! {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: self.written as u32 + 1,
            }
            .into());
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len().saturating_sub(self.pos),
            }
            .into());
        }
    };
    if !null_stmts.is_empty() {
        add_body.extend(null_stmts);
    }
    add_body.extend(quote::quote! {
        // SAFETY: same borrow-split pattern as the group encoder method above.
        // The closure `f` only operates on __entry (which holds __buf), never
        // on `self`. The block scope drops __buf before `self.pos` is written.
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            // SAFETY: capacity check above proved pos+block_len ≤ buf.len().
            let mut __entry = #entry_enc_ident::wrap(__buf, self.pos);
            f(&mut __entry)?;
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    });

    let mut ts = proc_macro2::TokenStream::new();
    ts.extend(quote::quote! {
        #[must_use = "group encoder must call add() to write entries"]
        pub struct #group_enc_ident<'a> {
            buf: &'a mut [u8],
            pos: usize,
            count: #count_ty,
            written: #count_ty,
        }

        impl<'a> #group_enc_ident<'a> {
            pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;
            pub const GROUP_DIM_TEMPLATE: [u8; #dim_size_lit] = [#(#dim_bytes),*];
            const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == #dim_size_lit);

            #[inline]
            pub fn wrap(buf: &'a mut [u8], pos: usize, count: #count_ty) -> Self {
                Self { buf, pos, count, written: 0 }
            }

            /// Write one group entry. Closure may return `()` or `Result<(), E>`
            /// ([`sbe_rt::GroupEncodeResult`]) so `?` works without `try_add`.
            #[inline]
            #[must_use]
            pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #entry_enc_ident<'b>) -> sbe_rt::GroupResult,
            {
                #add_body
            }

            /// Manual entry creation: returns a borrowed entry encoder.
            /// The entry writes fixed fields directly into the group buffer.
            /// Drop the entry or let it go out of scope to commit it.
            /// The group position is pre-advanced, so fields are written
            /// to the correct offset.
            #[must_use]
            #[inline]
            pub fn start_entry(&mut self) -> Result<#entry_enc_ident<'_>, sbe_rt::EncodeError> {
                if self.written as u32 >= self.count as u32 {
                    return Err(sbe_rt::EncodeError::GroupFull {
                        declared: self.count as u32,
                        attempted: (self.written as u32) + 1,
                    });
                }
                let block_len = Self::ENTRY_BLOCK_LENGTH;
                if self
                    .pos
                    .checked_add(block_len)
                    .map(|end| end > self.buf.len())
                    .unwrap_or(true)
                {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        needed: block_len,
                        available: self.buf.len().saturating_sub(self.pos),
                    });
                }
                let entry_pos = self.pos;
                self.pos += block_len;
                self.written += 1;
                // SAFETY: capacity check above proved entry_pos..entry_pos+block_len
                // is in-bounds; entry wrap only writes fixed fields in that region.
                Ok(unsafe {
                    #entry_enc_ident::wrap(&mut self.buf[entry_pos..self.pos], 0)
                })
            }
        }
    });

    // written() accessor — used by _unknown_size to back-patch the count.
    ts.extend(quote::quote! {
        impl<'a> #group_enc_ident<'a> {
            /// Number of entries written so far (for `_unknown_size` back-patch).
            #[inline]
            pub fn written(&self) -> #count_ty {
                self.written
            }
        }
    });

    // add_struct: when the entry has no nested groups or var-data, generate
    // a named value struct so callers can write whole entries in one call.
    if g.groups.is_empty() && g.var_data.is_empty() {
        let entry_struct_ident = syn::Ident::new(&format!("{}Entry", name), span);
        let mut struct_fields = proc_macro2::TokenStream::new();
        let mut struct_write = proc_macro2::TokenStream::new();
        let mut bulk_struct_write = proc_macro2::TokenStream::new();
        for f in &g.fields {
            if f.presence == Presence::Constant {
                continue;
            }
            let f_name = syn::Ident::new(&to_snake_case(&f.name), span);
            let f_ty = field_type_ident(&f.field_type, span);
            let f_offset = syn::Index::from(f.offset);
            let f_size = syn::LitInt::new(&f.field_type.size().to_string(), span);
            struct_fields.extend(quote::quote! { pub #f_name: #f_ty, });
            match &f.field_type {
                FieldType::Composite { .. } => {
                    struct_write.extend(quote::quote! {
                        self.buf[pos + #f_offset..pos + #f_offset + #f_size].copy_from_slice(&entry.#f_name.0);
                    });
                    bulk_struct_write.extend(quote::quote! {
                        slot[#f_offset..#f_offset + #f_size].copy_from_slice(&entry.#f_name.0);
                    });
                }
                FieldType::Enum { encoding_type, .. } | FieldType::Set { encoding_type, .. } => {
                    let r_ty = syn::Ident::new(&rust_type(*encoding_type), span);
                    struct_write.extend(quote::quote! {
                        self.buf[pos + #f_offset..pos + #f_offset + #f_size]
                            .copy_from_slice(&(#r_ty::from(entry.#f_name)).#to_endian());
                    });
                    bulk_struct_write.extend(quote::quote! {
                        slot[#f_offset..#f_offset + #f_size]
                            .copy_from_slice(&(#r_ty::from(entry.#f_name)).#to_endian());
                    });
                }
                FieldType::Primitive(pt, Some(len)) => {
                    let len_lit = syn::LitInt::new(&len.to_string(), span);
                    let prim_size_lit = syn::LitInt::new(&pt.size().to_string(), span);
                    struct_write.extend(quote::quote! {
                        let mut idx = 0usize;
                        while idx < #len_lit {
                            let offset = pos + #f_offset + idx * #prim_size_lit;
                            self.buf[offset..offset + #prim_size_lit]
                                .copy_from_slice(&entry.#f_name[idx].#to_endian());
                            idx += 1;
                        }
                    });
                    bulk_struct_write.extend(quote::quote! {
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
                    struct_write.extend(quote::quote! {
                        self.buf[pos + #f_offset..pos + #f_offset + #f_size]
                            .copy_from_slice(&entry.#f_name.#to_endian());
                    });
                    bulk_struct_write.extend(quote::quote! {
                        slot[#f_offset..#f_offset + #f_size]
                            .copy_from_slice(&entry.#f_name.#to_endian());
                    });
                }
            }
        }
        ts.extend(quote::quote! {
            /// Value struct for this group's entries.
            #[derive(Debug, Clone, PartialEq)]
            pub struct #entry_struct_ident {
                #struct_fields
            }
        });
        ts.extend(quote::quote! {
            impl<'a> #group_enc_ident<'a> {
                /// Write one entry from a struct. Faster than [`Self::add`] when
                /// the entry has no nested groups or var-data.
                #[inline]
                pub fn add_struct(&mut self, entry: &#entry_struct_ident) -> Result<(), sbe_rt::EncodeError> {
                    if self.written as u32 >= self.count as u32 {
                        return Err(sbe_rt::EncodeError::GroupFull {
                            declared: self.count as u32,
                            attempted: (self.written as u32) + 1,
                        });
                    }
                    let block_len = Self::ENTRY_BLOCK_LENGTH;
                    if self.pos + block_len > self.buf.len() {
                        return Err(sbe_rt::EncodeError::BufferTooShort {
                            needed: block_len,
                            available: self.buf.len().saturating_sub(self.pos),
                        });
                    }
                    let pos = self.pos;
                    self.pos += block_len;
                    self.written += 1;
                    #struct_write
                    Ok(())
                }

                #[inline]
                fn bulk_add_with<T, F>(
                    &mut self,
                    entries: &[T],
                    mut write_entry: F,
                ) -> Result<(), sbe_rt::EncodeError>
                where
                    F: FnMut(&T, &mut [u8]) -> Result<(), sbe_rt::EncodeError>,
                {
                    let count = entries.len();
                    if count == 0 {
                        return Ok(());
                    }
                    let attempted = (self.written as usize)
                        .checked_add(count)
                        .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                    if attempted > self.count as usize {
                        return Err(sbe_rt::EncodeError::GroupFull {
                            declared: self.count as u32,
                            attempted: attempted.min(u32::MAX as usize) as u32,
                        });
                    }
                    let block_len = Self::ENTRY_BLOCK_LENGTH;
                    if block_len == 0 {
                        self.written = attempted as #count_ty;
                        return Ok(());
                    }
                    let needed = count
                        .checked_mul(block_len)
                        .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                    let end = self
                        .pos
                        .checked_add(needed)
                        .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                    if end > self.buf.len() {
                        return Err(sbe_rt::EncodeError::BufferTooShort {
                            needed,
                            available: self.buf.len().saturating_sub(self.pos),
                        });
                    }
                    {
                        let region = &mut self.buf[self.pos..end];
                        for (entry, slot) in entries
                            .iter()
                            .zip(region.chunks_exact_mut(block_len))
                        {
                            write_entry(entry, slot)?;
                        }
                    }
                    self.pos = end;
                    self.written = attempted as #count_ty;
                    Ok(())
                }

                /// Encode a slice of fixed-size entries after validating the
                /// complete destination region once.
                #[inline]
                pub fn bulk_add(&mut self, entries: &[#entry_struct_ident]) -> Result<(), sbe_rt::EncodeError> {
                    self.bulk_add_with(entries, |entry, slot| {
                        #bulk_struct_write
                        Ok(())
                    })
                }
            }
        });
    }

    // Entry encoder struct + all methods in a single impl block
    let mut entry_methods = proc_macro2::TokenStream::new();

    entry_methods.extend(quote::quote! {
        pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

        /// Private entry wrap after the group encoder proved the fixed block
        /// region fits (via `add` / `start_entry` capacity checks).
        ///
        /// # Safety
        /// `pos + ENTRY_BLOCK_LENGTH` must not overflow and must be ≤ `buf.len()`
        /// for the lifetime of the returned encoder.
        #[inline]
        pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
            Self {
                buf,
                entry_start: pos,
                pos: pos + Self::ENTRY_BLOCK_LENGTH,
            }
        }
    });

    for f in &g.fields {
        let f_snake = to_snake_case(&f.name);
        // Raw entry setters become *_wire when a conversion is configured.
        let wire_name =
            field_has_conversion_free(f, conversions).then(|| format!("{f_snake}_wire"));
        let setter_name = wire_name.as_deref().unwrap_or(&f_snake);
        let f_ident = syn::Ident::new(&setter_name, span);
        let f_offset = syn::Index::from(f.offset);

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_ty = syn::Ident::new(&rust_type(*prim), span);
                let prim_size = prim.size();
                if f.presence == Presence::Constant {
                    continue;
                } else if let Some(len) = length {
                    let len_lit = syn::LitInt::new(&len.to_string(), span);
                    let sz = syn::LitInt::new(&prim_size.to_string(), span);
                    entry_methods.extend(quote::quote! {
                        #[inline]
                        pub fn #f_ident(&mut self, val: [#r_ty; #len_lit]) -> &mut Self {
                            let offset = self.entry_start + #f_offset;
                            let mut idx = 0;
                            while idx < #len_lit {
                                self.buf[offset + idx * #sz..][..#sz].copy_from_slice(&val[idx].#to_endian());
                                idx += 1;
                            }
                            self
                        }
                    });
                } else if prim_size == 1 {
                    entry_methods.extend(quote::quote! {
                        #[inline]
                        pub fn #f_ident(&mut self, val: #r_ty) -> &mut Self {
                            self.buf[self.entry_start + #f_offset] = val as u8;
                            self
                        }
                    });
                } else {
                    let sz = syn::LitInt::new(&prim_size.to_string(), span);
                    entry_methods.extend(quote::quote! {
                        #[inline]
                        pub fn #f_ident(&mut self, val: #r_ty) -> &mut Self {
                            let offset = self.entry_start + #f_offset;
                            self.buf[offset..offset + #sz].copy_from_slice(&val.#to_endian());
                            self
                        }
                    });
                }
            }
            FieldType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target = syn::Ident::new(&to_pascal_case(comp_name), span);
                let sz = syn::LitInt::new(&comp_size.to_string(), span);
                entry_methods.extend(quote::quote! {
                    #[inline]
                    pub fn #f_ident(&mut self, val: #target) -> &mut Self {
                        let offset = self.entry_start + #f_offset;
                        self.buf[offset..offset + #sz].copy_from_slice(&val.0);
                        self
                    }
                });
            }
            FieldType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                let target = syn::Ident::new(&to_pascal_case(enum_name), span);
                let r_ty = syn::Ident::new(&rust_type(*encoding_type), span);
                let sz = syn::LitInt::new(&encoding_type.size().to_string(), span);
                entry_methods.extend(quote::quote! {
                    #[inline]
                    pub fn #f_ident(&mut self, val: #target) -> &mut Self {
                        let offset = self.entry_start + #f_offset;
                        self.buf[offset..offset + #sz].copy_from_slice(&(val as #r_ty).#to_endian());
                        self
                    }
                });
                if crate::structured_ir::is_bool_enum(elements, enum_name) {
                    let f_name_bool = syn::Ident::new(&format!("{}_bool", f_snake), span);
                    entry_methods.extend(quote::quote! {
                        #[inline]
                        pub fn #f_name_bool(&mut self, val: bool) -> &mut Self {
                            self.buf[self.entry_start + #f_offset] = val as u8;
                            self
                        }
                    });
                }
            }
            FieldType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target = syn::Ident::new(&to_pascal_case(set_name), span);
                let sz = syn::LitInt::new(&encoding_type.size().to_string(), span);
                entry_methods.extend(quote::quote! {
                    #[inline]
                    pub fn #f_ident(&mut self, val: #target) -> &mut Self {
                        let offset = self.entry_start + #f_offset;
                        self.buf[offset..offset + #sz].copy_from_slice(&val.0.#to_endian());
                        self
                    }
                });
            }
        }
    }

    // Nested group setters — scope under parent group name
    for ng in &g.groups {
        let ng_pascal_scoped = format!("{}{}", name, to_pascal_case(&ng.name));
        let ng_snake = syn::Ident::new(&to_snake_case(&ng.name), span);
        let ng_snake_unknown =
            syn::Ident::new(&format!("{}_unknown_size", to_snake_case(&ng.name)), span);
        let ng_enc = syn::Ident::new(&format!("{ng_pascal_scoped}Encoder"), span);
        let (_dim_name, ng_dim_size, _, _) = get_dimension_info(elements, &ng.dimension_type);
        let (num_off, num_sz, ng_num_prim) = get_dim_num_layout(elements, &ng.dimension_type);
        let ng_dim = syn::LitInt::new(&ng_dim_size.to_string(), span);
        let num_off_idx = syn::Index::from(num_off);
        let num_sz_lit = syn::LitInt::new(&num_sz.to_string(), span);
        let ng_count_ty: syn::Type = syn::parse_str(rust_type(ng_num_prim)).unwrap();

        entry_methods.extend(quote::quote! {
            #[inline]
            #[must_use]
            pub fn #ng_snake<F>(&mut self, count: #ng_count_ty, f: F) -> Result<&mut Self, sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #ng_enc<'a>) -> sbe_rt::GroupResult,
            {
                if self.pos + #ng_dim > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        needed: #ng_dim,
                        available: self.buf.len().saturating_sub(self.pos),
                    }
                    .into());
                }
                self.buf[self.pos..self.pos + #ng_dim].copy_from_slice(&#ng_enc::GROUP_DIM_TEMPLATE);
                self.buf[self.pos + #num_off_idx..self.pos + #num_off_idx + #num_sz_lit].copy_from_slice(&count.#to_endian());
                // SAFETY: the closure `f` only operates on the group encoder (which
                // holds __buf), never on `self`. The block scope ensures __buf is
                // dropped before `self.pos` is written. No aliasing occurs because
                // `self.buf` is not accessed through `self` while __buf is live.
                // This is the standard borrow-split pattern (same as split_at_mut
                // internals) — the raw pointer cast is a borrow-checker workaround,
                // not an actual aliasing violation.
                let __pos;
                {
                    let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                    let mut group = #ng_enc::wrap(__buf, self.pos + #ng_dim, count);
                    f(&mut group)?;
                    let written = group.written();
                    if written != count {
                        return Err(sbe_rt::EncodeError::GroupCountMismatch {
                            declared: count as u32,
                            actual: written as u32,
                        });
                    }
                    __pos = group.pos;
                }
                self.pos = __pos;
                Ok(self)
            }

            /// Nested-group `_unknown_size` variant — back-patches count.
            #[inline]
            pub fn #ng_snake_unknown<F>(&mut self, f: F) -> Result<&mut Self, sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #ng_enc<'a>) -> sbe_rt::GroupResult,
            {
                if self.pos + #ng_dim > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        needed: #ng_dim,
                        available: self.buf.len().saturating_sub(self.pos),
                    }.into());
                }
                self.buf[self.pos..self.pos + #ng_dim].copy_from_slice(&#ng_enc::GROUP_DIM_TEMPLATE);
                let count_offset = self.pos + #num_off_idx;
                self.buf[count_offset..count_offset + #num_sz_lit].fill(0);
                let __pos;
                {
                    let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                    let mut group = #ng_enc::wrap(__buf, self.pos + #ng_dim, #ng_count_ty::MAX);
                    f(&mut group)?;
                    let actual: #ng_count_ty = group.written();
                    __pos = group.pos;
                    group.buf[count_offset..count_offset + #num_sz_lit]
                        .copy_from_slice(&actual.#to_endian());
                }
                self.pos = __pos;
                Ok(self)
            }
        });
    }

    // VarData setters
    for vd in &g.var_data {
        let vd_snake = syn::Ident::new(&to_snake_case(&vd.name), span);
        let (_, prefix_sz, _, len_type) = get_vardata_info(elements, &vd.type_name);
        let pfx = syn::LitInt::new(&prefix_sz.to_string(), span);
        let len_ty = syn::Ident::new(&rust_type(len_type), span);
        let vd_name_lit = syn::LitStr::new(&vd.name, span);
        let schema_max_check = if let Some(max) = vd.max_length {
            let max_lit = syn::LitInt::new(&max.to_string(), span);
            quote::quote! {
                if data.len() > #max_lit {
                    return Err(sbe_rt::EncodeError::VarDataTooLong {
                        field: #vd_name_lit,
                        max_length: #max_lit,
                        actual: data.len(),
                    });
                }
            }
        } else {
            quote::quote! {}
        };

        entry_methods.extend(quote::quote! {
            #[inline]
            #[must_use]
            pub fn #vd_snake(&mut self, data: &[u8]) -> Result<&mut Self, sbe_rt::EncodeError> {
                #schema_max_check
                let needed = #pfx + data.len();
                if self.pos + needed > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort { needed, available: self.buf.len().saturating_sub(self.pos) });
                }
                let wire_length = #len_ty::try_from(data.len()).map_err(|_| {
                    sbe_rt::EncodeError::VarDataTooLong {
                        field: #vd_name_lit,
                        max_length: #len_ty::MAX as usize,
                        actual: data.len(),
                    }
                })?;
                let len_bytes = wire_length.#to_endian();
                self.buf[self.pos..self.pos + #pfx].copy_from_slice(&len_bytes);
                let start = self.pos + #pfx;
                self.buf[start..start + data.len()].copy_from_slice(data);
                self.pos = start + data.len();
                Ok(self)
            }
        });
    }

    ts.extend(quote::quote! {
        #[must_use = "entry encoder fields must be set before the next entry"]
        pub struct #entry_enc_ident<'a> {
            buf: &'a mut [u8],
            entry_start: usize,
            pos: usize,
        }

        impl<'a> #entry_enc_ident<'a> {
            #entry_methods
        }
    });

    src.push_str(&ts.to_string());
    src.push('\n');
    src.push('\n');

    // Recursively generate nested Repeating Groups encoders
    for ng in &g.groups {
        let nested_name = format!("{}{}", name, to_pascal_case(&ng.name));
        generate_group_encoder(
            src,
            ng,
            elements,
            byte_order,
            &nested_name,
            &conversions,
            domain_types,
        );
    }
}

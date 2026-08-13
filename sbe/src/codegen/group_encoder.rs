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
    let entry_complete_ident = syn::Ident::new(&format!("{}EntryComplete", name), span);
    // Built with `format!` rather than `///`: a doc comment inside `quote!` is
    // already a string literal by the time interpolation runs, so `#ident`
    // inside one would be emitted verbatim instead of the type's real name.
    let add_checked_doc = format!(
        "Write one group entry, proving completeness in the type system.\n\n\
         The closure takes the entry encoder **by value** and must return \
         `{entry_complete_ident}` — reachable only by writing every required \
         tail in wire order. An entry that skips, reorders, or repeats a tail \
         cannot produce that type, so it fails to compile rather than \
         producing a short entry at run time.\n\n\
         [`Self::add`] stays available for entries whose tails are already \
         checked elsewhere.",
    );
    // `add_checked` lives on the *group* encoder, so `Self::` would not resolve
    // from the entry encoder this doc is attached to.
    let complete_doc = format!(
        "Finish a flat entry, producing the `{entry_complete_ident}` that \
         [`{group_enc_ident}::add_checked`] requires.\n\n\
         Only for entries with no required tails — an entry that has them \
         reaches this type through its last tail method instead."
    );
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
        // Exact width in schema endianness — not a full u64 array.
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
                let offset = self.offset + #f_offset;
                self.buf[offset..offset + #size_lit].copy_from_slice(&null_bytes);
            }
        });
    }

    let is_dynamic = !g.has_fixed_stride();

    // Capacity / null-init prelude shared by both add forms.
    let mut capacity_body = quote::quote! {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: self.written as u32 + 1,
            }
            .into());
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.offset + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: "group entry",
                needed: block_len,
                available: self.buf.len().saturating_sub(self.offset),
            }
            .into());
        }
    };
    if !null_stmts.is_empty() {
        capacity_body.extend(null_stmts.clone());
    }

    // Dynamic (T-19): add commits only from EntryComplete.
    // Fixed-stride: add still accepts &mut + GroupResult; add_checked needs Complete.
    let add_fn: proc_macro2::TokenStream = if is_dynamic {
        let mut body = capacity_body.clone();
        body.extend(quote::quote! {
            // SAFETY: borrow-split — closure only uses __entry, never self.buf.
            {
                let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                let __entry = unsafe { #entry_enc_ident::wrap(__buf, self.offset) };
                let __complete = f(__entry)?;
                self.offset = __complete.into_cursor();
            }
            self.written += 1;
            Ok(())
        });
        quote::quote! {
            /// Write one group entry, proving required tails are complete.
            ///
            /// The closure takes the entry encoder **by value** and must return
            /// the entry-complete proof — reachable only by writing every
            /// required nested group and var-data field in wire order.
            #[inline]
            #[must_use]
            pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
            where
                F: FnOnce(#entry_enc_ident<'b>) -> Result<#entry_complete_ident<'b>, sbe_rt::EncodeError>,
            {
                #body
            }
        }
    } else {
        let mut body = capacity_body.clone();
        body.extend(quote::quote! {
            {
                let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                let mut __entry = unsafe { #entry_enc_ident::wrap(__buf, self.offset) };
                f(&mut __entry)?;
                self.offset = __entry.offset;
            }
            self.written += 1;
            Ok(())
        });
        quote::quote! {
            /// Write one group entry. The closure may return `()` or
            /// `Result<(), sbe_rt::EncodeError>` (both satisfy
            /// [`sbe_rt::GroupResult`]), so `?` works without a `try_add`.
            #[inline]
            #[must_use]
            pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #entry_enc_ident<'b>) -> sbe_rt::GroupResult,
            {
                #body
            }
        }
    };

    let fixed_stride_methods: proc_macro2::TokenStream = if g.has_fixed_stride() {
        let mut checked_body = capacity_body.clone();
        checked_body.extend(quote::quote! {
            {
                let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                let __entry = unsafe { #entry_enc_ident::wrap(__buf, self.offset) };
                let __complete = f(__entry)?;
                self.offset = __complete.into_cursor();
            }
            self.written += 1;
            Ok(())
        });
        quote::quote! {
            #[doc = #add_checked_doc]
            #[inline]
            pub fn add_checked<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
            where
                F: FnOnce(#entry_enc_ident<'b>) -> Result<#entry_complete_ident<'b>, sbe_rt::EncodeError>,
            {
                #checked_body
            }

            /// Manual entry creation: returns a borrowed entry encoder.
            /// The entry writes fixed fields directly into the group buffer.
            /// Drop the entry or let it go out of scope to commit it.
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
                    .offset
                    .checked_add(block_len)
                    .map(|end| end > self.buf.len())
                    .unwrap_or(true)
                {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        field: "group entry",
                        needed: block_len,
                        available: self.buf.len().saturating_sub(self.offset),
                    });
                }
                let entry_offset = self.offset;
                self.offset += block_len;
                self.written += 1;
                let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                Ok(unsafe { #entry_enc_ident::wrap(__buf, entry_offset) })
            }
        }
    } else {
        quote::quote! {}
    };

    let mut ts = proc_macro2::TokenStream::new();
    ts.extend(quote::quote! {
        #[doc = concat!("Encoder for the `", stringify!(#group_enc_ident), "` group — call `add()` to write entries.")]
        #[must_use = "group encoder must call add() to write entries"]
        pub struct #group_enc_ident<'a> {
            buf: &'a mut [u8],
            offset: usize,
            count: #count_ty,
            written: #count_ty,
        }

        impl<'a> #group_enc_ident<'a> {
            pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;
            pub const GROUP_DIM_TEMPLATE: [u8; #dim_size_lit] = [#(#dim_bytes),*];
            const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == #dim_size_lit);

            #[inline]
            pub fn wrap(buf: &'a mut [u8], offset: usize, count: #count_ty) -> Self {
                Self { buf, offset, count, written: 0 }
            }

            #add_fn
            #fixed_stride_methods
            /// Number of entries written so far (for `_unknown_size` back-patch).
            #[inline]
            pub fn written(&self) -> #count_ty {
                self.written
            }
        }
    });

    if g.has_fixed_stride() {
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
                        self.buf[offset + #f_offset..offset + #f_offset + #f_size].copy_from_slice(&entry.#f_name.0);
                    });
                    bulk_struct_write.extend(quote::quote! {
                        slot[#f_offset..#f_offset + #f_size].copy_from_slice(&entry.#f_name.0);
                    });
                }
                FieldType::Enum { encoding_type, .. } | FieldType::Set { encoding_type, .. } => {
                    let r_ty = syn::Ident::new(&rust_type(*encoding_type), span);
                    struct_write.extend(quote::quote! {
                        self.buf[offset + #f_offset..offset + #f_offset + #f_size]
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
                            let offset = offset + #f_offset + idx * #prim_size_lit;
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
                        self.buf[offset + #f_offset..offset + #f_offset + #f_size]
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
                    if self.offset + block_len > self.buf.len() {
                        return Err(sbe_rt::EncodeError::BufferTooShort {
                            field: "group entry",
                            needed: block_len,
                            available: self.buf.len().saturating_sub(self.offset),
                        });
                    }
                    let offset = self.offset;
                    self.offset += block_len;
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
                        .offset
                        .checked_add(needed)
                        .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                    if end > self.buf.len() {
                        return Err(sbe_rt::EncodeError::BufferTooShort {
                            field: "group entry",
                            needed,
                            available: self.buf.len().saturating_sub(self.offset),
                        });
                    }
                    {
                        let region = &mut self.buf[self.offset..end];
                        for (entry, slot) in entries
                            .iter()
                            .zip(region.chunks_exact_mut(block_len))
                        {
                            write_entry(entry, slot)?;
                        }
                    }
                    self.offset = end;
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

    // Tail order: nested groups then var-data (wire order).
    #[derive(Clone, Copy)]
    enum TailRef {
        Nested(usize),
        Var(usize),
    }
    let mut tails: Vec<TailRef> = Vec::new();
    for i in 0..g.groups.len() {
        tails.push(TailRef::Nested(i));
    }
    for i in 0..g.var_data.len() {
        tails.push(TailRef::Var(i));
    }

    // stage_idents: [EntryEncoder, AfterT0, ..., EntryComplete] for dynamic
    let mut stage_idents: Vec<syn::Ident> = vec![entry_enc_ident.clone()];
    if is_dynamic {
        for (i, t) in tails.iter().enumerate() {
            if i + 1 == tails.len() {
                stage_idents.push(entry_complete_ident.clone());
            } else {
                let tail_pascal = match t {
                    TailRef::Nested(j) => to_pascal_case(&g.groups[*j].name),
                    TailRef::Var(j) => to_pascal_case(&g.var_data[*j].name),
                };
                stage_idents.push(syn::Ident::new(
                    &format!("{}After{}", name, tail_pascal),
                    span,
                ));
            }
        }
    }

    // Fixed-field methods on EntryEncoder.
    let mut entry_methods = proc_macro2::TokenStream::new();
    entry_methods.extend(quote::quote! {
        pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

        /// Private entry wrap after the group encoder proved the fixed block
        /// region fits (via `add` / `start_entry` capacity checks).
        ///
        /// # Safety
        /// `offset + ENTRY_BLOCK_LENGTH` must not overflow and must be ≤ `buf.len()`
        /// for the lifetime of the returned encoder.
        #[inline]
        unsafe fn wrap(buf: &'a mut [u8], offset: usize) -> Self {
            Self {
                buf,
                entry_start: offset,
                offset: offset + Self::ENTRY_BLOCK_LENGTH,
            }
        }
    });

    if g.has_fixed_stride() {
        entry_methods.extend(quote::quote! {
            #[doc = #complete_doc]
            #[inline]
            pub fn complete(self) -> #entry_complete_ident<'a> {
                #entry_complete_ident {
                    buf: self.buf,
                    entry_start: self.entry_start,
                    offset: self.offset,
                }
            }
        });
    }

    for f in &g.fields {
        let f_snake = to_snake_case(&f.name);
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
                                self.buf[offset + idx * #sz..][..#sz]
                                    .copy_from_slice(&val[idx].#to_endian());
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
                        self.buf[offset..offset + #sz]
                            .copy_from_slice(&(val as #r_ty).#to_endian());
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
                        self.buf[offset..offset + #sz]
                            .copy_from_slice(&val.0.#to_endian());
                        self
                    }
                });
            }
        }
    }

    // Emit one consuming nested-group method pair for a stage → next_stage.
    let emit_nested = |ng: &MessageGroup, next_stage: &syn::Ident| -> proc_macro2::TokenStream {
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
        quote::quote! {
            #[inline]
            #[must_use]
            pub fn #ng_snake<F>(mut self, count: #ng_count_ty, f: F) -> Result<#next_stage<'a>, sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #ng_enc<'a>) -> sbe_rt::GroupResult,
            {
                if self.offset + #ng_dim > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        field: "group entry",
                        needed: #ng_dim,
                        available: self.buf.len().saturating_sub(self.offset),
                    }
                    .into());
                }
                self.buf[self.offset..self.offset + #ng_dim]
                    .copy_from_slice(&#ng_enc::GROUP_DIM_TEMPLATE);
                self.buf[self.offset + #num_off_idx..self.offset + #num_off_idx + #num_sz_lit]
                    .copy_from_slice(&count.#to_endian());
                let __offset;
                {
                    let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                    let mut group = #ng_enc::wrap(__buf, self.offset + #ng_dim, count);
                    f(&mut group)?;
                    let written = group.written();
                    if written != count {
                        return Err(sbe_rt::EncodeError::GroupCountMismatch {
                            declared: count as u32,
                            actual: written as u32,
                        });
                    }
                    __offset = group.offset;
                }
                Ok(#next_stage {
                    buf: self.buf,
                    entry_start: self.entry_start,
                    offset: __offset,
                })
            }

            /// Nested-group `_unknown_size` variant — back-patches count.
            #[inline]
            pub fn #ng_snake_unknown<F>(mut self, f: F) -> Result<#next_stage<'a>, sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #ng_enc<'a>) -> sbe_rt::GroupResult,
            {
                if self.offset + #ng_dim > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        field: "group entry",
                        needed: #ng_dim,
                        available: self.buf.len().saturating_sub(self.offset),
                    }
                    .into());
                }
                self.buf[self.offset..self.offset + #ng_dim]
                    .copy_from_slice(&#ng_enc::GROUP_DIM_TEMPLATE);
                let count_offset = self.offset + #num_off_idx;
                self.buf[count_offset..count_offset + #num_sz_lit].fill(0);
                let __offset;
                {
                    let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                    let mut group = #ng_enc::wrap(__buf, self.offset + #ng_dim, #ng_count_ty::MAX);
                    f(&mut group)?;
                    let actual: #ng_count_ty = group.written();
                    __offset = group.offset;
                    group.buf[count_offset..count_offset + #num_sz_lit]
                        .copy_from_slice(&actual.#to_endian());
                }
                Ok(#next_stage {
                    buf: self.buf,
                    entry_start: self.entry_start,
                    offset: __offset,
                })
            }
        }
    };

    let emit_var = |vd: &crate::structured_ir::MessageVarData,
                    next_stage: &syn::Ident|
     -> proc_macro2::TokenStream {
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
        quote::quote! {
            #[inline]
            #[must_use]
            pub fn #vd_snake(mut self, data: &[u8]) -> Result<#next_stage<'a>, sbe_rt::EncodeError> {
                #schema_max_check
                let needed = #pfx + data.len();
                if self.offset + needed > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        field: "group entry",
                        needed,
                        available: self.buf.len().saturating_sub(self.offset),
                    });
                }
                let wire_length = #len_ty::try_from(data.len()).map_err(|_| {
                    sbe_rt::EncodeError::VarDataTooLong {
                        field: #vd_name_lit,
                        max_length: #len_ty::MAX as usize,
                        actual: data.len(),
                    }
                })?;
                let len_bytes = wire_length.#to_endian();
                self.buf[self.offset..self.offset + #pfx].copy_from_slice(&len_bytes);
                let start = self.offset + #pfx;
                self.buf[start..start + data.len()].copy_from_slice(data);
                self.offset = start + data.len();
                Ok(#next_stage {
                    buf: self.buf,
                    entry_start: self.entry_start,
                    offset: self.offset,
                })
            }
        }
    };

    // Attach first tail to EntryEncoder when dynamic.
    if is_dynamic {
        if let Some(first) = tails.first() {
            let next = &stage_idents[1];
            match *first {
                TailRef::Nested(j) => {
                    entry_methods.extend(emit_nested(&g.groups[j], next));
                }
                TailRef::Var(j) => {
                    entry_methods.extend(emit_var(&g.var_data[j], next));
                }
            }
        }
    }

    // EntryComplete always generated (fixed-stride complete() + dynamic tails).
    ts.extend(quote::quote! {
        #[doc = concat!("Proven-complete entry for the `", stringify!(#entry_complete_ident), "` group.")]
        pub struct #entry_complete_ident<'a> {
            buf: &'a mut [u8],
            entry_start: usize,
            offset: usize,
        }
        impl<'a> #entry_complete_ident<'a> {
            pub(crate) fn into_cursor(self) -> usize {
                self.offset
            }
        }
    });

    let entry_complete_note: &str = if is_dynamic {
        " — write required tails in wire order to reach EntryComplete"
    } else {
        " — set fields then call `complete()`"
    };

    ts.extend(quote::quote! {
        #[doc = concat!("Entry encoder for the `", stringify!(#entry_enc_ident), "` group", #entry_complete_note, ".")]
        #[must_use = "entry encoder fields must be set before the next entry"]
        pub struct #entry_enc_ident<'a> {
            buf: &'a mut [u8],
            entry_start: usize,
            offset: usize,
        }

        impl<'a> #entry_enc_ident<'a> {
            #entry_methods
        }
    });

    // Intermediate stages: stage_idents[k] for k in 1..tails.len() has method for tails[k].
    if is_dynamic {
        for k in 1..tails.len() {
            let cur = &stage_idents[k];
            let next = &stage_idents[k + 1];
            let methods = match tails[k] {
                TailRef::Nested(j) => emit_nested(&g.groups[j], next),
                TailRef::Var(j) => emit_var(&g.var_data[j], next),
            };
            ts.extend(quote::quote! {
                /// Intermediate entry stage after a required tail — continue in wire order.
                #[must_use = "entry stage must continue writing required tails"]
                pub struct #cur<'a> {
                    buf: &'a mut [u8],
                    entry_start: usize,
                    offset: usize,
                }
                impl<'a> #cur<'a> {
                    #methods
                }
            });
        }
    }

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

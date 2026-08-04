//! Message/entry consuming tail-stage codegen (DECISIONS.md §3).
//!
//! `generate_owner_consuming_stages` emits the type-state tail stages that own
//! sequential access to an owner's (message or entry) tail groups + var-data.
//! `generate_decoder_consuming_stages` / `generate_entry_consuming_stages`
//! resolve message- and entry-level tail components and delegate to it.

use crate::ir::ByteOrder;
use crate::structured_ir::{
    MessageGroup, MessageStructure, OwnerTailGroup, OwnerTailVarData, SchemaElements,
    decoder_stage_after_ident, get_vardata_info, rust_type,
};

use super::runtime::{to_pascal_case, to_snake_case};

pub(crate) fn generate_owner_consuming_stages(
    initial_ident: syn::Ident,
    stage_prefix: &str,
    header_size: usize,
    byte_order: ByteOrder,
    groups: &[OwnerTailGroup],
    vardata: &[OwnerTailVarData],
    enable_dispatch: bool,
) -> proc_macro2::TokenStream {
    let total_tail = groups.len() + vardata.len();
    if total_tail == 0 {
        return proc_macro2::TokenStream::new();
    }
    let span = proc_macro2::Span::call_site();
    let header_size_lit = syn::LitInt::new(&header_size.to_string(), span);

    let field_pascals: Vec<String> = groups
        .iter()
        .map(|g| g.field_pascal.clone())
        .chain(vardata.iter().map(|v| v.field_pascal.clone()))
        .collect();

    let stage_after_ident =
        |i: usize| decoder_stage_after_ident(stage_prefix, &field_pascals[i], i, total_tail, span);

    let mut ts = proc_macro2::TokenStream::new();

    // 1. Stage struct definitions (After + Complete). Identical 5-field layout,
    //    non-Copy: a stage carries the tail cursor, so consuming it prevents reuse.
    for i in 0..total_tail {
        let stage = stage_after_ident(i);
        ts.extend(quote::quote! {
            pub struct #stage<'a> {
                pub(crate) buf: &'a [u8],
                pub(crate) pos: usize,
                pub(crate) tail_start: usize,
                pub(crate) acting_version: u16,
                pub(crate) acting_block_length: usize,
            }
        });
    }

    // acting_version() / acting_block_length() on every stage (DECISIONS.md §3).
    for i in 0..total_tail {
        let stage = stage_after_ident(i);
        ts.extend(quote::quote! {
            impl<'a> #stage<'a> {
                #[inline]
                pub const fn acting_version(&self) -> u16 { self.acting_version }
                #[inline]
                pub const fn acting_block_length(&self) -> usize { self.acting_block_length }
            }
        });
    }

    let start_expr = |i: usize| -> syn::Expr {
        if i == 0 {
            syn::parse_str("self.pos + self.acting_block_length").unwrap()
        } else {
            syn::parse_str("self.tail_start").unwrap()
        }
    };

    // 2a. Group into_<g>() on the stage that precedes each group.
    for (gi, tg) in groups.iter().enumerate() {
        let i = gi;
        let current_stage = if i == 0 {
            initial_ident.clone()
        } else {
            stage_after_ident(i - 1)
        };
        let into_ident = syn::Ident::new(&format!("into_{}", tg.accessor_snake), span);
        let g_decoder_ident = syn::Ident::new(&tg.group_decoder_ident, span);
        let se = start_expr(i);
        ts.extend(quote::quote! {
            impl<'a> #current_stage<'a> {
                /// Consume this stage and start decoding the next tail group,
                /// enforcing wire order. The returned group decoder owns the
                /// right to advance to the following stage via `finish()`.
                #[inline]
                pub fn #into_ident(self) -> Result<#g_decoder_ident<'a>, sbe_rt::DecodeError> {
                    let group_start = #se;
                    #g_decoder_ident::wrap_with_parent(
                        self.buf,
                        group_start,
                        self.acting_version,
                        self.pos,
                        self.acting_block_length,
                    )
                }
            }
        });
    }

    // 2b. Var-data into_<vd>(): read the field and advance.
    for (vi, vd) in vardata.iter().enumerate() {
        let i = groups.len() + vi;
        let current_stage = if i == 0 {
            initial_ident.clone()
        } else {
            stage_after_ident(i - 1)
        };
        let next_stage = stage_after_ident(i);
        let into_ident = syn::Ident::new(&format!("into_{}", vd.accessor_snake), span);
        let slice_ident = syn::Ident::new(&format!("{}_slice", vd.accessor_snake), span);
        let prefix_size_lit = syn::LitInt::new(&vd.prefix_size.to_string(), span);
        let len_type_ident = syn::Ident::new(rust_type(vd.len_type), span);
        let len_from_endian = syn::Ident::new(
            match byte_order {
                ByteOrder::LittleEndian => "from_le_bytes",
                ByteOrder::BigEndian => "from_be_bytes",
            },
            span,
        );
        let vd_name_lit = syn::LitStr::new(&vd.name, span);
        let se = start_expr(i);
        let mut max_check = proc_macro2::TokenStream::new();
        if let Some(max) = vd.max_length {
            let max_lit = syn::LitInt::new(&max.to_string(), span);
            max_check.extend(quote::quote! {
                if len > #max_lit {
                    return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                        field: #vd_name_lit,
                        length: len,
                        max_length: #max_lit as u64,
                    });
                }
            });
        }
        ts.extend(quote::quote! {
            impl<'a> #current_stage<'a> {
                /// Consume this stage, read the next var-data field, and advance
                /// to the following stage. Wire order is enforced by consumption.
                #[inline]
                pub fn #into_ident(self) -> Result<(&'a [u8], #next_stage<'a>), sbe_rt::DecodeError> {
                    let offset = #se;
                    if offset + #prefix_size_lit > self.buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #vd_name_lit,
                            needed: #prefix_size_lit,
                            available: self.buf.len().saturating_sub(offset),
                        });
                    }
                    // SAFETY: bounds verified by the preceding check
                    // (offset + prefix_size <= buf.len()).
                    let bytes: [u8; #prefix_size_lit] = unsafe {
                        core::ptr::read_unaligned(
                            self.buf.as_ptr().add(offset) as *const [u8; #prefix_size_lit],
                        )
                    };
                    // Direct integer read — avoids constructing the var-data
                    // encoding struct while preserving its width and schema byte order.
                    let len = #len_type_ident::#len_from_endian(bytes) as u64;
                    #max_check
                    let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                        #vd_name_lit,
                        offset,
                        #prefix_size_lit,
                        len,
                        self.buf.len(),
                    )?;
                    let data = &self.buf[data_start..data_end];
                    let next = #next_stage {
                        buf: self.buf,
                        pos: self.pos,
                        tail_start: data_end,
                        acting_version: self.acting_version,
                        acting_block_length: self.acting_block_length,
                    };
                    Ok((data, next))
                }

                /// Non-consuming variant: read this var-data field as `&[u8]`
                /// without advancing or constructing the next stage. Cheaper
                /// than [`Self::#into_ident`] when only the bytes are needed.
                #[inline]
                pub fn #slice_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    let offset = #se;
                    if offset + #prefix_size_lit > self.buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #vd_name_lit,
                            needed: #prefix_size_lit,
                            available: self.buf.len().saturating_sub(offset),
                        });
                    }
                    let bytes: [u8; #prefix_size_lit] = unsafe {
                        core::ptr::read_unaligned(
                            self.buf.as_ptr().add(offset) as *const [u8; #prefix_size_lit],
                        )
                    };
                    // Direct integer read — avoids constructing the var-data
                    // encoding struct while preserving its width and schema byte order.
                    let len = #len_type_ident::#len_from_endian(bytes) as u64;
                    #max_check
                    let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                        #vd_name_lit,
                        offset,
                        #prefix_size_lit,
                        len,
                        self.buf.len(),
                    )?;
                    Ok(&self.buf[data_start..data_end])
                }
            }
        });

        // Text var-data: into_<field>_as_str() for schema-declared characterEncoding.
        if let Some(ref enc) = vd.character_encoding {
            let is_utf8 = enc.eq_ignore_ascii_case("UTF-8") || enc.eq_ignore_ascii_case("UTF8");
            let is_ascii =
                enc.eq_ignore_ascii_case("ASCII") || enc.eq_ignore_ascii_case("US-ASCII");
            if is_utf8 || is_ascii {
                let as_str_ident =
                    syn::Ident::new(&format!("into_{}_as_str", vd.accessor_snake), span);
                let into_ident = syn::Ident::new(&format!("into_{}", vd.accessor_snake), span);
                if is_ascii {
                    ts.extend(quote::quote! {
                        impl<'a> #current_stage<'a> {
                            /// Consume this stage, read the next ASCII var-data
                            /// field as a validated `&str`, and advance.
                            #[inline]
                            pub fn #as_str_ident(self) -> Result<(&'a str, #next_stage<'a>), sbe_rt::DecodeError> {
                                let (bytes, next) = self.#into_ident()?;
                                if !bytes.is_ascii() {
                                    return Err(sbe_rt::DecodeError::InvalidAscii { field: #vd_name_lit });
                                }
                                let s = unsafe { core::str::from_utf8_unchecked(bytes) };
                                Ok((s, next))
                            }
                        }
                    });
                } else {
                    ts.extend(quote::quote! {
                        impl<'a> #current_stage<'a> {
                            /// Consume this stage, read the next UTF-8 var-data
                            /// field as a validated `&str`, and advance.
                            #[inline]
                            pub fn #as_str_ident(self) -> Result<(&'a str, #next_stage<'a>), sbe_rt::DecodeError> {
                                let (bytes, next) = self.#into_ident()?;
                                let s = core::str::from_utf8(bytes).map_err(|e| {
                                    sbe_rt::DecodeError::InvalidUtf8 { field: #vd_name_lit, error: e }
                                })?;
                                Ok((s, next))
                            }
                        }
                    });
                }

                let as_str_unchecked = syn::Ident::new(
                    &format!("into_{}_as_str_unchecked", vd.accessor_snake),
                    span,
                );
                ts.extend(quote::quote! {
                    impl<'a> #current_stage<'a> {
                        /// Consume this stage, read the next text var-data field as
                        /// a `&str` without encoding validation, and advance.
                        ///
                        /// # Safety
                        /// The wire bytes must be valid for the schema-declared
                        /// character encoding (UTF-8 or ASCII).
                        #[inline]
                        pub unsafe fn #as_str_unchecked(self) -> (&'a str, #next_stage<'a>) {
                            let (bytes, next) = unsafe { self.#into_ident().unwrap() };
                            let s = unsafe { core::str::from_utf8_unchecked(bytes) };
                            (s, next)
                        }
                    }
                });
            }
        }

        // Scoped fallible combinator: try_<data> always available.
        let try_data_ident = syn::Ident::new(&format!("try_{}", vd.accessor_snake), span);
        ts.extend(quote::quote! {
            impl<'a> #current_stage<'a> {
                /// Fallible scoped var-data accessor. Calls the closure with
                /// the decoded bytes and returns the next stage on success.
                #[inline]
                pub fn #try_data_ident<E, F>(
                    self,
                    f: F,
                ) -> Result<#next_stage<'a>, E>
                where
                    E: From<sbe_rt::DecodeError>,
                    F: FnOnce(&[u8]) -> Result<(), E>,
                {
                    let (data, next) = self.#into_ident()?;
                    f(data)?;
                    Ok(next)
                }
            }
        });

        // Nested-message helpers need AnyMessage/DecodedFrame (dispatch surface).
        if enable_dispatch {
            let as_msg_ident =
                syn::Ident::new(&format!("into_{}_as_message", vd.accessor_snake), span);
            let try_data_as_msg_ident =
                syn::Ident::new(&format!("try_{}_as_message", vd.accessor_snake), span);
            ts.extend(quote::quote! {
                impl<'a> #current_stage<'a> {
                    /// Consume this stage, decode the var-data field as a nested
                    /// SBE message via `AnyMessage::decode_frame`, and advance
                    /// to the next stage.
                    #[inline]
                    pub fn #as_msg_ident(self) -> Result<(DecodedFrame<'a>, #next_stage<'a>), sbe_rt::DecodeError> {
                        let (data, next) = self.#into_ident()?;
                        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
                        Ok((frame, next))
                    }

                    /// Fallible scoped nested-message accessor.
                    #[inline]
                    pub fn #try_data_as_msg_ident<E, F>(
                        self,
                        f: F,
                    ) -> Result<#next_stage<'a>, E>
                    where
                        E: From<sbe_rt::DecodeError>,
                        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
                    {
                        let (frame, next) = self.#as_msg_ident()?;
                        f(frame)?;
                        Ok(next)
                    }
                }
            });
        }
    }

    for (gi, tg) in groups.iter().enumerate() {
        let i = gi;
        let next_stage = stage_after_ident(i);
        let g_decoder_ident = syn::Ident::new(&tg.group_decoder_ident, span);
        let entry_decoder_ident = syn::Ident::new(&tg.entry_decoder_ident, span);
        ts.extend(quote::quote! {
            impl<'a> #g_decoder_ident<'a> {
                /// Scan past any unread entries (including nested tails) in wire
                /// order and return the next decoder stage.
                #[inline]
                pub fn finish(self) -> Result<#next_stage<'a>, sbe_rt::DecodeError> {
                    let mut pos = self.pos;
                    let mut remaining = self.count;
                    let block_len = self.acting_block_length;
                    while remaining > 0 {
                        pos = #entry_decoder_ident::skip(self.buf, pos, block_len, self.acting_version)?;
                        remaining -= 1;
                    }
                    Ok(#next_stage {
                        buf: self.buf,
                        pos: self.parent_pos,
                        tail_start: pos,
                        acting_version: self.acting_version,
                        acting_block_length: self.parent_block_length,
                    })
                }
                /// Explicit sequential spelling of "advance past the rest of this group".
                #[inline]
                pub fn skip_remaining(self) -> Result<#next_stage<'a>, sbe_rt::DecodeError> {
                    self.finish()
                }
            }
        });
    }

    let complete_ident = stage_after_ident(total_tail - 1);
    // Message complete stages: `pos` is body start; header is `header_size`
    // bytes before. Entry complete stages pass `header_size == 0`, so the
    // header-inclusive view equals the body view.
    ts.extend(quote::quote! {
        impl<'a> #complete_ident<'a> {
            /// Body bytes (excluding the message header; for entries this is the
            /// complete entry bytes).
            #[inline]
            pub fn as_body_bytes(&self) -> &'a [u8] {
                &self.buf[self.pos..self.tail_start]
            }
            /// Complete SBE frame (header + body) for message stages.
            /// For entry stages (`HEADER_LENGTH == 0`) this equals [`Self::as_body_bytes`].
            #[inline]
            pub fn as_bytes_with_header(&self) -> &'a [u8] {
                &self.buf[self.pos - #header_size_lit..self.tail_start]
            }
            /// Body length (excluding header).
            #[inline]
            pub fn encoded_length(&self) -> usize {
                self.tail_start - self.pos
            }
            /// Total message length including the schema-declared header.
            /// Pure arithmetic: body length + `HEADER_LENGTH`.
            #[inline]
            pub fn encoded_length_with_header(&self) -> usize {
                self.tail_start - self.pos + #header_size_lit
            }
            /// Bytes after this message/entry.
            #[inline]
            pub fn remaining(&self) -> &'a [u8] {
                &self.buf[self.tail_start..]
            }
        }
    });

    ts
}

/// Message-level consuming tail stages (DECISIONS.md §3): thin wrapper that
/// resolves the message's tail groups + var-data into descriptors and delegates
/// to `generate_owner_consuming_stages`.
pub(crate) fn generate_decoder_consuming_stages(
    msg: &MessageStructure,
    elements: &SchemaElements,
    name: &str,
    header_size: usize,
    byte_order: ByteOrder,
    _multi_message: bool,
    group_unique_names: &[String],
    enable_dispatch: bool,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let stage_prefix = format!("{name}Decoder");
    let initial_ident = syn::Ident::new(&stage_prefix, span);
    let groups: Vec<OwnerTailGroup> = msg
        .groups
        .iter()
        .enumerate()
        .map(|(gi, g)| OwnerTailGroup {
            accessor_snake: to_snake_case(&g.name),
            field_pascal: to_pascal_case(&g.name),
            group_decoder_ident: format!("{}Decoder", group_unique_names[gi]),
            entry_decoder_ident: format!("{}EntryDecoder", group_unique_names[gi]),
        })
        .collect();
    let vardata: Vec<OwnerTailVarData> = msg
        .var_data
        .iter()
        .map(|vd| {
            let (type_pascal, prefix_size, len_field, len_type) =
                get_vardata_info(elements, &vd.type_name);
            OwnerTailVarData {
                accessor_snake: to_snake_case(&vd.name),
                field_pascal: to_pascal_case(&vd.name),
                type_pascal,
                prefix_size,
                len_field,
                len_type,
                max_length: vd.max_length,
                name: vd.name.clone(),
                character_encoding: vd.character_encoding.clone(),
            }
        })
        .collect();
    generate_owner_consuming_stages(
        initial_ident,
        &stage_prefix,
        header_size,
        byte_order,
        &groups,
        &vardata,
        enable_dispatch,
    )
}

/// Entry-level consuming tail stages for a group whose entries have nested
/// groups and/or var-data (DECISIONS.md §3, Task D). `name` is the group's
/// scoped name; nested group decoder names are `{name}{Ng}Decoder`.
pub(crate) fn generate_entry_consuming_stages(
    g: &MessageGroup,
    elements: &SchemaElements,
    name: &str,
    byte_order: ByteOrder,
    enable_dispatch: bool,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let entry_prefix = format!("{name}EntryDecoder");
    let initial_ident = syn::Ident::new(&entry_prefix, span);
    let groups: Vec<OwnerTailGroup> = g
        .groups
        .iter()
        .map(|ng| {
            let ng_pascal = format!("{}{}", name, to_pascal_case(&ng.name));
            OwnerTailGroup {
                accessor_snake: to_snake_case(&ng.name),
                field_pascal: to_pascal_case(&ng.name),
                group_decoder_ident: format!("{ng_pascal}Decoder"),
                entry_decoder_ident: format!("{ng_pascal}EntryDecoder"),
            }
        })
        .collect();
    let vardata: Vec<OwnerTailVarData> = g
        .var_data
        .iter()
        .map(|vd| {
            let (type_pascal, prefix_size, len_field, len_type) =
                get_vardata_info(elements, &vd.type_name);
            OwnerTailVarData {
                accessor_snake: to_snake_case(&vd.name),
                field_pascal: to_pascal_case(&vd.name),
                type_pascal,
                prefix_size,
                len_field,
                len_type,
                max_length: vd.max_length,
                name: vd.name.clone(),
                character_encoding: vd.character_encoding.clone(),
            }
        })
        .collect();
    generate_owner_consuming_stages(
        initial_ident,
        &entry_prefix,
        0,
        byte_order,
        &groups,
        &vardata,
        enable_dispatch,
    )
}

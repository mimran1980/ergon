//! Message/entry consuming tail-stage codegen.
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

/// Absent tails occupy no bytes and contain no entries at older wire versions.
///
/// `acting_version` is how the generated type reaches the wire version: most
/// decoders read `self.acting_version`, the memoized wrapper reads
/// `self.inner.acting_version`. One function so the four locations that emit
/// `<group>_count` / `<field>_len` cannot drift on what "absent" means.
pub(crate) fn absent_tail_length(
    since_version: u16,
    acting_version: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    if since_version > 0 {
        quote::quote! {
            if #acting_version < #since_version { return Ok(0); }
        }
    } else {
        proc_macro2::TokenStream::new()
    }
}

/// `self.acting_version` — the reader for every decoder except the memoized
/// wrapper.
pub(crate) fn own_acting_version() -> proc_macro2::TokenStream {
    quote::quote! { self.acting_version }
}

pub(crate) fn generate_owner_consuming_stages(
    initial_ident: syn::Ident,
    stage_prefix: &str,
    header_size: usize,
    byte_order: ByteOrder,
    groups: &[OwnerTailGroup],
    vardata: &[OwnerTailVarData],
    enable_dispatch: bool,
    // True when the initial owner stage is a message decoder (keeps `offset`,
    // exposes `byte_offset()`); false for entry decoders, which also keep `offset`.
    initial_has_byte_offset: bool,
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
            /// Consuming decoder stage — drop without `into_*` / `skip_*`
            /// skips remaining wire tails.
            #[must_use = "decoder stage must be advanced with into_*/skip_* or tails are skipped"]
            pub struct #stage<'a> {
                pub(crate) buf: &'a [u8],
                pub(crate) offset: usize,
                pub(crate) tail_start: usize,
                pub(crate) acting_version: u16,
                pub(crate) acting_block_length: usize,
            }
        });
    }

    // acting_version() / acting_block_length() on every stage.
    for i in 0..total_tail {
        let stage = stage_after_ident(i);
        ts.extend(quote::quote! {
            impl<'a> #stage<'a> {
                /// Schema version from the message header (or wrap args), not the
                /// compiled schema constant. Fields with `sinceVersion` and optional
                /// presence depend on this value.
                #[inline]
                pub const fn acting_version(&self) -> u16 { self.acting_version }
                /// Block length from the wire header / wrap args. Tail offsets use
                /// this acting length, not only the compiled `BLOCK_LENGTH`.
                #[inline]
                pub const fn acting_block_length(&self) -> usize { self.acting_block_length }
            }
        });
    }

    // The initial stage is the message decoder itself (message tails), which
    // keeps `offset` and exposes `byte_offset()`. For entry tails the initial stage
    // is the entry decoder, which also keeps `offset` and has no `byte_offset()`.
    // Later stages all carry `offset` directly.
    let parent_pos_expr = |i: usize| -> syn::Expr {
        if i == 0 && initial_has_byte_offset {
            syn::parse_str("self.byte_offset()").unwrap()
        } else {
            syn::parse_str("self.offset").unwrap()
        }
    };
    let start_expr = |i: usize| -> syn::Expr {
        if i == 0 && initial_has_byte_offset {
            syn::parse_str("self.byte_offset() + self.acting_block_length").unwrap()
        } else if i == 0 {
            syn::parse_str("self.offset + self.acting_block_length").unwrap()
        } else {
            syn::parse_str("self.tail_start").unwrap()
        }
    };

    // 2a. Group into_<g>() / skip_<g>() live on the stage that precedes each
    // group. The iterator *is* the stage: finish() or a following into_*/skip_*
    // yields the next parent stage. Generated together with skip_all.

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
        let len_ident = quote::format_ident!("{}_len", vd.accessor_snake);
        let absent_len = absent_tail_length(vd.since_version, &own_acting_version());
        // Initial owners also have random-access lengths; subsequent stages
        // expose only the next tail, using the same wire-order offset.
        let stage_len = (i > 0).then(|| {
            quote::quote! {
                /// Byte length without advancing this decoder stage.
                #[inline]
                pub fn #len_ident(&self) -> Result<usize, sbe_rt::DecodeError> {
                    #absent_len
                    Ok(self.#slice_ident()?.len())
                }
            }
        });
        let slice_doc = format!(
            "Non-consuming variant: read this var-data field as `&[u8]` without \
             advancing or constructing the next stage.\n\n\
             Cheaper than [`Self::{into_ident}`] when only the bytes are needed."
        );
        let slice_doc_tokens = crate::codegen::runtime::doc_lines_tokens(&slice_doc);
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
        let pp = parent_pos_expr(i);
        // Only the entry decoder carries the `tail_end` one-shot cache, and it
        // holds the end of the *last* tail component. So the non-consuming
        // slice accessor can reuse it exactly when this var-data is both the
        // first and the last tail of a group entry — the common
        // "group of records, one string each" shape. Mirrors the flat entry
        // accessor in `group_decoder.rs`; without it the two same-signature
        // methods on the same type differ only in that this one re-reads and
        // re-validates a length header the iterator already resolved.
        // The message-level cache lives on the opt-in memoized lane, not here.
        let slice_cached_tail = proc_macro2::TokenStream::new();
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
        let absent_vardata = if vd.since_version > 0 {
            let since_lit = syn::LitInt::new(
                &vd.since_version.to_string(),
                proc_macro2::Span::call_site(),
            );
            quote::quote! {
                if self.acting_version < #since_lit {
                    let next = #next_stage {
                        buf: self.buf,
                        offset: #pp,
                        tail_start: #se,
                        acting_version: self.acting_version,
                        acting_block_length: self.acting_block_length,
                    };
                    return Ok((&[][..], next));
                }
            }
        } else {
            proc_macro2::TokenStream::new()
        };
        ts.extend(quote::quote! {
            impl<'a> #current_stage<'a> {
                #stage_len
                /// Consume this stage, read the next var-data field, and advance
                /// to the following stage. Wire order is enforced by consumption.
                #[inline]
                pub fn #into_ident(self) -> Result<(&'a [u8], #next_stage<'a>), sbe_rt::DecodeError> {
                    #absent_vardata
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
                        offset: #pp,
                        tail_start: data_end,
                        acting_version: self.acting_version,
                        acting_block_length: self.acting_block_length,
                    };
                    Ok((data, next))
                }

                #slice_doc_tokens
                #[inline]
                pub fn #slice_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    #slice_cached_tail
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
        {
            if let Some(kind) = super::runtime::text_encoding_kind(vd.character_encoding.as_deref())
            {
                let as_str_ident =
                    syn::Ident::new(&format!("into_{}_as_str", vd.accessor_snake), span);
                let into_ident = syn::Ident::new(&format!("into_{}", vd.accessor_snake), span);
                if matches!(kind, super::runtime::TextEncoding::Ascii) {
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
                        /// Structural bounds (truncated payload, overflowing length)
                        /// remain fallible — only character validation is skipped.
                        ///
                        /// # Safety
                        /// The wire bytes must be valid for the schema-declared
                        /// character encoding (UTF-8 or ASCII).
                        #[inline]
                        pub unsafe fn #as_str_unchecked(
                            self,
                        ) -> Result<(&'a str, #next_stage<'a>), sbe_rt::DecodeError> {
                            let (bytes, next) = self.#into_ident()?;
                            let s = unsafe { core::str::from_utf8_unchecked(bytes) };
                            Ok((s, next))
                        }
                    }
                });
            }
        }

        // Optional-crate accessors. Emitted only when the *generator* was built
        // with the matching feature — a consumer who did not ask for
        // `compact_str` gets no `compact_str` code at all. (Emitting them
        // unconditionally behind `#[cfg(feature = ...)]` put the gate on the
        // consumer crate's own feature set, where it silently never matched.)
        {
            #[cfg(any(feature = "compact_str", feature = "smol_str", feature = "bytes"))]
            let into_ident = syn::Ident::new(&format!("into_{}", vd.accessor_snake), span);
            #[cfg(feature = "compact_str")]
            {
                let as_compact_ident =
                    syn::Ident::new(&format!("into_{}_as_compact_str", vd.accessor_snake), span);
                ts.extend(quote::quote! {
                    impl<'a> #current_stage<'a> {
                        /// Consume this stage, read the next var-data field as a
                        /// [`ergo_sbe::compact_str::CompactString`] (≤24 bytes inline), and advance.
                        #[inline]
                        pub fn #as_compact_ident(self) -> Result<(ergo_sbe::compact_str::CompactString, #next_stage<'a>), sbe_rt::DecodeError> {
                            let (bytes, next) = self.#into_ident()?;
                            let s = core::str::from_utf8(bytes).map_err(|e| {
                                sbe_rt::DecodeError::InvalidUtf8 { field: #vd_name_lit, error: e }
                            })?;
                            Ok((ergo_sbe::compact_str::CompactString::new(s), next))
                        }
                    }
                });
            }
            #[cfg(feature = "smol_str")]
            {
                let as_smol_ident =
                    syn::Ident::new(&format!("into_{}_as_smol_str", vd.accessor_snake), span);
                ts.extend(quote::quote! {
                    impl<'a> #current_stage<'a> {
                        /// Consume this stage, read the next var-data field as a
                        /// [`ergo_sbe::smol_str::SmolStr`] (O(1) clone), and advance.
                        #[inline]
                        pub fn #as_smol_ident(self) -> Result<(ergo_sbe::smol_str::SmolStr, #next_stage<'a>), sbe_rt::DecodeError> {
                            let (bytes, next) = self.#into_ident()?;
                            let s = core::str::from_utf8(bytes).map_err(|e| {
                                sbe_rt::DecodeError::InvalidUtf8 { field: #vd_name_lit, error: e }
                            })?;
                            Ok((ergo_sbe::smol_str::SmolStr::new(s), next))
                        }
                    }
                });
            }
            #[cfg(feature = "bytes")]
            {
                let as_bytes_ident =
                    syn::Ident::new(&format!("into_{}_as_bytes", vd.accessor_snake), span);
                ts.extend(quote::quote! {
                    impl<'a> #current_stage<'a> {
                        /// Consume this stage, read the next var-data field as
                        /// [`ergo_sbe::bytes::Bytes`] (one copy from wire, then shared ownership), and advance.
                        #[inline]
                        pub fn #as_bytes_ident(self) -> Result<(ergo_sbe::bytes::Bytes, #next_stage<'a>), sbe_rt::DecodeError> {
                            let (data, next) = self.#into_ident()?;
                            Ok((ergo_sbe::bytes::Bytes::copy_from_slice(data), next))
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

    // Group tails come in two shapes, and the shape decides the API.
    //
    // Entries with no tails of their own have a fixed stride, so the next entry
    // is `offset + acting_block_length` — knowing where an entry ends costs
    // nothing, and the group is a real `Iterator`.
    //
    // Entries that carry their own groups or var-data have no stride. The only
    // way to find entry N+1 is to measure entry N, and `Iterator::next` cannot
    // learn that from the entry it already handed away — `Item` does not borrow
    // from the `&mut self` that produced it. An iterator would therefore have to
    // measure every entry before yielding it, and the caller's own walk of that
    // entry would repeat the traversal. So these keep the fused visit closure:
    // the callback returns the entry's completion stage, and *that* is the next
    // cursor. One pass, proven by the type rather than re-derived.
    for (gi, tg) in groups.iter().enumerate() {
        let i = gi;
        let current_stage = if i == 0 {
            initial_ident.clone()
        } else {
            stage_after_ident(i - 1)
        };
        let next_stage = stage_after_ident(i);
        let into_ident = syn::Ident::new(&format!("into_{}", tg.accessor_snake), span);
        let skip_ident = syn::Ident::new(&format!("skip_{}", tg.accessor_snake), span);
        let g_decoder_ident = syn::Ident::new(&tg.group_decoder_ident, span);
        let entry_decoder_ident = syn::Ident::new(&tg.entry_decoder_ident, span);
        let iter_type = quote::format_ident!("{}Iter", tg.group_decoder_ident);
        let count_method = quote::format_ident!("{}_count", tg.accessor_snake);
        let g_name_lit = syn::LitStr::new(&tg.name, span);
        let since_version = tg.since_version;
        let se = start_expr(i);
        let pp = parent_pos_expr(i);
        // The group is not on the wire at this version: it occupies zero bytes
        // at `group_start`, which is therefore already the next tail cursor.
        // Used by the methods that return the *following stage* — `skip_*` and
        // the dynamic-entry `into_*(visit)`. The iterator `into_*` returns an
        // iterator instead, so it takes `versioned_wrap` below.
        let absent_stage_guard = (since_version > 0).then(|| {
            let since_lit =
                syn::LitInt::new(&since_version.to_string(), proc_macro2::Span::call_site());
            quote::quote! {
                if self.acting_version < #since_lit {
                    // SAFETY: this stage was reached in wire order, so `#pp`
                    // and `self.acting_block_length` describe the real parent
                    // body.
                    let attached = unsafe {
                        <#g_decoder_ident<'a, sbe_rt::Attached>>::wrap_absent_parent(
                            self.buf,
                            group_start,
                            self.acting_version,
                            #pp,
                            self.acting_block_length,
                        )
                    };
                    return Ok(attached.into_parent_stage(group_start));
                }
            }
        });
        let wrap_attached = quote::quote! {
            <#g_decoder_ident<'a, sbe_rt::Attached>>::wrap_with_parent(
                self.buf, group_start, self.acting_version, #pp, self.acting_block_length,
            )
        };
        // Version-aware wrap producing the group decoder itself: an absent
        // tail yields an empty group rather than returning a stage. Used by the
        // non-advancing count and by the iterator `into_*`.
        let versioned_wrap = if since_version > 0 {
            quote::quote! {
                if self.acting_version < #since_version {
                    <#g_decoder_ident<'a, sbe_rt::Attached>>::wrap_absent_parent(
                        self.buf, group_start, self.acting_version, #pp, self.acting_block_length,
                    )
                } else { #wrap_attached? }
            }
        } else {
            quote::quote! { #wrap_attached? }
        };
        let stage_count = (i > 0).then(|| {
            quote::quote! {
                /// Wire-declared entry count without advancing this decoder stage.
                #[inline]
                pub fn #count_method(&self) -> Result<usize, sbe_rt::DecodeError> {
                    let group_start = #se;
                    // SAFETY: this stage owns the genuine parent and next-tail
                    // offset; the header and extent are validated inside.
                    let inner = unsafe { #versioned_wrap };
                    Ok(inner.remaining_entries())
                }
            }
        });
        // Dimension read for the ordered lane's *dynamic* branch, which has no
        // iterator to ask: `(count, block_length)` without advancing. Always
        // emitted — unlike the public `<group>_count()`, this one cannot be
        // suppressed by a name collision, because the `__sbe_` prefix is not
        // reachable from any schema element name.
        let dim_helper = quote::format_ident!("__sbe_{}_dim", tg.accessor_snake);
        let stage_dim = quote::quote! {
            #[doc(hidden)]
            #[inline]
            pub(crate) fn #dim_helper(&self) -> Result<(usize, usize), sbe_rt::DecodeError> {
                let group_start = #se;
                // SAFETY: same proof as `into_*` / `skip_*` on this stage.
                let inner = unsafe { #versioned_wrap };
                Ok((inner.remaining_entries(), inner.acting_block_length))
            }
        };

        if tg.entries_have_tails {
            let entry_complete_ident =
                syn::Ident::new(&format!("{entry_decoder_ident}Complete"), span);
            ts.extend(quote::quote! {
                impl<'a> #g_decoder_ident<'a, sbe_rt::Attached> {
                    // Internal implementation detail of the fused `into_*`
                    // method on the preceding stage — not part of the public
                    // API. Consumes every remaining entry in one pass and
                    // returns the next parent stage.
                    //
                    // The callback must return this entry's generated
                    // completion stage; the next cursor comes from that
                    // completion, not a pre-scan of `encoded_length()`. Empty
                    // groups invoke the callback zero times. A callback or
                    // decoding error consumes this stage and returns no
                    // continuation. Returning a completion that does not belong
                    // to the supplied entry panics.
                    #[inline]
                    fn walk<E, F>(mut self, mut visit: F) -> Result<#next_stage<'a>, E>
                    where
                        E: From<sbe_rt::DecodeError>,
                        F: FnMut(#entry_decoder_ident<'a>) -> Result<#entry_complete_ident<'a>, E>,
                    {
                        if let Some(error) = self.poisoned {
                            return Err(E::from(error));
                        }
                        while self.count > 0 {
                            let available = self.buf.len().saturating_sub(self.offset);
                            if self.min_entry_extent > available {
                                return Err(E::from(sbe_rt::DecodeError::BufferTooShort {
                                    field: #g_name_lit,
                                    needed: self.min_entry_extent,
                                    available,
                                }));
                            }
                            // SAFETY: acting fixed block proven in-bounds above.
                            // The callback walks the dynamic tail; the next
                            // cursor is the returned completion's `tail_start`.
                            let entry = unsafe {
                                #entry_decoder_ident::wrap(
                                    self.buf,
                                    self.offset,
                                    self.acting_block_length,
                                    self.acting_version,
                                )
                            };
                            let complete = visit(entry)?;
                            if !core::ptr::eq(complete.buf.as_ptr(), self.buf.as_ptr())
                                || complete.buf.len() != self.buf.len()
                                || complete.offset != self.offset
                                || complete.acting_version != self.acting_version
                                || complete.acting_block_length != self.acting_block_length
                            {
                                panic!(
                                    "group visit callback returned a completion that does not belong to the supplied entry"
                                );
                            }
                            self.offset = complete.tail_start;
                            self.count -= 1;
                        }
                        let tail_start = self.offset;
                        Ok(self.into_parent_stage(tail_start))
                    }
                }
                impl<'a> #current_stage<'a> {
                    /// Consume this stage, visit every entry of the next group
                    /// in wire order, and return the following stage.
                    ///
                    /// These entries carry tails of their own, so they have no
                    /// stride. The callback returns the entry's completion
                    /// stage and that *is* the next cursor — the group is
                    /// walked exactly once. A callback error consumes this
                    /// stage and returns no continuation; there is nothing to
                    /// retry from a consuming lane.
                    #[inline]
                    pub fn #into_ident<E, F>(self, visit: F) -> Result<#next_stage<'a>, E>
                    where
                        E: From<sbe_rt::DecodeError>,
                        F: FnMut(#entry_decoder_ident<'a>) -> Result<#entry_complete_ident<'a>, E>,
                    {
                        let group_start = #se;
                        #absent_stage_guard
                        // SAFETY: this stage was reached by consuming the
                        // message in wire order, so `#pp` and
                        // `self.acting_block_length` describe the real parent
                        // body and `group_start` is this group's genuine
                        // dimension-header offset. The header, block length,
                        // and extent are still validated inside.
                        let attached = unsafe { #wrap_attached }?;
                        attached.walk(visit)
                    }
                }
            });
        } else {
            let into_doc = format!(
                " Consume this stage into a group iterator. These entries have a \
                 fixed stride, so the iterator *is* this tail at no cost: unread \
                 entries are skipped when you [`{iter}::finish`] or call a \
                 following `into_*` / `skip_*`. Iterate with \
                 `for entry in &mut iter`. `for entry in iter` does not compile, \
                 so the rest of the message cannot be dropped by a loop.",
                iter = iter_type
            );
            ts.extend(quote::quote! {
                /// Fixed-stride group iterator — this type *is* the decoder
                /// stage for the group. Call [`Self::finish`] or a following
                /// `into_*` / `skip_*` to reach the next tail.
                #[must_use = "call finish() or a following into_*/skip_* or remaining tails are skipped"]
                pub struct #iter_type<'a> {
                    inner: #g_decoder_ident<'a, sbe_rt::Attached>,
                }
                impl<'a> #iter_type<'a> {
                    /// Entries not yet yielded. Exact: a fixed-stride group's
                    /// extent was proven when this stage was constructed.
                    #[inline]
                    pub const fn remaining_entries(&self) -> usize {
                        self.inner.remaining_entries()
                    }
                    /// Acting block length of one entry, from the group's
                    /// dimension header.
                    #[inline]
                    #[must_use]
                    pub const fn entry_block_length(&self) -> usize {
                        self.inner.acting_block_length
                    }
                    /// Skip any unread entries and return the following decoder stage.
                    #[inline]
                    pub fn finish(self) -> Result<#next_stage<'a>, sbe_rt::DecodeError> {
                        self.inner.skip_all()
                    }
                }
                impl<'a> Iterator for &mut #iter_type<'a> {
                    type Item = #entry_decoder_ident<'a>;
                    #[inline]
                    fn next(&mut self) -> Option<Self::Item> { self.inner.next() }
                    #[inline]
                    fn size_hint(&self) -> (usize, Option<usize>) { self.inner.size_hint() }
                }
                impl<'a> ExactSizeIterator for &mut #iter_type<'a> {
                    #[inline]
                    fn len(&self) -> usize { self.inner.remaining_entries() }
                }
                impl<'a> core::iter::FusedIterator for &mut #iter_type<'a> {}
                impl<'a> #current_stage<'a> {
                    #[doc = #into_doc]
                    #[inline]
                    pub fn #into_ident(self) -> Result<#iter_type<'a>, sbe_rt::DecodeError> {
                        let group_start = #se;
                        // SAFETY: this stage owns the genuine parent and
                        // next-tail offset; header and extent validated inside.
                        // A tail absent at this version wraps as an empty
                        // group, so the iterator yields nothing and `finish()`
                        // hands back the stage at `group_start`.
                        let inner = unsafe { #versioned_wrap };
                        Ok(#iter_type { inner })
                    }
                }
            });
        }

        let poisoned_finish_guard = tg.entries_have_tails.then(|| {
            quote::quote! {
                if let Some(error) = self.poisoned {
                    return Err(error);
                }
            }
        });
        // Entries with tails have no stride, so the end of the group is only
        // knowable by walking them. Fixed-stride entries all occupy the acting
        // block length, so the end is `count * block_length` — one multiply and
        // one bounds check instead of a loop that the optimiser does not
        // collapse.
        let skip_all_body = if tg.entries_have_tails {
            quote::quote! {
                let mut offset = self.offset;
                let mut remaining = self.count;
                let block_len = self.acting_block_length;
                while remaining > 0 {
                    offset = #entry_decoder_ident::skip(self.buf, offset, block_len, self.acting_version)?;
                    remaining -= 1;
                }
                Ok(self.into_parent_stage(offset))
            }
        } else {
            quote::quote! {
                let span = self
                    .count
                    .checked_mul(self.acting_block_length)
                    .ok_or(sbe_rt::DecodeError::BufferTooShort {
                        field: #g_name_lit,
                        needed: usize::MAX,
                        available: self.buf.len().saturating_sub(self.offset),
                    })?;
                let end = self.offset.checked_add(span).filter(|e| *e <= self.buf.len()).ok_or(
                    sbe_rt::DecodeError::BufferTooShort {
                        field: #g_name_lit,
                        needed: span,
                        available: self.buf.len().saturating_sub(self.offset),
                    },
                )?;
                Ok(self.into_parent_stage(end))
            }
        };
        ts.extend(quote::quote! {
            impl<'a> #g_decoder_ident<'a, sbe_rt::Attached> {
                #[inline]
                fn into_parent_stage(self, tail_start: usize) -> #next_stage<'a> {
                    #next_stage {
                        buf: self.buf,
                        offset: self.parent_pos,
                        tail_start,
                        acting_version: self.acting_version,
                        acting_block_length: self.parent_block_length,
                    }
                }
                // Internal implementation detail of `skip_*` / `Iter::finish`
                // on the preceding stage — not part of the public API. Scans
                // past any unread entries (including nested tails) in wire
                // order and returns the next decoder stage.
                //
                // Only an *attached* group — one reached through its
                // message's tail — can complete into a message stage. A
                // standalone [`Self::wrap`] has no parent to return to.
                #[inline]
                fn skip_all(self) -> Result<#next_stage<'a>, sbe_rt::DecodeError> {
                    // A poisoned group's position came from an entry that
                    // failed to decode, so the next stage would be built at a
                    // meaningless offset. Return the stored error instead.
                    #poisoned_finish_guard
                    #skip_all_body
                }
            }
            impl<'a> #current_stage<'a> {
                #stage_count
                #stage_dim
                /// Consume this stage, advance past the next group without
                /// visiting any entry, and return the following stage.
                #[inline]
                pub fn #skip_ident(self) -> Result<#next_stage<'a>, sbe_rt::DecodeError> {
                    let group_start = #se;
                    #absent_stage_guard
                    // SAFETY: same wrap as into_*, same parent-position proof.
                    let attached = unsafe { #wrap_attached }?;
                    attached.skip_all()
                }
            }
        });
    }

    // Following-tail methods on each fixed-stride group iterator: skip unread
    // entries, then delegate, so callers rarely name `finish()`. Dynamic groups
    // have no iterator — their `into_*(visit)` already returns the next stage.
    for (gi, tg) in groups.iter().enumerate() {
        if tg.entries_have_tails {
            continue;
        }
        let iter_type = quote::format_ident!("{}Iter", tg.group_decoder_ident);
        let mut next_tail = proc_macro2::TokenStream::new();
        if gi + 1 < groups.len() {
            let ng = &groups[gi + 1];
            let into_next = syn::Ident::new(&format!("into_{}", ng.accessor_snake), span);
            let skip_next = syn::Ident::new(&format!("skip_{}", ng.accessor_snake), span);
            let after_next = stage_after_ident(gi + 1);
            let into_doc = format!(
                " Skip any unread `{}` entries and consume `{}`.",
                tg.accessor_snake, ng.accessor_snake
            );
            let skip_doc = format!(
                " Skip any unread `{}` entries and skip `{}`.",
                tg.accessor_snake, ng.accessor_snake
            );
            if ng.entries_have_tails {
                let next_entry = syn::Ident::new(&ng.entry_decoder_ident, span);
                let next_complete =
                    syn::Ident::new(&format!("{}Complete", ng.entry_decoder_ident), span);
                next_tail.extend(quote::quote! {
                    #[doc = #into_doc]
                    #[inline]
                    pub fn #into_next<E, F>(self, visit: F) -> Result<#after_next<'a>, E>
                    where
                        E: From<sbe_rt::DecodeError>,
                        F: FnMut(#next_entry<'a>) -> Result<#next_complete<'a>, E>,
                    {
                        self.finish()?.#into_next(visit)
                    }
                });
            } else {
                let next_iter = quote::format_ident!("{}Iter", ng.group_decoder_ident);
                next_tail.extend(quote::quote! {
                    #[doc = #into_doc]
                    #[inline]
                    pub fn #into_next(self) -> Result<#next_iter<'a>, sbe_rt::DecodeError> {
                        self.finish()?.#into_next()
                    }
                });
            }
            next_tail.extend(quote::quote! {
                #[doc = #skip_doc]
                #[inline]
                pub fn #skip_next(self) -> Result<#after_next<'a>, sbe_rt::DecodeError> {
                    self.finish()?.#skip_next()
                }
            });
        } else if let Some(vd) = vardata.first() {
            let into_vd = syn::Ident::new(&format!("into_{}", vd.accessor_snake), span);
            let after_vd = stage_after_ident(groups.len());
            let into_doc = format!(
                " Skip any unread `{}` entries and read `{}`.",
                tg.accessor_snake, vd.accessor_snake
            );
            next_tail.extend(quote::quote! {
                #[doc = #into_doc]
                #[inline]
                pub fn #into_vd(self) -> Result<(&'a [u8], #after_vd<'a>), sbe_rt::DecodeError> {
                    self.finish()?.#into_vd()
                }
            });
            if super::runtime::text_encoding_kind(vd.character_encoding.as_deref()).is_some() {
                let as_str = syn::Ident::new(&format!("into_{}_as_str", vd.accessor_snake), span);
                let as_str_doc = format!(
                    " Skip any unread `{}` entries and read `{}` as `&str`.",
                    tg.accessor_snake, vd.accessor_snake
                );
                next_tail.extend(quote::quote! {
                    #[doc = #as_str_doc]
                    #[inline]
                    pub fn #as_str(self) -> Result<(&'a str, #after_vd<'a>), sbe_rt::DecodeError> {
                        self.finish()?.#as_str()
                    }
                });
            }
        }
        if !next_tail.is_empty() {
            ts.extend(quote::quote! {
                impl<'a> #iter_type<'a> {
                    #next_tail
                }
            });
        }
    }

    let complete_ident = stage_after_ident(total_tail - 1);
    // Message complete stages: `offset` is body start; header is `header_size`
    // bytes before. Entry complete stages pass `header_size == 0`, so the
    // header-inclusive view equals the body view.
    ts.extend(quote::quote! {
        impl<'a> #complete_ident<'a> {
            /// Body bytes (excluding the message header; for entries this is the
            /// complete entry bytes).
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn as_body_bytes(&self) -> &'a [u8] {
                &self.buf[self.offset..self.tail_start]
            }
            /// Complete SBE frame (header + body) for message stages.
            /// For entry stages (`HEADER_LENGTH == 0`) this equals [`Self::as_body_bytes`].
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn as_bytes_with_header(&self) -> &'a [u8] {
                &self.buf[self.offset - #header_size_lit..self.tail_start]
            }
            /// Body length (excluding header).
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn encoded_length(&self) -> usize {
                self.tail_start - self.offset
            }
            /// Total message length including the schema-declared header.
            /// Pure arithmetic: body length + `HEADER_LENGTH`.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn encoded_length_with_header(&self) -> usize {
                self.tail_start - self.offset + #header_size_lit
            }
            /// Bytes after this message/entry.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn remaining(&self) -> &'a [u8] {
                &self.buf[self.tail_start..]
            }
        }
    });

    ts
}

/// Message-level consuming tail stages: thin wrapper that resolves the
/// message's tail groups + var-data into descriptors and delegates to
/// `generate_owner_consuming_stages`.
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
            name: g.name.clone(),
            accessor_snake: to_snake_case(&g.name),
            field_pascal: to_pascal_case(&g.name),
            group_decoder_ident: format!("{}Decoder", group_unique_names[gi]),
            entry_decoder_ident: format!("{}EntryDecoder", group_unique_names[gi]),
            entries_have_tails: g.has_dynamic_entries(),
            since_version: g.since_version,
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
                since_version: vd.since_version,
            }
        })
        .collect();
    let mut ts = generate_owner_consuming_stages(
        initial_ident.clone(),
        &stage_prefix,
        header_size,
        byte_order,
        &groups,
        &vardata,
        enable_dispatch,
        true,
    );
    // Message level only: entries reach their nested tails through the staged
    // entry stages, which already give one callback per tail.
    ts.extend(generate_ordered_lane(
        &initial_ident,
        &stage_prefix,
        &groups,
        &vardata,
    ));
    ts
}

/// Entry-level consuming tail stages for a group whose entries have nested
/// groups and/or var-data. `name` is the group's scoped name; nested group
/// decoder names are `{name}{Ng}Decoder`.
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
                name: ng.name.clone(),
                accessor_snake: to_snake_case(&ng.name),
                field_pascal: to_pascal_case(&ng.name),
                group_decoder_ident: format!("{ng_pascal}Decoder"),
                entries_have_tails: ng.has_dynamic_entries(),
                entry_decoder_ident: format!("{ng_pascal}EntryDecoder"),
                since_version: ng.since_version,
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
                since_version: vd.since_version,
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
        false,
    )
}

/// Ordered lane: one callback per tail, in wire order, over the staged stages.
///
/// The staged lane spells a group two ways because the shapes genuinely differ.
/// The ordered lane offers one spelling for callers who want the whole message
/// in order and would rather write the same thing at every tail: every group
/// takes `FnMut(Entry, EntryInfo)`, every var-data takes `FnOnce(&[u8])`. It
/// owns no cursor of its own — each method delegates to the staged stage it
/// wraps, so the one-pass property and the compile-time ordering come from
/// there rather than being re-implemented.
///
/// Entries that carry their own tails still return their completion stage from
/// the callback: that completion *is* where the next entry starts, and giving
/// it up would mean pre-scanning every entry.
pub(crate) fn generate_ordered_lane(
    initial_ident: &syn::Ident,
    stage_prefix: &str,
    groups: &[OwnerTailGroup],
    vardata: &[OwnerTailVarData],
) -> proc_macro2::TokenStream {
    let total_tail = groups.len() + vardata.len();
    if total_tail == 0 {
        return proc_macro2::TokenStream::new();
    }
    let span = proc_macro2::Span::call_site();
    let field_pascals: Vec<String> = groups
        .iter()
        .map(|g| g.field_pascal.clone())
        .chain(vardata.iter().map(|v| v.field_pascal.clone()))
        .collect();

    let staged_stage =
        |i: usize| decoder_stage_after_ident(stage_prefix, &field_pascals[i], i, total_tail, span);
    let ordered_ident = syn::Ident::new(&format!("{stage_prefix}Ordered"), span);
    let ordered_stage =
        |i: usize| syn::Ident::new(&format!("{stage_prefix}Ordered{}", field_pascals[i]), span);

    let mut ts = proc_macro2::TokenStream::new();

    // Stage 0 wraps the base decoder; every later stage wraps its staged peer.
    ts.extend(quote::quote! {
        /// Ordered decode lane — one callback per tail, in wire order.
        ///
        /// Reached with [`Self::inner`]'s `ordered()`. Each method consumes
        /// this stage and returns the next, so the compiler enforces tail
        /// order exactly as the staged lane does.
        #[must_use = "ordered stage must be advanced or remaining tails are skipped"]
        pub struct #ordered_ident<'a> {
            inner: #initial_ident<'a>,
        }
        impl<'a> #initial_ident<'a> {
            /// Walk the whole message in wire order with one callback per tail.
            ///
            /// A façade over the staged `into_*` / `skip_*` stages: same single
            /// traversal, same compile-time ordering, one uniform spelling.
            #[inline]
            pub fn ordered(self) -> #ordered_ident<'a> {
                #ordered_ident { inner: self }
            }
        }
        impl<'a> #ordered_ident<'a> {
            /// Read the fixed block before any tail. Does not advance.
            #[inline]
            pub fn fixed<E, F>(self, f: F) -> Result<Self, E>
            where
                E: From<sbe_rt::DecodeError>,
                F: FnOnce(&#initial_ident<'a>) -> Result<(), E>,
            {
                f(&self.inner)?;
                Ok(self)
            }
        }
    });
    for i in 0..total_tail {
        let stage = ordered_stage(i);
        let staged = staged_stage(i);
        ts.extend(quote::quote! {
            /// Ordered decode stage — the tail named by this type has been read.
            #[must_use = "ordered stage must be advanced or remaining tails are skipped"]
            pub struct #stage<'a> {
                inner: #staged<'a>,
            }
        });
    }
    // Terminal stage hands the staged complete back, so extent helpers and
    // full-frame byte views stay reachable without a second set of methods.
    let last = ordered_stage(total_tail - 1);
    let last_staged = staged_stage(total_tail - 1);
    ts.extend(quote::quote! {
        impl<'a> #last<'a> {
            /// The completed staged decoder, for extent and byte-range helpers.
            #[inline]
            pub fn done(self) -> #last_staged<'a> { self.inner }
        }
    });

    for (i, tg) in groups.iter().enumerate() {
        let current: syn::Ident = if i == 0 {
            ordered_ident.clone()
        } else {
            ordered_stage(i - 1)
        };
        let next = ordered_stage(i);
        let method = syn::Ident::new(&tg.accessor_snake, span);
        let into_ident = syn::Ident::new(&format!("into_{}", tg.accessor_snake), span);
        let entry_ident = syn::Ident::new(&tg.entry_decoder_ident, span);
        let dim_helper = quote::format_ident!("__sbe_{}_dim", tg.accessor_snake);
        let doc = format!(
            " Visit every `{}` entry in wire order, then advance to the next tail.\n\n\
             The callback receives the entry and an [`sbe_rt::EntryInfo`] carrying\n\
             its index, the wire-declared count, and the acting block length.\n\
             Empty groups invoke it zero times.",
            tg.accessor_snake
        );
        let body = if tg.entries_have_tails {
            let complete_ident =
                syn::Ident::new(&format!("{}Complete", tg.entry_decoder_ident), span);
            quote::quote! {
                #[doc = #doc]
                ///
                /// These entries carry tails of their own, so the callback
                /// returns the entry's completion — that is where the next
                /// entry starts, so nothing is scanned twice.
                #[inline]
                pub fn #method<E, F>(self, mut f: F) -> Result<#next<'a>, E>
                where
                    E: From<sbe_rt::DecodeError>,
                    F: FnMut(#entry_ident<'a>, sbe_rt::EntryInfo) -> Result<#complete_ident<'a>, E>,
                {
                    let (count, block_length) = self.inner.#dim_helper()?;
                    let mut index = 0usize;
                    let inner = self.inner.#into_ident(|entry| {
                        let info = sbe_rt::EntryInfo { index, count, block_length };
                        index += 1;
                        f(entry, info)
                    })?;
                    Ok(#next { inner })
                }
            }
        } else {
            quote::quote! {
                #[doc = #doc]
                ///
                /// These entries have a fixed stride, so the callback returns
                /// `()` — there is no tail to complete.
                #[inline]
                pub fn #method<E, F>(self, mut f: F) -> Result<#next<'a>, E>
                where
                    E: From<sbe_rt::DecodeError>,
                    F: FnMut(#entry_ident<'a>, sbe_rt::EntryInfo) -> Result<(), E>,
                {
                    // No pre-read: the iterator itself carries the count and
                    // the stride, so this adds nothing to the staged walk.
                    let mut iter = self.inner.#into_ident()?;
                    let count = iter.remaining_entries();
                    let block_length = iter.entry_block_length();
                    let mut index = 0usize;
                    for entry in &mut iter {
                        f(entry, sbe_rt::EntryInfo { index, count, block_length })?;
                        index += 1;
                    }
                    let inner = iter.finish()?;
                    Ok(#next { inner })
                }
            }
        };
        ts.extend(quote::quote! { impl<'a> #current<'a> { #body } });
    }

    for (vi, vd) in vardata.iter().enumerate() {
        let i = groups.len() + vi;
        let current: syn::Ident = if i == 0 {
            ordered_ident.clone()
        } else {
            ordered_stage(i - 1)
        };
        let next = ordered_stage(i);
        let method = syn::Ident::new(&vd.accessor_snake, span);
        let into_ident = syn::Ident::new(&format!("into_{}", vd.accessor_snake), span);
        let doc = format!(
            " Read `{}` as bytes, then advance to the next tail.",
            vd.accessor_snake
        );
        ts.extend(quote::quote! {
            impl<'a> #current<'a> {
                #[doc = #doc]
                #[inline]
                pub fn #method<E, F>(self, f: F) -> Result<#next<'a>, E>
                where
                    E: From<sbe_rt::DecodeError>,
                    F: FnOnce(&'a [u8]) -> Result<(), E>,
                {
                    let (bytes, inner) = self.inner.#into_ident()?;
                    f(bytes)?;
                    Ok(#next { inner })
                }
            }
        });
        // Strict text callback, only where the schema declares an encoding.
        if super::runtime::text_encoding_kind(vd.character_encoding.as_deref()).is_some() {
            let str_method = syn::Ident::new(&format!("{}_as_str", vd.accessor_snake), span);
            let into_str = syn::Ident::new(&format!("into_{}_as_str", vd.accessor_snake), span);
            let str_doc = format!(
                " Read `{}` as `&str`, then advance to the next tail.\n\n\
                 Validation covers this field only — there is no whole-message\n\
                 text pass. Invalid text is an error, never a sentinel.",
                vd.accessor_snake
            );
            ts.extend(quote::quote! {
                impl<'a> #current<'a> {
                    #[doc = #str_doc]
                    #[inline]
                    pub fn #str_method<E, F>(self, f: F) -> Result<#next<'a>, E>
                    where
                        E: From<sbe_rt::DecodeError>,
                        F: FnOnce(&'a str) -> Result<(), E>,
                    {
                        let (text, inner) = self.inner.#into_str()?;
                        f(text)?;
                        Ok(#next { inner })
                    }
                }
            });
        }
    }

    ts
}

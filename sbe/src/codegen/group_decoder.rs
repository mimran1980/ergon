//! Repeating-group decoder codegen.
//!
//! `generate_group_decoder` emits the group decoder flyweight, its random-access
//! `&self` accessors, nested group decoders (recursively), domain-object entry
//! builders, and the concrete consuming entry-level tail stages.

use crate::ir::{ByteOrder, Presence, PrimitiveType};
use crate::structured_ir::{
    FieldType, MessageGroup, SchemaElements, get_dimension_info, get_vardata_info, is_bool_enum,
    rust_type,
};

use super::conversion_helpers::{
    enum_uses_null_as_option, field_has_conversion_free, find_domain_type,
    fixed_array_from_bulk_bytes,
};
use super::field_type::field_type_ident;
use super::generate_entry_consuming_stages;
use super::runtime::{
    constant_value_expr, doc_attr_tokens, emit_field_consts, to_pascal_case, to_snake_case,
};

/// Pure flyweight observers — discarding the return is almost always a mistake.
fn must_use_observer() -> proc_macro2::TokenStream {
    quote::quote! {
        #[must_use = "discarding this value is almost always a mistake"]
    }
}

pub(crate) fn generate_group_decoder(
    g: &MessageGroup,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    scoped_name: &str,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    enable_meta_attributes: bool,
    enable_dispatch: bool,
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
    enable_display_debug: bool,
) -> proc_macro2::TokenStream {
    let mut ts = proc_macro2::TokenStream::new();
    let mu = must_use_observer();
    let span = proc_macro2::Span::call_site();
    let name = scoped_name.to_string();
    let decoder_ident = quote::format_ident!("{}Decoder", name);
    let entry_decoder_ident = quote::format_ident!("{}EntryDecoder", name);
    let (dim_name, dim_size, bl_field, count_field) =
        get_dimension_info(elements, &g.dimension_type);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };
    let order_fn = quote::format_ident!("from_{}_bytes", order_suffix);
    let dim_name_ident = syn::Ident::new(&dim_name, proc_macro2::Span::call_site());
    let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), proc_macro2::Span::call_site());
    let block_len_lit = syn::LitInt::new(
        &g.effective_block_length().to_string(),
        proc_macro2::Span::call_site(),
    );
    let bl_field_ident = syn::Ident::new(&bl_field, proc_macro2::Span::call_site());
    let count_field_ident = syn::Ident::new(&count_field, proc_macro2::Span::call_site());
    let g_name_lit = syn::LitStr::new(&g.name, proc_macro2::Span::call_site());
    let total_tail = g.groups.len() + g.var_data.len();
    // Bulk decode is only safe when every non-constant entry field is
    // present in all supported versions (sinceVersion == 0) and required.
    let bulk_decode_eligible = g.has_fixed_stride()
        && super::runtime::encodable_at(g.since_version)
        && g.fields.iter().all(|f| {
            f.presence == Presence::Constant
                || (f.presence != Presence::Optional && f.since_version == 0)
        });
    // Unified extent rule: see `emit_readable_extent_body` in runtime.rs.
    let min_extent_arms = crate::codegen::runtime::emit_readable_extent_body(&g.fields);

    let fixed_extent_validation = if g.has_fixed_stride() {
        quote::quote! {
            let entries_length = count.checked_mul(block_length).ok_or(
                sbe_rt::DecodeError::BufferTooShort {
                    field: #g_name_lit,
                    needed: usize::MAX,
                    available: buf.len().saturating_sub(entries_start),
                },
            )?;
            if entries_length > buf.len().saturating_sub(entries_start) {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: #g_name_lit,
                    needed: entries_length,
                    available: buf.len().saturating_sub(entries_start),
                });
            }
        }
    } else {
        quote::quote! {}
    };

    // Dynamic groups have no constant stride, so the whole region cannot be
    // proven once at wrap time. Each private entry construction is instead
    // preceded by a proof of the acting fixed block at that position.
    //
    // How much an entry needs, though, depends only on `acting_version` and
    // `acting_block_length` — both fixed for the decoder's lifetime. It is
    // resolved once at wrap and stored, so the per-entry cost in the iteration
    // hot path is one subtraction and one comparison, not a re-run of the
    // version-branch chain in `min_readable_fixed_extent`.
    let (
        dyn_extent_field,
        dyn_extent_decl,
        dyn_extent_init,
        dyn_extent_reinit,
        dyn_extent_init_absent,
    ) = if g.has_dynamic_entries() {
        (
            quote::quote! { min_entry_extent: usize, },
            quote::quote! {
                let min_entry_extent = if block_length > min_fixed { block_length } else { min_fixed };
            },
            quote::quote! { min_entry_extent, },
            quote::quote! { min_entry_extent: attached.min_entry_extent, },
            quote::quote! { min_entry_extent: 0, },
        )
    } else {
        // A fixed-stride group proves its whole entry region at wrap time, so
        // it needs no per-entry extent and carries no field for one.
        (
            proc_macro2::TokenStream::new(),
            proc_macro2::TokenStream::new(),
            proc_macro2::TokenStream::new(),
            proc_macro2::TokenStream::new(),
            proc_macro2::TokenStream::new(),
        )
    };

    let dynamic_entry_extent_proof = quote::quote! {
        let __available = self.buf.len().saturating_sub(self.offset);
        if self.min_entry_extent > __available {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: #g_name_lit,
                needed: self.min_entry_extent,
                available: __available,
            });
        }
    };

    // A dynamic group whose entry fails to decode has lost its framing: every
    // later offset in the group is derived from the failed entry's length. The
    // first error is stored, yielded once, and then the group is finished —
    // it can neither yield another entry nor construct a later message stage.
    // Fixed-stride groups have a constant, already-proven stride, so they carry
    // no poison state and no extra field.
    let clear_poison = if g.has_dynamic_entries() {
        quote::quote! { self.poisoned = None; }
    } else {
        proc_macro2::TokenStream::new()
    };
    let (poison_field, poison_init) = if g.has_dynamic_entries() {
        (
            quote::quote! { poisoned: Option<sbe_rt::DecodeError>, },
            quote::quote! { poisoned: None, },
        )
    } else {
        (
            proc_macro2::TokenStream::new(),
            proc_macro2::TokenStream::new(),
        )
    };

    // Struct definition + wrap() + wrap_with_parent() + is_empty()
    if let Some(ref desc) = g.description {
        ts.extend(doc_attr_tokens(desc));
    } else {
        ts.extend(quote::quote! {
            #[doc = concat!("Group `", stringify!(#decoder_ident), "` decoder — iterate entries in wire order.")]
        });
    }
    // When entries have nested groups or var-data, there is no constant stride,
    // so O(1) random access is not available — use the iterator or skip_n().
    if g.has_dynamic_entries() {
        ts.extend(quote::quote! {
            #[doc = " This group has entries with nested groups or var-data —"]
            #[doc = " there is no constant stride, so `entry_at` (O(1) random"]
            #[doc = " access) is **not** available. Use the [`Iterator`]"]
            #[doc = " implementation, [`Self::scan_entry_at`], or"]
            #[doc = " [`Self::skip_n`] to advance positionally instead."]
        });
    }
    ts.extend(quote::quote! {
        pub struct #decoder_ident<'a, C: sbe_rt::GroupContext = sbe_rt::Detached> {
            buf: &'a [u8],
            offset: usize,
            count: usize,
            start: usize,
            total: usize,
            acting_version: u16,
            acting_block_length: usize,
            // Parent message body position + acting block length, so `finish()`
            // can reconstruct the next message decoder stage. Unused by
            // random-access entry accessors.
            parent_pos: usize,
            parent_block_length: usize,
            #poison_field
            #dyn_extent_field
            // Zero-sized: attachment is a type-level fact, not a runtime flag.
            _context: core::marker::PhantomData<C>,
        }

        impl<'a, C: sbe_rt::GroupContext> #decoder_ident<'a, C> {
            /// Proof-dependent constructor: like `wrap()` but remembers the
            /// parent message body position and acting block length so
            /// `finish()` can rebuild the next stage.
            ///
            /// Private to the generated module — a caller outside it cannot
            /// invent parent state and then `finish()` into a message stage
            /// that never existed.
            ///
            /// # Safety
            /// `parent_pos` and `parent_block_length` must describe the message
            /// body this group is genuinely nested in, and `offset` must be that
            /// message's real dimension-header offset for this group. The
            /// dimension header, the acting block length, and the group extent
            /// are still validated here and may be untrusted.
            #[inline]
            unsafe fn wrap_with_parent(
                buf: &'a [u8],
                offset: usize,
                acting_version: u16,
                parent_pos: usize,
                parent_block_length: usize,
            ) -> Result<#decoder_ident<'a, sbe_rt::Attached>, sbe_rt::DecodeError> {
                // Trust boundary: always validate dimension header fits in buffer
                if #dim_size_lit > buf.len().saturating_sub(offset) {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: #g_name_lit,
                        needed: #dim_size_lit,
                        available: buf.len().saturating_sub(offset),
                    });
                }
                let bytes: [u8; #dim_size_lit] = read_bytes::<#dim_size_lit>(buf, offset);
                let header = #dim_name_ident(bytes);
                let count = sbe_rt::checked_group_count(
                    "numInGroup",
                    header.#count_field_ident() as u64,
                )?;
                let block_length = sbe_rt::checked_header_usize(
                    "blockLength",
                    header.#bl_field_ident() as u64,
                )?;
                let entries_start = offset + #dim_size_lit;
                // SBE acting-version rule at the flyweight trust boundary: an
                // entry whose wire block length cannot hold the required fixed
                // fields active at this version is malformed, and a required
                // getter would otherwise perform an unchecked read past it.
                let min_fixed = <#decoder_ident<'_, sbe_rt::Detached>>::min_readable_fixed_extent(acting_version);
                if count > 0 && block_length < min_fixed {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: #g_name_lit,
                        needed: min_fixed,
                        available: block_length,
                    });
                }
                #dyn_extent_decl
                #fixed_extent_validation
                Ok(#decoder_ident {
                    buf,
                    offset: entries_start,
                    count,
                    start: entries_start,
                    total: count,
                    acting_version,
                    acting_block_length: block_length,
                    parent_pos,
                    parent_block_length,
                    #poison_init
                    #dyn_extent_init
                    _context: core::marker::PhantomData,
                })
            }

            /// Attached decoder for a group that is not in the acting version:
            /// zero entries, zero bytes, immediately complete.
            ///
            /// # Safety
            /// `parent_pos` and `parent_block_length` must describe the message
            /// body this group is nested in, and `offset` must be the byte
            /// position where this group would have started had it been present.
            #[inline]
            unsafe fn wrap_absent_parent(
                buf: &'a [u8],
                offset: usize,
                acting_version: u16,
                parent_pos: usize,
                parent_block_length: usize,
            ) -> #decoder_ident<'a, sbe_rt::Attached> {
                #decoder_ident {
                    buf,
                    offset,
                    count: 0,
                    start: offset,
                    total: 0,
                    acting_version,
                    acting_block_length: 0,
                    parent_pos,
                    parent_block_length,
                    #poison_init
                    #dyn_extent_init_absent
                    _context: core::marker::PhantomData,
                }
            }

            #mu
            #[inline]
            pub fn is_empty(&self) -> bool {
                self.count == 0
            }

            /// Wire-declared entries not yet consumed.
            ///
            /// O(1): `into_*` already read the SBE dimension header containing
            /// `numInGroup`. This does not promise that remaining entries will
            /// decode, so dynamic groups are not [`core::iter::ExactSizeIterator`].
            #mu
            #[inline]
            pub const fn remaining_entries(&self) -> usize {
                self.count
            }
        }

        impl<'a> #decoder_ident<'a, sbe_rt::Detached> {
            pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

            /// Minimum entry bytes needed to safely read every **required**
            /// fixed field present at `acting_version`.
            ///
            /// Version-aware, and not always the compiled
            /// `ENTRY_BLOCK_LENGTH`: a forward-compatible reader accepts a
            /// wire block length it does not recognise, but never one too
            /// small for the fields it will actually read.
            #[must_use = "this extent is the minimum readable body size; ignoring it skips a bounds check"]
            #[inline]
            pub const fn min_readable_fixed_extent(acting_version: u16) -> usize {
                #min_extent_arms
            }

            /// Wrap a standalone group at its dimension header, with bounds
            /// checks.
            ///
            /// This is the only public constructor. It validates the dimension
            /// header, rejects a wire block length too small to hold the
            /// required fixed fields active at `acting_version`, and — for
            /// fixed-stride groups — proves the whole entry region at once.
            ///
            /// The result is *detached*: it iterates, random-accesses, and
            /// rewinds, but has no parent message to complete into, so it has
            /// no `finish` / `skip_remaining`.
            #[inline]
            pub fn wrap(
                buf: &'a [u8],
                offset: usize,
                acting_version: u16,
            ) -> Result<#decoder_ident<'a, sbe_rt::Detached>, sbe_rt::DecodeError> {
                // SAFETY: a standalone group has no parent to prove; the zero
                // parent position can never be observed, because a detached
                // decoder cannot reach a message stage.
                let attached = unsafe {
                    <#decoder_ident<'a, sbe_rt::Attached>>::wrap_with_parent(
                        buf, offset, acting_version, 0, 0,
                    )?
                };
                Ok(#decoder_ident {
                    buf: attached.buf,
                    offset: attached.offset,
                    count: attached.count,
                    start: attached.start,
                    total: attached.total,
                    acting_version: attached.acting_version,
                    acting_block_length: attached.acting_block_length,
                    parent_pos: attached.parent_pos,
                    parent_block_length: attached.parent_block_length,
                    #poison_init
                    #dyn_extent_reinit
                    _context: core::marker::PhantomData,
                })
            }
        }
    });

    // remaining(), rewind()
    ts.extend(quote::quote! {
        impl<'a, C: sbe_rt::GroupContext> #decoder_ident<'a, C> {
            /// Entries not yet advanced (count), not a byte slice.
            /// For message-level byte tails use `get_metadata().remaining()`.
            /// Prefer [`Self::remaining_entries`] at call sites that mean
            /// group cardinality rather than a byte tail.
            #mu
            #[inline]
            pub const fn remaining(&self) -> usize {
                self.remaining_entries()
            }

            /// Dimension wrap after the caller has proven
            /// the dimension header (and, for fixed groups, the full entry
            /// region) is in-bounds. Prefer [`Self::wrap`] / [`Self::wrap_with_parent`].
            ///
            /// # Safety
            /// `offset + dimension_header_size` must not overflow and must be
            /// ≤ `buf.len()`. For fixed-block groups (no nested tail),
            /// `offset + dim + count * acting_block_length` must also fit. Entry
            /// accessors then use unchecked fixed-field reads under that proof.
            #[inline]
            pub(crate) unsafe fn wrap_trusted(
                buf: &'a [u8], offset: usize, acting_version: u16,
                parent_pos: usize, parent_block_length: usize,
            ) -> Result<Self, sbe_rt::DecodeError> {
                let bytes: [u8; #dim_size_lit] = unsafe { read_bytes_unchecked::<#dim_size_lit>(buf, offset) };
                let header = #dim_name_ident(bytes);
                let count = sbe_rt::checked_group_count(
                    "numInGroup",
                    header.#count_field_ident() as u64,
                )?;
                let block_length = sbe_rt::checked_header_usize(
                    "blockLength",
                    header.#bl_field_ident() as u64,
                )?;
                let min_fixed = <#decoder_ident<'_, sbe_rt::Detached>>::min_readable_fixed_extent(acting_version);
                #dyn_extent_decl
                Ok(Self {
                    buf, offset: offset + #dim_size_lit, count, start: offset + #dim_size_lit,
                    total: count, acting_version, acting_block_length: block_length,
                    parent_pos, parent_block_length,
                    #poison_init
                    #dyn_extent_init
                    _context: core::marker::PhantomData,
                })
            }

            /// Restart iteration from the group's proven start.
            ///
            /// This is the one operation that clears a poisoned group: the
            /// start offset was validated at wrap time, so retrying from there
            /// is sound even after an entry failed.
            #[inline]
            pub fn rewind(&mut self) -> &mut Self {
                self.offset = self.start;
                self.count = self.total;
                #clear_poison
                self
            }
        }
    });

    // skip_n()
    if g.has_fixed_stride() {
        // no tails: tail_offset_0 is a no-op
        // (count-based total_tail remains for index use below)
        // Build field read expressions for bulk_decode (reverse of
        // add_struct's struct_write in generate_group_encoder).
        let entry_struct_ident = syn::Ident::new(&format!("{}Entry", name), span);
        let mut field_reads = proc_macro2::TokenStream::new();
        for f in &g.fields {
            if f.presence == Presence::Constant {
                continue;
            }
            let f_name = syn::Ident::new(&to_snake_case(&f.name), span);
            let f_offset = syn::Index::from(f.offset);
            let f_size = syn::LitInt::new(&f.field_type.size().to_string(), span);
            match &f.field_type {
                FieldType::Composite { .. } => {
                    let f_ty = field_type_ident(&f.field_type, span);
                    field_reads.extend(quote::quote! {
                        #f_name: {
                            let mut bytes = [0u8; #f_size];
                            bytes.copy_from_slice(&self.buf[offset + #f_offset..offset + #f_offset + #f_size]);
                            #f_ty(bytes)
                        },
                    });
                }
                FieldType::Enum { encoding_type, .. } | FieldType::Set { encoding_type, .. } => {
                    let r_ty = syn::Ident::new(&rust_type(*encoding_type), span);
                    field_reads.extend(quote::quote! {
                        #f_name: {
                            let raw = #r_ty::#order_fn(
                                self.buf[offset + #f_offset..offset + #f_offset + #f_size].try_into().unwrap()
                            );
                            raw.into()
                        },
                    });
                }
                FieldType::Primitive(pt, Some(len)) => {
                    let len_lit = syn::LitInt::new(&len.to_string(), span);
                    let r_ty = syn::Ident::new(&rust_type(*pt), span);
                    field_reads.extend(quote::quote! {
                        #f_name: {
                            let mut arr = [0 as #r_ty; #len_lit];
                            let mut i = 0usize;
                            while i < #len_lit {
                                let elem_offset = offset + #f_offset + i * core::mem::size_of::<#r_ty>();
                                arr[i] = #r_ty::#order_fn(
                                    self.buf[elem_offset..][..core::mem::size_of::<#r_ty>()].try_into().unwrap()
                                );
                                i += 1;
                            }
                            arr
                        },
                    });
                }
                FieldType::Primitive(pt, None) => {
                    let r_ty = syn::Ident::new(&rust_type(*pt), span);
                    field_reads.extend(quote::quote! {
                        #f_name: #r_ty::#order_fn(
                            self.buf[offset + #f_offset..offset + #f_offset + #f_size].try_into().unwrap()
                        ),
                    });
                }
            }
        }

        // Eager owned rows can only be emitted when the row type can represent
        // every valid acting version: flat, required, since-v0 fields. An
        // optional or versioned field has no representation in a plain struct,
        // so a bulk row would have to fabricate a value for something absent.
        // Groups the configured `encode_version` drops are excluded: their
        // encoder is not generated, and it owns the `{Group}Entry` struct that
        // `bulk_decode` names. Under default generation every group qualifies.
        let bulk_methods = if bulk_decode_eligible {
            quote::quote! {
                /// Bulk-decode all remaining entries into a caller-owned `Vec`.
                /// Zero-allocation after warm-up — the caller reuses the
                /// destination buffer across messages.
                #[inline]
                pub fn bulk_decode_into(
                    &mut self,
                    dst: &mut Vec<#entry_struct_ident>,
                ) -> Result<usize, sbe_rt::DecodeError> {
                    let needed = self.count.checked_mul(self.acting_block_length)
                        .ok_or(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: usize::MAX,
                            available: 0,
                        })?;
                    if self.offset + needed > self.buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed,
                            available: self.buf.len().saturating_sub(self.offset),
                        });
                    }
                    let cap = self.count;
                    dst.clear();
                    dst.reserve(cap);
                    for _ in 0..cap {
                        let offset = self.offset;
                        self.offset += self.acting_block_length;
                        dst.push(#entry_struct_ident { #field_reads });
                    }
                    self.count = 0;
                    Ok(cap)
                }

                /// Bulk-decode all remaining entries into a new `Vec`.
                /// Convenience wrapper around [`Self::bulk_decode_into`].
                /// One bounds check for the whole batch — faster than
                /// iterating with [`Iterator::next`] when materialising
                /// the entire group (DTO construction, snapshots).
                #[inline]
                pub fn bulk_decode(&mut self) -> Result<Vec<#entry_struct_ident>, sbe_rt::DecodeError> {
                    let mut out = Vec::new();
                    self.bulk_decode_into(&mut out)?;
                    Ok(out)
                }
            }
        } else {
            proc_macro2::TokenStream::new()
        };

        ts.extend(quote::quote! {
            impl<'a, C: sbe_rt::GroupContext> #decoder_ident<'a, C> {
                #[inline]
                pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
                    if n > self.count {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: n.saturating_mul(self.acting_block_length),
                            available: self.count.saturating_mul(self.acting_block_length),
                        });
                    }
                    self.offset += n.saturating_mul(self.acting_block_length);
                    self.count -= n;
                    Ok(())
                }
                #bulk_methods

            }
        });
    } else {
        ts.extend(quote::quote! {
            impl<'a, C: sbe_rt::GroupContext> #decoder_ident<'a, C> {
                #[inline]
                pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
                    if n > self.count {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: n.saturating_mul(<#decoder_ident<'_, sbe_rt::Detached>>::ENTRY_BLOCK_LENGTH),
                            available: self.count.saturating_mul(<#decoder_ident<'_, sbe_rt::Detached>>::ENTRY_BLOCK_LENGTH),
                        });
                    }
                    for _ in 0..n {
                        #dynamic_entry_extent_proof
                        // SAFETY: acting fixed block proven in-bounds directly
                        // above; encoded_length re-validates the dynamic tail.
                        let entry = unsafe {
                            #entry_decoder_ident::wrap(
                                self.buf,
                                self.offset,
                                self.acting_block_length,
                                self.acting_version,
                            )
                        };
                        self.offset += entry.encoded_length()?;
                        self.count -= 1;
                    }
                    Ok(())
                }
            }
        });
    }

    // Random access is direct for fixed entries. Entries with nested tails
    // must be walked because their encoded lengths are not a constant stride.
    if g.has_fixed_stride() {
        ts.extend(quote::quote! {
            impl<'a, C: sbe_rt::GroupContext> #decoder_ident<'a, C> {
                #[inline]
                pub fn entry_at(&self, idx: usize) -> Result<#entry_decoder_ident<'a>, sbe_rt::DecodeError> {
                    if idx >= self.total {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: idx.saturating_add(1).saturating_mul(self.acting_block_length),
                            available: self.total.saturating_mul(self.acting_block_length),
                        });
                    }
                    let byte_offset = idx.checked_mul(self.acting_block_length).ok_or(
                        sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: usize::MAX,
                            available: self.buf.len().saturating_sub(self.start),
                        },
                    )?;
                    let offset = self.start.checked_add(byte_offset).ok_or(
                        sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: usize::MAX,
                            available: self.buf.len().saturating_sub(self.start),
                        },
                    )?;
                    if self.acting_block_length > self.buf.len().saturating_sub(offset) {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: self.acting_block_length,
                            available: self.buf.len().saturating_sub(offset),
                        });
                    }
                    // SAFETY: acting block at offset proven above.
                    Ok(unsafe {
                        #entry_decoder_ident::wrap(
                            self.buf,
                            offset,
                            self.acting_block_length,
                            self.acting_version,
                        )
                    })
                }
            }
        });
    } else {
        ts.extend(quote::quote! {
            impl<'a, C: sbe_rt::GroupContext> #decoder_ident<'a, C> {
                #[inline]
                pub fn scan_entry_at(&self, idx: usize) -> Result<#entry_decoder_ident<'a>, sbe_rt::DecodeError> {
                    if idx >= self.total {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: idx.saturating_add(1).saturating_mul(<#decoder_ident<'_, sbe_rt::Detached>>::ENTRY_BLOCK_LENGTH),
                            available: self.total.saturating_mul(<#decoder_ident<'_, sbe_rt::Detached>>::ENTRY_BLOCK_LENGTH),
                        });
                    }
                    let mut offset = self.start;
                    for _ in 0..idx {
                        offset = #entry_decoder_ident::skip(
                            self.buf,
                            offset,
                            self.acting_block_length,
                            self.acting_version,
                        )?;
                    }
                    let available = self.buf.len().saturating_sub(offset);
                    if self.min_entry_extent > available {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: self.min_entry_extent,
                            available,
                        });
                    }
                    // SAFETY: skip walked prior entries and the acting fixed
                    // block at `offset` is proven above; encoded_length
                    // validates the dynamic tail.
                    let entry = unsafe {
                        #entry_decoder_ident::wrap(
                            self.buf,
                            offset,
                            self.acting_block_length,
                            self.acting_version,
                        )
                    };
                    entry.encoded_length()?;
                    Ok(entry)
                }
            }
        });
    }

    if g.has_fixed_stride() {
        ts.extend(quote::quote! {
            impl<'a, C: sbe_rt::GroupContext> Iterator for #decoder_ident<'a, C> {
                type Item = #entry_decoder_ident<'a>;

                #[inline]
                fn next(&mut self) -> Option<Self::Item> {
                    if self.count == 0 {
                        return None;
                    }
                    // SAFETY: wrap_with_parent validated dim + count*block_length
                    // for fixed groups; offset walks that region one block at a time.
                    let entry = unsafe {
                        #entry_decoder_ident::wrap(
                            self.buf,
                            self.offset,
                            self.acting_block_length,
                            self.acting_version,
                        )
                    };
                    self.offset += self.acting_block_length;
                    self.count -= 1;
                    Some(entry)
                }
            }

            impl<'a, C: sbe_rt::GroupContext> ExactSizeIterator for #decoder_ident<'a, C> {
                #[inline]
                fn len(&self) -> usize {
                    self.count
                }
            }
        });
    } else {
        ts.extend(quote::quote! {
            impl<'a, C: sbe_rt::GroupContext> Iterator for #decoder_ident<'a, C> {
                type Item = Result<#entry_decoder_ident<'a>, sbe_rt::DecodeError>;

                #[inline]
                fn next(&mut self) -> Option<Self::Item> {
                    // Poisoned: the error was already yielded once. Every later
                    // offset in this group came from the entry that failed, so
                    // there is nothing truthful left to produce.
                    if self.poisoned.is_some() || self.count == 0 {
                        return None;
                    }
                    // Extent resolved once at wrap: the hot path is a
                    // subtraction and a comparison.
                    let available = self.buf.len().saturating_sub(self.offset);
                    if self.min_entry_extent > available {
                        let error = sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: self.min_entry_extent,
                            available,
                        };
                        // Zero the count with the poison so `remaining()`,
                        // `is_empty()`, and `size_hint()` all agree with what
                        // iteration will actually yield. `rewind()` restores it
                        // from `total`, so nothing is lost.
                        self.poisoned = Some(error);
                        self.count = 0;
                        return Some(Err(error));
                    }
                    // SAFETY: acting fixed block at offset proven directly above;
                    // encoded_length() re-validates the dynamic tail before
                    // advancing.
                    let entry = unsafe {
                        #entry_decoder_ident::wrap(
                            self.buf,
                            self.offset,
                            self.acting_block_length,
                            self.acting_version,
                        )
                    };
                    let size = match entry.encoded_length() {
                        Ok(s) => s,
                        Err(e) => {
                            self.poisoned = Some(e);
                            self.count = 0;
                            return Some(Err(e));
                        }
                    };
                    self.offset += size;
                    self.count -= 1;
                    Some(Ok(entry))
                }

                /// Conservative: the declared count is an upper bound, but a
                /// malformed entry can end iteration early, so the lower bound
                /// is zero. This group is deliberately **not**
                /// `ExactSizeIterator` — a size-based allocation must not trust
                /// a count the wire has not yet justified.
                ///
                /// Poisoning zeroes the count, so this collapses to `(0, Some(0))`
                /// on a broken group without a separate branch.
                #[inline]
                fn size_hint(&self) -> (usize, Option<usize>) {
                    (0, Some(self.count))
                }
            }

            /// Exhausted by count or by poison, `next()` keeps returning `None`.
            ///
            /// [`Self::rewind`] is the documented exception: it is not `next`,
            /// and it deliberately restarts a *new* iteration from the start
            /// offset proven at wrap time. Do not call it partway through an
            /// adaptor that has cached this fuse.
            impl<'a, C: sbe_rt::GroupContext> core::iter::FusedIterator for #decoder_ident<'a, C> {}
        });
    }

    let mut entry_body = proc_macro2::TokenStream::new();
    // Entry decoders keep a one-shot extent cache in every lane: the group
    // iterator computes each entry's end to advance, and the last var-data
    // accessor reuses it instead of re-reading its length header. Dropping it
    // makes full-message decode lose to sbe-tool, which the
    // `decode_full_message` gate measures. The message-level progressive cache
    // is a separate, opt-in lane (`memoized_decoder.rs`).
    let entry_cache_init = quote::quote! { tail_end: core::cell::Cell::new(None), };
    let entry_cache_field = quote::quote! {
        /// One-shot entry-extent cache: filled by `encoded_length`, reused by
        /// the last var-data accessor. `Cell` keeps `&self` getters and makes
        /// the entry `Send` + `!Sync`.
        tail_end: core::cell::Cell<Option<usize>>,
    };

    // wrap() method header. Entries with tail components carry a one-shot
    // tail-end cache: the group iterator computes the entry extent to
    // advance, and var-data accessors reuse it instead of re-reading the
    // length header.
    if total_tail == 0 {
        entry_body.extend(quote::quote! {
            pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

            /// Private entry wrap after the group iterator (or equivalent)
            /// has proven the acting fixed block is in-bounds at `offset`.
            ///
            /// # Safety
            /// `offset + max(acting_block_length, ENTRY_BLOCK_LENGTH)` (and any
            /// field offset used by accessors) must not overflow and must be
            /// ≤ `buf.len()`. Fixed-field getters may then use unchecked reads.
            #[inline]
            unsafe fn wrap(
                buf: &'a [u8],
                offset: usize,
                acting_block_length: usize,
                acting_version: u16,
            ) -> Self {
                Self {
                    buf,
                    offset,
                    acting_version,
                    acting_block_length,
                }
            }
        });
    } else {
        entry_body.extend(quote::quote! {
            pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

            /// Private entry wrap after the group iterator has proven extents.
            ///
            /// # Safety
            /// Fixed block at `offset` and every dynamic tail extent this entry
            /// will traverse must be fully in-bounds in `buf`.
            #[inline]
            unsafe fn wrap(
                buf: &'a [u8],
                offset: usize,
                acting_block_length: usize,
                acting_version: u16,
            ) -> Self {
                Self {
                    buf,
                    offset,
                    acting_version,
                    acting_block_length,
                    #entry_cache_init
                }
            }
        });
    }

    for f in &g.fields {
        let f_name = to_snake_case(&f.name);
        // In converter mode, Decimal-composite-backed raw entry accessors are
        // suffixed _wire when a conversion is configured (same rule as
        // message-level fields).
        let wire_name = field_has_conversion_free(f, conversions).then(|| format!("{f_name}_wire"));
        let accessor_name = wire_name.as_deref().unwrap_or(&f_name);
        let f_name_ident = syn::Ident::new(accessor_name, proc_macro2::Span::call_site());
        let raw_ident = syn::Ident::new(&format!("raw_{}", f_name), proc_macro2::Span::call_site());
        let offset_lit = syn::LitInt::new(&f.offset.to_string(), proc_macro2::Span::call_site());
        let f_name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());

        if let Some(ref desc) = f.description {
            entry_body.extend(doc_attr_tokens(desc));
        }

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_type = rust_type(*prim);
                let r_type_ty: syn::Type = syn::parse_str(r_type).unwrap();
                let prim_size = prim.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                if f.presence == Presence::Constant {
                    if let Some(ref val) = f.constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            let val_lit = syn::LitStr::new(val, proc_macro2::Span::call_site());
                            entry_body.extend(quote::quote! {
                                #mu
                                #[inline]
                                pub const fn #f_name_ident(&self) -> &'static str {
                                    #val_lit
                                }
                            });
                        } else {
                            let expr = constant_value_expr(*prim, val);
                            let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                            entry_body.extend(quote::quote! {
                                #mu
                                #[inline]
                                pub const fn #f_name_ident(&self) -> #r_type_ty {
                                    #expr_parsed
                                }
                            });
                        }
                    }
                } else if let Some(len) = length {
                    let len_lit =
                        syn::LitInt::new(&len.to_string(), proc_macro2::Span::call_site());

                    // Build unrolled element parses via direct constant indexing of a
                    // bulk-read local `all` array. One bulk read (single bounds check
                    // via read_bytes) + direct constant indexing (no per-element
                    // bounds check). Matching the message-level decoder pattern.
                    let total_size_lit = syn::LitInt::new(
                        &(prim_size * len).to_string(),
                        proc_macro2::Span::call_site(),
                    );
                    let offset_end_lit = syn::LitInt::new(
                        &(f.offset + prim_size * len).to_string(),
                        proc_macro2::Span::call_site(),
                    );
                    let since_lit = syn::LitInt::new(
                        &f.since_version.to_string(),
                        proc_macro2::Span::call_site(),
                    );
                    let elements =
                        fixed_array_from_bulk_bytes(&r_type_ty, *prim, prim_size, *len, &order_fn);
                    // Skip `acting_version < 0` (always false for u16).
                    let version_guard = if f.since_version > 0 {
                        quote::quote! {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return [0 as #r_type_ty; #len_lit];
                            }
                        }
                    } else {
                        quote::quote! {
                            if #offset_end_lit > self.acting_block_length {
                                return [0 as #r_type_ty; #len_lit];
                            }
                        }
                    };
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #f_name_ident(&self) -> [#r_type_ty; #len_lit] {
                            #version_guard
                            let offset = self.offset + #offset_lit;
                            let all: [u8; #total_size_lit] = unsafe { read_bytes_unchecked::<#total_size_lit>(self.buf, offset) };
                            #elements
                        }
                    });
                } else if f.presence == Presence::Optional {
                    let null_val = f.null_value.unwrap_or(0);
                    let null_check =
                        if *prim == PrimitiveType::Float || *prim == PrimitiveType::Double {
                            let _ = null_val;
                            "val.is_nan()".to_string()
                        } else {
                            format!("val == {}_u64 as {}", null_val, r_type)
                        };
                    let null_check_expr: syn::Expr = syn::parse_str(&null_check).unwrap();
                    let offset_end_lit = syn::LitInt::new(
                        &(f.offset + prim_size).to_string(),
                        proc_macro2::Span::call_site(),
                    );
                    let since_lit = syn::LitInt::new(
                        &f.since_version.to_string(),
                        proc_macro2::Span::call_site(),
                    );

                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #f_name_ident(&self) -> Option<#r_type_ty> {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return None;
                            }
                            let offset = self.offset + #offset_lit;
                            let val = #r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) });
                            if #null_check_expr {
                                None
                            } else {
                                Some(val)
                            }
                        }
                    });
                } else if f.since_version > 0 {
                    let offset_end_lit = syn::LitInt::new(
                        &(f.offset + prim_size).to_string(),
                        proc_macro2::Span::call_site(),
                    );
                    let since_lit = syn::LitInt::new(
                        &f.since_version.to_string(),
                        proc_macro2::Span::call_site(),
                    );
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #f_name_ident(&self) -> Option<#r_type_ty> {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return None;
                            }
                            let offset = self.offset + #offset_lit;
                            Some(#r_type_ty::#order_fn(
                                unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }
                            ))
                        }
                    });
                } else {
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #f_name_ident(&self) -> #r_type_ty {
                            let offset = self.offset + #offset_lit;
                            #r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) })
                        }
                    });
                }
            }
            FieldType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target_name = to_pascal_case(comp_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let comp_size_lit =
                    syn::LitInt::new(&comp_size.to_string(), proc_macro2::Span::call_site());
                let target_decoder_name = syn::Ident::new(
                    &format!("{}Decoder", target_name),
                    proc_macro2::Span::call_site(),
                );

                let offset_end_lit = syn::LitInt::new(
                    &(f.offset + comp_size).to_string(),
                    proc_macro2::Span::call_site(),
                );
                let since_lit =
                    syn::LitInt::new(&f.since_version.to_string(), proc_macro2::Span::call_site());

                // Default: flyweight (zero-copy).
                if f.since_version > 0 {
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #f_name_ident(&self) -> Option<#target_decoder_name<'_>> {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return None;
                            }
                            let offset = self.offset + #offset_lit;
                            Some(#target_decoder_name { buf: self.buf, offset: offset })
                        }
                    });
                } else {
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #f_name_ident(&self) -> #target_decoder_name<'_> {
                            let offset = self.offset + #offset_lit;
                            #target_decoder_name { buf: self.buf, offset: offset }
                        }
                    });
                }

                let as_struct_ident =
                    syn::Ident::new(&format!("{}_value", f_name), proc_macro2::Span::call_site());
                if f.since_version > 0 {
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #as_struct_ident(&self) -> Option<#target_ident> {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return None;
                            }
                            let offset = self.offset + #offset_lit;
                            Some(#target_ident(
                                unsafe { read_bytes_unchecked::<#comp_size_lit>(self.buf, offset) }
                            ))
                        }

                        #mu
                        #[inline]
                        pub fn #raw_ident(&self) -> Option<#target_ident> {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return None;
                            }
                            let offset = self.offset + #offset_lit;
                            Some(#target_ident(
                                unsafe { read_bytes_unchecked::<#comp_size_lit>(self.buf, offset) }
                            ))
                        }
                    });
                } else {
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #as_struct_ident(&self) -> #target_ident {
                            let offset = self.offset + #offset_lit;
                            #target_ident(unsafe { read_bytes_unchecked::<#comp_size_lit>(self.buf, offset) })
                        }

                        #mu
                        #[inline]
                        pub const fn #raw_ident(&self) -> #target_ident {
                            let offset = self.offset + #offset_lit;
                            let mut bytes = [0u8; #comp_size_lit];
                            bytes.copy_from_slice(unsafe { core::slice::from_raw_parts(self.buf.as_ptr().add(offset), #comp_size_lit) });
                            #target_ident(bytes)
                        }
                    });
                }

                // no lazy alias, base accessor is canonical; delete branch if lazy aliases never return
            }
            FieldType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(enum_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                let offset_end_lit = syn::LitInt::new(
                    &(f.offset + prim_size).to_string(),
                    proc_macro2::Span::call_site(),
                );
                let since_lit =
                    syn::LitInt::new(&f.since_version.to_string(), proc_macro2::Span::call_site());
                if f.since_version > 0 {
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #f_name_ident(&self) -> Option<#target_ident> {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return None;
                            }
                            let offset = self.offset + #offset_lit;
                            Some(#target_ident::from_raw(#r_type_ty::#order_fn(
                                unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }
                            )))
                        }

                        #mu
                        #[inline]
                        pub fn #raw_ident(&self) -> Option<#r_type_ty> {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return None;
                            }
                            let offset = self.offset + #offset_lit;
                            Some(#r_type_ty::#order_fn(
                                unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }
                            ))
                        }
                    });
                } else {
                    let raw_getter = quote::quote! {
                        #mu
                        #[inline]
                        pub const fn #raw_ident(&self) -> #r_type_ty {
                            let offset = self.offset + #offset_lit;
                            let mut bytes = [0u8; #prim_size_lit];
                            bytes.copy_from_slice(unsafe { core::slice::from_raw_parts(self.buf.as_ptr().add(offset), #prim_size_lit) });
                            #r_type_ty::#order_fn(bytes)
                        }
                    };
                    if enum_uses_null_as_option(enum_name, null_as_option, all_enums_as_option) {
                        entry_body.extend(quote::quote! {
                            /// Returns [`None`] when the wire discriminant equals
                            /// [`#target_ident::NullVal`]; [`Some`] otherwise.
                            #mu
                            #[inline]
                            pub fn #f_name_ident(&self) -> Option<#target_ident> {
                                let offset = self.offset + #offset_lit;
                                let raw = #r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) });
                                #target_ident::from_raw(raw).as_option()
                            }
                        });
                        entry_body.extend(raw_getter);
                    } else {
                        entry_body.extend(quote::quote! {
                            #mu
                            #[inline]
                            pub fn #f_name_ident(&self) -> #target_ident {
                                let offset = self.offset + #offset_lit;
                                #target_ident::from_raw(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }))
                            }
                        });
                        entry_body.extend(raw_getter);
                    }
                }

                if crate::structured_ir::is_bool_enum(elements, enum_name) {
                    // Group entries name every boolean accessor `try_*`, unlike
                    // message level where an `Option`-returning one drops the
                    // prefix. Consumers depend on these names, so the domain
                    // DTO adapts to the location rather than the names moving.
                    let bool_ident = quote::format_ident!("try_{}_bool", f_name);
                    let f_name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
                    // `null_as_option` makes the raw enum accessor return
                    // `Option<T>`; `.as_bool()` cannot be called on that
                    // directly. `NullVal` stays rejected either way.
                    let plain_bool_read =
                        if enum_uses_null_as_option(enum_name, null_as_option, all_enums_as_option)
                        {
                            quote::quote! { self.#f_name_ident().and_then(|v| v.as_bool()) }
                        } else {
                            quote::quote! { self.#f_name_ident().as_bool() }
                        };
                    // ── Boolean matrix for group entries (same contract as message-level) ──
                    // `presence="optional"` alone does NOT make the raw enum
                    // accessor `Option`: an optional enum carries `NullVal` as
                    // a variant. Only version-gating (or `null_as_option`)
                    // does. Treating optional as `Option` here matched on a
                    // plain `BooleanType` and did not compile.
                    // Only version-gating makes the boolean accessor
                    // `Option`-returning, at group-entry level exactly as at
                    // message level. `presence="optional"` does not (an
                    // optional enum carries `NullVal` as a variant) and neither
                    // does `null_as_option` — the boolean accessor still
                    // rejects `NullVal`; `plain_bool_read` unwraps the
                    // `Option<T>` raw accessor that option produces.
                    if f.since_version > 0 {
                        // Required but not yet present → None means
                        // absent from the acting version. A present
                        // non-boolean value → InvalidBoolean.
                        entry_body.extend(quote::quote! {
                            #[inline]
                            pub fn #bool_ident(&self) -> Result<Option<bool>, sbe_rt::DecodeError> {
                                match self.#f_name_ident() {
                                    None => Ok(None),
                                    Some(v) => v.as_bool().map(Some).ok_or(
                                        sbe_rt::DecodeError::InvalidBoolean {
                                            field: #f_name_lit,
                                            discriminant: v as u64,
                                        }
                                    ),
                                }
                            }
                        });
                    } else {
                        // Required since-v0: the raw value always
                        // returns a discriminant; NullVal and unknown
                        // are InvalidBoolean.
                        entry_body.extend(quote::quote! {
                            #[inline]
                            pub fn #bool_ident(&self) -> Result<bool, sbe_rt::DecodeError> {
                                #plain_bool_read.ok_or(
                                    sbe_rt::DecodeError::InvalidBoolean {
                                        field: #f_name_lit,
                                        discriminant: self.#raw_ident() as u64,
                                    }
                                )
                            }
                        });
                    }
                }
            }
            FieldType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(set_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                let offset_end_lit = syn::LitInt::new(
                    &(f.offset + prim_size).to_string(),
                    proc_macro2::Span::call_site(),
                );
                let since_lit =
                    syn::LitInt::new(&f.since_version.to_string(), proc_macro2::Span::call_site());
                if f.since_version > 0 {
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #f_name_ident(&self) -> Option<#target_ident> {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return None;
                            }
                            let offset = self.offset + #offset_lit;
                            Some(#target_ident(#r_type_ty::#order_fn(
                                unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }
                            )))
                        }

                        #mu
                        #[inline]
                        pub fn #raw_ident(&self) -> Option<#r_type_ty> {
                            if self.acting_version < #since_lit
                                || #offset_end_lit > self.acting_block_length
                            {
                                return None;
                            }
                            let offset = self.offset + #offset_lit;
                            Some(#r_type_ty::#order_fn(
                                unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }
                            ))
                        }
                    });
                } else {
                    entry_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #f_name_ident(&self) -> #target_ident {
                            let offset = self.offset + #offset_lit;
                            #target_ident(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }))
                        }

                        #mu
                        #[inline]
                        pub const fn #raw_ident(&self) -> #r_type_ty {
                            let offset = self.offset + #offset_lit;
                            let mut bytes = [0u8; #prim_size_lit];
                            bytes.copy_from_slice(unsafe { core::slice::from_raw_parts(self.buf.as_ptr().add(offset), #prim_size_lit) });
                            #target_ident(#r_type_ty::#order_fn(bytes)).0
                        }
                    });
                }
            }
        }
        if enable_meta_attributes {
            entry_body.extend(emit_field_consts(f));
        }
    }

    let nested_entry_skip: Vec<syn::Ident> = g
        .groups
        .iter()
        .map(|ng| {
            let ng_pascal = format!("{}{}", name, to_pascal_case(&ng.name));
            quote::format_ident!("{}EntryDecoder", ng_pascal)
        })
        .collect();
    entry_body.extend(super::tail_cache::emit_tail_offsets(
        &g.groups,
        &g.var_data,
        elements,
        &nested_entry_skip,
        quote::quote! {
            if self.acting_block_length > self.buf.len().saturating_sub(self.offset) {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "group entry",
                    needed: self.acting_block_length,
                    available: self.buf.len().saturating_sub(self.offset),
                });
            }
            Ok(self.offset + self.acting_block_length)
        },
        quote::quote! { self.offset },
    ));

    // Nested group accessors — scope under parent group name
    let mut ng_idx = 0usize;
    for ng in &g.groups {
        let ng_pascal = format!("{}{}", name, to_pascal_case(&ng.name));
        let ng_decoder_ident = quote::format_ident!("{}Decoder", ng_pascal);
        let ng_snake = to_snake_case(&ng.name);
        let ng_snake_ident = syn::Ident::new(&ng_snake, proc_macro2::Span::call_site());
        let ng_idx_lit = syn::LitInt::new(&ng_idx.to_string(), proc_macro2::Span::call_site());

        let tail_ng_fn = quote::format_ident!("tail_offset_{}", ng_idx);
        let ng_snake_str = ng_snake.clone();
        let version_check = if ng.since_version > 0 {
            let since_lit = syn::LitInt::new(
                &ng.since_version.to_string(),
                proc_macro2::Span::call_site(),
            );
            quote::quote! {
                if self.acting_version < #since_lit {
                    return Err(sbe_rt::DecodeError::FieldNotInVersion {
                        field: #ng_snake_str,
                        wire_version: self.acting_version,
                        since_version: #since_lit,
                    });
                }
            }
        } else {
            proc_macro2::TokenStream::new()
        };
        // A warm entry cache proves the whole entry extent was validated, so
        // the nested dim header is known in-bounds and `wrap_trusted` is sound
        // here. The two modes spell "warm" differently; both are one load.
        let entry_extent_known = quote::quote! { self.tail_end.get().is_some() };
        let cached_first_tail = if ng_idx == 0 {
            quote::quote! {
                // `Iterator::next` cached the complete validated entry extent,
                // so this first-tail offset cannot overflow or exceed `buf`.
                if #entry_extent_known {
                    let offset = self.offset + self.acting_block_length;
                    // SAFETY: a warm entry cache proves the nested dim is in-bounds.
                    return unsafe {
                        #ng_decoder_ident::wrap_trusted(
                            self.buf, offset, self.acting_version, 0, 0,
                        )
                    };
                }
            }
        } else {
            quote::quote! {}
        };
        let trusted_ng_wrap = quote::quote! {
            if #entry_extent_known {
                // SAFETY: tail_offset_* validated the nested dim header region.
                return unsafe {
                    #ng_decoder_ident::wrap_trusted(
                        self.buf, offset, self.acting_version, 0, 0,
                    )
                };
            }
        };
        entry_body.extend(quote::quote! {
            #[inline]
            pub fn #ng_snake_ident(&self) -> Result<#ng_decoder_ident<'a>, sbe_rt::DecodeError> {
                #version_check
                #cached_first_tail
                let offset = self.#tail_ng_fn()?;
                #trusted_ng_wrap
                #ng_decoder_ident::wrap(self.buf, offset, self.acting_version)
            }
        });
        ng_idx += 1;
    }

    let mut nvd_idx = g.groups.len();
    for vd in &g.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let type_pascal_ident = syn::Ident::new(&type_pascal, proc_macro2::Span::call_site());
        let len_field_ident = syn::Ident::new(&len_field, proc_macro2::Span::call_site());
        let prefix_size_lit =
            syn::LitInt::new(&prefix_size.to_string(), proc_macro2::Span::call_site());
        let vd_snake = to_snake_case(&vd.name);
        let vd_snake_ident = syn::Ident::new(&vd_snake, proc_macro2::Span::call_site());
        let tail_nvd_fn = quote::format_ident!("tail_offset_{}", nvd_idx);
        let vd_snake_str = vd_snake.clone();
        let version_check = if vd.since_version > 0 {
            let since_lit = syn::LitInt::new(
                &vd.since_version.to_string(),
                proc_macro2::Span::call_site(),
            );
            quote::quote! {
                if self.acting_version < #since_lit {
                    return Err(sbe_rt::DecodeError::FieldNotInVersion {
                        field: #vd_snake_str,
                        wire_version: self.acting_version,
                        since_version: #since_lit,
                    });
                }
            }
        } else {
            proc_macro2::TokenStream::new()
        };
        if nvd_idx + 1 == total_tail {
            let last_slot = syn::LitInt::new(
                &(total_tail.saturating_sub(1)).to_string(),
                proc_macro2::Span::call_site(),
            );
            let warm_entry_end = quote::quote! { self.tail_end.get() };
            let cached_first_tail = if nvd_idx == 0 {
                quote::quote! {
                    // `Iterator::next` cached the complete validated entry
                    // extent, including this prefix and payload.
                    if let Some(end) = #warm_entry_end {
                        let data_offset =
                            self.offset + self.acting_block_length + #prefix_size_lit;
                        return Ok(unsafe { self.buf.get_unchecked(data_offset..end) });
                    }
                }
            } else {
                quote::quote! {}
            };
            // Last tail component: a warm cache (filled by the iterator's
            // encoded_length) gives the slice end directly — no second
            // length-header read, bounds already validated.
            let nvd_cache_hit_last = quote::quote! {
                if let Some(end) = self.tail_end.get() {
                    let data_offset = offset.checked_add(#prefix_size_lit).ok_or(
                        sbe_rt::DecodeError::BufferTooShort {
                            field: stringify!(#vd_snake_ident),
                            needed: usize::MAX,
                            available: self.buf.len().saturating_sub(offset),
                        },
                    )?;
                    // SAFETY: a warm `tail_end` proves this entry's extent was
                    // validated when the iterator computed it.
                    return Ok(unsafe { self.buf.get_unchecked(data_offset..end) });
                }
            };
            let nvd_cache_publish = proc_macro2::TokenStream::new();
            entry_body.extend(quote::quote! {
                #[inline]
                pub fn #vd_snake_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    #version_check
                    #cached_first_tail
                    let offset = self.#tail_nvd_fn()?;
                    #nvd_cache_hit_last
                    let bytes: [u8; #prefix_size_lit] = read_bytes::<#prefix_size_lit>(self.buf, offset);
                    let header = #type_pascal_ident(bytes);
                    let wire_length = header.#len_field_ident() as u64;
                    let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                        stringify!(#vd_snake_ident),
                        offset,
                        #prefix_size_lit,
                        wire_length,
                        self.buf.len(),
                    )?;
                    #nvd_cache_publish
                    Ok(&self.buf[data_start..data_end])
                }
            });
        } else {
            // Only the LAST tail's end coincides with the entry extent, so a
            // mid-entry var-data field has no one-shot cache to consult.
            let nvd_cache_hit_mid = proc_macro2::TokenStream::new();
            let nvd_cache_publish = proc_macro2::TokenStream::new();
            entry_body.extend(quote::quote! {
                #[inline]
                pub fn #vd_snake_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    #version_check
                    let offset = self.#tail_nvd_fn()?;
                    #nvd_cache_hit_mid
                    let bytes: [u8; #prefix_size_lit] = read_bytes::<#prefix_size_lit>(self.buf, offset);
                    let header = #type_pascal_ident(bytes);
                    let wire_length = header.#len_field_ident() as u64;
                    let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                        stringify!(#vd_snake_ident),
                        offset,
                        #prefix_size_lit,
                        wire_length,
                        self.buf.len(),
                    )?;
                    #nvd_cache_publish
                    Ok(&self.buf[data_start..data_end])
                }
            });
        }
        // Same text helpers as the message-level accessor. A group entry's
        // var-data is text or binary for exactly the same reason the message's
        // is — the schema says so — so it gets the same `*_as_str` /
        // `*_as_str_unchecked` surface under the same names. Emitting them only
        // at message level forced callers to drop to `&[u8]` inside a group and
        // re-validate by hand.
        //
        // Unless the schema already used the name. Entry fields keep their
        // schema names in *every* entry location — decoder, ordered decoder,
        // encoder, DTO — so renaming one to free up `<vd>_as_str` would give
        // the same field different names per location, which is exactly what
        // the naming rule forbids. A field the author explicitly called
        // `noteAsStr` wins the name; `note()` still returns the bytes.
        let claims_taken = g.fields.iter().any(|f| {
            let n = to_snake_case(&f.name);
            n == format!("{vd_snake}_as_str") || n == format!("{vd_snake}_as_str_unchecked")
        });
        if !claims_taken {
            entry_body.extend(crate::codegen::message_decoder::vardata_text_helpers(
                &vd_snake,
                vd.character_encoding.as_deref(),
            ));
        }
        nvd_idx += 1;
    }

    // encoded_length, skip — tail shape is a compile-time constant;
    // emit only the live path (no dead branch in the generated source).
    let tail_total_fn = quote::format_ident!("tail_offset_{}", total_tail);
    let entry_len_cache_hit = quote::quote! {
        if let Some(end) = self.tail_end.get() {
            return Ok(end - self.offset);
        }
    };
    // Filled here, by the iterator call that computes the entry extent in
    // order to advance to the next entry.
    let entry_len_cache_publish = quote::quote! { self.tail_end.set(Some(end)); };
    if total_tail == 0 {
        entry_body.extend(quote::quote! {
            #mu
            #[inline]
            pub fn encoded_length(&self) -> usize {
                self.acting_block_length
            }
            #[inline]
            pub fn skip(buf: &'a [u8], offset: usize, block_len: usize, _acting_version: u16) -> Result<usize, sbe_rt::DecodeError> {
                if block_len > buf.len().saturating_sub(offset) {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "group entry",
                        needed: block_len,
                        available: buf.len().saturating_sub(offset),
                    });
                }
                Ok(offset + block_len)
            }
        });
    } else {
        entry_body.extend(quote::quote! {
            #[inline]
            pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
                #entry_len_cache_hit
                let end = self.#tail_total_fn()?;
                #entry_len_cache_publish
                Ok(end - self.offset)
            }
            #[inline]
            pub fn skip(
                buf: &'a [u8],
                offset: usize,
                block_len: usize,
                acting_version: u16,
            ) -> Result<usize, sbe_rt::DecodeError> {
                if block_len > buf.len().saturating_sub(offset) {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "group entry",
                        needed: block_len,
                        available: buf.len().saturating_sub(offset),
                    });
                }
                // SAFETY: fixed block length proven above; tail_total validates
                // nested groups and var-data extents before returning the end.
                let entry = unsafe { Self::wrap(buf, offset, block_len, acting_version) };
                entry.#tail_total_fn()
            }
        });
    }

    // A domain-typed primitive/enum/set field uses the fallible `try_*`
    // accessor for Display — the plain accessor is renamed to `*_wire` (or
    // removed) once a domain type is registered, so it no longer exists
    // under that name. Shared by the Primitive, Enum, and Set branches.
    fn domain_display_write(f_name: &str, field_name: &str, sep: &str) -> proc_macro2::TokenStream {
        let try_ident = syn::Ident::new(&format!("try_{f_name}"), proc_macro2::Span::call_site());
        let fmt_str = format!("{sep}{field_name}: {{:?}}");
        quote::quote! {
            { if let Ok(v) = self.#try_ident() { write!(f, #fmt_str, v)?; } }
        }
    }

    let mut entry_display_body = proc_macro2::TokenStream::new();
    let mut entry_display_out_idx = 0usize;
    for f in &g.fields {
        let f_name = to_snake_case(&f.name);
        let f_ident = syn::Ident::new(&f_name, proc_macro2::Span::call_site());
        let sep = if entry_display_out_idx == 0 { "" } else { ", " };
        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                if f.presence == Presence::Constant || length.is_some() {
                    continue;
                }
                // Domain-converted primitives use the fallible try_* accessor
                // (the plain accessor is renamed to *_wire and no longer
                // exists under that name once a domain type is registered —
                // matches the message-level Primitive branch's handling).
                if find_domain_type(f, domain_types).is_some() {
                    entry_display_body.extend(domain_display_write(&f_name, &f.name, sep));
                } else {
                    // A conversion renames the raw getter to `*_wire` even
                    // without a domain type, so the bare name may not exist.
                    let raw_ident = if field_has_conversion_free(f, conversions) {
                        syn::Ident::new(&format!("{f_name}_wire"), proc_macro2::Span::call_site())
                    } else {
                        f_ident.clone()
                    };
                    let fmt_str = format!("{sep}{}: {{:?}}", f.name);
                    entry_display_body.extend(quote::quote! {
                        { let v = self.#raw_ident(); write!(f, #fmt_str, v)?; }
                    });
                }
                entry_display_out_idx += 1;
            }
            FieldType::Enum {
                name: enum_name, ..
            } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                // Same rationale as the Primitive branch above: a domain
                // enum (e.g. bool) uses the fallible try_* accessor; the
                // plain accessor no longer exists under that name.
                if find_domain_type(f, domain_types).is_some() {
                    entry_display_body.extend(domain_display_write(&f_name, &f.name, sep));
                } else {
                    let fmt_str = format!("{sep}{}: {enum_name}::{{e:?}}", f.name);
                    entry_display_body.extend(quote::quote! {
                        { let e = self.#f_ident(); write!(f, #fmt_str)?; }
                    });
                }
                entry_display_out_idx += 1;
            }
            FieldType::Set { .. } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                // Domain-mapped sets rename the raw accessor to `*_wire` and
                // only require Debug on the domain type — same rule as
                // Primitive/Enum. The bitset's own Display is used otherwise.
                if find_domain_type(f, domain_types).is_some() {
                    entry_display_body.extend(domain_display_write(&f_name, &f.name, sep));
                } else {
                    let fmt_str = format!("{sep}{}: {{}}", f.name);
                    if f.since_version > 0 {
                        entry_display_body.extend(quote::quote! {
                            if let Some(v) = self.#f_ident() { write!(f, #fmt_str, v)?; }
                        });
                    } else {
                        entry_display_body.extend(quote::quote! {
                            { let v = self.#f_ident(); write!(f, #fmt_str, v)?; }
                        });
                    }
                }
                entry_display_out_idx += 1;
            }
            FieldType::Composite { .. } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                let f_value =
                    syn::Ident::new(&format!("{}_value", f_name), proc_macro2::Span::call_site());
                if find_domain_type(f, domain_types).is_some() {
                    // A caller's domain type is only ever required to be
                    // `Debug` — never assume `Display`.
                    let fmt_str = format!("{sep}{}: {{:?}}", f.name);
                    let err_fmt = format!("{sep}{}: <?>", f.name);
                    // Go through the generated `try_*` accessor rather than
                    // re-deriving the conversion here. Re-deriving fed it the
                    // raw `*_value()`, which is `Option<T>` for a
                    // `sinceVersion > 0` composite — no `TryFromSbe<Option<T>>`
                    // impl exists and the entry would not compile.
                    let try_ident =
                        syn::Ident::new(&format!("try_{f_name}"), proc_macro2::Span::call_site());
                    if f.since_version > 0 {
                        entry_display_body.extend(quote::quote! {
                            match self.#try_ident() {
                                Ok(Some(v)) => write!(f, #fmt_str, v)?,
                                Ok(None) => {}
                                Err(_) => write!(f, #err_fmt)?,
                            }
                        });
                    } else {
                        entry_display_body.extend(quote::quote! {
                            match self.#try_ident() {
                                Ok(v) => write!(f, #fmt_str, v)?,
                                Err(_) => write!(f, #err_fmt)?,
                            }
                        });
                    }
                } else {
                    let fmt_str = format!("{sep}{}: {{:?}}", f.name);
                    entry_display_body.extend(quote::quote! {
                        { write!(f, #fmt_str, self.#f_value())?; }
                    });
                }
                entry_display_out_idx += 1;
            }
        }
    }
    // Entry varData fields in Display — try UTF-8 first, fall back to bytes.
    for vd in &g.var_data {
        let vd_snake = to_snake_case(&vd.name);
        let vd_ident = syn::Ident::new(&vd_snake, proc_macro2::Span::call_site());
        let sep = if entry_display_out_idx == 0 { "" } else { ", " };
        let fmt_str = format!("{sep}{}: {{}}", vd.name);
        let err_fmt = format!("{sep}{}: <{{}} bytes>", vd.name);
        entry_display_body.extend(quote::quote! {
            if let Ok(d) = self.#vd_ident() {
                match std::str::from_utf8(d) {
                    Ok(s) => write!(f, #fmt_str, s)?,
                    Err(_) => write!(f, #err_fmt, d.len())?,
                }
            }
        });
        entry_display_out_idx += 1;
    }
    for ng in &g.groups {
        let ng_snake = to_snake_case(&ng.name);
        let ng_ident = syn::Ident::new(&ng_snake, proc_macro2::Span::call_site());
        let sep = if entry_display_out_idx == 0 { "" } else { ", " };
        let fmt_open = format!("{sep}{}: [", ng.name);
        if ng.has_fixed_stride() {
            entry_display_body.extend(quote::quote! {
                write!(f, #fmt_open)?;
                if let Ok(ng_decoder) = self.#ng_ident() {
                    for (i, entry) in ng_decoder.enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", entry)?;
                    }
                }
                write!(f, "]")?;
            });
        } else {
            // Nested group with tail: entries are Result-wrapped
            entry_display_body.extend(quote::quote! {
                write!(f, #fmt_open)?;
                if let Ok(ng_decoder) = self.#ng_ident() {
                    for (i, result) in ng_decoder.enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        match result {
                            Ok(entry) => write!(f, "{}", entry)?,
                            Err(_) => write!(f, "{{err}}")?,
                        }
                    }
                }
                write!(f, "]")?;
            });
        }
        entry_display_out_idx += 1;
    }

    if let Some(ref desc) = g.description {
        ts.extend(doc_attr_tokens(desc));
    } else {
        ts.extend(quote::quote! {
            #[doc = concat!("Entry decoder for the `", stringify!(#entry_decoder_ident), "` group — access fixed fields and var-data for one entry.")]
        });
    }
    if total_tail == 0 {
        ts.extend(quote::quote! {
            pub struct #entry_decoder_ident<'a> {
                buf: &'a [u8],
                offset: usize,
                acting_version: u16,
                acting_block_length: usize,
            }
        });
    } else {
        ts.extend(quote::quote! {
            pub struct #entry_decoder_ident<'a> {
                buf: &'a [u8],
                offset: usize,
                acting_version: u16,
                acting_block_length: usize,
                #entry_cache_field
            }
        });
    }
    ts.extend(quote::quote! {
        impl<'a> #entry_decoder_ident<'a> {
            #entry_body
        }
    });
    // Same gate as the message decoder's Display: `with_display_debug(false)`
    // must not emit formatting code anywhere, entries included.
    if enable_display_debug {
        ts.extend(quote::quote! {
            impl<'a> core::fmt::Display for #entry_decoder_ident<'a> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{{ ")?;
                    #entry_display_body
                    write!(f, " }}")
                }
            }
        });
    }

    // Recursively generate nested group decoders — scope under parent group name
    // to avoid collisions when different parent groups have same-named children
    // A group the encoder dropped drops its whole subtree, so nested decoders
    // generated underneath it must not name encoder-owned entry structs either.
    let nested_cap = if super::runtime::encodable_at(g.since_version) {
        super::runtime::encode_version_cap()
    } else {
        None
    };
    for ng in &g.groups {
        let nested_name = format!("{}{}", name, to_pascal_case(&ng.name));
        ts.extend(super::runtime::with_encode_version_cap(nested_cap, || {
            generate_group_decoder(
                ng,
                elements,
                byte_order,
                &nested_name,
                &conversions,
                domain_types,
                enable_meta_attributes,
                enable_dispatch,
                null_as_option,
                all_enums_as_option,
                enable_display_debug,
            )
        }));
    }

    // Consuming entry-level tail stages for entries with nested groups and/or
    // var-data. Random-access `&self` entry accessors remain. Emitted after the
    // nested group decoders above so `finish()` can name them.
    ts.extend(generate_entry_consuming_stages(
        g,
        elements,
        &name,
        byte_order,
        enable_dispatch,
    ));

    ts
}

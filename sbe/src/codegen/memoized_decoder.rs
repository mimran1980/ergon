//! `{Name}MemoizedDecoder` — the opt-in progressive tail cache.
//!
//! Produced by consuming `{Name}Decoder::memoized(self)`. It wraps the base
//! decoder and adds a progressive `Cell` cache of discovered dynamic-tail
//! ends, so reading tails out of order — or reading one twice — walks the
//! wire at most once. Construction is O(1) and allocates nothing;
//! undiscovered slots are never read.
//!
//! The walkers themselves stay on the base decoder (see `tail_cache.rs`);
//! this lane only changes how a tail *start* is reached. Getter names are the
//! same as the base decoder's, so swapping lanes is a one-line change.
//!
//! The cache uses `Cell`, so the wrapper is `Send` but not `Sync`. Build it
//! once and pass `&{Name}MemoizedDecoder` around: calling `.memoized()` again
//! produces a second, empty cache and re-walks everything.

use crate::structured_ir::{MessageStructure, SchemaElements, get_vardata_info};

use super::doc_attr_tokens;
use super::ordered_decoder::forward_fixed_fields;
use super::to_snake_case;

/// Emit `memoized()` plus the `{Name}MemoizedDecoder` wrapper.
///
/// Fixed-block messages have no dynamic tails to memoize, so nothing is
/// emitted for them — the base decoder is already the whole story.
#[allow(clippy::too_many_lines)]
pub(crate) fn generate_memoized_decoder(
    msg: &MessageStructure,
    elements: &SchemaElements,
    name: &str,
    group_unique_names: &[String],
    enable_display_debug: bool,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let total_tail = msg.groups.len() + msg.var_data.len();
    if total_tail == 0 {
        return proc_macro2::TokenStream::new();
    }

    let decoder_ident = syn::Ident::new(&format!("{name}Decoder"), span);
    let memo_ident = syn::Ident::new(&format!("{name}MemoizedDecoder"), span);
    let cache_ty = super::tail_cache::cache_type_tokens(total_tail);

    let core = quote::quote! { self.inner };

    let mut ts = quote::quote! {
        impl<'a> #decoder_ident<'a> {
            /// Consume this decoder and return one that memoizes dynamic-tail
            /// boundaries.
            ///
            /// Use it when you read tails out of order, or read the same tail
            /// more than once. A single cold pass in wire order gains nothing
            /// — each tail already begins where the last one ended — and pays
            /// for the cache, so the base decoder stays the default.
            ///
            /// Construction is O(1) and allocates nothing. Decoded values and
            /// wire bytes are identical to the base lane.
            ///
            /// Build it **once** and share `&`-references: each call creates a
            /// separate empty cache.
            #[inline]
            #[must_use = "memoized() returns a new decoder; the original is consumed"]
            pub fn memoized(self) -> #memo_ident<'a> {
                #memo_ident {
                    inner: self,
                    cache: sbe_rt::TailBoundaryCache::new(),
                }
            }
        }

        /// Random-access decoder with a progressive dynamic-tail cache.
        ///
        /// Same getter names as the base decoder. `Send` but not `Sync` — the
        /// cache uses `Cell`, so use one instance per thread over shareable
        /// immutable bytes.
        #[must_use = "decoder must be read; dropping it discards the cache"]
        pub struct #memo_ident<'a> {
            inner: #decoder_ident<'a>,
            cache: #cache_ty,
        }
    };

    let mut impl_body = proc_macro2::TokenStream::new();

    // Header/state accessors and an escape hatch back to the base lane.
    impl_body.extend(quote::quote! {
        /// Schema version from the message header (or wrap args).
        #[inline]
        pub const fn acting_version(&self) -> u16 {
            self.inner.acting_version
        }

        /// Acting block length from the message header (or wrap args).
        #[inline]
        pub const fn acting_block_length(&self) -> usize {
            self.inner.acting_block_length
        }

        /// Borrow the underlying uncached decoder (fixed fields, metadata).
        #[inline]
        pub const fn inner(&self) -> &#decoder_ident<'a> {
            &self.inner
        }

        /// Discard the cache and return the base decoder.
        #[inline]
        #[must_use = "discarding the returned decoder discards the message"]
        pub fn into_inner(self) -> #decoder_ident<'a> {
            self.inner
        }
    });

    // Fixed fields are random-access in both lanes and never touch the cache,
    // so they forward straight to the inner decoder — same forwarder the
    // ordered lane uses, so conversions and domain types stay consistent.
    impl_body.extend(forward_fixed_fields(
        &msg.fields,
        conversions,
        domain_types,
        null_as_option,
        all_enums_as_option,
    ));

    // Cache-consulting tail starts over the base decoder's pure walkers.
    impl_body.extend(super::tail_cache::emit_cached_tail_offsets(
        total_tail, &core,
    ));

    // Group getters: identical names, cached tail starts.
    for (gi, g) in msg.groups.iter().enumerate() {
        let scoped = &group_unique_names[gi];
        let g_snake = to_snake_case(&g.name);
        let g_snake_ident = syn::Ident::new(&g_snake, span);
        let g_decoder_ident = syn::Ident::new(&format!("{scoped}Decoder"), span);
        let tail_ident = quote::format_ident!("tail_offset_{gi}");
        let version_check = version_guard(g.since_version, &g_snake, span);
        if let Some(ref desc) = g.description {
            impl_body.extend(doc_attr_tokens(desc));
        }
        impl_body.extend(quote::quote! {
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn #g_snake_ident(&self) -> Result<#g_decoder_ident<'a>, sbe_rt::DecodeError> {
                #version_check
                let offset = self.#tail_ident()?;
                #g_decoder_ident::wrap(
                    self.inner.buf,
                    offset,
                    self.inner.acting_version,
                )
            }
        });
    }

    // Var-data getters: a warm slot gives the end directly, so the length
    // header is not re-read. Publishing happens only after validation.
    let mut vd_idx = msg.groups.len();
    for vd in &msg.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let vd_snake = to_snake_case(&vd.name);
        let vd_ident = syn::Ident::new(&vd_snake, span);
        let type_pascal_ident = syn::Ident::new(&type_pascal, span);
        let len_field_ident = syn::Ident::new(&len_field, span);
        let prefix_lit = syn::LitInt::new(&prefix_size.to_string(), span);
        let slot_lit = syn::LitInt::new(&vd_idx.to_string(), span);
        let tail_ident = quote::format_ident!("tail_offset_{vd_idx}");
        let max = vd.max_length.unwrap_or(0);
        let max_lit = syn::LitInt::new(&max.to_string(), span);
        let version_check = version_guard(vd.since_version, &vd_snake, span);
        if let Some(ref desc) = vd.description {
            impl_body.extend(doc_attr_tokens(desc));
        }
        impl_body.extend(quote::quote! {
            #[inline]
            pub fn #vd_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                #version_check
                let offset = self.#tail_ident()?;
                let buf = self.inner.buf;
                if let Some(end) = self.cache.end_of(#slot_lit) {
                    let data_start = offset + #prefix_lit;
                    return Ok(&buf[data_start..end]);
                }
                if offset + #prefix_lit > buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: stringify!(#vd_ident),
                        needed: #prefix_lit,
                        available: buf.len().saturating_sub(offset),
                    });
                }
                let bytes: [u8; #prefix_lit] = read_bytes::<#prefix_lit>(buf, offset);
                let header = #type_pascal_ident(bytes);
                let wire_length = header.#len_field_ident() as u64;
                if wire_length > #max_lit as u64 {
                    return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                        field: stringify!(#vd_ident),
                        length: wire_length,
                        max_length: #max_lit as u64,
                    });
                }
                let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                    stringify!(#vd_ident),
                    offset,
                    #prefix_lit,
                    wire_length,
                    buf.len(),
                )?;
                self.cache.publish(#slot_lit, data_end);
                Ok(&buf[data_start..data_end])
            }
        });

        if vd.character_encoding.as_deref() == Some("UTF-8") {
            let str_ident = syn::Ident::new(&format!("{vd_snake}_as_str"), span);
            let vd_name_lit = vd_snake.clone();
            impl_body.extend(quote::quote! {
                /// View this UTF-8 var-data field as `&str`.
                #[inline]
                pub fn #str_ident(&self) -> Result<&'a str, sbe_rt::DecodeError> {
                    let bytes = self.#vd_ident()?;
                    core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::InvalidUtf8 {
                        field: #vd_name_lit,
                        error: e,
                    })
                }
            });
        }
        vd_idx += 1;
    }

    // Complete-message length: the last tail's end relative to the body start.
    let total_tail_ident = quote::format_ident!("tail_offset_{total_tail}");
    impl_body.extend(quote::quote! {
        /// Total body length, walking (and caching) every remaining tail.
        #[must_use = "discarding this value is almost always a mistake"]
        #[inline]
        pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
            let end = self.#total_tail_ident()?;
            Ok(end - self.inner.offset)
        }
    });

    ts.extend(quote::quote! {
        impl<'a> #memo_ident<'a> {
            #impl_body
        }
    });

    if enable_display_debug {
        let memo_name_lit = syn::LitStr::new(&format!("{name}MemoizedDecoder"), span);
        ts.extend(quote::quote! {
            impl<'a> core::fmt::Debug for #memo_ident<'a> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    f.debug_struct(#memo_name_lit)
                        .field("inner", &self.inner)
                        .finish_non_exhaustive()
                }
            }
        });
    }

    ts
}

/// `FieldNotInVersion` guard for a member introduced after version 0.
fn version_guard(
    since_version: u16,
    field: &str,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    if since_version == 0 {
        return proc_macro2::TokenStream::new();
    }
    let since_lit = syn::LitInt::new(&since_version.to_string(), span);
    let field_lit = syn::LitStr::new(field, span);
    quote::quote! {
        if self.inner.acting_version < #since_lit {
            return Err(sbe_rt::DecodeError::FieldNotInVersion {
                field: #field_lit,
                wire_version: self.inner.acting_version,
                since_version: #since_lit,
            });
        }
    }
}

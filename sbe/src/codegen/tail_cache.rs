//! Random-access tail-offset emission.
//!
//! One kernel, two lanes. The per-tail `walk_tail_k(start)` walkers and the
//! `walk_dynamic_tail(k, start)` dispatch over them are pure: given a start
//! offset they validate and return that tail's end, consulting no cache. They
//! are emitted once, on the base decoder.
//!
//! * Base lane — `tail_offset_k` chains back through `tail_offset_0`,
//!   re-walking on every access. No cache field, so the decoder stays `Sync`
//!   and small. This is what `Decoder::try_decode` gives you.
//! * Memoized lane — [`emit_cached_tail_offsets`] emits a progressive `Cell`
//!   cache over the *same* walkers, on the `{Name}MemoizedDecoder` wrapper
//!   produced by `Decoder::memoized(self)`. Walked forward from the frontier
//!   only, so repeated and out-of-order access walks each boundary at most
//!   once.
//!
//! Group *entry* decoders keep their own one-shot extent cache in both lanes
//! (see `group_decoder.rs`): the group iterator computes each entry's end to
//! advance, and the last var-data accessor reuses it.

use crate::structured_ir::{
    MessageGroup, MessageVarData, SchemaElements, get_dimension_info, get_vardata_info,
};

pub(crate) fn cache_type_tokens(n: usize) -> proc_macro2::TokenStream {
    let n_lit = syn::LitInt::new(&n.to_string(), proc_macro2::Span::call_site());
    quote::quote! { sbe_rt::TailBoundaryCache<#n_lit> }
}

/// Emit the per-tail walkers and `tail_offset_*` accessors for an owner with
/// `groups` then `var_data`, in whichever mode the config selected.
pub(crate) fn emit_tail_offsets(
    groups: &[MessageGroup],
    var_data: &[MessageVarData],
    elements: &SchemaElements,
    entry_skip: &[syn::Ident],
    tail0: proc_macro2::TokenStream,
    cache_base: proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let total_tail = groups.len() + var_data.len();
    let mut ts = proc_macro2::TokenStream::new();
    ts.extend(quote::quote! {
        #[inline]
        fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
            #tail0
        }
    });
    if total_tail == 0 {
        return ts;
    }

    let mut walk_arms = proc_macro2::TokenStream::new();
    let mut walk_fns = proc_macro2::TokenStream::new();
    let mut k = 0usize;
    for (gi, g) in groups.iter().enumerate() {
        let k_lit = syn::LitInt::new(&k.to_string(), proc_macro2::Span::call_site());
        let walk_ident = quote::format_ident!("walk_tail_{k}");
        let (dim_name, dim_size, bl_field, count_field) =
            get_dimension_info(elements, &g.dimension_type);
        let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), proc_macro2::Span::call_site());
        let dn_ident: syn::Ident = syn::parse_str(&dim_name).unwrap();
        let cf_ident = syn::Ident::new(&count_field, proc_macro2::Span::call_site());
        let bf_ident = syn::Ident::new(&bl_field, proc_macro2::Span::call_site());
        let gn_lit = g.name.as_str();
        let entry_decoder_ident = &entry_skip[gi];
        let version_skip = if g.since_version > 0 {
            let since_lit =
                syn::LitInt::new(&g.since_version.to_string(), proc_macro2::Span::call_site());
            quote::quote! {
                if self.acting_version < #since_lit {
                    return Ok(start);
                }
            }
        } else {
            proc_macro2::TokenStream::new()
        };
        walk_fns.extend(quote::quote! {
            #[inline]
            fn #walk_ident(&self, start: usize) -> Result<usize, sbe_rt::DecodeError> {
                #version_skip
                if start + #dim_size_lit > self.buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: #gn_lit,
                        needed: #dim_size_lit,
                        available: self.buf.len().saturating_sub(start),
                    });
                }
                let bytes: [u8; #dim_size_lit] = read_bytes::<#dim_size_lit>(self.buf, start);
                let header = #dn_ident(bytes);
                let count = sbe_rt::checked_group_count(
                    "numInGroup",
                    header.#cf_ident() as u64,
                )?;
                let block_len = sbe_rt::checked_header_usize(
                    "blockLength",
                    header.#bf_ident() as u64,
                )?;
                let mut offset = start + #dim_size_lit;
                let mut idx = 0;
                while idx < count {
                                        offset = #entry_decoder_ident::skip(
                        self.buf,
                        offset,
                        block_len,
                        self.acting_version,
                    )?;
                    idx += 1;
                }
                Ok(offset)
            }
        });
        walk_arms.extend(quote::quote! { #k_lit => self.#walk_ident(start), });
        k += 1;
    }
    for vd in var_data {
        let k_lit = syn::LitInt::new(&k.to_string(), proc_macro2::Span::call_site());
        let walk_ident = quote::format_ident!("walk_tail_{k}");
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let prefix_size_lit =
            syn::LitInt::new(&prefix_size.to_string(), proc_macro2::Span::call_site());
        let vd_type_ident = syn::Ident::new(&type_pascal, proc_macro2::Span::call_site());
        let vd_len_field_ident = syn::Ident::new(&len_field, proc_macro2::Span::call_site());
        let vd_name_lit = syn::LitStr::new(&vd.name, proc_macro2::Span::call_site());
        let version_skip = if vd.since_version > 0 {
            let since_lit = syn::LitInt::new(
                &vd.since_version.to_string(),
                proc_macro2::Span::call_site(),
            );
            quote::quote! {
                if self.acting_version < #since_lit {
                    return Ok(start);
                }
            }
        } else {
            proc_macro2::TokenStream::new()
        };
        walk_fns.extend(quote::quote! {
            #[inline]
            fn #walk_ident(&self, start: usize) -> Result<usize, sbe_rt::DecodeError> {
                #version_skip
                if #prefix_size_lit > self.buf.len().saturating_sub(start) {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: #vd_name_lit,
                        needed: #prefix_size_lit,
                        available: self.buf.len().saturating_sub(start),
                    });
                }
                let bytes: [u8; #prefix_size_lit] =
                    read_bytes::<#prefix_size_lit>(self.buf, start);
                let header = #vd_type_ident(bytes);
                let wire_length = header.#vd_len_field_ident() as u64;
                let (_, data_end) = sbe_rt::checked_var_data_bounds(
                    #vd_name_lit,
                    start,
                    #prefix_size_lit,
                    wire_length,
                    self.buf.len(),
                )?;
                Ok(data_end)
            }
        });
        walk_arms.extend(quote::quote! { #k_lit => self.#walk_ident(start), });
        k += 1;
    }

    ts.extend(walk_fns);

    if total_tail > 0 {
        // Pure dispatch over the walkers. Emitted on the base decoder in every
        // lane; the memoized wrapper drives it through the inner decoder.
        ts.extend(quote::quote! {
            #[inline]
            pub(crate) fn walk_dynamic_tail(
                &self,
                k: usize,
                start: usize,
            ) -> Result<usize, sbe_rt::DecodeError> {
                match k {
                    #walk_arms
                    _ => Ok(start),
                }
            }
        });
    }

    // Base lane: each tail chains back through its predecessor, re-walking the
    // wire on every access. No cache field exists to consult.
    for i in 1..=total_tail {
        let ident = quote::format_ident!("tail_offset_{i}");
        let prev = quote::format_ident!("tail_offset_{}", i - 1);
        let walk = quote::format_ident!("walk_tail_{}", i - 1);
        ts.extend(quote::quote! {
            #[inline]
            fn #ident(&self) -> Result<usize, sbe_rt::DecodeError> {
                let start = self.#prev()?;
                self.#walk(start)
            }
        });
    }

    ts
}

/// Cache-consulting `tail_offset_*` for the memoized wrapper.
///
/// `core` names the inner base decoder (`self.inner`), which owns the pure
/// walkers; the cache and its `tail_offset_*` live on the wrapper. A boundary
/// is published only after its walk succeeded, so a decode error can never
/// leave a bogus offset in the cache.
pub(crate) fn emit_cached_tail_offsets(
    total_tail: usize,
    core: &proc_macro2::TokenStream,
) -> proc_macro2::TokenStream {
    let mut ts = proc_macro2::TokenStream::new();
    if total_tail == 0 {
        return ts;
    }
    ts.extend(quote::quote! {
        #[inline]
        fn ensure_tail_start(&self, idx: usize) -> Result<usize, sbe_rt::DecodeError> {
            if idx == 0 {
                return #core.tail_offset_0();
            }
            let need_slot = idx - 1;
            if let Some(abs) = self.cache.end_of(need_slot) {
                #[cfg(debug_assertions)]
                self.cache.record_hit();
                return Ok(abs);
            }
            #[cfg(debug_assertions)]
            self.cache.record_miss();
            let mut k = self.cache.known_through();
            let mut pos = if k == 0 {
                #core.tail_offset_0()?
            } else {
                match self.cache.end_of(k - 1) {
                    Some(abs) => abs,
                    None => #core.tail_offset_0()?,
                }
            };
            while k < idx {
                #[cfg(debug_assertions)]
                self.cache.record_boundary();
                // A failed walk returns here, so the boundary is never
                // published: the frontier only ever advances over validated
                // offsets.
                pos = #core.walk_dynamic_tail(k, pos)?;
                self.cache.publish(k, pos);
                k += 1;
            }
            Ok(pos)
        }
    });

    for i in 0..=total_tail {
        let ident = quote::format_ident!("tail_offset_{i}");
        let i_lit = syn::LitInt::new(&i.to_string(), proc_macro2::Span::call_site());
        ts.extend(quote::quote! {
            #[inline]
            fn #ident(&self) -> Result<usize, sbe_rt::DecodeError> {
                self.ensure_tail_start(#i_lit)
            }
        });
    }

    ts.extend(quote::quote! {
        /// Debug-build cache counters: hits, misses, boundary walks, frontier.
        #[cfg(debug_assertions)]
        #[must_use = "discarding cache stats is almost always a mistake"]
        #[inline]
        pub fn decode_cache_stats(&self) -> sbe_rt::DecodeCacheStats {
            self.cache.stats()
        }
    });

    ts
}

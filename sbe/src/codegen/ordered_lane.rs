//! Ordered decode lane: one callback per tail, wrapping the staged stages.
//!
//! The public type is `sbe_rt::Ordered<S>` generated in the consumer's
//! `sbe_rt` module. Message-level `fixed` is optional, consumes into
//! `Ordered<OrderedFixed<Decoder>>`, and hands a fixed-fields-only view to
//! the callback. Entry-level ordered wrappers start at the first nested tail.

use crate::structured_ir::{
    MessageField, OwnerTailGroup, OwnerTailVarData, decoder_stage_after_ident,
};

use super::conversion_helpers::{acting_accessors, named_accessor_ident, tail_accessor_ident};
use super::memoized_decoder::forward_fixed_fields;

/// Fixed-field set used to generate the ordered lane's message-level view.
pub(crate) struct OrderedFixedFields<'a> {
    pub fields: &'a [MessageField],
    pub conversions: &'a [crate::ConversionSelector],
    pub domain_types: &'a [(crate::ConversionSelector, String)],
    pub null_as_option: &'a [crate::ConversionSelector],
    pub all_enums_as_option: bool,
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
    is_message: bool,
    taken_accessors: &[String],
    fixed_fields: Option<OrderedFixedFields<'_>>,
) -> proc_macro2::TokenStream {
    let total_tail = groups.len() + vardata.len();
    if total_tail == 0 {
        return proc_macro2::TokenStream::new();
    }
    if named_accessor_ident("ordered", taken_accessors).is_none() {
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

    let first_tail_snake = groups
        .first()
        .map(|g| g.accessor_snake.as_str())
        .or_else(|| vardata.first().map(|v| v.accessor_snake.as_str()));
    // Message-level `fixed` / `try_fixed` live on the same type as the first
    // tail unless they consume into `OrderedFixed`. Each name yields through
    // `named_accessor_ident`. The first tail also owns `try_{tail}`, so a
    // group/var-data named `fixed` takes both `fixed()` and `try_fixed()`.
    // A first tail named `tryFixed` only takes `try_fixed()`; `fixed()` stays.
    let first_taken: Vec<String> = first_tail_snake
        .map(|s| vec![s.to_string(), format!("try_{s}")])
        .unwrap_or_default();
    let emit_fixed_cb = is_message
        && fixed_fields.is_some()
        && named_accessor_ident("fixed", &first_taken).is_some();
    let emit_try_fixed = is_message
        && fixed_fields.is_some()
        && named_accessor_ident("try_fixed", &first_taken).is_some();
    let emit_fixed_phase = emit_fixed_cb || emit_try_fixed;

    let mut ts = proc_macro2::TokenStream::new();
    let ordered_doc = if is_message {
        "Walk the whole message in wire order with one callback per tail."
    } else {
        "Walk this entry's own tails in wire order with one callback each. `done()` returns the entry completion the parent's visit closure must hand back, so an entry can be walked in the ordered spelling without breaking the parent's one-pass traversal."
    };
    let ordered_doc = super::runtime::doc_lines_tokens(ordered_doc);
    ts.extend(quote::quote! {
        impl<'a> #initial_ident<'a> {
            #ordered_doc
            ///
            /// A façade over the staged `into_*` / `skip_*` stages: same single
            /// traversal, same compile-time ordering, one uniform spelling.
            #[inline]
            pub fn ordered(self) -> sbe_rt::Ordered<Self> {
                sbe_rt::Ordered { inner: self }
            }
        }
    });

    let decoder_ty = quote::quote! { sbe_rt::Ordered<#initial_ident<'a>> };
    let after_fixed_ty =
        quote::quote! { sbe_rt::Ordered<sbe_rt::OrderedFixed<#initial_ident<'a>>> };
    let decoder_inner = quote::quote! { self.inner };
    let after_fixed_inner = quote::quote! { self.inner.inner };

    if emit_fixed_phase {
        let ff = fixed_fields
            .as_ref()
            .expect("emit_fixed_phase implies Some");
        let view_ident = syn::Ident::new(&format!("{stage_prefix}FixedView"), span);
        // Forwarded fields carry their `DECODER_RESERVED` renames, so a field
        // can never own `acting_*` on the view.
        let view_acting = acting_accessors(
            &[],
            &quote::quote! { self.inner },
            &proc_macro2::TokenStream::new(),
        );
        let forwarded = forward_fixed_fields(
            ff.fields,
            ff.conversions,
            ff.domain_types,
            ff.null_as_option,
            ff.all_enums_as_option,
        );
        let advance = quote::quote! {
            let view = #view_ident { inner: &self.inner };
            f(&view)?;
            Ok(sbe_rt::Ordered {
                inner: sbe_rt::OrderedFixed { inner: self.inner },
            })
        };
        let fixed_method = emit_fixed_cb.then(|| {
            if emit_try_fixed {
                quote::quote! {
                    /// Read the fixed block before any tail, then continue from a
                    /// distinct stage that carries only the tail methods. Yields
                    /// when the first tail is named `fixed`; use [`Self::try_fixed`]
                    /// there.
                    #[inline]
                    pub fn fixed<F>(
                        self,
                        f: F,
                    ) -> Result<#after_fixed_ty, sbe_rt::DecodeError>
                    where
                        F: FnOnce(&#view_ident<'_, 'a>) -> Result<(), sbe_rt::DecodeError>,
                    {
                        self.try_fixed(f)
                    }
                }
            } else {
                quote::quote! {
                    /// Read the fixed block before any tail, then continue from a
                    /// distinct stage that carries only the tail methods. Standalone
                    /// because the first tail owns `try_fixed()`.
                    #[inline]
                    pub fn fixed<F>(
                        self,
                        f: F,
                    ) -> Result<#after_fixed_ty, sbe_rt::DecodeError>
                    where
                        F: FnOnce(&#view_ident<'_, 'a>) -> Result<(), sbe_rt::DecodeError>,
                    {
                        #advance
                    }
                }
            }
        });
        let try_fixed_method = emit_try_fixed.then(|| {
            quote::quote! {
                /// Read the fixed block before any tail, with a caller error type.
                /// Yields when the first tail is named `tryFixed`; use [`Self::fixed`]
                /// there.
                #[inline]
                pub fn try_fixed<E, F>(self, f: F) -> Result<#after_fixed_ty, E>
                where
                    F: FnOnce(&#view_ident<'_, 'a>) -> Result<(), E>,
                {
                    #advance
                }
            }
        });
        ts.extend(quote::quote! {
            /// Fixed-block view for the ordered lane's `fixed` callback.
            ///
            /// Only fixed-field getters, acting version, and acting block
            /// length. Group and var-data accessors are not on this type, so
            /// the callback cannot start a second walk of the tails.
            /// `'v` is the callback's borrow of the decoder; `'a` is the
            /// buffer lifetime. They must be distinct so `fixed` can move
            /// the decoder into the following stage after the callback returns.
            pub struct #view_ident<'v, 'a> {
                inner: &'v #initial_ident<'a>,
            }
            impl<'v, 'a> #view_ident<'v, 'a> {
                #view_acting
                #forwarded
            }
            impl<'a> #decoder_ty {
                #fixed_method
                #try_fixed_method
            }
        });
    }

    // First tail: on Ordered<Decoder> (skip-fixed or no-fixed) and, when
    // `fixed` exists, also on Ordered<OrderedFixed<Decoder>>.
    ts.extend(emit_tail_stage(
        &decoder_ty,
        &decoder_inner,
        0,
        groups,
        vardata,
        &staged_stage,
        taken_accessors,
        true,
        span,
    ));
    if emit_fixed_phase {
        ts.extend(emit_tail_stage(
            &after_fixed_ty,
            &after_fixed_inner,
            0,
            groups,
            vardata,
            &staged_stage,
            taken_accessors,
            true,
            span,
        ));
    }
    for i in 1..total_tail {
        let current_inner = staged_stage(i - 1);
        let current_ty = quote::quote! { sbe_rt::Ordered<#current_inner<'a>> };
        ts.extend(emit_tail_stage(
            &current_ty,
            &decoder_inner,
            i,
            groups,
            vardata,
            &staged_stage,
            taken_accessors,
            false,
            span,
        ));
    }

    let last = staged_stage(total_tail - 1);
    let last_ty = quote::quote! { sbe_rt::Ordered<#last<'a>> };
    let done_doc = if is_message {
        "The completed staged decoder. Prefer [`Self::encoded_length_with_header`] / [`Self::as_bytes_with_header`] on this type — the message chain ends like encode, without `.done()`."
    } else {
        "The entry completion the parent's visit closure must hand back. A nested `ordered()` walk can return this wrapper directly; call `done()` only to unwrap the staged complete."
    };
    let done_doc = super::runtime::doc_lines_tokens(done_doc);
    let extent = is_message.then(|| {
        quote::quote! {
            /// Body bytes (excluding the message header).
            #[inline]
            pub fn as_body_bytes(&self) -> &'a [u8] {
                self.inner.as_body_bytes()
            }
            /// Complete SBE frame (header + body).
            #[inline]
            pub fn as_bytes_with_header(&self) -> &'a [u8] {
                self.inner.as_bytes_with_header()
            }
            /// Body length (excluding header).
            #[inline]
            pub fn encoded_length(&self) -> usize {
                self.inner.encoded_length()
            }
            /// Total message length including the schema-declared header.
            #[inline]
            pub fn encoded_length_with_header(&self) -> usize {
                self.inner.encoded_length_with_header()
            }
            /// Bytes after this message.
            #[inline]
            pub fn remaining(&self) -> &'a [u8] {
                self.inner.remaining()
            }
        }
    });
    // The last wrapper holds only `done` and extent methods, which no tail can
    // share a name with here, and the staged stage it wraps has no fields.
    let last_acting = acting_accessors(
        &[],
        &quote::quote! { self.inner },
        &proc_macro2::TokenStream::new(),
    );
    ts.extend(quote::quote! {
        impl<'a> #last_ty {
            #last_acting
            #done_doc
            #[inline]
            pub fn done(self) -> #last<'a> {
                self.inner
            }
            #extent
        }
    });

    ts
}

fn emit_tail_stage(
    current_ty: &proc_macro2::TokenStream,
    inner_access: &proc_macro2::TokenStream,
    i: usize,
    groups: &[OwnerTailGroup],
    vardata: &[OwnerTailVarData],
    staged_stage: &dyn Fn(usize) -> syn::Ident,
    taken_accessors: &[String],
    inner_may_yield_count_len: bool,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    let next = staged_stage(i);
    let next_ty = quote::quote! { sbe_rt::Ordered<#next<'a>> };
    // Per type, not per owner: every wrapper defines only tail `i`'s methods
    // (plus the peek), so tail `i` is the only name that can take `acting_*`
    // from it. The owner's own fields and other tails live on other types and
    // are not in play here — suppressing on those would drop an accessor
    // nothing on this type defines. `inner_may_yield_count_len` answers a
    // different question, for the peek: whether the *inner* type still has the
    // count/len method this wrapper forwards to.
    let tail_name = if i < groups.len() {
        &groups[i].accessor_snake
    } else {
        &vardata[i - groups.len()].accessor_snake
    };
    let own_tail = [tail_name.clone()];
    let acting = acting_accessors(&own_tail, inner_access, &proc_macro2::TokenStream::new());
    if i < groups.len() {
        let tg = &groups[i];
        let peek = peek_count(inner_access, tg, taken_accessors, inner_may_yield_count_len);
        let methods = emit_group_methods(inner_access, tg, &next_ty, span);
        quote::quote! {
            impl<'a> #current_ty {
                #acting
                #peek
                #methods
            }
        }
    } else {
        let vd = &vardata[i - groups.len()];
        let peek = peek_len(inner_access, vd, taken_accessors, inner_may_yield_count_len);
        let methods = emit_vardata_methods(inner_access, vd, &next_ty, span);
        quote::quote! {
            impl<'a> #current_ty {
                #acting
                #peek
                #methods
            }
        }
    }
}

fn peek_count(
    inner_access: &proc_macro2::TokenStream,
    tg: &OwnerTailGroup,
    taken: &[String],
    inner_may_yield_count_len: bool,
) -> proc_macro2::TokenStream {
    if inner_may_yield_count_len
        && tail_accessor_ident(&tg.accessor_snake, "count", taken).is_none()
    {
        return proc_macro2::TokenStream::new();
    }
    let ident = quote::format_ident!("{}_count", tg.accessor_snake);
    quote::quote! {
        /// Wire-declared entry count without advancing this ordered stage.
        #[inline]
        pub fn #ident(&self) -> Result<usize, sbe_rt::DecodeError> {
            #inner_access.#ident()
        }
    }
}

fn peek_len(
    inner_access: &proc_macro2::TokenStream,
    vd: &OwnerTailVarData,
    taken: &[String],
    inner_may_yield_count_len: bool,
) -> proc_macro2::TokenStream {
    if inner_may_yield_count_len && tail_accessor_ident(&vd.accessor_snake, "len", taken).is_none()
    {
        return proc_macro2::TokenStream::new();
    }
    let ident = quote::format_ident!("{}_len", vd.accessor_snake);
    quote::quote! {
        /// Byte length without advancing this ordered stage.
        #[inline]
        pub fn #ident(&self) -> Result<usize, sbe_rt::DecodeError> {
            #inner_access.#ident()
        }
    }
}

fn emit_group_methods(
    inner_access: &proc_macro2::TokenStream,
    tg: &OwnerTailGroup,
    next_ty: &proc_macro2::TokenStream,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    let method = syn::Ident::new(&tg.accessor_snake, span);
    let try_method = syn::Ident::new(&format!("try_{}", tg.accessor_snake), span);
    let into_ident = syn::Ident::new(&format!("into_{}", tg.accessor_snake), span);
    let into_with_info = syn::Ident::new(&format!("into_{}_with_info", tg.accessor_snake), span);
    let entry_ident = syn::Ident::new(&tg.entry_decoder_ident, span);
    let doc = format!(
        " Visit every `{}` entry in wire order, then advance to the next tail.\n\n\
         The callback receives the entry and an [`sbe_rt::EntryInfo`] carrying its index, \
         the wire-declared count and the acting block length. Empty groups invoke it zero times.",
        tg.accessor_snake
    );
    let doc = super::runtime::doc_lines_tokens(&doc);
    if tg.entries_have_tails {
        let complete_ident = syn::Ident::new(&format!("{}Complete", tg.entry_decoder_ident), span);
        quote::quote! {
            #doc
            ///
            /// These entries carry tails of their own, so the callback
            /// returns the entry's completion — that is where the next
            /// entry starts, so nothing is scanned twice. A nested
            /// `ordered()` walk may return `Ordered<completion>` directly;
            /// `.done()` is only needed to hand the staged complete out.
            #[inline]
            pub fn #method<F, R>(self, f: F) -> Result<#next_ty, sbe_rt::DecodeError>
            where
                F: FnMut(
                    #entry_ident<'a>,
                    sbe_rt::EntryInfo,
                ) -> Result<R, sbe_rt::DecodeError>,
                R: sbe_rt::IntoEntryComplete<#complete_ident<'a>>,
            {
                self.#try_method(f)
            }
            #doc
            #[inline]
            pub fn #try_method<E, F, R>(self, mut f: F) -> Result<#next_ty, E>
            where
                E: From<sbe_rt::DecodeError>,
                F: FnMut(
                    #entry_ident<'a>,
                    sbe_rt::EntryInfo,
                ) -> Result<R, E>,
                R: sbe_rt::IntoEntryComplete<#complete_ident<'a>>,
            {
                let inner = #inner_access.#into_with_info(|entry, info| {
                    f(entry, info).map(sbe_rt::IntoEntryComplete::into_entry_complete)
                })?;
                Ok(sbe_rt::Ordered { inner })
            }
        }
    } else {
        quote::quote! {
            #doc
            ///
            /// These entries have a fixed stride, so the callback returns
            /// `()` — there is no tail to complete.
            #[inline]
            pub fn #method<F>(self, f: F) -> Result<#next_ty, sbe_rt::DecodeError>
            where
                F: FnMut(#entry_ident<'a>, sbe_rt::EntryInfo) -> Result<(), sbe_rt::DecodeError>,
            {
                self.#try_method(f)
            }
            #doc
            #[inline]
            pub fn #try_method<E, F>(self, mut f: F) -> Result<#next_ty, E>
            where
                E: From<sbe_rt::DecodeError>,
                F: FnMut(#entry_ident<'a>, sbe_rt::EntryInfo) -> Result<(), E>,
            {
                let mut iter = #inner_access.#into_ident()?;
                let count = iter.remaining_entries();
                let block_length = iter.entry_block_length();
                let mut index = 0usize;
                for entry in &mut iter {
                    f(entry, sbe_rt::EntryInfo { index, count, block_length })?;
                    index += 1;
                }
                let inner = iter.finish()?;
                Ok(sbe_rt::Ordered { inner })
            }
        }
    }
}

fn emit_vardata_methods(
    inner_access: &proc_macro2::TokenStream,
    vd: &OwnerTailVarData,
    next_ty: &proc_macro2::TokenStream,
    span: proc_macro2::Span,
) -> proc_macro2::TokenStream {
    let method = syn::Ident::new(&vd.accessor_snake, span);
    let try_method = syn::Ident::new(&format!("try_{}", vd.accessor_snake), span);
    let into_ident = syn::Ident::new(&format!("into_{}", vd.accessor_snake), span);
    let doc = format!(
        " Read `{}` as bytes, then advance to the next tail.",
        vd.accessor_snake
    );
    let doc = super::runtime::doc_lines_tokens(&doc);
    let mut ts = quote::quote! {
        #doc
        #[inline]
        pub fn #method<F>(self, f: F) -> Result<#next_ty, sbe_rt::DecodeError>
        where
            F: FnOnce(&'a [u8]) -> Result<(), sbe_rt::DecodeError>,
        {
            self.#try_method(f)
        }
        #doc
        #[inline]
        pub fn #try_method<E, F>(self, f: F) -> Result<#next_ty, E>
        where
            E: From<sbe_rt::DecodeError>,
            F: FnOnce(&'a [u8]) -> Result<(), E>,
        {
            let (bytes, inner) = #inner_access.#into_ident()?;
            f(bytes)?;
            Ok(sbe_rt::Ordered { inner })
        }
    };
    if super::runtime::text_encoding_kind(vd.character_encoding.as_deref()).is_some() {
        let str_method = syn::Ident::new(&format!("{}_as_str", vd.accessor_snake), span);
        let try_str = syn::Ident::new(&format!("try_{}_as_str", vd.accessor_snake), span);
        let into_str = syn::Ident::new(&format!("into_{}_as_str", vd.accessor_snake), span);
        let str_doc = super::runtime::doc_lines_tokens(&format!(
            " Read `{}` as `&str`, then advance to the next tail.\n\n\
             Only this field is validated; invalid text is an error, never a sentinel.",
            vd.accessor_snake
        ));
        ts.extend(quote::quote! {
            #str_doc
            #[inline]
            pub fn #str_method<F>(self, f: F) -> Result<#next_ty, sbe_rt::DecodeError>
            where
                F: FnOnce(&'a str) -> Result<(), sbe_rt::DecodeError>,
            {
                self.#try_str(f)
            }
            #str_doc
            #[inline]
            pub fn #try_str<E, F>(self, f: F) -> Result<#next_ty, E>
            where
                E: From<sbe_rt::DecodeError>,
                F: FnOnce(&'a str) -> Result<(), E>,
            {
                let (text, inner) = #inner_access.#into_str()?;
                f(text)?;
                Ok(sbe_rt::Ordered { inner })
            }
        });
    }
    ts
}

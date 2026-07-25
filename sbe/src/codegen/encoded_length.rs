//! Encoded-length classification and code generation.
//!
//! Three strategies:
//! - `Fixed`: no groups, no varData → use existing encoder constants.
//! - `Direct`: flat groups + message varData → checked const-fn helpers.
//! - `Staged`: nested groups or entry varData → staged builder types.

use crate::structured_ir::{
    MessageGroup, MessageStructure, MessageVarData, SchemaElements, get_dim_num_layout,
    get_dimension_info, get_vardata_info, rust_type,
};
use proc_macro2::TokenStream;
use quote::format_ident;

/// How the encoded length of a message should be computed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LengthStrategy {
    Fixed,
    Direct,
    Staged,
}

/// Output of encoded-length code generation for one message.
pub(super) struct GeneratedEncodedLength {
    /// Methods on the initial encoder (`impl {Msg}Encoder`).
    pub(super) encoder_impl: TokenStream,
    /// Standalone types appended after encoder stage generation.
    pub(super) standalone: TokenStream,
}

/// Classify a message into one of the three length strategies.
pub(super) fn strategy(message: &MessageStructure) -> LengthStrategy {
    if message.groups.is_empty() && message.var_data.is_empty() {
        return LengthStrategy::Fixed;
    }
    let has_dynamic_entry = message
        .groups
        .iter()
        .any(|group| !group.groups.is_empty() || !group.var_data.is_empty());
    if has_dynamic_entry {
        LengthStrategy::Staged
    } else {
        LengthStrategy::Direct
    }
}

/// Generate encoded-length support for one message.
pub(super) fn generate(
    message: &MessageStructure,
    block_length: usize,
    header_size: usize,
    elements: &SchemaElements,
) -> GeneratedEncodedLength {
    let s = strategy(message);
    match s {
        LengthStrategy::Fixed => GeneratedEncodedLength {
            encoder_impl: TokenStream::new(),
            standalone: TokenStream::new(),
        },
        LengthStrategy::Direct => generate_direct(message, block_length, header_size, elements),
        LengthStrategy::Staged => generate_staged(message, block_length, header_size, elements),
    }
}

/// Generate uniform staged builder types for structurally dynamic messages.
fn generate_staged(
    msg: &MessageStructure,
    block_length: usize,
    header_size: usize,
    elements: &SchemaElements,
) -> GeneratedEncodedLength {
    let span = proc_macro2::Span::call_site();
    let msg_name = crate::codegen::to_pascal_case(&msg.name);
    let bl_lit = syn::LitInt::new(&block_length.to_string(), span);
    let hs_lit = syn::LitInt::new(&header_size.to_string(), span);
    let entry_ident = syn::Ident::new(&format!("{msg_name}EncodedLength"), span);

    let mut standalone = TokenStream::new();

    // ── Entry-point struct ──
    standalone.extend(quote::quote! {
        /// Exact-length calculator for this message.
        #[must_use = "length builder must be consumed"]
        pub struct #entry_ident {
            state: EncodedLengthAccumulator,
        }

        impl #entry_ident {
            pub const BLOCK_LENGTH: usize = #bl_lit;
            pub const HEADER_LENGTH: usize = #hs_lit;

            /// Start computing the encoded length.
            pub const fn new() -> Self {
                Self { state: EncodedLengthAccumulator::new(Self::BLOCK_LENGTH) }
            }
        }
    });

    // ── Pre-emit all stage structs (except entry and uniform pending) ──
    let mut stage_names: Vec<String> = Vec::new();
    let total_tail = msg.groups.len() + msg.var_data.len();
    {
        let mut idx = 0;
        for g in &msg.groups {
            idx += 1;
            if idx < total_tail {
                let next_pascal = if idx < msg.groups.len() {
                    crate::codegen::to_pascal_case(&msg.groups[idx].name)
                } else {
                    crate::codegen::to_pascal_case(&msg.var_data[idx - msg.groups.len()].name)
                };
                stage_names.push(format!("{msg_name}EncodedLengthAfter{next_pascal}"));
            } else {
                stage_names.push(format!("{msg_name}EncodedLengthComplete"));
            }
        }
        for (vi, _vd) in msg.var_data.iter().enumerate() {
            let gi = msg.groups.len() + vi;
            idx = gi + 1;
            if idx < total_tail {
                let next_pascal = crate::codegen::to_pascal_case(&msg.var_data[vi + 1].name);
                stage_names.push(format!("{msg_name}EncodedLengthAfter{next_pascal}"));
            } else {
                stage_names.push(format!("{msg_name}EncodedLengthComplete"));
            }
        }
    }
    for sn in &stage_names {
        let sid = syn::Ident::new(sn, span);
        standalone.extend(quote::quote! {
            #[doc(hidden)]
            pub struct #sid {
                state: EncodedLengthAccumulator,
            }
        });
    }

    // ── Walk tail groups + varData, emitting uniform stages ──
    let mut pending_name = entry_ident.clone();
    let total_tail = msg.groups.len() + msg.var_data.len();
    let mut tail_idx: usize = 0;

    for g in &msg.groups {
        let g_snake = syn::Ident::new(&crate::codegen::to_snake_case(&g.name), span);
        let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
        let (_, _, num_prim) = get_dim_num_layout(elements, &g.dimension_type);
        let count_ty: syn::Type = syn::parse_str(rust_type(num_prim)).unwrap();
        let ds = syn::LitInt::new(&dim_size.to_string(), span);
        let g_bl = syn::LitInt::new(&g.block_length.to_string(), span);

        let has_dynamic_entry = !g.groups.is_empty() || !g.var_data.is_empty();
        let tail_after_group = tail_idx + 1;
        let next_name = if tail_after_group < total_tail {
            let next_pascal = if tail_after_group < msg.groups.len() {
                crate::codegen::to_pascal_case(&msg.groups[tail_after_group].name)
            } else {
                crate::codegen::to_pascal_case(
                    &msg.var_data[tail_after_group - msg.groups.len()].name,
                )
            };
            syn::Ident::new(&format!("{msg_name}EncodedLengthAfter{next_pascal}"), span)
        } else {
            syn::Ident::new(&format!("{msg_name}EncodedLengthComplete"), span)
        };

        let mut entry_tail_methods = TokenStream::new();

        if has_dynamic_entry {
            // Generate nested-group methods and varData methods on a pending uniform stage.
            let pending_ident = syn::Ident::new(
                &format!(
                    "{msg_name}{}UniformEncodedLength",
                    crate::codegen::to_pascal_case(&g.name)
                ),
                span,
            );

            // Pending uniform stage struct
            standalone.extend(quote::quote! {
                #[doc(hidden)]
                #[must_use = "complete the nested shape or call finish_empty()"]
                pub struct #pending_ident {
                    state: EncodedLengthAccumulator,
                    parent_multiplier: usize,
                    declared_count: u32,
                }
            });

            // Nested groups inside the entry
            for ng in &g.groups {
                let ng_snake = syn::Ident::new(&crate::codegen::to_snake_case(&ng.name), span);
                let (_, ng_dim, _, _) = get_dimension_info(elements, &ng.dimension_type);
                let (_, _, ng_num_prim) = get_dim_num_layout(elements, &ng.dimension_type);
                let ng_count_ty: syn::Type = syn::parse_str(rust_type(ng_num_prim)).unwrap();
                let ng_ds = syn::LitInt::new(&ng_dim.to_string(), span);
                let ng_bl = syn::LitInt::new(&ng.block_length.to_string(), span);

                let is_flat_nested = ng.groups.is_empty() && ng.var_data.is_empty();
                if is_flat_nested {
                    // Flat nested group: adds dim + count * block, restores multiplier.
                    entry_tail_methods.extend(quote::quote! {
                        pub const fn #ng_snake(
                            mut self, count: #ng_count_ty,
                        ) -> Result<#next_name, sbe_rt::EncodeError> {
                            let pm = self.state.enter_group(count as usize, #ng_ds as usize, #ng_bl as usize);
                            self.state.leave_group(pm);
                            match self.state.check() {
                                Ok(()) => Ok(#next_name { state: self.state }),
                                Err(e) => Err(e),
                            }
                        }
                    });
                } else {
                    // Nested group with entry varData: enter group, return nested pending stage.
                    let nested_pending = syn::Ident::new(
                        &format!(
                            "{msg_name}{}{}UniformEncodedLength",
                            crate::codegen::to_pascal_case(&g.name),
                            crate::codegen::to_pascal_case(&ng.name)
                        ),
                        span,
                    );

                    standalone.extend(quote::quote! {
                        #[doc(hidden)]
                        pub struct #nested_pending {
                            state: EncodedLengthAccumulator,
                            parent_multiplier: usize,
                            outer_multiplier: usize,
                        }
                    });

                    entry_tail_methods.extend(quote::quote! {
                        pub const fn #ng_snake(
                            mut self, count: #ng_count_ty,
                        ) -> #nested_pending {
                            let pm = self.state.enter_group(
                                count as usize, #ng_ds as usize, #ng_bl as usize,
                            );
                            #nested_pending {
                                state: self.state,
                                parent_multiplier: pm,
                                outer_multiplier: self.parent_multiplier,
                            }
                        }
                    });

                    // VarData on the nested pending stage — fallible
                    for nvd in &ng.var_data {
                        let nvd_snake =
                            syn::Ident::new(&crate::codegen::to_snake_case(&nvd.name), span);
                        let (_, nvd_prefix, _, _) = get_vardata_info(elements, &nvd.type_name);
                        let nvd_ps = syn::LitInt::new(&nvd_prefix.to_string(), span);
                        let nvd_field = &nvd.name;
                        let mut max_chk = TokenStream::new();
                        if let Some(max) = nvd.max_length {
                            let max_lit = syn::LitInt::new(&max.to_string(), span);
                            max_chk.extend(quote::quote! {
                                if byte_len > #max_lit {
                                    self.state.fail(sbe_rt::EncodeError::VarDataTooLong {
                                        field: #nvd_field, max_length: #max_lit, actual: byte_len,
                                    });
                                    return Err(sbe_rt::EncodeError::VarDataTooLong {
                                        field: #nvd_field, max_length: #max_lit, actual: byte_len,
                                    });
                                }
                            });
                        }
                        // ponytail: nested varData completes the nested group and returns to outer
                        let back_to = next_name.clone();
                        standalone.extend(quote::quote! {
                            impl #nested_pending {
                                pub const fn #nvd_snake(
                                    mut self, byte_len: usize,
                                ) -> Result<#back_to, sbe_rt::EncodeError> {
                                    #max_chk
                                    let m = self.state.multiplier();
                                    self.state.add_scaled(#nvd_ps as usize, m);
                                    self.state.add_scaled(byte_len, m);
                                    self.state.leave_group(self.parent_multiplier);
                                    self.state.leave_group(self.outer_multiplier);
                                    match self.state.check() {
                                        Ok(()) => Ok(#back_to { state: self.state }),
                                        Err(e) => Err(e),
                                    }
                                }
                            }
                        });
                    }
                }
            }

            // Entry varData on the pending stage
            for vd in &g.var_data {
                let vd_snake = syn::Ident::new(&crate::codegen::to_snake_case(&vd.name), span);
                let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
                let ps_lit = syn::LitInt::new(&prefix_size.to_string(), span);
                let field_name = &vd.name;
                let mut max_chk = TokenStream::new();
                if let Some(max) = vd.max_length {
                    let max_lit = syn::LitInt::new(&max.to_string(), span);
                    max_chk.extend(quote::quote! {
                        if byte_len > #max_lit {
                            self.state.fail(sbe_rt::EncodeError::VarDataTooLong {
                                field: #field_name, max_length: #max_lit, actual: byte_len,
                            });
                            return Err(sbe_rt::EncodeError::VarDataTooLong {
                                field: #field_name, max_length: #max_lit, actual: byte_len,
                            });
                        }
                    });
                }

                entry_tail_methods.extend(quote::quote! {
                    pub const fn #vd_snake(
                        mut self, byte_len: usize,
                    ) -> Result<#next_name, sbe_rt::EncodeError> {
                        #max_chk
                        let m = self.state.multiplier();
                        self.state.add_scaled(#ps_lit as usize, m);
                        self.state.add_scaled(byte_len, m);
                        self.state.leave_group(self.parent_multiplier);
                        match self.state.check() {
                            Ok(()) => Ok(#next_name { state: self.state }),
                            Err(e) => Err(e),
                        }
                    }
                });
            }

            standalone.extend(quote::quote! {
                impl #pending_ident {
                    #entry_tail_methods

                    /// Complete this group when the entry count is zero.
                    /// Returns an error if the declared count is non-zero.
                    pub fn finish_empty(self)
                        -> Result<#next_name, sbe_rt::EncodeError>
                    {
                        if self.declared_count != 0 {
                            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                                declared: self.declared_count,
                                actual: 0,
                            });
                        }
                        let mut state = self.state;
                        state.leave_group(self.parent_multiplier);
                        match state.check() {
                            Ok(()) => Ok(#next_name { state }),
                            Err(e) => Err(e),
                        }
                    }
                }
            });

            // ── Zero-count forwarding: forward the next tail component's
            //     method so bids(0).asks(0) works without finish_empty() ──
            // Only forward if the next component's method name doesn't
            // collide with the pending stage's own methods.
            {
                let next_tail_idx = tail_idx + 1;
                if next_tail_idx < total_tail {
                    let (next_method_name, next_param_ty, next_param_name) =
                        if next_tail_idx < msg.groups.len() {
                            let ng = &msg.groups[next_tail_idx];
                            let (_, _, ng_num_prim) = get_dim_num_layout(elements, &ng.dimension_type);
                            let ng_count_ty: syn::Type = syn::parse_str(rust_type(ng_num_prim)).unwrap();
                            (
                                syn::Ident::new(&crate::codegen::to_snake_case(&ng.name), span),
                                ng_count_ty,
                                syn::Ident::new("count", span),
                            )
                        } else {
                            let vdi = next_tail_idx - msg.groups.len();
                            let vd = &msg.var_data[vdi];
                            (
                                syn::Ident::new(&crate::codegen::to_snake_case(&vd.name), span),
                                syn::parse_str::<syn::Type>("usize").unwrap(),
                                syn::Ident::new("byte_len", span),
                            )
                        };

                    // Check for name collision with pending stage methods
                    let method_name_str = next_method_name.to_string();
                    let has_collision = g.groups.iter().any(|ng| {
                        crate::codegen::to_snake_case(&ng.name) == method_name_str
                    }) || g.var_data.iter().any(|vd| {
                        crate::codegen::to_snake_case(&vd.name) == method_name_str
                    });

                    if !has_collision {
                        // Generate forwarding: on error, store in accumulator
                        // and continue the chain. Error surfaces at next fallible boundary.
                        standalone.extend(quote::quote! {
                            impl #pending_ident {
                                pub fn #next_method_name(
                                    self, #next_param_name: #next_param_ty,
                                ) -> #next_name {
                                    if self.declared_count != 0 {
                                        let mut state = self.state;
                                        state.fail(sbe_rt::EncodeError::GroupCountMismatch {
                                            declared: self.declared_count,
                                            actual: 0,
                                        });
                                        return #next_name { state };
                                    }
                                    // Zero-count: advance state in-place rather than
                                    // calling finish_empty(self), which would consume
                                    // self and make .state unreachable in the Err
                                    // branch (E0382 use after move).
                                    let mut state = self.state;
                                    state.leave_group(self.parent_multiplier);
                                    match state.check() {
                                        Ok(()) => #next_name { state },
                                        Err(e) => {
                                            state.fail(e);
                                            #next_name { state }
                                        }
                                    }
                                }
                            }
                        });
                    }
                }
            }

            // Uniform group method + ragged + unknown_size on the previous stage
            let g_ragged = syn::Ident::new(
                &format!("{}_ragged", crate::codegen::to_snake_case(&g.name)),
                span,
            );
            let g_unknown = syn::Ident::new(
                &format!("{}_unknown_size", crate::codegen::to_snake_case(&g.name)),
                span,
            );
            standalone.extend(quote::quote! {
                impl #pending_name {
                    /// **Uniform** group — every one of the `count` entries shares
                    /// exactly the same wire shape (same fixed block AND the same
                    /// nested-group counts / var-data lengths). The length is the
                    /// single entry shape multiplied by `count`, so no per-entry
                    /// description is needed. This is the fastest path; prefer it
                    /// whenever all entries are identical.
                    pub const fn #g_snake(
                        self, count: #count_ty,
                    ) -> #pending_ident {
                        let mut state = self.state;
                        let pm = state.enter_group(
                            count as usize, #ds as usize, #g_bl as usize,
                        );
                        #pending_ident {
                            state,
                            parent_multiplier: pm,
                            declared_count: count as u32,
                        }
                    }

                    /// **Ragged** group (known count) — entries may have *different*
                    /// shapes: e.g. each bid has a different number of orders, or
                    /// each entry carries var-data of a different length. The total
                    /// entry count is known up-front (`count`); the closure describes
                    /// each entry's *variable* contribution (nested groups via
                    /// `builder.group(dim, block, count)` and var-data via
                    /// `builder.var_data(prefix, len)`), calling `builder.add()` once
                    /// per entry. The builder verifies `add()` was called exactly
                    /// `count` times. Each entry's fixed block is pre-counted, so
                    /// `add()` only registers the entry — describe its variable tail
                    /// with `group()`/`var_data()`.
                    pub fn #g_ragged<F>(
                        mut self, count: #count_ty, f: F,
                    ) -> Result<#next_name, sbe_rt::EncodeError>
                    where
                        F: FnOnce(&mut RaggedEntryBuilder) -> sbe_rt::GroupResult,
                    {
                        let pm = self.state.enter_group(
                            count as usize, #ds as usize, #g_bl as usize,
                        );
                        // enter_group pre-counts the dimension (×pm) and all
                        // `count` entry fixed-blocks (×pm×count). For ragged
                        // entries the per-entry VARIABLE parts (nested groups,
                        // var-data) must scale by `pm` (the parent multiplier)
                        // — each entry contributes once — NOT by `pm×count`.
                        // Reset the multiplier to `pm` so the builder's
                        // group()/var_data()/add() operate at the parent scale.
                        self.state.leave_group(pm);
                        let mut builder = RaggedEntryBuilder::new(self.state, pm, 0);
                        f(&mut builder)?;
                        if builder.written != count as usize {
                            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                                declared: count as u32,
                                actual: builder.written as u32,
                            });
                        }
                        self.state = builder.state;
                        self.state.leave_group(pm);
                        match self.state.check() {
                            Ok(()) => Ok(#next_name { state: self.state }),
                            Err(e) => Err(e),
                        }
                    }

                    /// **Unknown-size** group — the entry count is discovered from
                    /// the data (e.g. draining an iterator), not known up-front.
                    /// Like the ragged path but without a declared `count`: call
                    /// `builder.add()` (or `builder.entries(n)`) once per entry;
                    /// the builder counts completed entries and rejects overflow
                    /// of the wire count type (`#count_ty`). Each `add()` contributes
                    /// the entry's fixed block, plus any `group()`/`var_data()` you
                    /// record for that entry.
                    pub fn #g_unknown<F>(
                        mut self, f: F,
                    ) -> Result<#next_name, sbe_rt::EncodeError>
                    where
                        F: FnOnce(&mut RaggedEntryBuilder) -> sbe_rt::GroupResult,
                    {
                        let max_count = #count_ty::MAX as usize;
                        let pm = self.state.multiplier();
                        // Add dimension header only (entries added by builder.add()/entries())
                        self.state.add_scaled(#ds as usize, pm);
                        let mut builder = RaggedEntryBuilder::new(self.state, pm, #g_bl as usize);
                        f(&mut builder)?;
                        if builder.written > max_count {
                            return Err(sbe_rt::EncodeError::GroupCountOverflow {
                                maximum: #count_ty::MAX as u32,
                                actual: builder.written as u32,
                            });
                        }
                        self.state = builder.state;
                        match self.state.check() {
                            Ok(()) => Ok(#next_name { state: self.state }),
                            Err(e) => Err(e),
                        }
                    }
                }
            });
        } else {
            // Flat group — simple, no pending stage.
            standalone.extend(quote::quote! {
                impl #pending_name {
                    pub const fn #g_snake(
                        self, count: #count_ty,
                    ) -> Result<#next_name, sbe_rt::EncodeError> {
                        let entries_len = match (#g_bl as usize).checked_mul(count as usize) {
                            Some(v) => v,
                            None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
                        };
                        let len = match self.state.len.checked_add(#ds as usize) {
                            Some(v) => v,
                            None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
                        };
                        let len = match len.checked_add(entries_len) {
                            Some(v) => v,
                            None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
                        };
                        Ok(#next_name { state: EncodedLengthAccumulator { len, multiplier: 1, error: None } })
                    }
                }
            });
        }

        pending_name = next_name;
        tail_idx += 1;
    }

    // VarData at message level
    for vd in &msg.var_data {
        let vd_snake = syn::Ident::new(&crate::codegen::to_snake_case(&vd.name), span);
        let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
        let ps_lit = syn::LitInt::new(&prefix_size.to_string(), span);
        let field_name = &vd.name;

        let tail_after = tail_idx + 1;
        let next_name = if tail_after < total_tail {
            let next_pascal =
                crate::codegen::to_pascal_case(&msg.var_data[tail_after - msg.groups.len()].name);
            syn::Ident::new(&format!("{msg_name}EncodedLengthAfter{next_pascal}"), span)
        } else {
            syn::Ident::new(&format!("{msg_name}EncodedLengthComplete"), span)
        };

        let mut max_chk = TokenStream::new();
        if let Some(max) = vd.max_length {
            let max_lit = syn::LitInt::new(&max.to_string(), span);
            max_chk.extend(quote::quote! {
                if byte_len > #max_lit {
                    return Err(sbe_rt::EncodeError::VarDataTooLong {
                        field: #field_name, max_length: #max_lit, actual: byte_len,
                    });
                }
            });
        }

        standalone.extend(quote::quote! {
            impl #pending_name {
                pub const fn #vd_snake(
                    self, byte_len: usize,
                ) -> Result<#next_name, sbe_rt::EncodeError> {
                    #max_chk
                    let len = match self.state.len.checked_add(#ps_lit as usize) {
                        Some(v) => v,
                        None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
                    };
                    let len = match len.checked_add(byte_len) {
                        Some(v) => v,
                        None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
                    };
                    Ok(#next_name { state: EncodedLengthAccumulator { len, multiplier: 1, error: None } })
                }
            }
        });

        pending_name = next_name;
        tail_idx += 1;
    }

    // Complete stage
    let complete_ident = syn::Ident::new(&format!("{msg_name}EncodedLengthComplete"), span);
    standalone.extend(quote::quote! {
        impl #complete_ident {
            pub const fn encoded_length(&self) -> usize { self.state.len }
            pub const fn encoded_length_with_header(&self) -> usize {
                self.state.len + #hs_lit as usize
            }
        }
    });

    GeneratedEncodedLength {
        encoder_impl: TokenStream::new(),
        standalone,
    }
}

/// Generate direct `compute_encoded_length` + checked `try_compute_encoded_length` helpers.
fn generate_direct(
    msg: &MessageStructure,
    block_length: usize,
    header_size: usize,
    elements: &SchemaElements,
) -> GeneratedEncodedLength {
    let span = proc_macro2::Span::call_site();
    let block_len_lit = syn::LitInt::new(&block_length.to_string(), span);
    let header_size_lit = syn::LitInt::new(&header_size.to_string(), span);

    // Compatibility methods (existing pattern)
    let mut compat_param_decls = Vec::new();
    let mut compat_param_names = Vec::new();
    let mut compat_body = Vec::new();

    for g in &msg.groups {
        let g_snake = crate::codegen::to_snake_case(&g.name);
        let param_ident = syn::Ident::new(&format!("{g_snake}_count"), span);
        let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
        let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), span);
        let g_bl = syn::LitInt::new(&g.block_length.to_string(), span);
        compat_body.push(quote::quote! {
            len += #dim_size_lit + #param_ident * #g_bl;
        });
        compat_param_decls.push(quote::quote! { #param_ident: usize });
        compat_param_names.push(param_ident);
    }
    for vd in &msg.var_data {
        let vd_snake = crate::codegen::to_snake_case(&vd.name);
        let param_ident = syn::Ident::new(&format!("{vd_snake}_len"), span);
        let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
        let ps = syn::LitInt::new(&prefix_size.to_string(), span);
        compat_body.push(quote::quote! { len += #ps + #param_ident; });
        compat_param_decls.push(quote::quote! { #param_ident: usize });
        compat_param_names.push(param_ident);
    }

    let compat = quote::quote! {
        /// Compute the exact SBE message body length before encoding.
        /// Parameters: one `usize` per group (entry count) and one `usize`
        /// per var-data field (byte length).
        #[inline]
        pub const fn compute_encoded_length(#(#compat_param_decls),*) -> usize {
            let mut len = #block_len_lit;
            #(#compat_body)*
            len
        }

        /// Compute the exact SBE message length including the standard
        /// message header.
        #[inline]
        pub const fn compute_encoded_length_with_message_header(
            #(#compat_param_decls),*
        ) -> usize {
            #header_size + Self::compute_encoded_length(#(#compat_param_names),*)
        }
    };

    // Checked methods with typed group counts
    let mut checked_param_decls = Vec::new();
    let mut checked_param_names = Vec::new();
    let mut checked_body = Vec::new();

    for g in &msg.groups {
        let g_snake = crate::codegen::to_snake_case(&g.name);
        let param_ident = syn::Ident::new(&format!("{g_snake}_count"), span);
        let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
        let (_, _, num_prim) = get_dim_num_layout(elements, &g.dimension_type);
        let count_ty: syn::Type = syn::parse_str(rust_type(num_prim)).unwrap();
        let ds = syn::LitInt::new(&dim_size.to_string(), span);
        let g_bl = syn::LitInt::new(&g.block_length.to_string(), span);

        checked_param_decls.push(quote::quote! { #param_ident: #count_ty });
        checked_param_names.push(param_ident.clone());

        checked_body.push(quote::quote! {
            let entries_len = match (#g_bl as usize).checked_mul(#param_ident as usize) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
            len = match len.checked_add(#ds as usize) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
            len = match len.checked_add(entries_len) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
        });
    }

    for vd in &msg.var_data {
        let vd_snake = crate::codegen::to_snake_case(&vd.name);
        let param_ident = syn::Ident::new(&format!("{vd_snake}_len"), span);
        let vd_name = &vd.name;
        let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
        let ps = syn::LitInt::new(&prefix_size.to_string(), span);

        let mut max_check = TokenStream::new();
        if let Some(max) = vd.max_length {
            let max_lit = syn::LitInt::new(&max.to_string(), span);
            let pi = param_ident.clone();
            max_check.extend(quote::quote! {
                if #pi > #max_lit {
                    return Err(sbe_rt::EncodeError::VarDataTooLong {
                        field: #vd_name,
                        max_length: #max_lit,
                        actual: #pi,
                    });
                }
            });
        }

        let pi_decl = param_ident.clone();
        checked_param_decls.push(quote::quote! { #pi_decl: usize });
        let pi_name = param_ident.clone();
        checked_param_names.push(pi_name);
        let pi_body = param_ident.clone();

        checked_body.push(quote::quote! {
            #max_check
            len = match len.checked_add(#ps as usize) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
            len = match len.checked_add(#pi_body) {
                Some(v) => v,
                None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
            };
        });
    }

    let checked = quote::quote! {
        /// Compute the exact SBE message body length with checked arithmetic.
        /// Group counts use the wire type (`u16` or `u8`); var-data lengths
        /// use `usize`.
        #[inline]
        pub fn try_compute_encoded_length(
            #(#checked_param_decls),*
        ) -> Result<usize, sbe_rt::EncodeError> {
            let mut len: usize = #block_len_lit;
            #(#checked_body)*
            Ok(len)
        }

        /// Compute the exact SBE message length including the header, with
        /// checked arithmetic.
        #[inline]
        pub fn try_compute_encoded_length_with_header(
            #(#checked_param_decls),*
        ) -> Result<usize, sbe_rt::EncodeError> {
            let body = Self::try_compute_encoded_length(#(#checked_param_names),*)?;
            body.checked_add(#header_size)
                .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)
        }
    };

    let mut encoder_impl = TokenStream::new();
    encoder_impl.extend(compat);
    encoder_impl.extend(checked);

    GeneratedEncodedLength {
        encoder_impl,
        standalone: TokenStream::new(),
    }
}

// ── Accumulator emitted into generated schema modules ──────────────────

/// Emit the `EncodedLengthAccumulator` + `RaggedEntryBuilder` helpers for staged messages.
pub(super) fn generate_support() -> TokenStream {
    quote::quote! {
        #[doc(hidden)]
        pub(crate) struct EncodedLengthAccumulator {
            len: usize,
            multiplier: usize,
            error: Option<sbe_rt::EncodeError>,
        }

        impl EncodedLengthAccumulator {
            pub(crate) const fn new(block_length: usize) -> Self {
                Self { len: block_length, multiplier: 1, error: None }
            }

            pub(crate) const fn multiplier(&self) -> usize {
                self.multiplier
            }

            pub(crate) const fn add_scaled(&mut self, unit_len: usize, repetitions: usize) {
                if self.error.is_some() { return; }
                let contribution = match unit_len.checked_mul(repetitions) {
                    Some(c) => c,
                    None => { self.error = Some(sbe_rt::EncodeError::EncodedLengthOverflow); return; }
                };
                self.len = match self.len.checked_add(contribution) {
                    Some(l) => l,
                    None => { self.error = Some(sbe_rt::EncodeError::EncodedLengthOverflow); self.len }
                };
            }

            pub(crate) const fn enter_group(
                &mut self, count: usize, dimension_length: usize, entry_block_length: usize,
            ) -> usize {
                let parent_multiplier = self.multiplier;
                self.add_scaled(dimension_length, parent_multiplier);
                self.multiplier = match parent_multiplier.checked_mul(count) {
                    Some(m) => m,
                    None => { self.error = Some(sbe_rt::EncodeError::EncodedLengthOverflow); 0 }
                };
                self.add_scaled(entry_block_length, self.multiplier);
                parent_multiplier
            }

            pub(crate) const fn leave_group(&mut self, parent_multiplier: usize) {
                self.multiplier = parent_multiplier;
            }

            pub(crate) const fn fail(&mut self, error: sbe_rt::EncodeError) {
                if self.error.is_none() { self.error = Some(error); }
            }

            pub(crate) const fn check(&self) -> Result<(), sbe_rt::EncodeError> {
                match self.error { Some(e) => Err(e), None => Ok(()) }
            }

            pub(crate) const fn finish(self, header_length: usize)
                -> Result<(usize, usize), sbe_rt::EncodeError>
            {
                if let Err(e) = self.check() { return Err(e); }
                match self.len.checked_add(header_length) {
                    Some(full) => Ok((self.len, full)),
                    None => Err(sbe_rt::EncodeError::EncodedLengthOverflow),
                }
            }
        }

        /// Builder for ragged/unknown-size entries.
        /// `entry_block_length` is 0 for known-size ragged (blocks already
        /// counted by `enter_group`) and the actual block length for
        /// unknown-size (blocks added per-entry via `add()`/`entries()`).
        #[doc(hidden)]
        pub struct RaggedEntryBuilder {
            state: EncodedLengthAccumulator,
            parent_multiplier: usize,
            entry_block_length: usize,
            pub written: usize,
        }

        impl RaggedEntryBuilder {
            fn new(state: EncodedLengthAccumulator, parent_multiplier: usize, entry_block_length: usize) -> Self {
                Self { state, parent_multiplier, entry_block_length, written: 0 }
            }

            /// Register one entry (adds entry block for unknown-size groups).
            pub fn add(&mut self) -> sbe_rt::GroupResult {
                self.state.add_scaled(self.entry_block_length, self.parent_multiplier);
                self.written += 1;
                Ok(())
            }

            /// Register N flat entries at once (for fixed-width unknown-size groups).
            pub fn entries(&mut self, n: usize) -> sbe_rt::GroupResult {
                for _ in 0..n {
                    self.state.add_scaled(self.entry_block_length, self.parent_multiplier);
                }
                self.written += n;
                Ok(())
            }

            /// Add a nested group dimension + entries.
            pub fn group(&mut self, dim: usize, block: usize, count: usize) -> sbe_rt::GroupResult {
                let pm = self.state.enter_group(count, dim, block);
                self.state.leave_group(pm);
                self.state.check()?;
                Ok(())
            }

            /// Add a nested **ragged** group — entries may differ (e.g. var-data
            /// of differing length per entry). Adds the group dimension once,
            /// then the closure describes each entry (`sub.add()` for the entry
            /// block, `sub.var_data(...)` for per-entry var-data). The closure
            /// receives a sub-builder scoped to this group's parent multiplier.
            pub fn group_ragged<F>(
                &mut self, dim: usize, entry_block: usize, f: F,
            ) -> sbe_rt::GroupResult
            where
                F: FnOnce(&mut RaggedEntryBuilder) -> sbe_rt::GroupResult,
            {
                let pm = self.state.multiplier();
                self.state.add_scaled(dim, pm);
                let state = core::mem::replace(&mut self.state, EncodedLengthAccumulator::new(0));
                let mut sub = RaggedEntryBuilder::new(state, pm, entry_block);
                f(&mut sub)?;
                self.state = sub.state;
                self.state.check()?;
                Ok(())
            }

            /// Add a varData field for the current entry.
            pub fn var_data(&mut self, prefix: usize, byte_len: usize) -> sbe_rt::GroupResult {
                self.state.add_scaled(prefix, self.parent_multiplier);
                self.state.add_scaled(byte_len, self.parent_multiplier);
                self.state.check()?;
                Ok(())
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{LengthStrategy, strategy};
    use crate::structured_ir::{parse_message_structure, partition_tokens};
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("schemas")
            .join(name)
    }

    fn strategy_for(
        path: &std::path::Path,
        message_name: &str,
    ) -> Result<LengthStrategy, Box<dyn std::error::Error>> {
        let ir = crate::parse_file(path)?;
        let elements = partition_tokens(&ir.tokens);
        let message_tokens = elements
            .messages
            .iter()
            .find(|tokens| tokens[0].name == message_name)
            .ok_or_else(|| format!("missing message {message_name}"))?;
        let message = parse_message_structure(message_tokens, &elements);
        Ok(strategy(&message))
    }

    #[test]
    fn classifies_repository_message_shapes() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            strategy_for(&fixture("basic-schema.xml"), "TestMessage50001")?,
            LengthStrategy::Fixed,
        );
        assert_eq!(
            strategy_for(&fixture("basic-variable-length-schema.xml"), "TestMessage1")?,
            LengthStrategy::Direct,
        );
        assert_eq!(
            strategy_for(&fixture("basic-group-schema.xml"), "TestMessage1")?,
            LengthStrategy::Direct,
        );
        assert_eq!(
            strategy_for(&fixture("group-with-data-schema.xml"), "TestMessage1")?,
            LengthStrategy::Staged,
        );
        assert_eq!(
            strategy_for(&fixture("nested-group-schema.xml"), "Top")?,
            LengthStrategy::Staged,
        );
        assert_eq!(
            strategy_for(&fixture("l3-orderbook-schema.xml"), "L3Book")?,
            LengthStrategy::Staged,
        );
        Ok(())
    }
}

//! Message decoder codegen.
//!
//! `generate_message_decoder` emits the decoder flyweight for a message: the
//! schema marker struct, the fixed-field `&self` accessor surface (primitives,
//! arrays, composites, enums, sets), group/var-data tail accessors, `verify`,
//! `TryFrom`/`SbeMessage`/`Display` impls, group decoders, consuming tail
//! stages, and optional domain DTOs. Depends on [`super::decoder_display`],
//! [`super::group_decoder`], [`super::tail_stages`], [`super::domain_cluster`],
//! [`super::conversion_helpers`], [`super::runtime`], and `structured_ir`.

use quote::format_ident;

use crate::ir::{ByteOrder, Presence, PrimitiveType};
use crate::structured_ir::*;

use super::conversion_helpers::{field_has_conversion_free, resolve_field_ident};
use super::decoder_display::generate_decoder_display;
use super::domain_cluster::generate_domain_objects;
use super::group_decoder::generate_group_decoder;
use super::runtime::{
    constant_value_expr, deprecated_attr_tokens, doc_attr_tokens, emit_field_consts,
    schema_marker_ident, to_pascal_case, to_snake_case,
};
use super::tail_stages::generate_decoder_consuming_stages;

pub(crate) fn generate_message_decoder(
    msg: &MessageStructure,
    elements: &SchemaElements,
    schema_markers: &mut std::collections::HashSet<String>,
    byte_order: ByteOrder,
    schema_id: u16,
    schema_version: u16,
    header_type: &str,
    schema_name: &str,
    multi_message: bool,
    enable_display_debug: bool,
    enable_meta_attributes: bool,
    enable_dispatch: bool,
    domain_objects: bool,
    domain_var_data: crate::config::DomainVarData,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    _unchecked_companions: bool,
    hooks: &crate::config::Hooks,
    schema: &crate::Schema,
) -> (proc_macro2::TokenStream, String) {
    let raw_name = &msg.name;
    let name = to_pascal_case(raw_name);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    // Prefer the resolved message block length (includes schema-declared
    // padding via `blockLength="…"`). Fall back to a tight field-span only if
    // resolve left it zero (should not happen for real messages).
    // Constant fields have zero wire footprint; skip them so block_length stays
    // 0 for constant-only messages (e.g. value_ref MsgTwo–MsgFive).
    let computed_block_length = msg
        .fields
        .iter()
        .filter(|f| f.presence != Presence::Constant)
        .fold(0, |acc, f| {
            let size = f.field_type.size();
            acc.max(f.offset + size)
        });
    let block_length = msg.block_length.max(computed_block_length);

    let header_pascal = to_pascal_case(header_type);
    let (header_bl, header_ti, header_si, header_vr, header_ti_constant, header_si_constant) = {
        let mut bl = "block_length".to_string();
        let mut ti = "template_id".to_string();
        let mut si = "schema_id".to_string();
        let mut vr = "version".to_string();
        let mut ti_constant = false;
        let mut si_constant = false;
        if let Some(comp) = elements
            .composites
            .iter()
            .find(|c| c[0].name == header_type)
        {
            let members = parse_composite_members(comp);
            for m in members {
                let lower = m.name.to_lowercase();
                let is_constant = matches!(
                    m.member_type,
                    MemberType::Primitive {
                        presence: Presence::Constant,
                        ..
                    }
                );
                if lower.contains("blocklength") {
                    bl = to_snake_case(&m.name);
                } else if lower.contains("templateid") {
                    ti = to_snake_case(&m.name);
                    ti_constant = is_constant;
                } else if lower.contains("schemaid") {
                    si = to_snake_case(&m.name);
                    si_constant = is_constant;
                } else if lower.contains("version") {
                    vr = to_snake_case(&m.name);
                }
            }
        }
        (bl, ti, si, vr, ti_constant, si_constant)
    };

    let header_size = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| c[0].encoding.offset)
        .unwrap_or(8);

    let is_fixed = msg.groups.is_empty() && msg.var_data.is_empty();
    let encoded_length = header_size + block_length;
    let mut max_tail = 0usize;
    for g in &msg.groups {
        let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
        max_tail = max_tail.saturating_add(dim_size.saturating_add(g.effective_block_length()));
    }
    for vd in &msg.var_data {
        let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
        max_tail = max_tail.saturating_add(prefix_size.saturating_add(vd.max_length.unwrap_or(0)));
    }
    let max_encoded_length = header_size
        .saturating_add(block_length)
        .saturating_add(max_tail);

    let _name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let decoder_ident =
        syn::Ident::new(&format!("{}Decoder", name), proc_macro2::Span::call_site());
    let _header_pascal_ident = syn::Ident::new(&header_pascal, proc_macro2::Span::call_site());
    let _header_bl_ident = syn::Ident::new(&header_bl, proc_macro2::Span::call_site());
    let _header_ti_ident = syn::Ident::new(&header_ti, proc_macro2::Span::call_site());
    let _header_si_ident = syn::Ident::new(&header_si, proc_macro2::Span::call_site());
    let _header_vr_ident = syn::Ident::new(&header_vr, proc_macro2::Span::call_site());

    let mut ts = proc_macro2::TokenStream::new();
    let schema_id_lit = syn::LitInt::new(&schema_id.to_string(), proc_macro2::Span::call_site());
    let schema_version_lit =
        syn::LitInt::new(&schema_version.to_string(), proc_macro2::Span::call_site());
    let msg_id_lit = syn::LitInt::new(&msg.id.to_string(), proc_macro2::Span::call_site());
    let bl_lit = syn::LitInt::new(&block_length.to_string(), proc_macro2::Span::call_site());
    let hdr_size_lit = syn::LitInt::new(&header_size.to_string(), proc_macro2::Span::call_site());
    let encoded_len_lit =
        syn::LitInt::new(&encoded_length.to_string(), proc_macro2::Span::call_site());

    // Schema constants struct — no turbofish, shared by encoder and decoder.
    // Disambiguate when a composite/enum/set already claims `{Name}Schema`.
    // Emitted *before* the decoder so message `description` rustdoc attaches
    // to the decoder/encoder types, not this marker.
    let schema_ident = schema_marker_ident(&name, schema_markers);
    let marker_name = schema_ident.to_string();
    schema_markers.insert(marker_name.clone());
    ts.extend(quote::quote! {
        pub struct #schema_ident;
        impl #schema_ident {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #bl_lit;
            pub const HEADER_LENGTH: usize = #hdr_size_lit;
        }
    });

    // Fixed-block-only decoders (no groups/var-data) are Copy: they have no
    // tail cursor, so copying cannot weaken an ordering invariant. Tailed
    // decoders are NOT Copy/Clone — consumption enforces wire order.
    let derive_attr = if is_fixed {
        quote::quote! { #[derive(Clone, Copy)] }
    } else {
        quote::quote! {}
    };
    if let Some(ref desc) = msg.description {
        ts.extend(doc_attr_tokens(desc));
    }
    ts.extend(quote::quote! {
        #derive_attr
        pub struct #decoder_ident<'a> {
            pub(crate) buf: &'a [u8],
            pub(crate) pos: usize,
            pub(crate) acting_version: u16,
            pub(crate) acting_block_length: usize,
        }
    });

    let mut impl_body = proc_macro2::TokenStream::new();
    if is_fixed {
        impl_body.extend(quote::quote! {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #bl_lit;
            const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == #bl_lit);
            /// Schema-declared message header size in bytes.
            pub const HEADER_LENGTH: usize = #hdr_size_lit;
            /// Stack-allocate with `let mut buf = [0u8; Msg::ENCODED_LENGTH];`
            /// Header-inclusive fixed length (header + body). For session
            /// framing, app payload starts at `frame[Self::ENCODED_LENGTH..]`.
            pub const ENCODED_LENGTH: usize = #encoded_len_lit;
            const _ENCODED_LEN: () = assert!(Self::ENCODED_LENGTH >= Self::BLOCK_LENGTH);
            /// Slice after one full header-inclusive message of this type
            /// (e.g. SessionMessageHeader then application payload).
            #[inline]
            pub fn after_this_message(frame: &[u8]) -> Option<&[u8]> {
                if frame.len() < Self::ENCODED_LENGTH {
                    return None;
                }
                Some(&frame[Self::ENCODED_LENGTH..])
            }

            /// Absolute offset of this message within the original buffer
            /// (the `message_offset` argument passed to `wrap`).
            #[inline]
            pub const fn message_offset(&self) -> usize {
                self.pos.saturating_sub(Self::HEADER_LENGTH)
            }

            /// Absolute current read cursor within the original buffer.
            #[inline]
            pub const fn limit(&self) -> usize {
                self.pos + self.acting_block_length
            }

            /// The complete original buffer this decoder wraps.
            #[inline]
            pub fn buffer(&self) -> &'a [u8] {
                self.buf
            }

            /// Bytes after this message in the original buffer
            /// (e.g. the application payload following a SessionMessageHeader).
            /// Returns the slice starting after header + block body.
            /// Clamps to `buf.len()` so truncated/invalid data returns
            /// an empty slice rather than panicking.
            #[inline]
            pub fn remaining(&self) -> &'a [u8] {
                let end = (self.pos + self.acting_block_length).min(self.buf.len());
                &self.buf[end..]
            }
        });
    } else {
        const STACK_LIMIT: usize = 65536;
        let max_encoded_capped = max_encoded_length.min(STACK_LIMIT);
        let max_encoded_lit = syn::LitInt::new(
            &max_encoded_capped.to_string(),
            proc_macro2::Span::call_site(),
        );
        let is_capped = max_encoded_length > STACK_LIMIT;
        let max_len_suffix: proc_macro2::TokenStream = if is_capped {
            // When theoretical max exceeds 64KB, do NOT emit MAX_ENCODED_LENGTH —
            // the constant would be a dangerous lie. Use EncodedLength instead.
            quote::quote! {}
        } else {
            let max_doc = " Upper bound of any encoded form of this message (header + body). \
                 Prefer exact sizing via `Encoder::compute_length()` / the staged \
                 `*EncodedLength` builder when the message has groups or var-data; \
                 a stack `[0u8; Self::MAX_ENCODED_LENGTH]` is fine only when this \
                 constant is a true fixed upper bound you intend to use.";
            let max_doc_lit = syn::LitStr::new(max_doc, proc_macro2::Span::call_site());
            quote::quote! {
                #[doc = #max_doc_lit]
                pub const MAX_ENCODED_LENGTH: usize = #max_encoded_lit;
                const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
            }
        };
        impl_body.extend(quote::quote! {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #bl_lit;
            const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == #bl_lit);
            /// Schema-declared message header size in bytes.
            pub const HEADER_LENGTH: usize = #hdr_size_lit;
            #max_len_suffix
        });
    }

    // Version-aware minimum fixed extent: every field readable at
    // `acting_version` must fit in the body buffer (HFT-001).
    let mut min_extent_arms = proc_macro2::TokenStream::new();
    {
        // Collect unique since_version thresholds and the max end offset
        // of fields at that version.
        let mut by_version: Vec<(u16, usize)> = Vec::new();
        for f in &msg.fields {
            let end = f.offset.saturating_add(f.field_type.size());
            by_version.push((f.since_version, end));
        }
        by_version.sort_by_key(|(v, _)| *v);
        // Build a stepwise max: for each distinct version V, extent is max of
        // all fields with since_version <= V.
        let mut versions: Vec<u16> = by_version.iter().map(|(v, _)| *v).collect();
        versions.sort_unstable();
        versions.dedup();
        // Seed with v0 extent (always present for acting_version in range).
        let mut m0 = 0usize;
        for f in &msg.fields {
            if f.since_version == 0 {
                m0 = m0.max(f.offset.saturating_add(f.field_type.size()));
            }
        }
        let m0_lit = syn::LitInt::new(&m0.to_string(), proc_macro2::Span::call_site());
        min_extent_arms.extend(quote::quote! {
            let mut m = #m0_lit;
        });
        for &v in &versions {
            if v == 0 {
                continue; // already seeded; avoid `acting_version >= 0` (always true)
            }
            let mut m = 0usize;
            for f in &msg.fields {
                if f.since_version <= v {
                    m = m.max(f.offset.saturating_add(f.field_type.size()));
                }
            }
            let v_lit = syn::LitInt::new(&v.to_string(), proc_macro2::Span::call_site());
            let m_lit = syn::LitInt::new(&m.to_string(), proc_macro2::Span::call_site());
            min_extent_arms.extend(quote::quote! {
                if acting_version >= #v_lit {
                    m = #m_lit;
                }
            });
        }
    }

    impl_body.extend(quote::quote! {
        /// Minimum body bytes needed to safely read every fixed field present
        /// at `acting_version` (version-aware; not always full `BLOCK_LENGTH`).
        #[inline]
        pub const fn min_readable_fixed_extent(acting_version: u16) -> usize {
            #min_extent_arms
            m
        }

        /// Wrap a buffer for decoding at **message start** with bounds checks.
        /// Fields are at `message_offset + HEADER_LENGTH + field_offset`.
        ///
        /// Validates that the body holds `max(acting_block_length,
        /// min_readable_fixed_extent(acting_version))` bytes so required
        /// accessors never read out of bounds from safe code.
        ///
        /// # Migration from sbe-tool
        ///
        /// sbe-tool Rust `wrap` takes the **body** offset (usually
        /// `message_start + 8`). ergo-sbe takes the **message** start so the
        /// same offset works for `wrap`, `decode`, and claim buffers.
        #[inline]
        pub fn wrap(
            buf: &'a [u8],
            message_offset: usize,
            acting_block_length: usize,
            acting_version: u16,
        ) -> Result<Self, sbe_rt::DecodeError> {
            let Some(body_pos) = message_offset.checked_add(Self::HEADER_LENGTH) else {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "message header",
                    needed: Self::HEADER_LENGTH,
                    available: buf.len().saturating_sub(message_offset),
                });
            };
            let available_body = buf.len().saturating_sub(body_pos);
            let min_fixed = Self::min_readable_fixed_extent(acting_version);
            let body_need = if acting_block_length > min_fixed {
                acting_block_length
            } else {
                min_fixed
            };
            if body_need > available_body {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "message body",
                    needed: Self::HEADER_LENGTH.saturating_add(body_need),
                    available: buf.len().saturating_sub(message_offset),
                });
            }
            // SAFETY: body_need bytes after header are in-bounds.
            Ok(unsafe { Self::wrap_unchecked(buf, message_offset, acting_block_length, acting_version) })
        }

        /// Private zero-check external-metadata wrap core (HFT-008 keep=false).
        ///
        /// # Safety
        /// `message_offset + HEADER_LENGTH + max(acting_block_length,
        /// min_readable_fixed_extent(acting_version))` must not overflow and
        /// must be ≤ `buf.len()`.
        #[inline]
        pub unsafe fn wrap_unchecked(
            buf: &'a [u8],
            message_offset: usize,
            acting_block_length: usize,
            acting_version: u16,
        ) -> Self {
            let body_pos = message_offset + Self::HEADER_LENGTH;
            Self {
                buf,
                pos: body_pos,
                acting_block_length,
                acting_version,
            }
        }
    });

    {
        let hs = syn::LitInt::new(&header_size.to_string(), proc_macro2::Span::call_site());
        let hp = syn::Ident::new(&header_pascal, proc_macro2::Span::call_site());
        let hsi = syn::Ident::new(&header_si, proc_macro2::Span::call_site());
        let hti = syn::Ident::new(&header_ti, proc_macro2::Span::call_site());
        let hbl = syn::Ident::new(&header_bl, proc_macro2::Span::call_site());
        let hvr = syn::Ident::new(&header_vr, proc_macro2::Span::call_site());
        let en = syn::LitStr::new(&schema_name, proc_macro2::Span::call_site());
        let template_id_validation = if header_ti_constant {
            quote::quote! {}
        } else {
            quote::quote! {
                if template_id != Self::TEMPLATE_ID {
                    return Err(sbe_rt::DecodeError::WrongSchema {
                        expected: Self::TEMPLATE_ID,
                        actual: template_id,
                        expected_name: #en,
                    });
                }
            }
        };
        let schema_id_validation = if header_si_constant {
            quote::quote! {}
        } else {
            quote::quote! {
                if schema_id != Self::SCHEMA_ID {
                    return Err(sbe_rt::DecodeError::WrongSchema {
                        expected: Self::SCHEMA_ID,
                        actual: schema_id,
                        expected_name: #en,
                    });
                }
            }
        };
        impl_body.extend(quote::quote! {
            /// Decode a framed message at **message start** (`pos` = first
            /// byte of the header). Validates header fields and the
            /// version-aware fixed body extent. See [`Self::wrap`] for the
            /// message-start vs sbe-tool body-offset migration note.
            #[inline]
            pub fn decode(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
                if #hs > buf.len().saturating_sub(pos) {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "message header",
                        needed: #hs,
                        available: buf.len().saturating_sub(pos),
                    });
                }
                let header_bytes: [u8; #hs] = read_bytes::<#hs>(buf, pos);
                let header = #hp(header_bytes);
                let template_id = sbe_rt::checked_header_u16(
                    "templateId",
                    header.#hti() as u64,
                )?;
                #template_id_validation
                let schema_id = sbe_rt::checked_header_u16(
                    "schemaId",
                    header.#hsi() as u64,
                )?;
                #schema_id_validation
                let acting_block_length = sbe_rt::checked_header_usize(
                    "blockLength",
                    header.#hbl() as u64,
                )?;
                let acting_version = sbe_rt::checked_header_u16(
                    "version",
                    header.#hvr() as u64,
                )?;
                // Shared path with wrap: version-aware min fixed extent.
                Self::wrap(buf, pos, acting_block_length, acting_version)
            }

            /// Private zero-check framed decode core (HFT-008 keep=false).
            ///
            /// # Safety
            /// Header and version-readable fixed body for this template must
            /// be fully in-bounds at `pos`.
            #[inline]
            pub unsafe fn decode_unchecked(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
                // Still validates schema/template identity (protocol, not memory).
                let header_bytes: [u8; #hs] = unsafe { read_bytes_unchecked::<#hs>(buf, pos) };
                let header = #hp(header_bytes);
                let template_id = sbe_rt::checked_header_u16(
                    "templateId",
                    header.#hti() as u64,
                )?;
                #template_id_validation
                let schema_id = sbe_rt::checked_header_u16(
                    "schemaId",
                    header.#hsi() as u64,
                )?;
                #schema_id_validation
                let acting_block_length = sbe_rt::checked_header_usize(
                    "blockLength",
                    header.#hbl() as u64,
                )?;
                let acting_version = sbe_rt::checked_header_u16(
                    "version",
                    header.#hvr() as u64,
                )?;
                Ok(unsafe { Self::wrap_unchecked(buf, pos, acting_block_length, acting_version) })
            }
        });
    }

    impl_body.extend(syn::parse_str::<proc_macro2::TokenStream>(
        "#[inline]\n    pub const fn acting_version(&self) -> u16 {\n        self.acting_version\n    }\n\n    pub const fn acting_block_length(&self) -> usize {\n        self.acting_block_length\n    }\n\n"
    ).unwrap());

    for f in &msg.fields {
        let fname_snake = to_snake_case(&f.name);
        let offset = f.offset;
        let since = f.since_version;
        // In converter mode, raw accessors are suffixed _wire when a domain
        // type is configured so the concrete converted method takes the
        // original name.
        let wire_name =
            field_has_conversion_free(f, conversions).then(|| format!("{fname_snake}_wire"));
        let method_name = wire_name.as_deref().unwrap_or(&fname_snake);
        const DECODER_RESERVED: &[&str] = &[
            "remaining",
            "message_offset",
            "limit",
            "buffer",
            "wrap",
            "wrap_unchecked",
            "decode",
            "decode_unchecked",
            "min_readable_fixed_extent",
            "header",
            "encoded_length",
            "encoded_length_with_header",
            "as_body_bytes",
            "as_bytes_with_header",
            "as_ref_opt",
            "verify",
            "acting_version",
            "acting_block_length",
            // Consuming stage transition (self → Self).
            "rewind",
        ];
        let fname_ident = resolve_field_ident(&fname_snake, &wire_name, DECODER_RESERVED);

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_type = rust_type(*prim);
                let r_type_ty: syn::Type = syn::parse_str(r_type).unwrap();
                let prim_size = prim.size();
                let offset_lit =
                    syn::LitInt::new(&offset.to_string(), proc_macro2::Span::call_site());
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());
                let order_fn = format_ident!("from_{order_suffix}_bytes");

                if f.presence == Presence::Constant {
                    if let Some(ref val) = f.constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            let val_lit = syn::LitStr::new(val, proc_macro2::Span::call_site());
                            impl_body.extend(quote::quote! {
                                #[inline]
                                pub const fn #fname_ident(&self) -> &'static str {
                                    #val_lit
                                }
                            });
                        } else {
                            let expr = constant_value_expr(*prim, val);
                            let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                            if let Some(ref desc) = f.description {
                                impl_body.extend(doc_attr_tokens(desc));
                            }
                            impl_body.extend(quote::quote! {
                                #[inline]
                                pub const fn #fname_ident(&self) -> #r_type_ty {
                                    #expr_parsed
                                }
                            });
                        }
                    }
                } else if let Some(len) = length {
                    let len_val = *len;
                    let len_lit =
                        syn::LitInt::new(&len_val.to_string(), proc_macro2::Span::call_site());
                    let _since_lit =
                        syn::LitInt::new(&since.to_string(), proc_macro2::Span::call_site());
                    let offset_end = offset + prim_size * len_val;
                    let offset_end_lit =
                        syn::LitInt::new(&offset_end.to_string(), proc_macro2::Span::call_site());
                    let total_size = prim_size * len_val;
                    let total_size_lit =
                        syn::LitInt::new(&total_size.to_string(), proc_macro2::Span::call_site());
                    // Runtime array accessor: non-const fn, bulk try_into + unrolled parsing
                    let offset_lit =
                        syn::LitInt::new(&offset.to_string(), proc_macro2::Span::call_site());
                    let since_lit =
                        syn::LitInt::new(&since.to_string(), proc_macro2::Span::call_site());
                    let ps_lit =
                        syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());
                    let fn_name_ident = syn::Ident::new(&f.name, proc_macro2::Span::call_site());

                    // Build unrolled element parses via direct constant indexing of a
                    // bulk-read local `all` array. One bulk read (single bounds check
                    // via read_bytes) + direct constant indexing (no per-element
                    // bounds check). This is the fastest safe-mode shape: sbe-tool's
                    // 4x per-element try_into is slower here because LLVM cannot elide
                    // the redundant checks when the offset is runtime-derived.
                    let mut elements: Vec<proc_macro2::TokenStream> = Vec::new();
                    for i in 0..len_val {
                        let start = i * prim_size;
                        let end = start + prim_size;
                        let byte_indices: Vec<proc_macro2::TokenStream> = (start..end)
                            .map(|idx| quote::quote! { all[#idx] })
                            .collect();
                        elements.push(quote::quote! {
                            #r_type_ty::#order_fn([#(#byte_indices),*])
                        });
                    }

                    let fn_snake_ident = fname_ident.clone();
                    // Fixed-length array accessors are INFALLIBLE: a fixed array that
                    // lies within the message body is guaranteed in-bounds by the
                    // version/block-length check below (and by wrap, which validates the
                    // body extent). Returning `Result` here is over-cautious, diverges
                    // from sbe-tool (which returns `[T; N]`), and adds Result+unwrap
                    // overhead that measurably slows decode. OOB only happens for a
                    // structurally malformed buffer shorter than its declared
                    // block_length, in which case read_bytes panics — same as sbe-tool's
                    // try_into. This matches sbe-tool's `[T; N]` signature and perf.
                    // Skip `acting_version < 0` (always false for u16); keep block-length guard.
                    let version_guard = if since > 0 {
                        quote::quote! {
                            if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
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
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fn_snake_ident(&self) -> [#r_type_ty; #len_lit] {
                            #version_guard
                            let offset = self.pos + #offset_lit;
                            let all: [u8; #total_size_lit] = unsafe { read_bytes_unchecked::<#total_size_lit>(self.buf, offset) };
                            [#(#elements),*]
                        }
                    });
                    // Destination-buffer copy for byte-width arrays (Java getVehicleCode(byte[])).
                    if prim_size == 1 {
                        let copy_ident = syn::Ident::new(
                            &format!("copy_{}", fname_snake),
                            proc_macro2::Span::call_site(),
                        );
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub fn #copy_ident(&self, dst: &mut [u8]) -> usize {
                                let src = self.#fn_snake_ident();
                                let n = src.len().min(dst.len());
                                let mut i = 0usize;
                                while i < n {
                                    dst[i] = src[i] as u8;
                                    i += 1;
                                }
                                n
                            }
                        });
                    }
                } else {
                    if f.presence == Presence::Optional {
                        let null_val = f.null_value.unwrap_or(0);
                        let null_check_expr = if *prim == PrimitiveType::Float {
                            format!("val.to_bits() == {null_val} as u32")
                        } else if *prim == PrimitiveType::Double {
                            format!("val.to_bits() == {null_val}")
                        } else {
                            format!("val == {null_val}_u64 as {r_type}")
                        };
                        let _since_lit =
                            syn::LitInt::new(&since.to_string(), proc_macro2::Span::call_site());
                        let offset_end = offset + prim_size;
                        let _offset_end_lit = syn::LitInt::new(
                            &offset_end.to_string(),
                            proc_macro2::Span::call_site(),
                        );
                        if let Some(ref desc) = f.description {
                            impl_body.extend(doc_attr_tokens(desc));
                        }
                        impl_body.extend(deprecated_attr_tokens(f.deprecated));
                        let version_guard = if since > 0 {
                            format!(
                                "if self.acting_version < {since} || {offset_end} > self.acting_block_length {{\n\
                                     return None;\n\
                                 }}\n"
                            )
                        } else {
                            format!(
                                "if {offset_end} > self.acting_block_length {{\n\
                                     return None;\n\
                                 }}\n"
                            )
                        };
                        let accessor = format!(
                            "#[inline]\n\
                             pub fn {snake}(&self) -> Option<{rt}> {{\n\
                                 {version_guard}\
                                 let offset = self.pos + {offset};\n\
                                 let val = {rt}::{order}(unsafe {{ read_bytes_unchecked::<{ps}>(self.buf, offset) }});\n\
                                 if {null_check} {{\n\
                                     None\n\
                                 }} else {{\n\
                                     Some(val)\n\
                                 }}\n\
                             }}\n",
                            snake = fname_ident,
                            rt = r_type,
                            version_guard = version_guard,
                            offset = offset,
                            order = order_fn,
                            ps = prim_size,
                            null_check = null_check_expr,
                        );
                        impl_body
                            .extend(syn::parse_str::<proc_macro2::TokenStream>(&accessor).unwrap());
                    } else if since > 0 {
                        let since_lit =
                            syn::LitInt::new(&since.to_string(), proc_macro2::Span::call_site());
                        let offset_end = offset + prim_size;
                        let offset_end_lit = syn::LitInt::new(
                            &offset_end.to_string(),
                            proc_macro2::Span::call_site(),
                        );
                        if let Some(ref desc) = f.description {
                            impl_body.extend(doc_attr_tokens(desc));
                        }
                        impl_body.extend(deprecated_attr_tokens(f.deprecated));
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub fn #fname_ident(&self) -> Option<#r_type_ty> {
                                if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                    return None;
                                }
                                let offset = self.pos + #offset_lit;
                                Some(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }))
                            }
                        });
                    } else {
                        if let Some(ref desc) = f.description {
                            impl_body.extend(doc_attr_tokens(desc));
                        }
                        impl_body.extend(deprecated_attr_tokens(f.deprecated));
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub fn #fname_ident(&self) -> #r_type_ty {
                                let offset = self.pos + #offset_lit;
                                #r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) })
                            }
                        });
                    }
                }
            }
            FieldType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target_name = to_pascal_case(comp_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let offset_lit =
                    syn::LitInt::new(&offset.to_string(), proc_macro2::Span::call_site());
                let comp_size_lit =
                    syn::LitInt::new(&comp_size.to_string(), proc_macro2::Span::call_site());
                let target_decoder_name = syn::Ident::new(
                    &format!("{}Decoder", target_name),
                    proc_macro2::Span::call_site(),
                );

                // Default: flyweight (zero-copy, reads directly from buffer)
                if since > 0 {
                    let since_lit =
                        syn::LitInt::new(&since.to_string(), proc_macro2::Span::call_site());
                    let offset_end = offset + comp_size;
                    let offset_end_lit =
                        syn::LitInt::new(&offset_end.to_string(), proc_macro2::Span::call_site());
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fname_ident(&self) -> Option<#target_decoder_name<'_>> {
                            if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                return None;
                            }
                            let offset = self.pos + #offset_lit;
                            Some(#target_decoder_name { buf: self.buf, pos: offset })
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fname_ident(&self) -> #target_decoder_name<'_> {
                            let offset = self.pos + #offset_lit;
                            #target_decoder_name { buf: self.buf, pos: offset }
                        }
                    });
                }

                let as_struct_ident = syn::Ident::new(
                    &format!("{}_value", fname_snake),
                    proc_macro2::Span::call_site(),
                );
                if since > 0 {
                    let since_lit =
                        syn::LitInt::new(&since.to_string(), proc_macro2::Span::call_site());
                    let offset_end = offset + comp_size;
                    let offset_end_lit =
                        syn::LitInt::new(&offset_end.to_string(), proc_macro2::Span::call_site());
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #as_struct_ident(&self) -> Option<#target_ident> {
                            if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                return None;
                            }
                            let offset = self.pos + #offset_lit;
                            Some(#target_ident(unsafe { read_bytes_unchecked::<#comp_size_lit>(self.buf, offset) }))
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #as_struct_ident(&self) -> #target_ident {
                            let offset = self.pos + #offset_lit;
                            #target_ident(unsafe { read_bytes_unchecked::<#comp_size_lit>(self.buf, offset) })
                        }
                    });
                }

                // no lazy alias generated, base accessor is canonical; delete branch if lazy aliases never return
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
                let offset_lit =
                    syn::LitInt::new(&offset.to_string(), proc_macro2::Span::call_site());
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());
                let order_fn = format_ident!("from_{order_suffix}_bytes");

                if f.presence == Presence::Constant {
                    if let Some(ref val) = f.constant_value {
                        let variant = val.rsplit('.').next().unwrap_or(val);
                        let pascal_variant = to_pascal_case(variant);
                        let variant_ident =
                            syn::Ident::new(&pascal_variant, proc_macro2::Span::call_site());
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub const fn #fname_ident(&self) -> #target_ident {
                                #target_ident::#variant_ident
                            }
                        });
                    }
                } else if since > 0 {
                    let since_lit =
                        syn::LitInt::new(&since.to_string(), proc_macro2::Span::call_site());
                    let offset_end = offset + prim_size;
                    let offset_end_lit =
                        syn::LitInt::new(&offset_end.to_string(), proc_macro2::Span::call_site());
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fname_ident(&self) -> Option<#target_ident> {
                            if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                return None;
                            }
                            let offset = self.pos + #offset_lit;
                            Some(#target_ident::from_raw(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) })))
                        }
                    });
                    if crate::structured_ir::is_bool_value_enum(elements, enum_name) {
                        let fname_bool = quote::format_ident!("{}_bool", fname_snake);
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub fn #fname_bool(&self) -> Option<bool> {
                                self.#fname_ident().map(bool::from)
                            }
                        });
                    }
                } else {
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fname_ident(&self) -> #target_ident {
                            let offset = self.pos + #offset_lit;
                            #target_ident::from_raw(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }))
                        }
                    });
                    if crate::structured_ir::is_bool_value_enum(elements, enum_name) {
                        let fname_bool = quote::format_ident!("{}_bool", fname_snake);
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub fn #fname_bool(&self) -> bool {
                                bool::from(self.#fname_ident())
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
                let offset_lit =
                    syn::LitInt::new(&offset.to_string(), proc_macro2::Span::call_site());
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());
                let order_fn = format_ident!("from_{order_suffix}_bytes");

                if f.presence == Presence::Constant {
                    if let Some(ref val) = f.constant_value {
                        let bits: u8 = val.parse().unwrap_or(0);
                        let bits_lit =
                            syn::LitInt::new(&bits.to_string(), proc_macro2::Span::call_site());
                        impl_body.extend(
                            syn::parse_str::<proc_macro2::TokenStream>(&format!(
                                "#[inline] pub const fn {fn_name}(&self) -> {t} {{ {t}({bits}) }}",
                                fn_name = fname_ident,
                                t = target_ident,
                                bits = bits_lit,
                            ))
                            .expect("constant set/bool field accessor"),
                        );
                    }
                } else if since > 0 {
                    let since_lit =
                        syn::LitInt::new(&since.to_string(), proc_macro2::Span::call_site());
                    let offset_end = offset + prim_size;
                    let offset_end_lit =
                        syn::LitInt::new(&offset_end.to_string(), proc_macro2::Span::call_site());
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fname_ident(&self) -> Option<#target_ident> {
                            if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                return None;
                            }
                            let offset = self.pos + #offset_lit;
                            Some(#target_ident(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) })))
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fname_ident(&self) -> #target_ident {
                            let offset = self.pos + #offset_lit;
                            #target_ident(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, offset) }))
                        }
                    });
                }
            }
        }
        if enable_meta_attributes {
            impl_body.extend(emit_field_consts(f));
        }
    }

    let total_tail = msg.groups.len() + msg.var_data.len();

    // tail_offset_0
    impl_body.extend(quote::quote! {
        #[inline]
        fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
            Ok(self.pos + self.acting_block_length)
        }
    });

    let group_unique_names: Vec<String> = msg
        .groups
        .iter()
        .map(|g| {
            let raw = to_pascal_case(&g.name);
            if multi_message {
                format!("{}{}", &name, raw)
            } else {
                raw
            }
        })
        .collect();
    let mut k = 0usize;
    for (gi, g) in msg.groups.iter().enumerate() {
        let (dim_name, dim_size, bl_field, count_field) =
            get_dimension_info(elements, &g.dimension_type);
        let g_pascal = &group_unique_names[gi];
        let _dim_name_ident = syn::Ident::new(&dim_name, proc_macro2::Span::call_site());
        let _count_field_ident = syn::Ident::new(&count_field, proc_macro2::Span::call_site());
        let _bl_field_ident = syn::Ident::new(&bl_field, proc_macro2::Span::call_site());
        let _g_entry_ident = syn::Ident::new(
            &format!("{}EntryDecoder", g_pascal),
            proc_macro2::Span::call_site(),
        );
        let k1 = k + 1;
        let tail_k_ident = format_ident!("tail_offset_{k}");
        let tail_k1_ident = format_ident!("tail_offset_{k1}");
        let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), proc_macro2::Span::call_site());
        let dn_ident: syn::Ident = syn::parse_str(&dim_name).unwrap();
        let cf_ident = syn::Ident::new(&count_field, proc_macro2::Span::call_site());
        let bf_ident = syn::Ident::new(&bl_field, proc_macro2::Span::call_site());
        let gn_lit = g.name.as_str();
        let entry_decoder_ident = syn::Ident::new(
            &format!("{}EntryDecoder", g_pascal),
            proc_macro2::Span::call_site(),
        );

        impl_body.extend(quote::quote! {
            #[inline]
            fn #tail_k1_ident(&self) -> Result<usize, sbe_rt::DecodeError> {
                let start = self.#tail_k_ident()?;
                if start + #dim_size_lit > self.buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: #gn_lit,
                        needed: #dim_size_lit,
                        available: self.buf.len().saturating_sub(start),
                    });
                }
                let bytes: [u8; #dim_size_lit] = read_bytes::<#dim_size_lit>(self.buf, start);
                let header = #dn_ident(bytes);
                let count = header.#cf_ident() as usize;
                let block_len = header.#bf_ident() as usize;
                let mut pos = start + #dim_size_lit;
                let mut idx = 0;
                while idx < count {
                    pos = #entry_decoder_ident::skip(self.buf, pos, block_len, self.acting_version)?;
                    idx += 1;
                }
                Ok(pos)
            }
        });
        k += 1;
    }

    // VarData tail offsets
    for vd in &msg.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let prefix_size_lit =
            syn::LitInt::new(&prefix_size.to_string(), proc_macro2::Span::call_site());
        let vd_type_ident = syn::Ident::new(&type_pascal, proc_macro2::Span::call_site());
        let vd_len_field_ident = syn::Ident::new(&len_field, proc_macro2::Span::call_site());
        let vd_name_lit = syn::LitStr::new(&vd.name, proc_macro2::Span::call_site());
        let tail_k_ident = quote::format_ident!("tail_offset_{}", k);
        let tail_k1_ident = quote::format_ident!("tail_offset_{}", k + 1);
        impl_body.extend(quote::quote! {
            #[inline]
            fn #tail_k1_ident(&self) -> Result<usize, sbe_rt::DecodeError> {
                let start = self.#tail_k_ident()?;
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
        k += 1;
    }

    let mut g_idx = 0usize;
    for (gi, g) in msg.groups.iter().enumerate() {
        let scoped = &group_unique_names[gi];
        let g_snake = to_snake_case(&g.name);
        let g_snake_ident = syn::Ident::new(&g_snake, proc_macro2::Span::call_site());
        let g_decoder_ident =
            syn::Ident::new(&format!("{scoped}Decoder"), proc_macro2::Span::call_site());
        let m_idx_lit = syn::LitInt::new(&g_idx.to_string(), proc_macro2::Span::call_site());
        let tail_offset_ident: syn::Ident = syn::Ident::new(
            &format!("tail_offset_{}", g_idx),
            proc_macro2::Span::call_site(),
        );
        let g_snake_str = g_snake.clone();
        let version_check = if g.since_version > 0 {
            let g_since_lit =
                syn::LitInt::new(&g.since_version.to_string(), proc_macro2::Span::call_site());
            quote::quote! {
                if self.acting_version < #g_since_lit {
                    return Err(sbe_rt::DecodeError::FieldNotInVersion {
                        field: #g_snake_str,
                        wire_version: self.acting_version,
                        since_version: #g_since_lit,
                    });
                }
            }
        } else {
            quote::quote! {}
        };
        impl_body.extend(quote::quote! {
            #[inline]
            fn #g_snake_ident(&self) -> Result<#g_decoder_ident<'a>, sbe_rt::DecodeError> {
                #version_check
                let offset = self.#tail_offset_ident()?;
                #g_decoder_ident::wrap(self.buf, offset, self.acting_version)
            }
        });
        g_idx += 1;
    }

    let mut vd_idx = msg.groups.len();
    for vd in &msg.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let vd_snake = to_snake_case(&vd.name);
        let vd_snake_ident = syn::Ident::new(&vd_snake, proc_macro2::Span::call_site());
        let type_pascal_ident = syn::Ident::new(&type_pascal, proc_macro2::Span::call_site());
        let len_field_ident = syn::Ident::new(&len_field, proc_macro2::Span::call_site());
        let prefix_size_lit =
            syn::LitInt::new(&prefix_size.to_string(), proc_macro2::Span::call_site());
        let vd_tail_ident: syn::Ident = syn::Ident::new(
            &format!("tail_offset_{}", vd_idx),
            proc_macro2::Span::call_site(),
        );

        // resolver fills default_max for every group; generator trusts it, add validation if resolver ever skips a group
        // primitive, so max_length is always Some. The else branch can't fire.
        let max = vd.max_length.unwrap_or(0);
        let max_lit = syn::LitInt::new(&max.to_string(), proc_macro2::Span::call_site());
        let vd_snake_str = vd_snake.clone();
        let version_check = if vd.since_version > 0 {
            let vd_since_lit = syn::LitInt::new(
                &vd.since_version.to_string(),
                proc_macro2::Span::call_site(),
            );
            quote::quote! {
                if self.acting_version < #vd_since_lit {
                    return Err(sbe_rt::DecodeError::FieldNotInVersion {
                        field: #vd_snake_str,
                        wire_version: self.acting_version,
                        since_version: #vd_since_lit,
                    });
                }
            }
        } else {
            quote::quote! {}
        };
        impl_body.extend(quote::quote! {
            #[inline]
            pub fn #vd_snake_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                #version_check
                let offset = self.#vd_tail_ident()?;
                let bytes: [u8; #prefix_size_lit] = read_bytes::<#prefix_size_lit>(self.buf, offset);
                let header = #type_pascal_ident(bytes);
                let wire_length = header.#len_field_ident() as u64;
                if wire_length > #max_lit as u64 {
                    return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                        field: stringify!(#vd_snake_ident),
                        length: wire_length,
                        max_length: #max_lit as u64,
                    });
                }
                let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                    stringify!(#vd_snake_ident),
                    offset,
                    #prefix_size_lit,
                    wire_length,
                    self.buf.len(),
                )?;
                Ok(&self.buf[data_start..data_end])
            }
        });

        // Fallible UTF-8/ASCII str accessor (characterEncoding-aware).
        let str_ident = syn::Ident::new(
            &format!("{vd_snake}_as_str"),
            proc_macro2::Span::call_site(),
        );
        let vd_snake_str = vd_snake.clone();
        impl_body.extend(quote::quote! {
            #[inline]
            fn #str_ident(&self) -> Result<&'a str, sbe_rt::DecodeError> {
                let bytes = self.#vd_snake_ident()?;
                core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::InvalidUtf8 {
                    field: #vd_snake_str,
                    error: e,
                })
            }
        });

        // Unchecked str accessor — zero validation, trusts the wire.
        let str_unchecked = syn::Ident::new(
            &format!("{vd_snake}_as_str_unchecked"),
            proc_macro2::Span::call_site(),
        );
        impl_body.extend(quote::quote! {
            /// View this text var-data field as `&str` without UTF-8
            /// validation.
            ///
            /// # Safety
            ///
            /// The wire bytes must be valid UTF-8. For schema-declared
            /// ASCII encoding this is always true (ASCII ⊂ UTF-8).
            #[inline]
            pub unsafe fn #str_unchecked(&self) -> &'a str {
                let bytes = unsafe { self.#vd_snake_ident().unwrap_unchecked() };
                // SAFETY: caller guarantees valid UTF-8
                unsafe { core::str::from_utf8_unchecked(bytes) }
            }
        });

        vd_idx += 1;
    }

    // 9b. rewind() — consume any current stage and return a fresh initial
    // decoder at the original message position. Enforces consumption: the
    // old stage is moved and cannot be reused.
    if total_tail > 0 {
        impl_body.extend(quote::quote! {
            /// Consume this stage and return a fresh decoder at the initial
            /// message position. The consumed stage cannot be reused.
            #[inline]
            pub fn rewind(self) -> Self {
                self
            }
        });
    }

    let total_tail_ident: syn::Ident = syn::Ident::new(
        &format!("tail_offset_{}", total_tail),
        proc_macro2::Span::call_site(),
    );
    impl_body.extend(quote::quote! {
        #[inline]
        pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
            let end = self.#total_tail_ident()?;
            Ok(end - self.pos)
        }

        #[inline]
        pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
            let len = self.encoded_length()?;
            Ok(len + #hdr_size_lit)
        }

        #[inline]
        pub fn as_body_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
            let end = self.#total_tail_ident()?;
            Ok(&self.buf[self.pos..end])
        }

        #[inline]
        pub fn as_bytes_with_header(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
            let end = self.#total_tail_ident()?;
            let start = self.pos.saturating_sub(Self::HEADER_LENGTH);
            Ok(&self.buf[start..end])
        }
    });

    let hs_lit = syn::LitInt::new(&header_size.to_string(), proc_macro2::Span::call_site());
    let hp_ident = syn::Ident::new(&header_pascal, proc_macro2::Span::call_site());
    let hbl_ident = syn::Ident::new(&header_bl, proc_macro2::Span::call_site());

    let mut verify_stmts: Vec<proc_macro2::TokenStream> = Vec::new();

    // Header + block_length preamble
    verify_stmts.push(quote::quote! {
        if buf.len() < #hs_lit {
            return Err(sbe_rt::VerifyError::HeaderTooShort);
        }
        let header_bytes: [u8; #hs_lit] = read_bytes::<#hs_lit>(buf, 0);
        let header = #hp_ident(header_bytes);
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.#hbl_ident() as u64,
        )?;
        if block_length < Self::BLOCK_LENGTH {
            return Err(sbe_rt::VerifyError::InvalidBlockLength {
                expected_min: Self::BLOCK_LENGTH,
                actual: block_length,
            });
        }
        let body_end = (#hs_lit as usize).checked_add(block_length).ok_or(
            sbe_rt::VerifyError::MessageTooShort {
                needed: usize::MAX,
                available: buf.len(),
            },
        )?;
        if body_end > buf.len() {
            return Err(sbe_rt::VerifyError::MessageTooShort {
                needed: body_end,
                available: buf.len(),
            });
        }
        let mut offset = body_end;
    });

    // Group dimension checks
    for g in &msg.groups {
        let (dim_name, dim_size, _, count_field) = get_dimension_info(elements, &g.dimension_type);
        let g_snake = to_snake_case(&g.name);
        let ds_lit = syn::LitInt::new(&dim_size.to_string(), proc_macro2::Span::call_site());
        let dn_ident = syn::Ident::new(&dim_name, proc_macro2::Span::call_site());
        let cf_ident = syn::Ident::new(&count_field, proc_macro2::Span::call_site());
        let ebl_lit = syn::LitInt::new(
            &g.effective_block_length().to_string(),
            proc_macro2::Span::call_site(),
        );
        let has_tails = !g.groups.is_empty() || !g.var_data.is_empty();
        let entry_dec_ident = {
            let raw = to_pascal_case(&g.name);
            let unique = if multi_message {
                format!("{name}{raw}")
            } else {
                raw
            };
            syn::Ident::new(
                &format!("{unique}EntryDecoder"),
                proc_macro2::Span::call_site(),
            )
        };
        if has_tails {
            verify_stmts.push(quote::quote! {
                {
                    if offset + #ds_lit > buf.len() {
                        return Err(sbe_rt::VerifyError::GroupDimOutOfBounds {
                            field: #g_snake,
                            offset,
                        });
                    }
                    let bytes: [u8; #ds_lit] = read_bytes::<#ds_lit>(buf, offset);
                    let dim = #dn_ident(bytes);
                    let count = dim.#cf_ident() as usize;
                    let mut entry_pos = offset + #ds_lit;
                    for _ in 0..count {
                        match #entry_dec_ident::skip(buf, entry_pos, #ebl_lit, 0) {
                            Ok(next) => entry_pos = next,
                            Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
                        }
                    }
                    offset = entry_pos;
                }
            });
        } else {
            verify_stmts.push(quote::quote! {
                {
                    if offset + #ds_lit > buf.len() {
                        return Err(sbe_rt::VerifyError::GroupDimOutOfBounds {
                            field: #g_snake,
                            offset,
                        });
                    }
                    let bytes: [u8; #ds_lit] = read_bytes::<#ds_lit>(buf, offset);
                    let dim = #dn_ident(bytes);
                    let count = dim.#cf_ident() as usize;
                    let entries_end = offset + #ds_lit + count * #ebl_lit;
                    if entries_end > buf.len() {
                        return Err(sbe_rt::VerifyError::MessageTooShort {
                            needed: entries_end,
                            available: buf.len(),
                        });
                    }
                    offset = entries_end;
                }
            });
        }
    }

    // VarData checks
    for vd in &msg.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let vd_snake = to_snake_case(&vd.name);
        let ps_lit = syn::LitInt::new(&prefix_size.to_string(), proc_macro2::Span::call_site());
        let tp_ident = syn::Ident::new(&type_pascal, proc_macro2::Span::call_site());
        let lf_ident = syn::Ident::new(&len_field, proc_macro2::Span::call_site());
        verify_stmts.push(quote::quote! {
            {
                if #ps_lit > buf.len().saturating_sub(offset) {
                    return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                        field: #vd_snake,
                        offset,
                        length: 0,
                    });
                }
                let bytes: [u8; #ps_lit] = read_bytes::<#ps_lit>(buf, offset);
                let var_header = #tp_ident(bytes);
                let len = var_header.#lf_ident() as u64;
                let (_, data_end) = match sbe_rt::checked_var_data_bounds(
                    #vd_snake,
                    offset,
                    #ps_lit,
                    len,
                    buf.len(),
                ) {
                    Ok(bounds) => bounds,
                    Err(_) => return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                        field: #vd_snake,
                        offset,
                        length: len,
                    }),
                };
                offset = data_end;
            }
        });
    }

    impl_body.extend(quote::quote! {
        #[inline]
        pub fn verify(buf: &[u8]) -> Result<(), sbe_rt::VerifyError> {
            #(#verify_stmts)*
            Ok(())
        }
    });

    ts.extend(quote::quote! {
        impl<'a> #decoder_ident<'a> {
            #impl_body
        }
    });

    let msg_id_lit = syn::LitInt::new(&msg.id.to_string(), proc_macro2::Span::call_site());
    let schema_id_lit = syn::LitInt::new(&schema_id.to_string(), proc_macro2::Span::call_site());
    let schema_version_lit =
        syn::LitInt::new(&schema_version.to_string(), proc_macro2::Span::call_site());
    ts.extend(quote::quote! {

        impl<'a> TryFrom<&'a [u8]> for #decoder_ident<'a> {
            type Error = sbe_rt::DecodeError;

            fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
                Self::decode(buf, 0)
            }
        }

        impl<'a> sbe_rt::private::Sealed for #decoder_ident<'a> {}

        impl<'a> sbe_rt::SbeMessage for #decoder_ident<'a> {
            const TEMPLATE_ID: u16 = #msg_id_lit;
            const BLOCK_LENGTH: usize = #bl_lit;
            const SCHEMA_ID: u16 = #schema_id_lit;
            const SCHEMA_VERSION: u16 = #schema_version_lit;
        }

        impl<'a> #decoder_ident<'a> {
            /// Fallible byte view of the complete SBE frame (header + body).
            /// Returns `None` if the buffer is malformed or truncated.
            /// Prefer [`Self::as_bytes_with_header`] for explicit error handling.
            pub fn as_ref_opt(&self) -> Option<&[u8]> {
                self.as_bytes_with_header().ok()
            }
        }
    });

    let display_ts = if enable_display_debug {
        generate_decoder_display(msg, domain_types)
    } else {
        proc_macro2::TokenStream::new()
    };
    ts.extend(display_ts);

    for (gi, g) in msg.groups.iter().enumerate() {
        let unique = &group_unique_names[gi];
        ts.extend(generate_group_decoder(
            g,
            elements,
            byte_order,
            unique,
            &conversions,
            domain_types,
            enable_meta_attributes,
            enable_dispatch,
        ));
    }

    // 14b. Concrete consuming decoder tail stages (DECISIONS.md §3):
    //      NameDecoder --into_<g>()--> GroupDecoder --finish()--> NameDecoderAfter<G>
    //      -> ... -> NameDecoderComplete. Additive: leaves the legacy `&self`
    //      random-access surface in place so existing call sites stay green.
    ts.extend(generate_decoder_consuming_stages(
        msg,
        elements,
        &name,
        header_size,
        byte_order,
        multi_message,
        &group_unique_names,
        enable_dispatch,
    ));

    // 15. Close the main impl block (if is_fixed or not, the block is closed already)
    // Actually the impl block is opened but the `}` is emitted by the trait impls section above.
    // The quote! for trait impls starts with `}` to close the impl block first.
    // Let me verify: the impl block opening uses `quote! { impl ... { ... }`
    // Wait, no. Let me re-check the flow.
    // Section 2 opens: quote! { impl ... { ...  (no closing })
    // Section 12 starts with: `}` (closing the impl)
    // So the impl is properly closed.

    if domain_objects {
        let domain_ts = generate_domain_objects(
            msg,
            elements,
            &name,
            &name,
            multi_message,
            byte_order,
            conversions,
            domain_types,
            domain_var_data,
            hooks,
            schema,
        );
        ts.extend(domain_ts);
    }

    (ts, marker_name)
}

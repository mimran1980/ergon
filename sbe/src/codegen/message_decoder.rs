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

use super::conversion_helpers::{
    DECODER_RESERVED, enum_uses_null_as_option, field_has_conversion_free, resolve_field_ident,
};
use super::decoder_display::generate_decoder_display;
use super::domain_cluster::generate_domain_objects;
use super::group_decoder::generate_group_decoder;
use super::ordered_decoder::generate_ordered_decoder;
use super::runtime::{
    constant_value_expr, deprecated_attr_tokens, doc_attr_tokens, emit_field_consts,
    schema_marker_ident, to_pascal_case, to_snake_case,
};
use super::tail_stages::generate_decoder_consuming_stages;

/// Pure flyweight observers (getters, predicates, lengths) — discarding the
/// return value does no decode work and is almost always a caller mistake.
fn must_use_observer() -> proc_macro2::TokenStream {
    quote::quote! {
        #[must_use = "discarding this value is almost always a mistake"]
    }
}

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
    hooks: &crate::config::Hooks,
    schema: &crate::Schema,
    null_as_option: &[crate::ConversionSelector],
    all_enums_as_option: bool,
    gen_ctx: &super::runtime::GenerationContext,
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

    let sealed_path = &gen_ctx.sealed_path;

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
        #[doc = concat!("Schema constants for the `", stringify!(#schema_ident), "` message: `SCHEMA_ID`, `SCHEMA_VERSION`, `TEMPLATE_ID`, `BLOCK_LENGTH`, `HEADER_LENGTH`.")]
        pub struct #schema_ident;
        impl #schema_ident {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #bl_lit;
            pub const HEADER_LENGTH: usize = #hdr_size_lit;

            /// Full structural verification of a buffer: validates header,
            /// block-length extent, group dimension headers, entry strides,
            /// and var-data bounds. Use **before** construction when the
            /// entire frame must be proven valid without building a decoder.
            #[inline]
            pub fn verify(buf: &[u8]) -> Result<(), sbe_rt::VerifyError> {
                #decoder_ident::verify(buf)
            }
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
        #[must_use = "decoder must be read or advanced; dropping is fine only after use"]
        pub struct #decoder_ident<'a> {
            pub(crate) buf: &'a [u8],
            /// Byte offset of the message body within `self.buf`.
            pub(crate) offset: usize,
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
            #[must_use = "the slice after this message is unused; ignoring it skips payload framing"]
            #[inline]
            pub fn after_this_message(frame: &[u8]) -> Option<&[u8]> {
                if frame.len() < Self::ENCODED_LENGTH {
                    return None;
                }
                Some(&frame[Self::ENCODED_LENGTH..])
            }
            // Placement utils (`message_offset` / `limit` / `buffer` /
            // `remaining`) live only on `{Name}DecoderMetadata` via
            // `get_metadata()` so schema fields may use those names.
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

    // Unified extent rule shared with group entries (runtime.rs).
    let min_extent_arms = crate::codegen::runtime::emit_readable_extent_body(&msg.fields);

    impl_body.extend(quote::quote! {
        /// Minimum body bytes needed to safely read every fixed field present
        /// at `acting_version` (version-aware; not always full `BLOCK_LENGTH`).
        #[must_use = "this extent is the minimum readable body size; ignoring it skips a bounds check"]
        #[inline]
        pub const fn min_readable_fixed_extent(acting_version: u16) -> usize {
            #min_extent_arms
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
        pub fn try_wrap(
            buf: &'a [u8],
            message_offset: usize,
            acting_block_length: usize,
            acting_version: u16,
        ) -> Result<Self, sbe_rt::DecodeError> {
            let Some(body_offset) = message_offset.checked_add(Self::HEADER_LENGTH) else {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "message header",
                    needed: Self::HEADER_LENGTH,
                    available: buf.len().saturating_sub(message_offset),
                });
            };
            let available_body = buf.len().saturating_sub(body_offset);
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
            Ok(unsafe {
                Self::wrap_unchecked(buf, message_offset, acting_block_length, acting_version)
            })
        }

        /// Trusted external-metadata wrap. Proves version-aware fixed extent
        /// then constructs; **panics** if the buffer is too short. Field
        /// accessors use unchecked reads justified by that proof.
        ///
        /// Prefer [`Self::try_wrap`] at untrusted boundaries. Uses a direct
        /// extent check (not `try_wrap` + match) so the hot success path does
        /// not construct a `Result` — same contract as encoder bare `wrap`.
        #[inline]
        pub fn wrap(
            buf: &'a [u8],
            message_offset: usize,
            acting_block_length: usize,
            acting_version: u16,
        ) -> Self {
            // Cold panics use static strings so the success path stays free of
            // DecodeError/Display monomorphisation (batch decode no-LTO).
            let Some(body_offset) = message_offset.checked_add(Self::HEADER_LENGTH) else {
                panic!("buffer too short for message header");
            };
            let available_body = buf.len().saturating_sub(body_offset);
            let min_fixed = Self::min_readable_fixed_extent(acting_version);
            let body_need = if acting_block_length > min_fixed {
                acting_block_length
            } else {
                min_fixed
            };
            if body_need > available_body {
                panic!("buffer too short for message body");
            }
            // SAFETY: extent check above proved header + version-aware fixed body fit.
            unsafe {
                Self::wrap_unchecked(buf, message_offset, acting_block_length, acting_version)
            }
        }

        /// Zero-check wrap — raw pointer accessors, **UB** on OOB.
        /// Only for proven-tight hot loops after an external extent proof.
        ///
        /// # Safety
        /// `message_offset + HEADER_LENGTH + max(acting_block_length,
        /// min_readable_fixed_extent(acting_version))` must not overflow
        /// and must be ≤ `buf.len()`.
        #[inline]
        pub unsafe fn wrap_unchecked(
            buf: &'a [u8],
            message_offset: usize,
            acting_block_length: usize,
            acting_version: u16,
        ) -> Self {
            let body_offset = message_offset + Self::HEADER_LENGTH;
            Self {
                buf,
                offset: body_offset,
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
        let mn = syn::LitStr::new(&name, proc_macro2::Span::call_site());
        let template_id_validation = if header_ti_constant {
            quote::quote! {}
        } else {
            quote::quote! {
                if template_id != Self::TEMPLATE_ID {
                    return Err(sbe_rt::DecodeError::WrongTemplate {
                        expected: Self::TEMPLATE_ID,
                        actual: template_id,
                        expected_name: #mn,
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
            /// Decode a framed message at **message start** (`offset` = first
            /// byte of the header). Validates header fields and the
            /// version-aware fixed body extent. See [`Self::wrap`] for the
            /// message-start coordinate system.
            #[inline]
            pub fn try_decode(buf: &'a [u8], offset: usize) -> Result<Self, sbe_rt::DecodeError> {
                if #hs > buf.len().saturating_sub(offset) {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "message header",
                        needed: #hs,
                        available: buf.len().saturating_sub(offset),
                    });
                }
                let header_bytes: [u8; #hs] = read_bytes::<#hs>(buf, offset);
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
                // Shared path with try_wrap: version-aware min fixed extent.
                Self::try_wrap(buf, offset, acting_block_length, acting_version)
            }

            /// Trusted framed decode — **hybrid return** (freeze-friendly):
            ///
            /// - **Extent (short buffer):** panics after the same proof as
            ///   [`Self::wrap`] (trusted tier).
            /// - **Identity (wrong template/schema):** still returns `Err`
            ///   so session demux can recover without catch_unwind.
            ///
            /// Signature therefore looks like [`Self::try_decode`], but short
            /// buffers do **not** yield `BufferTooShort` — they panic. Prefer
            /// [`Self::try_decode`] at untrusted boundaries when every failure
            /// must be a `Result`.
            #[inline]
            pub fn decode(buf: &'a [u8], offset: usize) -> Result<Self, sbe_rt::DecodeError> {
                // Header read panics if the header region is short.
                let header_bytes: [u8; #hs] = read_bytes::<#hs>(buf, offset);
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
                // Body extent: panic (trusted tier), not Err.
                Ok(Self::wrap(buf, offset, acting_block_length, acting_version))
            }

            /// Unchecked **extent**, checked **identity**.
            ///
            /// Header/body bytes are read without bounds checks (**UB** if the
            /// caller has not proven the frame fits). Template/schema identity
            /// still returns `Err` (same hybrid policy as [`Self::decode`]).
            ///
            /// # Safety
            /// Header and version-readable fixed body for this template must
            /// be fully in-bounds at `offset`.
            #[inline]
            pub unsafe fn decode_unchecked(buf: &'a [u8], offset: usize) -> Result<Self, sbe_rt::DecodeError> {
                // SAFETY: caller guarantees header bytes are in-bounds.
                let header_bytes: [u8; #hs] = unsafe { read_bytes_unchecked::<#hs>(buf, offset) };
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
                Ok(unsafe {
                    Self::wrap_unchecked(buf, offset, acting_block_length, acting_version)
                })
            }
        });
    }

    let mu = must_use_observer();
    impl_body.extend(quote::quote! {
        /// Schema version from the message header (or wrap args), not the
        /// compiled schema constant. Fields with `sinceVersion` and optional
        /// presence depend on this value.
        #mu
        #[inline]
        pub const fn acting_version(&self) -> u16 {
            self.acting_version
        }

        /// Block length from the wire header / wrap args. Tail offsets use
        /// this acting length, not only the compiled `BLOCK_LENGTH`.
        #mu
        #[inline]
        pub const fn acting_block_length(&self) -> usize {
            self.acting_block_length
        }
    });
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
        // Placement utils live only on DecoderMetadata via get_metadata() —
        // see DECODER_RESERVED in conversion_helpers (inherent methods only).
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
                                #mu
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
                                #mu
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

                    // One bulk read, then reconstruct. 1-byte primitives return
                    // `all` (endianness is a no-op); wider types unroll
                    // from_{le,be}_bytes on constant indices.
                    let elements = super::conversion_helpers::fixed_array_from_bulk_bytes(
                        &r_type_ty, *prim, prim_size, len_val, &order_fn,
                    );

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
                        #mu
                        #[inline]
                        pub fn #fn_snake_ident(&self) -> [#r_type_ty; #len_lit] {
                            #version_guard
                            let all: [u8; #total_size_lit] = unsafe { read_bytes_unchecked::<#total_size_lit>(self.buf, self.offset + #offset_lit) };
                            #elements
                        }
                    });
                    // Destination-buffer copy for byte-width arrays (Java getVehicleCode(byte[])).
                    if prim_size == 1 {
                        let copy_ident = syn::Ident::new(
                            &format!("copy_{}", fname_snake),
                            proc_macro2::Span::call_site(),
                        );
                        impl_body.extend(quote::quote! {
                            #mu
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
                        let null_check_expr =
                            if *prim == PrimitiveType::Float || *prim == PrimitiveType::Double {
                                // Any IEEE NaN is null (matches sbe-tool is_nan()).
                                let _ = null_val;
                                "val.is_nan()".to_string()
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
                            "#[must_use = \"discarding this value is almost always a mistake\"]\n\
                             #[inline]\n\
                             pub fn {snake}(&self) -> Option<{rt}> {{\n\
                                 {version_guard}\
                                 let val = {rt}::{order}(unsafe {{ read_bytes_unchecked::<{ps}>(self.buf, self.offset + {offset}) }});\n\
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
                            #mu
                            #[inline]
                            pub fn #fname_ident(&self) -> Option<#r_type_ty> {
                                if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                    return None;
                                }
                                Some(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) }))
                            }
                        });
                    } else {
                        if let Some(ref desc) = f.description {
                            impl_body.extend(doc_attr_tokens(desc));
                        }
                        impl_body.extend(deprecated_attr_tokens(f.deprecated));
                        // Required scalar primitives are the decode_scalar hot path.
                        // `always` is required for no-LTO parity with sbe-tool
                        // (2026-08-13: plain `#[inline]` regressed decode_scalar
                        // no-LTO to 1.47×; with always, LTO+no-LTO stay ≤1.00).
                        impl_body.extend(quote::quote! {
                            #mu
                            #[inline(always)]
                            pub fn #fname_ident(&self) -> #r_type_ty {
                                #r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) })
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
                        #mu
                        #[inline]
                        pub fn #fname_ident(&self) -> Option<#target_decoder_name<'_>> {
                            if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                return None;
                            }
                            Some(#target_decoder_name { buf: self.buf, offset: self.offset + #offset_lit })
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #fname_ident(&self) -> #target_decoder_name<'_> {
                            #target_decoder_name { buf: self.buf, offset: self.offset + #offset_lit }
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
                        #mu
                        #[inline]
                        pub fn #as_struct_ident(&self) -> Option<#target_ident> {
                            if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                return None;
                            }
                            Some(#target_ident(unsafe { read_bytes_unchecked::<#comp_size_lit>(self.buf, self.offset + #offset_lit) }))
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #as_struct_ident(&self) -> #target_ident {
                            #target_ident(unsafe { read_bytes_unchecked::<#comp_size_lit>(self.buf, self.offset + #offset_lit) })
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
                            #mu
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
                    let raw_ident = quote::format_ident!("raw_{}", fname_snake);
                    if enum_uses_null_as_option(enum_name, null_as_option, all_enums_as_option) {
                        impl_body.extend(quote::quote! {
                            /// Returns [`None`] when the field is absent at this version
                            /// OR the wire discriminant equals [`#target_ident::NullVal`].
                            #mu
                            #[inline]
                            pub fn #fname_ident(&self) -> Option<#target_ident> {
                                if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                    return None;
                                }
                                #target_ident::from_raw(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) })).as_option()
                            }
                            /// Raw wire discriminant — bypasses enum mapping.
                            /// Returns `None` when the field is not present in the acting version.
                            #mu
                            #[inline]
                            pub fn #raw_ident(&self) -> Option<#r_type_ty> {
                                if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                    return None;
                                }
                                Some(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) }))
                            }
                        });
                    } else {
                        impl_body.extend(quote::quote! {
                            #mu
                            #[inline]
                            pub fn #fname_ident(&self) -> Option<#target_ident> {
                                if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                    return None;
                                }
                                Some(#target_ident::from_raw(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) })))
                            }
                            /// Raw wire discriminant — bypasses enum mapping.
                            /// Returns `None` when the field is not present in the acting version.
                            #mu
                            #[inline]
                            pub fn #raw_ident(&self) -> Option<#r_type_ty> {
                                if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                    return None;
                                }
                                Some(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) }))
                            }
                        });
                    }
                    if crate::structured_ir::is_bool_value_enum(elements, enum_name) {
                        // One name for the decoder's boolean getter at every
                        // shape and location: `try_*` means fallible decode.
                        // `{field}_bool` is the *encoder setter*, so using it
                        // for a getter too collided on meaning, and naming the
                        // two shapes differently is what let the domain DTO
                        // call an accessor that did not exist.
                        let fname_bool = quote::format_ident!("try_{}_bool", fname_snake);
                        impl_body.extend(quote::quote! {
                            /// Returns `Some(true)` / `Some(false)` for valid
                            /// boolean values, or `None` when the field is absent
                            /// from the acting version or the wire carries `NullVal`.
                            #[inline]
                            pub fn #fname_bool(&self) -> Result<Option<bool>, sbe_rt::DecodeError> {
                                match self.#fname_ident() {
                                    None => Ok(None),
                                    Some(v) => v.as_bool().map(Some).ok_or(
                                        sbe_rt::DecodeError::InvalidBoolean {
                                            field: stringify!(#fname_ident),
                                            discriminant: v as u64,
                                        }
                                    ),
                                }
                            }
                        });
                    }
                } else {
                    let raw_ident = quote::format_ident!("raw_{}", fname_snake);
                    let raw_body = quote::quote! {
                        /// Raw wire discriminant — bypasses enum mapping.
                        /// Use to inspect unknown/forward enum values without losing the original byte.
                        #mu
                        #[inline]
                        pub fn #raw_ident(&self) -> #r_type_ty {
                            #r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) })
                        }
                    };
                    if enum_uses_null_as_option(enum_name, null_as_option, all_enums_as_option) {
                        impl_body.extend(quote::quote! {
                            /// Returns [`None`] when the wire discriminant equals
                            /// [`#target_ident::NullVal`]; [`Some`] otherwise.
                            #mu
                            #[inline]
                            pub fn #fname_ident(&self) -> Option<#target_ident> {
                                let raw = #r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) });
                                #target_ident::from_raw(raw).as_option()
                            }
                        });
                        impl_body.extend(raw_body);
                    } else {
                        impl_body.extend(quote::quote! {
                            #mu
                            #[inline]
                            pub fn #fname_ident(&self) -> #target_ident {
                                #target_ident::from_raw(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) }))
                            }
                        });
                        impl_body.extend(raw_body);
                    }
                    if crate::structured_ir::is_bool_value_enum(elements, enum_name) {
                        let fname_bool = quote::format_ident!("try_{}_bool", fname_snake);
                        // `null_as_option` makes the raw enum accessor return
                        // `Option<T>`, so `.as_bool()` cannot be called on it
                        // directly. The contract is unchanged — `NullVal` is
                        // still rejected — only the unwrapping differs.
                        let read = if enum_uses_null_as_option(
                            enum_name,
                            null_as_option,
                            all_enums_as_option,
                        ) {
                            quote::quote! { self.#fname_ident().and_then(|v| v.as_bool()) }
                        } else {
                            quote::quote! { self.#fname_ident().as_bool() }
                        };
                        impl_body.extend(quote::quote! {
                            /// Returns `true` / `false` for valid boolean values.
                            /// Rejects `NullVal` or unknown raw discriminants —
                            /// the SBE boolean wire type is tri-state (F, T, null).
                            #[inline]
                            pub fn #fname_bool(&self) -> Result<bool, sbe_rt::DecodeError> {
                                #read.ok_or(
                                    sbe_rt::DecodeError::InvalidBoolean {
                                        field: stringify!(#fname_ident),
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
                                "#[must_use = \"discarding this value is almost always a mistake\"]\n\
                                 #[inline] pub const fn {fn_name}(&self) -> {t} {{ {t}({bits}) }}",
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
                        #mu
                        #[inline]
                        pub fn #fname_ident(&self) -> Option<#target_ident> {
                            if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                return None;
                            }
                            Some(#target_ident(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) })))
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #mu
                        #[inline]
                        pub fn #fname_ident(&self) -> #target_ident {
                            #target_ident(#r_type_ty::#order_fn(unsafe { read_bytes_unchecked::<#prim_size_lit>(self.buf, self.offset + #offset_lit) }))
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
        /// Byte offset of the message body within `self.buf`.
        #[inline]
        fn byte_offset(&self) -> usize {
            self.offset
        }

        #[inline]
        fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
            Ok(self.byte_offset() + self.acting_block_length)
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
        impl_body.extend(quote::quote! {
            #[inline]
            fn #tail_k1_ident(&self) -> Result<usize, sbe_rt::DecodeError> {
                let start = self.#tail_k_ident()?;
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
                    offset = #entry_decoder_ident::skip(self.buf, offset, block_len, self.acting_version)?;
                    idx += 1;
                }
                Ok(offset)
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
        impl_body.extend(quote::quote! {
            #[inline]
            fn #tail_k1_ident(&self) -> Result<usize, sbe_rt::DecodeError> {
                let start = self.#tail_k_ident()?;
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
            #mu
            #[inline]
            pub fn #g_snake_ident(&self) -> Result<#g_decoder_ident<'a>, sbe_rt::DecodeError> {
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
        if let Some(ref desc) = vd.description {
            impl_body.extend(doc_attr_tokens(desc));
        }
        impl_body.extend(quote::quote! {
            #[inline]
            pub fn #vd_snake_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                #version_check
                let offset = self.#vd_tail_ident()?;
                if offset + #prefix_size_lit > self.buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: stringify!(#vd_snake_ident),
                        needed: #prefix_size_lit,
                        available: self.buf.len().saturating_sub(offset),
                    });
                }
                // SAFETY: bounds verified by the preceding check
                let bytes: [u8; #prefix_size_lit] = unsafe {
                    core::ptr::read_unaligned(
                        self.buf.as_ptr().add(offset) as *const [u8; #prefix_size_lit]
                    )
                };
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

        #[allow(clippy::collapsible_else_if)]
        if vd.character_encoding.as_deref() == Some("UTF-8") {
            let str_ident = syn::Ident::new(
                &format!("{vd_snake}_as_str"),
                proc_macro2::Span::call_site(),
            );
            let vd_snake_str = vd_snake.clone();
            impl_body.extend(quote::quote! {
                /// View this UTF-8 var-data field as `&str`.
                #[inline]
                pub fn #str_ident(&self) -> Result<&'a str, sbe_rt::DecodeError> {
                    let bytes = self.#vd_snake_ident()?;
                    core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::InvalidUtf8 {
                        field: #vd_snake_str,
                        error: e,
                    })
                }
            });
            let str_unchecked = syn::Ident::new(
                &format!("{vd_snake}_as_str_unchecked"),
                proc_macro2::Span::call_site(),
            );
            impl_body.extend(quote::quote! {
                /// View this text var-data field as `&str` without character
                /// encoding validation. Structural bounds are still checked.
                ///
                /// # Safety
                ///
                /// The wire bytes must be valid UTF-8.
                #[inline]
                pub unsafe fn #str_unchecked(&self) -> Result<&'a str, sbe_rt::DecodeError> {
                    let bytes = self.#vd_snake_ident()?;
                    Ok(unsafe { core::str::from_utf8_unchecked(bytes) })
                }
            });
        } else if vd.character_encoding.as_deref() == Some("ASCII") {
            let str_ident = syn::Ident::new(
                &format!("{vd_snake}_as_str"),
                proc_macro2::Span::call_site(),
            );
            let vd_snake_str = vd_snake.clone();
            impl_body.extend(quote::quote! {
                /// View this ASCII var-data field as `&str`.
                #[inline]
                pub fn #str_ident(&self) -> Result<&'a str, sbe_rt::DecodeError> {
                    let bytes = self.#vd_snake_ident()?;
                    if bytes.iter().any(|b| *b > 0x7F) {
                        return Err(sbe_rt::DecodeError::InvalidAscii {
                            field: #vd_snake_str,
                        });
                    }
                    // Valid 7-bit ASCII is always valid UTF-8.
                    Ok(unsafe { core::str::from_utf8_unchecked(bytes) })
                }
            });
            let str_unchecked = syn::Ident::new(
                &format!("{vd_snake}_as_str_unchecked"),
                proc_macro2::Span::call_site(),
            );
            impl_body.extend(quote::quote! {
                /// View this text var-data field as `&str` without ASCII
                /// validation. Structural bounds remain fallible.
                ///
                /// # Safety
                ///
                /// The wire bytes must be 7-bit ASCII. For ASCII-declared
                /// fields from a trusted source this is always true.
                #[inline]
                pub unsafe fn #str_unchecked(&self) -> Result<&'a str, sbe_rt::DecodeError> {
                    let bytes = self.#vd_snake_ident()?;
                    Ok(unsafe { core::str::from_utf8_unchecked(bytes) })
                }
            });
        }
        // Binary / unspecified encoding: no string helper at all. The caller
        // has the raw `_slice` / `into_<field>` accessors and can interpret
        // the bytes as needed.

        vd_idx += 1;
    }

    // 9b. rewind() — consume any current stage and return a fresh initial
    // decoder at the original message position. Enforces consumption: the
    // old stage is moved and cannot be reused.
    if msg.has_tails() {
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
        #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
        pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
            let end = self.#total_tail_ident()?;
            Ok(end - self.byte_offset())
        }

        #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
        pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
            let len = self.encoded_length()?;
            Ok(len + #hdr_size_lit)
        }

        #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
        pub fn as_body_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
            let end = self.#total_tail_ident()?;
            let start = self.byte_offset();
            Ok(&self.buf[start..end])
        }

        #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
        pub fn as_bytes_with_header(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
            let end = self.#total_tail_ident()?;
            let start = self.byte_offset().saturating_sub(Self::HEADER_LENGTH);
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
                    let count = match sbe_rt::checked_group_count(
                        "numInGroup",
                        dim.#cf_ident() as u64,
                    ) {
                        Ok(count) => count,
                        Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
                    };
                    let mut entry_offset = match offset.checked_add(#ds_lit) {
                        Some(v) => v,
                        None => {
                            return Err(sbe_rt::VerifyError::GroupDimOutOfBounds {
                                field: #g_snake,
                                offset,
                            });
                        }
                    };
                    for _ in 0..count {
                        match #entry_dec_ident::skip(buf, entry_offset, #ebl_lit, 0) {
                            Ok(next) => entry_offset = next,
                            Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
                        }
                    }
                    offset = entry_offset;
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
                    let count = match sbe_rt::checked_group_count(
                        "numInGroup",
                        dim.#cf_ident() as u64,
                    ) {
                        Ok(count) => count,
                        Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
                    };
                    let dim_end = match offset.checked_add(#ds_lit) {
                        Some(v) => v,
                        None => {
                            return Err(sbe_rt::VerifyError::GroupDimOutOfBounds {
                                field: #g_snake,
                                offset,
                            });
                        }
                    };
                    let entries = match count.checked_mul(#ebl_lit) {
                        Some(v) => v,
                        None => {
                            return Err(sbe_rt::VerifyError::MessageTooShort {
                                needed: usize::MAX,
                                available: buf.len(),
                            });
                        }
                    };
                    let entries_end = match dim_end.checked_add(entries) {
                        Some(v) => v,
                        None => {
                            return Err(sbe_rt::VerifyError::MessageTooShort {
                                needed: usize::MAX,
                                available: buf.len(),
                            });
                        }
                    };
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

    // ── Metadata facet ──────────────────────────────────────────────────
    let metadata_ident = syn::Ident::new(
        &format!("{}DecoderMetadata", name),
        proc_macro2::Span::call_site(),
    );
    // Metadata only spans the acting fixed block (header + body at wrap).
    // Complete-sounding names only when there are no tails; otherwise mirror
    // encoder `as_fixed_region_with_header` so mid-decode metadata cannot be
    // mistaken for a publishable full frame.
    let meta_bytes = if msg.is_fixed() {
        quote::quote! {
                /// Message body bytes (header exclusive). Fixed-only message —
                /// this is the complete body.
                #[must_use = "discarding this value is almost always a mistake"]
        #[inline]
                pub fn as_body_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    let start = self.decoder.byte_offset();
                    let end = self.decoder.byte_offset() + self.decoder.acting_block_length;
                    if start > self.decoder.buf.len() || end > self.decoder.buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "body",
                            needed: end.saturating_sub(start),
                            available: self.decoder.buf.len().saturating_sub(start),
                        });
                    }
                    Ok(&self.decoder.buf[start..end])
                }
                /// Header-inclusive frame bytes (fixed-only message — complete).
                #[must_use = "discarding this value is almost always a mistake"]
        #[inline]
                pub fn as_bytes_with_header(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    let start = self.message_offset();
                    let end = self.decoder.byte_offset() + self.decoder.acting_block_length;
                    if start > self.decoder.buf.len() || end > self.decoder.buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "frame",
                            needed: end.saturating_sub(start),
                            available: self.decoder.buf.len().saturating_sub(start),
                        });
                    }
                    Ok(&self.decoder.buf[start..end])
                }
            }
    } else {
        quote::quote! {
                /// Fixed-block body only (groups/var-data not included).
                /// For a complete frame walk tails then use the complete stage's
                /// `as_bytes_with_header`, or the decoder's inherent
                /// `as_bytes_with_header` which rescans tails without consuming
                /// the stage.
                #[must_use = "discarding this value is almost always a mistake"]
        #[inline]
                pub fn as_fixed_body_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    let start = self.decoder.byte_offset();
                    let end = self.decoder.byte_offset() + self.decoder.acting_block_length;
                    if start > self.decoder.buf.len() || end > self.decoder.buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "body",
                            needed: end.saturating_sub(start),
                            available: self.decoder.buf.len().saturating_sub(start),
                        });
                    }
                    Ok(&self.decoder.buf[start..end])
                }
                /// Header + fixed block only — **not** a complete SBE message when
                /// groups or var-data remain. Prefer the complete stage's
                /// `as_bytes_with_header` after finishing the walk.
                #[must_use = "discarding this value is almost always a mistake"]
        #[inline]
                pub fn as_fixed_region_with_header(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    let start = self.message_offset();
                    let end = self.decoder.byte_offset() + self.decoder.acting_block_length;
                    if start > self.decoder.buf.len() || end > self.decoder.buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "frame",
                            needed: end.saturating_sub(start),
                            available: self.decoder.buf.len().saturating_sub(start),
                        });
                    }
                    Ok(&self.decoder.buf[start..end])
                }
            }
    };
    ts.extend(quote::quote! {
        /// Buffer-placement and wire-frame metadata. Holds a reference to the
        /// parent decoder — zero-copy. Utility methods live here so no schema
        /// field can collide with them. Byte views on this facet span the
        /// **acting fixed block only**; complete frames use the complete stage
        /// or the decoder's tail-rescan helpers when the message has groups
        /// or var-data.
        #[derive(Clone, Copy)]
        pub struct #metadata_ident<'m, 'a> {
            decoder: &'m #decoder_ident<'a>,
        }

        impl<'m, 'a> #metadata_ident<'m, 'a> {
            /// Absolute offset of this message's frame start (first header byte)
            /// within the underlying buffer.
            #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
            pub fn message_offset(&self) -> usize {
                self.decoder.byte_offset().saturating_sub(#decoder_ident::HEADER_LENGTH)
            }
            /// End of the **acting fixed block** (body start + acting block length).
            /// Not the full message end when groups/var-data follow — use a complete
            /// stage or inherent `encoded_length_with_header` after walking tails.
            #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
            pub fn limit(&self) -> usize {
                self.decoder.byte_offset() + self.decoder.acting_block_length
            }
            /// The full underlying buffer slice this decoder was wrapped on.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn buffer(&self) -> &'a [u8] {
                self.decoder.buf
            }
            /// Bytes after the acting fixed block end. May still contain unread
            /// groups/var-data of **this** message until the consuming walk finishes.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn remaining(&self) -> &'a [u8] {
                let end = (self.decoder.byte_offset() + self.decoder.acting_block_length).min(self.decoder.buf.len());
                &self.decoder.buf[end..]
            }
            #meta_bytes
            /// Schema version from the message header (or wrap args), not the
            /// compiled schema constant. Fields with `sinceVersion` and optional
            /// presence depend on this value.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline] pub fn acting_version(&self) -> u16 { self.decoder.acting_version }
            /// Block length from the wire header / wrap args. Tail offsets use
            /// this acting length, not only the compiled `BLOCK_LENGTH`.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline] pub fn acting_block_length(&self) -> usize { self.decoder.acting_block_length }
        }
    });

    ts.extend(quote::quote! {
        impl<'a> #decoder_ident<'a> {
            /// Metadata accessor: buffer positions, wire-frame boundaries,
            /// version/block-length state. Returns a zero-copy reference to
            /// the parent decoder — no fields are copied. All utility methods
            /// are scoped here so no schema field name can collide with them.
            #[must_use = "discarding this value is almost always a mistake"]
            #[inline]
            pub fn get_metadata(&self) -> #metadata_ident<'_, 'a> {
                #metadata_ident { decoder: self }
            }

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

            #[inline]
            fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
                Self::try_decode(buf, 0)
            }
        }

        impl<'a> #sealed_path::Sealed for #decoder_ident<'a> {}

        impl<'a> sbe_rt::SbeMessage for #decoder_ident<'a> {
            const TEMPLATE_ID: u16 = #msg_id_lit;
            const BLOCK_LENGTH: usize = #bl_lit;
            const SCHEMA_ID: u16 = #schema_id_lit;
            const SCHEMA_VERSION: u16 = #schema_version_lit;
        }
    });

    let display_ts = if enable_display_debug {
        generate_decoder_display(msg, domain_types, conversions)
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
            null_as_option,
            all_enums_as_option,
            enable_display_debug,
        ));
    }

    // Consuming decoder tail stages:
    //   NameDecoder --into_<g>()--> GroupDecoder --finish()--> NameDecoderAfter<G>
    //   -> ... -> NameDecoderComplete. Random-access `&self` accessors remain.
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
    ts.extend(generate_ordered_decoder(
        msg,
        elements,
        &name,
        header_size,
        byte_order,
        multi_message,
        &group_unique_names,
        conversions,
        domain_types,
        enable_dispatch,
        null_as_option,
        all_enums_as_option,
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
            null_as_option,
            all_enums_as_option,
        );
        ts.extend(domain_ts);
    }

    (ts, marker_name)
}

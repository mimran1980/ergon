//! Message encoder codegen.
//!
//! `generate_message_encoder` emits the encoder flyweight for a message: the
//! schema marker struct, fixed-field setters (primitives, arrays, composites,
//! enums, sets), `wrap`/`wrap_and_apply_header`/`*_unchecked` entry points,
//! encoded-length support, group/var-data tail setters, consuming tail
//! stages, unchecked companions, and optional `apply_nulls`. Depends on
//! [`super::group_encoder`], [`super::message_header_template`],
//! [`super::nullification`], [`super::conversion_helpers`],
//! [`super::field_type`], [`super::encoded_length`], and `structured_ir`.

use crate::ir::{ByteOrder, Presence};
use crate::structured_ir::*;

use super::conversion_helpers::{field_has_conversion_free, resolve_field_ident};
use super::encoded_length;
use super::field_type::field_type_ident;
use super::group_encoder::generate_group_encoder;
use super::message_header_template::message_header_template;
use super::nullification::generate_nullification;
use super::runtime::{doc_attr_tokens, emit_field_consts, to_pascal_case, to_snake_case};

pub(crate) fn generate_message_encoder(
    msg: &MessageStructure,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    schema_id: u16,
    schema_version: u16,
    header_type: &str,
    multi_message: bool,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    enable_meta_attributes: bool,
    enable_display_debug: bool,
) -> proc_macro2::TokenStream {
    let raw_name = &msg.name;
    let name = to_pascal_case(raw_name);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    // Prefer the resolved message block length (includes schema-declared
    // padding via `blockLength="…"`). Fall back to a tight field-span only if
    // resolve left it zero (should not happen for real messages).
    // Constant fields have zero wire footprint.
    let computed_block_length = msg
        .fields
        .iter()
        .filter(|f| f.presence != Presence::Constant)
        .fold(0, |acc, f| {
            let size = f.field_type.size();
            acc.max(f.offset + size)
        });
    let block_length = msg.block_length.max(computed_block_length);

    #[expect(unused_variables)]
    let header_pascal = to_pascal_case(header_type);
    let header_size = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| c[0].encoding.offset)
        .unwrap_or(8);

    let total_tail = msg.groups.len() + msg.var_data.len();
    let is_fixed = total_tail == 0;
    // Classify and generate encoded-length support.
    let encoded_len_gen = encoded_length::generate(msg, block_length, header_size, elements);
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

    const STACK_LIMIT: usize = 65536;
    let max_encoded_capped = max_encoded_length.min(STACK_LIMIT);
    let is_capped = max_encoded_length > STACK_LIMIT;

    let span = proc_macro2::Span::call_site();
    let snake_name = to_snake_case(&msg.name);
    let name_encoder_ident = syn::Ident::new(&format!("{}Encoder", name), span);
    let name_decoder_ident = syn::Ident::new(&format!("{}Decoder", name), span);

    // Pre-compute the exact schema-declared header wire image. Composite
    // offsets may introduce padding and blockLength may use another unsigned
    // primitive width; every multi-octet member follows schema byteOrder.
    let header_tpl = message_header_template(
        elements,
        header_type,
        header_size,
        byte_order,
        block_length,
        msg.id,
        schema_id,
        schema_version,
    );
    let hdr_lits: Vec<syn::LitInt> = header_tpl
        .iter()
        .map(|b| syn::LitInt::new(&b.to_string(), span))
        .collect();

    let header_size_lit = syn::LitInt::new(&header_size.to_string(), span);
    let block_length_lit = syn::LitInt::new(&block_length.to_string(), span);
    let schema_id_lit = syn::LitInt::new(&schema_id.to_string(), span);
    let schema_version_lit = syn::LitInt::new(&schema_version.to_string(), span);
    let msg_id_lit = syn::LitInt::new(&msg.id.to_string(), span);
    let encoded_length_lit = syn::LitInt::new(&encoded_length.to_string(), span);
    let max_encoded_capped_lit = syn::LitInt::new(&max_encoded_capped.to_string(), span);
    let to_endian = syn::Ident::new(&format!("to_{}_bytes", order_suffix), span);

    let mut ts = proc_macro2::TokenStream::new();

    let tail_pascal: Vec<String> = msg
        .groups
        .iter()
        .map(|g| to_pascal_case(&g.name))
        .chain(msg.var_data.iter().map(|vd| to_pascal_case(&vd.name)))
        .collect();

    let stage_idents: Vec<syn::Ident> = if total_tail > 0 {
        let mut stages = vec![name_encoder_ident.clone()];
        for (i, field) in tail_pascal.iter().enumerate() {
            if i < total_tail - 1 {
                stages.push(syn::Ident::new(&format!("{}After{}", name, field), span));
            } else {
                stages.push(syn::Ident::new(&format!("{}Complete", name), span));
            }
        }
        stages
    } else {
        vec![name_encoder_ident.clone()]
    };

    if let Some(ref desc) = msg.description {
        ts.extend(doc_attr_tokens(desc));
    }
    for stage in &stage_idents {
        let stage_name = stage.to_string();
        let stage_name_lit = syn::LitStr::new(&stage_name, span);
        ts.extend(quote::quote! {
            #[must_use = "encoder must be consumed to write the message"]
            pub struct #stage<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
                buf: &'a mut [u8],
                msg_offset: usize,
                pos: usize,
                _header: core::marker::PhantomData<H>,
            }

        });
        // Encoder Display + Debug: only when the decoder has Display/Debug.
        if enable_display_debug {
            ts.extend(quote::quote! {
                impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for #stage<'a, H> {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        match #name_decoder_ident::decode(
                            self.buf, self.msg_offset,
                        ) {
                            Ok(dec) => core::fmt::Display::fmt(&dec, f),
                            Err(_) => write!(f, "<partial {}>", #stage_name_lit),
                        }
                    }
                }

                impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for #stage<'a, H> {
                    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                        match #name_decoder_ident::decode(
                            self.buf, self.msg_offset,
                        ) {
                            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
                            Err(_) => f.debug_struct(#stage_name_lit)
                                .field("msg_offset", &self.msg_offset)
                                .field("pos", &self.pos)
                                .field("buf_len", &self.buf.len())
                                .finish(),
                        }
                    }
                }
            });
        }
    }

    // Associated constants live on the *defaulted* concrete impl so
    // `CarEncoder::TEMPLATE_ID` needs no turbofish. Instance methods go on
    // the generic `H` impl so HeaderAbsent and HeaderPresent share setters.
    let mut impl_consts = proc_macro2::TokenStream::new();
    let mut impl_contents = proc_macro2::TokenStream::new();

    if is_fixed {
        impl_consts.extend(quote::quote! {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #block_length_lit;
            const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == #block_length_lit);
            /// Schema-declared message header size in bytes.
            pub const HEADER_LENGTH: usize = #header_size_lit;
            /// Stack-allocate with `let mut buf = [0u8; Msg::ENCODED_LENGTH];`
            /// Header-inclusive fixed length. Claim/app framing: payload starts
            /// at `frame[Self::ENCODED_LENGTH..]`.
            pub const ENCODED_LENGTH: usize = #encoded_length_lit;
            const _ENCODED_LEN: () = assert!(Self::ENCODED_LENGTH >= Self::BLOCK_LENGTH);
            /// Header-inclusive encoded length. Same as [`Self::ENCODED_LENGTH`];
            /// provided for API consistency with flat and complex message
            /// shapes so every encoder has a `compute_length_with_header` method.
            #[inline]
            pub const fn compute_length_with_header() -> usize {
                Self::ENCODED_LENGTH
            }
            pub const HEADER_TEMPLATE: [u8; #header_size_lit] = [#(#hdr_lits),*];
            const _HEADER_TEMPLATE_LEN: () =
                assert!(Self::HEADER_TEMPLATE.len() == #header_size_lit);
        });
        impl_contents.extend(quote::quote! {
            /// Absolute offset of this message within the original buffer
            /// (the `msg_offset` argument passed to `wrap`).
            #[inline]
            pub const fn message_offset(&self) -> usize {
                self.msg_offset
            }

            /// Absolute current write cursor within the original buffer.
            #[inline]
            pub const fn limit(&self) -> usize {
                self.pos
            }

            /// The complete original buffer this encoder wraps.
            #[inline]
            pub fn buffer(&self) -> &[u8] {
                self.buf
            }
        });
    } else {
        let impl_consts_suffix = if is_capped {
            // When theoretical max exceeds 64KB, do NOT emit MAX_ENCODED_LENGTH —
            // the constant would be a dangerous lie. Use EncodedLength instead.
            quote::quote! {}
        } else {
            quote::quote! {
                #[doc = " Upper bound of any encoded form of this message (header + body). \
                         Prefer exact sizing via `Self::compute_length()` / the staged \
                         `*EncodedLength` builder when the message has groups or var-data; \
                         a stack `[0u8; Self::MAX_ENCODED_LENGTH]` is fine only when this \
                         constant is a true fixed upper bound you intend to use."]
                pub const MAX_ENCODED_LENGTH: usize = #max_encoded_capped_lit;
                const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
            }
        };
        impl_consts.extend(quote::quote! {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #block_length_lit;
            const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == #block_length_lit);
            /// Schema-declared message header size in bytes.
            pub const HEADER_LENGTH: usize = #header_size_lit;
            #impl_consts_suffix
            pub const HEADER_TEMPLATE: [u8; #header_size_lit] = [#(#hdr_lits),*];
            const _HEADER_TEMPLATE_LEN: () =
                assert!(Self::HEADER_TEMPLATE.len() == #header_size_lit);
        });

        // compute_length() — convenience factory for the staged length builder
        if !encoded_len_gen.standalone.is_empty() {
            let el_ident = syn::Ident::new(&format!("{name}EncodedLength"), span);
            impl_consts.extend(quote::quote! {
                #[inline]
                pub const fn compute_length() -> #el_ident {
                    #el_ident::new()
                }
            });
        }
        impl_contents.extend(quote::quote! {
            /// Absolute offset of this message within the original buffer
            /// (the `msg_offset` argument passed to `wrap`).
            #[inline]
            pub const fn message_offset(&self) -> usize {
                self.msg_offset
            }

            /// Absolute current write cursor within the original buffer.
            #[inline]
            pub const fn limit(&self) -> usize {
                self.pos
            }

            /// The complete original buffer this encoder wraps.
            #[inline]
            pub fn buffer(&self) -> &[u8] {
                self.buf
            }
        });
    }

    // ── Hot-path bounds check: one cmp, cold error construction ──
    // The error path is `#[cold] #[inline(never)]` so the hot path is a single
    // `cmp + ja` followed by the same body as the unchecked companion. The
    // compiler keeps the cold `Err` constructor out of the hot icache, and
    // the branch predictor always predicts not-taken for correctly-sized buffers.
    let needed_lit = syn::LitInt::new(&(header_size + block_length).to_string(), span);
    let cold_check = quote::quote! {
        /// Cold error constructor — never inlined into the hot path.
        #[cold]
        #[inline(never)]
        fn buffer_too_short(buf: &[u8], pos: usize, needed: usize) -> sbe_rt::EncodeError {
            sbe_rt::EncodeError::BufferTooShort {
                field: "message header+body",
                needed,
                available: buf.len().saturating_sub(pos),
            }
        }
    };
    // Constructors + cold helper on the concrete (default-H) impl so
    // `CarEncoder::wrap_and_apply_header` needs no turbofish (HFT-001).
    impl_consts.extend(cold_check);

    // Three-tier constructors:
    //   try_*        — safe, returns Result on short buffers
    //   bare name    — safe, panics on short buffers (extent proved before
    //                  unchecked field setters)
    //   *_unchecked  — unsafe, caller proves HEADER + fixed body extent
    let wrap_fn = quote::quote! {
        /// Wrap a mutable buffer for encoding with one bounds/overflow check.
        /// Does **not** write the message header (`HeaderAbsent`).
        ///
        /// `msg_offset` is the **message start** (first byte of the SBE frame),
        /// not the body. sbe-tool Rust `wrap` takes the body offset instead.
        ///
        /// Prefer [`Self::wrap_and_apply_header`] when encoding a full frame.
        #[inline]
        pub fn try_wrap(
            buf: &'a mut [u8],
            msg_offset: usize,
        ) -> Result<#name_encoder_ident<'a, sbe_rt::HeaderAbsent>, sbe_rt::EncodeError> {
            if #needed_lit > buf.len().saturating_sub(msg_offset) {
                return Err(Self::buffer_too_short(buf, msg_offset, #needed_lit));
            }
            // SAFETY: extent check above proved header + fixed body fit.
            Ok(unsafe { Self::wrap_unchecked(buf, msg_offset) })
        }

        /// Trusted body-only wrap. Proves header + fixed-body extent then
        /// constructs; **panics** if the buffer is too short. Field setters
        /// use unchecked writes justified by that proof.
        ///
        /// Prefer [`Self::try_wrap`] at untrusted boundaries.
        #[inline]
        pub fn wrap(
            buf: &'a mut [u8],
            msg_offset: usize,
        ) -> #name_encoder_ident<'a, sbe_rt::HeaderAbsent> {
            if #needed_lit > buf.len().saturating_sub(msg_offset) {
                panic!("{}", Self::buffer_too_short(buf, msg_offset, #needed_lit));
            }
            // SAFETY: extent check above proved header + fixed body fit.
            unsafe { Self::wrap_unchecked(buf, msg_offset) }
        }

        /// Zero-check body-only wrap — raw pointer ops, **UB** on OOB.
        /// Only for proven-tight HFT loops where the panic machinery is
        /// measurable in the critical path.
        ///
        /// # Safety
        /// `msg_offset + HEADER_LENGTH + BLOCK_LENGTH` must not overflow
        /// and must be ≤ `buf.len()` for the lifetime of the encoder.
        #[inline]
        pub unsafe fn wrap_unchecked(
            buf: &'a mut [u8],
            msg_offset: usize,
        ) -> #name_encoder_ident<'a, sbe_rt::HeaderAbsent> {
            let body_pos = msg_offset + #header_size_lit;
            #name_encoder_ident {
                buf,
                msg_offset,
                pos: body_pos + #block_length_lit,
                _header: core::marker::PhantomData,
            }
        }
    };
    impl_consts.extend(wrap_fn);

    let wrap_apply_fn = quote::quote! {
        /// Wrap a mutable buffer, write the header, with one bounds/overflow check.
        /// `pos` is the **message start** (see [`Self::wrap`]).
        ///
        /// Optional-field nullification is **not** applied by default — call
        /// `apply_nulls()` if you want null sentinels.
        #[inline]
        pub fn try_wrap_and_apply_header(
            buf: &'a mut [u8],
            pos: usize,
        ) -> Result<#name_encoder_ident<'a, sbe_rt::HeaderPresent>, sbe_rt::EncodeError> {
            if #needed_lit > buf.len().saturating_sub(pos) {
                return Err(Self::buffer_too_short(buf, pos, #needed_lit));
            }
            // SAFETY: extent check above proved header + fixed body fit.
            Ok(unsafe { Self::wrap_and_apply_header_unchecked(buf, pos) })
        }

        /// Trusted full-frame wrap + header. Proves header + fixed-body extent
        /// then writes the header; **panics** if the buffer is too short.
        /// Field setters use unchecked writes justified by that proof.
        ///
        /// Prefer [`Self::try_wrap_and_apply_header`] at untrusted boundaries.
        /// Call [`Self::wrap_and_apply_header_unchecked`] only with a proven
        /// extent when even panic machinery must be avoided.
        #[inline]
        pub fn wrap_and_apply_header(
            buf: &'a mut [u8],
            pos: usize,
        ) -> #name_encoder_ident<'a, sbe_rt::HeaderPresent> {
            if #needed_lit > buf.len().saturating_sub(pos) {
                panic!("{}", Self::buffer_too_short(buf, pos, #needed_lit));
            }
            // SAFETY: extent check above proved header + fixed body fit.
            unsafe { Self::wrap_and_apply_header_unchecked(buf, pos) }
        }

        /// Zero-check full-frame wrap + header — `copy_nonoverlapping`, **UB**
        /// on OOB. Only for proven-tight HFT loops.
        ///
        /// # Safety
        /// `pos + HEADER_LENGTH + BLOCK_LENGTH` must not overflow and must be
        /// ≤ `buf.len()` for the lifetime of the encoder.
        #[inline]
        pub unsafe fn wrap_and_apply_header_unchecked(
            buf: &'a mut [u8],
            pos: usize,
        ) -> #name_encoder_ident<'a, sbe_rt::HeaderPresent> {
            // SAFETY: caller guarantees pos + HEADER_LENGTH ≤ buf.len().
            unsafe {
                core::ptr::copy_nonoverlapping(
                    Self::HEADER_TEMPLATE.as_ptr(),
                    buf.as_mut_ptr().add(pos),
                    #header_size_lit,
                );
            }
            let body_pos = pos + #header_size_lit;
            #name_encoder_ident {
                buf,
                msg_offset: pos,
                pos: body_pos + #block_length_lit,
                _header: core::marker::PhantomData,
            }
        }
    };
    impl_consts.extend(wrap_apply_fn);

    // Claim-compatible wrap: validates buffer is exactly ENCODED_LENGTH bytes.
    // For use with try_claim / pre-sized claim buffers where the buffer is pre-sized to the message.
    if is_fixed {
        impl_consts.extend(quote::quote! {
            /// Wrap a mutable buffer sized exactly to `ENCODED_LENGTH` bytes.
            /// For use with claim buffers (`try_claim`) where the caller has
            /// already allocated exactly the right size.
            #[inline]
            pub fn wrap_into_claim(
                buf: &'a mut [u8],
            ) -> Result<#name_encoder_ident<'a, sbe_rt::HeaderPresent>, sbe_rt::EncodeError> {
                if buf.len() != Self::ENCODED_LENGTH {
                    return Err(sbe_rt::EncodeError::ClaimLengthMismatch {
                        expected: Self::ENCODED_LENGTH,
                        actual: buf.len(),
                    });
                }
                Ok(Self::wrap_and_apply_header(buf, 0))
            }
        });
    }

    // Opt-in: write null sentinels for all optional fields. Call this after
    // wrap_and_apply_header if you want unset optional fields to carry their
    // schema-defined null value instead of whatever was in the buffer.
    // Not called by default (sbe-tool does not nullify on wrap).
    {
        let mut null_buf = String::new();
        let offset_base = format!("self.msg_offset + {header_size}");
        generate_nullification(
            &mut null_buf,
            &msg.fields,
            &offset_base,
            "self.buf",
            byte_order,
        );
        if !null_buf.is_empty() {
            let null_ts: proc_macro2::TokenStream = null_buf
                .parse()
                .expect("generate_nullification produced invalid token stream");
            let apply_nulls_fn = quote::quote! {
                /// Write the schema-defined null sentinel into every optional field.
                ///
                /// Optional only — `wrap_and_apply_header` does not nullify by default
                /// (matching sbe-tool). Call this if you want unset optional fields to
                /// carry their null value rather than stale buffer contents.
                #[inline]
                pub fn apply_nulls(&mut self) -> &mut Self {
                    #null_ts
                    self
                }
            };
            impl_contents.extend(apply_nulls_fn);
        }
    }

    const ENCODER_RESERVED: &[&str] = &[
        "message_offset",
        "limit",
        "buffer",
        "wrap",
        "wrap",
        "wrap_and_apply_header",
        "wrap_and_apply_header",
        "wrap_into_claim",
        "compute_length_with_header",
        // Complete-stage inherent methods emitted on the encoder struct — a
        // field named after any of these would otherwise collide (matches the
        // corresponding names in DECODER_RESERVED).
        "as_body_bytes",
        "as_bytes_with_header",
        "into_remaining_mut",
        "encoded_length",
        "encoded_length_with_header",
        // Emitted when the message has optional fields.
        "apply_nulls",
        // Stage transitions taking `self` (encoded-length struct wraps into
        // a stage, so they always exist on the main encoder struct).
        "fixed",
        "raw_fixed",
        // Associated fn (no receiver) — a field-named setter with `&mut self`
        // collides with this because Rust does not separate associated fns from
        // methods in the inherent namespace.
        "buffer_too_short",
    ];

    for f in &msg.fields {
        let f_name = to_snake_case(&f.name);
        // Offset of this field from the message header start (header + body offset).
        let body_offset = header_size + f.offset;
        let body_offset_lit = syn::LitInt::new(&body_offset.to_string(), span);
        // Absolute buffer index under the truthful coordinate system.
        let abs_offset = quote::quote! { self.msg_offset + #body_offset_lit };
        // In converter mode, raw setters are suffixed _wire when a domain
        // Raw setters become *_wire when a conversion is configured so the
        // converted setter takes the original name.
        let wire_name = field_has_conversion_free(f, conversions).then(|| format!("{f_name}_wire"));
        let method_name = wire_name.as_deref().unwrap_or(&f_name);
        let f_ident = resolve_field_ident(&f_name, &wire_name, ENCODER_RESERVED);

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                if f.presence == Presence::Constant {
                    continue;
                }
                let prim_size = prim.size();
                let prim_size_lit = syn::LitInt::new(&prim_size.to_string(), span);
                let r_type: syn::Type = syn::parse_str(rust_type(*prim)).unwrap();
                if let Some(len) = length {
                    let len_lit = syn::LitInt::new(&len.to_string(), span);
                    if prim_size == 1 {
                        // [u8; N] / [i8; N] / char: no multi-byte endian swap; bulk
                        // copy via u8 view (i8 arrays cannot `copy_from_slice` into [u8]).
                        impl_contents.extend(quote::quote! {
                            #[inline]
                            pub fn #f_ident(&mut self, val: [#r_type; #len_lit]) -> &mut Self {
                                let offset = #abs_offset;
                                unsafe {
                                    let dst = self.buf.get_unchecked_mut(offset..offset + #len_lit);
                                    let src = core::slice::from_raw_parts(
                                        val.as_ptr() as *const u8,
                                        #len_lit,
                                    );
                                    dst.copy_from_slice(src);
                                }
                                self
                            }
                        });
                        // Zero-padded string write (Java vehicleCode(String) parity).
                        let str_ident = syn::Ident::new(&format!("{method_name}_str"), span);
                        let field_lit = syn::LitStr::new(&f.name, span);
                        impl_contents.extend(quote::quote! {
                            #[inline]
                            pub fn #str_ident(&mut self, src: &str) -> Result<&mut Self, sbe_rt::EncodeError> {
                                if src.len() > #len_lit {
                                    return Err(sbe_rt::EncodeError::FixedArrayTooLong {
                                        field: #field_lit,
                                        max_length: #len_lit,
                                        actual: src.len(),
                                    });
                                }
                                let mut tmp = [0 as #r_type; #len_lit];
                                let bytes = src.as_bytes();
                                let mut i = 0usize;
                                while i < bytes.len() {
                                    tmp[i] = bytes[i] as #r_type;
                                    i += 1;
                                }
                                Ok(self.#f_ident(tmp))
                            }
                        });
                    } else {
                        impl_contents.extend(quote::quote! {
                            #[inline]
                            pub fn #f_ident(&mut self, val: [#r_type; #len_lit]) -> &mut Self {
                                let offset = #abs_offset;
                                let mut idx = 0usize;
                                while idx < #len_lit {
                                    unsafe {
                                        self.buf.get_unchecked_mut(offset + idx * #prim_size_lit..offset + (idx + 1) * #prim_size_lit)
                                            .copy_from_slice(&val[idx].#to_endian());
                                    }
                                    idx += 1;
                                }
                                self
                            }
                        });
                    }
                    // Unrolled put_field(v0, v1, …) for small fixed arrays (Java putSomeNumbers).
                    if (2..=8).contains(len) {
                        let put_ident = syn::Ident::new(&format!("put_{method_name}"), span);
                        let params: Vec<syn::Ident> = (0..*len)
                            .map(|i| syn::Ident::new(&format!("v{i}"), span))
                            .collect();
                        impl_contents.extend(quote::quote! {
                            #[inline]
                            pub fn #put_ident(&mut self, #(#params: #r_type),*) -> &mut Self {
                                self.#f_ident([#(#params),*])
                            }
                        });
                    }
                } else if prim_size == 1 {
                    // Direct byte write for u8/i8/char — 1 instruction vs 3.
                    impl_contents.extend(quote::quote! {
                        #[inline]
                        pub fn #f_ident(&mut self, val: #r_type) -> &mut Self {
                            *unsafe { self.buf.get_unchecked_mut(#abs_offset) } = val as u8;
                            self
                        }
                    });
                } else {
                    impl_contents.extend(quote::quote! {
                        #[inline]
                        pub fn #f_ident(&mut self, val: #r_type) -> &mut Self {
                            let offset = #abs_offset;
                            // SAFETY: wrap/wrap_and_apply_header validates buf.len() >= msg_offset + HEADER + BLOCK,
                            // and field extent is within BLOCK_LENGTH by construction.
                            unsafe {
                                self.buf.get_unchecked_mut(offset..offset + #prim_size_lit)
                                    .copy_from_slice(&val.#to_endian());
                            }
                            self
                        }
                    });
                }
            }
            FieldType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target_type: syn::Type = syn::parse_str(&to_pascal_case(comp_name)).unwrap();
                let comp_size_lit = syn::LitInt::new(&comp_size.to_string(), span);
                impl_contents.extend(quote::quote! {
                    #[inline]
                    pub fn #f_ident(&mut self, val: #target_type) -> &mut Self {
                        let offset = #abs_offset;
                        self.buf[offset..offset + #comp_size_lit]
                            .copy_from_slice(&val.0);
                        self
                    }
                });
            }
            FieldType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                let target_type: syn::Type = syn::parse_str(&to_pascal_case(enum_name)).unwrap();
                let r_type: syn::Type = syn::parse_str(rust_type(*encoding_type)).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit = syn::LitInt::new(&prim_size.to_string(), span);
                impl_contents.extend(quote::quote! {
                    #[inline]
                    pub fn #f_ident(&mut self, val: #target_type) -> &mut Self {
                        let offset = #abs_offset;
                        self.buf[offset..offset + #prim_size_lit].copy_from_slice(&(val as #r_type).#to_endian());
                        self
                    }
                });
                // Boolean fields get an additional setter that accepts bool directly
                if crate::structured_ir::is_bool_enum(elements, enum_name) {
                    let f_name_bool = syn::Ident::new(&format!("{}_bool", f_name), span);
                    impl_contents.extend(quote::quote! {
                        #[inline]
                        pub fn #f_name_bool(&mut self, val: bool) -> &mut Self {
                            self.buf[#abs_offset] = val as u8;
                            self
                        }
                    });
                }
            }
            FieldType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target_type: syn::Type = syn::parse_str(&to_pascal_case(set_name)).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit = syn::LitInt::new(&prim_size.to_string(), span);
                impl_contents.extend(quote::quote! {
                    #[inline]
                    pub fn #f_ident(&mut self, val: #target_type) -> &mut Self {
                        let offset = #abs_offset;
                        self.buf[offset..offset + #prim_size_lit].copy_from_slice(&val.0.#to_endian());
                        self
                    }
                });
            }
        }
        // Field id / offset / length / MetaAttribute (also on encoder, Java parity).
        // Field NULL/MIN/MAX consts on concrete impl — turbofish-free access.
        if enable_meta_attributes {
            impl_consts.extend(emit_field_consts(f));
        }
    }

    // No partial as_bytes on incomplete stages — complete-message byte/length
    // views exist only on the terminal complete stage (DECISIONS.md §2).
    // Callers that genuinely need partial inspection should use an explicit
    // name such as `written_prefix()`."

    // Encoded-length support: strategy-classified (computed above).
    // Length helpers are associated functions — keep on concrete impl for
    // turbofish-free `CarEncoder::compute_encoded_length_with_message_header(...)`.
    impl_consts.extend(encoded_len_gen.encoder_impl.clone());

    // A complete, owned, latest-version snapshot of every required fixed
    // field. Optional fields are `Option<T>`; constants are excluded.
    // No `Default` — every required field must be explicitly initialised.
    {
        let fixed_name = syn::Ident::new(&format!("{name}FixedFields"), span);
        let mut fixed_fields_ts = proc_macro2::TokenStream::new();
        for f in &msg.fields {
            if f.presence == crate::Presence::Constant {
                continue;
            }
            let fname_snake = to_snake_case(&f.name);
            let f_ident = syn::Ident::new(&fname_snake, span);
            let is_optional = f.presence == crate::Presence::Optional;
            if is_optional {
                let ty = field_type_ident(&f.field_type, span);
                fixed_fields_ts.extend(quote::quote! {
                    pub #f_ident: Option<#ty>,
                });
            } else {
                let ty = field_type_ident(&f.field_type, span);
                fixed_fields_ts.extend(quote::quote! {
                    pub #f_ident: #ty,
                });
            }
        }
        ts.extend(quote::quote! {
            /// Complete set of latest-version fixed fields for this message.
            /// Required fields are concrete values; optional/versioned fields
            /// are `Option<T>`. Constants are excluded.
            ///
            /// This struct is **intentionally exhaustive** (not
            /// `#[non_exhaustive]`): when the schema adds a fixed field, every
            /// `fixed(&…)` call site must be updated. That is a feature — schema
            /// changes surface as compile errors rather than silent defaults.
            #[derive(Debug, Clone)]
            pub struct #fixed_name {
                #fixed_fields_ts
            }
        });
    }

    {
        let fixed_name = syn::Ident::new(&format!("{name}FixedFields"), span);
        // Build the write block: for each non-constant field, write from the struct.
        // Use _wire suffixed setters for converter-enabled composite fields.
        let mut write_stmts = proc_macro2::TokenStream::new();
        for f in &msg.fields {
            if f.presence == crate::Presence::Constant {
                continue;
            }
            let fname_snake = to_snake_case(&f.name);
            let is_converted = field_has_conversion_free(f, conversions);
            let setter_ident = {
                let base = resolve_field_ident(&fname_snake, &None, ENCODER_RESERVED);
                if is_converted {
                    syn::Ident::new(&format!("{}_wire", base), span)
                } else {
                    base
                }
            };
            let field_ident = syn::Ident::new(&fname_snake, span);
            if f.presence == crate::Presence::Optional {
                // Optional fields: write when Some, skip when None.
                // Callers who need null sentinels can call apply_nulls() explicitly.
                write_stmts.extend(quote::quote! {
                    if let Some(ref v) = fixed.#field_ident {
                        self.#setter_ident(*v);
                    }
                });
            } else {
                write_stmts.extend(quote::quote! {
                    self.#setter_ident(fixed.#field_ident);
                });
            }
        }
        impl_contents.extend(quote::quote! {
            /// Set all fixed fields at once from a [`#fixed_name`] value.
            /// Required fields are always written; optional fields are
            /// written when `Some`. Returns the encoder for tail methods.
            #[inline]
            #[must_use]
            pub fn fixed(mut self, fixed: &#fixed_name) -> Self {
                #write_stmts
                self
            }
        });
    }

    {
        let raw_name = syn::Ident::new(&format!("{name}RawFixedWriter"), span);
        ts.extend(quote::quote! {
            /// Raw fixed-field writer. Individual field setters are available
            /// only on this writer. When done, embed the fields in a
            /// `#fixed_name` and call the encoder's `fixed()`.
            #[must_use = "raw fixed writer must be embedded in FixedFields"]
            pub struct #raw_name<'a> {
                buf: &'a mut [u8],
                msg_offset: usize,
                pos: usize,
            }
        });
        impl_contents.extend(quote::quote! {
            /// Return a dedicated raw fixed-field writer. All individual field
            /// setters are available on the writer. To advance to tail stages,
            /// collect the values into a `#fixed_name` and call `fixed()`.
            #[inline]
            #[must_use]
            pub fn raw_fixed(self) -> #raw_name<'a> {
                let body_start = self.msg_offset + #header_size_lit;
                #raw_name {
                    buf: &mut self.buf[body_start..],
                    msg_offset: 0,
                    pos: self.pos - body_start,
                }
            }
        });
    }

    // Concrete (default H) for associated constants + constructors — no turbofish.
    ts.extend(quote::quote! {
        impl<'a> #name_encoder_ident<'a> {
            #impl_consts
        }
    });
    // Generic over H so body-only wrap (HeaderAbsent) can set fields and run
    // the same stage chain. Default `H = HeaderPresent` keeps the happy path
    // inference-friendly.
    ts.extend(quote::quote! {
        impl<'a, H: sbe_rt::HeaderState> #name_encoder_ident<'a, H> {
            #impl_contents
        }
    });

    // ── Metadata facet ──────────────────────────────────────────────────
    let enc_metadata_ident = syn::Ident::new(&format!("{}EncoderMetadata", name), span);
    // Complete-sounding `as_bytes_with_header` only when there are no tails;
    // otherwise this stage is fixed-block only and must not look like a frame.
    let meta_bytes = if total_tail == 0 {
        quote::quote! {
            /// Message body bytes written so far (header exclusive).
            #[inline]
            pub fn as_body_bytes(&self) -> &[u8] {
                &self.encoder.buf[self.encoder.msg_offset + #header_size_lit..self.encoder.pos]
            }
            /// Header-inclusive frame bytes (message is fixed-only — complete).
            #[inline]
            pub fn as_bytes_with_header(&self) -> &[u8] {
                &self.encoder.buf[self.encoder.msg_offset..self.encoder.pos]
            }
        }
    } else {
        quote::quote! {
            /// Fixed-block body bytes only (groups/var-data not yet written).
            /// For a complete frame use the terminal stage's
            /// `as_bytes_with_header`.
            #[inline]
            pub fn as_fixed_body_bytes(&self) -> &[u8] {
                &self.encoder.buf[self.encoder.msg_offset + #header_size_lit..self.encoder.pos]
            }
            /// Header + fixed block only — **not** a complete SBE message when
            /// groups or var-data remain. Prefer the complete stage's
            /// `as_bytes_with_header`.
            #[inline]
            pub fn as_fixed_region_with_header(&self) -> &[u8] {
                &self.encoder.buf[self.encoder.msg_offset..self.encoder.pos]
            }
        }
    };
    ts.extend(quote::quote! {
        /// Buffer-placement metadata. Holds a reference to the parent encoder
        /// — zero-copy. Utility methods live here so no schema field can
        /// collide with them.
        #[derive(Clone, Copy)]
        pub struct #enc_metadata_ident<'m, 'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
            encoder: &'m #name_encoder_ident<'a, H>,
        }

        impl<'m, 'a, H: sbe_rt::HeaderState> #enc_metadata_ident<'m, 'a, H> {
            #meta_bytes
            /// Absolute offset of this message within the original buffer.
            #[inline]
            pub fn message_offset(&self) -> usize {
                self.encoder.msg_offset
            }
        }
    });

    ts.extend(quote::quote! {
        impl<'a, H: sbe_rt::HeaderState> #name_encoder_ident<'a, H> {
            /// Metadata accessor: buffer positions, wire-frame boundaries.
            /// Returns a zero-copy reference to the parent encoder.
            /// All utility methods are scoped here so no schema field name
            /// can collide with them.
            #[inline]
            pub fn get_metadata(&self) -> #enc_metadata_ident<'_, 'a, H> {
                #enc_metadata_ident { encoder: self }
            }
        }
    });

    if total_tail > 0 {
        let mut tail_idx = 0;

        for g in &msg.groups {
            let current_stage = &stage_idents[tail_idx];
            let next_stage = &stage_idents[tail_idx + 1];

            let g_snake = syn::Ident::new(&to_snake_case(&g.name), span);
            let raw_enc_name = to_pascal_case(&g.name);
            let scoped_enc = if multi_message {
                format!("{}{}", &name, raw_enc_name)
            } else {
                raw_enc_name
            };
            let g_pascal_enc = syn::Ident::new(&format!("{scoped_enc}Encoder"), span);
            let (_dim_name, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
            let (num_offset, num_size, num_prim) = get_dim_num_layout(elements, &g.dimension_type);
            let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), span);
            let num_offset_lit = syn::LitInt::new(&num_offset.to_string(), span);
            let num_size_lit = syn::LitInt::new(&num_size.to_string(), span);
            let count_ty: syn::Type = syn::parse_str(rust_type(num_prim)).unwrap();

            let g_snake_unknown =
                syn::Ident::new(&format!("{}_unknown_size", to_snake_case(&g.name)), span);

            ts.extend(quote::quote! {
                impl<'a, H: sbe_rt::HeaderState> #current_stage<'a, H> {
                    /// Encode this group with a known count up front.
                    /// Closures return [`sbe_rt::GroupResult`]
                    /// (`Result<(), EncodeError>`); `?` works — there is no
                    /// separate `try_*` method name.
                    #[inline]
                    #[must_use]
                    pub fn #g_snake<F>(
                        mut self,
                        count: #count_ty,
                        f: F,
                    ) -> Result<#next_stage<'a, H>, sbe_rt::EncodeError>
                    where
                                                F: FnOnce(&mut #g_pascal_enc<'a>) -> sbe_rt::GroupResult,
                    {
                        if self.pos + #dim_size_lit > self.buf.len() {
                            return Err(sbe_rt::EncodeError::BufferTooShort {
                                field: stringify!(#g_snake),
                                needed: #dim_size_lit,
                                available: self.buf.len().saturating_sub(self.pos),
                            }
                            .into());
                        }
                        self.buf[self.pos..self.pos + #dim_size_lit]
                            .copy_from_slice(&#g_pascal_enc::GROUP_DIM_TEMPLATE);
                        self.buf
                            [self.pos + #num_offset_lit..self.pos + #num_offset_lit + #num_size_lit]
                            .copy_from_slice(&count.#to_endian());
                        let mut group =
                            #g_pascal_enc::wrap(self.buf, self.pos + #dim_size_lit, count);
                        f(&mut group)?;
                        let written = group.written();
                        if written != count {
                            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                                declared: count as u32,
                                actual: written as u32,
                            });
                        }
                        Ok(#next_stage {
                            buf: group.buf,
                            msg_offset: self.msg_offset,
                            pos: group.pos,
                            _header: core::marker::PhantomData,
                        })
                    }

                    /// Encode this group without knowing the count up front.
                    /// The dimension header is written with a zero placeholder;
                    /// after the closure returns, the actual entry count is
                    /// back-patched into the header. No `GroupFull` check —
                    /// overflow is the caller's responsibility.
                    ///
                    /// Prefer [`Self::#g_snake`] when the count is known at
                    /// compile time or from a small input.
                    #[inline]
                    #[must_use]
                    pub fn #g_snake_unknown<F>(
                        mut self,
                        f: F,
                    ) -> Result<#next_stage<'a, H>, sbe_rt::EncodeError>
                    where
                                                F: FnOnce(&mut #g_pascal_enc<'a>) -> sbe_rt::GroupResult,
                    {
                        if self.pos + #dim_size_lit > self.buf.len() {
                            return Err(sbe_rt::EncodeError::BufferTooShort {
                                field: stringify!(#g_snake),
                                needed: #dim_size_lit,
                                available: self.buf.len().saturating_sub(self.pos),
                            }
                            .into());
                        }
                        self.buf[self.pos..self.pos + #dim_size_lit]
                            .copy_from_slice(&#g_pascal_enc::GROUP_DIM_TEMPLATE);
                        let count_offset = self.pos + #num_offset_lit;
                        self.buf[count_offset..count_offset + #num_size_lit].fill(0);
                        // Use MAX count to skip GroupFull checks during add().
                        // Run in a block so group's reborrow of self.buf ends
                        // before we back-patch the count.
                        let (buf, pos, actual) = {
                            let mut group = #g_pascal_enc::wrap(
                                self.buf, self.pos + #dim_size_lit, #count_ty::MAX,
                            );
                            f(&mut group)?;
                            let n = group.written();
                            (group.buf, group.pos, n)
                        };
                        // Back-patch the actual count.
                        buf[count_offset..count_offset + #num_size_lit]
                            .copy_from_slice(&actual.#to_endian());
                        Ok(#next_stage {
                            buf,
                            msg_offset: self.msg_offset,
                            pos,
                            _header: core::marker::PhantomData,
                        })
                    }
                }
            });
            tail_idx += 1;
        }

        // VarData methods
        for vd in &msg.var_data {
            let current_stage = &stage_idents[tail_idx];
            let next_stage = &stage_idents[tail_idx + 1];

            let vd_snake = syn::Ident::new(&to_snake_case(&vd.name), span);
            let vd_snake_unchecked =
                syn::Ident::new(&format!("{}_unchecked", to_snake_case(&vd.name)), span);
            let vd_snake_with = syn::Ident::new(&format!("{}_with", to_snake_case(&vd.name)), span);
            let (_, prefix_size, _, len_type) = get_vardata_info(elements, &vd.type_name);
            let prefix_size_lit = syn::LitInt::new(&prefix_size.to_string(), span);
            let len_rust_type: syn::Type = syn::parse_str(rust_type(len_type)).unwrap();

            // Checked body: conditionally includes max_length guard.
            let mut checked_body = proc_macro2::TokenStream::new();
            let mut with_checked_body = proc_macro2::TokenStream::new();
            if let Some(max) = vd.max_length {
                let max_lit = syn::LitInt::new(&max.to_string(), span);
                let vd_name_str = &vd.name;
                checked_body.extend(quote::quote! {
                    if data.len() > #max_lit {
                        return Err(sbe_rt::EncodeError::VarDataTooLong {
                            field: #vd_name_str,
                            max_length: #max_lit,
                            actual: data.len(),
                        });
                    }
                });
                with_checked_body.extend(quote::quote! {
                    if exact_len > #max_lit {
                        return Err(sbe_rt::EncodeError::VarDataTooLong {
                            field: #vd_name_str,
                            max_length: #max_lit,
                            actual: exact_len,
                        }.into());
                    }
                });
            }

            let shared_body = quote::quote! {
                let needed = #prefix_size_lit + data.len();
                if self.pos + needed > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        field: stringify!(#vd_snake),
                        needed,
                        available: self.buf.len().saturating_sub(self.pos),
                    });
                }
                let wire_length = <#len_rust_type>::try_from(data.len()).map_err(|_| {
                    sbe_rt::EncodeError::VarDataTooLong {
                        field: stringify!(#vd_snake),
                        max_length: <#len_rust_type>::MAX as usize,
                        actual: data.len(),
                    }
                })?;
                let len_bytes = wire_length.#to_endian();
                self.buf[self.pos..self.pos + #prefix_size_lit]
                    .copy_from_slice(&len_bytes);
                let start = self.pos + #prefix_size_lit;
                self.buf[start..start + data.len()].copy_from_slice(data);
                Ok(#next_stage {
                    buf: self.buf,
                    msg_offset: self.msg_offset,
                    pos: start + data.len(),
                    _header: core::marker::PhantomData,
                })
            };

            ts.extend(quote::quote! {
                impl<'a, H: sbe_rt::HeaderState> #current_stage<'a, H> {
                    #[inline]
                    #[must_use]
                    pub fn #vd_snake(
                        mut self,
                        data: &[u8],
                    ) -> Result<#next_stage<'a, H>, sbe_rt::EncodeError> {
                        #checked_body
                        #shared_body
                    }

                    #[inline]
                    #[must_use]
                    pub fn #vd_snake_unchecked(
                        mut self,
                        data: &[u8],
                    ) -> Result<#next_stage<'a, H>, sbe_rt::EncodeError> {
                        #shared_body
                    }

                    /// Lend exactly `exact_len` bytes of the var-data region
                    /// to a closure for nested-message encoding. Zero-copy:
                    /// the closure writes directly into the outer buffer.
                    ///
                    /// Canonical nested-SBE pattern (AppMessage → L2Book):
                    /// ```text
                    /// let inner_len = InnerEncoder::compute_length_with_header(...);
                    /// after.payload_with(inner_len, |payload| {
                    ///     let len = InnerEncoder::wrap_and_apply_header(payload, 0)?
                    ///         .field(value)
                    ///         // continue the single encoder chain through all tail stages
                    ///         .encoded_length_with_header();
                    ///     debug_assert_eq!(len, inner_len);
                    ///     Ok(())
                    /// })?;
                    /// ```
                    /// Returns the next stage on success; on failure the
                    /// caller error propagates unchanged and no partial
                    /// data is published.
                    #[inline]
                    #[must_use]
                    pub fn #vd_snake_with<E, F>(
                        mut self,
                        exact_len: usize,
                        f: F,
                    ) -> Result<#next_stage<'a, H>, E>
                    where
                        E: From<sbe_rt::EncodeError>,
                        F: FnOnce(&mut [u8]) -> Result<(), E>,
                    {
                        #with_checked_body
                        let needed = #prefix_size_lit + exact_len;
                        if self.pos + needed > self.buf.len() {
                            return Err(sbe_rt::EncodeError::BufferTooShort {
                                field: stringify!(#vd_snake),
                                needed,
                                available: self.buf.len().saturating_sub(self.pos),
                            }.into());
                        }
                        let wire_length = <#len_rust_type>::try_from(exact_len).map_err(|_| {
                            sbe_rt::EncodeError::VarDataTooLong {
                                field: stringify!(#vd_snake),
                                max_length: <#len_rust_type>::MAX as usize,
                                actual: exact_len,
                            }
                        })?;
                        let len_bytes = wire_length.#to_endian();
                        self.buf[self.pos..self.pos + #prefix_size_lit]
                            .copy_from_slice(&len_bytes);
                        let start = self.pos + #prefix_size_lit;
                        f(&mut self.buf[start..start + exact_len])?;
                        Ok(#next_stage {
                            buf: self.buf,
                            msg_offset: self.msg_offset,
                            pos: start + exact_len,
                            _header: core::marker::PhantomData,
                        })
                    }
                }
            });
            tail_idx += 1;
        }

        // Complete state: body methods on any H; header bytes only on HeaderPresent.
        let complete_ident = &stage_idents[total_tail];
        ts.extend(quote::quote! {
            impl<'a, H: sbe_rt::HeaderState> #complete_ident<'a, H> {
                /// SBE message body bytes (excluding the message header).
                #[inline]
                pub fn as_body_bytes(&self) -> &[u8] {
                    let body_start = self.msg_offset + #header_size_lit;
                    &self.buf[body_start..self.pos]
                }
                /// SBE message body length (excluding the message header).
                #[inline]
                pub fn encoded_length(&self) -> usize {
                    self.pos - self.msg_offset - #header_size_lit
                }
                /// Total SBE message length including the header region.
                /// Pure arithmetic — available for body-only wraps too.
                #[inline]
                pub fn encoded_length_with_header(&self) -> usize {
                    self.pos - self.msg_offset
                }
                /// Unwritten region after this message.
                #[inline]
                pub fn into_remaining_mut(self) -> &'a mut [u8] {
                    &mut self.buf[self.pos..]
                }
            }

            impl<'a> #complete_ident<'a, sbe_rt::HeaderPresent> {
                /// Header-inclusive bytes. Only available when the encoder was
                /// constructed via `wrap_and_apply_header` (not raw `wrap`).
                #[inline]
                pub fn as_bytes_with_header(&self) -> &[u8] {
                    &self.buf[self.msg_offset..self.pos]
                }
            }
        });
    } else {
        ts.extend(quote::quote! {
            impl<'a, H: sbe_rt::HeaderState> #name_encoder_ident<'a, H> {
                /// SBE message body bytes (excluding the message header).
                #[inline]
                pub fn as_body_bytes(&self) -> &[u8] {
                    let body_start = self.msg_offset + #header_size_lit;
                    &self.buf[body_start..self.pos]
                }
                /// SBE message body length (excluding the message header).
                #[inline]
                pub fn encoded_length(&self) -> usize {
                    self.pos - self.msg_offset - #header_size_lit
                }
                /// Total SBE message length including the header region.
                /// Pure arithmetic — available for body-only wraps too.
                #[inline]
                pub fn encoded_length_with_header(&self) -> usize {
                    self.pos - self.msg_offset
                }
                /// Unwritten region after this message.
                #[inline]
                pub fn into_remaining_mut(self) -> &'a mut [u8] {
                    &mut self.buf[self.pos..]
                }
            }

            impl<'a> #name_encoder_ident<'a, sbe_rt::HeaderPresent> {
                /// Header-inclusive bytes. Only available when the encoder was
                /// constructed via `wrap_and_apply_header` (not raw `wrap`).
                #[inline]
                pub fn as_bytes_with_header(&self) -> &[u8] {
                    &self.buf[self.msg_offset..self.pos]
                }
            }
        });
    }

    if total_tail > 0 {
        ts.extend(quote::quote! {
            impl<'a> sbe_rt::private::Sealed for #name_encoder_ident<'a> {}

            impl<'a> sbe_rt::SbeMessage for #name_encoder_ident<'a> {
                const TEMPLATE_ID: u16 = #msg_id_lit;
                const BLOCK_LENGTH: usize = #block_length_lit;
                const SCHEMA_ID: u16 = #schema_id_lit;
                const SCHEMA_VERSION: u16 = #schema_version_lit;
            }
        });
    } else {
        ts.extend(quote::quote! {
            impl<'a> sbe_rt::private::Sealed for #name_encoder_ident<'a> {}

            impl<'a> sbe_rt::SbeMessage for #name_encoder_ident<'a> {
                const TEMPLATE_ID: u16 = #msg_id_lit;
                const BLOCK_LENGTH: usize = #block_length_lit;
                const SCHEMA_ID: u16 = #schema_id_lit;
                const SCHEMA_VERSION: u16 = #schema_version_lit;
            }
        });
    }

    let mut group_buf = String::new();
    let enc_group_names: Vec<String> = msg
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
    for (gi, g) in msg.groups.iter().enumerate() {
        generate_group_encoder(
            &mut group_buf,
            g,
            elements,
            byte_order,
            &enc_group_names[gi],
            &conversions,
            domain_types,
        );
    }
    if !group_buf.is_empty() {
        let group_ts: proc_macro2::TokenStream = group_buf
            .parse()
            .expect("generate_group_encoder produced invalid token stream");
        ts.extend(group_ts);
    }

    // Checked + unsafe unchecked constructors are emitted once on the
    // concrete impl above (HFT-001). Do not re-emit a second safe zero-check
    // pair here — that reintroduced UB from safe Rust.

    ts.extend(encoded_len_gen.standalone);

    ts
}

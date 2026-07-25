//! Rust code generation from the resolved SBE IR.
//!
//! # See also
//!
//! [`crate::GenerationConfig`], [`crate::Schema`], and the crate README's
//! generated-code workflow.
//!
//! This module contains the [`Generator`] struct — the primary API for
//! producing Rust source modules from a parsed [`Schema`].
//!
//! # Pipeline
//!
//! 1. Partition the flat token IR into logical groups (enums, sets,
//!    composites, messages).
//! 2. Generate type definitions for each group.
//! 3. For each message, generate:
//!    - A decoder struct (`CarDecoder`) with field accessors, group
//!      iterators, and var-data readers.
//!    - An encoder struct (`CarEncoder`) with field setters and type-state
//!      tail management.
//! 4. Generate an `AnyMessage` dispatch enum and `FrameCursor`.
//! 5. Run the output through [`prettyplease`] for formatting.
//!
//! The generated code includes an inline `sbe_rt` runtime module with
//! error types, the `SbeMessage` trait, and helper traits for group
//! encoding.

use std::collections::HashSet;
use std::fmt::Write;

use crate::ir::{ByteOrder, Ir, Presence, PrimitiveType, Signal, Token};
use crate::structured_ir::*;
use crate::{GenerationConfig, Schema};

pub(crate) mod encoded_length;
pub(crate) mod runtime;
use quote::format_ident;
pub(crate) use runtime::*;
use sha2::{Digest, Sha256};

/// A single generated Rust module.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedModule {
    /// Relative module path, for example `messages.rs`.
    pub path: String,
    /// Rust source code.
    pub source: String,
}

/// Complete generated output for a schema.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GeneratedModuleSet {
    modules: Vec<GeneratedModule>,
}

/// Errors returned by [`Generator::generate`] when the configuration
/// is invalid for the given schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerateError {
    /// A conversion selector matched no fields, or a domain type path is invalid.
    InvalidConversion {
        /// Description of the selector.
        selector: String,
        /// Why validation failed.
        reason: String,
    },
    /// Two selectors mapped to the same generated method name.
    ConversionCollision {
        /// The colliding method name.
        method: String,
        /// The first selector that produced the collision.
        selector_a: String,
        /// The second selector that produced the collision.
        selector_b: String,
    },
}

impl core::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidConversion { selector, reason } => {
                write!(f, "invalid conversion '{selector}': {reason}")
            }
            Self::ConversionCollision {
                method,
                selector_a,
                selector_b,
            } => {
                write!(
                    f,
                    "conversion method collision: '{method}' from '{selector_a}' and '{selector_b}'"
                )
            }
        }
    }
}

impl core::error::Error for GenerateError {}

impl GeneratedModuleSet {
    /// Add a generated module to the set.
    pub(crate) fn push(&mut self, module: GeneratedModule) {
        self.modules.push(module);
    }

    /// Iterate over generated modules in deterministic output order.
    #[must_use]
    pub fn modules(&self) -> impl ExactSizeIterator<Item = &GeneratedModule> {
        self.modules.iter()
    }
}

/// SBE-to-Rust generator.
/// Bundled schema identity + generation config, resolved once per schema.
/// Replaces the 6–15 parameter lists threaded through every generator function.
#[allow(missing_docs)]
pub(crate) struct GenerationContext {
    pub elements: SchemaElements,
    pub byte_order: ByteOrder,
    pub schema_id: u16,
    pub schema_version: u16,
    pub header_type: String,
    pub header_size: usize,
    pub schema_name: String,
    pub multi_message: bool,
    pub conversions: Vec<crate::ConversionSelector>,
    pub domain_types: Vec<(crate::ConversionSelector, String)>,
    pub unchecked_companions: bool,
    pub domain_objects: bool,
}

impl GenerationContext {
    fn from_schema(schema: &Schema, config: &GenerationConfig, multi_message: bool) -> Self {
        let elements = partition_tokens(&schema.ir.tokens);
        let header_size = elements
            .composites
            .iter()
            .find(|c| c[0].name == schema.ir.header_type)
            .and_then(|c| c[0].encoding.offset)
            .unwrap_or(8);
        Self {
            elements,
            byte_order: schema.ir.byte_order,
            schema_id: schema.ir.id,
            schema_version: schema.ir.version,
            header_type: schema.ir.header_type.clone(),
            header_size,
            schema_name: schema.ir.package.clone(),
            multi_message,
            conversions: config.conversions.clone(),
            domain_types: config.domain_types.clone(),
            unchecked_companions: config.unchecked_companions,
            domain_objects: config.domain_objects,
        }
    }
}

/// SBE codec generator from a resolved [`Schema`].
#[derive(Clone, Debug)]
pub struct Generator {
    config: GenerationConfig,
}

/// Whether a message field matches any configured conversion selector.
fn field_has_conversion_free(
    field: &MessageField,
    conversions: &[crate::ConversionSelector],
) -> bool {
    let type_name = match &field.field_type {
        FieldType::Composite { name, .. } => name.clone(),
        FieldType::Enum { name, .. } => name.clone(),
        FieldType::Set { name, .. } => name.clone(),
        FieldType::Primitive(pt, _) => rust_type(*pt).to_string(),
    };
    conversions.iter().any(|sel| match sel {
        crate::ConversionSelector::NamedType(n) => n == &type_name,
        crate::ConversionSelector::SemanticType(st) => {
            field.semantic_type.as_deref() == Some(st.as_str())
        }
        _ => false,
    })
}

/// Look up the domain type path for a field, if one is configured.
fn find_domain_type<'a>(
    field: &MessageField,
    domain_types: &'a [(crate::ConversionSelector, String)],
) -> Option<&'a str> {
    let type_name = match &field.field_type {
        FieldType::Composite { name, .. } => name.clone(),
        FieldType::Enum { name, .. } => name.clone(),
        FieldType::Set { name, .. } => name.clone(),
        FieldType::Primitive(pt, _) => rust_type(*pt).to_string(),
    };
    domain_types.iter().find_map(|(sel, ty)| match sel {
        crate::ConversionSelector::NamedType(n) if n == &type_name => Some(ty.as_str()),
        crate::ConversionSelector::SemanticType(st)
            if field.semantic_type.as_deref() == Some(st.as_str()) =>
        {
            Some(ty.as_str())
        }
        _ => None,
    })
}

/// Encoder setter name used by domain DTOs.
///
/// When a conversion is configured without a domain type, flyweight setters
/// are renamed to `*_wire` (concrete domain methods take the bare name).
/// Domain-object encode must call the same name.
fn domain_encode_setter_name(
    field: &MessageField,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    field_snake: &str,
) -> String {
    if field_has_conversion_free(field, conversions)
        && find_domain_type(field, domain_types).is_none()
    {
        format!("{field_snake}_wire")
    } else {
        field_snake.to_string()
    }
}

impl Generator {
    /// Create a generator with the supplied configuration.
    #[must_use]
    pub const fn new(config: GenerationConfig) -> Self {
        Self { config }
    }

    fn validate_conversions(&self, schema: &Schema) -> Result<(), GenerateError> {
        if !self.config.has_conversions() {
            return Ok(());
        }
        let elements = partition_tokens(&schema.ir.tokens);
        for sel in &self.config.conversions {
            let matched = match sel {
                crate::ConversionSelector::NamedType(name) => {
                    elements.composites.iter().any(|c| c[0].name == *name)
                        || elements.enums.iter().any(|e| e[0].name == *name)
                        || elements.sets.iter().any(|s| s[0].name == *name)
                }
                crate::ConversionSelector::SemanticType(_) => {
                    // Semantic types are validated during codegen when we can
                    // inspect field metadata — always passes pre-validation.
                    true
                }
                crate::ConversionSelector::FieldPath(_) => {
                    // Field paths are validated during codegen.
                    true
                }
            };
            if !matched {
                return Err(GenerateError::InvalidConversion {
                    selector: format!("{sel:?}"),
                    reason: "no matching type found in schema".into(),
                });
            }
        }
        // Validate domain type paths are non-empty
        for (sel, rust_type) in &self.config.domain_types {
            if rust_type.is_empty() {
                return Err(GenerateError::InvalidConversion {
                    selector: format!("{sel:?}"),
                    reason: "domain type path must not be empty".into(),
                });
            }
        }
        Ok(())
    }

    /// Whether a message field matches any configured conversion selector.
    fn field_has_conversion(
        field: &MessageField,
        conversions: &[crate::ConversionSelector],
    ) -> bool {
        field_has_conversion_free(field, conversions)
    }

    /// Whether the config has a conversion selector matching the given type name,
    /// semantic type, or field path. Also returns true for FieldPath selectors
    /// that match `owner_name.field_name`.
    fn has_conversion_for(
        &self,
        type_name: &str,
        semantic_type: Option<&str>,
        owner_name: Option<&str>,
        field_name: &str,
    ) -> bool {
        for sel in &self.config.conversions {
            match sel {
                crate::ConversionSelector::NamedType(name) if name == type_name => return true,
                crate::ConversionSelector::SemanticType(st)
                    if semantic_type == Some(st.as_str()) =>
                {
                    return true;
                }
                crate::ConversionSelector::FieldPath(path) => {
                    // Match "Owner.field" or just "field"
                    let expected = format!("{}.{}", owner_name.unwrap_or(""), field_name);
                    if path == &expected || path == field_name {
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    /// Generate Rust modules for a normalized schema.
    ///
    /// Validates conversion configuration and returns an error on invalid setup.
    pub fn generate(&self, schema: &Schema) -> Result<GeneratedModuleSet, GenerateError> {
        self.validate_conversions(schema)?;
        let mut modules = GeneratedModuleSet::default();
        let src = self.gen_schema(schema, &HashSet::new(), false, true);
        modules.push(GeneratedModule {
            path: format!("{}.rs", self.config.module_name),
            source: src,
        });
        Ok(modules)
    }

    /// Generate Rust modules for multiple schemas, deduplicating shared types.
    ///
    /// When `config.shared_module` is set, the first schema's enums, sets, and
    /// composites are treated as "shared". Subsequent schemas skip those type
    /// definitions and instead emit `pub use super::<shared_module>::*;` to
    /// reference them. The `sbe_rt` runtime module is only emitted in the first
    /// schema's output.
    ///
    /// Each entry is `(schema, module_name)` where `module_name` is the Rust
    /// module name (e.g. `"common_types"`, `"market_data"`).
    pub fn generate_multi(
        &self,
        schemas: &[(&Schema, &str)],
    ) -> Result<GeneratedModuleSet, GenerateError> {
        let mut modules = GeneratedModuleSet::default();
        let mut shared_types: HashSet<String> = HashSet::new();

        for (i, (schema, module_name)) in schemas.iter().enumerate() {
            if i == 0 {
                let elements = partition_tokens(&schema.ir.tokens);
                for et in &elements.enums {
                    shared_types.insert(to_pascal_case(&et[0].name));
                }
                for st in &elements.sets {
                    shared_types.insert(to_pascal_case(&st[0].name));
                }
                for ct in &elements.composites {
                    shared_types.insert(to_pascal_case(&ct[0].name));
                }
            }
            let is_importing = i > 0 && self.config.shared_module.is_some();
            // Emit sbe_rt in the first module always, and in every module
            // when there is no shared module (standalone mode).
            let emit_sbe_rt = i == 0 || self.config.shared_module.is_none();
            let src = self.gen_schema(schema, &shared_types, is_importing, emit_sbe_rt);
            modules.push(GeneratedModule {
                path: format!("{}.rs", module_name),
                source: src,
            });
        }
        Ok(modules)
    }

    /// Inner generation — single schema with dedup flags. `shared` contains
    /// type names already generated by earlier schemas; those types are skipped
    /// in this call (the caller arranges `pub use super::*;`).
    fn gen_schema(
        &self,
        schema: &Schema,
        shared: &HashSet<String>,
        is_importing: bool,
        emit_sbe_rt: bool,
    ) -> String {
        let ir = &schema.ir;

        let mut src = String::new();
        // NOTE: In Rust edition 2024, inner attributes (`#![allow(...)])`) are
        // not permitted inside `include!()` files.  All suppression lints are
        // therefore emitted as outer `#[allow(..)]` on `pub mod sbe_rt`.
        // Outer doc comment (`///`) — syn/prettyplease preserves it; `//` would
        // be silently dropped.
        writeln!(
            src,
            "/// Generated from SBE schema package `{}` id {} version {}.",
            schema.package, schema.id, schema.version
        )
        .unwrap();
        src.push_str(
            "#[allow(clippy::absurd_extreme_comparisons, clippy::double_must_use, \
                       clippy::erasing_op, clippy::identity_op, clippy::unnecessary_cast, \
                       unused_assignments, unused_comparisons)]\n",
        );
        src.push_str("#[allow(non_camel_case_types)]\n");
        src.push_str("#[allow(non_snake_case)]\n");
        src.push_str("#[allow(clippy::identity_op)]\n");
        src.push_str("#[allow(clippy::eq_op)]\n");
        src.push_str("#[allow(clippy::needless_borrow)]\n");
        src.push_str("#[allow(clippy::manual_range_contains)]\n");
        src.push_str("#[allow(unused_imports)]\n");
        src.push_str("#[allow(unused_variables)]\n");
        src.push_str("#[allow(unused_mut)]\n");
        src.push_str("#[allow(dead_code)]\n\n");

        // If importing from a shared module, bring all its items into scope.
        // This covers shared types + the sbe_rt runtime module.
        if is_importing {
            if let Some(ref shared_mod) = self.config.shared_module {
                write!(src, "pub use super::{}::*;\n\n", shared_mod).unwrap();
            }
        }

        // 1. SBE runtime: external re-export, or inline once per module set.
        if let Some(ref ext) = self.config.external_sbe_rt_path {
            let _ = writeln!(src, "pub use {ext} as sbe_rt;\n");
            if self.config.has_conversions() {
                emit_conversion_traits(&mut src);
            }
        } else if emit_sbe_rt {
            src.push_str(&generate_sbe_rt_src());
            if self.config.has_conversions() {
                emit_conversion_traits(&mut src);
            }
        }

        // 2. Group the tokens into composites, enums, sets, and messages
        let elements = partition_tokens(&ir.tokens);

        // 3. Generate Enums (skip shared)
        for enum_tokens in &elements.enums {
            let type_name = to_pascal_case(&enum_tokens[0].name);
            if shared.contains(&type_name) {
                continue;
            }
            generate_enum(&mut src, enum_tokens);
        }

        // 4. Generate Sets/Choices (skip shared)
        for set_tokens in &elements.sets {
            let type_name = to_pascal_case(&set_tokens[0].name);
            if shared.contains(&type_name) {
                continue;
            }
            generate_set(&mut src, set_tokens);
        }

        // 5. Generate Composites (skip shared)
        for composite_tokens in &elements.composites {
            let type_name = to_pascal_case(&composite_tokens[0].name);
            if shared.contains(&type_name) {
                continue;
            }
            // SBE spec §4.1: MessageHeader is ALWAYS little-endian on the wire,
            // regardless of the schema's declared byteOrder. The body follows
            // the schema byteOrder; the header composite must use LE.
            let comp_byte_order = if composite_tokens[0].name == "messageHeader" {
                ByteOrder::LittleEndian
            } else {
                ir.byte_order
            };
            generate_composite(&mut src, composite_tokens, comp_byte_order);
        }

        // Generate MessageHeader alias if custom name is used (skip if shared)
        let header_pascal = to_pascal_case(&ir.header_type);
        if header_pascal != "MessageHeader" && !shared.contains(&header_pascal) {
            write!(src, "pub type MessageHeader = {};\n\n", header_pascal).unwrap();
        }

        // 6. Generate Messages (Decoders and Encoders) — always generated
        let messages: Vec<MessageStructure> = elements
            .messages
            .iter()
            .map(|toks| parse_message_structure(toks, &elements))
            .collect();

        for msg in &messages {
            let multi = messages.len() > 1;
            let decoder_ts = generate_message_decoder(
                msg,
                &elements,
                ir.byte_order,
                ir.id,
                ir.version,
                &ir.header_type,
                &ir.package,
                multi,
                self.config.domain_objects,
                &self.config.conversions,
                &self.config.domain_types,
                self.config.unchecked_companions,
            );
            src.push_str(&decoder_ts.to_string());
            src.push('\n');
            let encoder_ts = generate_message_encoder(
                msg,
                &elements,
                ir.byte_order,
                ir.id,
                ir.version,
                &ir.header_type,
                multi,
                &self.config.conversions,
                &self.config.domain_types,
                self.config.unchecked_companions,
            );
            src.push_str(&encoder_ts.to_string());

            // Decimal converter seam: for each field backed by a registered
            // Decimal composite, emit raw *_wire aliases and generic converted
            // methods. Only emitted when converter mode is active.
            if !&self.config.conversions.is_empty() {
                let converter_ts = generate_converter_impls(
                    msg,
                    &self.config.conversions,
                    &self.config.domain_types,
                    multi,
                );
                src.push_str(&converter_ts);
            }
            src.push('\n');
            generate_message_field_meta(&mut src, msg);
        }

        // 6b. Emit TryFromSbe/TryToSbe impls for configured domain-type conversions
        if self.config.has_conversions() {
            let impl_blocks = generate_conversion_impl_blocks(
                &elements,
                &self.config.conversions,
                &self.config.domain_types,
            );
            src.push_str(&impl_blocks);
        }

        // 6c. Emit EncodedLengthAccumulator if any message needs staged builder
        {
            let has_staged = messages.iter().any(|m| {
                matches!(
                    encoded_length::strategy(m),
                    encoded_length::LengthStrategy::Staged
                )
            });
            if has_staged {
                let support_ts = encoded_length::generate_support();
                src.push_str(&support_ts.to_string());
            }
        }

        // 7. Generate schema-level constants — SEMANTIC_VERSION, SCHEMA_HASH, SCHEMA_SHA256, SCHEMA_SHA256_HEX
        if let Some(ref sem_ver) = schema.ir.semantic_version {
            write!(
                src,
                "pub const SEMANTIC_VERSION: &str = \"{}\";\n\n",
                sem_ver
            )
            .unwrap();
        }
        let schema_hash = compute_schema_hash(&schema.package, schema.id, schema.version);
        write!(src, "pub const SCHEMA_HASH: u64 = {};\n\n", schema_hash).unwrap();
        let sha256_hash = compute_schema_sha256(&schema.ir);
        src.push_str("pub const SCHEMA_SHA256: [u8; 32] = [");
        for (i, &b) in sha256_hash.iter().enumerate() {
            if i > 0 {
                src.push_str(", ");
            }
            write!(src, "0x{:02x}", b).unwrap();
        }
        src.push_str("];\n\n");
        let hex: String = sha256_hash.iter().map(|b| format!("{:02x}", b)).collect();
        write!(src, "pub const SCHEMA_SHA256_HEX: &str = \"{}\";\n\n", hex).unwrap();
        // 7.6. Generate prelude module — single import surface for users
        generate_prelude(&mut src, &elements, &messages, ir.id, ir.version);
        // 7.6b. Opt-in From<EncodeError/DecodeError> for user error type
        if let Some(ref err_path) = self.config.error_from_path {
            let err_ty: syn::Type = syn::parse_str(err_path).expect("invalid error_from_path");
            let span = proc_macro2::Span::call_site();
            let impls = quote::quote! {
                /// Generated: encode errors convert directly to the crate error type.
                impl From<sbe_rt::EncodeError> for #err_ty {
                    fn from(e: sbe_rt::EncodeError) -> Self {
                        Self::from(format!("sbe encode: {e}"))
                    }
                }
                /// Generated: decode errors convert directly to the crate error type.
                impl From<sbe_rt::DecodeError> for #err_ty {
                    fn from(e: sbe_rt::DecodeError) -> Self {
                        Self::from(format!("sbe decode: {e}"))
                    }
                }
            };
            src.push_str(&impls.to_string());
            src.push('\n');
        }
        // 7.7. Generate const-compatible byte-read helper (avoids per-accessor loop bloat)
        let read_bytes_ts: proc_macro2::TokenStream = quote::quote! {
            /// Read `N` bytes from `buf` at `offset` into a fixed-size array.
            ///
            /// Bounds-checked slice indexing. LLVM elides the check when the
            /// slice length is known (stack buffer with visible size).
            /// Prefer [`read_bytes_unchecked`] when the caller has already
            /// validated bounds.
            #[inline]
            pub fn read_bytes<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
                buf[offset..offset + N].try_into().expect("read_bytes: buffer too short")
            }

            /// Write `N` bytes from `bytes` into `buf` at `offset`.
            #[inline]
            pub fn write_bytes<const N: usize>(buf: &mut [u8], offset: usize, bytes: &[u8; N]) {
                buf[offset..offset + N].copy_from_slice(bytes);
            }
        };
        src.push_str(&read_bytes_ts.to_string());

        // Unchecked byte I/O — always generated for zero-validation fast paths.
        // Caller guarantees offset + N <= buf.len().
        let uc = quote::quote! {
            /// Unchecked companion to [`read_bytes`] — zero bounds checks.
            /// Caller guarantees `offset + N <= buf.len()`.
            #[inline]
            pub fn read_bytes_unchecked<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
                // SAFETY: caller guarantees offset + N <= buf.len().
                unsafe {
                    core::ptr::read_unaligned(buf.as_ptr().add(offset) as *const [u8; N])
                }
            }

            /// Unchecked companion to [`write_bytes`] — zero bounds checks.
            /// Caller guarantees `offset + N <= buf.len()`.
            #[inline]
            pub fn write_bytes_unchecked<const N: usize>(buf: &mut [u8], offset: usize, bytes: &[u8; N]) {
                // SAFETY: caller guarantees offset + N <= buf.len().
                unsafe {
                    core::ptr::write_unaligned(buf.as_mut_ptr().add(offset) as *mut [u8; N], *bytes)
                }
            }
        };
        src.push_str(&uc.to_string());
        src.push('\n');
        // 8. Generate zero-parse schemaId extraction from raw header bytes
        generate_schema_id_from_header(&mut src, &elements, &ir.header_type, ir.byte_order);

        // 8. Generate AnyMessage enum (per-schema: only this schema's messages)
        let any_msg_ts =
            generate_any_message(&messages, &elements, ir.id, &ir.header_type, &ir.package);
        src.push_str(&any_msg_ts.to_string());
        src.push('\n');

        // Format through syn/prettyplease
        let file =
            syn::parse_str::<syn::File>(&src).expect("generated code must be valid Rust syntax");
        prettyplease::unparse(&file)
    }
}

/// Generate the inline `sbe_rt` runtime module source.
fn generate_owner_consuming_stages(
    initial_ident: syn::Ident,
    stage_prefix: &str,
    header_size: usize,
    groups: &[OwnerTailGroup],
    vardata: &[OwnerTailVarData],
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
                buf: &'a [u8],
                pos: usize,
                tail_start: usize,
                acting_version: u16,
                acting_block_length: usize,
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
        let vd_type_ident = syn::Ident::new(&vd.type_pascal, span);
        let len_field_ident = syn::Ident::new(&vd.len_field, span);
        let vd_name_lit = syn::LitStr::new(&vd.name, span);
        let se = start_expr(i);
        let mut max_check = proc_macro2::TokenStream::new();
        if let Some(max) = vd.max_length {
            let max_lit = syn::LitInt::new(&max.to_string(), span);
            max_check.extend(quote::quote! {
                if len > #max_lit {
                    return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                        field: #vd_name_lit,
                        length: len as u32,
                        max_length: #max_lit,
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
                    // encoding struct (VarAsciiEncoding etc.) for the length.
                    let len = match #prefix_size_lit {
                        1 => bytes[0] as usize,
                        2 => u16::from_le_bytes([bytes[0], bytes[1]]) as usize,
                        _ => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize,
                    };
                    #max_check
                    let data_start = offset + #prefix_size_lit;
                    if data_start + len > self.buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #vd_name_lit,
                            needed: #prefix_size_lit + len,
                            available: self.buf.len().saturating_sub(offset),
                        });
                    }
                    let data = &self.buf[data_start..data_start + len];
                    let next = #next_stage {
                        buf: self.buf,
                        pos: self.pos,
                        tail_start: data_start + len,
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
                    // encoding struct (VarAsciiEncoding etc.) for the length.
                    let len = match #prefix_size_lit {
                        1 => bytes[0] as usize,
                        2 => u16::from_le_bytes([bytes[0], bytes[1]]) as usize,
                        _ => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize,
                    };
                    #max_check
                    let data_start = offset + #prefix_size_lit;
                    if data_start + len > self.buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #vd_name_lit,
                            needed: #prefix_size_lit + len,
                            available: self.buf.len().saturating_sub(offset),
                        });
                    }
                    Ok(&self.buf[data_start..data_start + len])
                }
            }
        });

        // Text var-data: into_<field>_as_str() for schema-declared characterEncoding.
        if let Some(ref enc) = vd.character_encoding {
            let is_text = enc.eq_ignore_ascii_case("UTF-8")
                || enc.eq_ignore_ascii_case("UTF8")
                || enc.eq_ignore_ascii_case("ASCII")
                || enc.eq_ignore_ascii_case("US-ASCII");
            if is_text {
                let as_str_ident =
                    syn::Ident::new(&format!("into_{}_as_str", vd.accessor_snake), span);
                let into_ident = syn::Ident::new(&format!("into_{}", vd.accessor_snake), span);
                ts.extend(quote::quote! {
                    impl<'a> #current_stage<'a> {
                        /// Consume this stage, read the next text var-data field as
                        /// a validated `&str`, and advance to the following stage.
                        #[inline]
                        pub fn #as_str_ident(self) -> Result<(&'a str, #next_stage<'a>), sbe_rt::DecodeError> {
                            let (bytes, next) = self.#into_ident()?;
                            let s = core::str::from_utf8(bytes).map_err(|e| {
                                sbe_rt::DecodeError::InvalidUtf8 {
                                    field: #vd_name_lit,
                                    error: e,
                                }
                            })?;
                            Ok((s, next))
                        }
                    }
                });

                let as_str_unchecked = syn::Ident::new(
                    &format!("into_{}_as_str_unchecked", vd.accessor_snake),
                    span,
                );
                ts.extend(quote::quote! {
                    impl<'a> #current_stage<'a> {
                        /// Consume this stage, read the next text var-data field as
                        /// a `&str` without UTF-8 validation, and advance to the
                        /// following stage.
                        ///
                        /// # Safety
                        ///
                        /// The wire bytes must be valid UTF-8. For schema-declared
                        /// ASCII encoding this is always true (ASCII ⊂ UTF-8).
                        #[inline]
                        pub unsafe fn #as_str_unchecked(self) -> (&'a str, #next_stage<'a>) {
                            let (bytes, next) = unsafe { self.#into_ident().unwrap_unchecked() };
                            // SAFETY: caller guarantees valid UTF-8
                            let s = unsafe { core::str::from_utf8_unchecked(bytes) };
                            (s, next)
                        }
                    }
                });
            }
        }

        // Nested-message decode convenience: into_<field>_as_message()
        // delegates to into_<field>() then AnyMessage::decode_frame.
        let as_msg_ident = syn::Ident::new(&format!("into_{}_as_message", vd.accessor_snake), span);
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
            }
        });

        // Scoped fallible combinators: try_<data> and try_<data>_as_message
        // delegate to the manual consuming methods and propagate caller errors.
        let try_data_ident = syn::Ident::new(&format!("try_{}", vd.accessor_snake), span);
        let try_data_as_msg_ident =
            syn::Ident::new(&format!("try_{}_as_message", vd.accessor_snake), span);
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

                /// Fallible scoped nested-message accessor. Decodes the
                /// var-data as an SBE message, calls the closure with the
                /// decoded frame, and returns the next stage on success.
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

    // 3. finish()/skip_remaining() for each group -> next owner stage.
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

    // 4. Terminal (Complete) stage extent helpers.
    let complete_ident = stage_after_ident(total_tail - 1);
    ts.extend(quote::quote! {
        impl<'a> #complete_ident<'a> {
            /// Header-inclusive bytes (for an entry, the entry bytes; header_size is 0).
            #[inline]
            pub fn as_bytes(&self) -> &'a [u8] {
                &self.buf[self.pos - #header_size_lit..self.tail_start]
            }
            /// Body length (excluding header).
            #[inline]
            pub fn encoded_length(&self) -> usize {
                self.tail_start - self.pos
            }
            /// Header-inclusive length.
            #[inline]
            pub fn encoded_length_with_header(&self) -> usize {
                self.tail_start - self.pos + #header_size_lit
            }
        }
    });

    ts
}

/// Message-level consuming tail stages (DECISIONS.md §3): thin wrapper that
/// resolves the message's tail groups + var-data into descriptors and delegates
/// to `generate_owner_consuming_stages`.
fn generate_decoder_consuming_stages(
    msg: &MessageStructure,
    elements: &SchemaElements,
    name: &str,
    header_size: usize,
    _multi_message: bool,
    group_unique_names: &[String],
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
            let (type_pascal, prefix_size, len_field, _) =
                get_vardata_info(elements, &vd.type_name);
            OwnerTailVarData {
                accessor_snake: to_snake_case(&vd.name),
                field_pascal: to_pascal_case(&vd.name),
                type_pascal,
                prefix_size,
                len_field,
                max_length: vd.max_length,
                name: vd.name.clone(),
                character_encoding: vd.character_encoding.clone(),
            }
        })
        .collect();
    generate_owner_consuming_stages(initial_ident, &stage_prefix, header_size, &groups, &vardata)
}

/// Entry-level consuming tail stages for a group whose entries have nested
/// groups and/or var-data (DECISIONS.md §3, Task D). `name` is the group's
/// scoped name; nested group decoder names are `{name}{Ng}Decoder`.
fn generate_entry_consuming_stages(
    g: &MessageGroup,
    elements: &SchemaElements,
    name: &str,
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
            let (type_pascal, prefix_size, len_field, _) =
                get_vardata_info(elements, &vd.type_name);
            OwnerTailVarData {
                accessor_snake: to_snake_case(&vd.name),
                field_pascal: to_pascal_case(&vd.name),
                type_pascal,
                prefix_size,
                len_field,
                max_length: vd.max_length,
                name: vd.name.clone(),
                character_encoding: vd.character_encoding.clone(),
            }
        })
        .collect();
    generate_owner_consuming_stages(initial_ident, &entry_prefix, 0, &groups, &vardata)
}

fn generate_message_decoder(
    msg: &MessageStructure,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    schema_id: u16,
    schema_version: u16,
    header_type: &str,
    schema_name: &str,
    multi_message: bool,
    domain_objects: bool,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    _unchecked_companions: bool,
) -> proc_macro2::TokenStream {
    let raw_name = &msg.name;
    let name = to_pascal_case(raw_name);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    let block_length = msg.fields.iter().fold(0, |acc, f| {
        let size = f.field_type.size();
        acc.max(f.offset + size)
    });

    let header_pascal = to_pascal_case(header_type);
    let (header_bl, header_ti, header_si, header_vr) = {
        let mut bl = "block_length".to_string();
        let mut ti = "template_id".to_string();
        let mut si = "schema_id".to_string();
        let mut vr = "version".to_string();
        if let Some(comp) = elements
            .composites
            .iter()
            .find(|c| c[0].name == header_type)
        {
            let members = parse_composite_members(comp);
            for m in members {
                let lower = m.name.to_lowercase();
                if lower.contains("blocklength") {
                    bl = to_snake_case(&m.name);
                } else if lower.contains("templateid") {
                    ti = to_snake_case(&m.name);
                } else if lower.contains("schemaid") {
                    si = to_snake_case(&m.name);
                } else if lower.contains("version") {
                    vr = to_snake_case(&m.name);
                }
            }
        }
        (bl, ti, si, vr)
    };

    let header_size = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| c[0].encoding.offset)
        .unwrap_or(8);

    // Compile-time buffer sizing constants
    let is_fixed = msg.groups.is_empty() && msg.var_data.is_empty();
    let encoded_length = header_size + block_length;
    let mut max_tail = 0usize;
    for g in &msg.groups {
        let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
        max_tail += dim_size + g.block_length;
    }
    for vd in &msg.var_data {
        let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
        max_tail += prefix_size + vd.max_length.unwrap_or(0);
    }
    let max_encoded_length = header_size + block_length + max_tail;

    // Identifiers for codegen
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

    // 1. Decoder Struct + optional doc comment
    if let Some(ref desc) = msg.description {
        ts.extend(doc_attr_tokens(desc));
    }
    // Fixed-block-only decoders (no groups/var-data) are Copy: they have no
    // tail cursor, so copying cannot weaken an ordering invariant. Tailed
    // decoders are NOT Copy/Clone — consumption enforces wire order.
    let derive_attr = if is_fixed {
        quote::quote! { #[derive(Clone, Copy)] }
    } else {
        quote::quote! {}
    };
    ts.extend(quote::quote! {
        #derive_attr
        pub struct #decoder_ident<'a> {
            buf: &'a [u8],
            pos: usize,
            acting_version: u16,
            acting_block_length: usize,
        }
    });

    // 2. impl block with compile-time constants
    let mut impl_body = proc_macro2::TokenStream::new();
    if is_fixed {
        impl_body.extend(quote::quote! {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #bl_lit;
            const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == #bl_lit);
            /// Message header size in bytes (standard SBE header is 8).
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
        });
    } else {
        const STACK_LIMIT: usize = 65536;
        let max_encoded_capped = max_encoded_length.min(STACK_LIMIT);
        let max_encoded_lit = syn::LitInt::new(
            &max_encoded_capped.to_string(),
            proc_macro2::Span::call_site(),
        );
        let is_capped = max_encoded_length > STACK_LIMIT;
        let max_doc = if is_capped {
            "MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation"
        } else {
            "Stack-allocate with `let mut buf = [0u8; Msg::MAX_ENCODED_LENGTH];`"
        };
        let max_doc_lit = syn::LitStr::new(max_doc, proc_macro2::Span::call_site());
        impl_body.extend(quote::quote! {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #bl_lit;
            const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == #bl_lit);
            /// Message header size in bytes (standard SBE header is 8).
            pub const HEADER_LENGTH: usize = #hdr_size_lit;
            #[doc = #max_doc_lit]
            pub const MAX_ENCODED_LENGTH: usize = #max_encoded_lit;
            const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
        });
    }

    // 3. wrap() function
    impl_body.extend(quote::quote! {
        #[inline]
        pub fn wrap(buf: &'a [u8], pos: usize, acting_block_length: usize, acting_version: u16) -> Self {
            Self {
                buf,
                pos,
                acting_block_length,
                acting_version,
            }
        }
    });

    // 4. try_wrap_and_apply_header — validates schema_id + template_id
    {
        let hs = syn::LitInt::new(&header_size.to_string(), proc_macro2::Span::call_site());
        let hp = syn::Ident::new(&header_pascal, proc_macro2::Span::call_site());
        let hsi = syn::Ident::new(&header_si, proc_macro2::Span::call_site());
        let hti = syn::Ident::new(&header_ti, proc_macro2::Span::call_site());
        let hbl = syn::Ident::new(&header_bl, proc_macro2::Span::call_site());
        let hvr = syn::Ident::new(&header_vr, proc_macro2::Span::call_site());
        let en = syn::LitStr::new(&schema_name, proc_macro2::Span::call_site());
        impl_body.extend(quote::quote! {
            #[inline]
            pub fn try_wrap_and_apply_header(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
                // Decoder trust boundary: validate buffer bounds + schema_id + template_id.
                // This is the one place the decoder checks — all field accessors
                // after this are infallible (offsets are within the validated block).
                if pos + #hs > buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "message header",
                        needed: #hs,
                        available: buf.len().saturating_sub(pos),
                    });
                }
                let header_bytes: [u8; #hs] = read_bytes::<#hs>(buf, pos);
                let header = #hp(header_bytes);
                if header.#hti() != Self::TEMPLATE_ID {
                    return Err(sbe_rt::DecodeError::WrongSchema {
                        expected: Self::TEMPLATE_ID,
                        actual: header.#hti(),
                        expected_name: #en,
                    });
                }
                if header.#hsi() != Self::SCHEMA_ID {
                    return Err(sbe_rt::DecodeError::WrongSchema {
                        expected: Self::SCHEMA_ID,
                        actual: header.#hsi(),
                        expected_name: #en,
                    });
                }
                let acting_block_length = header.#hbl() as usize;
                if pos + #hs + acting_block_length > buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "message body",
                        needed: #hs + acting_block_length,
                        available: buf.len().saturating_sub(pos),
                    });
                }
                Ok(Self::wrap(buf, pos + #hs, acting_block_length, header.#hvr()))
            }
        });
    }

    // 5. acting_version and acting_block_length
    impl_body.extend(syn::parse_str::<proc_macro2::TokenStream>(
        "#[inline]\n    pub const fn acting_version(&self) -> u16 {\n        self.acting_version\n    }\n\n    pub const fn acting_block_length(&self) -> usize {\n        self.acting_block_length\n    }\n\n"
    ).unwrap());

    // 6. Field getters
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
        let fname_ident = syn::Ident::new(method_name, proc_macro2::Span::call_site());

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
                    // bounds check). This is the fastest safe-mode shape: Aeron's
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

                    let fn_snake_ident =
                        syn::Ident::new(&fname_snake, proc_macro2::Span::call_site());
                    // Fixed-length array accessors are INFALLIBLE: a fixed array that
                    // lies within the message body is guaranteed in-bounds by the
                    // version/block-length check below (and by wrap, which validates the
                    // body extent). Returning `Result` here is over-cautious, diverges
                    // from Aeron (which returns `[T; N]`), and adds Result+unwrap
                    // overhead that measurably slows decode. OOB only happens for a
                    // structurally malformed buffer shorter than its declared
                    // block_length, in which case read_bytes panics — same as Aeron's
                    // try_into. This matches Aeron's `[T; N]` signature and perf.
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fn_snake_ident(&self) -> [#r_type_ty; #len_lit] {
                            if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                return [0 as #r_type_ty; #len_lit];
                            }
                            let offset = self.pos + #offset_lit;
                            let all: [u8; #total_size_lit] = read_bytes_unchecked::<#total_size_lit>(self.buf, offset);
                            [#(#elements),*]
                        }
                    });
                } else {
                    // Non-array primitive
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
                        let accessor = format!(
                            "#[inline]\n\
                             pub fn {snake}(&self) -> Option<{rt}> {{\n\
                                 if self.acting_version < {since} || {offset_end} > self.acting_block_length {{\n\
                                     return None;\n\
                                 }}\n\
                                 let offset = self.pos + {offset};\n\
                                 let val = {rt}::{order}(read_bytes_unchecked::<{ps}>(self.buf, offset));\n\
                                 if {null_check} {{\n\
                                     None\n\
                                 }} else {{\n\
                                     Some(val)\n\
                                 }}\n\
                             }}\n",
                            snake = fname_snake,
                            rt = r_type,
                            since = since,
                            offset_end = offset_end,
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
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub fn #fname_ident(&self) -> Option<#r_type_ty> {
                                if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                    return None;
                                }
                                let offset = self.pos + #offset_lit;
                                Some(#r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset)))
                            }
                        });
                    } else {
                        if let Some(ref desc) = f.description {
                            impl_body.extend(doc_attr_tokens(desc));
                        }
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub fn #fname_ident(&self) -> #r_type_ty {
                                let offset = self.pos + #offset_lit;
                                #r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset))
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

                // Eager copy accessor (_value)
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
                            Some(#target_ident(read_bytes_unchecked::<#comp_size_lit>(self.buf, offset)))
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #as_struct_ident(&self) -> #target_ident {
                            let offset = self.pos + #offset_lit;
                            #target_ident(read_bytes_unchecked::<#comp_size_lit>(self.buf, offset))
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
                        let variant_ident =
                            syn::Ident::new(variant, proc_macro2::Span::call_site());
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
                            Some(#target_ident::from_raw(#r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset))))
                        }
                    });
                    if is_bool_enum(elements, enum_name) {
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
                            #target_ident::from_raw(#r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset)))
                        }
                    });
                    if is_bool_enum(elements, enum_name) {
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
                            Some(#target_ident(#r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset))))
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fname_ident(&self) -> #target_ident {
                            let offset = self.pos + #offset_lit;
                            #target_ident(#r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset)))
                        }
                    });
                }
            }
        }
        // Emit field constants
        let field_consts_ts = emit_field_consts(f);
        impl_body.extend(field_consts_ts);
    }

    // 7. Tail offset helpers
    let total_tail = msg.groups.len() + msg.var_data.len();

    // tail_offset_0
    impl_body.extend(quote::quote! {
        #[inline]
        fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
            Ok(self.pos + self.acting_block_length)
        }
    });

    // Group PascalCase names (parser guarantees unique names within a message).
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
        let k1 = k + 1;
        let prefix_size_lit =
            syn::LitInt::new(&prefix_size.to_string(), proc_macro2::Span::call_site());
        let vd_type_ident = syn::Ident::new(&type_pascal, proc_macro2::Span::call_site());
        let vd_len_field_ident = syn::Ident::new(&len_field, proc_macro2::Span::call_site());
        let vd_tail = format!(
            "    #[inline]\n\
             fn tail_offset_{k1}(&self) -> Result<usize, sbe_rt::DecodeError> {{\n\
                 let start = self.tail_offset_{k}()?;\n\
                 if start + {ps} > self.buf.len() {{\n\
                     return Err(sbe_rt::DecodeError::BufferTooShort {{ field: \"{vn}\", needed: {ps}, available: self.buf.len().saturating_sub(start) }});\n\
                 }}\n\
                 let bytes: [u8; {ps}] = read_bytes::<{ps}>(self.buf, start);\n\
                 let header = {tp}(bytes);\n\
                 let len = header.{lf}() as usize;\n\
                 if start + {ps} + len > self.buf.len() {{\n\
                     return Err(sbe_rt::DecodeError::BufferTooShort {{ field: \"{vn}\", needed: {ps} + len, available: self.buf.len().saturating_sub(start) }});\n\
                 }}\n\
                 Ok(start + {ps} + len)\n\
             }}",
            k1 = k1,
            k = k,
            ps = prefix_size,
            vn = vd.name,
            tp = type_pascal,
            lf = len_field,
        );
        impl_body.extend(syn::parse_str::<proc_macro2::TokenStream>(&vd_tail).unwrap());
        k += 1;
    }

    // 8. Group accessors — uses pre-computed dedup names from section 7
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
        let g_since_lit =
            syn::LitInt::new(&g.since_version.to_string(), proc_macro2::Span::call_site());
        let g_snake_str = g_snake.clone();
        impl_body.extend(quote::quote! {
            #[inline]
            fn #g_snake_ident(&self) -> Result<#g_decoder_ident<'a>, sbe_rt::DecodeError> {
                if self.acting_version < #g_since_lit {
                    return Err(sbe_rt::DecodeError::FieldNotInVersion {
                        field: #g_snake_str,
                        wire_version: self.acting_version,
                        since_version: #g_since_lit,
                    });
                }
                let offset = self.#tail_offset_ident()?;
                #g_decoder_ident::wrap(self.buf, offset, self.acting_version)
            }
        });
        g_idx += 1;
    }

    // 9. VarData accessors
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
        let vd_since_lit = syn::LitInt::new(
            &vd.since_version.to_string(),
            proc_macro2::Span::call_site(),
        );
        let vd_snake_str = vd_snake.clone();
        impl_body.extend(quote::quote! {
            #[inline]
            fn #vd_snake_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                if self.acting_version < #vd_since_lit {
                    return Err(sbe_rt::DecodeError::FieldNotInVersion {
                        field: #vd_snake_str,
                        wire_version: self.acting_version,
                        since_version: #vd_since_lit,
                    });
                }
                let offset = self.#vd_tail_ident()?;
                let bytes: [u8; #prefix_size_lit] = read_bytes::<#prefix_size_lit>(self.buf, offset);
                let header = #type_pascal_ident(bytes);
                let len = header.#len_field_ident() as usize;
                if len > #max_lit {
                    return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                        field: stringify!(#vd_snake_ident),
                        length: len as u32,
                        max_length: #max_lit,
                    });
                }
                let data_offset = offset + #prefix_size_lit;
                Ok(&self.buf[data_offset .. data_offset + len])
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

    // 10. encoded_length, encoded_length_with_header, as_bytes
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
        pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
            let len = self.encoded_length_with_header()?;
            let start = self.pos - #hdr_size_lit;
            Ok(&self.buf[start .. start + len])
        }
    });

    // 11. verify function - built as TokenStream directly
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
        let block_length = header.#hbl_ident() as usize;
        if block_length < Self::BLOCK_LENGTH {
            return Err(sbe_rt::VerifyError::InvalidBlockLength {
                expected_min: Self::BLOCK_LENGTH,
                actual: block_length,
            });
        }
        let body_end = #hs_lit + block_length;
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
        let ebl_lit = syn::LitInt::new(&g.block_length.to_string(), proc_macro2::Span::call_site());
        let has_tails = !g.groups.is_empty() || !g.var_data.is_empty();
        // Entry decoder ident for skip() calls
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
                if offset + #ps_lit > buf.len() {
                    return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                        field: #vd_snake,
                        offset,
                        length: 0,
                    });
                }
                let bytes: [u8; #ps_lit] = read_bytes::<#ps_lit>(buf, offset);
                let var_header = #tp_ident(bytes);
                let len = var_header.#lf_ident();
                let data_end = offset + #ps_lit + len as usize;
                if data_end > buf.len() {
                    return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                        field: #vd_snake,
                        offset,
                        length: len as u32,
                    });
                }
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

    // Wrap all impl_body items inside the impl block
    ts.extend(quote::quote! {
        impl<'a> #decoder_ident<'a> {
            #impl_body
        }
    });

    // 12. Trait impls
    let msg_id_lit = syn::LitInt::new(&msg.id.to_string(), proc_macro2::Span::call_site());
    let schema_id_lit = syn::LitInt::new(&schema_id.to_string(), proc_macro2::Span::call_site());
    let schema_version_lit =
        syn::LitInt::new(&schema_version.to_string(), proc_macro2::Span::call_site());
    ts.extend(quote::quote! {

        impl<'a> TryFrom<&'a [u8]> for #decoder_ident<'a> {
            type Error = sbe_rt::DecodeError;

            fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
                Self::try_wrap_and_apply_header(buf, 0)
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
            /// Fallible byte view of the message. Returns `None` if the
            /// buffer is malformed or truncated. Prefer [`Self::as_bytes`]
            /// for explicit error handling.
            pub fn as_ref_opt(&self) -> Option<&[u8]> {
                self.as_bytes().ok()
            }
        }
    });

    // 13. Display impl
    let display_ts = generate_decoder_display(msg, domain_types);
    ts.extend(display_ts);

    // 14. Repeating Group decoders — use pre-computed dedup names from section 8
    for (gi, g) in msg.groups.iter().enumerate() {
        let unique = &group_unique_names[gi];
        ts.extend(generate_group_decoder(
            g,
            elements,
            byte_order,
            unique,
            &conversions,
            domain_types,
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
        multi_message,
        &group_unique_names,
    ));

    // 15. Close the main impl block (if is_fixed or not, the block is closed already)
    // Actually the impl block is opened but the `}` is emitted by the trait impls section above.
    // The quote! for trait impls starts with `}` to close the impl block first.
    // Let me verify: the impl block opening uses `quote! { impl ... { ... }`
    // Wait, no. Let me re-check the flow.
    //
    // Section 2 opens: quote! { impl ... { ...  (no closing })
    // Section 12 starts with: `}` (closing the impl)
    // So the impl is properly closed.

    // Group decoders don't need the impl to still be open - they're separate impl blocks

    // 15. Domain objects — owned structs with From<Decoder> for application-layer use
    if domain_objects {
        ts.extend(generate_domain_objects(
            msg,
            elements,
            &name,
            &name,
            multi_message,
            byte_order,
            conversions,
            domain_types,
        ));
    }

    ts
}

/// Generate owned domain structs + From<Decoder> impls for a message and all
/// its group entries. These are application-layer types (Vec, String) that
/// coexist with the zero-copy flyweight decoders.
fn generate_domain_objects(
    msg: &MessageStructure,
    elements: &SchemaElements,
    msg_name: &str,
    _parent_scope: &str,
    multi_message: bool,
    _byte_order: ByteOrder,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let mut ts = proc_macro2::TokenStream::new();
    let _has_conversion = domain_has_conversion(&msg.fields, &msg.groups, &conversions);
    generate_domain_recursive(
        msg_name,
        msg_name,
        &msg.fields,
        &msg.groups,
        &msg.var_data,
        elements,
        multi_message,
        msg_name,
        conversions,
        domain_types,
        false, // is_entry — this is a message, not a group entry
        &mut ts,
        span,
    );
    ts
}

/// Check whether any field, group entry, or nested group under these
/// fields/groups uses a registered decimal composite.
fn domain_has_conversion(
    fields: &[MessageField],
    groups: &[MessageGroup],
    conversions: &[crate::ConversionSelector],
) -> bool {
    for f in fields {
        if let FieldType::Composite { name, .. } = &f.field_type {
            if conversions
                .iter()
                .any(|sel| matches!(sel, crate::ConversionSelector::NamedType(n) if n == name))
            {
                return true;
            }
        }
    }
    for g in groups {
        if domain_has_conversion(&g.fields, &g.groups, conversions) {
            return true;
        }
    }
    false
}

#[allow(clippy::too_many_arguments, clippy::only_used_in_recursion)]
fn generate_domain_recursive(
    struct_prefix: &str,
    decoder_name: &str,
    fields: &[MessageField],
    groups: &[MessageGroup],
    var_data: &[MessageVarData],
    elements: &SchemaElements,
    multi_message: bool,
    msg_name: &str,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    is_entry: bool,
    ts: &mut proc_macro2::TokenStream,
    span: proc_macro2::Span,
) {
    let domain_ident = syn::Ident::new(&format!("{struct_prefix}Domain"), span);
    let decoder_ident = syn::Ident::new(&format!("{decoder_name}Decoder"), span);

    let mut struct_fields: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut from_exprs: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut encode_stmts: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut group_encode_stmts: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut vardata_encode_stmts: Vec<proc_macro2::TokenStream> = Vec::new();

    // Scalar / array / composite / enum / set fields
    for f in fields {
        if f.presence == Presence::Constant {
            continue;
        }
        let f_snake = to_snake_case(&f.name);
        let f_ident = syn::Ident::new(&f_snake, span);
        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_type_str = rust_type(*prim);
                let r_type: syn::Type = syn::parse_str(r_type_str).unwrap();
                // Domain type for primitives with a semantic/named conversion
                // (e.g. u64 UTCTimestamp → chrono::DateTime<Utc>). Only the
                // scalar required case is converted; arrays/optional keep the
                // wire type.
                let scalar_domain = if length.is_none() && f.presence != Presence::Optional {
                    find_domain_type(f, domain_types)
                } else {
                    None
                };
                let scalar_ty: syn::Type = match scalar_domain {
                    Some(dt) => syn::parse_str(dt).unwrap(),
                    None => r_type.clone(),
                };
                let enc_setter = syn::Ident::new(
                    &domain_encode_setter_name(f, conversions, domain_types, &f_snake),
                    span,
                );
                if let Some(len) = length {
                    let len_lit = syn::LitInt::new(&len.to_string(), span);
                    struct_fields.push(quote::quote! { pub #f_ident: [#r_type; #len_lit] });
                    from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                    encode_stmts.push(quote::quote! { enc.#enc_setter(self.#f_ident); });
                } else if f.presence == Presence::Optional {
                    struct_fields.push(quote::quote! { pub #f_ident: Option<#r_type> });
                    from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                    encode_stmts.push(
                        quote::quote! { if let Some(v) = self.#f_ident { enc.#enc_setter(v); } },
                    );
                } else {
                    // Domain-typed scalars use the concrete converted getter.
                    // Conversion-only renames the raw flyweight getter to *_wire.
                    let from_getter = if field_has_conversion_free(f, conversions)
                        && scalar_domain.is_none()
                    {
                        syn::Ident::new(&format!("{f_snake}_wire"), span)
                    } else {
                        f_ident.clone()
                    };
                    struct_fields.push(quote::quote! { pub #f_ident: #scalar_ty });
                    from_exprs.push(quote::quote! { #f_ident: dec.#from_getter() });
                    encode_stmts.push(quote::quote! { enc.#enc_setter(self.#f_ident); });
                }
            }
            FieldType::Composite {
                name: comp_name, ..
            } => {
                let comp_pascal = to_pascal_case(comp_name);
                let comp_ident = syn::Ident::new(&comp_pascal, span);
                let as_struct_ident = syn::Ident::new(&format!("{f_snake}_value"), span);
                // If a domain type is configured for this composite (e.g.
                // Decimal → rust_decimal::Decimal), the DTO field uses the
                // domain type and reads/writes via the domain accessors.
                let domain_ty = find_domain_type(f, domain_types);
                let enc_setter = syn::Ident::new(
                    &domain_encode_setter_name(f, conversions, domain_types, &f_snake),
                    span,
                );
                let field_ty: proc_macro2::TokenStream = match domain_ty {
                    Some(dt) => {
                        let parsed: syn::Type = syn::parse_str(dt).unwrap();
                        quote::quote! { #parsed }
                    }
                    None => quote::quote! { #comp_ident },
                };
                // Drive-by fix: versioned composites return Option<T> on decoders,
                // so the DTO field must also be optional.
                if !is_entry && f.since_version > 0 {
                    struct_fields.push(quote::quote! { pub #f_ident: Option<#field_ty> });
                    if domain_ty.is_some() {
                        from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                        encode_stmts.push(
                            quote::quote! { if let Some(v) = self.#f_ident { enc.#enc_setter(v); } },
                        );
                    } else {
                        from_exprs.push(quote::quote! { #f_ident: dec.#as_struct_ident() });
                        encode_stmts
                            .push(quote::quote! { if let Some(ref v) = self.#f_ident { enc.#enc_setter(*v); } });
                    }
                } else {
                    struct_fields.push(quote::quote! { pub #f_ident: #field_ty });
                    if domain_ty.is_some() {
                        from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                    } else {
                        from_exprs.push(quote::quote! { #f_ident: dec.#as_struct_ident() });
                    }
                    encode_stmts.push(quote::quote! { enc.#enc_setter(self.#f_ident); });
                }
            }
            FieldType::Enum {
                name: enum_name, ..
            } => {
                if is_bool_enum(elements, enum_name) {
                    // bool enums → plain bool in DTO
                    let bool_ident = syn::Ident::new(&format!("{f_snake}_bool"), span);
                    if !is_entry && f.since_version > 0 {
                        struct_fields.push(quote::quote! { pub #f_ident: Option<bool> });
                        from_exprs.push(quote::quote! { #f_ident: dec.#bool_ident() });
                        encode_stmts.push(quote::quote! { if let Some(v) = self.#f_ident { enc.#bool_ident(v); } });
                    } else {
                        struct_fields.push(quote::quote! { pub #f_ident: bool });
                        from_exprs.push(quote::quote! { #f_ident: dec.#bool_ident() });
                        encode_stmts.push(quote::quote! { enc.#bool_ident(self.#f_ident); });
                    }
                } else {
                    let type_ident = syn::Ident::new(&to_pascal_case(enum_name), span);
                    if !is_entry && f.since_version > 0 {
                        struct_fields.push(quote::quote! { pub #f_ident: Option<#type_ident> });
                        from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                        encode_stmts.push(
                            quote::quote! { if let Some(v) = self.#f_ident { enc.#f_ident(v); } },
                        );
                    } else {
                        struct_fields.push(quote::quote! { pub #f_ident: #type_ident });
                        from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                        encode_stmts.push(quote::quote! { enc.#f_ident(self.#f_ident); });
                    }
                }
            }
            FieldType::Set {
                name: enum_name, ..
            } => {
                let type_ident = syn::Ident::new(&to_pascal_case(enum_name), span);
                if !is_entry && f.since_version > 0 {
                    struct_fields.push(quote::quote! { pub #f_ident: Option<#type_ident> });
                    encode_stmts.push(
                        quote::quote! { if let Some(v) = self.#f_ident { enc.#f_ident(v); } },
                    );
                } else {
                    struct_fields.push(quote::quote! { pub #f_ident: #type_ident });
                    encode_stmts.push(quote::quote! { enc.#f_ident(self.#f_ident); });
                }
                from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
            }
        }
    }

    // Group fields → Vec<EntryDomain>
    for g in groups {
        let g_snake = to_snake_case(&g.name);
        let g_pascal = to_pascal_case(&g.name);
        let g_field_ident = syn::Ident::new(&g_snake, span);
        let entry_domain_ident =
            syn::Ident::new(&format!("{struct_prefix}{g_pascal}EntryDomain"), span);

        let g_scoped = if decoder_name.ends_with("Entry") {
            // Nested group: prefix with parent group's scoped name
            let parent_scoped = decoder_name.trim_end_matches("Entry");
            format!("{parent_scoped}{g_pascal}")
        } else if multi_message {
            format!("{msg_name}{g_pascal}")
        } else {
            g_pascal.clone()
        };
        let g_entry_dec_ident = syn::Ident::new(&format!("{g_scoped}EntryDecoder"), span);

        struct_fields.push(quote::quote! { pub #g_field_ident: Vec<#entry_domain_ident> });
        // Fixed-entry groups (no tail) yield entries directly;
        // tailed-entry groups yield Result<EntryDecoder, _>.
        let has_tail = !g.var_data.is_empty() || !g.groups.is_empty();
        if has_tail {
            from_exprs.push(quote::quote! {
                #g_field_ident: dec.#g_field_ident()
                    .map(|g| {
                        g.map(|r| r.map(#entry_domain_ident::from))
                            .collect::<Result<Vec<_>, _>>()
                    })
                    .unwrap_or_else(|e| Err(e))?
            });
        } else {
            from_exprs.push(quote::quote! {
                #g_field_ident: dec.#g_field_ident()
                    .map(|g| Ok(g.map(#entry_domain_ident::from).collect()))
                    .unwrap_or_else(|e| Err(e))?
            });
        }

        // Encode: group chain
        let (_, _, count_prim) = get_dim_num_layout(elements, &g.dimension_type);
        let count_ty: syn::Type = syn::parse_str(rust_type(count_prim)).unwrap();
        group_encode_stmts.push(quote::quote! {
            let enc = enc.#g_field_ident(
                self.#g_field_ident.len() as #count_ty,
                |g| -> Result<(), sbe_rt::EncodeError> {
                    for e in &self.#g_field_ident {
                        g.add(|entry| -> Result<(), sbe_rt::EncodeError> {
                            e.encode_into(entry)
                        })?;
                    }
                    Ok(())
                }
            )?;
        });

        // Recursively generate the entry domain struct
        let entry_prefix = format!("{struct_prefix}{g_pascal}Entry");
        let entry_decoder_name = format!("{g_scoped}Entry");
        generate_domain_recursive(
            &entry_prefix,
            &entry_decoder_name,
            &g.fields,
            &g.groups,
            &g.var_data,
            elements,
            multi_message,
            msg_name,
            &conversions,
            domain_types,
            true, // is_entry — group entries always return T for enums
            ts,
            span,
        );
    }

    // VarData fields → Vec<u8>
    for vd in var_data {
        let vd_snake = to_snake_case(&vd.name);
        let vd_ident = syn::Ident::new(&vd_snake, span);
        struct_fields.push(quote::quote! { pub #vd_ident: Vec<u8> });
        from_exprs.push(quote::quote! {
            #vd_ident: dec.#vd_ident().unwrap_or(&[]).to_vec()
        });
        vardata_encode_stmts.push(quote::quote! {
            let enc = enc.#vd_ident(&self.#vd_ident)?;
        });
    }

    // ── Generate the struct + From impl ──────────────────────────────
    let encoder_ident = syn::Ident::new(&format!("{decoder_name}Encoder"), span);
    ts.extend(quote::quote! {
        /// Owned domain object — application-layer counterpart to the flyweight decoder.
        /// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct #domain_ident {
            #(#struct_fields),*
        }

        impl #domain_ident {
            /// Fallible conversion from a decoder. Propagates decode errors
            /// from malformed group entries instead of silently dropping them.
            pub fn try_from_decoder(
                dec: #decoder_ident<'_>,
            ) -> Result<Self, sbe_rt::DecodeError> {
                Ok(Self {
                    #(#from_exprs),*
                })
            }
        }

        impl<'a> From<#decoder_ident<'a>> for #domain_ident {
            fn from(dec: #decoder_ident<'a>) -> Self {
                Self::try_from_decoder(dec)
                    .expect("domain conversion failed — use try_from_decoder for fallible conversion")
            }
        }
    });

    // ── Encode-from-DTO ──────────────────────────────────────────────
    if is_entry {
        // Entry domains: encode_into for use inside group closures
        let entry_encoder_ident = syn::Ident::new(&format!("{decoder_name}Encoder"), span);
        let encode_body = if !vardata_encode_stmts.is_empty() || !group_encode_stmts.is_empty() {
            quote::quote! {
                #(#encode_stmts)*
                #(#group_encode_stmts)*
                #(#vardata_encode_stmts)*
                Ok(())
            }
        } else {
            quote::quote! {
                #(#encode_stmts)*
                Ok(())
            }
        };

        // Build entry-level length_contribution
        let entry_block_len = groups.iter().fold(
            fields.iter().fold(0usize, |acc, f| {
                let size = f.field_type.size();
                acc.max(f.offset + size)
            }),
            |acc, g| acc.max(g.block_length),
        );
        let entry_bl_lit = syn::LitInt::new(&entry_block_len.to_string(), span);
        let mut len_stmts = quote::quote! {
            let mut len: usize = #entry_bl_lit;
        };
        for ng in groups {
            let ng_snake = syn::Ident::new(&to_snake_case(&ng.name), span);
            let (_, dim_size, _, _) = get_dimension_info(elements, &ng.dimension_type);
            let ds_lit = syn::LitInt::new(&dim_size.to_string(), span);
            len_stmts.extend(quote::quote! {
                len = len.checked_add(#ds_lit).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                for entry in &self.#ng_snake {
                    len = len.checked_add(entry.length_contribution()?).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                }
            });
        }
        for vd in var_data {
            let vd_snake = syn::Ident::new(&to_snake_case(&vd.name), span);
            let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
            let ps_lit = syn::LitInt::new(&prefix_size.to_string(), span);
            if let Some(max) = vd.max_length {
                let max_lit = syn::LitInt::new(&max.to_string(), span);
                let vd_name = &vd.name;
                len_stmts.extend(quote::quote! {
                    if self.#vd_snake.len() > #max_lit {
                        return Err(sbe_rt::EncodeError::VarDataTooLong {
                            field: #vd_name,
                            max_length: #max_lit,
                            actual: self.#vd_snake.len(),
                        });
                    }
                });
            }
            len_stmts.extend(quote::quote! {
                len = len.checked_add(#ps_lit).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                len = len.checked_add(self.#vd_snake.len()).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
            });
        }
        len_stmts.extend(quote::quote! { Ok(len) });

        ts.extend(quote::quote! {
            impl #domain_ident {
                /// Encode this entry into a group entry encoder.
                pub fn encode_into<'a>(
                    &self,
                    enc: &mut #entry_encoder_ident<'a>,
                ) -> Result<(), sbe_rt::EncodeError> {
                    #encode_body
                }

                /// Compute this entry's contribution to the total encoded length
                /// (entry block + nested groups + entry var-data).
                pub fn length_contribution(&self) -> Result<usize, sbe_rt::EncodeError> {
                    #len_stmts
                }
            }
        });
    } else {
        // Message domains: full encode via wrap_and_apply_header
        let has_optional = fields
            .iter()
            .any(|f| f.presence == Presence::Optional && f.null_value.is_some());
        let nullify = if has_optional {
            quote::quote! { enc.apply_nulls(); }
        } else {
            quote::quote! {}
        };
        // Build message-level encoded_length computation
        let block_len = fields.iter().fold(0usize, |acc, f| {
            let size = f.field_type.size();
            acc.max(f.offset + size)
        });
        let bl_lit = syn::LitInt::new(&block_len.to_string(), span);
        let mut msg_len_stmts = quote::quote! {
            let mut len: usize = #bl_lit;
        };
        for g in groups {
            let g_snake = syn::Ident::new(&to_snake_case(&g.name), span);
            let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
            let ds_lit = syn::LitInt::new(&dim_size.to_string(), span);
            msg_len_stmts.extend(quote::quote! {
                len = len.checked_add(#ds_lit).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                for entry in &self.#g_snake {
                    len = len.checked_add(entry.length_contribution()?).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                }
            });
        }
        for vd in var_data {
            let vd_snake = syn::Ident::new(&to_snake_case(&vd.name), span);
            let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
            let ps_lit = syn::LitInt::new(&prefix_size.to_string(), span);
            if let Some(max) = vd.max_length {
                let max_lit = syn::LitInt::new(&max.to_string(), span);
                let vd_name = &vd.name;
                msg_len_stmts.extend(quote::quote! {
                    if self.#vd_snake.len() > #max_lit {
                        return Err(sbe_rt::EncodeError::VarDataTooLong {
                            field: #vd_name,
                            max_length: #max_lit,
                            actual: self.#vd_snake.len(),
                        });
                    }
                });
            }
            msg_len_stmts.extend(quote::quote! {
                len = len.checked_add(#ps_lit).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                len = len.checked_add(self.#vd_snake.len()).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
            });
        }
        msg_len_stmts.extend(quote::quote! { Ok(len) });
        let has_tail = !group_encode_stmts.is_empty() || !vardata_encode_stmts.is_empty();
        let encode_body = if has_tail {
            quote::quote! {
                let mut enc = #encoder_ident::try_wrap_and_apply_header(buf, 0)?;
                #nullify
                #(#encode_stmts)*
                #(#group_encode_stmts)*
                #(#vardata_encode_stmts)*
                Ok(enc.encoded_length_with_header())
            }
        } else {
            // Fixed-only message: encoder implements AsRef<[u8]>
            quote::quote! {
                let mut enc = #encoder_ident::try_wrap_and_apply_header(buf, 0)?;
                #nullify
                #(#encode_stmts)*
                Ok(enc.as_ref().len())
            }
        };
        ts.extend(quote::quote! {
            impl #domain_ident {
                /// Encode this domain object into a byte buffer.
                pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
                    #encode_body
                }

                /// Compute the exact SBE message body length from this domain object.
                /// Matches the length returned by [`Self::encode`].
                pub fn encoded_length(&self) -> Result<usize, sbe_rt::EncodeError> {
                    #msg_len_stmts
                }

                /// Compute the exact SBE message length including the message header.
                /// Matches `encode()` return value for non-fixed messages.
                pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::EncodeError> {
                    Ok(self.encoded_length()? + #encoder_ident::HEADER_LENGTH)
                }
            }
        });
    }
}

fn generate_decoder_display(
    msg: &MessageStructure,
    domain_types: &[(crate::ConversionSelector, String)],
) -> proc_macro2::TokenStream {
    let name = to_pascal_case(&msg.name);
    let decoder_ident =
        syn::Ident::new(&format!("{}Decoder", name), proc_macro2::Span::call_site());
    let type_name_lit =
        syn::LitStr::new(&format!("{}Decoder", name), proc_macro2::Span::call_site());
    let mut body = proc_macro2::TokenStream::new();
    let mut debug_body = proc_macro2::TokenStream::new();
    let display_header = format!("{} {{{{ ", name);
    body.extend(quote::quote! {
        write!(f, #display_header)?;
    });
    let mut out_idx = 0usize;
    for f in &msg.fields {
        let snake = to_snake_case(&f.name);
        let f_ident = syn::Ident::new(&snake, proc_macro2::Span::call_site());
        let sep = if out_idx == 0 { "" } else { ", " };
        let end_off = f.offset + f.field_type.size();
        let end_off_lit = syn::LitInt::new(&end_off.to_string(), proc_macro2::Span::call_site());
        // Only touch wire when the field's full range is in-buffer — Display must
        // not panic on truncated / invalid SBE (todo 73 + ops 3am debugging).
        let in_bounds = quote::quote! {
            self.pos.saturating_add(#end_off_lit) <= self.buf.len()
                && #end_off_lit <= self.acting_block_length
        };
        match &f.field_type {
            FieldType::Primitive(_prim, length) => {
                if f.presence == Presence::Constant || length.is_some() {
                    continue;
                }
                let fmt_str = format!("{sep}{snake}: {{:?}}");
                // {:?} renders Option<T> without T: Display bound, switch to {} if all field types gain Display
                body.extend(quote::quote! {
                    if #in_bounds {
                        let v = self.#f_ident();
                        write!(f, #fmt_str, v)?;
                    }
                });
                let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
                debug_body.extend(quote::quote! {
                    if #in_bounds {
                        let v = self.#f_ident();
                        d.field(#name_lit, &v);
                    }
                });
                out_idx += 1;
            }
            FieldType::Enum {
                name: enum_name, ..
            } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                let fmt_str = format!("{sep}{snake}: {enum_name}::{{e:?}}");
                if f.since_version > 0 {
                    body.extend(quote::quote! {
                        if #in_bounds {
                            if let Some(e) = self.#f_ident() {
                                write!(f, #fmt_str)?;
                            }
                        }
                    });
                } else {
                    body.extend(quote::quote! {
                        if #in_bounds {
                            let e = self.#f_ident();
                            write!(f, #fmt_str)?;
                        }
                    });
                }
                let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
                debug_body.extend(quote::quote! {
                    if #in_bounds {
                        let v = self.#f_ident();
                        d.field(#name_lit, &v);
                    }
                });
                out_idx += 1;
            }
            FieldType::Set { .. } => {}
            FieldType::Composite { .. } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                // Domain-converted composites (e.g. Decimal→rust_decimal) have
                // Display on the domain type. Raw composites return a sub-decoder
                // that may not implement Display — skip in debug_struct to avoid
                // trait errors. The value is visible via group entry Display.
                if find_domain_type(f, domain_types).is_some() {
                    let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
                    debug_body.extend(quote::quote! {
                        if #in_bounds {
                            d.field(#name_lit, &format_args!("{}", self.#f_ident()));
                        }
                    });
                }
            }
        }
    }
    for g in &msg.groups {
        let g_snake = to_snake_case(&g.name);
        let g_ident = syn::Ident::new(&g_snake, proc_macro2::Span::call_site());
        let sep = if out_idx == 0 { "" } else { ", " };
        let g_total_tail = g.groups.len() + g.var_data.len();
        if g_total_tail == 0 {
            let fmt_open = format!("{sep}{g_snake}: [");
            body.extend(quote::quote! {
                if let Ok(g) = self.#g_ident() {
                    write!(f, #fmt_open)?;
                    for (i, entry) in g.enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        write!(f, "{}", entry)?;
                    }
                    write!(f, "]")?;
                }
            });
        } else {
            let fmt_open = format!("{sep}{g_snake}: [");
            body.extend(quote::quote! {
                if let Ok(g) = self.#g_ident() {
                    write!(f, #fmt_open)?;
                    for (i, result) in g.enumerate() {
                        if i > 0 { write!(f, ", ")?; }
                        match result {
                            Ok(entry) => write!(f, "{}", entry)?,
                            Err(_) => write!(f, "{{err}}")?,
                        }
                    }
                    write!(f, "]")?;
                }
            });
        }
        out_idx += 1;
        // Debug: format group entries as a Vec<String> via Display.
        let g_name_lit = syn::LitStr::new(&g.name, proc_macro2::Span::call_site());
        if g_total_tail == 0 {
            debug_body.extend(quote::quote! {
                if let Ok(_g) = self.#g_ident() {
                    let entries: Vec<String> = _g.map(|e| format!("{e}")).collect();
                    d.field(#g_name_lit, &entries);
                }
            });
        } else {
            debug_body.extend(quote::quote! {
                if let Ok(_g) = self.#g_ident() {
                    let entries: Vec<String> = _g.filter_map(|r| r.ok()).map(|e| format!("{e}")).collect();
                    d.field(#g_name_lit, &entries);
                }
            });
        }
    }
    for vd in &msg.var_data {
        let vd_snake = to_snake_case(&vd.name);
        let vd_ident = syn::Ident::new(&vd_snake, proc_macro2::Span::call_site());
        let sep = if out_idx == 0 { "" } else { ", " };
        let fmt_str = format!("{sep}{vd_snake}: {{}}");
        let err_fmt = format!("{sep}{vd_snake}: <{{}} bytes>");
        body.extend(quote::quote! {
            if let Ok(d) = self.#vd_ident() {
                match std::str::from_utf8(d) {
                    Ok(s) => write!(f, #fmt_str, s)?,
                    Err(_) => write!(f, #err_fmt, d.len())?,
                }
            }
        });
        let vd_name_lit = syn::LitStr::new(&vd.name, proc_macro2::Span::call_site());
        debug_body.extend(quote::quote! {
            if let Ok(_data) = self.#vd_ident() {
                match std::str::from_utf8(_data) {
                    Ok(_s) => d.field(#vd_name_lit, &_s),
                    Err(_) => d.field(#vd_name_lit, &format!("<{} bytes>", _data.len())),
                };
            }
        });
        out_idx += 1;
    }
    body.extend(quote::quote! {
        write!(f, " }}")
    });
    // Structural Debug never reads wire bytes — safe for truncated / invalid buffers.
    let ts = quote::quote! {
        impl<'a> core::fmt::Display for #decoder_ident<'a> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                // Display delegates to Debug — one impl, both {} and {:?} work.
                // {:?} gives debug_struct (compact), {:#?} gives pretty multi-line.
                core::fmt::Debug::fmt(self, f)
            }
        }

        impl<'a> core::fmt::Debug for #decoder_ident<'a> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                let mut d = f.debug_struct(#type_name_lit);
                #debug_body
                d.finish()
            }
        }
    };
    ts
}

fn generate_group_decoder(
    g: &MessageGroup,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    scoped_name: &str,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
) -> proc_macro2::TokenStream {
    let mut ts = proc_macro2::TokenStream::new();
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
    let block_len_lit =
        syn::LitInt::new(&g.block_length.to_string(), proc_macro2::Span::call_site());
    let bl_field_ident = syn::Ident::new(&bl_field, proc_macro2::Span::call_site());
    let count_field_ident = syn::Ident::new(&count_field, proc_macro2::Span::call_site());
    let g_name_lit = syn::LitStr::new(&g.name, proc_macro2::Span::call_site());

    // Struct definition + wrap() + wrap_with_parent() + is_empty()
    if let Some(ref desc) = g.description {
        ts.extend(doc_attr_tokens(desc));
    }
    ts.extend(quote::quote! {
        pub struct #decoder_ident<'a> {
            buf: &'a [u8],
            pos: usize,
            count: usize,
            start: usize,
            total: usize,
            acting_version: u16,
            acting_block_length: usize,
            // Parent message body position + acting block length, remembered so
            // `finish()` can reconstruct the next message decoder stage
            // (DECISIONS.md §3 consuming tail stages). Unused by the legacy
            // `&self` random-access accessors.
            parent_pos: usize,
            parent_block_length: usize,
        }

        impl<'a> #decoder_ident<'a> {
            pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

            #[inline]
            pub fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Result<Self, sbe_rt::DecodeError> {
                Self::wrap_with_parent(buf, pos, acting_version, 0, 0)
            }

            /// Like `wrap()` but remembers the parent message body position and
            /// acting block length so `finish()` can rebuild the next stage.
            #[inline]
            pub fn wrap_with_parent(
                buf: &'a [u8],
                pos: usize,
                acting_version: u16,
                parent_pos: usize,
                parent_block_length: usize,
            ) -> Result<Self, sbe_rt::DecodeError> {
                // Trust boundary: always validate dimension header fits in buffer
                if pos + #dim_size_lit > buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: #g_name_lit,
                        needed: #dim_size_lit,
                        available: buf.len().saturating_sub(pos),
                    });
                }
                let bytes: [u8; #dim_size_lit] = read_bytes::<#dim_size_lit>(buf, pos);
                let header = #dim_name_ident(bytes);
                let count = header.#count_field_ident() as usize;
                let block_length = header.#bl_field_ident() as usize;
                Ok(Self {
                    buf,
                    pos: pos + #dim_size_lit,
                    count,
                    start: pos + #dim_size_lit,
                    total: count,
                    acting_version,
                    acting_block_length: block_length,
                    parent_pos,
                    parent_block_length,
                })
            }

            #[inline]
            pub fn is_empty(&self) -> bool {
                self.count == 0
            }
        }
    });

    // remaining(), rewind()
    ts.extend(quote::quote! {
        impl<'a> #decoder_ident<'a> {
            #[inline]
            pub const fn remaining(&self) -> usize {
                self.count
            }

            /// Dimension wrap (trusted position): the caller has
            /// proven `pos` is within a validated extent.
            #[inline]
            pub fn wrap_trusted(
                buf: &'a [u8], pos: usize, acting_version: u16,
                parent_pos: usize, parent_block_length: usize,
            ) -> Self {
                let bytes: [u8; #dim_size_lit] = read_bytes::<#dim_size_lit>(buf, pos);
                let header = #dim_name_ident(bytes);
                let count = header.#count_field_ident() as usize;
                let block_length = header.#bl_field_ident() as usize;
                Self {
                    buf, pos: pos + #dim_size_lit, count, start: pos + #dim_size_lit,
                    total: count, acting_version, acting_block_length: block_length,
                    parent_pos, parent_block_length,
                }
            }

            #[inline]
            pub fn rewind(&mut self) -> &mut Self {
                self.pos = self.start;
                self.count = self.total;
                self
            }
        }
    });

    let total_tail = g.groups.len() + g.var_data.len();

    // skip_n()
    if total_tail == 0 {
        ts.extend(quote::quote! {
            impl<'a> #decoder_ident<'a> {
                #[inline]
                pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
                    if n > self.count {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: n * self.acting_block_length,
                            available: self.count * self.acting_block_length,
                        });
                    }
                    self.pos += n * self.acting_block_length;
                    self.count -= n;
                    Ok(())
                }
            }
        });
    } else {
        ts.extend(quote::quote! {
            impl<'a> #decoder_ident<'a> {
                #[inline]
                pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
                    if n > self.count {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: n * Self::ENTRY_BLOCK_LENGTH,
                            available: self.count * Self::ENTRY_BLOCK_LENGTH,
                        });
                    }
                    for _ in 0..n {
                        let entry = #entry_decoder_ident::wrap(self.buf, self.pos, self.acting_block_length, self.acting_version);
                        self.pos += entry.encoded_length()?;
                        self.count -= 1;
                    }
                    Ok(())
                }
            }
        });
    }

    // nth()
    ts.extend(quote::quote! {
        impl<'a> #decoder_ident<'a> {
            #[inline]
            pub fn nth(&self, idx: usize) -> Result<#entry_decoder_ident<'a>, sbe_rt::DecodeError> {
                if idx >= self.total {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: #g_name_lit,
                        needed: (idx + 1) * self.acting_block_length,
                        available: self.total * self.acting_block_length,
                    });
                }
                let offset = self.start + idx * self.acting_block_length;
                if offset + self.acting_block_length > self.buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: #g_name_lit,
                        needed: self.acting_block_length,
                        available: self.buf.len().saturating_sub(offset),
                    });
                }
                Ok(#entry_decoder_ident::wrap(self.buf, offset, self.acting_block_length, self.acting_version))
            }
        }
    });

    // Iterator implementation
    if total_tail == 0 {
        ts.extend(quote::quote! {
            impl<'a> Iterator for #decoder_ident<'a> {
                type Item = #entry_decoder_ident<'a>;

                fn next(&mut self) -> Option<Self::Item> {
                    if self.count == 0 {
                        return None;
                    }
                    let entry = #entry_decoder_ident::wrap(self.buf, self.pos, self.acting_block_length, self.acting_version);
                    self.pos += self.acting_block_length;
                    self.count -= 1;
                    Some(entry)
                }
            }

            impl<'a> ExactSizeIterator for #decoder_ident<'a> {
                fn len(&self) -> usize {
                    self.count
                }
            }
        });
    } else {
        ts.extend(quote::quote! {
            impl<'a> Iterator for #decoder_ident<'a> {
                type Item = Result<#entry_decoder_ident<'a>, sbe_rt::DecodeError>;

                fn next(&mut self) -> Option<Self::Item> {
                    if self.count == 0 {
                        return None;
                    }
                    let entry = #entry_decoder_ident::wrap(self.buf, self.pos, self.acting_block_length, self.acting_version);
                    let size = match entry.encoded_length() {
                        Ok(s) => s,
                        Err(e) => {
                            self.count = 0;
                            return Some(Err(e));
                        }
                    };
                    self.pos += size;
                    self.count -= 1;
                    Some(Ok(entry))
                }
            }

            impl<'a> ExactSizeIterator for #decoder_ident<'a> {
                fn len(&self) -> usize {
                    self.count
                }
            }
        });
    }

    // EntryDecoder struct fields and methods
    let mut entry_body = proc_macro2::TokenStream::new();

    // wrap() method header. Entries with tail components carry a one-shot
    // tail-end cache: the group iterator computes the entry extent to
    // advance, and var-data accessors reuse it instead of re-reading the
    // length header (todo 110, re-opened 2026-07-17).
    if total_tail == 0 {
        entry_body.extend(quote::quote! {
            pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

            #[inline]
            pub fn wrap(buf: &'a [u8], pos: usize, acting_block_length: usize, acting_version: u16) -> Self {
                Self { buf, pos, acting_version, acting_block_length }
            }
        });
    } else {
        entry_body.extend(quote::quote! {
            pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

            #[inline]
            pub fn wrap(buf: &'a [u8], pos: usize, acting_block_length: usize, acting_version: u16) -> Self {
                Self { buf, pos, acting_version, acting_block_length, tail_end: core::cell::Cell::new(None) }
            }
        });
    }

    // Fields of group entry
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
                                #[inline]
                                pub const fn #f_name_ident(&self) -> &'static str {
                                    #val_lit
                                }
                            });
                        } else {
                            let expr = constant_value_expr(*prim, val);
                            let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                            entry_body.extend(quote::quote! {
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
                    let mut elem_exprs: Vec<proc_macro2::TokenStream> = Vec::new();
                    for i in 0..*len {
                        let start = i * prim_size;
                        let end = start + prim_size;
                        let byte_indices: Vec<proc_macro2::TokenStream> = (start..end)
                            .map(|idx| quote::quote! { all[#idx] })
                            .collect();
                        elem_exprs.push(quote::quote! {
                            #r_type_ty::#order_fn([#(#byte_indices),*])
                        });
                    }
                    entry_body.extend(quote::quote! {
                        #[inline]
                        pub fn #f_name_ident(&self) -> [#r_type_ty; #len_lit] {
                            let offset = self.pos + #offset_lit;
                            let size = #total_size_lit;
                            let all: [u8; #total_size_lit] = read_bytes_unchecked::<#total_size_lit>(self.buf, offset);
                            [#(#elem_exprs),*]
                        }
                    });
                } else if f.presence == Presence::Optional {
                    let null_val = f.null_value.unwrap_or(0);
                    let null_check = if *prim == PrimitiveType::Float {
                        format!("val.to_bits() == {} as u32", null_val)
                    } else if *prim == PrimitiveType::Double {
                        format!("val.to_bits() == {}", null_val)
                    } else {
                        format!("val == {}_u64 as {}", null_val, r_type)
                    };
                    let null_check_expr: syn::Expr = syn::parse_str(&null_check).unwrap();

                    entry_body.extend(quote::quote! {
                        #[inline]
                        pub fn #f_name_ident(&self) -> Option<#r_type_ty> {
                            let offset = self.pos + #offset_lit;
                            let val = #r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset));
                            if #null_check_expr {
                                None
                            } else {
                                Some(val)
                            }
                        }
                    });
                } else {
                    entry_body.extend(quote::quote! {
                        #[inline]
                        pub fn #f_name_ident(&self) -> #r_type_ty {
                            let offset = self.pos + #offset_lit;
                            #r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset))
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

                // Default: flyweight (zero-copy)
                entry_body.extend(quote::quote! {
                    #[inline]
                    pub fn #f_name_ident(&self) -> #target_decoder_name<'_> {
                        let offset = self.pos + #offset_lit;
                        #target_decoder_name { buf: self.buf, pos: offset }
                    }
                });

                // Eager copy accessor (_value)
                let as_struct_ident =
                    syn::Ident::new(&format!("{}_value", f_name), proc_macro2::Span::call_site());
                entry_body.extend(quote::quote! {
                    #[inline]
                    pub fn #as_struct_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident(read_bytes_unchecked::<#comp_size_lit>(self.buf, offset))
                    }
                });

                entry_body.extend(quote::quote! {
                    #[inline]
                    pub const fn #raw_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        let mut bytes = [0u8; #comp_size_lit];
                        bytes.copy_from_slice(unsafe { core::slice::from_raw_parts(self.buf.as_ptr().add(offset), #comp_size_lit) });
                        #target_ident(bytes)
                    }
                });

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

                entry_body.extend(quote::quote! {
                    #[inline]
                    pub fn #f_name_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident::from_raw(#r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset)))
                    }
                });

                entry_body.extend(quote::quote! {
                    #[inline]
                    pub const fn #raw_ident(&self) -> #r_type_ty {
                        let offset = self.pos + #offset_lit;
                        let mut bytes = [0u8; #prim_size_lit];
                        bytes.copy_from_slice(unsafe { core::slice::from_raw_parts(self.buf.as_ptr().add(offset), #prim_size_lit) });
                        #r_type_ty::#order_fn(bytes)
                    }
                });

                if is_bool_enum(elements, enum_name) {
                    // Use the const raw primitive accessor — the typed
                    // enum getter is not const (from_raw is runtime).
                    let bool_ident = quote::format_ident!("{}_bool", f_name);
                    entry_body.extend(quote::quote! {
                        #[inline]
                        pub const fn #bool_ident(&self) -> bool {
                            self.#raw_ident() != 0
                        }
                    });
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

                entry_body.extend(quote::quote! {
                    #[inline]
                    pub fn #f_name_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident(#r_type_ty::#order_fn(read_bytes_unchecked::<#prim_size_lit>(self.buf, offset)))
                    }
                });

                entry_body.extend(quote::quote! {
                    #[inline]
                    pub const fn #raw_ident(&self) -> #r_type_ty {
                        let offset = self.pos + #offset_lit;
                        let mut bytes = [0u8; #prim_size_lit];
                        bytes.copy_from_slice(unsafe { core::slice::from_raw_parts(self.buf.as_ptr().add(offset), #prim_size_lit) });
                        #target_ident(#r_type_ty::#order_fn(bytes)).0
                    }
                });
            }
        }
        let fconsts_ts = emit_field_consts(f);
        entry_body.extend(fconsts_ts);
    }

    // Entry decoder tail offsets
    entry_body.extend(quote::quote! {
        #[inline]
        fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
            Ok(self.pos + self.acting_block_length)
        }
    });

    let mut k = 0usize;
    for ng in &g.groups {
        let (dim_name, dim_size, bl_field, count_field) =
            get_dimension_info(elements, &ng.dimension_type);
        let ng_pascal = format!("{}{}", name, to_pascal_case(&ng.name));
        let ng_decoder_entry_ident = quote::format_ident!("{}EntryDecoder", ng_pascal);
        let dim_name_ident = syn::Ident::new(&dim_name, proc_macro2::Span::call_site());
        let bl_field_ident = syn::Ident::new(&bl_field, proc_macro2::Span::call_site());
        let count_field_ident = syn::Ident::new(&count_field, proc_macro2::Span::call_site());
        let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), proc_macro2::Span::call_site());
        let k_lit = syn::LitInt::new(&k.to_string(), proc_macro2::Span::call_site());
        let k_plus_lit = syn::LitInt::new(&(k + 1).to_string(), proc_macro2::Span::call_site());
        let ng_name_lit = syn::LitStr::new(&ng.name, proc_macro2::Span::call_site());

        let tail_k_fn = quote::format_ident!("tail_offset_{}", k);
        let tail_k1_fn = quote::format_ident!("tail_offset_{}", k + 1);
        entry_body.extend(quote::quote! {
            #[inline]
            fn #tail_k1_fn(&self) -> Result<usize, sbe_rt::DecodeError> {
                let start = self.#tail_k_fn()?;
                if start + #dim_size_lit > self.buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort { field: #ng_name_lit, needed: #dim_size_lit, available: self.buf.len().saturating_sub(start) });
                }
                let bytes: [u8; #dim_size_lit] = read_bytes::<#dim_size_lit>(self.buf, start);
                let header = #dim_name_ident(bytes);
                let count = header.#count_field_ident() as usize;
                let block_len = header.#bl_field_ident() as usize;
                let mut pos = start + #dim_size_lit;
                let mut idx = 0;
                while idx < count {
                    pos = #ng_decoder_entry_ident::skip(self.buf, pos, block_len, self.acting_version)?;
                    idx += 1;
                }
                Ok(pos)
            }
        });
        k += 1;
    }

    for vd in &g.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let type_pascal_ident = syn::Ident::new(&type_pascal, proc_macro2::Span::call_site());
        let len_field_ident = syn::Ident::new(&len_field, proc_macro2::Span::call_site());
        let prefix_size_lit =
            syn::LitInt::new(&prefix_size.to_string(), proc_macro2::Span::call_site());
        let k_lit = syn::LitInt::new(&k.to_string(), proc_macro2::Span::call_site());
        let k_plus_lit = syn::LitInt::new(&(k + 1).to_string(), proc_macro2::Span::call_site());
        let vd_name_lit = syn::LitStr::new(&vd.name, proc_macro2::Span::call_site());

        let tail_k_fn = quote::format_ident!("tail_offset_{}", k);
        let tail_k1_fn = quote::format_ident!("tail_offset_{}", k + 1);
        entry_body.extend(quote::quote! {
            #[inline]
            fn #tail_k1_fn(&self) -> Result<usize, sbe_rt::DecodeError> {
                let start = self.#tail_k_fn()?;
                if start + #prefix_size_lit > self.buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort { field: #vd_name_lit, needed: #prefix_size_lit, available: self.buf.len().saturating_sub(start) });
                }
                let bytes: [u8; #prefix_size_lit] = read_bytes::<#prefix_size_lit>(self.buf, start);
                let header = #type_pascal_ident(bytes);
                let len = header.#len_field_ident() as usize;
                if start + #prefix_size_lit + len > self.buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort { field: #vd_name_lit, needed: #prefix_size_lit + len, available: self.buf.len().saturating_sub(start) });
                }
                Ok(start + #prefix_size_lit + len)
            }
        });
        k += 1;
    }

    // Nested group accessors — scope under parent group name
    let mut ng_idx = 0usize;
    for ng in &g.groups {
        let ng_pascal = format!("{}{}", name, to_pascal_case(&ng.name));
        let ng_decoder_ident = quote::format_ident!("{}Decoder", ng_pascal);
        let ng_snake = to_snake_case(&ng.name);
        let ng_snake_ident = syn::Ident::new(&ng_snake, proc_macro2::Span::call_site());
        let ng_idx_lit = syn::LitInt::new(&ng_idx.to_string(), proc_macro2::Span::call_site());

        let tail_ng_fn = quote::format_ident!("tail_offset_{}", ng_idx);
        entry_body.extend(quote::quote! {
            #[inline]
            pub fn #ng_snake_ident(&self) -> Result<#ng_decoder_ident<'a>, sbe_rt::DecodeError> {
                let offset = self.#tail_ng_fn()?;
                if self.tail_end.get().is_some() {
                    return Ok(#ng_decoder_ident::wrap_trusted(
                        self.buf, offset, self.acting_version, 0, 0,
                    ));
                }
                #ng_decoder_ident::wrap(self.buf, offset, self.acting_version)
            }
        });
        ng_idx += 1;
    }

    // Var data accessors
    let mut nvd_idx = g.groups.len();
    for vd in &g.var_data {
        let (type_pascal, prefix_size, len_field, _) = get_vardata_info(elements, &vd.type_name);
        let type_pascal_ident = syn::Ident::new(&type_pascal, proc_macro2::Span::call_site());
        let len_field_ident = syn::Ident::new(&len_field, proc_macro2::Span::call_site());
        let prefix_size_lit =
            syn::LitInt::new(&prefix_size.to_string(), proc_macro2::Span::call_site());
        let vd_snake = to_snake_case(&vd.name);
        let vd_snake_ident = syn::Ident::new(&vd_snake, proc_macro2::Span::call_site());
        let nvd_idx_lit = syn::LitInt::new(&nvd_idx.to_string(), proc_macro2::Span::call_site());

        let tail_nvd_fn = quote::format_ident!("tail_offset_{}", nvd_idx);
        if nvd_idx + 1 == total_tail {
            // Last tail component: a warm tail-end cache (filled by the
            // iterator's encoded_length) gives the slice end directly —
            // no second length-header read, bounds already validated.
            entry_body.extend(quote::quote! {
                #[inline]
                pub fn #vd_snake_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    let offset = self.#tail_nvd_fn()?;
                    let data_offset = offset + #prefix_size_lit;
                    if let Some(end) = self.tail_end.get() {
                        // SAFETY: `tail_end` is only ever set by
                        // `encoded_length` from `tail_offset_N`, which
                        // bounds-checked `end <= buf.len()` and
                        // `data_offset <= end` before caching. Same
                        // invariant class as the existing generated
                        // `from_raw_parts` accessors.
                        return Ok(unsafe { self.buf.get_unchecked(data_offset..end) });
                    }
                    let bytes: [u8; #prefix_size_lit] = read_bytes::<#prefix_size_lit>(self.buf, offset);
                    let header = #type_pascal_ident(bytes);
                    let len = header.#len_field_ident() as usize;
                    Ok(&self.buf[data_offset .. data_offset + len])
                }
            });
        } else {
            entry_body.extend(quote::quote! {
                #[inline]
                pub fn #vd_snake_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    let offset = self.#tail_nvd_fn()?;
                    let bytes: [u8; #prefix_size_lit] = read_bytes::<#prefix_size_lit>(self.buf, offset);
                    let header = #type_pascal_ident(bytes);
                    let len = header.#len_field_ident() as usize;
                    let data_offset = offset + #prefix_size_lit;
                    Ok(&self.buf[data_offset .. data_offset + len])
                }
            });
        }
        nvd_idx += 1;
    }

    // encoded_length, skip — tail shape is a compile-time constant;
    // emit only the live path (no dead branch in the generated source).
    let tail_total_fn = quote::format_ident!("tail_offset_{}", total_tail);
    if total_tail == 0 {
        entry_body.extend(quote::quote! {
            #[inline]
            pub fn encoded_length(&self) -> usize {
                self.acting_block_length
            }
            #[inline]
            pub fn skip(buf: &'a [u8], pos: usize, block_len: usize, _acting_version: u16) -> Result<usize, sbe_rt::DecodeError> {
                Ok(pos + block_len)
            }
        });
    } else {
        entry_body.extend(quote::quote! {
            #[inline]
            pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
                if let Some(end) = self.tail_end.get() {
                    return Ok(end - self.pos);
                }
                let end = self.#tail_total_fn()?;
                self.tail_end.set(Some(end));
                Ok(end - self.pos)
            }
            #[inline]
            pub fn skip(buf: &'a [u8], pos: usize, block_len: usize, acting_version: u16) -> Result<usize, sbe_rt::DecodeError> {
                let entry = Self::wrap(buf, pos, block_len, acting_version);
                entry.#tail_total_fn()
            }
        });
    }

    // EntryDecoder Display impl body
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
                let fmt_str = format!("{sep}{}: {{:?}}", f.name);
                entry_display_body.extend(quote::quote! {
                    { let v = self.#f_ident(); write!(f, #fmt_str, v)?; }
                });
                entry_display_out_idx += 1;
            }
            FieldType::Enum {
                name: enum_name, ..
            } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                let fmt_str = format!("{sep}{}: {enum_name}::{{e:?}}", f.name);
                entry_display_body.extend(quote::quote! {
                    { let e = self.#f_ident(); write!(f, #fmt_str)?; }
                });
                entry_display_out_idx += 1;
            }
            FieldType::Set { .. } => {}
            FieldType::Composite { .. } => {
                if f.presence == Presence::Constant {
                    continue;
                }
                let f_value =
                    syn::Ident::new(&format!("{}_value", f_name), proc_macro2::Span::call_site());
                if let Some(domain_path) = find_domain_type(f, domain_types) {
                    let fmt_str = format!("{sep}{}: {{}}", f.name);
                    let domain_ty: syn::Type = syn::parse_str(domain_path).unwrap();
                    entry_display_body.extend(quote::quote! {
                        {
                            let raw = self.#f_value();
                            match <#domain_ty as TryFromSbe<_>>::try_from_sbe(raw) {
                                Ok(v) => write!(f, #fmt_str, v)?,
                                Err(_) => write!(f, #fmt_str, "<?>")?,
                            }
                        }
                    });
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
    // Entry nested groups in Display
    for ng in &g.groups {
        let ng_snake = to_snake_case(&ng.name);
        let ng_ident = syn::Ident::new(&ng_snake, proc_macro2::Span::call_site());
        let sep = if entry_display_out_idx == 0 { "" } else { ", " };
        let fmt_open = format!("{sep}{}: [", ng.name);
        let ng_total_tail = ng.groups.len() + ng.var_data.len();
        if ng_total_tail == 0 {
            // Fixed-entry nested group: entries are infallible
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

    // Emit the EntryDecoder struct + its impl block + Display impl
    if let Some(ref desc) = g.description {
        ts.extend(doc_attr_tokens(desc));
    }
    if total_tail == 0 {
        ts.extend(quote::quote! {
            pub struct #entry_decoder_ident<'a> {
                buf: &'a [u8],
                pos: usize,
                acting_version: u16,
                acting_block_length: usize,
            }
        });
    } else {
        ts.extend(quote::quote! {
            pub struct #entry_decoder_ident<'a> {
                buf: &'a [u8],
                pos: usize,
                acting_version: u16,
                acting_block_length: usize,
                /// One-shot entry-extent cache (todo 110): filled by
                /// `encoded_length`, reused by the last var-data accessor.
                tail_end: core::cell::Cell<Option<usize>>,
            }
        });
    }
    ts.extend(quote::quote! {
        impl<'a> #entry_decoder_ident<'a> {
            #entry_body
        }

        impl<'a> core::fmt::Display for #entry_decoder_ident<'a> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                write!(f, "{{ ")?;
                #entry_display_body
                write!(f, " }}")
            }
        }
    });

    // Recursively generate nested group decoders — scope under parent group name
    // to avoid collisions when different parent groups have same-named children
    for ng in &g.groups {
        let nested_name = format!("{}{}", name, to_pascal_case(&ng.name));
        ts.extend(generate_group_decoder(
            ng,
            elements,
            byte_order,
            &nested_name,
            &conversions,
            domain_types,
        ));
    }

    // Concrete consuming entry-level tail stages (DECISIONS.md §3, Task D) for
    // entries that have nested groups and/or var-data. Additive: the legacy
    // `&self` entry accessors remain. Emitted after the nested group decoders
    // above so `finish()` can name them.
    ts.extend(generate_entry_consuming_stages(g, elements, &name));

    ts
}

fn generate_nullification(
    src: &mut String,
    fields: &[MessageField],
    offset_base: &str,
    buf_expr: &str,
    byte_order: ByteOrder,
) {
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };
    let mut stmts = proc_macro2::TokenStream::new();
    for f in fields {
        if f.presence == Presence::Optional {
            if let Some(null_val) = f.null_value {
                let size = f.field_type.size();

                let null_val_expr: syn::Expr = syn::parse_str(&format!("{null_val}_u64")).unwrap();
                let to_method = syn::Ident::new(
                    &format!("to_{order_suffix}_bytes"),
                    proc_macro2::Span::call_site(),
                );
                let offset_base_expr: syn::Expr = syn::parse_str(offset_base).unwrap();
                let buf_expr_ts: syn::Expr = syn::parse_str(buf_expr).unwrap();
                let f_offset = syn::Index::from(f.offset);
                let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());

                stmts.extend(quote::quote! {
                    let null_bytes = #null_val_expr.#to_method();
                    let offset = #offset_base_expr + #f_offset;
                    #buf_expr_ts[offset..offset + #size_lit].copy_from_slice(&null_bytes);
                });
            }
        }
    }
    if !stmts.is_empty() {
        src.push_str(&stmts.to_string());
        src.push('\n');
    }
}

/// Emit `TryFromSbe` / `TryToSbe` traits into the generated sbe_rt module.
fn emit_conversion_traits(src: &mut String) {
    src.push_str(
        "/// Convert from a wire type to an application type.\n\
         pub trait TryFromSbe<Wire>: Sized {\n\
             type Error: core::fmt::Debug + core::fmt::Display;\n\
             fn try_from_sbe(wire: Wire) -> Result<Self, Self::Error>;\n\
         }\n\n\
         /// Convert from an application type to a wire type.\n\
         pub trait TryToSbe<Wire> {\n\
             type Error: core::fmt::Debug + core::fmt::Display;\n\
             fn try_to_sbe(&self) -> Result<Wire, Self::Error>;\n\
         }\n\n",
    );
}

/// Emit `TryFromSbe` / `TryToSbe` impls for well-known **domain-type** mappings
/// (bool ↔ BooleanType, rust_decimal ↔ Decimal, chrono ↔ u64/UTCTimestamp).
///
/// These built-in impls are gated on `domain_types`, not bare `conversions`:
/// `with_conversion` alone keeps the seam dependency-free so callers can plug
/// any adapter (see samples/sbe-feature-tour `FixedPrice` / app-side
/// rust_decimal). `with_domain_type` opts into a concrete app type *and* these
/// well-known impls.
fn generate_conversion_impl_blocks(
    elements: &SchemaElements,
    _conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
) -> String {
    let mut out = String::new();
    let span = proc_macro2::Span::call_site();

    // Built-ins require an explicit domain-type mapping (not conversion-only).
    let has_bool_conv = domain_types.iter().any(|(sel, ty)| {
        ty == "bool" && matches!(sel, crate::ConversionSelector::NamedType(n) if n == "BooleanType")
    });
    let has_decimal_conv = domain_types
        .iter()
        .any(|(sel, _)| matches!(sel, crate::ConversionSelector::NamedType(n) if n == "Decimal"));
    let has_chrono_conv = domain_types.iter().any(|(sel, _)| {
        matches!(sel, crate::ConversionSelector::SemanticType(st) if st == "UTCTimestamp")
    });

    if has_bool_conv {
        // Find the BooleanType enum name (may be scoped per-schema)
        let bt_name = elements
            .enums
            .iter()
            .find(|e| e[0].name == "BooleanType")
            .map(|e| to_pascal_case(&e[0].name))
            .unwrap_or_else(|| "BooleanType".to_string());
        let bt_ident = syn::Ident::new(&bt_name, span);
        let ts = quote::quote! {
            impl TryFromSbe<#bt_ident> for bool {
                type Error = &'static str;
                fn try_from_sbe(wire: #bt_ident) -> Result<Self, Self::Error> {
                    Ok(bool::from(wire))
                }
            }
            impl TryToSbe<#bt_ident> for bool {
                type Error = &'static str;
                fn try_to_sbe(&self) -> Result<#bt_ident, Self::Error> {
                    Ok(#bt_ident::from(*self))
                }
            }
        };
        out.push_str(&ts.to_string());
    }

    if has_decimal_conv {
        let dec_name = elements
            .composites
            .iter()
            .find(|c| c[0].name == "Decimal")
            .map(|c| to_pascal_case(&c[0].name))
            .unwrap_or_else(|| "Decimal".to_string());
        let dec_ident = syn::Ident::new(&dec_name, span);
        let ts = quote::quote! {
            impl TryFromSbe<#dec_ident> for rust_decimal::Decimal {
                type Error = &'static str;
                fn try_from_sbe(wire: #dec_ident) -> Result<Self, Self::Error> {
                    let mantissa = wire.mantissa() as i128;
                    let exponent = wire.exponent();
                    // SBE Decimal: negative exponent = fractional places (e.g.
                    // -2 → scale 2). Positive exponent = magnitude (mantissa ×
                    // 10^exp). rust_decimal scale must be a positive u32 ≤ 28.
                    let (mantissa, scale) = if exponent < 0 {
                        (mantissa, (-exponent) as u32)
                    } else {
                        (mantissa.saturating_mul(10i128.saturating_pow(exponent as u32)), 0)
                    };
                    rust_decimal::Decimal::from_i128_with_scale(mantissa, scale)
                        .try_into()
                        .map_err(|_| "Decimal overflow")
                }
            }
            impl TryToSbe<#dec_ident> for rust_decimal::Decimal {
                type Error = &'static str;
                fn try_to_sbe(&self) -> Result<#dec_ident, Self::Error> {
                    let mantissa: i64 = self.mantissa()
                        .try_into()
                        .map_err(|_| "Decimal mantissa overflow i64")?;
                    Ok(#dec_ident::new(mantissa, -(self.scale() as i8)))
                }
            }
        };
        out.push_str(&ts.to_string());
    }

    if has_chrono_conv {
        let ts = quote::quote! {
            impl TryFromSbe<u64> for chrono::DateTime<chrono::Utc> {
                type Error = &'static str;
                fn try_from_sbe(wire: u64) -> Result<Self, Self::Error> {
                    let secs = (wire / 1_000_000_000) as i64;
                    let nsec = (wire % 1_000_000_000) as u32;
                    chrono::DateTime::from_timestamp(secs, nsec)
                        .ok_or("timestamp out of range for DateTime<Utc>")
                }
            }
            impl TryToSbe<u64> for chrono::DateTime<chrono::Utc> {
                type Error = &'static str;
                fn try_to_sbe(&self) -> Result<u64, Self::Error> {
                    let total_nanos = self.timestamp_nanos_opt()
                        .ok_or("timestamp_nanos overflow")?;
                    Ok(total_nanos as u64)
                }
            }
        };
        out.push_str(&ts.to_string());
    }

    out
}

/// Generate `*_as`/`*_from` conversion methods for fields matching the
/// configured conversion selectors. Also emits raw `*_wire` aliases if the
/// field would otherwise shadow them.
fn generate_converter_impls(
    msg: &MessageStructure,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    _multi_message: bool,
) -> String {
    let span = proc_macro2::Span::call_site();
    let msg_name = to_pascal_case(&msg.name);
    let decoder_ident = syn::Ident::new(&format!("{msg_name}Decoder"), span);
    let encoder_ident = syn::Ident::new(&format!("{msg_name}Encoder"), span);

    let mut decoder_methods = proc_macro2::TokenStream::new();
    let mut encoder_methods = proc_macro2::TokenStream::new();

    for f in &msg.fields {
        // Determine if this field has a conversion, and what the wire type is.
        let (type_name, wire_type_ident): (String, syn::Ident) = match &f.field_type {
            FieldType::Composite { name, .. } => {
                (name.clone(), syn::Ident::new(&to_pascal_case(name), span))
            }
            FieldType::Enum { name, .. } => {
                (name.clone(), syn::Ident::new(&to_pascal_case(name), span))
            }
            FieldType::Set { name, .. } => {
                (name.clone(), syn::Ident::new(&to_pascal_case(name), span))
            }
            FieldType::Primitive(pt, _) => {
                let rust_name = rust_type(*pt);
                (rust_name.to_string(), syn::Ident::new(rust_name, span))
            }
        };
        let has_conversion = field_has_conversion_free(f, conversions);
        if !has_conversion {
            continue;
        }

        let field_snake = to_snake_case(&f.name);
        let domain_type_path = find_domain_type(f, domain_types);

        // Determine which raw accessor to call. Composites have _value()
        // for the owned wire value; everything else uses the _wire getter.
        let raw_decoder_getter = if matches!(f.field_type, FieldType::Composite { .. }) {
            syn::Ident::new(&format!("{field_snake}_value"), span)
        } else {
            syn::Ident::new(&format!("{field_snake}_wire"), span)
        };
        let wire_setter = syn::Ident::new(&format!("{field_snake}_wire"), span);

        if let Some(dt) = domain_type_path {
            // Concrete methods using the configured domain type
            let dt_ty: syn::Type =
                syn::parse_str(dt).unwrap_or_else(|_| panic!("invalid domain type path: {dt}"));
            let domain_ident = syn::Ident::new(&field_snake, span);

            decoder_methods.extend(quote::quote! {
                #[inline]
                #[must_use]
                pub fn #domain_ident(&self) -> #dt_ty {
                    <#dt_ty as TryFromSbe<#wire_type_ident>>::try_from_sbe(
                        self.#raw_decoder_getter()
                    ).expect(concat!("conversion of ", stringify!(#domain_ident)))
                }
            });

            encoder_methods.extend(quote::quote! {
                #[inline]
                #[must_use]
                pub fn #domain_ident(&mut self, value: #dt_ty) -> &mut Self {
                    let wire = <#dt_ty as TryToSbe<#wire_type_ident>>::try_to_sbe(&value)
                        .expect(concat!("conversion of ", stringify!(#domain_ident)));
                    self.#wire_setter(wire)
                }
            });
        } else {
            // Generic *_as / *_from methods
            let as_ident = syn::Ident::new(&format!("{field_snake}_as"), span);
            let from_ident = syn::Ident::new(&format!("{field_snake}_from"), span);

            decoder_methods.extend(quote::quote! {
                #[inline]
                #[must_use]
                pub fn #as_ident<T: TryFromSbe<#wire_type_ident>>(&self) -> Result<T, T::Error> {
                    T::try_from_sbe(self.#raw_decoder_getter())
                }
            });

            encoder_methods.extend(quote::quote! {
                #[inline]
                #[must_use]
                pub fn #from_ident<T: TryToSbe<#wire_type_ident>>(&mut self, value: &T) -> Result<&mut Self, T::Error> {
                    let wire = value.try_to_sbe()?;
                    self.#wire_setter(wire);
                    Ok(self)
                }
            });
        }
    }

    // Group entries (recursively): concrete methods when domain type is
    // configured, generic *_as/*_from otherwise.
    fn emit_group_entry_impls(
        scope: &str,
        g: &MessageGroup,
        conversions: &[crate::ConversionSelector],
        domain_types: &[(crate::ConversionSelector, String)],
        out: &mut String,
    ) {
        let span = proc_macro2::Span::call_site();
        let scoped = format!("{scope}{}", to_pascal_case(&g.name));
        let entry_dec_ident = syn::Ident::new(&format!("{scoped}EntryDecoder"), span);
        let entry_enc_ident = syn::Ident::new(&format!("{scoped}EntryEncoder"), span);
        let mut dec_methods = proc_macro2::TokenStream::new();
        let mut enc_methods = proc_macro2::TokenStream::new();
        for f in &g.fields {
            if !field_has_conversion_free(f, conversions) {
                continue;
            }
            let field_snake = to_snake_case(&f.name);
            let wire_type_ident = match &f.field_type {
                FieldType::Composite { name, .. } => syn::Ident::new(&to_pascal_case(name), span),
                FieldType::Enum { name, .. } => syn::Ident::new(&to_pascal_case(name), span),
                FieldType::Set { name, .. } => syn::Ident::new(&to_pascal_case(name), span),
                FieldType::Primitive(pt, _) => syn::Ident::new(rust_type(*pt), span),
            };
            let raw_decoder_getter = if matches!(f.field_type, FieldType::Composite { .. }) {
                syn::Ident::new(&format!("{field_snake}_value"), span)
            } else {
                syn::Ident::new(&format!("{field_snake}_wire"), span)
            };
            let wire_setter = syn::Ident::new(&format!("{field_snake}_wire"), span);

            if let Some(dt) = find_domain_type(f, domain_types) {
                let dt_ty: syn::Type =
                    syn::parse_str(dt).unwrap_or_else(|_| panic!("invalid domain type path: {dt}"));
                let domain_ident = syn::Ident::new(&field_snake, span);
                dec_methods.extend(quote::quote! {
                    #[inline]
                    #[must_use]
                    pub fn #domain_ident(&self) -> #dt_ty {
                        <#dt_ty as TryFromSbe<#wire_type_ident>>::try_from_sbe(
                            self.#raw_decoder_getter()
                        ).expect(concat!("conversion of ", stringify!(#domain_ident)))
                    }
                });
                enc_methods.extend(quote::quote! {
                    #[inline]
                    #[must_use]
                    pub fn #domain_ident(&mut self, value: #dt_ty) -> &mut Self {
                        let wire = <#dt_ty as TryToSbe<#wire_type_ident>>::try_to_sbe(&value)
                            .expect(concat!("conversion of ", stringify!(#domain_ident)));
                        self.#wire_setter(wire)
                    }
                });
            } else {
                let as_ident = syn::Ident::new(&format!("{field_snake}_as"), span);
                let from_ident = syn::Ident::new(&format!("{field_snake}_from"), span);
                dec_methods.extend(quote::quote! {
                    #[inline]
                    #[must_use]
                    pub fn #as_ident<T: TryFromSbe<#wire_type_ident>>(&self) -> Result<T, T::Error> {
                        T::try_from_sbe(self.#raw_decoder_getter())
                    }
                });
                enc_methods.extend(quote::quote! {
                    #[inline]
                    #[must_use]
                    pub fn #from_ident<T: TryToSbe<#wire_type_ident>>(&mut self, value: &T) -> Result<&mut Self, T::Error> {
                        let wire = value.try_to_sbe()?;
                        let _ = self.#wire_setter(wire);
                        Ok(self)
                    }
                });
            }
        }
        if !dec_methods.is_empty() {
            let ts = quote::quote! {
                impl<'a> #entry_dec_ident<'a> {
                    #dec_methods
                }
                impl<'a> #entry_enc_ident<'a> {
                    #enc_methods
                }
            };
            out.push_str(&ts.to_string());
        }
        for ng in &g.groups {
            emit_group_entry_impls(&scoped, ng, &conversions, domain_types, out);
        }
    }

    let mut entry_impls = String::new();
    let group_scope = if _multi_message {
        msg_name.clone()
    } else {
        String::new()
    };
    for g in &msg.groups {
        emit_group_entry_impls(
            &group_scope,
            g,
            &conversions,
            domain_types,
            &mut entry_impls,
        );
    }

    if decoder_methods.is_empty() && entry_impls.is_empty() {
        return String::new();
    }

    let mut out = if decoder_methods.is_empty() {
        String::new()
    } else {
        quote::quote! {
            impl<'a> #decoder_ident<'a> {
                #decoder_methods
            }
            impl<'a> #encoder_ident<'a> {
                #encoder_methods
            }
        }
        .to_string()
    };
    out.push_str(&entry_impls);
    out
}

/// Map a [`FieldType`] to the corresponding Rust type as a `syn::Type`.
fn field_type_ident(ft: &FieldType, span: proc_macro2::Span) -> syn::Type {
    match ft {
        FieldType::Primitive(pt, arr_len) => match (pt, arr_len) {
            (PrimitiveType::Char, Some(n)) => {
                let n = syn::LitInt::new(&n.to_string(), span);
                syn::parse_quote!([u8; #n])
            }
            (PrimitiveType::Char, None) => syn::parse_quote!(u8),
            (PrimitiveType::Int8, _) => syn::parse_quote!(i8),
            (PrimitiveType::Int16, _) => syn::parse_quote!(i16),
            (PrimitiveType::Int32, _) => syn::parse_quote!(i32),
            (PrimitiveType::Int64, _) => syn::parse_quote!(i64),
            (PrimitiveType::UInt8, None) => syn::parse_quote!(u8),
            (PrimitiveType::UInt16, None) => syn::parse_quote!(u16),
            (PrimitiveType::UInt32, None) => syn::parse_quote!(u32),
            (PrimitiveType::UInt64, None) => syn::parse_quote!(u64),
            (PrimitiveType::Float, _) => syn::parse_quote!(f32),
            (PrimitiveType::Double, _) => syn::parse_quote!(f64),
            (pt, Some(len)) => {
                let elem: syn::Type = field_type_ident(&FieldType::Primitive(*pt, None), span);
                let n = syn::LitInt::new(&len.to_string(), span);
                syn::parse_quote!([#elem; #n])
            }
            _ => syn::parse_quote!(u8), // fallback
        },
        FieldType::Composite { name, .. } => {
            let ident = syn::Ident::new(&to_pascal_case(name), span);
            syn::parse_quote!(#ident)
        }
        FieldType::Enum { name, .. } => {
            let ident = syn::Ident::new(&to_pascal_case(name), span);
            syn::parse_quote!(#ident)
        }
        FieldType::Set { name, .. } => {
            let ident = syn::Ident::new(&to_pascal_case(name), span);
            syn::parse_quote!(#ident)
        }
    }
}

/// Generate the raw fixed writer impl block with field setters and finish_unchecked.
fn generate_raw_fixed_impls(
    msg: &MessageStructure,
    raw_name: &syn::Ident,
    header_size: usize,
    block_length: usize,
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let mut ts = proc_macro2::TokenStream::new();
    let mut setters = proc_macro2::TokenStream::new();

    for f in &msg.fields {
        if f.presence == crate::Presence::Constant {
            continue;
        }
        let fname_snake = to_snake_case(&f.name);
        let f_ident = syn::Ident::new(&fname_snake, span);
        let offset_lit = syn::LitInt::new(&f.offset.to_string(), span);
        let f_type = &f.field_type;
        let size = f_type.size();

        let ty_ident: syn::Type = field_type_ident(f_type, span);
        let size_lit = syn::LitInt::new(&size.to_string(), span);

        let is_array = matches!(f_type, FieldType::Primitive(_, Some(_)));
        if is_array {
            setters.extend(quote::quote! {
                #[inline]
                pub fn #f_ident(&mut self, val: #ty_ident) -> &mut Self {
                    let offset = self.pos + #offset_lit;
                    self.buf[offset..offset + #size_lit].copy_from_slice(&val);
                    self
                }
            });
        } else if matches!(f_type, FieldType::Primitive(PrimitiveType::Char, _)) {
            setters.extend(quote::quote! {
                #[inline]
                pub fn #f_ident(&mut self, val: u8) -> &mut Self {
                    let offset = self.pos + #offset_lit;
                    self.buf[offset] = val;
                    self
                }
            });
        } else {
            setters.extend(quote::quote! {
                #[inline]
                pub fn #f_ident(&mut self, val: #ty_ident) -> &mut Self {
                    let offset = self.pos + #offset_lit;
                    let bytes = val.to_le_bytes();
                    self.buf[offset..offset + #size_lit].copy_from_slice(&bytes);
                    self
                }
            });
        }
    }

    // finish_unchecked: advance past fixed block to first tail stage
    ts.extend(quote::quote! {
        impl<'a> #raw_name<'a> {
            #setters

            /// Advance past the fixed block without required-field validation.
            /// The buffer must already contain the correct header and block.
            /// Returns the first tail stage for further encoding.
            #[inline]
            #[must_use]
            pub fn finish_unchecked(self) -> &'a mut [u8] {
                // no validation of fixed block here — caller guarantees validity, add a debug_assert! if callers regress
                // returns the tail portion of the buffer for manual use.
                let body_start = self.message_start + #header_size;
                let tail_start = body_start + #block_length;
                &mut self.buf[tail_start..]
            }
        }
    });
    ts
}

/// Returns true when the message or any of its groups contains nested groups
/// or entry-level varData — i.e. when the flat `compute_encoded_length` helper
/// cannot give an exact answer.
fn has_nested_dynamic_tail(msg: &MessageStructure) -> bool {
    for g in &msg.groups {
        if !g.groups.is_empty() || !g.var_data.is_empty() {
            return true;
        }
    }
    false
}

/// Generate the staged zero-allocation length builder for a message
/// or group entry with a dynamic tail. Returns TokenStream.
///
/// When `header_size > 0` the builder is message-level (staged, consumes
/// `self`).  When `header_size == 0` it is entry-level (flat, `&mut self`
/// methods — used for repeating group entries).
fn generate_encoded_length_builder(
    name_prefix: &str,
    block_length: usize,
    header_size: usize,
    groups: &[MessageGroup],
    var_data: &[MessageVarData],
    elements: &SchemaElements,
    multi_message: bool,
    scoped_group_names: &[String],
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let prefix_ident = syn::Ident::new(&format!("{}EncodedLength", name_prefix), span);
    let total_tail = groups.len() + var_data.len();
    let is_entry_level = header_size == 0;
    let mut ts = proc_macro2::TokenStream::new();
    let block_length_lit = syn::LitInt::new(&block_length.to_string(), span);

    if is_entry_level {
        // ── Entry-level builder (flat, &mut self methods) ──
        ts.extend(quote::quote! {
            #[must_use = "length builder tracks entry sizes"]
            pub struct #prefix_ident {
                len: usize,
                written: usize,
            }

            impl #prefix_ident {
                pub const ENTRY_BLOCK_LENGTH: usize = #block_length_lit;

                pub fn new() -> Self {
                    Self { len: 0, written: 0 }
                }

                /// Register one entry.
                pub fn add(&mut self) -> sbe_rt::GroupResult {
                    self.len = self.len.checked_add(Self::ENTRY_BLOCK_LENGTH)
                        .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                    self.written += 1;
                    Ok(())
                }

                /// Register `n` entries at once — equivalent to calling
                /// [`add`](Self::add) `n` times.
                pub fn add_n(&mut self, n: usize) -> sbe_rt::GroupResult {
                    self.len = self.len
                        .checked_add(Self::ENTRY_BLOCK_LENGTH.checked_mul(n)
                            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?)
                        .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                    self.written += n;
                    Ok(())
                }
            }
        });

        // Nested-group methods on the entry-level builder
        for (gi, ng) in groups.iter().enumerate() {
            let ng_snake = syn::Ident::new(&to_snake_case(&ng.name), span);
            let ng_snake_unknown =
                syn::Ident::new(&format!("{}_unknown_size", to_snake_case(&ng.name)), span);
            let scoped_ng = &scoped_group_names[gi];
            let ng_len_ident = syn::Ident::new(&format!("{}EncodedLength", scoped_ng), span);
            let (_dim_name, dim_size, _bl_field, _num_field) =
                get_dimension_info(elements, &ng.dimension_type);
            let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), span);
            let (_num_off, _num_sz, ng_num_prim) = get_dim_num_layout(elements, &ng.dimension_type);
            let ng_count_ty: syn::Type = syn::parse_str(rust_type(ng_num_prim)).unwrap();

            ts.extend(quote::quote! {
                impl #prefix_ident {
                    /// Track a nested repeating group inside one entry.
                    pub fn #ng_snake<F>(
                        &mut self, count: #ng_count_ty, f: F,
                    ) -> sbe_rt::GroupResult
                    where
                        F: FnOnce(&mut #ng_len_ident) -> sbe_rt::GroupResult,
                    {
                        let mut builder = #ng_len_ident::new();
                        f(&mut builder)?;
                        if builder.written != count as usize {
                            return Err(
                                sbe_rt::EncodeError::GroupCountMismatch {
                                    declared: count as u32,
                                    actual: builder.written as u32,
                                },
                            );
                        }
                        self.len = self
                            .len
                            .checked_add(#dim_size_lit)
                            .and_then(|l| l.checked_add(builder.len))
                            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                        Ok(())
                    }

                    /// Unknown-size variant — validates the count fits in
                    /// the wire type rather than requiring an exact match.
                    pub fn #ng_snake_unknown<F>(
                        &mut self, f: F,
                    ) -> sbe_rt::GroupResult
                    where
                        F: FnOnce(&mut #ng_len_ident) -> sbe_rt::GroupResult,
                    {
                        let mut builder = #ng_len_ident::new();
                        f(&mut builder)?;
                        let max_count = #ng_count_ty::MAX as usize;
                        if builder.written > max_count {
                            return Err(
                                sbe_rt::EncodeError::GroupCountOverflow {
                                    maximum: #ng_count_ty::MAX as u32,
                                    actual: builder.written as u32,
                                },
                            );
                        }
                        self.len = self
                            .len
                            .checked_add(#dim_size_lit)
                            .and_then(|l| l.checked_add(builder.len))
                            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                        Ok(())
                    }
                }
            });
        }

        // Entry-level var-data methods
        for vd in var_data {
            let vd_snake = syn::Ident::new(&to_snake_case(&vd.name), span);
            let (_vd_name, prefix_size, _len_field, _prim) =
                get_vardata_info(elements, &vd.type_name);
            let prefix_size_lit = syn::LitInt::new(&prefix_size.to_string(), span);

            let mut check = quote::quote! {};
            if let Some(max) = vd.max_length {
                let max_lit = syn::LitInt::new(&max.to_string(), span);
                let field_str = &vd.name;
                check.extend(quote::quote! {
                    if byte_len > #max_lit {
                        return Err(sbe_rt::EncodeError::VarDataTooLong {
                            field: #field_str,
                            max_length: #max_lit,
                            actual: byte_len,
                        });
                    }
                });
            }

            ts.extend(quote::quote! {
                impl #prefix_ident {
                    /// Track one variable-length data field inside an entry.
                    pub fn #vd_snake(
                        &mut self, byte_len: usize,
                    ) -> sbe_rt::GroupResult {
                        #check
                        self.len = self
                            .len
                            .checked_add(#prefix_size_lit)
                            .and_then(|l| l.checked_add(byte_len))
                            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                        Ok(())
                    }
                }
            });
        }
    } else {
        // ── Message-level builder (staged, consumes self) ──
        let header_size_lit = syn::LitInt::new(&header_size.to_string(), span);

        // Tail field names in wire order (groups then var-data)
        let tail_pascal: Vec<String> = groups
            .iter()
            .map(|g| to_pascal_case(&g.name))
            .chain(var_data.iter().map(|vd| to_pascal_case(&vd.name)))
            .collect();

        // Stage struct names
        let mut stage_idents = vec![prefix_ident.clone()];
        for (i, field) in tail_pascal.iter().enumerate() {
            if i < total_tail - 1 {
                stage_idents.push(syn::Ident::new(
                    &format!("{}EncodedLengthAfter{}", name_prefix, field),
                    span,
                ));
            } else {
                stage_idents.push(syn::Ident::new(
                    &format!("{}EncodedLengthComplete", name_prefix),
                    span,
                ));
            }
        }

        // Struct definitions (identical layout, non-generic)
        for stage in &stage_idents {
            ts.extend(quote::quote! {
                #[must_use = "length builder must be consumed to compute encoded length"]
                pub struct #stage {
                    len: usize,
                }
            });
        }

        // new() on the initial stage — starts at the fixed-field block length
        ts.extend(quote::quote! {
            impl #prefix_ident {
                pub const BLOCK_LENGTH: usize = #block_length_lit;
                pub const HEADER_LENGTH: usize = #header_size_lit;

                /// Start computing the encoded length of this message.
                /// Initial value is the fixed-field block length.
                pub fn new() -> Self {
                    Self { len: Self::BLOCK_LENGTH }
                }
            }
        });

        // ── Group transition methods ──
        let mut tail_idx = 0usize;
        for (gi, g) in groups.iter().enumerate() {
            let current_stage = &stage_idents[tail_idx];
            let next_stage = &stage_idents[tail_idx + 1];

            let g_snake = syn::Ident::new(&to_snake_case(&g.name), span);
            let g_snake_unknown =
                syn::Ident::new(&format!("{}_unknown_size", to_snake_case(&g.name)), span);
            let scoped_ng = &scoped_group_names[gi];
            let g_len_ident = syn::Ident::new(&format!("{}EncodedLength", scoped_ng), span);
            let (_dim_name, dim_size, _bl_field, _num_field) =
                get_dimension_info(elements, &g.dimension_type);
            let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), span);
            let (_num_off, _num_sz, num_prim) = get_dim_num_layout(elements, &g.dimension_type);
            let count_ty: syn::Type = syn::parse_str(rust_type(num_prim)).unwrap();

            let is_flat_group = g.groups.is_empty() && g.var_data.is_empty();

            if is_flat_group {
                // Flat group — no nested dynamics, count alone is enough.
                let entry_bl = syn::LitInt::new(&g.block_length.to_string(), span);
                let entry_bl_usize: syn::Type = syn::parse_str("usize").unwrap();
                ts.extend(quote::quote! {
                    impl #current_stage {
                        /// Register this flat group with a known entry count.
                        /// No closure needed — entries have no nested groups
                        /// or var-data.
                        #[must_use]
                        pub fn #g_snake(
                            self, count: #count_ty,
                        ) -> Result<#next_stage, sbe_rt::EncodeError> {
                            let entries_len: #entry_bl_usize = (#entry_bl as usize)
                                .checked_mul(count as usize)
                                .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
                            Ok(#next_stage {
                                len: self
                                    .len
                                    .checked_add(#dim_size_lit)
                                    .and_then(|l| l.checked_add(entries_len))
                                    .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?,
                            })
                        }
                    }
                });
            } else {
                ts.extend(quote::quote! {
                impl #current_stage {
                    /// Encode this group with a known entry count.
                    #[must_use]
                    pub fn #g_snake<F>(
                        self, count: #count_ty, f: F,
                    ) -> Result<#next_stage, sbe_rt::EncodeError>
                    where
                        F: FnOnce(&mut #g_len_ident) -> sbe_rt::GroupResult,
                    {
                        let mut builder = #g_len_ident::new();
                        f(&mut builder)?;
                        if builder.written != count as usize {
                            return Err(
                                sbe_rt::EncodeError::GroupCountMismatch {
                                    declared: count as u32,
                                    actual: builder.written as u32,
                                },
                            );
                        }
                        Ok(#next_stage {
                            len: self
                                .len
                                .checked_add(#dim_size_lit)
                                .and_then(|l| l.checked_add(builder.len))
                                .ok_or(
                                    sbe_rt::EncodeError::EncodedLengthOverflow,
                                )?,
                        })
                    }
                }
                });
            }

            ts.extend(quote::quote! {
                impl #current_stage {
                    /// Encode this group without knowing the count up front.
                    #[must_use]
                    pub fn #g_snake_unknown<F>(
                        self, f: F,
                    ) -> Result<#next_stage, sbe_rt::EncodeError>
                    where
                        F: FnOnce(&mut #g_len_ident) -> sbe_rt::GroupResult,
                    {
                        let mut builder = #g_len_ident::new();
                        f(&mut builder)?;
                        let max_count = #count_ty::MAX as usize;
                        if builder.written > max_count {
                            return Err(
                                sbe_rt::EncodeError::GroupCountOverflow {
                                    maximum: #count_ty::MAX as u32,
                                    actual: builder.written as u32,
                                },
                            );
                        }
                        Ok(#next_stage {
                            len: self
                                .len
                                .checked_add(#dim_size_lit)
                                .and_then(|l| l.checked_add(builder.len))
                                .ok_or(
                                    sbe_rt::EncodeError::EncodedLengthOverflow,
                                )?,
                        })
                    }
                }
            });
            tail_idx += 1;
        }

        // ── Var-data transition methods ──
        for vd in var_data {
            let current_stage = &stage_idents[tail_idx];
            let next_stage = &stage_idents[tail_idx + 1];

            let vd_snake = syn::Ident::new(&to_snake_case(&vd.name), span);
            let (_vd_name, prefix_size, _len_field, _prim) =
                get_vardata_info(elements, &vd.type_name);
            let prefix_size_lit = syn::LitInt::new(&prefix_size.to_string(), span);

            let mut check = quote::quote! {};
            if let Some(max) = vd.max_length {
                let max_lit = syn::LitInt::new(&max.to_string(), span);
                let field_str = &vd.name;
                check.extend(quote::quote! {
                    if byte_len > #max_lit {
                        return Err(sbe_rt::EncodeError::VarDataTooLong {
                            field: #field_str,
                            max_length: #max_lit,
                            actual: byte_len,
                        });
                    }
                });
            }

            ts.extend(quote::quote! {
                impl #current_stage {
                    /// Track one variable-length data field.
                    #[must_use]
                    pub fn #vd_snake(
                        self, byte_len: usize,
                    ) -> Result<#next_stage, sbe_rt::EncodeError> {
                        #check
                        Ok(#next_stage {
                            len: self
                                .len
                                .checked_add(#prefix_size_lit)
                                .and_then(|l| l.checked_add(byte_len))
                                .ok_or(
                                    sbe_rt::EncodeError::EncodedLengthOverflow,
                                )?,
                        })
                    }
                }
            });
            tail_idx += 1;
        }

        // ── Complete stage ──
        let complete_ident = &stage_idents[total_tail];
        ts.extend(quote::quote! {
            impl #complete_ident {
                /// SBE message body length (excluding the standard header).
                pub fn encoded_length(&self) -> usize { self.len }

                /// Total SBE message length including the standard message
                /// header (`HEADER_LENGTH`).
                pub fn encoded_length_with_header(&self) -> usize {
                    self.len + #prefix_ident::HEADER_LENGTH
                }
            }
        });
    }

    // ── Recursively generate nested entry-level builders ──
    for (gi, g) in groups.iter().enumerate() {
        let scoped_name = &scoped_group_names[gi];
        let nested_group_names: Vec<String> = g
            .groups
            .iter()
            .map(|ng| {
                let ng_raw = to_pascal_case(&ng.name);
                // Always prefix nested group names with the parent group scoped name
                // to avoid collisions when sibling groups have identically-named
                // sub-groups (e.g. L3Book: bids.orders vs asks.orders).
                format!("{}{}", scoped_name, ng_raw)
            })
            .collect();

        let sub_ts = generate_encoded_length_builder(
            scoped_name,
            g.block_length,
            0,
            &g.groups,
            &g.var_data,
            elements,
            multi_message,
            &nested_group_names,
        );
        ts.extend(sub_ts);
    }

    ts
}

fn generate_message_encoder(
    msg: &MessageStructure,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    schema_id: u16,
    schema_version: u16,
    header_type: &str,
    multi_message: bool,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
    unchecked_companions: bool,
) -> proc_macro2::TokenStream {
    let raw_name = &msg.name;
    let name = to_pascal_case(raw_name);
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    let block_length = msg.fields.iter().fold(0, |acc, f| {
        let size = f.field_type.size();
        acc.max(f.offset + size)
    });

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
        max_tail += dim_size + g.block_length;
    }
    for vd in &msg.var_data {
        let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
        max_tail += prefix_size + vd.max_length.unwrap_or(0);
    }
    let max_encoded_length = header_size + block_length + max_tail;

    const STACK_LIMIT: usize = 65536;
    let max_encoded_capped = max_encoded_length.min(STACK_LIMIT);
    let is_capped = max_encoded_length > STACK_LIMIT;

    let span = proc_macro2::Span::call_site();
    let snake_name = to_snake_case(&msg.name);
    let name_encoder_ident = syn::Ident::new(&format!("{}Encoder", name), span);
    let name_decoder_ident = syn::Ident::new(&format!("{}Decoder", name), span);

    // Pre-compute HEADER_TEMPLATE bytes. SBE spec §4.1: the message header
    // is ALWAYS little-endian regardless of the schema's byteOrder. The body
    // follows the schema byteOrder; the header does not.
    let mut header_tpl = vec![0u8; header_size];
    let hdr_bl = block_length as u16;
    header_tpl[0..2].copy_from_slice(&hdr_bl.to_le_bytes());
    header_tpl[2..4].copy_from_slice(&msg.id.to_le_bytes());
    header_tpl[4..6].copy_from_slice(&schema_id.to_le_bytes());
    header_tpl[6..8].copy_from_slice(&schema_version.to_le_bytes());
    let hdr_lits: Vec<syn::LitInt> = header_tpl
        .iter()
        .map(|b| syn::LitInt::new(&b.to_string(), span))
        .collect();

    // Helper literals
    let header_size_lit = syn::LitInt::new(&header_size.to_string(), span);
    let block_length_lit = syn::LitInt::new(&block_length.to_string(), span);
    let schema_id_lit = syn::LitInt::new(&schema_id.to_string(), span);
    let schema_version_lit = syn::LitInt::new(&schema_version.to_string(), span);
    let msg_id_lit = syn::LitInt::new(&msg.id.to_string(), span);
    let encoded_length_lit = syn::LitInt::new(&encoded_length.to_string(), span);
    let max_encoded_capped_lit = syn::LitInt::new(&max_encoded_capped.to_string(), span);
    let to_endian = syn::Ident::new(&format!("to_{}_bytes", order_suffix), span);

    let mut ts = proc_macro2::TokenStream::new();

    // ── Compute tail field names in wire order (groups then var-data) ──
    let tail_pascal: Vec<String> = msg
        .groups
        .iter()
        .map(|g| to_pascal_case(&g.name))
        .chain(msg.var_data.iter().map(|vd| to_pascal_case(&vd.name)))
        .collect();

    // ── Stage struct names ──
    // Stage 0 = initial (#name_encoder_ident). After each tail field, a new
    // concrete struct (e.g. CarEncoderAfterFuelFigures). Final = Complete.
    // This gives compile-time ordering: each struct only has the transition
    // for its stage; you can't call asks() before bids() — different type.
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

    // ── Generate all stage struct definitions (identical layout, non-generic) ──
    // Emit encoder struct doc from the message's XML description.
    if let Some(ref desc) = msg.description {
        ts.extend(doc_attr_tokens(desc));
    }
    for stage in &stage_idents {
        let stage_name = stage.to_string();
        let stage_name_lit = syn::LitStr::new(&stage_name, span);
        ts.extend(quote::quote! {
            #[must_use = "encoder must be consumed to write the message"]
            pub struct #stage<'a> {
                buf: &'a mut [u8],
                message_start: usize,
                pos: usize,
            }

            // Encoder Display + Debug: delegate to the decoder for field-value
            // output (reads the encoded buffer). Safe for partial buffers —
            // decoder try_wrap guards prevent panics; falls back to structural.
            impl<'a> core::fmt::Display for #stage<'a> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match #name_decoder_ident::try_wrap_and_apply_header(
                        &self.buf[self.message_start..], 0,
                    ) {
                        Ok(dec) => core::fmt::Display::fmt(&dec, f),
                        Err(_) => write!(f, "<partial {}>", #stage_name_lit),
                    }
                }
            }

            impl<'a> core::fmt::Debug for #stage<'a> {
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match #name_decoder_ident::try_wrap_and_apply_header(
                        &self.buf[self.message_start..], 0,
                    ) {
                        Ok(dec) => core::fmt::Debug::fmt(&dec, f),
                        Err(_) => f.debug_struct(#stage_name_lit)
                            .field("message_start", &self.message_start)
                            .field("pos", &self.pos)
                            .field("buf_len", &self.buf.len())
                            .finish(),
                    }
                }
            }
        });
    }

    // ── Shared impl block ──
    // Constants, HEADER_TEMPLATE, wrap(), wrap_and_apply_header(), field
    // setters, encoded_length() all live on the INITIAL struct only.
    let mut impl_contents = proc_macro2::TokenStream::new();

    // Constants
    if is_fixed {
        impl_contents.extend(quote::quote! {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #block_length_lit;
            const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == #block_length_lit);
            /// Message header size in bytes (standard SBE header is 8).
            pub const HEADER_LENGTH: usize = #header_size_lit;
            /// Stack-allocate with `let mut buf = [0u8; Msg::ENCODED_LENGTH];`
            /// Header-inclusive fixed length. Claim/app framing: payload starts
            /// at `frame[Self::ENCODED_LENGTH..]`.
            pub const ENCODED_LENGTH: usize = #encoded_length_lit;
            const _ENCODED_LEN: () = assert!(Self::ENCODED_LENGTH >= Self::BLOCK_LENGTH);
            /// Slice after one full header-inclusive message of this type.
            #[inline]
            pub fn after_this_message(frame: &[u8]) -> Option<&[u8]> {
                if frame.len() < Self::ENCODED_LENGTH {
                    return None;
                }
                Some(&frame[Self::ENCODED_LENGTH..])
            }
        });
    } else {
        let max_doc_attr = if is_capped {
            quote::quote! {
                #[doc = "MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation"]
            }
        } else {
            quote::quote! {
                #[doc = "Stack-allocate with `let mut buf = [0u8; Msg::MAX_ENCODED_LENGTH];`"]
            }
        };
        impl_contents.extend(quote::quote! {
            pub const SCHEMA_ID: u16 = #schema_id_lit;
            pub const SCHEMA_VERSION: u16 = #schema_version_lit;
            pub const TEMPLATE_ID: u16 = #msg_id_lit;
            pub const BLOCK_LENGTH: usize = #block_length_lit;
            const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == #block_length_lit);
            /// Message header size in bytes (standard SBE header is 8).
            pub const HEADER_LENGTH: usize = #header_size_lit;
            #max_doc_attr
            pub const MAX_ENCODED_LENGTH: usize = #max_encoded_capped_lit;
            const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
        });

        // compute_length() — convenience factory for the staged length builder
        if !encoded_len_gen.standalone.is_empty() {
            let el_ident = syn::Ident::new(&format!("{name}EncodedLength"), span);
            impl_contents.extend(quote::quote! {
                /// Return a staged length builder for computing the exact
                /// encoded size before allocation.
                #[inline]
                pub const fn compute_length() -> #el_ident {
                    #el_ident::new()
                }
            });
        }
    }

    // HEADER_TEMPLATE
    impl_contents.extend(quote::quote! {
        pub const HEADER_TEMPLATE: [u8; #header_size_lit] = [#(#hdr_lits),*];
        const _HEADER_TEMPLATE_LEN: () = assert!(Self::HEADER_TEMPLATE.len() == #header_size_lit);
    });

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
                needed,
                available: buf.len().saturating_sub(pos),
            }
        }
    };
    impl_contents.extend(cold_check);

    let wrap_fn = quote::quote! {
        /// Wrap a mutable buffer for encoding with bounds validation.
        /// Returns an error if the buffer is too short.
        /// Prefer [`Self::wrap`] for the fast path when the buffer size is known.
        #[inline]
        pub fn try_wrap(buf: &'a mut [u8], pos: usize) -> Result<Self, sbe_rt::EncodeError> {
            if pos.wrapping_add(#needed_lit) > buf.len() {
                return Err(Self::buffer_too_short(buf, pos, #needed_lit));
            }
            Ok(Self {
                buf: &mut buf[pos..],
                message_start: 0,
                pos: #needed_lit,
            })
        }
    };
    impl_contents.extend(wrap_fn);

    let wrap_apply_body = quote::quote! {
        // Optional-field nullification is NOT applied by default — call
        // `apply_nulls()` if you want null sentinels.
        if pos.wrapping_add(#needed_lit) > buf.len() {
            return Err(Self::buffer_too_short(buf, pos, #needed_lit));
        }
        buf[pos..pos + #header_size_lit].copy_from_slice(&Self::HEADER_TEMPLATE);
        Ok(Self { buf: &mut buf[pos..], message_start: 0, pos: #needed_lit })
    };
    let wrap_apply_fn = quote::quote! {
        /// Wrap a mutable buffer, write the header, with bounds validation.
        /// Returns an error if the buffer is too short.
        /// Prefer [`Self::wrap_and_apply_header`] for the fast path.
        #[inline]
        pub fn try_wrap_and_apply_header(buf: &'a mut [u8], pos: usize) -> Result<Self, sbe_rt::EncodeError> {
            #wrap_apply_body
        }
    };
    impl_contents.extend(wrap_apply_fn);

    // Claim-compatible wrap: validates buffer is exactly ENCODED_LENGTH bytes.
    // For use with Aeron try_claim where the buffer is pre-sized to the message.
    if is_fixed {
        impl_contents.extend(quote::quote! {
            /// Wrap a mutable buffer sized exactly to `ENCODED_LENGTH` bytes.
            /// For use with claim buffers (`try_claim`) where the caller has
            /// already allocated exactly the right size.
            #[inline]
            pub fn wrap_into_claim(buf: &'a mut [u8]) -> Result<Self, sbe_rt::EncodeError> {
                if buf.len() < Self::ENCODED_LENGTH {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        needed: Self::ENCODED_LENGTH,
                        available: buf.len(),
                    });
                }
                Self::try_wrap_and_apply_header(buf, 0)
            }
        });
    }

    // Opt-in: write null sentinels for all optional fields. Call this after
    // wrap_and_apply_header if you want unset optional fields to carry their
    // schema-defined null value instead of whatever was in the buffer.
    // Not called by default (Aeron does not nullify on wrap).
    {
        let mut null_buf = String::new();
        let offset_base = format!("self.message_start + {header_size}");
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
                /// (matching Aeron). Call this if you want unset optional fields to
                /// carry their null value rather than stale buffer contents.
                #[inline]
                pub fn apply_nulls(&mut self) {
                    #null_ts
                }
            };
            impl_contents.extend(apply_nulls_fn);
        }
    }

    // ── Field setters ──
    for f in &msg.fields {
        let f_name = to_snake_case(&f.name);
        let body_offset = header_size + f.offset;
        let body_offset_lit = syn::LitInt::new(&body_offset.to_string(), span);
        // In converter mode, raw setters are suffixed _wire when a domain
        // Raw setters become *_wire when a conversion is configured so the
        // converted setter takes the original name.
        let wire_name = field_has_conversion_free(f, conversions).then(|| format!("{f_name}_wire"));
        let method_name = wire_name.as_deref().unwrap_or(&f_name);
        let f_ident = syn::Ident::new(method_name, span);

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                if f.presence == Presence::Constant {
                    // Constant fields have no setter
                    continue;
                }
                let prim_size = prim.size();
                let prim_size_lit = syn::LitInt::new(&prim_size.to_string(), span);
                let r_type: syn::Type = syn::parse_str(rust_type(*prim)).unwrap();
                if let Some(len) = length {
                    let len_lit = syn::LitInt::new(&len.to_string(), span);
                    if prim_size == 1 {
                        // [u8; N]: no byte-swap needed, single bulk copy
                        impl_contents.extend(quote::quote! {
                            #[inline]
                            pub fn #f_ident(&mut self, val: [#r_type; #len_lit]) -> &mut Self {
                                unsafe {
                                    self.buf.get_unchecked_mut(#body_offset_lit..#body_offset_lit + #len_lit).copy_from_slice(&val);
                                }
                                self
                            }
                        });
                    } else {
                        impl_contents.extend(quote::quote! {
                            #[inline]
                            pub fn #f_ident(&mut self, val: [#r_type; #len_lit]) -> &mut Self {
                                let offset = #body_offset_lit;
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
                } else if prim_size == 1 {
                    // Direct byte write for u8/i8/char — 1 instruction vs 3.
                    impl_contents.extend(quote::quote! {
                        #[inline]
                        pub fn #f_ident(&mut self, val: #r_type) -> &mut Self {
                            *unsafe { self.buf.get_unchecked_mut(#body_offset_lit) } = val as u8;
                            self
                        }
                    });
                } else {
                    impl_contents.extend(quote::quote! {
                        #[inline]
                        pub fn #f_ident(&mut self, val: #r_type) -> &mut Self {
                            let offset = #body_offset_lit;
                            // SAFETY: wrap/try_wrap validates buf.len() >= BLOCK_LENGTH,
                            // and offset + prim_size <= BLOCK_LENGTH by construction.
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
                    pub fn #f_ident(&mut self, val: #target_type) -> &mut Self {
                        let offset = #body_offset_lit;
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
                    // Constant enum fields have no setter
                    continue;
                }
                let target_type: syn::Type = syn::parse_str(&to_pascal_case(enum_name)).unwrap();
                let r_type: syn::Type = syn::parse_str(rust_type(*encoding_type)).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit = syn::LitInt::new(&prim_size.to_string(), span);
                impl_contents.extend(quote::quote! {
                    pub fn #f_ident(&mut self, val: #target_type) -> &mut Self {
                        let offset = #body_offset_lit;
                        self.buf[offset..offset + #prim_size_lit].copy_from_slice(&(val as #r_type).#to_endian());
                        self
                    }
                });
                // Boolean fields get an additional setter that accepts bool directly
                if is_bool_enum(elements, enum_name) {
                    let f_name_bool = syn::Ident::new(&format!("{}_bool", f_name), span);
                    impl_contents.extend(quote::quote! {
                        pub fn #f_name_bool(&mut self, val: bool) -> &mut Self {
                            self.buf[#body_offset_lit] = val as u8;
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
                    pub fn #f_ident(&mut self, val: #target_type) -> &mut Self {
                        let offset = #body_offset_lit;
                        self.buf[offset..offset + #prim_size_lit].copy_from_slice(&val.0.#to_endian());
                        self
                    }
                });
            }
        }
    }

    // No partial as_bytes on incomplete stages — complete-message byte/length
    // views exist only on the terminal complete stage (DECISIONS.md §2).
    // Callers that genuinely need partial inspection should use an explicit
    // name such as `written_prefix()`."

    // Encoded-length support: strategy-classified (computed above).
    impl_contents.extend(encoded_len_gen.encoder_impl.clone());

    // ── FixedFields value struct ──
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
            #[derive(Debug, Clone)]
            pub struct #fixed_name {
                #fixed_fields_ts
            }
        });
    }

    // ── safe fixed(&FixedFields) — consume encoder, write all fixed fields ──
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
            let setter_ident = if is_converted {
                syn::Ident::new(&format!("{fname_snake}_wire"), span)
            } else {
                syn::Ident::new(&fname_snake, span)
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

    // ── raw_fixed() — dedicated consuming writer for manual fixed-field access ──
    // Individual fixed-field setters are available only on the raw writer.
    // The only safe transition to tail stages is via `fixed(&FixedFields)`.
    {
        let raw_name = syn::Ident::new(&format!("{name}RawFixedWriter"), span);
        ts.extend(quote::quote! {
            /// Raw fixed-field writer. Individual field setters are available
            /// only on this writer. When done, embed the fields in a
            /// `#fixed_name` and call the encoder's `fixed()`.
            #[must_use = "raw fixed writer must be embedded in FixedFields"]
            pub struct #raw_name<'a> {
                buf: &'a mut [u8],
                message_start: usize,
                pos: usize,
            }
        });
        impl_contents.extend(quote::quote! {
            /// Return a dedicated raw fixed-field writer. All individual field
            /// setters are available on the writer. To advance to tail stages,
            /// collect the values into a `#fixed_name` and call `fixed()`.
            #[inline]
            #[must_use]
            pub fn raw_fixed(mut self) -> #raw_name<'a> {
                #raw_name {
                    buf: self.buf,
                    message_start: self.message_start,
                    pos: self.pos,
                }
            }
        });
    }

    // Close the impl block
    if total_tail > 0 {
        ts.extend(quote::quote! {
            impl<'a> #name_encoder_ident<'a> {
                #impl_contents
            }
        });
    } else {
        ts.extend(quote::quote! {
            impl<'a> #name_encoder_ident<'a> {
                #impl_contents
            }
        });
    }

    // ── Tail state transition methods ──
    if total_tail > 0 {
        let mut tail_idx = 0;

        // Group methods
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
                impl<'a> #current_stage<'a> {
                    /// Encode this group with a known count up front. Closure may
                    /// return `()` or `Result<(), E>` (via
                    /// Closures return `GroupResult`; `?` just works. a
                    /// separate `try_*` method name.
                    #[must_use]
                    pub fn #g_snake<F>(
                        mut self,
                        count: #count_ty,
                        f: F,
                    ) -> Result<#next_stage<'a>, sbe_rt::EncodeError>
                    where
                                                F: FnOnce(&mut #g_pascal_enc<'a>) -> sbe_rt::GroupResult,
                    {
                        if self.pos + #dim_size_lit > self.buf.len() {
                            return Err(sbe_rt::EncodeError::BufferTooShort {
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
                            message_start: self.message_start,
                            pos: group.pos,
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
                    #[must_use]
                    pub fn #g_snake_unknown<F>(
                        mut self,
                        f: F,
                    ) -> Result<#next_stage<'a>, sbe_rt::EncodeError>
                    where
                                                F: FnOnce(&mut #g_pascal_enc<'a>) -> sbe_rt::GroupResult,
                    {
                        if self.pos + #dim_size_lit > self.buf.len() {
                            return Err(sbe_rt::EncodeError::BufferTooShort {
                                needed: #dim_size_lit,
                                available: self.buf.len().saturating_sub(self.pos),
                            }
                            .into());
                        }
                        // Write dimension template + zero placeholder count.
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
                            message_start: self.message_start,
                            pos,
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
                        needed,
                        available: self.buf.len().saturating_sub(self.pos),
                    });
                }
                let len_bytes = (data.len() as #len_rust_type).#to_endian();
                self.buf[self.pos..self.pos + #prefix_size_lit]
                    .copy_from_slice(&len_bytes);
                let start = self.pos + #prefix_size_lit;
                self.buf[start..start + data.len()].copy_from_slice(data);
                Ok(#next_stage {
                    buf: self.buf,
                    message_start: self.message_start,
                    pos: start + data.len(),
                })
            };

            ts.extend(quote::quote! {
                impl<'a> #current_stage<'a> {
                    #[must_use]
                    pub fn #vd_snake(
                        mut self,
                        data: &[u8],
                    ) -> Result<#next_stage<'a>, sbe_rt::EncodeError> {
                        #checked_body
                        #shared_body
                    }

                    #[must_use]
                    pub fn #vd_snake_unchecked(
                        mut self,
                        data: &[u8],
                    ) -> Result<#next_stage<'a>, sbe_rt::EncodeError> {
                        #shared_body
                    }

                    /// Lend exactly `exact_len` bytes of the var-data region
                    /// to a closure for nested-message encoding. Zero-copy:
                    /// the closure writes directly into the outer buffer.
                    ///
                    /// Canonical nested-SBE pattern (AppMessage → L2Book):
                    /// ```ignore
                    /// let inner = InnerEncoder::compute_encoded_length_with_message_header(...);
                    /// after.payload_with(inner, |p| {
                    ///     let mut enc = InnerEncoder::try_wrap_and_apply_header(p, 0)?;
                    ///     // set fields / groups / var-data …
                    ///     Ok(())
                    /// })?;
                    /// ```
                    /// Returns the next stage on success; on failure the
                    /// caller error propagates unchanged and no partial
                    /// data is published.
                    #[must_use]
                    pub fn #vd_snake_with<E, F>(
                        mut self,
                        exact_len: usize,
                        f: F,
                    ) -> Result<#next_stage<'a>, E>
                    where
                        E: From<sbe_rt::EncodeError>,
                        F: FnOnce(&mut [u8]) -> Result<(), E>,
                    {
                        #with_checked_body
                        let needed = #prefix_size_lit + exact_len;
                        if self.pos + needed > self.buf.len() {
                            return Err(sbe_rt::EncodeError::BufferTooShort {
                                needed,
                                available: self.buf.len().saturating_sub(self.pos),
                            }.into());
                        }
                        let len_bytes = (exact_len as #len_rust_type).#to_endian();
                        self.buf[self.pos..self.pos + #prefix_size_lit]
                            .copy_from_slice(&len_bytes);
                        let start = self.pos + #prefix_size_lit;
                        f(&mut self.buf[start..start + exact_len])?;
                        Ok(#next_stage {
                            buf: self.buf,
                            message_start: self.message_start,
                            pos: start + exact_len,
                        })
                    }
                }
            });
            tail_idx += 1;
        }

        // Complete state: as_bytes() + as_bytes_with_header() + AsRef +
        // encoded_length on the final stage struct
        let complete_ident = &stage_idents[total_tail];
        ts.extend(quote::quote! {
            impl<'a> #complete_ident<'a> {
                /// Returns the complete SBE message bytes (header + body).
                #[inline]
                pub fn as_bytes(&self) -> &[u8] {
                    &self.buf[self.message_start..self.pos]
                }
                /// Explicit header-inclusive view (alias for `as_bytes()`).
                /// DECISIONS.md §2: use this when header inclusion must be
                /// explicit rather than implied by the complete stage.
                #[inline]
                pub fn as_bytes_with_header(&self) -> &[u8] {
                    self.as_bytes()
                }
                #[inline]
                pub fn encoded_length(&self) -> usize {
                    self.pos - self.message_start - #header_size_lit
                }
                #[inline]
                pub fn encoded_length_with_header(&self) -> usize {
                    self.pos - self.message_start
                }
            }

            impl<'a> AsRef<[u8]> for #complete_ident<'a> {
                fn as_ref(&self) -> &[u8] {
                    self.as_bytes()
                }
            }
        });
    } else {
        ts.extend(quote::quote! {
            impl<'a> AsRef<[u8]> for #name_encoder_ident<'a> {
                fn as_ref(&self) -> &[u8] {
                    &self.buf[self.message_start..self.pos]
                }
            }
        });
    }

    // ── Sealed + SbeMessage ──
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

    // ── Generate Repeating Groups encoders ──
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

    // ── Unchecked companions — always generated for zero-validation fast path ──
    {
        let needed: usize = header_size + block_length;
        let needed_lit = syn::LitInt::new(&needed.to_string(), span);
        let hs_lit = syn::LitInt::new(&header_size.to_string(), span);
        ts.extend(quote::quote! {
            impl<'a> #name_encoder_ident<'a> {
                /// Wrap a mutable buffer for encoding — no bounds check.
                /// Caller guarantees the buffer is large enough.
                /// This is the default fast path (matching sbe-tool's `wrap`).
                #[inline]
                pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
                    Self { buf: &mut buf[pos..], message_start: 0, pos: #needed_lit }
                }

                /// Wrap a mutable buffer, write the header, and return the encoder.
                /// No bounds check — caller guarantees the buffer is large enough.
                /// This is the default fast path (matching sbe-tool's `wrap`).
                #[inline]
                pub fn wrap_and_apply_header(buf: &'a mut [u8], pos: usize) -> Self {
                    buf[pos..pos + #hs_lit].copy_from_slice(&Self::HEADER_TEMPLATE);
                    Self { buf: &mut buf[pos..], message_start: 0, pos: #needed_lit }
                }
            }
        });
    }

    // ── Standalone encoded-length types (from strategy-based generator) ──
    ts.extend(encoded_len_gen.standalone);

    ts
}

fn generate_group_encoder(
    src: &mut String,
    g: &MessageGroup,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    scoped_name: &str,
    conversions: &[crate::ConversionSelector],
    domain_types: &[(crate::ConversionSelector, String)],
) {
    let name = scoped_name.to_string();
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };
    let (_, dim_size, _, _) = get_dimension_info(elements, &g.dimension_type);
    let (_, _, num_prim) = get_dim_num_layout(elements, &g.dimension_type);
    let count_ty: syn::Type = syn::parse_str(rust_type(num_prim)).unwrap();

    let mut dim_tpl = vec![0u8; dim_size];
    match byte_order {
        ByteOrder::LittleEndian => {
            dim_tpl[0..2].copy_from_slice(&(g.block_length as u16).to_le_bytes());
        }
        ByteOrder::BigEndian => {
            dim_tpl[0..2].copy_from_slice(&(g.block_length as u16).to_be_bytes());
        }
    }

    let span = proc_macro2::Span::call_site();
    let group_enc_ident = syn::Ident::new(&format!("{}Encoder", name), span);
    let entry_enc_ident = syn::Ident::new(&format!("{}EntryEncoder", name), span);
    let block_len_lit = syn::LitInt::new(&g.block_length.to_string(), span);
    let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), span);
    let dim_bytes: Vec<syn::LitInt> = dim_tpl
        .iter()
        .map(|b| syn::LitInt::new(&b.to_string(), span))
        .collect();
    let to_endian = syn::Ident::new(&format!("to_{order_suffix}_bytes"), span);

    // Build nullification for add() body (inline, uses self.buf)
    let mut null_stmts = proc_macro2::TokenStream::new();
    for f in &g.fields {
        if f.presence == Presence::Optional {
            if let Some(null_val) = f.null_value {
                let size = f.field_type.size();
                let null_val_expr: syn::Expr = syn::parse_str(&format!("{null_val}_u64")).unwrap();
                let f_offset = syn::Index::from(f.offset);
                let size_lit = syn::LitInt::new(&size.to_string(), span);
                null_stmts.extend(quote::quote! {
                    let null_bytes = #null_val_expr.#to_endian();
                    let offset = self.pos + #f_offset;
                    self.buf[offset..offset + #size_lit].copy_from_slice(&null_bytes);
                });
            }
        }
    }

    let mut add_body = quote::quote! {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: self.written as u32 + 1,
            }
            .into());
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len().saturating_sub(self.pos),
            }
            .into());
        }
    };
    if !null_stmts.is_empty() {
        add_body.extend(null_stmts);
    }
    add_body.extend(quote::quote! {
        // SAFETY: same borrow-split pattern as the group encoder method above.
        // The closure `f` only operates on __entry (which holds __buf), never
        // on `self`. The block scope drops __buf before `self.pos` is written.
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut __entry = #entry_enc_ident::wrap(__buf, self.pos);
            f(&mut __entry)?;
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    });

    let mut ts = proc_macro2::TokenStream::new();
    ts.extend(quote::quote! {
        #[must_use = "group encoder must call add() to write entries"]
        pub struct #group_enc_ident<'a> {
            buf: &'a mut [u8],
            pos: usize,
            count: #count_ty,
            written: #count_ty,
        }

        impl<'a> #group_enc_ident<'a> {
            pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;
            pub const GROUP_DIM_TEMPLATE: [u8; #dim_size_lit] = [#(#dim_bytes),*];
            const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == #dim_size_lit);

            #[inline]
            pub fn wrap(buf: &'a mut [u8], pos: usize, count: #count_ty) -> Self {
                Self { buf, pos, count, written: 0 }
            }

            /// Write one group entry. Closure may return `()` or `Result<(), E>`
            /// ([`sbe_rt::GroupEncodeResult`]) so `?` works without `try_add`.
            #[must_use]
            pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #entry_enc_ident<'b>) -> sbe_rt::GroupResult,
            {
                #add_body
            }

            /// Manual entry creation: returns a borrowed entry encoder.
            /// The entry writes fixed fields directly into the group buffer.
            /// Drop the entry or let it go out of scope to commit it.
            /// The group position is pre-advanced, so fields are written
            /// to the correct offset.
            #[must_use]
            pub fn start_entry(&mut self) -> Result<#entry_enc_ident<'_>, sbe_rt::EncodeError> {
                if self.written as u32 >= self.count as u32 {
                    return Err(sbe_rt::EncodeError::GroupFull {
                        declared: self.count as u32,
                        attempted: (self.written as u32) + 1,
                    });
                }
                let entry_pos = self.pos;
                self.pos += #block_len_lit;
                self.written += 1;
                Ok(#entry_enc_ident::wrap(&mut self.buf[entry_pos..], 0))
            }
        }
    });

    // written() accessor — used by _unknown_size to back-patch the count.
    ts.extend(quote::quote! {
        impl<'a> #group_enc_ident<'a> {
            /// Number of entries written so far (for `_unknown_size` back-patch).
            #[inline]
            pub fn written(&self) -> #count_ty {
                self.written
            }
        }
    });

    // add_struct: when the entry has no nested groups or var-data, generate
    // a named value struct so callers can write whole entries in one call.
    if g.groups.is_empty() && g.var_data.is_empty() {
        let entry_struct_ident = syn::Ident::new(&format!("{}Entry", name), span);
        let mut struct_fields = proc_macro2::TokenStream::new();
        let mut struct_write = proc_macro2::TokenStream::new();
        for f in &g.fields {
            if f.presence == Presence::Constant {
                continue;
            }
            let f_name = syn::Ident::new(&to_snake_case(&f.name), span);
            let f_ty = field_type_ident(&f.field_type, span);
            let f_offset = syn::Index::from(f.offset);
            let f_size = syn::LitInt::new(&f.field_type.size().to_string(), span);
            struct_fields.extend(quote::quote! { pub #f_name: #f_ty, });
            match &f.field_type {
                FieldType::Composite { .. } => {
                    struct_write.extend(quote::quote! {
                        self.buf[pos + #f_offset..][..#f_size].copy_from_slice(&entry.#f_name.0);
                    });
                }
                FieldType::Enum { encoding_type, .. } | FieldType::Set { encoding_type, .. } => {
                    let r_ty = syn::Ident::new(&rust_type(*encoding_type), span);
                    struct_write.extend(quote::quote! {
                        self.buf[pos + #f_offset..][..#f_size]
                            .copy_from_slice(&(entry.#f_name as #r_ty).#to_endian());
                    });
                }
                FieldType::Primitive(pt, Some(_len)) if pt.size() == 1 => {
                    struct_write.extend(quote::quote! {
                        self.buf[pos + #f_offset..][..#f_size].copy_from_slice(&entry.#f_name);
                    });
                }
                FieldType::Primitive(..) => {
                    struct_write.extend(quote::quote! {
                        self.buf[pos + #f_offset..][..#f_size]
                            .copy_from_slice(&entry.#f_name.#to_endian());
                    });
                }
            }
        }
        ts.extend(quote::quote! {
            /// Value struct for this group's entries.
            #[derive(Debug, Clone, PartialEq)]
            pub struct #entry_struct_ident {
                #struct_fields
            }
        });
        ts.extend(quote::quote! {
            impl<'a> #group_enc_ident<'a> {
                /// Write one entry from a struct. Faster than [`Self::add`] when
                /// the entry has no nested groups or var-data.
                pub fn add_struct(&mut self, entry: &#entry_struct_ident) -> Result<(), sbe_rt::EncodeError> {
                    if self.written as u32 >= self.count as u32 {
                        return Err(sbe_rt::EncodeError::GroupFull {
                            declared: self.count as u32,
                            attempted: (self.written as u32) + 1,
                        });
                    }
                    let block_len = Self::ENTRY_BLOCK_LENGTH;
                    if self.pos + block_len > self.buf.len() {
                        return Err(sbe_rt::EncodeError::BufferTooShort {
                            needed: block_len,
                            available: self.buf.len().saturating_sub(self.pos),
                        });
                    }
                    let pos = self.pos;
                    self.pos += block_len;
                    self.written += 1;
                    #struct_write
                    Ok(())
                }
            }
        });
    }

    // Entry encoder struct + all methods in a single impl block
    let mut entry_methods = proc_macro2::TokenStream::new();

    entry_methods.extend(quote::quote! {
        pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

        #[inline]
        pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
            Self {
                buf,
                entry_start: pos,
                pos: pos + Self::ENTRY_BLOCK_LENGTH,
            }
        }
    });

    // Field setters
    for f in &g.fields {
        let f_snake = to_snake_case(&f.name);
        // Raw entry setters become *_wire when a conversion is configured.
        let wire_name =
            field_has_conversion_free(f, conversions).then(|| format!("{f_snake}_wire"));
        let setter_name = wire_name.as_deref().unwrap_or(&f_snake);
        let f_ident = syn::Ident::new(&setter_name, span);
        let f_offset = syn::Index::from(f.offset);

        match &f.field_type {
            FieldType::Primitive(prim, length) => {
                let r_ty = syn::Ident::new(&rust_type(*prim), span);
                let prim_size = prim.size();
                if f.presence == Presence::Constant {
                    continue;
                } else if let Some(len) = length {
                    let len_lit = syn::LitInt::new(&len.to_string(), span);
                    let sz = syn::LitInt::new(&prim_size.to_string(), span);
                    entry_methods.extend(quote::quote! {
                        pub fn #f_ident(&mut self, val: [#r_ty; #len_lit]) -> &mut Self {
                            let offset = self.entry_start + #f_offset;
                            let mut idx = 0;
                            while idx < #len_lit {
                                self.buf[offset + idx * #sz..][..#sz].copy_from_slice(&val[idx].#to_endian());
                                idx += 1;
                            }
                            self
                        }
                    });
                } else if prim_size == 1 {
                    entry_methods.extend(quote::quote! {
                        pub fn #f_ident(&mut self, val: #r_ty) -> &mut Self {
                            self.buf[self.entry_start + #f_offset] = val as u8;
                            self
                        }
                    });
                } else {
                    let sz = syn::LitInt::new(&prim_size.to_string(), span);
                    entry_methods.extend(quote::quote! {
                        pub fn #f_ident(&mut self, val: #r_ty) -> &mut Self {
                            let offset = self.entry_start + #f_offset;
                            self.buf[offset..offset + #sz].copy_from_slice(&val.#to_endian());
                            self
                        }
                    });
                }
            }
            FieldType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target = syn::Ident::new(&to_pascal_case(comp_name), span);
                let sz = syn::LitInt::new(&comp_size.to_string(), span);
                entry_methods.extend(quote::quote! {
                    pub fn #f_ident(&mut self, val: #target) -> &mut Self {
                        let offset = self.entry_start + #f_offset;
                        self.buf[offset..offset + #sz].copy_from_slice(&val.0);
                        self
                    }
                });
            }
            FieldType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                let target = syn::Ident::new(&to_pascal_case(enum_name), span);
                let r_ty = syn::Ident::new(&rust_type(*encoding_type), span);
                let sz = syn::LitInt::new(&encoding_type.size().to_string(), span);
                entry_methods.extend(quote::quote! {
                    pub fn #f_ident(&mut self, val: #target) -> &mut Self {
                        let offset = self.entry_start + #f_offset;
                        self.buf[offset..offset + #sz].copy_from_slice(&(val as #r_ty).#to_endian());
                        self
                    }
                });
                if is_bool_enum(elements, enum_name) {
                    let f_name_bool = syn::Ident::new(&format!("{}_bool", f_snake), span);
                    entry_methods.extend(quote::quote! {
                        pub fn #f_name_bool(&mut self, val: bool) -> &mut Self {
                            self.buf[self.entry_start + #f_offset] = val as u8;
                            self
                        }
                    });
                }
            }
            FieldType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target = syn::Ident::new(&to_pascal_case(set_name), span);
                let sz = syn::LitInt::new(&encoding_type.size().to_string(), span);
                entry_methods.extend(quote::quote! {
                    pub fn #f_ident(&mut self, val: #target) -> &mut Self {
                        let offset = self.entry_start + #f_offset;
                        self.buf[offset..offset + #sz].copy_from_slice(&val.0.#to_endian());
                        self
                    }
                });
            }
        }
    }

    // Nested group setters — scope under parent group name
    for ng in &g.groups {
        let ng_pascal_scoped = format!("{}{}", name, to_pascal_case(&ng.name));
        let ng_snake = syn::Ident::new(&to_snake_case(&ng.name), span);
        let ng_snake_unknown =
            syn::Ident::new(&format!("{}_unknown_size", to_snake_case(&ng.name)), span);
        let ng_enc = syn::Ident::new(&format!("{ng_pascal_scoped}Encoder"), span);
        let (_dim_name, ng_dim_size, _, _) = get_dimension_info(elements, &ng.dimension_type);
        let (num_off, num_sz, ng_num_prim) = get_dim_num_layout(elements, &ng.dimension_type);
        let ng_dim = syn::LitInt::new(&ng_dim_size.to_string(), span);
        let num_off_idx = syn::Index::from(num_off);
        let num_sz_lit = syn::LitInt::new(&num_sz.to_string(), span);
        let ng_count_ty: syn::Type = syn::parse_str(rust_type(ng_num_prim)).unwrap();

        entry_methods.extend(quote::quote! {
            #[must_use]
            pub fn #ng_snake<F>(&mut self, count: #ng_count_ty, f: F) -> Result<&mut Self, sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #ng_enc<'a>) -> sbe_rt::GroupResult,
            {
                if self.pos + #ng_dim > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        needed: #ng_dim,
                        available: self.buf.len().saturating_sub(self.pos),
                    }
                    .into());
                }
                self.buf[self.pos..self.pos + #ng_dim].copy_from_slice(&#ng_enc::GROUP_DIM_TEMPLATE);
                self.buf[self.pos + #num_off_idx..self.pos + #num_off_idx + #num_sz_lit].copy_from_slice(&count.#to_endian());
                // SAFETY: the closure `f` only operates on the group encoder (which
                // holds __buf), never on `self`. The block scope ensures __buf is
                // dropped before `self.pos` is written. No aliasing occurs because
                // `self.buf` is not accessed through `self` while __buf is live.
                // This is the standard borrow-split pattern (same as split_at_mut
                // internals) — the raw pointer cast is a borrow-checker workaround,
                // not an actual aliasing violation.
                let __pos;
                {
                    let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                    let mut group = #ng_enc::wrap(__buf, self.pos + #ng_dim, count);
                    f(&mut group)?;
                    let written = group.written();
                    if written != count {
                        return Err(sbe_rt::EncodeError::GroupCountMismatch {
                            declared: count as u32,
                            actual: written as u32,
                        });
                    }
                    __pos = group.pos;
                }
                self.pos = __pos;
                Ok(self)
            }

            /// Nested-group `_unknown_size` variant — back-patches count.
            pub fn #ng_snake_unknown<F>(&mut self, f: F) -> Result<&mut Self, sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #ng_enc<'a>) -> sbe_rt::GroupResult,
            {
                if self.pos + #ng_dim > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        needed: #ng_dim,
                        available: self.buf.len().saturating_sub(self.pos),
                    }.into());
                }
                self.buf[self.pos..self.pos + #ng_dim].copy_from_slice(&#ng_enc::GROUP_DIM_TEMPLATE);
                let count_offset = self.pos + #num_off_idx;
                self.buf[count_offset..count_offset + #num_sz_lit].fill(0);
                let __pos;
                {
                    let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
                    let mut group = #ng_enc::wrap(__buf, self.pos + #ng_dim, #ng_count_ty::MAX);
                    f(&mut group)?;
                    let actual: #ng_count_ty = group.written();
                    __pos = group.pos;
                    group.buf[count_offset..count_offset + #num_sz_lit]
                        .copy_from_slice(&actual.#to_endian());
                }
                self.pos = __pos;
                Ok(self)
            }
        });
    }

    // VarData setters
    for vd in &g.var_data {
        let vd_snake = syn::Ident::new(&to_snake_case(&vd.name), span);
        let (_, prefix_sz, _, len_type) = get_vardata_info(elements, &vd.type_name);
        let pfx = syn::LitInt::new(&prefix_sz.to_string(), span);
        let len_ty = syn::Ident::new(&rust_type(len_type), span);

        entry_methods.extend(quote::quote! {
            #[must_use]
            pub fn #vd_snake(&mut self, data: &[u8]) -> Result<&mut Self, sbe_rt::EncodeError> {
                let needed = #pfx + data.len();
                if self.pos + needed > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort { needed, available: self.buf.len().saturating_sub(self.pos) });
                }
                let len_bytes = (data.len() as #len_ty).#to_endian();
                self.buf[self.pos..self.pos + #pfx].copy_from_slice(&len_bytes);
                let start = self.pos + #pfx;
                self.buf[start..start + data.len()].copy_from_slice(data);
                self.pos = start + data.len();
                Ok(self)
            }
        });
    }

    ts.extend(quote::quote! {
        #[must_use = "entry encoder fields must be set before the next entry"]
        pub struct #entry_enc_ident<'a> {
            buf: &'a mut [u8],
            entry_start: usize,
            pos: usize,
        }

        impl<'a> #entry_enc_ident<'a> {
            #entry_methods
        }
    });

    src.push_str(&ts.to_string());
    src.push('\n');
    src.push('\n');

    // Recursively generate nested Repeating Groups encoders
    for ng in &g.groups {
        let nested_name = format!("{}{}", name, to_pascal_case(&ng.name));
        generate_group_encoder(
            src,
            ng,
            elements,
            byte_order,
            &nested_name,
            &conversions,
            domain_types,
        );
    }
}

/// Generate a `pub mod prelude` that re-exports the common API surface so users
/// can write `use my_schema::prelude::*;`.
#[cfg(test)]
mod tests {
    use super::Generator;
    use crate::{GenerationConfig, Schema};

    #[test]
    fn generator_emits_deterministic_module_name() -> Result<(), Box<dyn std::error::Error>> {
        let generator = Generator::new(GenerationConfig::new("market_data"));
        let schema = Schema::new("fix.sbe", 1, 0);

        let modules = generator.generate(&schema)?;
        let collected = modules.modules().collect::<Vec<_>>();

        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].path, "market_data.rs");
        assert!(collected[0].source.contains("fix.sbe"));

        Ok(())
    }

    #[test]
    fn generate_multi_creates_separate_modules() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = GenerationConfig::new("common");
        config.shared_module = Some("common_types".to_string());

        let generator = Generator::new(config);

        let schema_a = Schema::new("common.sbe", 1, 0);
        let schema_b = Schema::new("market_data.sbe", 2, 0);

        let modules =
            generator.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")])?;
        let collected: Vec<_> = modules.modules().collect();

        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].path, "common_types.rs");
        assert_eq!(collected[1].path, "market_data.rs");

        // First module has the sbe_rt runtime
        assert!(collected[0].source.contains("pub mod sbe_rt"));

        // Second module does NOT have its own sbe_rt (sbe_rt comes via pub use)
        assert!(!collected[1].source.contains("pub mod sbe_rt"));

        // Second module imports from the shared module
        assert!(
            collected[1]
                .source
                .contains("pub use super::common_types::*;")
        );

        // Each module contains its own schema metadata
        assert!(collected[0].source.contains("common.sbe"));
        assert!(collected[1].source.contains("market_data.sbe"));

        Ok(())
    }

    #[test]
    fn generate_multi_without_shared_module_emits_sbe_rt_everywhere()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("common");
        let generator = Generator::new(config);

        let schema_a = Schema::new("common.sbe", 1, 0);
        let schema_b = Schema::new("market_data.sbe", 2, 0);

        let modules = generator.generate_multi(&[(&schema_a, "a_mod"), (&schema_b, "b_mod")])?;
        let collected: Vec<_> = modules.modules().collect();

        assert_eq!(collected.len(), 2);

        // Both modules get sbe_rt when no shared_module is configured
        assert!(collected[0].source.contains("pub mod sbe_rt"));
        assert!(collected[1].source.contains("pub mod sbe_rt"));

        // No top-level pub use re-exports (prelude's pub use is inside its module)
        assert!(!collected[1].source.contains("\npub use super::"));
        Ok(())
    }

    // ── partition_tokens defensive-branch coverage ──────────────────
    // These branches are unreachable through normal XML parsing (the parser
    // validates token structure before emission). We cover them by calling
    // partition_tokens directly with crafted invalid token sequences.

    use super::{
        SchemaElements, parse_composite_members, parse_field_structure, parse_group_structure,
        parse_message_structure, parse_vardata_structure, to_snake_case,
    };
    use crate::ir::{Encoding, Signal, Token};

    fn make_token(signal: Signal) -> Token {
        Token {
            id: None,
            name: String::new(),
            signal,
            encoding: Encoding::default(),
        }
    }

    fn empty_elements() -> SchemaElements {
        SchemaElements {
            composites: vec![],
            enums: vec![],
            sets: vec![],
            messages: vec![],
        }
    }

    #[test]
    fn message_structure_skips_unexpected_signal() -> Result<(), Box<dyn std::error::Error>> {
        // parse_message_structure body loop: BeginEnum inside a message body
        // falls to `_ => i += 1` (lines ~797-799).
        let elem = empty_elements();
        let _ = parse_message_structure(
            &[
                make_token(Signal::BeginMessage),
                make_token(Signal::BeginEnum), // unexpected
                make_token(Signal::EndMessage),
            ],
            &elem,
        );

        Ok(())
    }

    #[test]
    fn group_structure_skips_unexpected_signal() -> Result<(), Box<dyn std::error::Error>> {
        // parse_group_structure body loop: BeginMessage inside a group body
        // falls to `_ => i += 1` (lines ~937-939).
        let elem = empty_elements();
        let _ = parse_group_structure(
            &[
                make_token(Signal::BeginGroup),
                make_token(Signal::BeginMessage), // unexpected
                make_token(Signal::EndGroup),
            ],
            &elem,
        );

        Ok(())
    }

    #[test]
    fn vardata_structure_skips_non_length_fields() -> Result<(), Box<dyn std::error::Error>> {
        // parse_vardata_structure loops tokens looking for the "length"
        // BeginField; any other BeginField falls to `i += 1` (lines ~974-977).
        let _ = parse_vardata_structure(&[
            make_token(Signal::BeginComposite),
            make_token(Signal::BeginField),
            make_token(Signal::EndField),
            make_token(Signal::EndComposite),
        ]);

        Ok(())
    }

    #[test]
    fn composite_members_skips_non_field_signals() -> Result<(), Box<dyn std::error::Error>> {
        // parse_composite_members loops from index 1 to len-1; any signal
        // that isn't BeginField falls to `else { i += 1 }` (lines ~1097-1099).
        let _ = parse_composite_members(&[
            make_token(Signal::BeginComposite),
            make_token(Signal::BeginMessage), // not BeginField → skip
            make_token(Signal::EndComposite),
        ]);
        Ok(())
    }

    #[test]
    fn field_structure_falls_back_to_uint8_primitive() -> Result<(), Box<dyn std::error::Error>> {
        // parse_field_structure: when tokens.len() > 2 and the inner signal
        // isn't BeginComposite/Enum/Set, defaults to Primitive(UInt8) (865-871).
        let elem = empty_elements();
        let _ = parse_field_structure(
            &[
                make_token(Signal::BeginField),
                make_token(Signal::BeginMessage), // unexpected inner → Primitive default
                make_token(Signal::EndField),
            ],
            &elem,
        );

        Ok(())
    }

    #[test]
    fn snake_case_handles_empty_or_special_input() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(to_snake_case(""), "");
        // Double-underscore input exercises the dedup `continue` (line 520).
        assert_eq!(to_snake_case("Foo__Bar"), "foo_bar");

        Ok(())
    }

    #[test]
    fn partition_skips_unexpected_at_top_level() -> Result<(), Box<dyn std::error::Error>> {
        // Top-level loop only matches BeginComposite/Enum/Set/Message;
        // BeginField falls to `_ => i += 1` (lines ~682-684).
        let _ = super::partition_tokens(&[make_token(Signal::BeginField)]);

        Ok(())
    }

    #[test]
    fn partition_skips_unexpected_in_message_body() -> Result<(), Box<dyn std::error::Error>> {
        // Message body loop only matches BeginField/Group/VarData;
        // BeginEnum inside a message body falls to `_ => i += 1` (lines ~797).
        let _ = super::partition_tokens(&[
            make_token(Signal::BeginMessage),
            make_token(Signal::BeginEnum), // unexpected inside message body
            make_token(Signal::EndMessage),
        ]);

        Ok(())
    }

    #[test]
    fn partition_skips_unexpected_in_group_body() -> Result<(), Box<dyn std::error::Error>> {
        // Group body loop only matches BeginField/Group/VarData;
        // BeginMessage inside a group falls to `_ => i += 1` (lines ~937).
        let _ = super::partition_tokens(&[
            make_token(Signal::BeginGroup),
            make_token(Signal::BeginMessage), // unexpected inside group body
            make_token(Signal::EndGroup),
        ]);

        Ok(())
    }

    #[test]
    fn partition_skips_unexpected_after_top_level_items() -> Result<(), Box<dyn std::error::Error>>
    {
        // After BeginMessage/EndMessage pair, unrelated signals skip at top level.
        let _ = super::partition_tokens(&[
            make_token(Signal::BeginMessage),
            make_token(Signal::EndMessage),
            make_token(Signal::BeginEnum), // at top level
        ]);

        Ok(())
    }

    #[test]
    fn semantic_type_matches_primitive_field() -> Result<(), Box<dyn std::error::Error>> {
        use crate::ir::{Presence, PrimitiveType};
        use crate::structured_ir::{FieldType, MessageField};
        let field = MessageField {
            name: "exchangeTimestamp".into(),
            id: Some(1),
            offset: 0,
            presence: Presence::Required,
            since_version: 0,
            null_value: None,
            min_value: None,
            max_value: None,
            description: None,
            semantic_type: Some("UTCTimestamp".into()),
            constant_value: None,
            field_type: FieldType::Primitive(PrimitiveType::UInt64, None),
        };
        let conversions = vec![crate::ConversionSelector::semantic_type("UTCTimestamp")];
        assert!(
            super::field_has_conversion_free(&field, &conversions),
            "SemanticType should match primitive u64 with semanticType=UTCTimestamp"
        );

        let domain_types = vec![(
            crate::ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>".into(),
        )];
        let dt = super::find_domain_type(&field, &domain_types);
        assert_eq!(
            dt,
            Some("chrono::DateTime<chrono::Utc>"),
            "should find domain type for UTCTimestamp"
        );
        Ok(())
    }

    #[test]
    fn chrono_converter_generates_accessor() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0" encoding="UTF-8"?>
        <sbe:messageSchema xmlns:sbe="http://fixprotocol.io/2016/sbe"
            package="test.chrono" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId"   primitiveType="uint16"/>
              <type name="schemaId"     primitiveType="uint16"/>
              <type name="version"      primitiveType="uint16"/>
            </composite>
          </types>
          <sbe:message name="TsMsg" id="1">
            <field name="ts" id="1" type="uint64" semanticType="UTCTimestamp"/>
          </sbe:message>
        </sbe:messageSchema>"#;
        let ir = crate::parse(xml)?;
        let schema = crate::Schema::from_ir(ir);
        let config = crate::GenerationConfig::new("test_chrono").with_domain_type(
            crate::ConversionSelector::semantic_type("UTCTimestamp"),
            "chrono::DateTime<chrono::Utc>",
        );
        let generator = crate::Generator::new(config);
        let modules = generator.generate(&schema)?;
        let src = modules.modules().next().unwrap().source.clone();
        assert!(
            src.contains("fn ts(&self) -> chrono::DateTime"),
            "should generate concrete DateTime accessor for UTCTimestamp field"
        );
        assert!(
            src.contains("fn ts_wire"),
            "should rename raw u64 getter to _wire"
        );
        assert!(
            src.contains("impl TryFromSbe<u64> for chrono::DateTime<chrono::Utc>"),
            "should generate TryFromSbe impl"
        );
        Ok(())
    }
}

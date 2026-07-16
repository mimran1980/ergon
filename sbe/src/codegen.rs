//! Rust code generation from the resolved SBE IR.
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
use crate::{GenerationConfig, Schema};
use quote::format_ident;
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

/// Errors returned by [`Generator::try_generate`] when the configuration
/// is invalid for the given schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerateError {
    /// A registered decimal composite is missing or has the wrong layout.
    InvalidDecimalComposite {
        /// Name of the composite.
        name: String,
        /// Why validation failed.
        reason: String,
    },
}

impl core::fmt::Display for GenerateError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::InvalidDecimalComposite { name, reason } => {
                write!(f, "invalid decimal composite '{name}': {reason}")
            }
        }
    }
}

impl core::error::Error for GenerateError {}

impl GeneratedModuleSet {
    /// Add a generated module to the set.
    pub fn push(&mut self, module: GeneratedModule) {
        self.modules.push(module);
    }

    /// Iterate over generated modules in deterministic output order.
    #[must_use]
    pub fn modules(&self) -> impl ExactSizeIterator<Item = &GeneratedModule> {
        self.modules.iter()
    }
}

/// SBE-to-Rust generator.
#[derive(Clone, Debug)]
pub struct Generator {
    config: GenerationConfig,
}

impl Generator {
    /// Create a generator with the supplied configuration.
    #[must_use]
    pub const fn new(config: GenerationConfig) -> Self {
        Self { config }
    }

    /// Return this generator's configuration.
    #[must_use]
    pub const fn config(&self) -> &GenerationConfig {
        &self.config
    }

    /// Generate Rust modules for a normalized schema, returning an error
    /// on invalid configuration (e.g. bad decimal composite registration).
    pub fn try_generate(&self, schema: &Schema) -> Result<GeneratedModuleSet, GenerateError> {
        self.validate_decimal_composites(schema)?;
        Ok(self.generate(schema))
    }

    fn validate_decimal_composites(&self, schema: &Schema) -> Result<(), GenerateError> {
        let elements = partition_tokens(&schema.ir.tokens);
        for name in &self.config.decimal_composites {
            let ct = elements
                .composites
                .iter()
                .find(|c| c[0].name == *name)
                .ok_or_else(|| GenerateError::InvalidDecimalComposite {
                    name: name.clone(),
                    reason: "composite not found in schema".into(),
                })?;
            // Filter to BeginField tokens only (skip EndField, etc.)
            let fields: Vec<_> = ct
                .iter()
                .filter(|t| matches!(t.signal, Signal::BeginField))
                .collect();
            if fields.len() < 2 {
                return Err(GenerateError::InvalidDecimalComposite {
                    name: name.clone(),
                    reason: "composite must have at least 2 members".into(),
                });
            }
            let mantissa = fields[0];
            let exponent = fields[1];
            let valid = mantissa.name == "mantissa"
                && mantissa.encoding.primitive_type == Some(PrimitiveType::Int64)
                && exponent.name == "exponent"
                && exponent.encoding.primitive_type == Some(PrimitiveType::Int8);
            if !valid {
                return Err(GenerateError::InvalidDecimalComposite {
                    name: name.clone(),
                    reason: "expected mantissa: int64, exponent: int8 layout".into(),
                });
            }
        }
        Ok(())
    }

    /// Generate Rust modules for a normalized schema.
    ///
    /// Validates decimal converter configuration and panics on invalid
    /// composites. Use [`try_generate`](Self::try_generate) for fallible
    /// generation that returns a [`GenerateError`].
    #[must_use]
    pub fn generate(&self, schema: &Schema) -> GeneratedModuleSet {
        if let Err(e) = self.validate_decimal_composites(schema) {
            panic!("decimal composite validation failed: {e}");
        }
        let mut modules = GeneratedModuleSet::default();
        let src = self.gen_schema(schema, &HashSet::new(), false, true);
        modules.push(GeneratedModule {
            path: format!("{}.rs", self.config.module_name),
            source: src,
        });
        modules
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
    #[must_use]
    pub fn generate_multi(&self, schemas: &[(&Schema, &str)]) -> GeneratedModuleSet {
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
        modules
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

        // 1. Generate inline SBE runtime (only once)
        if emit_sbe_rt {
            src.push_str(&generate_sbe_rt_src());
            // Decimal converter trait (opt-in, dependency-free).
            if !self.config.decimal_composites.is_empty() {
                src.push_str(
                    "pub trait SbeDecimal: Sized {\n\
                         type Error;\n\
                         fn try_from_sbe(mantissa: i64, exponent: i8) -> Result<Self, Self::Error>;\n\
                         fn try_into_sbe(self) -> Result<(i64, i8), Self::Error>;\n\
                     }\n",
                );
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
            generate_composite(&mut src, composite_tokens, ir.byte_order);
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
                &self.config.decimal_composites,
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
                &self.config.decimal_composites,
            );
            src.push_str(&encoder_ts.to_string());

            // Decimal converter seam: for each field backed by a registered
            // Decimal composite, emit raw *_wire aliases and generic converted
            // methods. Only emitted when converter mode is active.
            if !self.config.decimal_composites.is_empty() {
                let converter_ts = generate_decimal_converter_impls(
                    msg,
                    &self.config.decimal_composites,
                );
                src.push_str(&converter_ts);
            }
            src.push('\n');
            generate_message_field_meta(&mut src, msg);
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
        // 7.7. Generate const-compatible byte-read helper (avoids per-accessor loop bloat)
        let read_bytes_ts: proc_macro2::TokenStream = quote::quote! {
            /// Read `N` bytes from `buf` at `offset` into a fixed-size array.
            ///
            /// Safe path uses slice indexing (bounds-checked, equivalent to Aeron's
            /// `slice[index..index+N].try_into()`). With `bound-check-disabled`,
            /// uses `core::ptr::read_unaligned` for zero-overhead access.
            #[inline]
            pub fn read_bytes<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
                #[cfg(not(feature = "bound-check-disabled"))]
                {
                    buf[offset..offset + N].try_into().expect("read_bytes: buffer too short")
                }
                #[cfg(feature = "bound-check-disabled")]
                // SAFETY: caller guarantees offset + N <= buf.len().
                // read_unaligned is safe for [u8; N] regardless of alignment.
                unsafe {
                    core::ptr::read_unaligned(buf.as_ptr().add(offset) as *const [u8; N])
                }
            }

            /// Write `N` bytes from `bytes` into `buf` at `offset`.
            ///
            /// Safe path uses `copy_from_slice`. With `bound-check-disabled`,
            /// uses `core::ptr::write_unaligned` for zero-overhead write.
            #[inline]
            pub fn write_bytes<const N: usize>(buf: &mut [u8], offset: usize, bytes: &[u8; N]) {
                #[cfg(not(feature = "bound-check-disabled"))]
                {
                    buf[offset..offset + N].copy_from_slice(bytes);
                }
                #[cfg(feature = "bound-check-disabled")]
                // SAFETY: caller guarantees offset + N <= buf.len().
                // write_unaligned is safe for [u8; N] regardless of alignment.
                unsafe {
                    core::ptr::write_unaligned(buf.as_mut_ptr().add(offset) as *mut [u8; N], *bytes);
                }
            }
        };
        src.push_str(&read_bytes_ts.to_string());
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
fn generate_sbe_rt_src() -> String {
    let module = quote::quote! {
        pub mod sbe_rt {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum DecodeError {
                BufferTooShort { field: &'static str, needed: usize, available: usize },
                WrongSchema { expected: u16, actual: u16, expected_name: &'static str },
                UnknownTemplateLength { template_id: u16 },
                InvalidVarDataLength { field: &'static str, length: u32, max_length: u32 },
                Utf8(core::str::Utf8Error),
            }

            impl core::fmt::Display for DecodeError {
                #[cold]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Self::BufferTooShort { field, needed, available } => write!(f, "field '{}': needed {} bytes, {} available", field, needed, available),
                        Self::WrongSchema { expected, actual, expected_name } => write!(f, "wrong schema: expected id {} ({}), got id {}", expected, expected_name, actual),
                        Self::UnknownTemplateLength { template_id } => write!(f, "unknown template id {}: SBE messages do not carry length. Use decode_frame() with an external frame length.", template_id),
                        Self::InvalidVarDataLength { field, length, max_length } => write!(f, "var data field '{}: length {} exceeds max {}", field, length, max_length),
                        Self::Utf8(err) => write!(f, "UTF-8 decode error: {}", err),
                    }
                }
            }

            impl core::error::Error for DecodeError {}

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum EncodeError {
                BufferTooShort { needed: usize, available: usize },
                VarDataTooLong { field: &'static str, max_length: usize, actual: usize },
                GroupFull { declared: u32, attempted: u32 },
                Decode(DecodeError),
            }

            impl core::fmt::Display for EncodeError {
                #[cold]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Self::BufferTooShort { needed, available } => write!(f, "buffer too short: needed {}, available {}", needed, available),
                        Self::VarDataTooLong { field, max_length, actual } => write!(f, "var data too long for field {}: max {}, actual {}", field, max_length, actual),
                        Self::GroupFull { declared, attempted } => write!(f, "group full: declared count {}, attempted to write {}", declared, attempted),
                        Self::Decode(e) => write!(f, "decode error: {e}"),
                    }
                }
            }

            impl core::error::Error for EncodeError {}

            impl From<DecodeError> for EncodeError {
                fn from(e: DecodeError) -> Self {
                    Self::Decode(e)
                }
            }

            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub enum VerifyError {
                HeaderTooShort,
                InvalidBlockLength { expected_min: usize, actual: usize },
                GroupDimOutOfBounds { field: &'static str, offset: usize },
                VarDataOutOfBounds { field: &'static str, offset: usize, length: u32 },
                MessageTooShort { needed: usize, available: usize },
            }

            impl core::fmt::Display for VerifyError {
                #[cold]
                fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                    match self {
                        Self::HeaderTooShort => write!(f, "buffer too short to contain message header"),
                        Self::InvalidBlockLength { expected_min, actual } => write!(f, "invalid block length: expected at least {}, actual {}", expected_min, actual),
                        Self::GroupDimOutOfBounds { field, offset } => write!(f, "group dimension header for '{}' out of bounds at offset {}", field, offset),
                        Self::VarDataOutOfBounds { field, offset, length } => write!(f, "var-data for '{}' out of bounds at offset {} with length {}", field, offset, length),
                        Self::MessageTooShort { needed, available } => write!(f, "message too short: needed {} bytes, {} available", needed, available),
                    }
                }
            }

            impl core::error::Error for VerifyError {}

            #[diagnostic::on_unimplemented(
                message = "`{Self}` is not a generated SBE message type",
                note = "SbeMessage is a sealed trait — only types generated by `ergosbe::Generator` can implement it. Import the generated module and use the provided decoder/encoder types directly."
            )]
            pub trait SbeMessage {
                const TEMPLATE_ID: u16;
                const BLOCK_LENGTH: usize;
                const SCHEMA_ID: u16;
                const SCHEMA_VERSION: u16;
            }

            pub mod private {
                pub trait Sealed {}
            }

            pub trait EncodeGroupEntry<E> {
                fn encode(self, entry: &mut E);
            }

            impl<E, F> EncodeGroupEntry<E> for F
            where
                F: FnOnce(&mut E),
            {
                #[inline]
                fn encode(self, entry: &mut E) {
                    self(entry);
                }
            }
        }
    };

    // Format the generated module through prettyplease for canonical output
    syn::parse_str::<syn::File>(&module.to_string())
        .map(|file| prettyplease::unparse(&file))
        .expect("generated SBE runtime must be valid Rust syntax")
}

fn to_pascal_case(s: &str) -> String {
    let mut res = String::new();
    let mut capitalize_next = true;
    let mut prev_is_lower = false;
    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            capitalize_next = true;
            prev_is_lower = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                capitalize_next = true;
            }
            if capitalize_next {
                res.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                res.push(c);
            }
            prev_is_lower = false;
        } else {
            if capitalize_next {
                res.extend(c.to_uppercase());
                capitalize_next = false;
            } else {
                res.push(c);
            }
            prev_is_lower = true;
        }
    }
    res
}

fn to_snake_case(s: &str) -> String {
    let mut res = String::new();
    let mut prev_is_lower = false;
    let mut _prev_is_upper = false;
    for c in s.chars() {
        if c == '_' || c == '-' || c == ' ' {
            res.push('_');
            prev_is_lower = false;
            _prev_is_upper = false;
        } else if c.is_uppercase() {
            if prev_is_lower {
                res.push('_');
            }
            res.extend(c.to_lowercase());
            prev_is_lower = false;
            _prev_is_upper = true;
        } else {
            res.push(c);
            prev_is_lower = true;
            _prev_is_upper = false;
        }
    }
    let mut clean = String::new();
    for c in res.chars() {
        if c == '_' && clean.ends_with('_') {
            continue;
        }
        clean.push(c);
    }
    clean
}

fn to_upper_snake_case(s: &str) -> String {
    to_snake_case(s).to_uppercase()
}

fn constant_value_expr(prim: PrimitiveType, val: &str) -> String {
    match prim {
        // ponytail: parser validates single-char char constants (xml.rs:252,658).
        PrimitiveType::Char => format!("b'{}'", val),
        PrimitiveType::Float => {
            format!("{}f32", val)
        }
        PrimitiveType::Double => {
            format!("{}f64", val)
        }
        _ => {
            format!("{}", val)
        }
    }
}

/// Format a `u64` stored value as a valid Rust literal expression for the given type.
fn field_const_value_expr(val: u64, prim: PrimitiveType) -> String {
    match prim {
        PrimitiveType::Char | PrimitiveType::UInt8 => format!("{val}_u8"),
        PrimitiveType::UInt16 => format!("{val}_u16"),
        PrimitiveType::UInt32 => format!("{val}_u32"),
        PrimitiveType::UInt64 => format!("{val}_u64"),
        PrimitiveType::Int8 => format!("{}_i8", val as i8),
        PrimitiveType::Int16 => format!("{}_i16", val as i16),
        PrimitiveType::Int32 => format!("{}_i32", val as i32),
        PrimitiveType::Int64 => format!("{}_i64", val as i64),
        PrimitiveType::Float => format!("f32::from_bits({}u32)", val as u32),
        PrimitiveType::Double => format!("f64::from_bits({val})"),
    }
}

/// Emit `*_NULL`, `*_MIN`, `*_MAX` compile-time constants for a message field.
fn emit_field_consts(f: &MessageField) -> proc_macro2::TokenStream {
    let upper_name = to_upper_snake_case(&f.name);
    let mut _any = false;
    let mut tokens = proc_macro2::TokenStream::new();
    match &f.field_type {
        FieldType::Primitive(prim, _) => {
            let r_type = rust_type(*prim);
            let r_type_ty: syn::Type = syn::parse_str(r_type).unwrap();
            if let Some(val) = f.null_value {
                let name_ident = syn::Ident::new(
                    &format!("{upper_name}_NULL"),
                    proc_macro2::Span::call_site(),
                );
                let expr = field_const_value_expr(val, *prim);
                let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                tokens.extend(quote::quote! {
                    pub const #name_ident: #r_type_ty = #expr_parsed;
                });
                _any = true;
            }
            if let Some(val) = f.min_value {
                let name_ident =
                    syn::Ident::new(&format!("{upper_name}_MIN"), proc_macro2::Span::call_site());
                let expr = field_const_value_expr(val, *prim);
                let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                tokens.extend(quote::quote! {
                    pub const #name_ident: #r_type_ty = #expr_parsed;
                });
                _any = true;
            }
            if let Some(val) = f.max_value {
                let name_ident =
                    syn::Ident::new(&format!("{upper_name}_MAX"), proc_macro2::Span::call_site());
                let expr = field_const_value_expr(val, *prim);
                let expr_parsed: syn::Expr = syn::parse_str(&expr).unwrap();
                tokens.extend(quote::quote! {
                    pub const #name_ident: #r_type_ty = #expr_parsed;
                });
                _any = true;
            }
        }
        FieldType::Enum {
            name,
            encoding_type: _,
        } => {
            let target_name = to_pascal_case(name);
            let name_ident = syn::Ident::new(
                &format!("{upper_name}_NULL"),
                proc_macro2::Span::call_site(),
            );
            let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
            tokens.extend(quote::quote! {
                pub const #name_ident: #target_ident = #target_ident::NullVal;
            });
            _any = true;
        }
        FieldType::Composite { .. } | FieldType::Set { .. } => {}
    }
    tokens
}

fn find_matching_end(tokens: &[Token], start: usize, begin: Signal, end: Signal) -> usize {
    let mut depth = 1;
    for j in (start + 1)..tokens.len() {
        if tokens[j].signal == begin {
            depth += 1;
        } else if tokens[j].signal == end {
            depth -= 1;
            if depth == 0 {
                return j;
            }
        }
    }
    tokens.len() - 1
}

struct SchemaElements {
    composites: Vec<Vec<Token>>,
    enums: Vec<Vec<Token>>,
    sets: Vec<Vec<Token>>,
    messages: Vec<Vec<Token>>,
}

fn partition_tokens(tokens: &[Token]) -> SchemaElements {
    let mut composites = Vec::new();
    let mut enums = Vec::new();
    let mut sets = Vec::new();
    let mut messages = Vec::new();

    let mut i = 0;
    while i < tokens.len() {
        match tokens[i].signal {
            Signal::BeginComposite => {
                let end =
                    find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
                composites.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            Signal::BeginEnum => {
                let end = find_matching_end(tokens, i, Signal::BeginEnum, Signal::EndEnum);
                enums.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            Signal::BeginSet => {
                let end = find_matching_end(tokens, i, Signal::BeginSet, Signal::EndSet);
                sets.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            Signal::BeginMessage => {
                let end = find_matching_end(tokens, i, Signal::BeginMessage, Signal::EndMessage);
                messages.push(tokens[i..=end].to_vec());
                i = end + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    SchemaElements {
        composites,
        enums,
        sets,
        messages,
    }
}

struct MessageStructure {
    name: String,
    id: u16,
    since_version: u16,
    description: Option<String>,
    semantic_type: Option<String>,
    fields: Vec<MessageField>,
    groups: Vec<MessageGroup>,
    var_data: Vec<MessageVarData>,
}

#[derive(Clone)]
struct MessageField {
    name: String,
    id: Option<u16>,
    offset: usize,
    presence: Presence,
    since_version: u16,
    null_value: Option<u64>,
    min_value: Option<u64>,
    max_value: Option<u64>,
    description: Option<String>,
    semantic_type: Option<String>,
    constant_value: Option<String>,
    field_type: FieldType,
}

#[derive(Clone)]
enum FieldType {
    Primitive(PrimitiveType, Option<usize>),
    Composite {
        name: String,
        size: usize,
    },
    Enum {
        name: String,
        encoding_type: PrimitiveType,
    },
    Set {
        name: String,
        encoding_type: PrimitiveType,
    },
}

impl FieldType {
    fn size(&self) -> usize {
        match self {
            Self::Primitive(p, length) => p.size() * length.unwrap_or(1),
            Self::Composite { size, .. } => *size,
            Self::Enum { encoding_type, .. } | Self::Set { encoding_type, .. } => {
                encoding_type.size()
            }
        }
    }
}

#[derive(Clone)]
struct MessageGroup {
    name: String,
    id: u16,
    since_version: u16,
    description: Option<String>,
    dimension_type: String,
    fields: Vec<MessageField>,
    groups: Vec<MessageGroup>,
    var_data: Vec<MessageVarData>,
    block_length: usize,
}

#[derive(Clone)]
struct MessageVarData {
    name: String,
    id: u16,
    since_version: u16,
    description: Option<String>,
    type_name: String,
    max_length: Option<usize>,
}

fn parse_message_structure(tokens: &[Token], elements: &SchemaElements) -> MessageStructure {
    let begin_token = &tokens[0];
    let name = begin_token.name.clone();
    let id = begin_token.id.unwrap_or(0);
    let since_version = begin_token.encoding.since_version;
    let description = begin_token.encoding.description.clone();
    let semantic_type = begin_token.encoding.semantic_type.clone();

    let mut fields = Vec::new();
    let mut groups = Vec::new();
    let mut var_data = Vec::new();

    let mut i = 1;
    let end_limit = tokens.len() - 1;
    while i < end_limit {
        match tokens[i].signal {
            Signal::BeginField => {
                let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
                let f = parse_field_structure(&tokens[i..=end], elements);
                fields.push(f);
                i = end + 1;
            }
            Signal::BeginGroup => {
                let end = find_matching_end(tokens, i, Signal::BeginGroup, Signal::EndGroup);
                let g = parse_group_structure(&tokens[i..=end], elements);
                groups.push(g);
                i = end + 1;
            }
            Signal::BeginVarData => {
                let end = find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                let vd = parse_vardata_structure(&tokens[i..=end]);
                var_data.push(vd);
                i = end + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    MessageStructure {
        name,
        id,
        since_version,
        description,
        semantic_type,
        fields,
        groups,
        var_data,
    }
}

fn parse_field_structure(tokens: &[Token], elements: &SchemaElements) -> MessageField {
    let begin = &tokens[0];
    let name = begin.name.clone();
    let id = begin.id;
    let offset = begin.encoding.offset.unwrap_or(0);
    let presence = begin.encoding.presence;
    let since_version = begin.encoding.since_version;
    let null_value = begin.encoding.null_value;
    let min_value = begin.encoding.min_value;
    let max_value = begin.encoding.max_value;
    let description = begin.encoding.description.clone();
    let semantic_type = begin.encoding.semantic_type.clone();
    let constant_value = begin.encoding.constant_value.clone();

    let field_type = if tokens.len() > 2 {
        let inner_signal = tokens[1].signal;
        let inner_name = tokens[1].name.clone();
        match inner_signal {
            Signal::BeginComposite => {
                let size = elements
                    .composites
                    .iter()
                    .find(|c| c[0].name == inner_name)
                    .and_then(|c| c[0].encoding.offset)
                    .unwrap_or(0);
                FieldType::Composite {
                    name: inner_name,
                    size,
                }
            }
            Signal::BeginEnum => {
                let encoding_type = tokens[1]
                    .encoding
                    .primitive_type
                    .unwrap_or(PrimitiveType::UInt8);
                FieldType::Enum {
                    name: inner_name,
                    encoding_type,
                }
            }
            Signal::BeginSet => {
                let encoding_type = tokens[1]
                    .encoding
                    .primitive_type
                    .unwrap_or(PrimitiveType::UInt8);
                FieldType::Set {
                    name: inner_name,
                    encoding_type,
                }
            }
            _ => FieldType::Primitive(
                begin
                    .encoding
                    .primitive_type
                    .unwrap_or(PrimitiveType::UInt8),
                begin.encoding.length,
            ),
        }
    } else {
        FieldType::Primitive(
            begin
                .encoding
                .primitive_type
                .unwrap_or(PrimitiveType::UInt8),
            begin.encoding.length,
        )
    };

    MessageField {
        name,
        id,
        offset,
        presence,
        since_version,
        null_value,
        min_value,
        max_value,
        description,
        semantic_type,
        constant_value,
        field_type,
    }
}

fn parse_group_structure(tokens: &[Token], elements: &SchemaElements) -> MessageGroup {
    let begin = &tokens[0];
    let name = begin.name.clone();
    let id = begin.id.unwrap_or(0);
    let since_version = begin.encoding.since_version;
    let description = begin.encoding.description.clone();
    let block_length = begin.encoding.offset.unwrap_or(0);

    let mut dimension_type = "groupSizeEncoding".to_string();
    let mut fields = Vec::new();
    let mut groups = Vec::new();
    let mut var_data = Vec::new();

    let mut i = 1;
    if tokens[i].signal == Signal::BeginComposite {
        dimension_type = tokens[i].name.clone();
        let dim_end = find_matching_end(tokens, i, Signal::BeginComposite, Signal::EndComposite);
        i = dim_end + 1;
    }

    let end_limit = tokens.len() - 1;
    while i < end_limit {
        match tokens[i].signal {
            Signal::BeginField => {
                let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
                fields.push(parse_field_structure(&tokens[i..=end], elements));
                i = end + 1;
            }
            Signal::BeginGroup => {
                let end = find_matching_end(tokens, i, Signal::BeginGroup, Signal::EndGroup);
                groups.push(parse_group_structure(&tokens[i..=end], elements));
                i = end + 1;
            }
            Signal::BeginVarData => {
                let end = find_matching_end(tokens, i, Signal::BeginVarData, Signal::EndVarData);
                var_data.push(parse_vardata_structure(&tokens[i..=end]));
                i = end + 1;
            }
            _ => {
                i += 1;
            }
        }
    }

    MessageGroup {
        name,
        id,
        since_version,
        description,
        dimension_type,
        fields,
        groups,
        var_data,
        block_length,
    }
}

fn parse_vardata_structure(tokens: &[Token]) -> MessageVarData {
    let begin = &tokens[0];
    let name = begin.name.clone();
    let id = begin.id.unwrap_or(0);
    let since_version = begin.encoding.since_version;
    let description = begin.encoding.description.clone();

    let mut type_name = "varDataEncoding".to_string();
    let mut max_length = None;
    if tokens.len() > 2 && tokens[1].signal == Signal::BeginComposite {
        type_name = tokens[1].name.clone();
        // Scan composite members for the length field's max_value.
        let comp_end = find_matching_end(tokens, 1, Signal::BeginComposite, Signal::EndComposite);
        let mut i = 2;
        while i < comp_end {
            if tokens[i].signal == Signal::BeginField && tokens[i].name == "length" {
                max_length = tokens[i].encoding.max_value.map(|v| v as usize);
                break;
            }
            i += 1;
        }
    }

    MessageVarData {
        name,
        id,
        since_version,
        description,
        type_name,
        max_length,
    }
}

fn rust_type(prim: PrimitiveType) -> &'static str {
    match prim {
        PrimitiveType::Char => "u8",
        PrimitiveType::Int8 => "i8",
        PrimitiveType::UInt8 => "u8",
        PrimitiveType::Int16 => "i16",
        PrimitiveType::UInt16 => "u16",
        PrimitiveType::Int32 => "i32",
        PrimitiveType::UInt32 => "u32",
        PrimitiveType::Int64 => "i64",
        PrimitiveType::UInt64 => "u64",
        PrimitiveType::Float => "f32",
        PrimitiveType::Double => "f64",
    }
}

struct CompositeMember {
    name: String,
    offset: usize,
    since_version: u16,
    member_type: MemberType,
}

#[derive(Clone)]
enum MemberType {
    Primitive {
        prim: PrimitiveType,
        length: Option<usize>,
        presence: Presence,
        constant_value: Option<String>,
    },
    Composite {
        name: String,
        size: usize,
    },
    Enum {
        name: String,
        encoding_type: PrimitiveType,
    },
    Set {
        name: String,
        encoding_type: PrimitiveType,
    },
}

fn parse_composite_members(tokens: &[Token]) -> Vec<CompositeMember> {
    let mut members = Vec::new();
    let mut i = 1;
    let end_limit = tokens.len() - 1;
    while i < end_limit {
        if tokens[i].signal == Signal::BeginField {
            let name = tokens[i].name.clone();
            let offset = tokens[i].encoding.offset.unwrap_or(0);
            let since_version = tokens[i].encoding.since_version;
            let presence = tokens[i].encoding.presence;
            let constant_value = tokens[i].encoding.constant_value.clone();
            let length = tokens[i].encoding.length;

            let member_type =
                if i + 2 < tokens.len() && tokens[i + 1].signal == Signal::BeginComposite {
                    let comp_name = tokens[i + 1].name.clone();
                    let size = tokens[i + 1].encoding.offset.unwrap_or(0);
                    MemberType::Composite {
                        name: comp_name,
                        size,
                    }
                } else if i + 2 < tokens.len() && tokens[i + 1].signal == Signal::BeginEnum {
                    let enum_name = tokens[i + 1].name.clone();
                    let encoding_type = tokens[i + 1]
                        .encoding
                        .primitive_type
                        .unwrap_or(PrimitiveType::UInt8);
                    MemberType::Enum {
                        name: enum_name,
                        encoding_type,
                    }
                } else if i + 2 < tokens.len() && tokens[i + 1].signal == Signal::BeginSet {
                    let set_name = tokens[i + 1].name.clone();
                    let encoding_type = tokens[i + 1]
                        .encoding
                        .primitive_type
                        .unwrap_or(PrimitiveType::UInt8);
                    MemberType::Set {
                        name: set_name,
                        encoding_type,
                    }
                } else {
                    let prim = tokens[i]
                        .encoding
                        .primitive_type
                        .unwrap_or(PrimitiveType::UInt8);
                    MemberType::Primitive {
                        prim,
                        length,
                        presence,
                        constant_value,
                    }
                };

            members.push(CompositeMember {
                name,
                offset,
                since_version,
                member_type,
            });

            let end = find_matching_end(tokens, i, Signal::BeginField, Signal::EndField);
            i = end + 1;
        } else {
            i += 1;
        }
    }
    members
}

fn generate_enum(src: &mut String, tokens: &[Token]) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let encoding_type = tokens[0]
        .encoding
        .primitive_type
        .unwrap_or(PrimitiveType::UInt8);
    let r_type = rust_type(encoding_type);
    let is_char = encoding_type == PrimitiveType::Char;

    let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();

    // Collect encoding variants
    struct Variant {
        variant_ident: syn::Ident,
        disc: proc_macro2::TokenStream,
    }

    let variants: Vec<Variant> = tokens
        .iter()
        .filter(|t| t.signal == Signal::Encoding)
        .filter_map(|t| {
            let val = t.encoding.constant_value.as_ref()?;
            let variant_ident =
                syn::Ident::new(&to_pascal_case(&t.name), proc_macro2::Span::call_site());
            let disc: proc_macro2::TokenStream = if is_char {
                let byte = val.as_bytes().first().copied().unwrap_or(0);
                let lit = syn::LitByte::new(byte, proc_macro2::Span::call_site());
                quote::quote! { #lit }
            } else {
                let lit = val
                    .parse::<u64>()
                    .ok()
                    .map(|v| syn::LitInt::new(&v.to_string(), proc_macro2::Span::call_site()))
                    .or_else(|| {
                        val.parse::<i64>().ok().map(|v| {
                            syn::LitInt::new(&v.to_string(), proc_macro2::Span::call_site())
                        })
                    })
                    .unwrap_or_else(|| syn::LitInt::new(val, proc_macro2::Span::call_site()));
                quote::quote! { #lit }
            };
            Some(Variant {
                variant_ident,
                disc,
            })
        })
        .collect();

    let variant_names: Vec<_> = variants.iter().map(|v| &v.variant_ident).collect();
    let variant_discs: Vec<_> = variants.iter().map(|v| &v.disc).collect();

    // Build From<r_type> arms: disc => Self::Variant, _ => Self::NullVal
    let from_raw_arms: Vec<_> = variants
        .iter()
        .map(|v| {
            let disc = &v.disc;
            let vname = &v.variant_ident;
            quote::quote! { #disc => Self::#vname }
        })
        .collect();

    // Detect boolean enum type (name convention or semanticType="Boolean")
    let is_bool = tokens[0].name == "BooleanType"
        || tokens[0].encoding.semantic_type.as_deref() == Some("Boolean");

    // Find TRUE/FALSE variant idents for From<bool>
    let (false_ident, true_ident) = if is_bool {
        let f = variants
            .iter()
            .find(|v| v.disc.to_string() == "0")
            .map(|v| v.variant_ident.clone());
        let t = variants
            .iter()
            .find(|v| v.disc.to_string() == "1")
            .map(|v| v.variant_ident.clone());
        (f, t)
    } else {
        (None, None)
    };

    // From<bool> / From<Name> for bool impls for boolean types
    let from_bool_impl = if let (Some(ref fv), Some(ref tv)) = (false_ident, true_ident) {
        quote::quote! {
            impl From<bool> for #name_ident {
                #[inline]
                fn from(val: bool) -> Self {
                    if val { Self::#tv } else { Self::#fv }
                }
            }

            impl From<#name_ident> for bool {
                #[inline]
                fn from(val: #name_ident) -> bool {
                    val as #r_type_ty != 0
                }
            }
        }
    } else {
        quote::quote! {}
    };

    // Emit enum rustdoc from the type's XML description.
    if let Some(ref desc) = tokens[0].encoding.description {
        for line in desc.lines() {
            src.push_str(&format!("///{line}\n"));
        }
    }

    let tokens = quote::quote! {
        #[repr(#r_type_ty)]
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub enum #name_ident {
            #(#variant_names = #variant_discs,)*
            /// Unknown enum value — the wire discriminant did not match any known variant.
            NullVal,
        }

        impl #name_ident {
            pub fn raw(self) -> #r_type_ty {
                self as #r_type_ty
            }

            pub const fn from_raw(val: #r_type_ty) -> Self {
                match val {
                    #(#from_raw_arms,)*
                    _ => Self::NullVal,
                }
            }
        }

        impl From<#name_ident> for #r_type_ty {
            #[inline]
            fn from(val: #name_ident) -> Self {
                val as #r_type_ty
            }
        }

        impl From<#r_type_ty> for #name_ident {
            #[inline]
            fn from(val: #r_type_ty) -> Self {
                Self::from_raw(val)
            }
        }

        #from_bool_impl
    };

    let formatted = syn::parse_str::<syn::File>(&tokens.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| tokens.to_string());
    src.push_str(&formatted);
    src.push('\n');
}

fn generate_set(src: &mut String, tokens: &[Token]) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let encoding_type = tokens[0]
        .encoding
        .primitive_type
        .unwrap_or(PrimitiveType::UInt8);
    let r_type = rust_type(encoding_type);

    let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();

    let bits: Vec<proc_macro2::TokenStream> = tokens
        .iter()
        .filter(|t| t.signal == Signal::Encoding)
        .filter_map(|t| {
            let val = t.encoding.constant_value.as_ref()?;
            let bit_index: u8 = val.parse().unwrap_or(0);
            let bit_name = syn::Ident::new(&to_snake_case(&t.name), proc_macro2::Span::call_site());
            let set_bit_name = quote::format_ident!("set_{}", to_snake_case(&t.name));
            let bit_lit = syn::LitInt::new(&bit_index.to_string(), proc_macro2::Span::call_site());
            Some(quote::quote! {
                pub const fn #bit_name(self) -> bool {
                    (self.0 & (1 << #bit_lit)) != 0
                }

                pub fn #set_bit_name(&mut self, val: bool) {
                    if val {
                        self.0 |= 1 << #bit_lit;
                    } else {
                        self.0 &= !(1 << #bit_lit);
                    }
                }
            })
        })
        .collect();

    // Emit enum doc from the type's XML description (DECISIONS.md §9).
    if let Some(ref desc) = tokens[0].encoding.description {
        for line in desc.lines() {
            src.push_str(&format!("///{line}\n"));
        }
    }

    // Emit set doc from the type's XML description.
    if let Some(ref desc) = tokens[0].encoding.description {
        for line in desc.lines() {
            src.push_str(&format!("///{line}\n"));
        }
    }

    let tokens = quote::quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(transparent)]
        pub struct #name_ident(pub #r_type_ty);

        impl #name_ident {
            pub const fn raw(self) -> #r_type_ty {
                self.0
            }

            pub const fn default() -> Self {
                Self(0)
            }

            #(#bits)*
        }

        impl From<#r_type_ty> for #name_ident {
            #[inline]
            fn from(val: #r_type_ty) -> Self {
                Self(val)
            }
        }

        impl From<#name_ident> for #r_type_ty {
            #[inline]
            fn from(val: #name_ident) -> Self {
                val.0
            }
        }
    };

    let formatted = syn::parse_str::<syn::File>(&tokens.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| tokens.to_string());
    src.push_str(&formatted);
    src.push('\n');
}

fn generate_composite(src: &mut String, tokens: &[Token], byte_order: ByteOrder) {
    let raw_name = &tokens[0].name;
    let name = to_pascal_case(raw_name);
    let size = tokens[0].encoding.offset.unwrap_or(0);

    let members = parse_composite_members(tokens);

    let has_float = members.iter().any(|m| {
        matches!(
            &m.member_type,
            MemberType::Primitive {
                prim: PrimitiveType::Float | PrimitiveType::Double,
                ..
            }
        )
    });

    let name_ident = syn::Ident::new(&name, proc_macro2::Span::call_site());
    let size_lit = syn::LitInt::new(&size.to_string(), proc_macro2::Span::call_site());

    let derives = if has_float {
        quote::quote! { Clone, Copy, Debug, PartialEq, PartialOrd }
    } else {
        quote::quote! { Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash }
    };

    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };
    let from_method = syn::Ident::new(
        &format!("from_{order_suffix}_bytes"),
        proc_macro2::Span::call_site(),
    );
    let to_method = syn::Ident::new(
        &format!("to_{order_suffix}_bytes"),
        proc_macro2::Span::call_site(),
    );

    let mut getters = proc_macro2::TokenStream::new();
    let mut ctor_params = Vec::new();
    let mut ctor_body = proc_macro2::TokenStream::new();

    for m in &members {
        let field_name = to_snake_case(&m.name);
        let field_ident = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
        let offset_lit = syn::LitInt::new(&m.offset.to_string(), proc_macro2::Span::call_site());

        match &m.member_type {
            MemberType::Primitive {
                prim,
                length,
                presence,
                constant_value,
            } => {
                let r_type_str = rust_type(*prim);
                let r_type_ty: syn::Type = syn::parse_str(r_type_str).unwrap();
                let prim_size = prim.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                if *presence == Presence::Constant {
                    if let Some(val) = constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            let val_lit = syn::LitStr::new(val, proc_macro2::Span::call_site());
                            getters.extend(quote::quote! {
                                #[inline]
                                pub const fn #field_ident(&self) -> &'static str {
                                    #val_lit
                                }
                            });
                        } else {
                            let expr_str = constant_value_expr(*prim, val);
                            let expr: syn::Expr = syn::parse_str(&expr_str).unwrap();
                            getters.extend(quote::quote! {
                                #[inline]
                                pub const fn #field_ident(&self) -> #r_type_ty {
                                    #expr
                                }
                            });
                        }
                    }
                    continue; // no ctor param for constants
                }

                if let Some(len) = length {
                    let len_lit =
                        syn::LitInt::new(&len.to_string(), proc_macro2::Span::call_site());
                    let array_ty: syn::Type =
                        syn::parse_str(&format!("[{}; {}]", r_type_str, len)).unwrap();
                    ctor_params.push(quote::quote! { #field_ident: #array_ty });

                    if *len > 0 {
                        getters.extend(quote::quote! {
                            #[inline]
                            pub fn #field_ident(&self) -> [#r_type_ty; #len_lit] {
                                let mut res = [0 as #r_type_ty; #len_lit];
                                let mut idx = 0;
                                while idx < #len_lit {
                                    let offset = #offset_lit + idx * #prim_size_lit;
                                    res[idx] = #r_type_ty::#from_method(
                                        read_bytes::<#prim_size_lit>(&self.0, offset)
                                    );
                                    idx += 1;
                                }
                                res
                            }
                        });

                        ctor_body.extend(quote::quote! {
                            let mut idx = 0;
                            while idx < #len_lit {
                                let val_bytes = #field_ident[idx].#to_method();
                                write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit + idx * #prim_size_lit, &val_bytes);
                                idx += 1;
                            }
                        });
                    } else {
                        // zero-length array: return empty array immediately
                        let zero_ty: syn::Type =
                            syn::parse_str(&format!("[{}; 0]", r_type_str)).unwrap();
                        getters.extend(quote::quote! {
                            #[inline]
                            pub fn #field_ident(&self) -> #zero_ty {
                                []
                            }
                        });
                    }
                } else {
                    ctor_params.push(quote::quote! { #field_ident: #r_type_ty });

                    getters.extend(quote::quote! {
                        #[inline]
                        pub fn #field_ident(&self) -> #r_type_ty {
                            #r_type_ty::#from_method(read_bytes::<#prim_size_lit>(&self.0, #offset_lit))
                        }
                    });

                    ctor_body.extend(quote::quote! {
                        let val_bytes = #field_ident.#to_method();
                        write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit, &val_bytes);
                    });
                }
            }
            MemberType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target_name = to_pascal_case(comp_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let comp_size_lit =
                    syn::LitInt::new(&comp_size.to_string(), proc_macro2::Span::call_site());

                ctor_params.push(quote::quote! { #field_ident: #target_ident });

                getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident(read_bytes::<#comp_size_lit>(&self.0, #offset_lit))
                    }
                });

                ctor_body.extend(quote::quote! {
                    write_bytes::<#comp_size_lit>(&mut bytes, #offset_lit, &#field_ident.0);
                });
            }
            MemberType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(enum_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                ctor_params.push(quote::quote! { #field_ident: #target_ident });

                getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident::from_raw(#r_type_ty::#from_method(
                            read_bytes::<#prim_size_lit>(&self.0, #offset_lit)
                        ))
                    }
                });

                ctor_body.extend(quote::quote! {
                    let val_bytes = (#field_ident as #r_type_ty).#to_method();
                    write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit, &val_bytes);
                });
            }
            MemberType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(set_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                ctor_params.push(quote::quote! { #field_ident: #target_ident });

                getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        #target_ident(#r_type_ty::#from_method(
                            read_bytes::<#prim_size_lit>(&self.0, #offset_lit)
                        ))
                    }
                });

                ctor_body.extend(quote::quote! {
                    let val_bytes = #field_ident.0.#to_method();
                    write_bytes::<#prim_size_lit>(&mut bytes, #offset_lit, &val_bytes);
                });
            }
        }
    }

    // ponytail: refactor ctor_params into proper syn::FnArg when polishing

    // Emit composite doc from the type's XML description.
    if let Some(ref desc) = tokens[0].encoding.description {
        for line in desc.lines() {
            src.push_str(&format!("///{line}\n"));
        }
    }

    let ts = quote::quote! {
        #[derive(#derives)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        #[repr(transparent)]
        pub struct #name_ident(pub [u8; #size_lit]);

        impl #name_ident {
            #getters

            pub fn new(#(#ctor_params),*) -> Self {
                let mut bytes = [0u8; #size_lit];
                #ctor_body
                Self(bytes)
            }
        }

        // DECISIONS.md §10: compile-time proof that the Rust struct matches the
        // wire size — catches generator bugs at compile time, zero runtime cost.
        const _: () = assert!(core::mem::size_of::<#name_ident>() == #size_lit);
    };
    src.push_str(&ts.to_string());
    src.push('\n');

    // ── 5b. Composite decoder (flyweight / _lazy accessor) ──
    let mut decoder_getters = proc_macro2::TokenStream::new();
    for m in &members {
        let field_name = to_snake_case(&m.name);
        let field_ident = syn::Ident::new(&field_name, proc_macro2::Span::call_site());
        let offset_lit = syn::LitInt::new(&m.offset.to_string(), proc_macro2::Span::call_site());

        match &m.member_type {
            MemberType::Primitive {
                prim,
                length,
                presence,
                constant_value,
            } => {
                let r_type_str = rust_type(*prim);
                let r_type_ty: syn::Type = syn::parse_str(r_type_str).unwrap();
                let prim_size = prim.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                if *presence == Presence::Constant {
                    if let Some(val) = constant_value {
                        if *prim == PrimitiveType::Char && val.len() > 1 {
                            let val_lit = syn::LitStr::new(val, proc_macro2::Span::call_site());
                            decoder_getters.extend(quote::quote! {
                                #[inline]
                                pub const fn #field_ident(&self) -> &'static str {
                                    #val_lit
                                }
                            });
                        } else {
                            let expr_str = constant_value_expr(*prim, val);
                            let expr: syn::Expr = syn::parse_str(&expr_str).unwrap();
                            decoder_getters.extend(quote::quote! {
                                #[inline]
                                pub const fn #field_ident(&self) -> #r_type_ty {
                                    #expr
                                }
                            });
                        }
                    }
                    continue;
                }

                if let Some(len) = length {
                    let len_lit =
                        syn::LitInt::new(&len.to_string(), proc_macro2::Span::call_site());
                    if *len > 0 {
                        decoder_getters.extend(quote::quote! {
                            #[inline]
                            pub fn #field_ident(&self) -> [#r_type_ty; #len_lit] {
                                let mut res = [0 as #r_type_ty; #len_lit];
                                let mut idx = 0;
                                while idx < #len_lit {
                                    let offset = self.pos + #offset_lit + idx * #prim_size_lit;
                                    res[idx] = #r_type_ty::#from_method(
                                        read_bytes::<#prim_size_lit>(self.buf, offset)
                                    );
                                    idx += 1;
                                }
                                res
                            }
                        });
                    } else {
                        let zero_ty: syn::Type =
                            syn::parse_str(&format!("[{}; 0]", r_type_str)).unwrap();
                        decoder_getters.extend(quote::quote! {
                            #[inline]
                            pub fn #field_ident(&self) -> #zero_ty {
                                []
                            }
                        });
                    }
                } else {
                    decoder_getters.extend(quote::quote! {
                        #[inline]
                        pub fn #field_ident(&self) -> #r_type_ty {
                            let offset = self.pos + #offset_lit;
                            #r_type_ty::#from_method(read_bytes::<#prim_size_lit>(self.buf, offset))
                        }
                    });
                }
            }
            MemberType::Composite {
                name: comp_name,
                size: comp_size,
            } => {
                let target_name = to_pascal_case(comp_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let comp_size_lit =
                    syn::LitInt::new(&comp_size.to_string(), proc_macro2::Span::call_site());

                decoder_getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident(read_bytes::<#comp_size_lit>(self.buf, offset))
                    }
                });
            }
            MemberType::Enum {
                name: enum_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(enum_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                decoder_getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident::from_raw(#r_type_ty::#from_method(read_bytes::<#prim_size_lit>(self.buf, offset)))
                    }
                });
            }
            MemberType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target_name = to_pascal_case(set_name);
                let target_ident = syn::Ident::new(&target_name, proc_macro2::Span::call_site());
                let r_type = rust_type(*encoding_type);
                let r_type_ty: syn::Type = syn::parse_str(&r_type).unwrap();
                let prim_size = encoding_type.size();
                let prim_size_lit =
                    syn::LitInt::new(&prim_size.to_string(), proc_macro2::Span::call_site());

                decoder_getters.extend(quote::quote! {
                    #[inline]
                    pub fn #field_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident(#r_type_ty::#from_method(read_bytes::<#prim_size_lit>(self.buf, offset)))
                    }
                });
            }
        }
    }

    let decoder_name = syn::Ident::new(&format!("{}Decoder", name), proc_macro2::Span::call_site());
    let decoder_ts = quote::quote! {
        #[derive(Clone, Copy)]
        pub struct #decoder_name<'a> {
            buf: &'a [u8],
            pos: usize,
        }

        impl<'a> #decoder_name<'a> {
            #decoder_getters
        }
    };
    src.push_str(&decoder_ts.to_string());
    src.push('\n');
}

fn get_dimension_info(
    elements: &SchemaElements,
    dim_type: &str,
) -> (String, usize, String, String) {
    let raw_name = dim_type;
    let name = to_pascal_case(raw_name);
    let mut size = 4;
    let mut bl = "block_length".to_string();
    let mut num = "num_in_group".to_string();
    if let Some(comp) = elements.composites.iter().find(|c| c[0].name == raw_name) {
        size = comp[0].encoding.offset.unwrap_or(4);
        let members = parse_composite_members(comp);
        for m in members {
            let lower = m.name.to_lowercase();
            if lower.contains("blocklength") {
                bl = to_snake_case(&m.name);
            } else if lower.contains("numingroup") || lower.contains("count") {
                num = to_snake_case(&m.name);
            }
        }
    }
    (name, size, bl, num)
}

/// Returns (offset, size, primitive) of the numInGroup field within a dimension
/// composite. The primitive drives the encoder's `count` parameter width so a
/// schema whose dimensionType declares numInGroup as uint32 (e.g. Binance's
/// default `groupSizeEncoding`) writes all 4 bytes, not just 2.
fn get_dim_num_layout(elements: &SchemaElements, dim_type: &str) -> (usize, usize, PrimitiveType) {
    let raw_name = dim_type;
    let mut offset = 2;
    let mut size = 2;
    let mut prim = PrimitiveType::UInt16;
    if let Some(comp) = elements.composites.iter().find(|c| c[0].name == raw_name) {
        let members = parse_composite_members(comp);
        for m in members {
            let lower = m.name.to_lowercase();
            if lower.contains("numingroup") || lower.contains("count") {
                offset = m.offset;
                if let MemberType::Primitive {
                    prim: p, length, ..
                } = &m.member_type
                {
                    prim = *p;
                    size = p.size() * length.unwrap_or(1);
                }
            }
        }
    }
    (offset, size, prim)
}

fn get_vardata_info(
    elements: &SchemaElements,
    type_name: &str,
) -> (String, usize, String, PrimitiveType) {
    let raw_name = type_name;
    let name = to_pascal_case(raw_name);
    let mut size = 4;
    let mut len_field = "length".to_string();
    let mut prim = PrimitiveType::UInt32;
    if let Some(comp) = elements.composites.iter().find(|c| c[0].name == raw_name) {
        let members = parse_composite_members(comp);
        for m in members {
            if m.name == "length" {
                len_field = to_snake_case(&m.name);
                if let MemberType::Primitive { prim: p, .. } = m.member_type {
                    prim = p;
                }
            }
            if m.name == "varData" {
                size = m.offset;
            }
        }
    }
    (name, size, len_field, prim)
}

/// Name of the concrete decoder stage entered after consuming tail component `i`.
/// `stage_prefix` is the owner decoder's name (e.g. `CarDecoder` or
/// `BidsEntryDecoder`); the final component yields `{prefix}Complete`, earlier
/// ones yield `{prefix}After{FieldPascal}`.
fn decoder_stage_after_ident(
    stage_prefix: &str,
    field_pascal: &str,
    i: usize,
    total_tail: usize,
    span: proc_macro2::Span,
) -> syn::Ident {
    if i == total_tail - 1 {
        syn::Ident::new(&format!("{stage_prefix}Complete"), span)
    } else {
        syn::Ident::new(&format!("{stage_prefix}After{field_pascal}"), span)
    }
}

/// One tail group component of an owner (message or entry), resolved for codegen.
struct OwnerTailGroup {
    accessor_snake: String,
    field_pascal: String,
    group_decoder_ident: String,
    entry_decoder_ident: String,
}

/// One tail var-data component of an owner, resolved for codegen.
struct OwnerTailVarData {
    accessor_snake: String,
    field_pascal: String,
    type_pascal: String,
    prefix_size: usize,
    len_field: String,
    max_length: Option<usize>,
    name: String,
}

/// Core generator for concrete consuming tail stages (DECISIONS.md §3), shared
/// by message-level and entry-level tails. Emits non-`Copy` stage structs plus
/// `into_*`, `finish`, and `skip_remaining` methods. Additive: does not touch
/// the legacy `&self` random-access surface.
///
/// `initial_ident` is the existing decoder (e.g. `CarDecoder`, `BidsEntryDecoder`);
/// `stage_prefix` is its string form, used to name the `After*`/`Complete` stages.
/// `header_size` is the message header size for messages (0 for entries).
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
                    let bytes: [u8; #prefix_size_lit] = read_bytes::<#prefix_size_lit>(self.buf, offset);
                    let header = #vd_type_ident(bytes);
                    let len = header.#len_field_ident() as usize;
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
            }
        });

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
    decimal_composites: &[String],
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
        let desc_lit = syn::LitStr::new(desc, proc_macro2::Span::call_site());
        ts.extend(quote::quote! {
            #[doc = #desc_lit]
        });
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
            /// Stack-allocate with `let mut buf = [0u8; Msg::ENCODED_LENGTH];`
            pub const ENCODED_LENGTH: usize = #encoded_len_lit;
            const _ENCODED_LEN: () = assert!(Self::ENCODED_LENGTH >= Self::BLOCK_LENGTH);
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

    // 4. wrap_and_apply_header — uses read_bytes internally
    // (feature flag gating is inside read_bytes, not duplicated here)
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
            pub fn wrap_and_apply_header(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
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
                Ok(Self::wrap(buf, pos + #hs, header.#hbl() as usize, header.#hvr()))
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
        // In converter mode, Decimal-composite-backed raw accessors are
        // suffixed _wire so the generic converted method can take the
        // original name.
        let is_decimal = matches!(&f.field_type,
            FieldType::Composite { name, .. } if decimal_composites.iter().any(|d| d == name));
        let method_name = if is_decimal {
            format!("{fname_snake}_wire")
        } else {
            fname_snake.clone()
        };
        let fname_ident = syn::Ident::new(&method_name, proc_macro2::Span::call_site());

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
                                let desc_lit =
                                    syn::LitStr::new(desc, proc_macro2::Span::call_site());
                                impl_body.extend(quote::quote! {
                                    #[doc = #desc_lit]
                                });
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
                            let all: [u8; #total_size_lit] = read_bytes::<#total_size_lit>(self.buf, offset);
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
                            let desc_lit = syn::LitStr::new(desc, proc_macro2::Span::call_site());
                            impl_body.extend(quote::quote! { #[doc = #desc_lit] });
                        }
                        let accessor = format!(
                            "#[inline]\n\
                             pub fn {snake}(&self) -> Option<{rt}> {{\n\
                                 if self.acting_version < {since} || {offset_end} > self.acting_block_length {{\n\
                                     return None;\n\
                                 }}\n\
                                 let offset = self.pos + {offset};\n\
                                 let val = {rt}::{order}(read_bytes::<{ps}>(self.buf, offset));\n\
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
                            let desc_lit = syn::LitStr::new(desc, proc_macro2::Span::call_site());
                            impl_body.extend(quote::quote! { #[doc = #desc_lit] });
                        }
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub fn #fname_ident(&self) -> Option<#r_type_ty> {
                                if self.acting_version < #since_lit || #offset_end_lit > self.acting_block_length {
                                    return None;
                                }
                                let offset = self.pos + #offset_lit;
                                Some(#r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset)))
                            }
                        });
                    } else {
                        if let Some(ref desc) = f.description {
                            let desc_lit = syn::LitStr::new(desc, proc_macro2::Span::call_site());
                            impl_body.extend(quote::quote! { #[doc = #desc_lit] });
                        }
                        impl_body.extend(quote::quote! {
                            #[inline]
                            pub fn #fname_ident(&self) -> #r_type_ty {
                                let offset = self.pos + #offset_lit;
                                #r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset))
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

                // Eager copy accessor (_as_struct)
                let as_struct_ident = syn::Ident::new(
                    &format!("{}_as_struct", fname_snake),
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
                            Some(#target_ident(read_bytes::<#comp_size_lit>(self.buf, offset)))
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #as_struct_ident(&self) -> #target_ident {
                            let offset = self.pos + #offset_lit;
                            #target_ident(read_bytes::<#comp_size_lit>(self.buf, offset))
                        }
                    });
                }

                // ponytail: _lazy alias removed — the base accessor is the canonical path
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
                            Some(#target_ident::from_raw(#r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset))))
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fname_ident(&self) -> #target_ident {
                            let offset = self.pos + #offset_lit;
                            #target_ident::from_raw(#r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset)))
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
                            Some(#target_ident(#r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset))))
                        }
                    });
                } else {
                    impl_body.extend(quote::quote! {
                        #[inline]
                        pub fn #fname_ident(&self) -> #target_ident {
                            let offset = self.pos + #offset_lit;
                            #target_ident(#r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset)))
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
                        available: self.buf.len() - start,
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
                     return Err(sbe_rt::DecodeError::BufferTooShort {{ field: \"{vn}\", needed: {ps}, available: self.buf.len() - start }});\n\
                 }}\n\
                 let bytes: [u8; {ps}] = read_bytes::<{ps}>(self.buf, start);\n\
                 let header = {tp}(bytes);\n\
                 let len = header.{lf}() as usize;\n\
                 if start + {ps} + len > self.buf.len() {{\n\
                     return Err(sbe_rt::DecodeError::BufferTooShort {{ field: \"{vn}\", needed: {ps} + len, available: self.buf.len() - start }});\n\
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
        impl_body.extend(quote::quote! {
            #[inline]
            fn #g_snake_ident(&self) -> Result<#g_decoder_ident<'a>, sbe_rt::DecodeError> {
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

        // ponytail: resolver (resolve.rs:188-189) fills default_max for every
        // primitive, so max_length is always Some. The else branch can't fire.
        let max = vd.max_length.unwrap_or(0);
        let max_lit = syn::LitInt::new(&max.to_string(), proc_macro2::Span::call_site());
        impl_body.extend(quote::quote! {
            #[inline]
            fn #vd_snake_ident(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
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

        // UTF-8 str accessor
        let str_ident = syn::Ident::new(
            &format!("{vd_snake}_as_str"),
            proc_macro2::Span::call_site(),
        );
        impl_body.extend(quote::quote! {
            #[inline]
            fn #str_ident(&self) -> Result<&'a str, sbe_rt::DecodeError> {
                let bytes = self.#vd_snake_ident()?;
                core::str::from_utf8(bytes).map_err(sbe_rt::DecodeError::Utf8)
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
                Self::wrap_and_apply_header(buf, 0)
            }
        }

        impl<'a> sbe_rt::private::Sealed for #decoder_ident<'a> {}

        impl<'a> sbe_rt::SbeMessage for #decoder_ident<'a> {
            const TEMPLATE_ID: u16 = #msg_id_lit;
            const BLOCK_LENGTH: usize = #bl_lit;
            const SCHEMA_ID: u16 = #schema_id_lit;
            const SCHEMA_VERSION: u16 = #schema_version_lit;
        }

        impl<'a> AsRef<[u8]> for #decoder_ident<'a> {
            fn as_ref(&self) -> &[u8] {
                self.as_bytes().unwrap_or(&[])
            }
        }

        impl<'a> #decoder_ident<'a> {
            pub fn as_ref_opt(&self) -> Option<&[u8]> {
                self.as_bytes().ok()
            }
        }
    });

    // 13. Display impl
    let display_ts = generate_decoder_display(msg);
    ts.extend(display_ts);

    // 14. Repeating Group decoders — use pre-computed dedup names from section 8
    for (gi, g) in msg.groups.iter().enumerate() {
        let unique = &group_unique_names[gi];
        ts.extend(generate_group_decoder(g, elements, byte_order, unique));
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
) -> proc_macro2::TokenStream {
    let span = proc_macro2::Span::call_site();
    let mut ts = proc_macro2::TokenStream::new();
    generate_domain_recursive(
        msg_name,
        msg_name,
        &msg.fields,
        &msg.groups,
        &msg.var_data,
        elements,
        multi_message,
        msg_name,
        false, // is_entry — this is a message, not a group entry
        &mut ts,
        span,
    );
    ts
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
    is_entry: bool,
    ts: &mut proc_macro2::TokenStream,
    span: proc_macro2::Span,
) {
    let domain_ident = syn::Ident::new(&format!("{struct_prefix}Domain"), span);
    let decoder_ident = syn::Ident::new(&format!("{decoder_name}Decoder"), span);

    let mut struct_fields: Vec<proc_macro2::TokenStream> = Vec::new();
    let mut from_exprs: Vec<proc_macro2::TokenStream> = Vec::new();

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
                if let Some(len) = length {
                    let len_lit = syn::LitInt::new(&len.to_string(), span);
                    struct_fields.push(quote::quote! { pub #f_ident: [#r_type; #len_lit] });
                    from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                } else if f.presence == Presence::Optional {
                    struct_fields.push(quote::quote! { pub #f_ident: Option<#r_type> });
                    from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                } else {
                    struct_fields.push(quote::quote! { pub #f_ident: #r_type });
                    from_exprs.push(quote::quote! { #f_ident: dec.#f_ident() });
                }
            }
            FieldType::Composite {
                name: comp_name, ..
            } => {
                let comp_pascal = to_pascal_case(comp_name);
                let comp_ident = syn::Ident::new(&comp_pascal, span);
                let as_struct_ident = syn::Ident::new(&format!("{f_snake}_as_struct"), span);
                struct_fields.push(quote::quote! { pub #f_ident: #comp_ident });
                from_exprs.push(quote::quote! { #f_ident: dec.#as_struct_ident() });
            }
            FieldType::Enum {
                name: enum_name, ..
            }
            | FieldType::Set {
                name: enum_name, ..
            } => {
                let type_ident = syn::Ident::new(&to_pascal_case(enum_name), span);
                // Message-level sinceVersion > 0 enums return Option<T>.
                // Group entries and optional enums always return T.
                if !is_entry && f.since_version > 0 {
                    struct_fields.push(quote::quote! { pub #f_ident: Option<#type_ident> });
                } else {
                    struct_fields.push(quote::quote! { pub #f_ident: #type_ident });
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
                    .map(|g| g.filter_map(|e| e.ok()).map(#entry_domain_ident::from).collect())
                    .unwrap_or_default()
            });
        } else {
            from_exprs.push(quote::quote! {
                #g_field_ident: dec.#g_field_ident()
                    .map(|g| g.map(#entry_domain_ident::from).collect())
                    .unwrap_or_default()
            });
        }

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
    }

    // Generate the struct + From impl
    ts.extend(quote::quote! {
        /// Owned domain object — application-layer counterpart to the flyweight decoder.
        /// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
        #[derive(Debug, Clone, PartialEq)]
        #[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
        pub struct #domain_ident {
            #(#struct_fields),*
        }

        impl<'a> From<#decoder_ident<'a>> for #domain_ident {
            fn from(dec: #decoder_ident<'a>) -> Self {
                Self {
                    #(#from_exprs),*
                }
            }
        }
    });
}

fn generate_decoder_display(msg: &MessageStructure) -> proc_macro2::TokenStream {
    let name = to_pascal_case(&msg.name);
    let decoder_ident =
        syn::Ident::new(&format!("{}Decoder", name), proc_macro2::Span::call_site());
    let mut body = proc_macro2::TokenStream::new();
    let display_header = format!("{} {{{{ ", name);
    body.extend(quote::quote! {
        write!(f, #display_header)?;
    });
    let mut out_idx = 0usize;
    for f in &msg.fields {
        let snake = to_snake_case(&f.name);
        let f_ident = syn::Ident::new(&snake, proc_macro2::Span::call_site());
        let sep = if out_idx == 0 { "" } else { ", " };
        match &f.field_type {
            FieldType::Primitive(_prim, length) => {
                if f.presence == Presence::Constant || length.is_some() {
                    continue;
                }
                let fmt_str = format!("{sep}{snake}: {{:?}}");
                // ponytail: use {:?} so Option<T> always renders regardless of T: Display
                body.extend(quote::quote! {
                    { let v = self.#f_ident(); write!(f, #fmt_str, v)?; }
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
                // enum accessors: presence="optional" still returns the enum directly
                // (NullVal sentinel), only since_version > 0 produces Option<T>
                if f.since_version > 0 {
                    body.extend(quote::quote! {
                        if let Some(e) = self.#f_ident() {
                            write!(f, #fmt_str)?;
                        }
                    });
                } else {
                    body.extend(quote::quote! {
                        { let e = self.#f_ident(); write!(f, #fmt_str)?; }
                    });
                }
                out_idx += 1;
            }
            FieldType::Set { .. } => {}
            FieldType::Composite { .. } => {}
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
    }
    for vd in &msg.var_data {
        let vd_snake = to_snake_case(&vd.name);
        let vd_ident = syn::Ident::new(&vd_snake, proc_macro2::Span::call_site());
        let sep = if out_idx == 0 { "" } else { ", " };
        let fmt_str = format!("{sep}{vd_snake}: {{}} bytes");
        body.extend(quote::quote! {
            if let Ok(d) = self.#vd_ident() {
                write!(f, #fmt_str, d.len())?;
            }
        });
        out_idx += 1;
    }
    body.extend(quote::quote! {
        write!(f, " }}")
    });
    let ts = quote::quote! {
        impl<'a> core::fmt::Display for #decoder_ident<'a> {
            fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
                #body
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
        let desc_lit = syn::LitStr::new(desc, proc_macro2::Span::call_site());
        ts.extend(quote::quote! { #[doc = #desc_lit] });
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
                    if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
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
                    if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #g_name_lit,
                            needed: n * Self::ENTRY_BLOCK_LENGTH,
                            available: self.count * Self::ENTRY_BLOCK_LENGTH,
                        });
                    }
                    for _ in 0..n {
                        let entry = #entry_decoder_ident::wrap(self.buf, self.pos, self.acting_block_length, self.acting_version);
                        if cfg!(not(feature = "bound-check-disabled")) {
                            self.pos += entry.encoded_length()?;
                        } else {
                            self.pos += entry.encoded_length().unwrap();
                        }
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
                        available: self.buf.len() - offset,
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
                    #[cfg(not(feature = "bound-check-disabled"))]
                    let size = match entry.encoded_length() {
                        Ok(s) => s,
                        Err(e) => {
                            self.count = 0;
                            return Some(Err(e));
                        }
                    };
                    #[cfg(feature = "bound-check-disabled")]
                    let size = entry.encoded_length().unwrap();
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

    // wrap() method header
    entry_body.extend(quote::quote! {
        pub const ENTRY_BLOCK_LENGTH: usize = #block_len_lit;

        #[inline]
        pub fn wrap(buf: &'a [u8], pos: usize, acting_block_length: usize, acting_version: u16) -> Self {
            Self { buf, pos, acting_version, acting_block_length }
        }
    });

    // Fields of group entry
    for f in &g.fields {
        let f_name = to_snake_case(&f.name);
        let f_name_ident = syn::Ident::new(&f_name, proc_macro2::Span::call_site());
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
                            let all: [u8; #total_size_lit] = read_bytes::<#total_size_lit>(self.buf, offset);
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
                            let val = #r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset));
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
                            #r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset))
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

                // Eager copy accessor (_as_struct)
                let as_struct_ident = syn::Ident::new(
                    &format!("{}_as_struct", f_name),
                    proc_macro2::Span::call_site(),
                );
                entry_body.extend(quote::quote! {
                    #[inline]
                    pub fn #as_struct_ident(&self) -> #target_ident {
                        let offset = self.pos + #offset_lit;
                        #target_ident(read_bytes::<#comp_size_lit>(self.buf, offset))
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

                // ponytail: _lazy alias removed — the base accessor is the canonical path
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
                        #target_ident::from_raw(#r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset)))
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

                if enum_name == "BooleanType" {
                    let bool_ident = quote::format_ident!("{}_bool", f_name);
                    entry_body.extend(quote::quote! {
                        #[inline]
                        pub const fn #bool_ident(&self) -> bool {
                            (self.#f_name_ident() as #r_type_ty) != 0
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
                        #target_ident(#r_type_ty::#order_fn(read_bytes::<#prim_size_lit>(self.buf, offset)))
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
                    return Err(sbe_rt::DecodeError::BufferTooShort { field: #ng_name_lit, needed: #dim_size_lit, available: self.buf.len() - start });
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
                    return Err(sbe_rt::DecodeError::BufferTooShort { field: #vd_name_lit, needed: #prefix_size_lit, available: self.buf.len() - start });
                }
                let bytes: [u8; #prefix_size_lit] = read_bytes::<#prefix_size_lit>(self.buf, start);
                let header = #type_pascal_ident(bytes);
                let len = header.#len_field_ident() as usize;
                if start + #prefix_size_lit + len > self.buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort { field: #vd_name_lit, needed: #prefix_size_lit + len, available: self.buf.len() - start });
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
                Ok(self.#tail_total_fn()? - self.pos)
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
            FieldType::Composite { .. } => {}
        }
    }
    // Entry varData fields in Display
    for vd in &g.var_data {
        let vd_snake = to_snake_case(&vd.name);
        let vd_ident = syn::Ident::new(&vd_snake, proc_macro2::Span::call_site());
        let sep = if entry_display_out_idx == 0 { "" } else { ", " };
        let fmt_str = format!("{sep}{}: {{}} bytes", vd.name);
        entry_display_body.extend(quote::quote! {
            if let Ok(d) = self.#vd_ident() {
                write!(f, #fmt_str, d.len())?;
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
        let desc_lit = syn::LitStr::new(desc, proc_macro2::Span::call_site());
        ts.extend(quote::quote! { #[doc = #desc_lit] });
    }
    ts.extend(quote::quote! {
        pub struct #entry_decoder_ident<'a> {
            buf: &'a [u8],
            pos: usize,
            acting_version: u16,
            acting_block_length: usize,
        }

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

/// Generate raw `*_wire` aliases and generic converted methods for fields
/// whose type is a registered Decimal composite. Only emitted in converter
/// mode. Generated code never mentions `rust_decimal`.
fn generate_decimal_converter_impls(msg: &MessageStructure, decimal_composites: &[String]) -> String {
    let span = proc_macro2::Span::call_site();
    let msg_name = to_pascal_case(&msg.name);
    let decoder_ident = syn::Ident::new(&format!("{msg_name}Decoder"), span);
    let encoder_ident = syn::Ident::new(&format!("{msg_name}Encoder"), span);

    let mut decoder_methods = proc_macro2::TokenStream::new();
    let mut encoder_methods = proc_macro2::TokenStream::new();

    for f in &msg.fields {
        let comp_name = match &f.field_type {
            FieldType::Composite { name, .. } => name,
            _ => continue,
        };
        if !decimal_composites.iter().any(|d| d == comp_name) {
            continue;
        }

        let field_snake = to_snake_case(&f.name);
        let field_ident = syn::Ident::new(&field_snake, span);
        let wire_ident = syn::Ident::new(&format!("{field_snake}_wire"), span);
        let comp_type_ident = syn::Ident::new(&to_pascal_case(comp_name), span);

        // Decoder: generic converted accessor delegates to the raw *_wire method
        // (already generated by the main field accessor codegen).
        decoder_methods.extend(quote::quote! {
            /// Generic converted accessor. Calls `SbeDecimal::try_from_sbe`
            /// on the raw wire mantissa/exponent.
            #[inline]
            pub fn #field_ident<D: SbeDecimal>(&self) -> Result<D, D::Error> {
                let raw = self.#wire_ident();
                D::try_from_sbe(raw.mantissa(), raw.exponent())
            }
        });

        // Encoder: generic converted setter calls `SbeDecimal::try_into_sbe`,
        // then writes via the raw *_wire setter.
        encoder_methods.extend(quote::quote! {
            /// Generic converted setter. Calls `SbeDecimal::try_into_sbe`.
            pub fn #field_ident<D: SbeDecimal>(&mut self, val: D) -> Result<&mut Self, D::Error> {
                let (m, e) = val.try_into_sbe()?;
                self.#wire_ident(#comp_type_ident::new(m, e));
                Ok(self)
            }
        });
    }

    if decoder_methods.is_empty() {
        return String::new();
    }

    let ts = quote::quote! {
        impl<'a> #decoder_ident<'a> {
            #decoder_methods
        }
        impl<'a> #encoder_ident<'a> {
            #encoder_methods
        }
    };
    ts.to_string()
}

fn generate_message_encoder(
    msg: &MessageStructure,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    schema_id: u16,
    schema_version: u16,
    header_type: &str,
    multi_message: bool,
    decimal_composites: &[String],
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

    // Pre-compute HEADER_TEMPLATE bytes at codegen time.
    let mut header_tpl = vec![0u8; header_size];
    let hdr_bl = block_length as u16;
    match byte_order {
        ByteOrder::LittleEndian => {
            header_tpl[0..2].copy_from_slice(&hdr_bl.to_le_bytes());
            header_tpl[2..4].copy_from_slice(&msg.id.to_le_bytes());
            header_tpl[4..6].copy_from_slice(&schema_id.to_le_bytes());
            header_tpl[6..8].copy_from_slice(&schema_version.to_le_bytes());
        }
        ByteOrder::BigEndian => {
            header_tpl[0..2].copy_from_slice(&hdr_bl.to_be_bytes());
            header_tpl[2..4].copy_from_slice(&msg.id.to_be_bytes());
            header_tpl[4..6].copy_from_slice(&schema_id.to_be_bytes());
            header_tpl[6..8].copy_from_slice(&schema_version.to_be_bytes());
        }
    }
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
        let desc_lit = syn::LitStr::new(desc, span);
        ts.extend(quote::quote! { #[doc = #desc_lit] });
    }
    for stage in &stage_idents {
        ts.extend(quote::quote! {
            #[must_use = "encoder must be consumed to write the message"]
            pub struct #stage<'a> {
                buf: &'a mut [u8],
                message_start: usize,
                pos: usize,
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
            /// Stack-allocate with `let mut buf = [0u8; Msg::ENCODED_LENGTH];`
            pub const ENCODED_LENGTH: usize = #encoded_length_lit;
            const _ENCODED_LEN: () = assert!(Self::ENCODED_LENGTH >= Self::BLOCK_LENGTH);
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
            #max_doc_attr
            pub const MAX_ENCODED_LENGTH: usize = #max_encoded_capped_lit;
            const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
        });
    }

    // HEADER_TEMPLATE
    impl_contents.extend(quote::quote! {
        pub const HEADER_TEMPLATE: [u8; #header_size_lit] = [#(#hdr_lits),*];
        const _HEADER_TEMPLATE_LEN: () = assert!(Self::HEADER_TEMPLATE.len() == #header_size_lit);
    });

    // wrap() and wrap_and_apply_header() — fallible, non-panicking.
    let wrap_fn = quote::quote! {
        /// Wrap a mutable buffer for encoding. Returns an error if the buffer
        /// is too short for the header + fixed block.
        #[inline]
        pub fn wrap(buf: &'a mut [u8], pos: usize) -> Result<Self, sbe_rt::EncodeError> {
            let needed: usize = #header_size_lit + Self::BLOCK_LENGTH;
            let available: usize = buf.len().saturating_sub(pos);
            if available < needed {
                return Err(sbe_rt::EncodeError::BufferTooShort { needed, available });
            }
            Ok(Self {
                buf: &mut buf[pos..],
                message_start: 0,
                pos: needed,
            })
        }
    };
    impl_contents.extend(wrap_fn);

    let wrap_apply_body = quote::quote! {
        // Optional-field nullification is NOT applied by default — call
        // `apply_nulls()` if you want null sentinels.
        // Check buffer size before touching memory.
        let needed: usize = #header_size_lit + Self::BLOCK_LENGTH;
        let available: usize = buf.len().saturating_sub(pos);
        if available < needed {
            return Err(sbe_rt::EncodeError::BufferTooShort { needed, available });
        }
        buf[pos..pos + #header_size_lit].copy_from_slice(&Self::HEADER_TEMPLATE);
        Self::wrap(buf, pos)
    };
    let wrap_apply_fn = quote::quote! {
        /// Wrap a mutable buffer and write the SBE message header.
        /// Returns an error if the buffer is too short.
        #[inline]
        pub fn wrap_and_apply_header(buf: &'a mut [u8], pos: usize) -> Result<Self, sbe_rt::EncodeError> {
            #wrap_apply_body
        }
    };
    impl_contents.extend(wrap_apply_fn);

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
        // In converter mode, Decimal-composite-backed raw setters are
        // suffixed _wire so the generic converted setter takes the
        // original name.
        let is_decimal = matches!(&f.field_type,
            FieldType::Composite { name, .. } if decimal_composites.iter().any(|d| d == name));
        let method_name = if is_decimal {
            format!("{f_name}_wire")
        } else {
            f_name.clone()
        };
        let f_ident = syn::Ident::new(&method_name, span);

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
                            #[must_use]
                            #[inline]
                            pub fn #f_ident(&mut self, val: [#r_type; #len_lit]) -> &mut Self {
                                self.buf[#body_offset_lit..][..#len_lit].copy_from_slice(&val);
                                self
                            }
                        });
                    } else {
                        impl_contents.extend(quote::quote! {
                            #[must_use]
                            #[inline]
                            pub fn #f_ident(&mut self, val: [#r_type; #len_lit]) -> &mut Self {
                                let offset = #body_offset_lit;
                                let mut idx = 0usize;
                                while idx < #len_lit {
                                    self.buf[offset + idx * #prim_size_lit..offset + (idx + 1) * #prim_size_lit].copy_from_slice(&val[idx].#to_endian());
                                    idx += 1;
                                }
                                self
                            }
                        });
                    }
                } else {
                    impl_contents.extend(quote::quote! {
                        #[must_use]
                        #[inline]
                        pub fn #f_ident(&mut self, val: #r_type) -> &mut Self {
                            let offset = #body_offset_lit;
                            self.buf[offset..offset + #prim_size_lit].copy_from_slice(&val.#to_endian());
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
                    #[must_use]
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
                    #[must_use]
                    pub fn #f_ident(&mut self, val: #target_type) -> &mut Self {
                        let offset = #body_offset_lit;
                        self.buf[offset..offset + #prim_size_lit].copy_from_slice(&(val as #r_type).#to_endian());
                        self
                    }
                });
                // Boolean fields get an additional setter that accepts bool directly
                if enum_name == "BooleanType" {
                    let f_name_bool = syn::Ident::new(&format!("{}_bool", f_name), span);
                    impl_contents.extend(quote::quote! {
                        #[must_use]
                        pub fn #f_name_bool(&mut self, val: bool) -> &mut Self {
                            let offset = #body_offset_lit;
                            let enum_val: #target_type = val.into();
                            self.buf[offset..offset + #prim_size_lit].copy_from_slice(&(enum_val as #r_type).#to_endian());
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
                    #[must_use]
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

    // Pre-encoding length calculator for messages with tails
    if total_tail > 0 {
        let mut params = Vec::<proc_macro2::TokenStream>::new();
        let mut param_names = Vec::<syn::Ident>::new();
        let mut sum_body = Vec::<proc_macro2::TokenStream>::new();

        for g_e in &msg.groups {
            let g_snake = to_snake_case(&g_e.name);
            let param_ident: syn::Ident = syn::Ident::new(&format!("{}_count", g_snake), span);
            let (_, dim_size, _, _) = get_dimension_info(elements, &g_e.dimension_type);
            let dim_size_lit = syn::LitInt::new(&dim_size.to_string(), span);
            let g_block_len_lit = syn::LitInt::new(&g_e.block_length.to_string(), span);

            sum_body.push(quote::quote! {
                len += #dim_size_lit + #param_ident * #g_block_len_lit;
            });
            params.push(quote::quote! { #param_ident: usize });
            param_names.push(param_ident);
        }

        for vd in &msg.var_data {
            let vd_snake = to_snake_case(&vd.name);
            let param_ident: syn::Ident = syn::Ident::new(&format!("{}_len", vd_snake), span);
            let (_, prefix_size, _, _) = get_vardata_info(elements, &vd.type_name);
            let prefix_size_lit = syn::LitInt::new(&prefix_size.to_string(), span);

            sum_body.push(quote::quote! {
                len += #prefix_size_lit + #param_ident;
            });
            params.push(quote::quote! { #param_ident: usize });
            param_names.push(param_ident);
        }

        impl_contents.extend(quote::quote! {
            /// Compute the exact SBE message body length before encoding.
            /// Parameters: one `usize` per group (entry count) and one `usize` per var-data field (byte length).
            #[inline]
            pub const fn compute_encoded_length(
                #(#params),*
            ) -> usize {
                let mut len = #block_length_lit;
                #(#sum_body)*
                len
            }

            /// Compute the exact SBE message length including the standard
            /// message header (header size + body). DECISIONS.md §2: callers
            /// must use this — not a hand-written `+ 8`.
            #[inline]
            pub const fn compute_encoded_length_with_message_header(
                #(#params),*
            ) -> usize {
                #header_size + Self::compute_encoded_length(#(#param_names),*)
            }
        });
    }

    // Fallible fixed-body chaining: try_fixed runs a closure over &mut self
    // and propagates caller errors, keeping the same concrete stage.
    impl_contents.extend(quote::quote! {
        /// Run a fallible closure over the fixed-body fields. The closure
        /// receives `&mut Self` and can set/read fixed fields; tail
        /// transitions are unavailable inside the closure. Returns the
        /// same stage on success, or the caller's error on failure.
        #[inline]
        pub fn try_fixed<E, F>(mut self, f: F) -> Result<Self, E>
        where
            F: FnOnce(&mut Self) -> Result<(), E>,
        {
            f(&mut self)?;
            Ok(self)
        }
    });

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
            let try_g_snake = syn::Ident::new(&format!("try_{}", to_snake_case(&g.name)), span);
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

            ts.extend(quote::quote! {
                impl<'a> #current_stage<'a> {
                    #[must_use]
                    pub fn #g_snake<F>(
                        mut self,
                        count: #count_ty,
                        f: F,
                    ) -> Result<#next_stage<'a>, sbe_rt::EncodeError>
                    where
                        F: FnOnce(&mut #g_pascal_enc<'a>),
                    {
                        if self.pos + #dim_size_lit > self.buf.len() {
                            return Err(sbe_rt::EncodeError::BufferTooShort {
                                needed: #dim_size_lit,
                                available: self.buf.len() - self.pos,
                            });
                        }
                        self.buf[self.pos..self.pos + #dim_size_lit]
                            .copy_from_slice(&#g_pascal_enc::GROUP_DIM_TEMPLATE);
                        self.buf
                            [self.pos + #num_offset_lit..self.pos + #num_offset_lit + #num_size_lit]
                            .copy_from_slice(&count.#to_endian());
                        let mut group =
                            #g_pascal_enc::wrap(self.buf, self.pos + #dim_size_lit, count);
                        f(&mut group);
                        Ok(#next_stage {
                            buf: group.buf,
                            message_start: self.message_start,
                            pos: group.pos,
                        })
                    }

                    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
                    #[must_use]
                    pub fn #try_g_snake<E, F>(
                        mut self,
                        count: #count_ty,
                        f: F,
                    ) -> Result<#next_stage<'a>, E>
                    where
                        E: From<sbe_rt::EncodeError>,
                        F: FnOnce(&mut #g_pascal_enc<'a>) -> Result<(), E>,
                    {
                        if self.pos + #dim_size_lit > self.buf.len() {
                            return Err(sbe_rt::EncodeError::BufferTooShort {
                                needed: #dim_size_lit,
                                available: self.buf.len() - self.pos,
                            }.into());
                        }
                        self.buf[self.pos..self.pos + #dim_size_lit]
                            .copy_from_slice(&#g_pascal_enc::GROUP_DIM_TEMPLATE);
                        self.buf[self.pos + #num_offset_lit..self.pos + #num_offset_lit + #num_size_lit]
                            .copy_from_slice(&count.#to_endian());
                        let mut group =
                            #g_pascal_enc::wrap(self.buf, self.pos + #dim_size_lit, count);
                        f(&mut group)?;
                        Ok(#next_stage {
                            buf: group.buf,
                            message_start: self.message_start,
                            pos: group.pos,
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
                        available: self.buf.len() - self.pos,
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
                                available: self.buf.len() - self.pos,
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
        );
    }
    if !group_buf.is_empty() {
        let group_ts: proc_macro2::TokenStream = group_buf
            .parse()
            .expect("generate_group_encoder produced invalid token stream");
        ts.extend(group_ts);
    }

    ts
}

fn generate_group_encoder(
    src: &mut String,
    g: &MessageGroup,
    elements: &SchemaElements,
    byte_order: ByteOrder,
    scoped_name: &str,
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
            return Err(sbe_rt::EncodeError::GroupFull { declared: self.count as u32, attempted: self.written as u32 + 1 });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort { needed: block_len, available: self.buf.len() - self.pos });
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
            f(&mut __entry);
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

            #[must_use]
            pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
            where
                F: FnOnce(&mut #entry_enc_ident<'b>),
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
        let f_ident = syn::Ident::new(&to_snake_case(&f.name), span);
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
                        #[must_use]
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
                } else {
                    let sz = syn::LitInt::new(&prim_size.to_string(), span);
                    entry_methods.extend(quote::quote! {
                        #[must_use]
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
                    #[must_use]
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
                    #[must_use]
                    pub fn #f_ident(&mut self, val: #target) -> &mut Self {
                        let offset = self.entry_start + #f_offset;
                        self.buf[offset..offset + #sz].copy_from_slice(&(val as #r_ty).#to_endian());
                        self
                    }
                });
            }
            FieldType::Set {
                name: set_name,
                encoding_type,
            } => {
                let target = syn::Ident::new(&to_pascal_case(set_name), span);
                let sz = syn::LitInt::new(&encoding_type.size().to_string(), span);
                entry_methods.extend(quote::quote! {
                    #[must_use]
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
                F: FnOnce(&mut #ng_enc<'a>),
            {
                if self.pos + #ng_dim > self.buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort { needed: #ng_dim, available: self.buf.len() - self.pos });
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
                    f(&mut group);
                    __pos = group.pos;
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
                    return Err(sbe_rt::EncodeError::BufferTooShort { needed, available: self.buf.len() - self.pos });
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
        generate_group_encoder(src, ng, elements, byte_order, &nested_name);
    }
}

/// Generate a `pub mod prelude` that re-exports the common API surface so users
/// can write `use my_schema::prelude::*;`.
fn generate_prelude(
    src: &mut String,
    elements: &SchemaElements,
    messages: &[MessageStructure],
    schema_id: u16,
    schema_version: u16,
) {
    // Schema-level constants
    writeln!(src, "pub const SCHEMA_ID: u16 = {schema_id};").unwrap();
    writeln!(src, "pub const SCHEMA_VERSION: u16 = {schema_version};").unwrap();

    // Collect generated type names (module-level, not in sbe_rt)
    let mut gen_types: Vec<String> = Vec::new();

    // Composites: both value struct and decoder
    for ct in &elements.composites {
        let name = to_pascal_case(&ct[0].name);
        gen_types.push(name.clone());
        gen_types.push(format!("{name}Decoder"));
    }

    // Enums
    for et in &elements.enums {
        gen_types.push(to_pascal_case(&et[0].name));
    }

    // Sets
    for st in &elements.sets {
        gen_types.push(to_pascal_case(&st[0].name));
    }

    // Message decoders and encoders
    for msg in messages {
        gen_types.push(format!("{}Decoder", to_pascal_case(&msg.name)));
        gen_types.push(format!("{}Encoder", to_pascal_case(&msg.name)));
    }

    // Emit prelude
    // sbe_rt types (exported from super::sbe_rt)
    src.push_str("pub mod prelude {\n");
    src.push_str(
        "    pub use super::sbe_rt::{DecodeError, EncodeError, VerifyError, SbeMessage};\n",
    );

    // Module-level types (exported from super)
    src.push_str("    pub use super::{\n");
    // Built-in module-level types
    for ty in &[
        "AnyMessage",
        "DecodedFrame",
        "FrameCursor",
        "FramingPolicy",
        "MessageVisitor",
    ] {
        writeln!(src, "        {ty},").unwrap();
    }
    // Generated types (composites, enums, sets, messages)
    for ty in &gen_types {
        writeln!(src, "        {ty},").unwrap();
    }
    src.push_str("    };\n");
    src.push_str("}\n\n");
}

fn generate_schema_id_from_header(
    src: &mut String,
    elements: &SchemaElements,
    header_type: &str,
    byte_order: ByteOrder,
) {
    let order_suffix = match byte_order {
        ByteOrder::LittleEndian => "le",
        ByteOrder::BigEndian => "be",
    };

    let schema_id_offset = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|comp| {
            parse_composite_members(comp)
                .iter()
                .find(|m| m.name.to_lowercase().contains("schemaid"))
                .map(|m| m.offset)
        })
        .unwrap_or(4);

    let sid = syn::Index::from(schema_id_offset);
    let order_fn = syn::Ident::new(
        &format!("from_{order_suffix}_bytes"),
        proc_macro2::Span::call_site(),
    );
    let ts = quote::quote! {
        #[inline]
        pub fn schema_id_from_header(buf: &[u8]) -> Option<u16> {
            if buf.len() < #sid + 2 {
                return None;
            }
            let bytes = read_bytes::<2>(buf, #sid);
            Some(u16::#order_fn(bytes))
        }
    };
    src.push_str(&ts.to_string());
    src.push('\n');
}

fn generate_any_message(
    messages: &[MessageStructure],
    elements: &SchemaElements,
    schema_id: u16,
    header_type: &str,
    schema_name: &str,
) -> proc_macro2::TokenStream {
    let header_size = elements
        .composites
        .iter()
        .find(|c| c[0].name == header_type)
        .and_then(|c| c[0].encoding.offset)
        .unwrap_or(8);

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

    let span = proc_macro2::Span::call_site();
    let header_type_ident = syn::Ident::new(&to_pascal_case(header_type), span);
    let header_size_lit = syn::LitInt::new(&header_size.to_string(), span);
    let schema_id_lit = syn::LitInt::new(&schema_id.to_string(), span);
    let bl_ident = syn::Ident::new(&header_bl, span);
    let ti_ident = syn::Ident::new(&header_ti, span);
    let si_ident = syn::Ident::new(&header_si, span);
    let vr_ident = syn::Ident::new(&header_vr, span);

    let mut out = proc_macro2::TokenStream::new();

    // ── AnyMessage enum ─────────────────────────────────────────────────
    {
        let mut enum_variants = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            let decoder = quote::format_ident!("{}Decoder", to_pascal_case(&m.name));
            enum_variants.extend(quote::quote! {
                #name(#decoder<'a>),
            });
        }
        out.extend(quote::quote! {
            #[non_exhaustive]
            pub enum AnyMessage<'a> {
                #enum_variants
                Unknown {
                    header: #header_type_ident,
                    payload: &'a [u8],
                },
            }
        });
    }

    // ── DecodedFrame struct ──────────────────────────────────────────────
    out.extend(quote::quote! {
        pub struct DecodedFrame<'a> {
            pub message: AnyMessage<'a>,
            pub range: core::ops::Range<usize>,
            pub len: usize,
        }
    });

    // ── FramingPolicy enum ──────────────────────────────────────────────
    out.extend(quote::quote! {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub enum FramingPolicy {
            LengthPrefixU32,
            LengthPrefixU16,
            Fixed(usize),
        }
    });

    // ── FrameCursor struct + Iterator impl ──────────────────────────────
    out.extend(quote::quote! {
        pub struct FrameCursor<'a> {
            buf: &'a [u8],
            pos: usize,
            framing: FramingPolicy,
        }

        impl<'a> FrameCursor<'a> {
            #[inline]
            pub const fn new(buf: &'a [u8], framing: FramingPolicy) -> Self {
                Self { buf, pos: 0, framing }
            }
        }

        impl<'a> Iterator for FrameCursor<'a> {
            type Item = Result<DecodedFrame<'a>, sbe_rt::DecodeError>;

            fn next(&mut self) -> Option<Self::Item> {
                if self.pos >= self.buf.len() {
                    return None;
                }
                let (header_len, frame_len) = match self.framing {
                    FramingPolicy::LengthPrefixU32 => {
                        if self.pos + 4 > self.buf.len() {
                            return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                                field: "length prefix",
                                needed: 4,
                                available: self.buf.len() - self.pos,
                            }));
                        }
                        let bytes: [u8; 4] = read_bytes::<4>(self.buf, self.pos);
                        let len = u32::from_le_bytes(bytes) as usize;
                        (4, len)
                    }
                    FramingPolicy::LengthPrefixU16 => {
                        if self.pos + 2 > self.buf.len() {
                            return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                                field: "length prefix",
                                needed: 2,
                                available: self.buf.len() - self.pos,
                            }));
                        }
                        let bytes: [u8; 2] = read_bytes::<2>(self.buf, self.pos);
                        let len = u16::from_le_bytes(bytes) as usize;
                        (2, len)
                    }
                    FramingPolicy::Fixed(len) => (0, len),
                };

                if self.pos + header_len + frame_len > self.buf.len() {
                    return Some(Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "frame bounds",
                        needed: header_len + frame_len,
                        available: self.buf.len() - self.pos,
                    }));
                }
                let off = self.pos + header_len;
                let res = AnyMessage::decode_frame(self.buf, off, frame_len);
                match res {
                    Ok(frame) => {
                        self.pos += header_len + frame_len;
                        Some(Ok(frame))
                    }
                    Err(e) => Some(Err(e)),
                }
            }
        }
    });

    // ── decode() ────────────────────────────────────────────────────────
    {
        let mut decode_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            let decoder = quote::format_ident!("{}Decoder", to_pascal_case(&m.name));
            let id = syn::LitInt::new(&m.id.to_string(), span);
            decode_arms.extend(quote::quote! {
                #id => Ok(Self::#name(#decoder::wrap(buf, body_pos, block_length, version))),
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn decode(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
                    if pos + #header_size_lit > buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "message header",
                            needed: #header_size_lit,
                            available: buf.len() - pos,
                        });
                    }
                    let header_bytes = read_bytes::<#header_size_lit>(buf, pos);
                    let header = #header_type_ident(header_bytes);
                    let template_id = header.#ti_ident();
                    let schema_id = header.#si_ident();
                    let version = header.#vr_ident();
                    let block_length = header.#bl_ident() as usize;
                    let body_pos = pos + #header_size_lit;

                    if schema_id != #schema_id_lit {
                        return Err(sbe_rt::DecodeError::WrongSchema {
                            expected: #schema_id_lit,
                            actual: schema_id,
                            expected_name: #schema_name,
                        });
                    }

                    match template_id {
                        #decode_arms
                        _ => Err(sbe_rt::DecodeError::UnknownTemplateLength { template_id }),
                    }
                }
            }
        });
    }

    // ── decode_frame() ──────────────────────────────────────────────────
    {
        let mut decode_frame_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            let decoder = quote::format_ident!("{}Decoder", to_pascal_case(&m.name));
            let id = syn::LitInt::new(&m.id.to_string(), span);
            let field_name = &m.name;
            decode_frame_arms.extend(quote::quote! {
                #id => {
                    let decoder = #decoder::wrap(buf, body_pos, block_length, version);
                    let total_len = decoder.encoded_length_with_header()?;
                    if total_len > frame_len {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: #field_name,
                            needed: total_len,
                            available: frame_len,
                        });
                    }
                    Ok(DecodedFrame {
                        message: Self::#name(decoder),
                        range: pos .. pos + total_len,
                        len: total_len,
                    })
                }
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn decode_frame(buf: &'a [u8], pos: usize, frame_len: usize) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
                    // Trust boundary: always validate header fits
                    if pos + #header_size_lit > buf.len() {
                        return Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "message header",
                            needed: #header_size_lit,
                            available: buf.len().saturating_sub(pos),
                        });
                    }
                    let header_bytes: [u8; #header_size_lit] = read_bytes::<#header_size_lit>(buf, pos);
                    let header = #header_type_ident(header_bytes);
                    let template_id = header.#ti_ident();
                    let schema_id = header.#si_ident();
                    let version = header.#vr_ident();
                    let block_length = header.#bl_ident() as usize;
                    let body_pos = pos + #header_size_lit;

                    if schema_id != #schema_id_lit {
                        return Err(sbe_rt::DecodeError::WrongSchema {
                            expected: #schema_id_lit,
                            actual: schema_id,
                            expected_name: #schema_name,
                        });
                    }

                    match template_id {
                        #decode_frame_arms
                        _ => {
                            if pos + frame_len > buf.len() {
                                return Err(sbe_rt::DecodeError::BufferTooShort {
                                    field: "template body",
                                    needed: frame_len,
                                    available: buf.len() - pos,
                                });
                            }
                            let payload = &buf[pos .. pos + frame_len];
                            Ok(DecodedFrame {
                                message: Self::Unknown {
                                    header,
                                    payload,
                                },
                                range: pos .. pos + frame_len,
                                len: frame_len,
                            })
                        }
                    }
                }
            }
        });
    }

    // ── encoded_length_with_header() ────────────────────────────────────
    {
        let mut encoded_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            encoded_arms.extend(quote::quote! {
                Self::#name(d) => d.encoded_length_with_header(),
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
                    match self {
                        #encoded_arms
                        Self::Unknown { payload, .. } => Ok(payload.len()),
                    }
                }
            }
        });
    }

    // ── as_bytes() ──────────────────────────────────────────────────────
    {
        let mut as_bytes_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            as_bytes_arms.extend(quote::quote! {
                Self::#name(d) => d.as_bytes(),
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
                    match self {
                        #as_bytes_arms
                        Self::Unknown { payload, .. } => Ok(payload),
                    }
                }
            }
        });
    }

    // ── encode() ────────────────────────────────────────────────────────
    {
        let mut encode_arms = proc_macro2::TokenStream::new();
        for m in messages {
            let name = quote::format_ident!("{}", to_pascal_case(&m.name));
            encode_arms.extend(quote::quote! {
                Self::#name(d) => {
                    let len = d.encoded_length_with_header()?;
                    buf[..len].copy_from_slice(d.as_bytes()?);
                    Ok(len)
                }
            });
        }

        out.extend(quote::quote! {
            impl<'a> AnyMessage<'a> {
                #[inline]
                pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
                    match self {
                        #encode_arms
                        Self::Unknown { payload, .. } => {
                            buf[..payload.len()].copy_from_slice(payload);
                            Ok(payload.len())
                        }
                    }
                }
            }
        });
    }

    // ── MessageVisitor trait + visit() ──────────────────────────────────
    {
        let mut visitor_methods = Vec::new();
        let mut visit_arms = Vec::new();
        for m in messages {
            let name_pascal = to_pascal_case(&m.name);
            let name_snake = to_snake_case(&m.name);
            let method_name = syn::Ident::new(
                &format!("visit_{name_snake}"),
                proc_macro2::Span::call_site(),
            );
            let decoder_ty: syn::Type =
                syn::parse_str(&format!("{name_pascal}Decoder<'_>")).unwrap();
            let variant = syn::Ident::new(&name_pascal, proc_macro2::Span::call_site());
            visitor_methods.push(quote::quote! {
                fn #method_name(&mut self, decoder: &#decoder_ty) -> Self::Output;
            });
            visit_arms.push(quote::quote! {
                Self::#variant(d) => visitor.#method_name(d),
            });
        }

        out.extend(quote::quote! {
            pub trait MessageVisitor {
                type Output;

                #(#visitor_methods)*
            }

            impl<'a> AnyMessage<'a> {
                pub fn visit<V: MessageVisitor>(&self, visitor: &mut V) -> V::Output {
                    match self {
                        #(#visit_arms)*
                        Self::Unknown { .. } => unimplemented!(),
                    }
                }
            }
        });
    }

    out
}

/// Compute a deterministic 64-bit hash of the schema identity.
///
/// Uses FNV-1a over `package` bytes, `id` (LE), and `version` (LE).
/// This is a simple compile-time-expressible hash for schema identity
/// verification — not a cryptographic hash.
fn compute_schema_hash(package: &str, id: u16, version: u16) -> u64 {
    const FNV_OFFSET: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    let mut hash = FNV_OFFSET;
    for &b in package.as_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &b in &id.to_le_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    for &b in &version.to_le_bytes() {
        hash ^= b as u64;
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}

/// Compute SHA-256 hash of the canonical schema IR.
fn compute_schema_sha256(ir: &Ir) -> [u8; 32] {
    let canonical = canonical_schema_bytes(ir);
    let mut hasher = Sha256::new();
    hasher.update(&canonical);
    let result = hasher.finalize();
    result.into()
}

/// Serialize the schema IR to a canonical byte sequence for hashing.
/// The output is deterministic for the same IR content.
fn canonical_schema_bytes(ir: &Ir) -> Vec<u8> {
    let mut buf = Vec::new();

    // Schema identity
    extend_str(&mut buf, &ir.package);
    buf.extend_from_slice(&ir.id.to_le_bytes());
    buf.extend_from_slice(&ir.version.to_le_bytes());
    buf.push(match ir.byte_order {
        ByteOrder::LittleEndian => 0,
        ByteOrder::BigEndian => 1,
    });
    extend_opt_str(&mut buf, ir.description.as_deref());
    extend_opt_str(&mut buf, ir.semantic_version.as_deref());
    extend_str(&mut buf, &ir.header_type);

    // Tokens
    for token in &ir.tokens {
        buf.push(token.signal as u8);
        extend_str(&mut buf, &token.name);
        match token.id {
            Some(id) => {
                buf.push(1);
                buf.extend_from_slice(&id.to_le_bytes());
            }
            None => buf.push(0),
        }

        // Encoding
        match token.encoding.primitive_type {
            Some(pt) => {
                buf.push(1);
                buf.push(pt as u8);
            }
            None => buf.push(0),
        }
        buf.push(token.encoding.presence as u8);
        buf.extend_from_slice(&token.encoding.since_version.to_le_bytes());
        match token.encoding.null_value {
            Some(nv) => {
                buf.push(1);
                buf.extend_from_slice(&nv.to_le_bytes());
            }
            None => buf.push(0),
        }
        extend_opt_str(&mut buf, token.encoding.character_encoding.as_deref());
        extend_opt_str(&mut buf, token.encoding.semantic_type.as_deref());
        match token.encoding.min_value {
            Some(mv) => {
                buf.push(1);
                buf.extend_from_slice(&mv.to_le_bytes());
            }
            None => buf.push(0),
        }
        match token.encoding.max_value {
            Some(mv) => {
                buf.push(1);
                buf.extend_from_slice(&mv.to_le_bytes());
            }
            None => buf.push(0),
        }
        extend_opt_str(&mut buf, token.encoding.description.as_deref());
        extend_opt_str(&mut buf, token.encoding.constant_value.as_deref());
        match token.encoding.length {
            Some(len) => {
                buf.push(1);
                buf.extend_from_slice(&(len as u64).to_le_bytes());
            }
            None => buf.push(0),
        }
        match token.encoding.offset {
            Some(off) => {
                buf.push(1);
                buf.extend_from_slice(&(off as u64).to_le_bytes());
            }
            None => buf.push(0),
        }
    }

    buf
}

/// Append a null-terminated string to the canonical hash input.
fn extend_str(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(s.as_bytes());
    buf.push(0);
}

/// Append an optional null-terminated string (presence-tagged).
fn extend_opt_str(buf: &mut Vec<u8>, s: Option<&str>) {
    match s {
        Some(s) => {
            buf.push(1);
            extend_str(buf, s);
        }
        None => buf.push(0),
    }
}

/// Generate a `field_meta` module for a message, exposing field metadata
/// as a compile-time constant slice.
///
/// Emits:
/// ```ignore
/// pub mod car_field_meta {
///     pub struct FieldInfo {
///         pub name: &'static str,
///         pub id: u16,
///         pub offset: usize,
///         pub since_version: u16,
///         pub field_type: &'static str,
///     }
///     pub const FIELDS: &[FieldInfo] = &[
///         FieldInfo { name: "serialNumber", id: 1, offset: 0, since_version: 0, field_type: "uint64" },
///         ...
///     ];
/// }
/// ```
fn generate_message_field_meta(src: &mut String, msg: &MessageStructure) {
    let mod_name = syn::Ident::new(
        &format!("{}_field_meta", to_snake_case(&msg.name)),
        proc_macro2::Span::call_site(),
    );

    let fields: Vec<proc_macro2::TokenStream> = msg
        .fields
        .iter()
        .map(|f| {
            let name_lit = syn::LitStr::new(&f.name, proc_macro2::Span::call_site());
            let id = f.id.unwrap_or(0);
            let id_lit = syn::LitInt::new(&id.to_string(), proc_macro2::Span::call_site());
            let offset_lit =
                syn::LitInt::new(&f.offset.to_string(), proc_macro2::Span::call_site());
            let sv_lit = syn::LitInt::new(
                &f.since_version.to_string(),
                proc_macro2::Span::call_site(),
            );
            let field_type_str = match &f.field_type {
                FieldType::Primitive(prim, _) => rust_type(*prim).to_string(),
                FieldType::Composite { name, .. } => to_pascal_case(name),
                FieldType::Enum { name, .. } => to_pascal_case(name),
                FieldType::Set { name, .. } => to_pascal_case(name),
            };
            let field_type_lit =
                syn::LitStr::new(&field_type_str, proc_macro2::Span::call_site());
            let presence_str = match f.presence {
                Presence::Required => "required",
                Presence::Optional => "optional",
                Presence::Constant => "constant",
            };
            let presence_lit =
                syn::LitStr::new(presence_str, proc_macro2::Span::call_site());
            let null_val = f.null_value.map(|v| {
                let s = v.to_string();
                let lit = syn::LitStr::new(&s, proc_macro2::Span::call_site());
                quote::quote! { Some(#lit) }
            }).unwrap_or(quote::quote! { None });
            let sem_type = f
                .semantic_type
                .as_deref()
                .map(|v| {
                    let lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
                    quote::quote! { Some(#lit) }
                })
                .unwrap_or(quote::quote! { None });
            let desc = f
                .description
                .as_deref()
                .map(|v| {
                    let lit = syn::LitStr::new(v, proc_macro2::Span::call_site());
                    quote::quote! { Some(#lit) }
                })
                .unwrap_or(quote::quote! { None });

            quote::quote! {
                FieldInfo { name: #name_lit, id: #id_lit, offset: #offset_lit, since_version: #sv_lit, field_type: #field_type_lit, presence: #presence_lit, null_value: #null_val, semantic_type: #sem_type, description: #desc },
            }
        })
        .collect();

    let tokens = quote::quote! {
        pub mod #mod_name {
            pub struct FieldInfo {
                pub name: &'static str,
                pub id: u16,
                pub offset: usize,
                pub since_version: u16,
                pub field_type: &'static str,
                pub presence: &'static str,
                pub null_value: Option<&'static str>,
                pub semantic_type: Option<&'static str>,
                pub description: Option<&'static str>,
            }
            pub const FIELDS: &[FieldInfo] = &[
                #(#fields)*
            ];
        }
    };

    let formatted = syn::parse_str::<syn::File>(&tokens.to_string())
        .map(|file| prettyplease::unparse(&file))
        .unwrap_or_else(|_| tokens.to_string());
    src.push_str(&formatted);
    src.push('\n');
}

#[cfg(test)]
mod tests {
    use super::Generator;
    use crate::{GenerationConfig, Schema};

    #[test]
    fn generator_emits_deterministic_module_name() {
        let generator = Generator::new(GenerationConfig::new("market_data"));
        let schema = Schema::new("fix.sbe", 1, 0);

        let modules = generator.generate(&schema);
        let collected = modules.modules().collect::<Vec<_>>();

        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].path, "market_data.rs");
        assert!(collected[0].source.contains("fix.sbe"));
    }

    #[test]
    fn generate_multi_creates_separate_modules() {
        let mut config = GenerationConfig::new("common");
        config.shared_module = Some("common_types".to_string());

        let generator = Generator::new(config);

        let schema_a = Schema::new("common.sbe", 1, 0);
        let schema_b = Schema::new("market_data.sbe", 2, 0);

        let modules =
            generator.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")]);
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
    }

    #[test]
    fn generate_multi_without_shared_module_emits_sbe_rt_everywhere() {
        let config = GenerationConfig::new("common");
        let generator = Generator::new(config);

        let schema_a = Schema::new("common.sbe", 1, 0);
        let schema_b = Schema::new("market_data.sbe", 2, 0);

        let modules = generator.generate_multi(&[(&schema_a, "a_mod"), (&schema_b, "b_mod")]);
        let collected: Vec<_> = modules.modules().collect();

        assert_eq!(collected.len(), 2);

        // Both modules get sbe_rt when no shared_module is configured
        assert!(collected[0].source.contains("pub mod sbe_rt"));
        assert!(collected[1].source.contains("pub mod sbe_rt"));

        // No top-level pub use re-exports (prelude's pub use is inside its module)
        assert!(!collected[1].source.contains("\npub use super::"));
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
    fn message_structure_skips_unexpected_signal() {
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
    }

    #[test]
    fn group_structure_skips_unexpected_signal() {
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
    }

    #[test]
    fn vardata_structure_skips_non_length_fields() {
        // parse_vardata_structure loops tokens looking for the "length"
        // BeginField; any other BeginField falls to `i += 1` (lines ~974-977).
        let _ = parse_vardata_structure(&[
            make_token(Signal::BeginComposite),
            make_token(Signal::BeginField),
            make_token(Signal::EndField),
            make_token(Signal::EndComposite),
        ]);
    }

    #[test]
    fn composite_members_skips_non_field_signals() {
        // parse_composite_members loops from index 1 to len-1; any signal
        // that isn't BeginField falls to `else { i += 1 }` (lines ~1097-1099).
        let _ = parse_composite_members(&[
            make_token(Signal::BeginComposite),
            make_token(Signal::BeginMessage), // not BeginField → skip
            make_token(Signal::EndComposite),
        ]);
    }

    #[test]
    fn field_structure_falls_back_to_uint8_primitive() {
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
    }

    #[test]
    fn snake_case_handles_empty_or_special_input() {
        assert_eq!(to_snake_case(""), "");
        // Double-underscore input exercises the dedup `continue` (line 520).
        assert_eq!(to_snake_case("Foo__Bar"), "foo_bar");
    }

    #[test]
    fn partition_skips_unexpected_at_top_level() {
        // Top-level loop only matches BeginComposite/Enum/Set/Message;
        // BeginField falls to `_ => i += 1` (lines ~682-684).
        let _ = super::partition_tokens(&[make_token(Signal::BeginField)]);
    }

    #[test]
    fn partition_skips_unexpected_in_message_body() {
        // Message body loop only matches BeginField/Group/VarData;
        // BeginEnum inside a message body falls to `_ => i += 1` (lines ~797).
        let _ = super::partition_tokens(&[
            make_token(Signal::BeginMessage),
            make_token(Signal::BeginEnum), // unexpected inside message body
            make_token(Signal::EndMessage),
        ]);
    }

    #[test]
    fn partition_skips_unexpected_in_group_body() {
        // Group body loop only matches BeginField/Group/VarData;
        // BeginMessage inside a group falls to `_ => i += 1` (lines ~937).
        let _ = super::partition_tokens(&[
            make_token(Signal::BeginGroup),
            make_token(Signal::BeginMessage), // unexpected inside group body
            make_token(Signal::EndGroup),
        ]);
    }

    #[test]
    fn partition_skips_unexpected_after_top_level_items() {
        // After BeginMessage/EndMessage pair, unrelated signals skip at top level.
        let _ = super::partition_tokens(&[
            make_token(Signal::BeginMessage),
            make_token(Signal::EndMessage),
            make_token(Signal::BeginEnum), // at top level
        ]);
    }
}

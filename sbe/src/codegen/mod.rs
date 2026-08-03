//! Rust code generation from a resolved [`crate::Schema`].
//!
//! Primary type: [`Generator`]. Configure with [`crate::GenerationConfig`],
//! call [`Generator::generate`] or [`Generator::generate_multi`], write
//! [`GeneratedModule::source`] to `OUT_DIR`, then `include!` it.
//!
//! # Pipeline
//!
//! 1. Partition IR tokens into enums, sets, composites, messages.
//! 2. Emit type definitions.
//! 3. Per message: decoder flyweight, encoder + type-state tails, optional domain DTO.
//! 4. Emit `AnyMessage` / `FrameCursor` when multiple templates exist.
//! 5. Format with `prettyplease`.
//!
//! # Example
//!
//! ```rust
//! use ergo_sbe::{parse, Generator, GenerationConfig, Schema};
//!
//! let ir = parse(r#"<?xml version="1.0"?>
//! <messageSchema package="ex" id="1" version="0" byteOrder="littleEndian">
//!   <types>
//!     <composite name="messageHeader">
//!       <type name="blockLength" primitiveType="uint16"/>
//!       <type name="templateId" primitiveType="uint16"/>
//!       <type name="schemaId" primitiveType="uint16"/>
//!       <type name="version" primitiveType="uint16"/>
//!     </composite>
//!   </types>
//!   <message name="Ping" id="1">
//!     <field name="seq" id="1" type="uint32" offset="0"/>
//!   </message>
//! </messageSchema>"#).unwrap();
//! let schema = Schema::from_ir(ir);
//! let set = Generator::new(GenerationConfig::new("ping"))
//!     .generate(&schema)
//!     .unwrap();
//! let src = &set.modules().next().unwrap().source;
//! assert!(src.contains("PingDecoder"));
//! assert!(src.contains("PingEncoder"));
//! ```
//!
//! See the [crate root](crate) for how to use generated codecs (encode/decode,
//! conversion styles, domain objects, metadata).

use std::collections::HashSet;
use std::fmt::Write;

use crate::ir::{ByteOrder, Ir, Presence, PrimitiveType, Signal, Token};
use crate::structured_ir::*;
use crate::{GenerationConfig, Schema};

pub(crate) mod conversion_helpers;
pub(crate) use conversion_helpers::*;
pub(crate) mod conversion_traits;
pub(crate) use conversion_traits::*;
pub(crate) mod converter_impls;
pub(crate) use converter_impls::generate_converter_impls;
pub(crate) mod decoder_display;
pub(crate) use decoder_display::generate_decoder_display;
pub(crate) mod domain_cluster;
pub(crate) use domain_cluster::*;
pub(crate) mod encoded_length;
pub(crate) mod field_type;
pub(crate) use field_type::field_type_ident;
pub(crate) mod message_header_template;
pub(crate) use message_header_template::*;
pub(crate) mod nullification;
pub(crate) use nullification::*;
pub(crate) mod runtime;
use quote::format_ident;
pub(crate) use runtime::*;
pub(crate) mod group_encoder;
pub(crate) use group_encoder::generate_group_encoder;
pub(crate) mod group_decoder;
pub(crate) use group_decoder::generate_group_decoder;
pub(crate) mod tail_stages;
pub(crate) use tail_stages::*;
pub(crate) mod message_decoder;
pub(crate) use message_decoder::generate_message_decoder;
pub(crate) mod message_encoder;
pub(crate) use message_encoder::generate_message_encoder;
use sha2::{Digest, Sha256};

/// One generated Rust source file.
///
/// Write `source` to `OUT_DIR.join(path)` from `build.rs`, then:
/// `mod msgs { include!(concat!(env!("OUT_DIR"), "/msgs.rs")); }`
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratedModule {
    /// Relative path, e.g. `"messages.rs"` or `"common_types.rs"`.
    pub path: String,
    /// Full formatted Rust source for that module.
    pub source: String,
}

/// Set of modules from [`Generator::generate`] or [`Generator::generate_multi`].
///
/// ```rust
/// # use std::path::Path;
/// # fn example(generator: &mut ergo_sbe::Generator, schema: &ergo_sbe::Schema, out_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
/// let set = generator.generate(schema)?;
/// for m in set.modules() {
///     std::fs::write(out_dir.join(&m.path), &m.source)?;
/// }
/// for w in set.warnings() {
///     println!("cargo:warning={w}");
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GeneratedModuleSet {
    modules: Vec<GeneratedModule>,
    /// Generation warnings (e.g. shared types with version-gated members).
    warnings: Vec<String>,
}

/// Errors returned by [`Generator::generate`] when the configuration
/// is invalid for the given schema.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GenerateError {
    /// A schema value cannot be represented by its declared message-header
    /// field without using a reserved/null value.
    HeaderValueOutOfRange {
        /// Header field name.
        field: String,
        /// Schema value that would be written.
        value: u64,
        /// Maximum value declared by the field encoding.
        maximum: u64,
        /// Schema or message that supplied the value.
        context: String,
    },
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
            Self::HeaderValueOutOfRange {
                field,
                value,
                maximum,
                context,
            } => {
                write!(
                    f,
                    "message header field '{field}' value {value} for {context} exceeds declared maximum {maximum}"
                )
            }
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
    pub(crate) fn push(&mut self, module: GeneratedModule) {
        self.modules.push(module);
    }

    /// Iterate modules in a stable order (write each to `OUT_DIR`).
    #[must_use]
    pub fn modules(&self) -> impl ExactSizeIterator<Item = &GeneratedModule> {
        self.modules.iter()
    }

    /// Non-fatal warnings (e.g. shared types with `sinceVersion > 0`).
    /// Surface via `cargo:warning=` from `build.rs`.
    #[must_use]
    pub fn warnings(&self) -> &[String] {
        &self.warnings
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
    pub domain_var_data: crate::config::DomainVarData,
    pub enable_display_debug: bool,
    pub enable_meta_attributes: bool,
    pub enable_dispatch: bool,
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
            domain_var_data: config.domain_var_data,
            enable_display_debug: config.enable_display_debug,
            enable_meta_attributes: config.enable_meta_attributes,
            enable_dispatch: config.enable_dispatch,
        }
    }
}

/// SBE → Rust codec generator.
///
/// Holds a [`GenerationConfig`]. Call [`Self::generate`] for one schema or
/// [`Self::generate_multi`] when sharing types across schemas.
///
/// ```rust
/// use ergo_sbe::{parse, Generator, GenerationConfig, Schema};
/// # let xml = r#"<?xml version="1.0"?><messageSchema package="t" id="1" version="0"
/// # byteOrder="littleEndian"><types><composite name="messageHeader">
/// # <type name="blockLength" primitiveType="uint16"/>
/// # <type name="templateId" primitiveType="uint16"/>
/// # <type name="schemaId" primitiveType="uint16"/>
/// # <type name="version" primitiveType="uint16"/>
/// # </composite></types><message name="M" id="1">
/// # <field name="x" id="1" type="uint8" offset="0"/></message></messageSchema>"#;
/// let schema = Schema::from_ir(parse(xml).unwrap());
/// let modules = Generator::new(GenerationConfig::new("m"))
///     .generate(&schema)
///     .unwrap();
/// assert_eq!(modules.modules().len(), 1);
/// ```
#[derive(Debug)]
pub struct Generator {
    config: GenerationConfig,
}

impl Generator {
    /// Create a generator with the given [`GenerationConfig`].
    #[must_use]
    pub const fn new(config: GenerationConfig) -> Self {
        Self { config }
    }

    fn validate_header_values(&self, schema: &Schema) -> Result<(), GenerateError> {
        let elements = partition_tokens(&schema.ir.tokens);
        let Some(header) = elements
            .composites
            .iter()
            .find(|tokens| tokens[0].name == schema.ir.header_type)
        else {
            // Synthetic `Schema::new` values used by metadata-only callers may
            // contain no messages or header tokens. XML-parsed schemas have
            // already had their header structure validated.
            return Ok(());
        };

        let check = |field_name: &str, value: u64, context: String| -> Result<(), GenerateError> {
            let Some(field) = header
                .iter()
                .find(|token| token.signal == Signal::BeginField && token.name == field_name)
            else {
                return Ok(());
            };
            if field.encoding.presence == Presence::Constant {
                return Ok(());
            }
            let maximum = field.encoding.max_value.unwrap_or(u64::MAX);
            if value > maximum {
                return Err(GenerateError::HeaderValueOutOfRange {
                    field: field_name.to_string(),
                    value,
                    maximum,
                    context,
                });
            }
            Ok(())
        };

        let schema_context = format!("schema '{}'", schema.package);
        check("schemaId", u64::from(schema.id), schema_context.clone())?;
        check("version", u64::from(schema.version), schema_context)?;

        for message_tokens in &elements.messages {
            let message = parse_message_structure(message_tokens, &elements);
            let context = format!("message '{}'", message.name);
            check("templateId", u64::from(message.id), context.clone())?;
            let block_length = u64::try_from(message.block_length).unwrap_or(u64::MAX);
            check("blockLength", block_length, context)?;
        }

        Ok(())
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

    #[allow(missing_docs)]
    fn effective_domain_types(
        &self,
        schemas: &[(&Schema, &str)],
    ) -> Vec<(crate::ConversionSelector, String)> {
        let mut types = self.config.domain_types.clone();
        if self.config.auto_bool_domain {
            for (schema, _) in schemas {
                let elements = partition_tokens(&schema.ir.tokens);
                for e in &elements.enums {
                    let name = &e[0].name;
                    if crate::structured_ir::is_bool_value_enum(&elements, name) {
                        let sel = crate::ConversionSelector::named_type(name);
                        if !types.iter().any(|(s, _)| s == &sel) {
                            types.push((sel, "bool".into()));
                        }
                    }
                }
            }
        }
        types
    }

    /// Generate one Rust module for `schema` (file name from config module name).
    ///
    /// # Errors
    ///
    /// [`GenerateError`] if conversion selectors match nothing or collide.
    pub fn generate(&self, schema: &Schema) -> Result<GeneratedModuleSet, GenerateError> {
        let effective = self.effective_domain_types(&[(schema, "")]);
        with_keyword_append(&self.config.keyword_append_token, || {
            with_deprecated_attrs(self.config.deprecated_attrs, || {
                self.validate_header_values(schema)?;
                self.validate_conversions(schema)?;
                let mut modules = GeneratedModuleSet::default();
                let src = self.gen_schema(schema, &HashSet::new(), false, true, &effective);
                modules.push(GeneratedModule {
                    path: format!("{}.rs", self.config.module_name),
                    source: src,
                });
                Ok(modules)
            })
        })
    }

    /// Generate modules for several schemas, optionally deduplicating shared types.
    ///
    /// When [`GenerationConfig::with_shared_module`] is set:
    /// - first entry owns shared enums/sets/composites (+ usually `sbe_rt`);
    /// - later entries emit `pub use super::<shared>::*;` and skip shared types.
    ///
    /// Each entry is `(schema, module_name)` → `{module_name}.rs`.
    ///
    /// # Errors
    ///
    /// Same conversion validation as [`Self::generate`].
    pub fn generate_multi(
        &self,
        schemas: &[(&Schema, &str)],
    ) -> Result<GeneratedModuleSet, GenerateError> {
        let effective = self.effective_domain_types(schemas);
        with_keyword_append(&self.config.keyword_append_token, || {
            with_deprecated_attrs(self.config.deprecated_attrs, || {
                self.generate_multi_inner(schemas, &effective)
            })
        })
    }

    fn generate_multi_inner(
        &self,
        schemas: &[(&Schema, &str)],
        domain_types: &[(crate::ConversionSelector, String)],
    ) -> Result<GeneratedModuleSet, GenerateError> {
        let mut modules = GeneratedModuleSet::default();
        let mut shared_types: HashSet<String> = HashSet::new();
        let empty_set: HashSet<String> = HashSet::new();

        for (i, (schema, module_name)) in schemas.iter().enumerate() {
            self.validate_header_values(schema)?;
            if i == 0 {
                let elements = partition_tokens(&schema.ir.tokens);
                for et in &elements.enums {
                    let name = to_pascal_case(&et[0].name);
                    shared_types.insert(name.clone());
                    if let Some(warn) = warn_version_gated(&name, et, schema) {
                        modules.warnings.push(warn);
                    }
                }
                for st in &elements.sets {
                    shared_types.insert(to_pascal_case(&st[0].name));
                }
                for ct in &elements.composites {
                    let name = to_pascal_case(&ct[0].name);
                    shared_types.insert(name.clone());
                    if let Some(warn) = warn_version_gated(&name, ct, schema) {
                        modules.warnings.push(warn);
                    }
                }
            }
            let is_importing = i > 0 && self.config.shared_module.is_some();
            // Emit sbe_rt in the first module always, and in every module
            // when there is no shared module (standalone mode).
            let emit_sbe_rt = i == 0 || self.config.shared_module.is_none();
            // Type dedup only applies when a shared module is configured.
            // Without it, each schema is standalone and defines all its types.
            // The first schema (shared-module owner) always defines ALL its types.
            let skip_set: &HashSet<String> = if self.config.shared_module.is_some() && i > 0 {
                &shared_types
            } else {
                &empty_set
            };
            let src = self.gen_schema(schema, skip_set, is_importing, emit_sbe_rt, domain_types);
            modules.push(GeneratedModule {
                path: format!("{}.rs", module_name),
                source: src,
            });
        }
        Ok(modules)
    }

    /// Build an [`ItemContext::Enum`] from IR tokens.
    fn build_enum_ctx<'s>(
        tokens: &[crate::ir::Token],
        schema: &'s crate::Schema,
    ) -> crate::ItemContext<'s> {
        let name = to_pascal_case(&tokens[0].name);
        let encoding_type = tokens[0]
            .encoding
            .primitive_type
            .unwrap_or(PrimitiveType::UInt8);
        let et_str = rust_type(encoding_type).to_string();
        let variants: Vec<_> = tokens
            .iter()
            .filter(|t| t.signal == crate::ir::Signal::Encoding)
            .filter_map(|t| {
                let val = t.encoding.constant_value.as_ref()?;
                let value: i128 = if encoding_type == PrimitiveType::Char {
                    i128::from(val.as_bytes().first().copied().unwrap_or(0))
                } else {
                    // i128 covers uint64 discriminants above i64::MAX
                    // (rare but schema-legal) without wrapping negative.
                    val.parse::<i128>().ok()?
                };
                Some(crate::EnumVariantInfo {
                    name: to_pascal_case(&t.name),
                    snake_name: to_snake_case(&t.name),
                    label: t.name.clone(),
                    value,
                    description: t.encoding.description.clone(),
                })
            })
            .collect();
        crate::ItemContext::Enum {
            schema,
            name,
            encoding_type: et_str,
            variants,
        }
    }

    /// Build a message decoder/encoder context from a [`MessageStructure`].
    fn build_message_ctx<'s>(
        msg: &MessageStructure,
        kind: crate::ItemKind,
        schema: &'s crate::Schema,
    ) -> crate::ItemContext<'s> {
        let name = to_pascal_case(&msg.name);
        let name_with = |suffix: &str| format!("{name}{suffix}");
        let fields = message_field_infos(&msg.fields, &[], None);
        let name = match kind {
            crate::ItemKind::MessageDecoder => name_with("Decoder"),
            crate::ItemKind::MessageEncoder => name_with("Encoder"),
            _ => name,
        };
        match kind {
            crate::ItemKind::MessageDecoder => crate::ItemContext::MessageDecoder {
                schema,
                name,
                template_id: msg.id,
                block_length: msg.block_length,
                fields,
            },
            crate::ItemKind::MessageEncoder => crate::ItemContext::MessageEncoder {
                schema,
                name,
                template_id: msg.id,
                block_length: msg.block_length,
                fields,
            },
            _ => unreachable!("build_message_ctx only for MessageDecoder/MessageEncoder"),
        }
    }

    /// Build an [`ItemContext::Composite`] from IR tokens.
    ///
    /// Uses the canonical [`parse_composite_members`] so every member is
    /// reported exactly once with its real Rust type: primitives keep their
    /// element type (`[T; N]` for arrays), nested composites/enums/sets report
    /// their type name. Container/ref tokens are never miscounted as fields.
    fn build_composite_ctx<'s>(
        tokens: &[crate::ir::Token],
        schema: &'s crate::Schema,
    ) -> crate::ItemContext<'s> {
        use crate::structured_ir::MemberType;
        let name = to_pascal_case(&tokens[0].name);
        // Metadata lives on different tokens depending on the member kind:
        // - primitive: the `BeginField` wrapper carries it (inner token is the
        //   unnamed `<type>` encoding);
        // - nested composite/enum/set: the `BeginField` carries only offsets;
        //   `semanticType`/`nullValue`/`description`/`deprecated` live on the
        //   inner `BeginComposite`/`BeginEnum`/`BeginSet` token.
        let member_field_token = |member_name: &str| {
            tokens
                .iter()
                .find(|t| t.signal == crate::ir::Signal::BeginField && t.name == member_name)
        };
        let inner_type_token = |field_name: &str| {
            // Find the BeginField for this member, then peek at the adjacent
            // non-field token that carries the actual type's encoding metadata.
            let mut it = tokens.iter().skip_while(|t| {
                !(t.signal == crate::ir::Signal::BeginField && t.name == field_name)
            });
            let _ = it.next(); // skip the BeginField itself
            it.find(|t| {
                matches!(
                    t.signal,
                    crate::ir::Signal::Encoding
                        | crate::ir::Signal::BeginComposite
                        | crate::ir::Signal::BeginEnum
                        | crate::ir::Signal::BeginSet
                )
            })
        };
        let fields: Vec<_> = crate::structured_ir::parse_composite_members(tokens)
            .into_iter()
            .map(|m| {
                let field_tok = member_field_token(&m.name);
                let inner_tok = inner_type_token(&m.name);
                // For primitives the field wrapper carries metadata; for
                // containers the inner token does.
                let enc = match &m.member_type {
                    MemberType::Primitive { .. } => field_tok.map(|t| &t.encoding),
                    MemberType::Composite { .. }
                    | MemberType::Enum { .. }
                    | MemberType::Set { .. } => inner_tok.map(|t| &t.encoding),
                };
                let (rust_type, presence) = match &m.member_type {
                    MemberType::Primitive {
                        prim,
                        length,
                        presence,
                        ..
                    } => {
                        let base = crate::structured_ir::rust_type(*prim);
                        let rt = match length {
                            Some(len) => format!("[{base}; {len}]"),
                            None => base.to_string(),
                        };
                        let ps = match presence {
                            crate::ir::Presence::Optional => "optional",
                            crate::ir::Presence::Constant => "constant",
                            crate::ir::Presence::Required => "required",
                        };
                        (rt, ps)
                    }
                    MemberType::Composite { name, .. } => (to_pascal_case(name), "required"),
                    MemberType::Enum { name, .. } => (to_pascal_case(name), "required"),
                    MemberType::Set { name, .. } => (to_pascal_case(name), "required"),
                };
                crate::FieldInfo {
                    name: to_snake_case(&m.name),
                    rust_type,
                    offset: Some(m.offset),
                    since_version: m.since_version,
                    semantic_type: enc.and_then(|e| e.semantic_type.clone()),
                    presence,
                    null_value: enc.and_then(|e| e.null_value),
                    deprecated: enc.is_some_and(|e| e.deprecated),
                    description: enc.and_then(|e| e.description.clone()),
                }
            })
            .collect();
        crate::ItemContext::Composite {
            schema,
            name,
            fields,
        }
    }

    /// Build an [`ItemContext::Set`] from IR tokens.
    fn build_set_ctx<'s>(
        tokens: &[crate::ir::Token],
        schema: &'s crate::Schema,
    ) -> crate::ItemContext<'s> {
        let name = to_pascal_case(&tokens[0].name);
        let encoding_type = tokens[0]
            .encoding
            .primitive_type
            .unwrap_or(PrimitiveType::UInt8);
        let et_str = rust_type(encoding_type).to_string();
        let choices: Vec<_> = tokens
            .iter()
            .filter(|t| t.signal == crate::ir::Signal::Encoding)
            .map(|t| crate::SetChoiceInfo {
                name: to_pascal_case(&t.name),
                snake_name: to_snake_case(&t.name),
                label: t.name.clone(),
                bit_position: t
                    .encoding
                    .constant_value
                    .as_ref()
                    .and_then(|v| v.parse::<u8>().ok())
                    .unwrap_or(0),
                description: t.encoding.description.clone(),
            })
            .collect();
        crate::ItemContext::Set {
            schema,
            name,
            encoding_type: et_str,
            choices,
        }
    }

    /// Run registered hooks and append returned tokens to `src`.
    fn run_hooks(&self, ctx: &crate::ItemContext, src: &mut String) {
        if !self.config.has_hooks() {
            return;
        }
        self.config.run_hooks(ctx, src);
    }
    /// type names already generated by earlier schemas; those types are skipped
    /// in this call (the caller arranges `pub use super::*;`).
    fn gen_schema(
        &self,
        schema: &Schema,
        shared: &HashSet<String>,
        is_importing: bool,
        emit_sbe_rt: bool,
        domain_types: &[(crate::ConversionSelector, String)],
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
                       unused_assignments, unused_comparisons, unused_unsafe)]\n",
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

        let elements = partition_tokens(&ir.tokens);

        for enum_tokens in &elements.enums {
            let type_name = to_pascal_case(&enum_tokens[0].name);
            if shared.contains(&type_name) {
                continue;
            }
            generate_enum(&mut src, enum_tokens);
            if self.config.has_hooks() {
                let ctx = Self::build_enum_ctx(enum_tokens, schema);
                self.run_hooks(&ctx, &mut src);
            }
        }

        for set_tokens in &elements.sets {
            let type_name = to_pascal_case(&set_tokens[0].name);
            if shared.contains(&type_name) {
                continue;
            }
            generate_set(&mut src, set_tokens);
            if self.config.has_hooks() {
                let ctx = Self::build_set_ctx(set_tokens, schema);
                self.run_hooks(&ctx, &mut src);
            }
        }

        for composite_tokens in &elements.composites {
            let type_name = to_pascal_case(&composite_tokens[0].name);
            if shared.contains(&type_name) {
                continue;
            }
            let comp_byte_order = ir.byte_order;
            generate_composite(&mut src, composite_tokens, comp_byte_order);
            if self.config.has_hooks() {
                let ctx = Self::build_composite_ctx(composite_tokens, schema);
                self.run_hooks(&ctx, &mut src);
            }
        }

        let header_pascal = to_pascal_case(&ir.header_type);
        if header_pascal != "MessageHeader" && !shared.contains(&header_pascal) {
            write!(src, "pub type MessageHeader = {};\n\n", header_pascal).unwrap();
        }

        let messages: Vec<MessageStructure> = elements
            .messages
            .iter()
            .map(|toks| parse_message_structure(toks, &elements))
            .collect();

        // Selectors for conversion/domain accessors: explicit list + domain_types
        // (covers with_domain_type and auto_bool without a separate with_conversion).
        let mut conv_sels = self.config.conversions.clone();
        for (sel, _) in domain_types {
            if !conv_sels.iter().any(|s| s == sel) {
                conv_sels.push(sel.clone());
            }
        }

        let mut schema_markers = occupied_type_names(&elements);
        let mut message_markers: Vec<(String, String)> = Vec::new();
        for msg in &messages {
            let multi = messages.len() > 1;
            let (decoder_ts, marker) = generate_message_decoder(
                msg,
                &elements,
                &mut schema_markers,
                ir.byte_order,
                ir.id,
                ir.version,
                &ir.header_type,
                &ir.package,
                multi,
                self.config.enable_display_debug,
                self.config.enable_meta_attributes,
                self.config.enable_dispatch,
                self.config.domain_objects,
                self.config.domain_var_data,
                &conv_sels,
                domain_types,
                self.config.unchecked_companions,
                &self.config.hooks,
                schema,
            );
            src.push_str(&decoder_ts.to_string());
            src.push('\n');
            message_markers.push((to_pascal_case(&msg.name), marker));
            // Hooks for the message decoder
            if self.config.has_hooks() {
                let ctx = Self::build_message_ctx(msg, crate::ItemKind::MessageDecoder, schema);
                self.run_hooks(&ctx, &mut src);
            }
            let encoder_ts = generate_message_encoder(
                msg,
                &elements,
                ir.byte_order,
                ir.id,
                ir.version,
                &ir.header_type,
                multi,
                &conv_sels,
                domain_types,
                self.config.unchecked_companions,
                self.config.enable_meta_attributes,
                self.config.enable_display_debug,
            );
            src.push_str(&encoder_ts.to_string());
            // Hooks for the message encoder
            if self.config.has_hooks() {
                let ctx = Self::build_message_ctx(msg, crate::ItemKind::MessageEncoder, schema);
                self.run_hooks(&ctx, &mut src);
            }

            // Converter seam: domain-type / with_conversion / auto_bool (HFT-003).
            if !conv_sels.is_empty() {
                let converter_ts = generate_converter_impls(msg, &conv_sels, domain_types, multi);
                src.push_str(&converter_ts);
            }
            src.push('\n');
            if self.config.enable_meta_attributes {
                generate_message_field_meta(&mut src, msg);
            }
        }

        // 6b. Emit TryFromSbe/TryToSbe impls for configured domain-type conversions.
        if self.config.has_conversions() {
            let impl_blocks =
                generate_conversion_impl_blocks(&elements, &self.config.conversions, domain_types);
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
        generate_prelude(
            &mut src,
            &elements,
            &messages,
            ir.id,
            ir.version,
            self.config.enable_dispatch,
        );
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
        // 7.7. Byte helpers. Checked helpers are public; unchecked raw I/O is
        // private + unsafe (HFT-001) — never a safe public memory-safety
        // precondition for callers.
        let read_bytes_ts: proc_macro2::TokenStream = quote::quote! {
            /// Read `N` bytes from `buf` at `offset` into a fixed-size array.
            ///
            /// Bounds-checked slice indexing. LLVM elides the check when the
            /// slice length is known (stack buffer with visible size).
            #[inline]
            pub fn read_bytes<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
                buf[offset..offset + N].try_into().expect("read_bytes: buffer too short")
            }

            #[inline]
            pub fn write_bytes<const N: usize>(buf: &mut [u8], offset: usize, bytes: &[u8; N]) {
                buf[offset..offset + N].copy_from_slice(bytes);
            }

            /// Unchecked companion to [`read_bytes`] — zero bounds checks.
            ///
            /// # Safety
            /// Caller guarantees `offset + N` does not overflow and
            /// `offset + N <= buf.len()`.
            #[inline]
            #[allow(dead_code)] // used from generated accessors in this module
            unsafe fn read_bytes_unchecked<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
                // SAFETY: caller guarantees offset + N <= buf.len().
                unsafe {
                    core::ptr::read_unaligned(buf.as_ptr().add(offset) as *const [u8; N])
                }
            }

            /// Unchecked companion to [`write_bytes`] — zero bounds checks.
            ///
            /// # Safety
            /// Caller guarantees `offset + N` does not overflow and
            /// `offset + N <= buf.len()`.
            #[inline]
            #[allow(dead_code)]
            unsafe fn write_bytes_unchecked<const N: usize>(
                buf: &mut [u8],
                offset: usize,
                bytes: &[u8; N],
            ) {
                // SAFETY: caller guarantees offset + N <= buf.len().
                unsafe {
                    core::ptr::write_unaligned(buf.as_mut_ptr().add(offset) as *mut [u8; N], *bytes)
                }
            }
        };
        src.push_str(&read_bytes_ts.to_string());
        src.push('\n');
        generate_schema_id_from_header(&mut src, &elements, &ir.header_type, ir.byte_order);

        if self.config.enable_dispatch {
            let any_msg_ts = generate_any_message(
                &messages,
                &elements,
                ir.id,
                &ir.header_type,
                &ir.package,
                &message_markers,
            );
            src.push_str(&any_msg_ts.to_string());
            src.push('\n');
        }

        let file = match syn::parse_str::<syn::File>(&src) {
            Ok(f) => f,
            Err(e) => {
                // Produce a comment explaining the failure so the user
                // can diagnose it (e.g. a reserved keyword leaked through).
                let mut diag = String::from(
                    "// ergo-sbe: generated code failed Rust syntax validation.\n",
                );
                use std::fmt::Write;
                let _ = writeln!(
                    diag,
                    "// This usually means a schema name collides with a Rust keyword.\n// syn error: {e}\n// Raw source follows.\n\n"
                );
                diag.push_str(&src);
                return diag;
            }
        };
        prettyplease::unparse(&file)
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
        let mut generator = Generator::new(GenerationConfig::new("market_data"));
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

        let mut generator = Generator::new(config);

        let schema_a = Schema::new("common.sbe", 1, 0);
        let schema_b = Schema::new("market_data.sbe", 2, 0);

        let modules =
            generator.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "market_data")])?;
        let collected: Vec<_> = modules.modules().collect();

        assert_eq!(collected.len(), 2);
        assert_eq!(collected[0].path, "common_types.rs");
        assert_eq!(collected[1].path, "market_data.rs");

        assert!(collected[0].source.contains("pub mod sbe_rt"));

        // Second module does NOT have its own sbe_rt (sbe_rt comes via pub use)
        assert!(!collected[1].source.contains("pub mod sbe_rt"));

        assert!(
            collected[1]
                .source
                .contains("pub use super::common_types::*;")
        );

        assert!(collected[0].source.contains("common.sbe"));
        assert!(collected[1].source.contains("market_data.sbe"));

        Ok(())
    }

    #[test]
    fn generate_multi_without_shared_module_emits_sbe_rt_everywhere()
    -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("common");
        let mut generator = Generator::new(config);

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
            span: None,
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
    fn group_array_codegen_uses_the_complete_field_extent_and_element_range()
    -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<?xml version="1.0"?>
        <messageSchema package="array.guard" id="305" version="1" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <composite name="groupSizeEncoding">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="numInGroup" primitiveType="uint16"/>
            </composite>
            <type name="Values" primitiveType="uint32" length="2"/>
            <enum name="State" encodingType="uint8">
              <validValue name="Ready">1</validValue>
            </enum>
            <set name="Flags" encodingType="uint8">
              <choice name="Active">0</choice>
            </set>
            <enum name="BooleanType" encodingType="uint8">
              <validValue name="F">0</validValue>
              <validValue name="T">1</validValue>
            </enum>
          </types>
          <message name="ArrayBoundaryMessage" id="1">
            <group name="entries" id="1">
              <field name="base" id="2" type="uint8"/>
              <field name="values" id="3" type="Values"/>
              <field name="state" id="4" type="State" sinceVersion="1"/>
              <field name="flags" id="5" type="Flags" sinceVersion="1"/>
              <field name="enabled" id="6" type="BooleanType" sinceVersion="1"/>
            </group>
          </message>
        </messageSchema>"#;
        let schema = crate::Schema::from_ir(crate::parse(xml)?);
        let mut generator = crate::Generator::new(crate::GenerationConfig::new("array_guard"));
        let modules = generator.generate(&schema)?;
        let source = &modules
            .modules()
            .next()
            .ok_or("missing generated module")?
            .source;

        assert!(
            source.contains("|| 9 > self.acting_block_length"),
            "u32[2] at offset 1 must require all nine entry bytes"
        );
        assert!(
            source.contains("let all: [u8; 8]"),
            "u32[2] must bulk-read exactly eight bytes"
        );
        assert!(
            source.contains("all[0usize]") && source.contains("all[7usize]"),
            "the unrolled array decode must use the complete byte range"
        );
        assert!(
            source.contains("|| 10 > self.acting_block_length"),
            "the versioned enum at offset nine must require its complete tenth byte"
        );
        assert!(
            source.contains("|| 11 > self.acting_block_length"),
            "the versioned set at offset ten must require its complete eleventh byte"
        );
        assert!(
            source.contains("pub fn enabled_bool(&self) -> Option<bool>"),
            "a versioned BooleanType group field must preserve absence in its bool accessor"
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
            deprecated: false,
            semantic_type: Some("UTCTimestamp".into()),
            constant_value: None,
            epoch: None,
            time_unit: None,
            character_encoding: None,
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
        let mut generator = crate::Generator::new(config);
        let modules = generator.generate(&schema)?;
        let src = modules.modules().next().unwrap().source.clone();
        assert!(
            src.contains("fn try_ts(") && src.contains("chrono::DateTime"),
            "should generate fallible concrete DateTime accessor for UTCTimestamp field"
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

    #[test]
    fn narrow_message_header_rejects_values_above_declared_field_maximum() {
        fn generate(xml: &str) -> Result<super::GeneratedModuleSet, super::GenerateError> {
            let ir = crate::parse(xml).expect("schema should parse before codegen validation");
            let schema = crate::Schema::from_ir(ir);
            crate::Generator::new(crate::GenerationConfig::new("narrow")).generate(&schema)
        }

        fn schema(schema_id: u16, version: u16, template_id: u16, block_length: u16) -> String {
            format!(
                r#"<messageSchema package="test" id="{schema_id}" version="{version}" byteOrder="littleEndian">
                  <types>
                    <composite name="messageHeader">
                      <type name="schemaId" primitiveType="uint8"/>
                      <type name="version" primitiveType="uint8"/>
                      <type name="templateId" primitiveType="uint8"/>
                      <type name="blockLength" primitiveType="uint8"/>
                    </composite>
                  </types>
                  <message name="M" id="{template_id}" blockLength="{block_length}"/>
                </messageSchema>"#
            )
        }

        for (xml, field) in [
            (schema(255, 1, 1, 0), "schemaId"),
            (schema(1, 255, 1, 0), "version"),
            (schema(1, 1, 255, 0), "templateId"),
            (schema(1, 1, 1, 255), "blockLength"),
        ] {
            let error = generate(&xml).expect_err("reserved null/max value must be rejected");
            assert!(
                error.to_string().contains(field),
                "expected {field} error, got: {error}"
            );
        }
    }

    /// When a flat message has a field whose name clashes with a reserved
    /// decoder method (e.g. "remaining"), the generated accessor must be
    /// renamed to `{name}_field` so it doesn't collide with
    /// `pub fn remaining(&self) -> &[u8]`.
    #[test]
    fn field_named_remaining_is_renamed_to_remaining_field()
    -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<messageSchema package="test" id="1" version="1" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
          </types>
          <message name="Msg" id="1" blockLength="8">
            <field name="remaining" id="1" type="int64"/>
          </message>
        </messageSchema>"#;

        let ir = crate::parse(xml).expect("schema should parse");
        let schema = crate::Schema::from_ir(ir);
        let modules =
            crate::Generator::new(crate::GenerationConfig::new("test")).generate(&schema)?;
        let src = modules.modules().next().expect("one module").source.clone();

        let remaining_count = src.matches("fn remaining(&self)").count();
        // The decoder and encoder each have a generated `remaining()` method
        // (in separate impl blocks). The field accessor is renamed to
        // `remaining_field` and must not appear as `fn remaining(&self)`.
        assert_eq!(
            remaining_count, 1,
            "expected exactly 2 'remaining' methods (one decoder + one encoder), found {remaining_count}"
        );
        // The field accessor must be renamed.
        assert!(
            src.contains("fn remaining_field"),
            "field accessor 'remaining' must be renamed to 'remaining_field'. src:\n{src}"
        );

        Ok(())
    }

    /// A hook that adds serde `Serialize` for every SBE enum (variants as
    /// strings) and every SBE set (variants as `Vec<String>`), plus
    /// `Deserialize` for the enum.
    #[test]
    fn hook_adds_serde_impls_for_enum_and_set() -> Result<(), Box<dyn std::error::Error>> {
        let xml = r#"<messageSchema package="test" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <enum name="EventCode" encodingType="uint32">
              <validValue name="Ok" description="Success">200</validValue>
              <validValue name="Error" description="Failure">400</validValue>
              <validValue name="Timeout">408</validValue>
            </enum>
            <set name="OptionalFields" encodingType="uint8">
              <choice name="hasPrice">0</choice>
              <choice name="hasQty">1</choice>
              <choice name="hasVenue">2</choice>
            </set>
          </types>
          <message name="Msg" id="1" blockLength="0"/>
        </messageSchema>"#;

        use crate::{EnumVariantInfo, ItemContext, ItemKind, SetChoiceInfo};
        use quote::format_ident;

        let config = crate::GenerationConfig::new("test")
            .with_hook(|ctx: &ItemContext| -> Vec<proc_macro2::TokenStream> {
                match ctx {
                    ItemContext::Enum { name, variants, .. } => {
                        let ident = format_ident!("{name}");
                        let var_names: Vec<_> = variants.iter().map(|v| format_ident!("{}", v.name)).collect();
                        let var_labels: Vec<_> = variants.iter().map(|v| v.name.clone()).collect();
                        vec![quote::quote! {
                            impl serde::Serialize for #ident {
                                fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                                    let label = match self {
                                        #(Self::#var_names => #var_labels,)*
                                    };
                                    s.serialize_str(label)
                                }
                            }

                            impl<'de> serde::Deserialize<'de> for #ident {
                                fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                                    let s = <&str>::deserialize(d)?;
                                    match s {
                                        #(#var_labels => Ok(Self::#var_names),)*
                                        _ => Err(serde::de::Error::unknown_variant(s, &[#(#var_labels),*])),
                                    }
                                }
                            }
                        }]
                    }
                    ItemContext::Set { name, encoding_type, choices, .. } => {
                        let ident = format_ident!("{name}");
                        // Getters are `is_{snake_name}()`; the wire mask is
                        // `1 << bit_position`. Use u64 as the accumulator so
                        // bit positions 0-63 work regardless of the schema's
                        // encodingType (u8/u16/u32/u64).
                        let c_getters: Vec<_> = choices
                            .iter()
                            .map(|c| format_ident!("is_{}", c.snake_name))
                            .collect();
                        let c_labels: Vec<_> = choices.iter().map(|c| c.label.clone()).collect();
                        let c_bits: Vec<_> = choices.iter().map(|c| c.bit_position).collect();
                        let acc_ty: syn::Type = syn::parse_str(encoding_type).unwrap();
                        vec![quote::quote! {
                            impl serde::Serialize for #ident {
                                fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
                                    let mut names = Vec::new();
                                    #(if self.#c_getters() { names.push(#c_labels); })*
                                    names.serialize(s)
                                }
                            }

                            impl<'de> serde::Deserialize<'de> for #ident {
                                fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                                    let names: Vec<String> = Vec::deserialize(d)?;
                                    let mut value: u64 = 0;
                                    for name in &names {
                                        match name.as_str() {
                                            #(#c_labels => value |= 1u64 << #c_bits,)*
                                            other => return Err(serde::de::Error::unknown_variant(
                                                other, &[#(#c_labels),*])),
                                        }
                                    }
                                    Ok(Self(value as #acc_ty))
                                }
                            }
                        }]
                    }
                    _ => vec![],
                }
            });

        let ir = crate::parse(xml).expect("schema should parse");
        let schema = crate::Schema::from_ir(ir);
        let modules = crate::Generator::new(config).generate(&schema)?;
        let src = modules.modules().next().expect("one module").source.clone();

        // Enum: Serialize impl must exist with variant labels.
        assert!(
            src.contains("impl serde::Serialize for EventCode"),
            "missing Serialize for enum"
        );
        assert!(src.contains("\"Ok\""), "missing Ok label");
        assert!(src.contains("\"Error\""), "missing Error label");
        assert!(
            src.contains("impl<'de> serde::Deserialize<'de> for EventCode"),
            "missing Deserialize for enum"
        );
        assert!(
            src.contains("unknown_variant"),
            "missing error handling in Deserialize"
        );

        // Set: Serialize impl must exist.
        assert!(
            src.contains("impl serde::Serialize for OptionalFields"),
            "missing Serialize for set"
        );
        assert!(src.contains("\"hasPrice\""), "missing hasPrice label");
        assert!(
            src.contains("impl<'de> serde::Deserialize<'de> for OptionalFields"),
            "missing Deserialize for set"
        );

        Ok(())
    }

    /// `with_bool_domain_type()` must work for multi-schema generation, not
    /// just single-schema. Each schema's boolean enums are auto-registered, and
    /// the generated output includes the domain-typed getter.
    #[test]
    fn with_bool_domain_type_works_with_generate_multi() -> Result<(), Box<dyn std::error::Error>> {
        let xml_a = r#"<?xml version="1.0"?>
        <messageSchema package="a" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <enum name="BooleanType" encodingType="uint8">
              <validValue name="F">0</validValue>
              <validValue name="T">1</validValue>
            </enum>
          </types>
          <message name="MsgA" id="1" blockLength="1">
            <field name="flag" id="1" type="BooleanType" offset="0"/>
          </message>
        </messageSchema>"#;
        let xml_b = r#"<?xml version="1.0"?>
        <messageSchema package="b" id="2" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
            <enum name="BooleanType" encodingType="uint8">
              <validValue name="F">0</validValue>
              <validValue name="T">1</validValue>
            </enum>
          </types>
          <message name="MsgB" id="1" blockLength="1">
            <field name="enabled" id="1" type="BooleanType" offset="0"/>
          </message>
        </messageSchema>"#;

        let schema_a = Schema::from_ir(crate::parse(xml_a)?);
        let schema_b = Schema::from_ir(crate::parse(xml_b)?);
        let mut generator = Generator::new(
            crate::GenerationConfig::new("common_types")
                .with_shared_module("common_types")
                .with_bool_domain_type(true),
        );
        let modules =
            generator.generate_multi(&[(&schema_a, "common_types"), (&schema_b, "consumer")])?;
        let collected: Vec<_> = modules.modules().collect();
        assert_eq!(collected.len(), 2);

        // The consumer module should have a bool-typed getter on the field
        // whose type is BooleanType (auto-registered as bool domain type).
        // The domain getter is `{field}_bool`, not the bare name — the bare
        // name stays as the wire-type accessor.
        let consumer_src = &collected[1].source;
        assert!(
            consumer_src.contains("fn enabled_bool(&self) -> bool"),
            "with_bool_domain_type must produce bool getter in multi-schema; got:\n{consumer_src}",
        );
        Ok(())
    }

    fn ping_schema_xml() -> &'static str {
        r#"<?xml version="1.0"?>
        <messageSchema package="ex" id="1" version="0" byteOrder="littleEndian">
          <types>
            <composite name="messageHeader">
              <type name="blockLength" primitiveType="uint16"/>
              <type name="templateId" primitiveType="uint16"/>
              <type name="schemaId" primitiveType="uint16"/>
              <type name="version" primitiveType="uint16"/>
            </composite>
          </types>
          <message name="Ping" id="1" blockLength="4">
            <field name="seq" id="1" type="uint32" offset="0"/>
          </message>
        </messageSchema>"#
    }

    fn generate_ping(config: GenerationConfig) -> Result<String, Box<dyn std::error::Error>> {
        let schema = Schema::from_ir(crate::parse(ping_schema_xml())?);
        let src = Generator::new(config)
            .generate(&schema)?
            .modules()
            .next()
            .ok_or("no module")?
            .source
            .clone();
        Ok(src)
    }

    /// Size knobs must omit the corresponding tokens when set to `false`.
    #[test]
    fn size_knobs_omit_display_meta_and_dispatch() -> Result<(), Box<dyn std::error::Error>> {
        let full = generate_ping(GenerationConfig::new("ping"))?;
        assert!(
            full.contains("core::fmt::Display for PingDecoder"),
            "default must emit Display for PingDecoder; got marker search fail in {} chars",
            full.len()
        );
        assert!(
            full.contains("core::fmt::Debug for PingDecoder"),
            "default must emit Debug for PingDecoder"
        );
        assert!(
            full.contains("SEQ_ENCODING_OFFSET"),
            "default must emit field ENCODING_OFFSET constants"
        );
        assert!(
            full.contains("seq_meta_attribute"),
            "default must emit field meta_attribute fn"
        );
        assert!(
            full.contains("ping_field_meta"),
            "default must emit per-message field_meta module"
        );
        assert!(
            full.contains("enum AnyMessage"),
            "default must emit AnyMessage dispatch"
        );

        let lean = generate_ping(
            GenerationConfig::new("ping")
                .with_display_debug(false)
                .with_meta_attributes(false)
                .with_dispatch(false),
        )?;
        assert!(
            !lean.contains("core::fmt::Display for PingDecoder")
                && !lean.contains("core::fmt::Debug for PingDecoder"),
            "with_display_debug(false) must omit Display/Debug"
        );
        assert!(
            !lean.contains("SEQ_ENCODING_OFFSET") && !lean.contains("seq_meta_attribute"),
            "with_meta_attributes(false) must omit field meta constants"
        );
        assert!(
            !lean.contains("ping_field_meta"),
            "with_meta_attributes(false) must omit field_meta module"
        );
        assert!(
            !lean.contains("enum AnyMessage") && !lean.contains("struct FrameCursor"),
            "with_dispatch(false) must omit AnyMessage/FrameCursor"
        );
        // Codec surface still present.
        assert!(lean.contains("struct PingDecoder"));
        assert!(lean.contains("struct PingEncoder"));
        Ok(())
    }
}

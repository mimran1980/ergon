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
#[non_exhaustive]
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
    /// Generated module source failed its own syntax check. This is always an
    /// ergo-sbe codegen bug — report it.
    InvalidGeneratedSource {
        /// Which module produced invalid Rust.
        module: String,
        /// The syn parse error.
        error: String,
    },
    /// A [`GenerationConfig`] field was rejected by codegen validation.
    InvalidConfiguration {
        /// Which config option was rejected.
        option: String,
        /// The rejected value.
        value: String,
        /// Why it was rejected.
        reason: String,
    },
    /// Multi-schema generation found the same type name with incompatible
    /// wire layouts across schemas.
    IncompatibleSharedType {
        /// Shared type name (enum, set, or composite).
        name: String,
        /// Module that first defined the type.
        owner_module: String,
        /// Module that reuses the name with a different layout.
        consumer_module: String,
        /// First differing property / fingerprint mismatch summary.
        difference: String,
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
            Self::InvalidGeneratedSource { module, error } => {
                write!(
                    f,
                    "generated module '{module}' failed Rust syntax validation: {error}"
                )
            }
            Self::InvalidConfiguration {
                option,
                value,
                reason,
            } => {
                write!(
                    f,
                    "invalid configuration option '{option}': value '{value}' — {reason}"
                )
            }
            Self::IncompatibleSharedType {
                name,
                owner_module,
                consumer_module,
                difference,
            } => {
                write!(
                    f,
                    "shared type '{name}' is wire-incompatible between modules \
                     '{owner_module}' (owner) and '{consumer_module}': {difference}"
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

    /// Consume the set and take ownership of the generated modules and
    /// warnings without cloning those buffers.
    ///
    /// Module order matches [`Self::modules`] (generation order; stable for
    /// a given schema set). Warnings are returned rather than discarded so a
    /// `build.rs` can still emit `cargo::warning=` after taking the source.
    ///
    /// ```rust
    /// # fn example(set: ergo_sbe::GeneratedModuleSet) {
    /// let (modules, warnings) = set.into_parts();
    /// for m in modules {
    ///     let _ = (m.path, m.source);
    /// }
    /// for w in warnings {
    ///     println!("cargo::warning={w}");
    /// }
    /// # }
    /// ```
    #[must_use]
    pub fn into_parts(self) -> (Vec<GeneratedModule>, Vec<String>) {
        (self.modules, self.warnings)
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
    pub domain_objects: bool,
    pub domain_var_data: crate::config::DomainVarData,
    pub enable_display_debug: bool,
    pub enable_meta_attributes: bool,
    pub enable_dispatch: bool,
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

        let schema_context = format!("schema '{}'", &schema.package);
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
            // Validate the path is parseable Rust — catch typos at build time,
            // not as a panic deep in codegen.
            syn::parse_str::<syn::Type>(rust_type).map_err(|e| {
                GenerateError::InvalidConversion {
                    selector: format!("{sel:?}"),
                    reason: format!("domain type path is not a valid Rust type: {e}"),
                }
            })?;
        }
        Ok(())
    }

    /// Validate conversions against the token union of all schemas in a
    /// multi-schema generation. A [`crate::ConversionSelector::NamedType`]
    /// that only exists in one schema's type declarations is valid as long
    /// as at least one schema in the union contains the named type.
    fn validate_conversions_union(
        &self,
        union: &[(&Schema, crate::structured_ir::SchemaElements)],
    ) -> Result<(), GenerateError> {
        if !self.config.has_conversions() {
            return Ok(());
        }
        for sel in &self.config.conversions {
            if let crate::ConversionSelector::NamedType(name) = sel {
                let matched = union.iter().any(|(_, elements)| {
                    elements.composites.iter().any(|c| c[0].name == *name)
                        || elements.enums.iter().any(|e| e[0].name == *name)
                        || elements.sets.iter().any(|s| s[0].name == *name)
                });
                if !matched {
                    return Err(GenerateError::InvalidConversion {
                        selector: format!("{sel:?}"),
                        reason: "no matching type found in any schema".into(),
                    });
                }
            }
            // SemanticType and FieldPath are validated during codegen per-field.
        }
        for (sel, rust_type) in &self.config.domain_types {
            syn::parse_str::<syn::Type>(rust_type).map_err(|e| {
                GenerateError::InvalidConversion {
                    selector: format!("{sel:?}"),
                    reason: format!("domain type path is not a valid Rust type: {e}"),
                }
            })?;
        }
        Ok(())
    }

    /// Validate user-supplied paths that will be parsed by syn later. Catches
    /// typos at config-validation time rather than as panics in codegen.
    fn validate_paths(&self) -> Result<(), GenerateError> {
        // Module name must be a single Rust identifier (no path separators).
        let mn = self.config.module_name();
        if !crate::config::is_valid_module_ident(mn) {
            return Err(GenerateError::InvalidConfiguration {
                option: "module_name".into(),
                value: mn.into(),
                reason:
                    "module name must be a single Rust identifier — no '/', '\\\\', '.', or '..'"
                        .into(),
            });
        }
        if let Some(ref err_path) = self.config.error_from_path {
            syn::parse_str::<syn::Type>(err_path).map_err(|e| {
                GenerateError::InvalidConversion {
                    selector: "error_from_path".into(),
                    reason: format!("error-from path is not a valid Rust type: {e}"),
                }
            })?;
        }
        if let Some(ref rt_path) = self.config.external_sbe_rt_path {
            syn::parse_str::<syn::Path>(rt_path).map_err(|e| {
                GenerateError::InvalidConfiguration {
                    option: "external_sbe_rt".into(),
                    value: rt_path.clone(),
                    reason: format!("not a valid Rust path: {e}"),
                }
            })?;
        }
        // A schema field named after a Rust keyword gets this token appended
        // (`type` -> `type_`). Prove a representative keyword plus the
        // configured token forms a valid, non-keyword identifier before
        // generation — an empty or invalid token produces uncompilable
        // generated Rust with no clear diagnostic otherwise.
        let token = &self.config.keyword_append_token;
        let candidate = format!("type{token}");
        let reason = if syn::parse_str::<syn::Ident>(&candidate).is_err() {
            Some(format!(
                "\"type{token}\" is not a valid Rust identifier — \
                 keyword_append_token must combine with a keyword to form one"
            ))
        } else if is_rust_keyword(&candidate) {
            Some(format!(
                "\"type{token}\" is itself a Rust keyword — \
                 keyword_append_token must produce a non-keyword identifier"
            ))
        } else {
            None
        };
        if let Some(reason) = reason {
            return Err(GenerateError::InvalidConfiguration {
                option: "keyword_append_token".into(),
                value: token.clone(),
                reason,
            });
        }
        Ok(())
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
                self.validate_paths()?;
                let mut modules = GeneratedModuleSet::default();
                let src = self.gen_schema(schema, &HashSet::new(), false, true, &effective)?;
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
                self.validate_paths()?;
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

        // Validate per-schema module names before emitting any file.
        {
            let mut seen = HashSet::new();
            for (i, (_, module_name)) in schemas.iter().enumerate() {
                if !crate::config::is_valid_module_ident(module_name) {
                    return Err(GenerateError::InvalidConfiguration {
                        option: format!("schemas[{i}].module_name"),
                        value: module_name.to_string(),
                        reason:
                            "module name must be a single Rust identifier — no '/', '\\\\', '.', or '..'"
                                .into(),
                    });
                }
                if !seen.insert(module_name.to_string()) {
                    return Err(GenerateError::InvalidConfiguration {
                        option: format!("schemas[{i}].module_name"),
                        value: module_name.to_string(),
                        reason:
                            "duplicate module name — each schema must have a unique module name"
                                .into(),
                    });
                }
            }
        }

        // Validate shared types have identical wire fingerprints when names
        // collide. A type name is not wire identity — same-name types with
        // different layouts or byte order silently produce corrupted codecs.
        if schemas.len() > 1 && self.config.shared_module.is_some() {
            let owner_module = schemas[0].1.to_string();
            let owner_byte_order = schemas[0].0.ir.byte_order;
            let first_elements = partition_tokens(&schemas[0].0.ir.tokens);
            for (schema, consumer_module) in schemas.iter().skip(1) {
                let elements = partition_tokens(&schema.ir.tokens);
                let consumer_byte_order = schema.ir.byte_order;
                let check = |kind: &str, name: String, a: String, b: String| {
                    if a != b {
                        Err(GenerateError::IncompatibleSharedType {
                            name: name.clone(),
                            owner_module: owner_module.clone(),
                            consumer_module: consumer_module.to_string(),
                            difference: format!(
                                "{kind} fingerprint mismatch (owner={a}, consumer={b})"
                            ),
                        })
                    } else {
                        Ok(())
                    }
                };
                // Compare enums
                for et in &elements.enums {
                    let name = to_pascal_case(&et[0].name);
                    if let Some(ref_et) = first_elements
                        .enums
                        .iter()
                        .find(|e| to_pascal_case(&e[0].name) == name)
                    {
                        check(
                            "enum",
                            name,
                            canonical_token_fingerprint(ref_et, owner_byte_order),
                            canonical_token_fingerprint(et, consumer_byte_order),
                        )?;
                    }
                }
                // Compare sets
                for st in &elements.sets {
                    let name = to_pascal_case(&st[0].name);
                    if let Some(ref_st) = first_elements
                        .sets
                        .iter()
                        .find(|s| to_pascal_case(&s[0].name) == name)
                    {
                        check(
                            "set",
                            name,
                            canonical_token_fingerprint(ref_st, owner_byte_order),
                            canonical_token_fingerprint(st, consumer_byte_order),
                        )?;
                    }
                }
                // Compare composites
                for ct in &elements.composites {
                    let name = to_pascal_case(&ct[0].name);
                    if let Some(ref_ct) = first_elements
                        .composites
                        .iter()
                        .find(|c| to_pascal_case(&c[0].name) == name)
                    {
                        check(
                            "composite",
                            name,
                            canonical_token_fingerprint(ref_ct, owner_byte_order),
                            canonical_token_fingerprint(ct, consumer_byte_order),
                        )?;
                    }
                }
            }
        }

        // Shared module name must identify the first-schema (owner) module so
        // consumers' `pub use super::<shared>::*` resolves to the owner crate
        // path rather than a free-floating alias.
        if let Some(ref shared) = self.config.shared_module {
            let owner = schemas[0].1;
            if shared != owner {
                return Err(GenerateError::InvalidConfiguration {
                    option: "shared_module".into(),
                    value: shared.clone(),
                    reason: format!(
                        "must equal the first schema module name (owner '{owner}'); \
                         consumers import `super::{shared}::*` from that module"
                    ),
                });
            }
        }

        // Validate conversions against the union of all schemas' types, not
        // each schema individually. A NamedType selector may only exist in
        // one schema's type declarations (valid), and the union covers all.
        {
            let mut union_elements: Vec<(&Schema, crate::structured_ir::SchemaElements)> =
                Vec::with_capacity(schemas.len());
            for (schema, _) in schemas.iter() {
                union_elements.push((schema, partition_tokens(&schema.ir.tokens)));
            }
            self.validate_conversions_union(&union_elements)?;
        }

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
            let src = self.gen_schema(schema, skip_set, is_importing, emit_sbe_rt, domain_types)?;
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
        let mut fields = message_field_infos(&msg.fields, &[], None);
        for g in &msg.groups {
            fields.push(crate::FieldInfo {
                name: to_snake_case(&g.name),
                rust_type: "group".to_string(),
                offset: None,
                since_version: g.since_version,
                semantic_type: None,
                presence: "required",
                null_value: None,
                deprecated: g.deprecated,
                description: g.description.clone(),
            });
        }
        for vd in &msg.var_data {
            fields.push(crate::FieldInfo {
                name: to_snake_case(&vd.name),
                rust_type: "data".to_string(),
                offset: None,
                since_version: vd.since_version,
                semantic_type: None,
                presence: "required",
                null_value: None,
                deprecated: vd.deprecated,
                description: vd.description.clone(),
            });
        }
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
                    deprecated: enc.map(|e| e.deprecated).unwrap_or(false),
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
    ) -> Result<String, GenerateError> {
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
            &schema.package, schema.id, schema.version
        )
        .unwrap();
        // Lint allow list is intentionally narrow — do not re-add
        // unused_unsafe / unused_imports / dead_code (hide generator bugs).
        // Remaining allows (schema reality):
        // - absurd_extreme_comparisons / identity_op / erasing_op / unnecessary_cast:
        //   schema min/max/const offsets can be tautological after folding.
        // - double_must_use: staged builders return must_use types from must_use methods.
        // - eq_op: schema-driven `x == x` style checks in generated matches.
        // - manual_range_contains: generated version gates prefer explicit compares
        //   that stay readable next to sinceVersion literals.
        // - non_camel_case_types / non_snake_case: SBE identifiers as emitted.
        src.push_str(
            "#[allow(clippy::absurd_extreme_comparisons, clippy::double_must_use, \
                       clippy::erasing_op, clippy::identity_op, clippy::unnecessary_cast)]\n",
        );
        src.push_str("#[allow(non_camel_case_types)]\n");
        src.push_str("#[allow(non_snake_case)]\n");
        src.push_str("#[allow(clippy::eq_op)]\n");
        src.push_str("#[allow(clippy::manual_range_contains)]\n\n");

        // If importing from a shared module, bring all its items into scope.
        // This covers shared types + the sbe_rt runtime module.
        if is_importing {
            if let Some(ref shared_mod) = self.config.shared_module {
                write!(src, "pub use super::{}::*;\n\n", shared_mod).unwrap();
            }
        }

        // `SbeMessage`'s sealing marker lives with the runtime that declares the
        // trait, so a module reusing someone else's `sbe_rt` must name that
        // owner's sealing module rather than declaring a second one.
        let sealed_path = if let Some(ref ext) = self.config.external_sbe_rt_path {
            let owner = ext.strip_suffix("::sbe_rt").unwrap_or(ext);
            format!("{owner}::{}", crate::codegen::runtime::SEALED_MODULE)
        } else if is_importing {
            let shared = self
                .config
                .shared_module
                .as_deref()
                .expect("is_importing implies a shared module");
            format!(
                "super::{shared}::{}",
                crate::codegen::runtime::SEALED_MODULE
            )
        } else {
            crate::codegen::runtime::SEALED_MODULE.to_string()
        };
        let gen_ctx = crate::codegen::runtime::GenerationContext {
            sealed_path: syn::parse_str(&sealed_path)
                .expect("sealing module path must be a valid Rust path"),
        };

        if let Some(ref ext) = self.config.external_sbe_rt_path {
            let _ = writeln!(src, "pub use {ext} as sbe_rt;\n");
            if self.config.has_conversions() {
                emit_conversion_traits(&mut src);
            }
        } else if emit_sbe_rt {
            src.push_str(&generate_sbe_rt_src());
            // A shared runtime is implemented against by sibling modules, so its
            // sealing module widens to `pub(super)`. A self-contained module
            // keeps it private, which is what makes `SbeMessage` unimplementable
            // outside the generated module.
            src.push_str(&crate::codegen::runtime::generate_sealed_module_src(
                self.config.shared_module.is_some(),
            ));
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
                &self.config.hooks,
                schema,
                &self.config.null_as_option,
                self.config.all_enums_as_option,
                &gen_ctx,
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
                self.config.enable_meta_attributes,
                self.config.enable_display_debug,
                &gen_ctx,
            );
            src.push_str(&encoder_ts.to_string());
            // Hooks for the message encoder
            if self.config.has_hooks() {
                let ctx = Self::build_message_ctx(msg, crate::ItemKind::MessageEncoder, schema);
                self.run_hooks(&ctx, &mut src);
            }

            // Converter seam: domain-type / with_conversion / auto_bool.
            if !conv_sels.is_empty() {
                let manual_impl_snippets = generate_manual_impl_snippets(
                    &elements,
                    domain_types,
                    &self.config.manual_impl_selectors,
                );
                let converter_ts = generate_converter_impls(
                    msg,
                    &conv_sels,
                    domain_types,
                    &manual_impl_snippets,
                    multi,
                );
                src.push_str(&converter_ts);
            }
            src.push('\n');
            if self.config.enable_meta_attributes {
                generate_message_field_meta(&mut src, msg);
            }
        }

        // 6b. Emit TryFromSbe/TryToSbe impls for configured domain-type conversions.
        // Only the module that owns `sbe_rt` emits these. The built-in impls
        // target well-known types (`bool`, `rust_decimal`, `chrono`), and a
        // shared-module consumer re-emitting `impl TryFromSbe<BooleanType> for
        // bool` against the imported `BooleanType` collides with the owner's
        // identical impl ("conflicting implementation"). Every non-shared
        // module owns its own `sbe_rt`, so this still fires for each of them.
        if self.config.has_conversions() && emit_sbe_rt {
            let impl_blocks = generate_conversion_impl_blocks(
                &elements,
                &self.config.conversions,
                domain_types,
                &self.config.manual_impl_selectors,
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
        // private + unsafe — never a safe public memory-safety
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
            // `always`: pairs with scalar getter `#[inline(always)]` for no-LTO
            // decode_scalar parity (plain `#[inline]` lost the maintained gate).
            #[inline(always)]
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

        let mut file = match syn::parse_str::<syn::File>(&src) {
            Ok(f) => f,
            Err(e) => {
                return Err(GenerateError::InvalidGeneratedSource {
                    module: self.config.module_name.clone(),
                    error: e.to_string(),
                });
            }
        };
        annotate_missing_public_docs(&mut file);
        Ok(prettyplease::unparse(&file))
    }
}

fn item_is_public(vis: &syn::Visibility) -> bool {
    matches!(vis, syn::Visibility::Public(_))
}

fn attrs_have_doc(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| a.path().is_ident("doc"))
}

fn fallback_public_doc(kind: &str, name: &str) -> syn::Attribute {
    let text = format!("Generated {kind} `{name}`.");
    syn::parse_quote!(#[doc = #text])
}

fn doc_is_placeholder(attr: &syn::Attribute) -> bool {
    if !attr.path().is_ident("doc") {
        return false;
    }
    let syn::Meta::NameValue(nv) = &attr.meta else {
        return false;
    };
    let syn::Expr::Lit(syn::ExprLit {
        lit: syn::Lit::Str(s),
        ..
    }) = &nv.value
    else {
        return false;
    };
    s.value().trim() == "Generated public API."
}

fn ensure_public_doc(attrs: &mut Vec<syn::Attribute>, kind: &str, name: &str) {
    if attrs.iter().any(doc_is_placeholder)
        && attrs
            .iter()
            .filter(|a| a.path().is_ident("doc"))
            .all(doc_is_placeholder)
    {
        attrs.retain(|a| !doc_is_placeholder(a));
    }
    if !attrs_have_doc(attrs) {
        attrs.insert(0, fallback_public_doc(kind, name));
    }
}

/// Fill operational-fallback rustdoc on every public item so generated
/// codecs compile under `#![deny(missing_docs)]` even when the schema
/// omits descriptions.
fn annotate_missing_public_docs(file: &mut syn::File) {
    for item in &mut file.items {
        annotate_item(item);
    }
}

fn annotate_item(item: &mut syn::Item) {
    match item {
        syn::Item::Struct(s) if item_is_public(&s.vis) => {
            let name = s.ident.to_string();
            ensure_public_doc(&mut s.attrs, "struct", &name);
            for field in &mut s.fields {
                if !item_is_public(&field.vis) {
                    continue;
                }
                match &field.ident {
                    Some(ident) => ensure_public_doc(&mut field.attrs, "field", &ident.to_string()),
                    None => {
                        // Keep `pub struct Engine(pub [u8; N])` on one line;
                        // the struct rustdoc covers the wire image.
                    }
                }
            }
        }
        syn::Item::Enum(e) if item_is_public(&e.vis) => {
            let name = e.ident.to_string();
            ensure_public_doc(&mut e.attrs, "enum", &name);
            for variant in &mut e.variants {
                ensure_public_doc(&mut variant.attrs, "variant", &variant.ident.to_string());
                for field in &mut variant.fields {
                    if let Some(ident) = &field.ident {
                        ensure_public_doc(&mut field.attrs, "field", &ident.to_string());
                    }
                }
            }
        }
        syn::Item::Fn(f) if item_is_public(&f.vis) => {
            ensure_public_doc(&mut f.attrs, "function", &f.sig.ident.to_string());
        }
        syn::Item::Const(c) if item_is_public(&c.vis) => {
            ensure_public_doc(&mut c.attrs, "constant", &c.ident.to_string());
        }
        syn::Item::Type(t) if item_is_public(&t.vis) => {
            ensure_public_doc(&mut t.attrs, "type", &t.ident.to_string());
        }
        syn::Item::Trait(t) if item_is_public(&t.vis) => {
            let name = t.ident.to_string();
            ensure_public_doc(&mut t.attrs, "trait", &name);
            for trait_item in &mut t.items {
                match trait_item {
                    syn::TraitItem::Fn(f) => {
                        ensure_public_doc(&mut f.attrs, "method", &f.sig.ident.to_string());
                    }
                    syn::TraitItem::Const(c) => {
                        ensure_public_doc(&mut c.attrs, "constant", &c.ident.to_string());
                    }
                    syn::TraitItem::Type(ty) => {
                        ensure_public_doc(&mut ty.attrs, "type", &ty.ident.to_string());
                    }
                    _ => {}
                }
            }
        }
        syn::Item::Impl(i) => {
            for impl_item in &mut i.items {
                match impl_item {
                    syn::ImplItem::Fn(f) if item_is_public(&f.vis) => {
                        ensure_public_doc(&mut f.attrs, "method", &f.sig.ident.to_string());
                    }
                    syn::ImplItem::Const(c) if item_is_public(&c.vis) => {
                        ensure_public_doc(&mut c.attrs, "constant", &c.ident.to_string());
                    }
                    syn::ImplItem::Type(t) if item_is_public(&t.vis) => {
                        ensure_public_doc(&mut t.attrs, "type", &t.ident.to_string());
                    }
                    _ => {}
                }
            }
        }
        syn::Item::Mod(m) if item_is_public(&m.vis) => {
            ensure_public_doc(&mut m.attrs, "module", &m.ident.to_string());
            if let Some((_, items)) = &mut m.content {
                for nested in items {
                    annotate_item(nested);
                }
            }
        }
        syn::Item::Use(u) if item_is_public(&u.vis) => {
            ensure_public_doc(&mut u.attrs, "import", "use");
        }
        syn::Item::Static(s) if item_is_public(&s.vis) => {
            ensure_public_doc(&mut s.attrs, "static", &s.ident.to_string());
        }
        _ => {}
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
    fn into_parts_preserves_module_order_and_warnings() -> Result<(), Box<dyn std::error::Error>> {
        let mut set = super::GeneratedModuleSet::default();
        set.push(super::GeneratedModule {
            path: "common_types.rs".into(),
            source: "mod common;".into(),
        });
        set.push(super::GeneratedModule {
            path: "market_data.rs".into(),
            source: "mod market;".into(),
        });
        set.warnings
            .push("shared type Price has sinceVersion > 0".into());
        let expected_paths: Vec<String> = set.modules().map(|m| m.path.clone()).collect();
        let expected_warnings = set.warnings().to_vec();
        let (modules, warnings) = set.into_parts();
        assert_eq!(
            modules.iter().map(|m| m.path.as_str()).collect::<Vec<_>>(),
            expected_paths
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        );
        assert_eq!(modules[0].source, "mod common;");
        assert_eq!(modules[1].source, "mod market;");
        assert_eq!(warnings, expected_warnings);
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
            source.contains("9 > self.acting_block_length"),
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
            source.contains("pub fn try_enabled_bool(&self) -> Result<Option<bool>,")
                && source.contains("InvalidBoolean"),
            "a versioned BooleanType group field must carry the typed bool accessor"
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

    /// Placement utils live on metadata only — a field named `remaining` keeps
    /// its natural accessor name and does not force `_field`.
    #[test]
    fn field_named_remaining_keeps_name_placement_on_metadata()
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

        assert!(
            src.contains("fn remaining(&self) -> i64")
                || src.contains("fn remaining(&self) -> i64,"),
            "field accessor must keep name remaining() as i64. src snippet check failed"
        );
        assert!(
            !src.contains("fn remaining_field"),
            "placement-name fields must not be renamed to remaining_field"
        );
        assert!(
            src.contains("fn get_metadata("),
            "placement utils must be on get_metadata()"
        );
        // Metadata still exposes remaining() as a byte slice utility.
        assert!(
            src.contains("DecoderMetadata"),
            "DecoderMetadata type must be emitted"
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
            consumer_src.contains("fn try_enabled_bool"),
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

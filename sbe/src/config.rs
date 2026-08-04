//! Code generation configuration ([`GenerationConfig`]).
//!
//! # Conversion: pick **one** style per selector
//!
//! | API | When to use | Generated decode | Generated encode |
//! |-----|-------------|------------------|------------------|
//! | [`GenerationConfig::with_conversion`] | Pluggable adapters; no forced crate dep | `dec.price_as::<T>()?` | `enc.price_from(&t)?` |
//! | [`GenerationConfig::with_domain_type`] | One canonical app type | `dec.try_price()? -> path::Type` | `enc.try_price(value)?` |
//!
//! `with_domain_type` **implies** conversion for that selector. Do **not** also
//! call `with_conversion` for the same selector.
//!
//! ```rust
//! use ergo_sbe::{GenerationConfig, ConversionSelector};
//!
//! // A — generic / pluggable (you implement TryFromSbe / TryToSbe)
//! let _a = GenerationConfig::new("msgs")
//!     .with_conversion(ConversionSelector::named_type("Decimal"));
//!
//! // B — concrete Rust type
//! let _b = GenerationConfig::new("msgs")
//!     .with_domain_type(
//!         ConversionSelector::named_type("Decimal"),
//!         "rust_decimal::Decimal",
//!     );
//! ```
//!
//! # Other features (generated surface)
//!
//! | Builder | What generated code looks like |
//! |---------|--------------------------------|
//! | [`with_domain_objects`](GenerationConfig::with_domain_objects) | `CarDomain` DTOs; pass [`DomainVarData`] for var-data shape |
//! | [`with_shared_module`](GenerationConfig::with_shared_module) | Multi-schema: shared types in one module, `pub use super::common::*` |
//! | [`with_external_sbe_rt`](GenerationConfig::with_external_sbe_rt) | `pub use path::sbe_rt as sbe_rt` instead of inlining runtime |
//! | [`with_error_from_impls`](GenerationConfig::with_error_from_impls) | `From<EncodeError> for YourError` so `?` works |
//! | [`with_keyword_append_token`](GenerationConfig::with_keyword_append_token) | Schema field `type` → `type_` (default `"_"`) |
//! | [`with_deprecated_attrs`](GenerationConfig::with_deprecated_attrs) | `#[deprecated]` on schema-deprecated items |

/// Selects which fields receive conversion / domain-type methods.
///
/// When several selectors could match the same field, precedence is:
/// 1. Exact `"Message.field"` path ([`ConversionSelector::FieldPath`])
/// 2. SBE `semanticType` ([`ConversionSelector::SemanticType`])
/// 3. Named type ([`ConversionSelector::NamedType`])
///
/// ```rust
/// use ergo_sbe::ConversionSelector;
///
/// let _ = ConversionSelector::named_type("Decimal");
/// let _ = ConversionSelector::semantic_type("UTCTimestamp");
/// let _ = ConversionSelector::field_path("Quote.price");
/// ```
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ConversionSelector {
    /// Match one field by path, e.g. `"Car.serialNumber"` or `"Quote.price"`.
    FieldPath(String),
    /// Match all fields with this SBE `semanticType` attribute (e.g. `"UTCTimestamp"`).
    SemanticType(String),
    /// Match all fields whose type name is this (composite, enum, set, alias).
    ///
    /// Example: `"Decimal"` matches every field of composite type `Decimal`.
    NamedType(String),
}

impl ConversionSelector {
    /// Select by SBE `semanticType` (e.g. `"UTCTimestamp"`, `"Price"`).
    #[must_use]
    pub fn semantic_type(name: impl Into<String>) -> Self {
        Self::SemanticType(name.into())
    }

    /// Select by named SBE type (composite / enum / set / alias), e.g. `"Decimal"`.
    #[must_use]
    pub fn named_type(name: impl Into<String>) -> Self {
        Self::NamedType(name.into())
    }

    /// Select by exact `"MessageName.fieldName"` path.
    #[must_use]
    pub fn field_path(path: impl Into<String>) -> Self {
        Self::FieldPath(path.into())
    }
}

/// How owned domain DTO `<data>` / var-data fields are typed.
///
/// Passed to [`GenerationConfig::with_domain_objects`]. Wire is always
/// length-prefixed **bytes**; this only chooses the **owned** DTO field type.
///
/// | Variant | DTO field | Invalid UTF-8 on materialise |
/// |---------|-----------|------------------------------|
/// | [`Bytes`](DomainVarData::Bytes) | `Vec<u8>` | n/a (raw copy) |
/// | [`LossyStrings`](DomainVarData::LossyStrings) | `String` | **`InvalidUtf8` error** (0.1.10; never invents empty) |
///
/// Name historical; 0.1.10 materialisation is strict (HFT-003).
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum DomainVarData {
    /// Byte-exact var-data (`Vec<u8>`) — binary tails or lossless re-encode.
    #[default]
    Bytes,
    /// Text-friendly var-data (`String`). Invalid UTF-8 returns
    /// `DecodeError::InvalidUtf8` (strict; HFT-003). Prefer
    /// [`DomainVarData::Bytes`] when non-UTF-8 tails must round-trip bit-exact.
    LossyStrings,
}

/// Generated-code surface presets (HFT-009).
///
/// Individual knobs (`with_display_debug`, …) still override after
/// [`GenerationConfig::profile`].
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum GenerationProfile {
    /// Display/Debug, field meta, multi-template dispatch, and all conveniences
    /// that the boolean knobs enable by default.
    #[default]
    Full,
    /// Byte codec + typed stages + exact sizing only. Omits Display/Debug,
    /// meta-attribute constants, and `AnyMessage`/`FrameCursor` dispatch.
    /// Domain DTOs and conversions stay off unless re-enabled explicitly.
    HftLean,
}

// ── Hook types ────────────────────────────────────────────────────────────

/// Kinds of generated items a hook can observe.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ItemKind {
    /// An SBE enum type.
    Enum,
    /// An SBE bitset type.
    Set,
    /// An SBE composite type.
    Composite,
    /// A message decoder (flyweight over `&[u8]`).
    MessageDecoder,
    /// A message encoder (writes into `&mut [u8]`).
    MessageEncoder,
    /// A domain DTO struct.
    DomainStruct,
}

/// One enum variant for hook introspection.
#[derive(Clone, Debug)]
pub struct EnumVariantInfo {
    /// Variant name in PascalCase (e.g. "Ok", "Error").
    pub name: String,
    /// Variant name in snake_case (e.g. "ok", "error").
    pub snake_name: String,
    /// Raw name from the schema (e.g. "Ok", "hasPrice"). Use for serde labels.
    pub label: String,
    /// Wire discriminant value. Widened to `i128` so `uint64` discriminants
    /// above `i64::MAX` are represented faithfully rather than wrapping negative.
    pub value: i128,
    /// Schema description, if present.
    pub description: Option<String>,
}

/// One bitset choice for hook introspection.
#[derive(Clone, Debug)]
pub struct SetChoiceInfo {
    /// Choice name in PascalCase (e.g. "HasPrice").
    pub name: String,
    /// Choice name in snake_case (e.g. "has_price"). Use for accessor calls.
    pub snake_name: String,
    /// Raw name from the schema (e.g. "hasPrice"). Use for serde labels.
    pub label: String,
    /// Zero-based bit position in the bitset.
    pub bit_position: u8,
    /// Schema description, if present.
    pub description: Option<String>,
}

/// One field for hook introspection.
#[derive(Clone, Debug)]
pub struct FieldInfo {
    /// Field name in snake_case.
    pub name: String,
    /// Rust type (e.g. "i64", "u8", "EventCode").
    pub rust_type: String,
    /// Byte offset from the message body start, when this is a fixed
    /// scalar/array/composite/enum/set field. `None` for groups and
    /// var-data fields, which have no single wire offset.
    pub offset: Option<usize>,
    /// Schema version this field was introduced in (0 = always present).
    pub since_version: u16,
    /// SBE `semanticType` attribute, if set.
    pub semantic_type: Option<String>,
    /// SBE presence: `"required"`, `"optional"`, or `"constant"`.
    pub presence: &'static str,
    /// Null sentinel value (optional fields only).
    pub null_value: Option<u64>,
    /// Whether the field is schema-deprecated.
    pub deprecated: bool,
    /// Schema description on the field, if present.
    pub description: Option<String>,
}

/// Per-item context passed to hooks.
///
/// Every variant carries a `schema` reference for full IR access
/// when the structured fields aren't enough.
///
/// Pattern-match on the variant to access item-specific data
/// (variants, choices, fields). Use [`quote::quote!`] in your
/// hook body to return tokens appended after the generated item.
// manual Debug/Clone because &Schema in every variant makes derive unhappy
#[derive(Clone)]
#[allow(missing_docs)]
pub enum ItemContext<'a> {
    Enum {
        schema: &'a crate::Schema,
        name: String,
        encoding_type: String,
        variants: Vec<EnumVariantInfo>,
    },
    Set {
        schema: &'a crate::Schema,
        name: String,
        encoding_type: String,
        choices: Vec<SetChoiceInfo>,
    },
    Composite {
        schema: &'a crate::Schema,
        name: String,
        fields: Vec<FieldInfo>,
    },
    MessageDecoder {
        schema: &'a crate::Schema,
        name: String,
        template_id: u16,
        block_length: usize,
        fields: Vec<FieldInfo>,
    },
    MessageEncoder {
        schema: &'a crate::Schema,
        name: String,
        template_id: u16,
        block_length: usize,
        fields: Vec<FieldInfo>,
    },
    DomainStruct {
        schema: &'a crate::Schema,
        name: String,
        fields: Vec<FieldInfo>,
    },
}

impl std::fmt::Debug for ItemContext<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (kind, name) = match self {
            Self::Enum { name, .. } => ("Enum", name.as_str()),
            Self::Set { name, .. } => ("Set", name.as_str()),
            Self::Composite { name, .. } => ("Composite", name.as_str()),
            Self::MessageDecoder { name, .. } => ("MessageDecoder", name.as_str()),
            Self::MessageEncoder { name, .. } => ("MessageEncoder", name.as_str()),
            Self::DomainStruct { name, .. } => ("DomainStruct", name.as_str()),
        };
        f.debug_struct("ItemContext")
            .field("kind", &kind)
            .field("name", &name)
            .finish()
    }
}

/// Token streams returned by hooks — appended after the generated item.
pub type HookFn = dyn Fn(&ItemContext<'_>) -> Vec<proc_macro2::TokenStream> + Send + Sync;

/// Wrapper so hooks can live in [`GenerationConfig`]. Not [`Clone`] or
/// [`PartialEq`] — hook closures can't be cloned or compared.
#[derive(Default)]
pub(crate) struct Hooks(Vec<Box<HookFn>>);

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Hooks").field(&self.0.len()).finish()
    }
}
impl Hooks {
    pub(crate) fn push(&mut self, hook: Box<HookFn>) {
        self.0.push(hook);
    }
    pub(crate) fn iter(&self) -> std::slice::Iter<'_, Box<HookFn>> {
        self.0.iter()
    }
    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

// ── GenerationConfig ──────────────────────────────────────────────────────

/// Options that shape generated Rust codecs.
///
/// Start with [`GenerationConfig::new`], chain builder methods, then pass to
/// [`crate::Generator::new`].
///
/// ```rust
/// use ergo_sbe::{DomainVarData, GenerationConfig, ConversionSelector};
///
/// let config = GenerationConfig::new("market_data")
///     .with_domain_objects(DomainVarData::LossyStrings)
///     .with_domain_type(
///         ConversionSelector::named_type("Decimal"),
///         "rust_decimal::Decimal",
///     );
/// ```
pub struct GenerationConfig {
    /// Rust module name for the generated output file (`{module_name}.rs`).
    pub(crate) module_name: String,
    /// Sibling module that already owns shared types (multi-schema mode).
    pub(crate) shared_module: Option<String>,
    /// Emit owned `*Domain` structs + `From<Decoder>` / `encode`.
    pub(crate) domain_objects: bool,
    /// Var-data shape on DTOs when `domain_objects` is set.
    pub(crate) domain_var_data: DomainVarData,
    /// Selectors for generic `*_as` / `*_from` conversion methods.
    pub(crate) conversions: Vec<ConversionSelector>,
    /// Domain-type mappings: `(selector, rust_type_path)`.
    /// Implicitly enables conversion for the same selector.
    pub(crate) domain_types: Vec<(ConversionSelector, String)>,
    /// When set, emit `pub use <path> as sbe_rt;` instead of inlining runtime.
    pub(crate) external_sbe_rt_path: Option<String>,
    /// Emit `From<EncodeError/DecodeError>` for this error type path.
    pub(crate) error_from_path: Option<String>,
    /// Emit `bool` ↔ BooleanType converters automatically for every enum
    /// detected as boolean (name `BooleanType` or `semanticType="Boolean"`).
    /// Equivalent to calling `with_domain_type(named_type(name), "bool")` for
    /// each — saves boilerplate on schemas with many boolean flags.
    pub(crate) auto_bool_domain: bool,
    /// Emit `_unchecked` companions for benchmarking.
    /// Appended when a name is a Rust keyword (default `"_"`).
    pub(crate) keyword_append_token: String,
    /// Emit `#[deprecated]` on schema-deprecated items (opt-in).
    pub(crate) deprecated_attrs: bool,
    /// Emit `Debug`/`Display` impls (default on; pass `false` to shrink output).
    pub(crate) enable_display_debug: bool,
    /// Emit meta-attribute constants (default on; pass `false` to shrink output).
    pub(crate) enable_meta_attributes: bool,
    /// Emit `AnyMessage`/`FrameCursor`/`MessageVisitor` dispatch (default on).
    pub(crate) enable_dispatch: bool,
    /// Hooks fired after each generated item (enum, set, composite, message).
    /// Returned tokens are appended after the item's definition.
    pub(crate) hooks: Hooks,
}

impl std::fmt::Debug for GenerationConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GenerationConfig")
            .field("module_name", &self.module_name)
            .field("shared_module", &self.shared_module)
            .field("domain_objects", &self.domain_objects)
            .field("domain_var_data", &self.domain_var_data)
            .field("conversions", &self.conversions)
            .field("domain_types", &self.domain_types)
            .field("external_sbe_rt_path", &self.external_sbe_rt_path)
            .field("error_from_path", &self.error_from_path)
            .field("auto_bool_domain", &self.auto_bool_domain)
            .field("keyword_append_token", &self.keyword_append_token)
            .field("deprecated_attrs", &self.deprecated_attrs)
            .field("enable_display_debug", &self.enable_display_debug)
            .field("enable_meta_attributes", &self.enable_meta_attributes)
            .field("enable_dispatch", &self.enable_dispatch)
            .field("hooks", &self.hooks)
            .finish()
    }
}

impl GenerationConfig {
    /// Create a config for output module `{module_name}.rs`.
    ///
    /// ```rust
    /// use ergo_sbe::GenerationConfig;
    /// let c = GenerationConfig::new("msgs");
    /// ```
    #[must_use]
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            shared_module: None,
            domain_objects: false,
            domain_var_data: DomainVarData::Bytes,
            conversions: Vec::new(),
            domain_types: Vec::new(),
            external_sbe_rt_path: None,
            error_from_path: None,
            keyword_append_token: "_".into(),
            deprecated_attrs: false,
            auto_bool_domain: false,
            enable_display_debug: true,
            enable_meta_attributes: true,
            enable_dispatch: true,
            hooks: Hooks::default(),
        }
    }

    /// The module name for generated output.
    #[must_use]
    pub(crate) fn module_name(&self) -> &str {
        &self.module_name
    }

    #[must_use]
    pub(crate) fn domain_objects_enabled(&self) -> bool {
        self.domain_objects
    }

    pub(crate) fn has_conversions(&self) -> bool {
        // `with_bool_domain_type` is syntax sugar for `with_domain_type` on
        // each boolean enum — it must also emit TryFromSbe/TryToSbe traits.
        !self.conversions.is_empty() || !self.domain_types.is_empty() || self.auto_bool_domain
    }

    /// The external sbe_rt path, if set.
    #[must_use]
    pub(crate) fn external_sbe_rt_path(&self) -> Option<&str> {
        self.external_sbe_rt_path.as_deref()
    }

    /// Re-use one `sbe_rt` runtime across separately generated schema modules.
    ///
    /// `path` must work in `pub use <path> as sbe_rt;`.
    ///
    /// ```
    /// # use ergo_sbe::GenerationConfig;
    /// // first module embeds sbe_rt; later modules do:
    /// // pub use crate::common::sbe_rt as sbe_rt;
    /// GenerationConfig::new("md")
    ///     .with_external_sbe_rt("crate::common::sbe_rt");
    /// ```
    #[must_use]
    pub fn with_external_sbe_rt(mut self, path: impl Into<String>) -> Self {
        self.external_sbe_rt_path = Some(path.into());
        self
    }

    /// Enable **generic** conversion methods for matching fields.
    ///
    /// # Generated API
    ///
    /// In build.rs: `.with_conversion(ConversionSelector::named_type("Decimal"))`.
    /// Application code: `enc.price_from(&my_price)?;` / `dec.price_as::<MyPrice>()?`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ergo_sbe::{GenerationConfig, ConversionSelector};
    ///
    /// let config = GenerationConfig::new("msgs")
    ///     .with_conversion(ConversionSelector::named_type("Decimal"));
    /// ```
    ///
    /// → [`sbe/tests/comprehensive_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/comprehensive_test.rs)
    ///
    /// Prefer [`Self::with_domain_type`] when one concrete Rust type is enough.
    /// Duplicate selectors are ignored; selectors matching nothing error at
    /// [`crate::Generator::generate`] time.
    #[must_use]
    pub fn with_conversion(mut self, selector: ConversionSelector) -> Self {
        if !self.conversions.contains(&selector) {
            self.conversions.push(selector);
        }
        self
    }

    /// Map matching fields to a **concrete** Rust type path.
    ///
    /// Implies [`Self::with_conversion`] for the same selector. Also emits
    /// well-known `TryFromSbe` impls for `bool`, `rust_decimal::Decimal`, and
    /// `chrono::DateTime<Utc>` when those paths are used.
    ///
    /// # Generated API
    ///
    /// In build.rs: `.with_domain_type(ConversionSelector::named_type("Decimal"), "rust_decimal::Decimal")`
    /// Application: `enc.try_price(rust_decimal::Decimal::new(12345, 2))?` / `let p = dec.try_price()?`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use ergo_sbe::{GenerationConfig, ConversionSelector};
    ///
    /// let config = GenerationConfig::new("msgs")
    ///     .with_domain_type(
    ///         ConversionSelector::named_type("Decimal"),
    ///         "rust_decimal::Decimal",
    ///     );
    /// ```
    ///
    /// Do **not** also call [`Self::with_conversion`] for the same selector.
    #[must_use]
    pub fn with_domain_type(
        mut self,
        selector: ConversionSelector,
        rust_type: impl Into<String>,
    ) -> Self {
        let sel = selector;
        let ty = rust_type.into();
        if !self.conversions.contains(&sel) {
            self.conversions.push(sel.clone());
        }
        if !self.domain_types.iter().any(|(s, _)| s == &sel) {
            self.domain_types.push((sel, ty));
        }
        self
    }

    /// Emit `From<sbe_rt::EncodeError>` / `From<sbe_rt::DecodeError>` for your error type.
    ///
    /// In build.rs: `.with_error_from_impls("crate::AppError")`.
    /// Application code: `enc.group(...)?;` — `EncodeError` auto-converts via `From`.
    #[must_use]
    pub fn with_error_from_impls(mut self, path: impl Into<String>) -> Self {
        self.error_from_path = Some(path.into());
        self
    }

    /// Generate owned domain structs next to flyweight codecs.
    ///
    /// # `var_data` — important choice ([`DomainVarData`])
    ///
    /// | Mode | DTO field | Invalid UTF-8 |
    /// |------|-----------|---------------|
    /// | [`DomainVarData::Bytes`] | `Vec<u8>` | n/a |
    /// | [`DomainVarData::LossyStrings`] | `String` | `InvalidUtf8` error (strict; HFT-003) |
    ///
    /// ```rust
    /// use ergo_sbe::{DomainVarData, GenerationConfig};
    /// let text = GenerationConfig::new("msgs")
    ///     .with_domain_objects(DomainVarData::LossyStrings);
    /// let bytes = GenerationConfig::new("msgs")
    ///     .with_domain_objects(DomainVarData::Bytes);
    /// let _ = (text, bytes);
    /// ```
    ///
    /// # Generated API
    ///
    /// `DomainVarData::LossyStrings` → `manufacturer: String`.
    /// `DomainVarData::Bytes` → `manufacturer: Vec<u8>`.
    ///
    /// → [`sbe/tests/domain_objects_test.rs`](https://github.com/mimran1980/ergon/blob/main/sbe/tests/domain_objects_test.rs)
    #[must_use]
    pub fn with_domain_objects(mut self, var_data: DomainVarData) -> Self {
        self.domain_objects = true;
        self.domain_var_data = var_data;
        self
    }

    /// Shared module name for multi-schema generation ([`crate::Generator::generate_multi`]).
    ///
    /// First schema owns shared enums/sets/composites; later modules
    /// `pub use super::<name>::*`.
    #[must_use]
    pub fn with_shared_module(mut self, name: impl Into<String>) -> Self {
        self.shared_module = Some(name.into());
        self
    }

    /// Token appended when a schema name is a Rust keyword (default `”_”`).
    ///
    /// Schema field `name="type"` becomes method `type_()`; with token `"x"`,
    /// it becomes `typex()`.
    ///
    /// ```rust
    /// use ergo_sbe::GenerationConfig;
    /// let c = GenerationConfig::new("m").with_keyword_append_token("_");
    /// let _ = c;
    /// ```
    #[must_use]
    pub fn with_keyword_append_token(mut self, token: impl Into<String>) -> Self {
        self.keyword_append_token = token.into();
        self
    }

    /// Auto-register `bool` converters for every boolean enum in the
    /// schema. Syntax sugar for calling
    /// `with_domain_type(named_type("BooleanType"), "bool")` for each —
    /// detects by name, `semanticType="Boolean"`, or True/False value pairs
    /// with discriminants `0` and `1`.
    ///
    /// Only the canonical `{0, 1}` discriminant representation is detected
    /// automatically. Schemas with non-standard boolean encodings (e.g.
    /// `Yes=5, No=3`) should use explicit [`ConversionSelector::named_type`]
    /// with [`GenerationConfig::with_conversion`] instead.
    #[must_use]
    pub fn with_bool_domain_type(mut self, enable: bool) -> Self {
        self.auto_bool_domain = enable;
        self
    }

    /// Emit `#[deprecated]` on schema-deprecated fields/types/messages.
    #[must_use]
    pub fn with_deprecated_attrs(mut self, enable: bool) -> Self {
        self.deprecated_attrs = enable;
        self
    }

    /// Control generated `Debug` and `Display` impls (**enabled by default**).
    /// Pass `false` to omit them and shrink generated output.
    #[must_use]
    pub fn with_display_debug(mut self, enable: bool) -> Self {
        self.enable_display_debug = enable;
        self
    }

    /// Control meta-attribute constants (**enabled by default**). Pass `false`
    /// to omit — removes `*_meta_attribute`, `*_ENCODING_OFFSET`,
    /// `*_ENCODING_LENGTH`, `*_ID`, `*_SINCE_VERSION`, null/min/max field
    /// constants, and the per-message `*_field_meta` module.
    #[must_use]
    pub fn with_meta_attributes(mut self, enable: bool) -> Self {
        self.enable_meta_attributes = enable;
        self
    }

    /// Control `AnyMessage` / `FrameCursor` / `MessageVisitor` dispatch code
    /// (**enabled by default**). Pass `false` to omit — saves ~300 lines;
    /// only meaningful when you do not need multi-template frame dispatch.
    #[must_use]
    pub fn with_dispatch(mut self, enable: bool) -> Self {
        self.enable_dispatch = enable;
        self
    }

    /// Apply a product profile that sets the size knobs together (HFT-009).
    ///
    /// | Profile | Display/Debug | Meta attrs | Dispatch | Domain objects |
    /// |---------|---------------|------------|----------|----------------|
    /// | [`GenerationProfile::Full`] | on | on | on | unchanged |
    /// | [`GenerationProfile::HftLean`] | off | off | off | forced off |
    ///
    /// Chain further `with_*` calls after `profile` to override individual
    /// knobs. Example:
    ///
    /// ```rust
    /// use ergo_sbe::{GenerationConfig, GenerationProfile};
    /// let _ = GenerationConfig::new("feed").profile(GenerationProfile::HftLean);
    /// ```
    #[must_use]
    pub fn profile(mut self, profile: GenerationProfile) -> Self {
        match profile {
            GenerationProfile::Full => {
                self.enable_display_debug = true;
                self.enable_meta_attributes = true;
                self.enable_dispatch = true;
            }
            GenerationProfile::HftLean => {
                self.enable_display_debug = false;
                self.enable_meta_attributes = false;
                self.enable_dispatch = false;
                self.domain_objects = false;
                self.conversions.clear();
                self.domain_types.clear();
                self.auto_bool_domain = false;
            }
        }
        self
    }

    /// Register a code-generation hook. The closure receives an
    /// [`ItemContext`] for each generated item (enum, set, composite,
    /// message decoder/encoder, domain struct) and returns token streams
    /// appended after the item's definition.
    ///
    /// Hooks fire in registration order. Use [`quote::quote!`] in your
    /// closure body to build the returned tokens.
    ///
    /// # Example — serde `Serialize` for enums
    ///
    /// ```rust
    /// use ergo_sbe::{GenerationConfig, ItemContext};
    /// use quote::quote;
    ///
    /// let config = GenerationConfig::new("msgs")
    ///     .with_hook(|ctx: &ItemContext| -> Vec<proc_macro2::TokenStream> {
    ///         match ctx {
    ///             ItemContext::Enum { name, variants, .. } => {
    ///                 // Manual Serialize impl appends after the enum definition
    ///                 vec![quote! { /* impl Serialize for ... */ }]
    ///             }
    ///             _ => vec![],
    ///         }
    ///     });
    /// ```
    #[must_use]
    pub fn with_hook<F>(mut self, hook: F) -> Self
    where
        F: Fn(&ItemContext) -> Vec<proc_macro2::TokenStream> + Send + Sync + 'static,
    {
        self.hooks.push(Box::new(hook));
        self
    }

    /// True when at least one hook is registered.
    pub(crate) fn has_hooks(&self) -> bool {
        !self.hooks.is_empty()
    }

    /// Iterate all registered hooks.
    pub(crate) fn run_hooks(&self, ctx: &ItemContext, out: &mut String) {
        for hook in self.hooks.iter() {
            for ts in hook(ctx) {
                // Use TokenStream Display impl for formatting.
                // For simple impl blocks this produces valid Rust.
                use std::fmt::Write;
                let _ = writeln!(out, "{}", ts);
            }
        }
    }
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self::new("messages")
    }
}

#[cfg(test)]
mod tests {
    use super::{ConversionSelector, DomainVarData, GenerationConfig, GenerationProfile};

    #[test]
    fn default_config_is_clean() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::default();
        assert_eq!(config.module_name(), "messages");
        assert!(!config.domain_objects_enabled());
        assert!(!config.has_conversions());
        Ok(())
    }

    #[test]
    fn with_conversion_adds_selector() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("test")
            .with_conversion(ConversionSelector::named_type("Decimal"));
        assert!(config.has_conversions());
        assert_eq!(config.conversions.len(), 1);
        Ok(())
    }

    #[test]
    fn profile_hft_lean_disables_size_knobs_and_domains() -> Result<(), Box<dyn std::error::Error>>
    {
        let full = GenerationConfig::new("m").profile(GenerationProfile::Full);
        assert!(full.enable_display_debug);
        assert!(full.enable_meta_attributes);
        assert!(full.enable_dispatch);

        let lean = GenerationConfig::new("m")
            .with_domain_objects(DomainVarData::Bytes)
            .with_conversion(ConversionSelector::named_type("Decimal"))
            .profile(GenerationProfile::HftLean);
        assert!(!lean.enable_display_debug);
        assert!(!lean.enable_meta_attributes);
        assert!(!lean.enable_dispatch);
        assert!(!lean.domain_objects);
        assert!(!lean.has_conversions());
        assert!(lean.domain_types.is_empty());

        // Later overrides still win.
        let override_dispatch = GenerationConfig::new("m")
            .profile(GenerationProfile::HftLean)
            .with_dispatch(true);
        assert!(override_dispatch.enable_dispatch);
        Ok(())
    }

    #[test]
    fn with_conversion_dedup() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("test")
            .with_conversion(ConversionSelector::named_type("Decimal"))
            .with_conversion(ConversionSelector::named_type("Decimal"));
        assert_eq!(config.conversions.len(), 1);
        Ok(())
    }

    #[test]
    fn with_domain_type_adds_conversion_and_type() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("test").with_domain_type(
            ConversionSelector::named_type("Decimal"),
            "rust_decimal::Decimal",
        );
        assert!(config.has_conversions());
        assert_eq!(config.conversions.len(), 1);
        assert_eq!(config.domain_types.len(), 1);
        Ok(())
    }

    #[test]
    fn with_domain_type_dedup() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("test")
            .with_domain_type(
                ConversionSelector::named_type("Decimal"),
                "rust_decimal::Decimal",
            )
            .with_domain_type(
                ConversionSelector::named_type("Decimal"),
                "rust_decimal::Decimal",
            );
        assert_eq!(config.domain_types.len(), 1);
        Ok(())
    }

    #[test]
    fn with_external_sbe_rt_sets_path() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("m").with_external_sbe_rt("crate::rt::sbe_rt");
        assert_eq!(config.external_sbe_rt_path(), Some("crate::rt::sbe_rt"));
        Ok(())
    }

    #[test]
    fn new_config_has_correct_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("mymod");
        assert_eq!(config.module_name(), "mymod");
        assert!(!config.domain_objects_enabled());
        assert!(config.conversions.is_empty());
        assert!(config.domain_types.is_empty());
        assert_eq!(config.domain_var_data, DomainVarData::Bytes);
        assert!(config.enable_display_debug);
        assert!(config.enable_meta_attributes);
        assert!(config.enable_dispatch);
        Ok(())
    }

    #[test]
    fn with_domain_objects_var_data_modes() -> Result<(), Box<dyn std::error::Error>> {
        let text = GenerationConfig::new("m").with_domain_objects(DomainVarData::LossyStrings);
        assert!(text.domain_objects_enabled());
        assert_eq!(text.domain_var_data, DomainVarData::LossyStrings);
        let bytes = GenerationConfig::new("m").with_domain_objects(DomainVarData::Bytes);
        assert!(bytes.domain_objects_enabled());
        assert_eq!(bytes.domain_var_data, DomainVarData::Bytes);
        Ok(())
    }

    #[test]
    fn opt_in_codegen_flags_and_field_selector_are_recorded()
    -> Result<(), Box<dyn std::error::Error>> {
        let selector = ConversionSelector::field_path("Order.price");
        assert_eq!(
            selector,
            ConversionSelector::FieldPath("Order.price".to_string())
        );

        let config = GenerationConfig::new("m")
            .with_error_from_impls("crate::AppError")
            .with_shared_module("shared")
            .with_keyword_append_token("x")
            .with_bool_domain_type(true)
            .with_deprecated_attrs(true);

        assert_eq!(config.error_from_path.as_deref(), Some("crate::AppError"));
        assert_eq!(config.shared_module.as_deref(), Some("shared"));
        assert_eq!(config.keyword_append_token, "x");
        assert!(config.auto_bool_domain);
        assert!(config.deprecated_attrs);
        Ok(())
    }
}

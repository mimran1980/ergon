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
//! // B — concrete Rust type (Generated impl is the default)
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
/// | Variant | DTO field | Feature | Invalid UTF-8 |
/// |---------|-----------|---------|---------------|
/// | [`Bytes`](DomainVarData::Bytes) | `Vec<u8>` | — | n/a |
/// | [`Strings`](DomainVarData::Strings) | `String` | — | **`InvalidUtf8` error** |
/// | `CompactStrings` | `CompactString` | `compact_str` | **`InvalidUtf8` error** |
/// | `SmolStrings` | `SmolStr` | `smol_str` | **`InvalidUtf8` error** |
/// | `BytesCrate` | `bytes::Bytes` | `bytes` | n/a |
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum DomainVarData {
    /// Byte-exact var-data (`Vec<u8>`) — binary tails or lossless re-encode.
    #[default]
    Bytes,
    /// Text-friendly var-data (`String`). Invalid UTF-8 returns
    /// `DecodeError::InvalidUtf8` (strict).
    Strings,
    /// Inline short strings (`compact_str::CompactString`, ≤24 bytes on stack).
    /// Requires feature `compact_str`.
    #[cfg(feature = "compact_str")]
    CompactStrings,
    /// O(1)-clone strings (`smol_str::SmolStr`). Requires feature `smol_str`.
    #[cfg(feature = "smol_str")]
    SmolStrings,
    /// Zero-copy shared buffer (`bytes::Bytes`). Requires feature `bytes`.
    #[cfg(feature = "bytes")]
    BytesCrate,
}

/// Who writes the `TryFromSbe`/`TryToSbe` impl for a built-in
/// [`GenerationConfig::with_domain_type`] mapping (`bool` /
/// `rust_decimal::Decimal` / `chrono::DateTime<Utc>`).
///
/// Only matters for those three well-known type paths — any other
/// `rust_type` never gets an auto-generated impl regardless of this setting,
/// since ergo-sbe has no built-in conversion logic to offer for it.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Hash)]
pub enum DomainImpl {
    /// ergo-sbe writes the impl for you (default).
    #[default]
    Generated,
    /// You write `impl TryFromSbe<Wire>` / `impl TryToSbe<Wire>` yourself —
    /// e.g. a custom rounding rule, or null/validation behaviour the
    /// built-in impl doesn't match. ergo-sbe still generates the concrete
    /// `try_price(...)?` / `try_price()?` signatures that call it.
    Manual,
}

/// Generated-code surface presets.
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
    Lean,
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

/// Kind of SBE field described by [`FieldInfo`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FieldKind {
    /// Fixed-block scalar, array, composite, enum, or set.
    Fixed,
    /// Repeating group.
    Group,
    /// Variable-length data.
    Data,
}

/// One field for hook introspection.
#[derive(Clone, Debug)]
pub struct FieldInfo {
    /// Field name in snake_case.
    pub name: String,
    /// Rust type (e.g. "i64", "u8", "EventCode", "LevelsDecoder", "&[u8]").
    /// Groups and var-data report a generated or slice type, never the
    /// sentinels `"group"` / `"data"`. Use [`Self::kind`] to classify.
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

impl FieldInfo {
    /// Classify this field as fixed, repeating group, or var-data.
    #[must_use]
    pub fn kind(&self) -> FieldKind {
        if self.offset.is_some() {
            return FieldKind::Fixed;
        }
        if self.rust_type.ends_with("Decoder")
            || (self.rust_type.starts_with("Vec<") && self.rust_type.contains("Domain"))
        {
            FieldKind::Group
        } else {
            FieldKind::Data
        }
    }
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
pub enum ItemContext<'a> {
    /// An SBE enum after codegen of its Rust type.
    Enum {
        /// Full schema IR for advanced hooks.
        schema: &'a crate::Schema,
        /// Enum type name (PascalCase).
        name: String,
        /// Wire encoding type (e.g. `"u8"`).
        encoding_type: String,
        /// Variants in schema order.
        variants: Vec<EnumVariantInfo>,
    },
    /// An SBE bitset after codegen.
    Set {
        /// Full schema IR for advanced hooks.
        schema: &'a crate::Schema,
        /// Set type name (PascalCase).
        name: String,
        /// Wire encoding type (e.g. `"u8"`).
        encoding_type: String,
        /// Bit choices in schema order.
        choices: Vec<SetChoiceInfo>,
    },
    /// An SBE composite after codegen.
    Composite {
        /// Full schema IR for advanced hooks.
        schema: &'a crate::Schema,
        /// Composite type name (PascalCase).
        name: String,
        /// Member fields in wire order.
        fields: Vec<FieldInfo>,
    },
    /// A message decoder flyweight.
    MessageDecoder {
        /// Full schema IR for advanced hooks.
        schema: &'a crate::Schema,
        /// Message name (PascalCase).
        name: String,
        /// SBE template id.
        template_id: u16,
        /// Compiled fixed block length in bytes.
        block_length: usize,
        /// Fixed/group/data fields visible on this message.
        fields: Vec<FieldInfo>,
    },
    /// A message encoder stage root.
    MessageEncoder {
        /// Full schema IR for advanced hooks.
        schema: &'a crate::Schema,
        /// Message name (PascalCase).
        name: String,
        /// SBE template id.
        template_id: u16,
        /// Compiled fixed block length in bytes.
        block_length: usize,
        /// Fixed/group/data fields visible on this message.
        fields: Vec<FieldInfo>,
    },
    /// An owned domain DTO (`*Domain`) when domain objects are enabled.
    DomainStruct {
        /// Full schema IR for advanced hooks.
        schema: &'a crate::Schema,
        /// DTO type name (PascalCase).
        name: String,
        /// Fields on the DTO (including groups/var-data as owned types).
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

/// Wrapper so hooks can live in [`GenerationConfig`]. Uses [`Arc`] so
/// `GenerationConfig` can derive [`Clone`].
#[derive(Clone, Default)]
pub(crate) struct Hooks(Vec<std::sync::Arc<HookFn>>);

impl std::fmt::Debug for Hooks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_tuple("Hooks").field(&self.0.len()).finish()
    }
}
impl Hooks {
    pub(crate) fn push(&mut self, hook: std::sync::Arc<HookFn>) {
        self.0.push(hook);
    }
    pub(crate) fn iter(&self) -> std::slice::Iter<'_, std::sync::Arc<HookFn>> {
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
///     .with_domain_objects(DomainVarData::Strings)
///     .with_domain_type(
///         ConversionSelector::named_type("Decimal"),
///         "rust_decimal::Decimal",
///     );
/// ```
#[derive(Clone)]
pub struct GenerationConfig {
    /// Rust module name for the generated output file (`{module_name}.rs`).
    pub(crate) module_name: String,
    /// Sibling module that already owns shared types (multi-schema mode).
    pub(crate) shared_module: Option<String>,
    /// Emit owned `*Domain` structs + `TryFrom<&Decoder>` / `encode`.
    pub(crate) domain_objects: bool,
    /// Var-data shape on DTOs when `domain_objects` is set.
    pub(crate) domain_var_data: DomainVarData,
    /// Selectors for generic `*_as` / `*_from` conversion methods.
    pub(crate) conversions: Vec<ConversionSelector>,
    /// Domain-type mappings: `(selector, rust_type_path)`.
    /// Implicitly enables conversion for the same selector.
    pub(crate) domain_types: Vec<(ConversionSelector, String)>,
    /// Selectors passed to [`Self::with_manual_domain_type`]:
    /// get the same concrete `try_*` signatures as any other `domain_types`
    /// entry, but the built-in `TryFromSbe`/`TryToSbe` impl (for `bool` /
    /// `rust_decimal::Decimal` / `chrono::DateTime<Utc>`) is not generated —
    /// the caller supplies it.
    pub(crate) manual_impl_selectors: Vec<ConversionSelector>,
    /// When set, emit `pub use <path> as sbe_rt;` instead of inlining runtime.
    pub(crate) external_sbe_rt_path: Option<String>,
    /// Emit `From<EncodeError/DecodeError>` for this error type path.
    pub(crate) error_from_path: Option<String>,
    /// Map enum/boolean NullVal → `Option<T>`. Matched selectors produce
    /// `Option<EventCode>` accessors instead of bare enum types.
    /// Wire is byte-identical: `None` writes the `NullVal` discriminant.
    pub(crate) null_as_option: Vec<ConversionSelector>,
    /// When true, every enum in the schema gets `Option<Enum>` accessors
    /// without needing individual `with_null_as_option` calls.
    pub(crate) all_enums_as_option: bool,
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
            .field("manual_impl_selectors", &self.manual_impl_selectors)
            .field("external_sbe_rt_path", &self.external_sbe_rt_path)
            .field("error_from_path", &self.error_from_path)
            .field("null_as_option", &self.null_as_option)
            .field("all_enums_as_option", &self.all_enums_as_option)
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
    /// Create a config for output module `{module_name}.rs` with
    /// [`GenerationProfile::Full`] defaults.
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
            manual_impl_selectors: Vec::new(),
            external_sbe_rt_path: None,
            error_from_path: None,
            keyword_append_token: "_".into(),
            deprecated_attrs: false,
            null_as_option: Vec::new(),
            all_enums_as_option: false,
            auto_bool_domain: false,
            enable_display_debug: true,
            enable_meta_attributes: true,
            enable_dispatch: true,
            hooks: Hooks::default(),
        }
    }

    /// Create a config with [`GenerationProfile::Lean`] defaults (no
    /// Display/Debug, no meta attributes, no dispatch, no domain objects).
    ///
    /// Equivalent to `GenerationConfig::new(name).profile(GenerationProfile::Lean)`
    /// but more direct. Explicit `with_*` settings (conversions, domain types,
    /// auto-bool) can be added after — they are not cleared.
    ///
    /// ```rust
    /// use ergo_sbe::GenerationConfig;
    /// let c = GenerationConfig::lean("minimal");
    /// ```
    #[must_use]
    pub fn lean(module_name: impl Into<String>) -> Self {
        Self::new(module_name).profile(GenerationProfile::Lean)
    }

    /// The module name for generated output.
    #[must_use]
    pub(crate) fn module_name(&self) -> &str {
        &self.module_name
    }

    /// Override the module name set in [`new`](Self::new). Use when cloning a base
    /// config across several schemas — set the placeholder in [`new`](Self::new), then
    /// call `.clone().with_module_name("orderbook")` on each.
    ///
    /// ```rust
    /// use ergo_sbe::GenerationConfig;
    /// let base = GenerationConfig::new("_");
    /// let a = base.clone().with_module_name("md");
    /// let b = base.clone().with_module_name("ob");
    /// // `a` generates `md.rs`, `b` generates `ob.rs`, `base` unchanged.
    /// ```
    #[must_use]
    pub fn with_module_name(mut self, name: impl Into<String>) -> Self {
        let name = name.into();
        debug_assert!(
            is_valid_module_ident(&name),
            "module name '{name}' contains path separators, '.', '..', or is empty"
        );
        self.module_name = name;
        self
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

    /// Map matching enum fields from `NullVal` → `Option<T>`.
    ///
    /// Wire encoding is byte-identical: `None` writes the `NullVal`
    /// discriminant, `Some(v)` writes `v`. Zero runtime cost.
    ///
    /// ```rust
    /// use ergo_sbe::{ConversionSelector, GenerationConfig};
    ///
    /// // EventCode fields → Option<EventCode>
    /// let config = GenerationConfig::new("msgs")
    ///     .with_null_as_option(ConversionSelector::named_type("EventCode"));
    /// ```
    #[must_use]
    pub fn with_null_as_option(mut self, selector: ConversionSelector) -> Self {
        if !self.null_as_option.contains(&selector) {
            self.null_as_option.push(selector);
        }
        self
    }

    /// Map **every** enum field in the schema to `Option<Enum>`.
    ///
    /// Shorthand for calling [`with_null_as_option`](Self::with_null_as_option)
    /// on every named type. Opt-out per-enum is not yet supported —
    /// the blanket flag always wins.
    ///
    /// ```rust
    /// use ergo_sbe::GenerationConfig;
    /// let config = GenerationConfig::new("msgs")
    ///     .with_all_enums_as_option();
    /// ```
    #[must_use]
    pub fn with_all_enums_as_option(mut self) -> Self {
        self.all_enums_as_option = true;
        self
    }

    /// Map matching fields to a **concrete** Rust type path.
    ///
    /// Implies [`Self::with_conversion`] for the same selector and emits
    /// well-known `TryFromSbe`/`TryToSbe` impls for `bool`,
    /// `rust_decimal::Decimal`, and `chrono::DateTime<Utc>` when those paths
    /// are used. For any other `rust_type`, no impl is auto-generated.
    ///
    /// To skip the built-in impl and supply your own, use
    /// [`Self::with_manual_domain_type`].
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
        self,
        selector: ConversionSelector,
        rust_type: impl Into<String>,
    ) -> Self {
        self.apply_domain_type(selector, rust_type, DomainImpl::Generated)
    }

    /// Like [`Self::with_domain_type`], but you write `impl TryFromSbe<Wire>`
    /// / `impl TryToSbe<Wire>` yourself — e.g. a custom rounding rule, or
    /// null/validation behaviour the built-in impl does not match.
    ///
    /// ergo-sbe still generates the concrete `try_price(...)?` / `try_price()?`
    /// signatures that call those impls. A missing impl fails closed with a
    /// named compile error, and for the three built-ins the generated
    /// accessor's rustdoc carries the exact impl `Generated` would have
    /// written, ready to copy and adjust.
    ///
    /// ```rust
    /// use ergo_sbe::{GenerationConfig, ConversionSelector};
    ///
    /// let config = GenerationConfig::new("msgs")
    ///     .with_manual_domain_type(
    ///         ConversionSelector::named_type("Decimal"),
    ///         "rust_decimal::Decimal",
    ///     );
    /// // Application code must provide:
    /// //   impl TryFromSbe<Decimal> for rust_decimal::Decimal { ... }
    /// //   impl TryToSbe<Decimal> for rust_decimal::Decimal { ... }
    /// ```
    #[must_use]
    pub fn with_manual_domain_type(
        self,
        selector: ConversionSelector,
        rust_type: impl Into<String>,
    ) -> Self {
        self.apply_domain_type(selector, rust_type, DomainImpl::Manual)
    }

    fn apply_domain_type(
        mut self,
        selector: ConversionSelector,
        rust_type: impl Into<String>,
        impl_kind: DomainImpl,
    ) -> Self {
        let sel = selector;
        let ty = rust_type.into();
        if !self.conversions.contains(&sel) {
            self.conversions.push(sel.clone());
        }
        match impl_kind {
            DomainImpl::Generated => self.manual_impl_selectors.retain(|s| s != &sel),
            DomainImpl::Manual => {
                if !self.manual_impl_selectors.contains(&sel) {
                    self.manual_impl_selectors.push(sel.clone());
                }
            }
        }
        // Last-write-wins: calling with_domain_type(sel, "B") after
        // with_domain_type(sel, "A") replaces the mapping.
        if let Some(existing) = self.domain_types.iter_mut().find(|(s, _)| s == &sel) {
            existing.1 = ty;
        } else {
            self.domain_types.push((sel, ty));
        }
        self
    }

    /// Emit `From<sbe_rt::EncodeError>` / `From<sbe_rt::DecodeError>` for your error type.
    ///
    /// In build.rs: `.with_error_from_impls("crate::AppError")`.
    /// Application code: `enc.group(...)?;` — `EncodeError` auto-converts via `From`.
    ///
    /// **Note:** The generated `From` impl uses `format!("sbe encode: {err}")` —
    /// stringifying the typed error through its `Display` form, then calling
    /// `YourType::from(String)`. This means (1) your error type must implement
    /// `From<String>`, and (2) field-level error details (e.g.
    /// `EncodeError::BufferTooShort { field, needed, available }`) are lost in
    /// the conversion. Implement `From<generated::sbe_rt::EncodeError>` and
    /// `From<generated::sbe_rt::DecodeError>` on your error type so those
    /// fields survive. Removal is scheduled for 1.0.
    #[must_use]
    #[deprecated(
        since = "0.1.20",
        note = "implement From<generated::sbe_rt::EncodeError> and From<generated::sbe_rt::DecodeError> on your error type so wire fields (needed/available) are preserved; this helper formats through String and will be removed in 1.0"
    )]
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
    /// | [`DomainVarData::Strings`] | `String` | `InvalidUtf8` error (strict) |
    ///
    /// ```rust
    /// use ergo_sbe::{DomainVarData, GenerationConfig};
    /// let text = GenerationConfig::new("msgs")
    ///     .with_domain_objects(DomainVarData::Strings);
    /// let bytes = GenerationConfig::new("msgs")
    ///     .with_domain_objects(DomainVarData::Bytes);
    /// let _ = (text, bytes);
    /// ```
    ///
    /// # Generated API
    ///
    /// `DomainVarData::Strings` → `manufacturer: String`.
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

    /// Apply a product profile that sets the size knobs together.
    ///
    /// | Profile | Display/Debug | Meta attrs | Dispatch | Domain objects |
    /// |---------|---------------|------------|----------|----------------|
    /// | [`GenerationProfile::Full`] | on | on | on | unchanged |
    /// | [`GenerationProfile::Lean`] | off | off | off | off |
    ///
    /// Explicit `with_*` settings (conversions, domain types, auto-bool) win
    /// regardless of order — `profile()` only sets the knobs it owns and
    /// never clears explicit configuration. Prefer
    /// [`lean`](Self::lean) for a clean Lean baseline.
    ///
    /// Chain further `with_*` calls after `profile` to override individual
    /// knobs. Example:
    ///
    /// ```rust
    /// use ergo_sbe::{GenerationConfig, GenerationProfile};
    /// let _ = GenerationConfig::new("feed").profile(GenerationProfile::Lean);
    /// ```
    #[must_use]
    pub fn profile(mut self, profile: GenerationProfile) -> Self {
        match profile {
            GenerationProfile::Full => {
                self.enable_display_debug = true;
                self.enable_meta_attributes = true;
                self.enable_dispatch = true;
            }
            GenerationProfile::Lean => {
                self.enable_display_debug = false;
                self.enable_meta_attributes = false;
                self.enable_dispatch = false;
                self.domain_objects = false;
                // ponytail: explicit conversion/domain-type settings survive
                // profile() — they represent deliberate schema choices with
                // higher precedence than a bulk surface preset.
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
        self.hooks.push(std::sync::Arc::new(hook));
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

/// Reject module names that are not valid for `mod name;` in generated Rust.
///
/// `syn::parse_str::<syn::Ident>` alone accepts keywords (they parse as idents
/// in isolation). We additionally reject strict keywords and reserved words
/// that require a raw identifier (`r#gen`) under current/edition-reserved
/// Rust editions — including 2024's `gen`.
pub(crate) fn is_valid_module_ident(name: &str) -> bool {
    if name.is_empty()
        || name.contains('/')
        || name.contains('\\')
        || name.contains('.')
        || name == ".."
    {
        return false;
    }
    if is_rust_keyword_or_reserved(name) {
        return false;
    }
    syn::parse_str::<syn::Ident>(name).is_ok()
}

/// Strict keywords + reserved identifiers that cannot appear as a plain
/// `mod name;` without `r#` escaping.
fn is_rust_keyword_or_reserved(name: &str) -> bool {
    // Keep in sync with the Rust reference keyword tables (strict + reserved
    // + edition-reserved). Weak keywords (`union`, `macro_rules`, …) remain
    // allowed as module names.
    matches!(
        name,
        // Strict
        "as" | "break" | "const" | "continue" | "crate" | "else" | "enum" | "extern"
            | "false" | "fn" | "for" | "if" | "impl" | "in" | "let" | "loop" | "match"
            | "mod" | "move" | "mut" | "pub" | "ref" | "return" | "self" | "Self"
            | "static" | "struct" | "super" | "trait" | "true" | "type" | "unsafe"
            | "use" | "where" | "while"
            // 2018+
            | "async" | "await" | "dyn"
            // Reserved
            | "abstract" | "become" | "box" | "do" | "final" | "macro" | "override"
            | "priv" | "typeof" | "unsized" | "virtual" | "yield"
            // Reserved / edition-reserved
            | "try" | "gen"
    )
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
    fn profile_lean_preserves_explicit_conversions_and_domain_types()
    -> Result<(), Box<dyn std::error::Error>> {
        let full = GenerationConfig::new("m").profile(GenerationProfile::Full);
        assert!(full.enable_display_debug);
        assert!(full.enable_meta_attributes);
        assert!(full.enable_dispatch);

        let lean = GenerationConfig::new("m")
            .with_domain_objects(DomainVarData::Bytes)
            .with_conversion(ConversionSelector::named_type("Decimal"))
            .with_domain_type(
                ConversionSelector::named_type("Decimal"),
                "rust_decimal::Decimal",
            )
            .profile(GenerationProfile::Lean);
        assert!(!lean.enable_display_debug);
        assert!(!lean.enable_meta_attributes);
        assert!(!lean.enable_dispatch);
        assert!(!lean.domain_objects);
        // Explicit conversions and domain types survive Lean — they're
        // deliberate schema choices with higher precedence than a bulk preset.
        assert!(
            lean.has_conversions(),
            "explicit conversions must survive Lean"
        );
        assert!(
            !lean.domain_types.is_empty(),
            "explicit domain types must survive Lean"
        );

        // Later overrides still win.
        let override_dispatch = GenerationConfig::new("m")
            .profile(GenerationProfile::Lean)
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
        assert!(config.manual_impl_selectors.is_empty());
        Ok(())
    }

    #[test]
    fn with_domain_type_two_argument_calls_agree() -> Result<(), Box<dyn std::error::Error>> {
        let sel = ConversionSelector::named_type("Decimal");
        let a =
            GenerationConfig::new("msgs").with_domain_type(sel.clone(), "rust_decimal::Decimal");
        let b = GenerationConfig::new("msgs").with_domain_type(sel, "rust_decimal::Decimal");
        assert_eq!(a.domain_types, b.domain_types);
        assert_eq!(a.manual_impl_selectors, b.manual_impl_selectors);
        Ok(())
    }

    #[test]
    fn with_manual_domain_type_is_additive() -> Result<(), Box<dyn std::error::Error>> {
        let sel = ConversionSelector::named_type("Decimal");
        let config = GenerationConfig::new("msgs")
            .with_manual_domain_type(sel.clone(), "rust_decimal::Decimal");
        assert_eq!(config.domain_types.len(), 1);
        assert_eq!(config.manual_impl_selectors, vec![sel]);
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
    fn module_ident_rejects_keywords_and_reserved() -> Result<(), Box<dyn std::error::Error>> {
        use super::is_valid_module_ident;
        assert!(is_valid_module_ident("messages"));
        assert!(is_valid_module_ident("common_types"));
        assert!(!is_valid_module_ident("gen")); // Rust 2024 reserved
        assert!(!is_valid_module_ident("mod"));
        assert!(!is_valid_module_ident("async"));
        assert!(!is_valid_module_ident("try"));
        assert!(!is_valid_module_ident(""));
        assert!(!is_valid_module_ident("a.b"));
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
        let text = GenerationConfig::new("m").with_domain_objects(DomainVarData::Strings);
        assert!(text.domain_objects_enabled());
        assert_eq!(text.domain_var_data, DomainVarData::Strings);
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
            .with_shared_module("shared")
            .with_keyword_append_token("x")
            .with_bool_domain_type(true)
            .with_deprecated_attrs(true);

        assert_eq!(config.shared_module.as_deref(), Some("shared"));
        assert_eq!(config.keyword_append_token, "x");
        assert!(config.auto_bool_domain);
        assert!(config.deprecated_attrs);
        Ok(())
    }
}

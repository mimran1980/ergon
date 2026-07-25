//! Code generation configuration.
//!
//! Options that shape the generated Rust output: module name,
//! domain objects, typed conversions, shared runtime, and more.
//!
//! Use builder methods to configure:
//!
//! ```rust
//! use ergo_sbe::{GenerationConfig, ConversionSelector};
//!
//! let config = GenerationConfig::new("market_data")
//!     .enable_domain_objects()
//!     .with_shared_module("common_types")
//!     .with_conversion(ConversionSelector::named_type("Decimal"));
//! ```

/// Selects which fields get generated `*_as`/`*_from` conversion methods.
///
/// Precedence when multiple selectors could match a field:
/// 1. Exact `"Message.field"` path
/// 2. SBE `semanticType`
/// 3. Named type (primitive alias, enum, set, composite, fixed array)
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum ConversionSelector {
    /// Match a specific message field by path: `"Car.serialNumber"`.
    FieldPath(String),
    /// Match all fields with a given SBE `semanticType`: `"UTCTimestamp"`.
    SemanticType(String),
    /// Match all fields of a named type: `"Decimal"`, `"Timestamp"`.
    NamedType(String),
}

impl ConversionSelector {
    /// Select by SBE `semanticType` attribute.
    #[must_use]
    pub fn semantic_type(name: impl Into<String>) -> Self {
        Self::SemanticType(name.into())
    }

    /// Select by named SBE type (composite, enum, set, alias).
    #[must_use]
    pub fn named_type(name: impl Into<String>) -> Self {
        Self::NamedType(name.into())
    }

    /// Select by exact `"Message.field"` path.
    #[must_use]
    pub fn field_path(path: impl Into<String>) -> Self {
        Self::FieldPath(path.into())
    }
}

/// Options that shape generated Rust.
///
/// Use builder methods to configure:
///
/// ```rust
/// use ergo_sbe::{GenerationConfig, ConversionSelector};
///
/// let config = GenerationConfig::new("market_data")
///     .enable_domain_objects()
///     .with_shared_module("common_types");
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationConfig {
    /// Rust module name for the generated output.
    pub(crate) module_name: String,
    /// Name of a sibling module that provides shared types (enums, sets, composites).
    pub(crate) shared_module: Option<String>,
    /// Generate owned domain structs alongside flyweight decoders.
    pub(crate) domain_objects: bool,
    /// Selectors for fields that get `*_as`/`*_from` conversion methods.
    pub(crate) conversions: Vec<ConversionSelector>,
    /// Domain-type mappings: `(selector, rust_type_path)`.
    /// Implicitly enables conversion methods for the same selector.
    pub(crate) domain_types: Vec<(ConversionSelector, String)>,
    /// When set, emit `pub use <path> as sbe_rt;` instead of inlining.
    pub(crate) external_sbe_rt_path: Option<String>,
    /// When set, emit `From<sbe_rt::EncodeError>` and `From<sbe_rt::DecodeError>`
    /// impls for the given error type path.
    pub(crate) error_from_path: Option<String>,
    /// Emit `_unchecked` companion methods for benchmarking.
    pub(crate) unchecked_companions: bool,
}

impl GenerationConfig {
    #[must_use]
    /// Create a new config with the given output module name.
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            shared_module: None,
            domain_objects: false,
            conversions: Vec::new(),
            domain_types: Vec::new(),
            external_sbe_rt_path: None,
            error_from_path: None,
            unchecked_companions: false,
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
        !self.conversions.is_empty() || !self.domain_types.is_empty()
    }

    /// The external sbe_rt path, if set.
    #[must_use]
    pub(crate) fn external_sbe_rt_path(&self) -> Option<&str> {
        self.external_sbe_rt_path.as_deref()
    }

    /// Share one `sbe_rt` module across separately generated schema files.
    ///
    /// `path` must be a valid Rust path usable in `pub use <path> as sbe_rt`.
    #[must_use]
    pub fn with_external_sbe_rt(mut self, path: impl Into<String>) -> Self {
        self.external_sbe_rt_path = Some(path.into());
        self
    }

    /// Enable generic `*_as` / `*_from` conversion methods for matching fields.
    ///
    /// Wire accessors stay primary (`price_value` / `price_wire`). Callers
    /// supply `TryFromSbe` / `TryToSbe` for their app type — the generator does
    /// not pull in rust_decimal. Prefer [`Self::with_domain_type`] when there
    /// is a single canonical Rust type (emits concrete methods + well-known
    /// impls).
    ///
    /// Duplicate selectors are ignored. Selectors matching no field are
    /// generation errors.
    #[must_use]
    pub fn with_conversion(mut self, selector: ConversionSelector) -> Self {
        if !self.conversions.contains(&selector) {
            self.conversions.push(selector);
        }
        self
    }

    /// Map matching fields to a concrete Rust domain type.
    ///
    /// Implicitly enables conversion for the same selector, **and** emits
    /// concrete methods named after the field (e.g. `price() -> Decimal`)
    /// plus well-known `TryFromSbe` impls for `bool`, `rust_decimal::Decimal`,
    /// and `chrono::DateTime<Utc>` when those paths are used. The `rust_type`
    /// must be a valid Rust type path (e.g. `"rust_decimal::Decimal"`).
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

    /// Emit `From<sbe_rt::EncodeError>` / `From<sbe_rt::DecodeError>` for
    /// the given error type, so callers can use `?` without `.map_err()`.
    #[must_use]
    pub fn enable_error_from_impls(mut self, path: impl Into<String>) -> Self {
        self.error_from_path = Some(path.into());
        self
    }

    /// Generate owned domain structs alongside flyweight decoders.
    #[must_use]
    pub fn enable_domain_objects(mut self) -> Self {
        self.domain_objects = true;
        self
    }

    #[must_use]
    /// Set the shared module name for multi-schema generation.
    pub fn with_shared_module(mut self, name: impl Into<String>) -> Self {
        self.shared_module = Some(name.into());
        self
    }

    /// Emit `_unchecked` companion methods for benchmarking.
    #[must_use]
    pub fn with_unchecked_companions(mut self) -> Self {
        self.unchecked_companions = true;
        self
    }
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self::new("messages")
    }
}

#[cfg(test)]
mod tests {
    use super::{ConversionSelector, GenerationConfig};

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
        Ok(())
    }
}

//! Code generation configuration.
//!
//! Options that shape the generated Rust output: module names,
//! wire-compatibility policy, bounds-check behaviour, and multi-schema
//! shared-type deduplication.
//!
//! The primary entry-point is [`GenerationConfig::new`] which
//! creates the standard configuration. Use direct field assignment
//! to customise:
//!
//! ```rust
//! use ergo_sbe::GenerationConfig;
//!
//! let mut config = GenerationConfig::new("market_data");
//! config.checked_accessors = false;
//! config.shared_module = Some("common_types".into());
//! ```

/// Controls how strictly generated codecs preserve official SBE wire behaviour.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityMode {
    /// Reject schema constructs that cannot be emitted with official SBE wire
    /// compatibility.
    Strict,
    /// Permit planned extensions only when they do not alter wire layout.
    WireCompatibleExtensions,
}

/// Options that shape generated Rust.
///
/// # Examples
///
/// Basic usage — equivalent to the default:
///
/// ```rust
/// use ergo_sbe::GenerationConfig;
/// let config = GenerationConfig::new("messages");
/// ```
///
/// Multi-schema setup with shared types:
///
/// ```rust
/// use ergo_sbe::GenerationConfig;
/// let mut config = GenerationConfig::new("common_types");
/// config.shared_module = Some("common_types".into());
/// assert!(config.shared_module.is_some());
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationConfig {
    /// Rust module name for the generated output.
    ///
    /// This becomes the file name (e.g. `"market_data"` produces
    /// `market_data.rs`). For single-schema generation this is the
    /// only module. For multi-schema this is one of several.
    pub module_name: String,
    /// Wire-compatibility policy.
    ///
    /// [`Strict`](CompatibilityMode::Strict) rejects any schema construct
    /// that would alter the official SBE wire layout.
    ///
    /// [`WireCompatibleExtensions`](CompatibilityMode::WireCompatibleExtensions)
    /// permits extensions that do not affect the wire format (e.g. richer
    /// Rust-side type annotations).
    pub compatibility: CompatibilityMode,
    /// Whether generated code should include bounds checks in public accessors.
    ///
    /// When `true` (default), all decoder accessors return `Result` and
    /// verify buffer length. When `false`, the generated code omits bounds
    /// checks for maximum throughput; callers must ensure the buffer is
    /// large enough.
    pub checked_accessors: bool,
    /// Name of a sibling module that provides shared types (enums, sets, composites).
    ///
    /// When set, [`generate_multi`](crate::Generator::generate_multi) treats the
    /// first schema as the shared source and subsequent schemas emit
    /// `pub use super::<shared_module>::*;` instead of regenerating those types.
    ///
    /// This avoids duplicate type definitions across schema modules and keeps
    /// the generated code DRY.
    pub shared_module: Option<String>,
    /// Generate owned domain structs alongside flyweight decoders.
    ///
    /// When `true`, each message gets a `MsgDomain` owned struct with
    /// `From<MsgDecoder>` for easy application-layer use (persist, serialize,
    /// cross-thread). Default: `false` (zero-cost for HFT-only users).
    pub domain_objects: bool,
    /// Composite type names registered for generic decimal conversion.
    ///
    /// Each entry names a composite whose first two members are
    /// `mantissa: int64` and `exponent: int8`. The generator emits a local
    /// `SbeDecimal` trait plus generic converter methods on fields backed
    /// by these composites. Insertion order is preserved; duplicates are
    /// ignored. Default: empty (no converter emission).
    pub decimal_composites: Vec<String>,
    /// When set, emit `pub use <path> as sbe_rt;` instead of inlining the
    /// full runtime module. Use with multi-schema crates so every generated
    /// module shares one `EncodeError` / `DecodeError` type (e.g.
    /// `"crate::sbe_common::sbe_rt"` after generating a shared module first).
    ///
    /// Default: `None` (inline `sbe_rt` in this module).
    pub external_sbe_rt_path: Option<String>,
    /// When set, emit `From<sbe_rt::EncodeError>` and `From<sbe_rt::DecodeError>`
    /// impls for the given error type path (e.g. `"crate::ClusterError"`).
    /// The target type must implement `From<String>`.
    ///
    /// Default: `None` (no `From` impls emitted).
    pub error_from_path: Option<String>,
    /// Emit `_unchecked` companion methods alongside checked constructors
    /// (`wrap_unchecked`, `wrap_and_apply_header_unchecked`, `read_bytes_unchecked`).
    /// For Criterion benchmarking — produces both checked and unchecked paths
    /// in one binary so within-session ratios are noise-free.
    ///
    /// Default: `false`.
    pub unchecked_companions: bool,
}

impl GenerationConfig {
    /// Create a configuration with strict wire compatibility and checked accessors.
    ///
    /// This is the recommended starting point.
    #[must_use]
    pub fn new(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            compatibility: CompatibilityMode::Strict,
            checked_accessors: true,
            shared_module: None,
            domain_objects: false,
            decimal_composites: Vec::new(),
            external_sbe_rt_path: None,
            error_from_path: None,
            unchecked_companions: false,
        }
    }

    /// Share one `sbe_rt` module across separately generated schema files.
    ///
    /// `path` must be a valid Rust path usable in `pub use <path> as sbe_rt`.
    #[must_use]
    pub fn with_external_sbe_rt(mut self, path: impl Into<String>) -> Self {
        self.external_sbe_rt_path = Some(path.into());
        self
    }

    /// Register a composite for generic decimal conversion.
    ///
    /// The composite must have `mantissa: int64` followed by `exponent: int8`.
    /// The generator emits a local `SbeDecimal` trait and generic converter
    /// methods. Insertion order is preserved; duplicate names are ignored.
    #[must_use]
    pub fn enable_decimal_converters(mut self, composite: impl Into<String>) -> Self {
        let name = composite.into();
        if !self.decimal_composites.contains(&name) {
            self.decimal_composites.push(name);
        }
        self
    }

    /// Emit `From<sbe_rt::EncodeError>` / `From<sbe_rt::DecodeError>` for
    /// the given error type, so callers can use `?` without `.map_err()`.
    ///
    /// `path` must resolve in the generated module, e.g.
    /// `"crate::ClusterError"` or `"super::MyError"`. The target type must
    /// implement `From<String>`.
    #[must_use]
    pub fn enable_error_from_impls(mut self, path: impl Into<String>) -> Self {
        self.error_from_path = Some(path.into());
        self
    }

    /// Emit `_unchecked` companion methods for benchmarking.
    /// Produces `wrap_unchecked`, `wrap_and_apply_header_unchecked`,
    /// and `read_bytes_unchecked` alongside the checked originals.
    #[must_use]
    pub fn with_unchecked_companions(mut self) -> Self {
        self.unchecked_companions = true;
        self
    }
}

impl Default for GenerationConfig {
    /// Returns the default configuration: strict wire compat, checked
    /// accessors, single-schema mode with module name `"messages"`.
    fn default() -> Self {
        Self::new("messages")
    }
}

#[cfg(test)]
mod tests {
    use super::{CompatibilityMode, GenerationConfig};

    #[test]
    fn default_config_is_strict_and_checked() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::default();

        assert_eq!(config.compatibility, CompatibilityMode::Strict);
        assert!(config.checked_accessors);

        Ok(())
    }

    #[test]
    fn enable_decimal_converters_adds_name() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("test").enable_decimal_converters("Decimal");
        assert_eq!(config.decimal_composites, vec!["Decimal"]);

        Ok(())
    }

    #[test]
    fn enable_decimal_converters_dedup() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("test")
            .enable_decimal_converters("Decimal")
            .enable_decimal_converters("Decimal");
        assert_eq!(config.decimal_composites.len(), 1);

        Ok(())
    }

    #[test]
    fn with_external_sbe_rt_sets_path() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("m").with_external_sbe_rt("crate::rt::sbe_rt");
        assert_eq!(
            config.external_sbe_rt_path.as_deref(),
            Some("crate::rt::sbe_rt")
        );

        Ok(())
    }

    #[test]
    fn enable_decimal_converters_multiple() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("test")
            .enable_decimal_converters("Decimal")
            .enable_decimal_converters("Price");
        assert_eq!(config.decimal_composites, vec!["Decimal", "Price"]);

        Ok(())
    }

    #[test]
    fn new_config_has_empty_decimal_composites() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("test");
        assert!(config.decimal_composites.is_empty());

        Ok(())
    }

    #[test]
    fn new_config_has_correct_defaults() -> Result<(), Box<dyn std::error::Error>> {
        let config = GenerationConfig::new("mymod");
        assert_eq!(config.module_name, "mymod");
        assert_eq!(config.compatibility, CompatibilityMode::Strict);
        assert!(config.checked_accessors);
        assert!(config.shared_module.is_none());
        assert!(!config.domain_objects);

        Ok(())
    }
}

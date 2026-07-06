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
//! use ergosbe::GenerationConfig;
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
/// use ergosbe::GenerationConfig;
/// let config = GenerationConfig::new("messages");
/// ```
///
/// Multi-schema setup with shared types:
///
/// ```rust
/// use ergosbe::GenerationConfig;
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
        }
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
    fn default_config_is_strict_and_checked() {
        let config = GenerationConfig::default();

        assert_eq!(config.compatibility, CompatibilityMode::Strict);
        assert!(config.checked_accessors);
    }
}

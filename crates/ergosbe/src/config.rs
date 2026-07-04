//! Code generation configuration.

/// Controls how strictly generated codecs preserve official SBE wire behavior.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompatibilityMode {
    /// Reject schema constructs that cannot be emitted with official SBE wire
    /// compatibility.
    Strict,
    /// Permit planned extensions only when they do not alter wire layout.
    WireCompatibleExtensions,
}

/// Controls whether generated Rust types borrow from the message buffer or own data.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnershipMode {
    /// Prefer zero-copy borrowed views over allocations.
    Borrowed,
    /// Generate owned structures for easier application-level movement.
    Owned,
}

/// Options that shape generated Rust.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GenerationConfig {
    /// Rust module name for the generated output.
    pub module_name: String,
    /// Wire-compatibility policy.
    pub compatibility: CompatibilityMode,
    /// Ownership model for generated accessors and data structures.
    pub ownership: OwnershipMode,
    /// Whether generated code should include bounds checks in public accessors.
    pub checked_accessors: bool,
}

impl GenerationConfig {
    /// Create a configuration for latency-sensitive wire-compatible code.
    #[must_use]
    pub fn low_latency(module_name: impl Into<String>) -> Self {
        Self {
            module_name: module_name.into(),
            compatibility: CompatibilityMode::Strict,
            ownership: OwnershipMode::Borrowed,
            checked_accessors: true,
        }
    }
}

impl Default for GenerationConfig {
    fn default() -> Self {
        Self::low_latency("messages")
    }
}

#[cfg(test)]
mod tests {
    use super::{CompatibilityMode, GenerationConfig, OwnershipMode};

    #[test]
    fn default_config_is_strict_borrowed_and_checked() {
        let config = GenerationConfig::default();

        assert_eq!(config.compatibility, CompatibilityMode::Strict);
        assert_eq!(config.ownership, OwnershipMode::Borrowed);
        assert!(config.checked_accessors);
    }
}

//! Rust code generation boundary.

use crate::{GenerationConfig, Schema};

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

    /// Generate Rust modules for a normalized schema.
    #[must_use]
    pub fn generate(&self, schema: &Schema) -> GeneratedModuleSet {
        let mut modules = GeneratedModuleSet::default();
        modules.push(GeneratedModule {
            path: format!("{}.rs", self.config.module_name),
            source: format!(
                "//! Generated from SBE schema package `{}` id {} version {}.\n",
                schema.package, schema.id, schema.version
            ),
        });
        modules
    }
}

#[cfg(test)]
mod tests {
    use crate::{GenerationConfig, Schema};

    use super::Generator;

    #[test]
    fn generator_emits_deterministic_module_name() {
        let generator = Generator::new(GenerationConfig::low_latency("market_data"));
        let schema = Schema::new("fix.sbe", 1, 0);

        let modules = generator.generate(&schema);
        let collected = modules.modules().collect::<Vec<_>>();

        assert_eq!(collected.len(), 1);
        assert_eq!(collected[0].path, "market_data.rs");
        assert!(collected[0].source.contains("fix.sbe"));
    }
}

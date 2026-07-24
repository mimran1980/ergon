//! Encoded-length classification and code generation.
//!
//! Three strategies:
//! - `Fixed`: no groups, no varData → use existing encoder constants.
//! - `Direct`: flat groups + message varData → checked const-fn helpers.
//! - `Staged`: nested groups or entry varData → staged builder types.

use crate::structured_ir::MessageStructure;

/// How the encoded length of a message should be computed.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum LengthStrategy {
    /// Fixed-only message — use `BLOCK_LENGTH` / `ENCODED_LENGTH`.
    Fixed,
    /// Directly computable — flat groups + message varData only.
    Direct,
    /// Needs a staged length builder — nested groups or entry varData.
    Staged,
}

/// Classify a message into one of the three length strategies.
pub(super) fn strategy(message: &MessageStructure) -> LengthStrategy {
    if message.groups.is_empty() && message.var_data.is_empty() {
        return LengthStrategy::Fixed;
    }

    let has_dynamic_entry = message
        .groups
        .iter()
        .any(|group| !group.groups.is_empty() || !group.var_data.is_empty());

    if has_dynamic_entry {
        LengthStrategy::Staged
    } else {
        LengthStrategy::Direct
    }
}

#[cfg(test)]
mod tests {
    use super::{strategy, LengthStrategy};
    use crate::structured_ir::{parse_message_structure, partition_tokens};
    use std::path::PathBuf;

    fn fixture(name: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("fixtures")
            .join("schemas")
            .join(name)
    }

    fn strategy_for(
        path: &std::path::Path,
        message_name: &str,
    ) -> Result<LengthStrategy, Box<dyn std::error::Error>> {
        let ir = crate::parse_file(path)?;
        let elements = partition_tokens(&ir.tokens);
        let message_tokens = elements
            .messages
            .iter()
            .find(|tokens| tokens[0].name == message_name)
            .ok_or_else(|| format!("missing message {message_name}"))?;
        let message = parse_message_structure(message_tokens, &elements);
        Ok(strategy(&message))
    }

    #[test]
    fn classifies_repository_message_shapes() -> Result<(), Box<dyn std::error::Error>> {
        assert_eq!(
            strategy_for(&fixture("basic-schema.xml"), "TestMessage50001")?,
            LengthStrategy::Fixed,
        );
        assert_eq!(
            strategy_for(&fixture("basic-variable-length-schema.xml"), "TestMessage1")?,
            LengthStrategy::Direct,
        );
        assert_eq!(
            strategy_for(&fixture("basic-group-schema.xml"), "TestMessage1")?,
            LengthStrategy::Direct,
        );
        assert_eq!(
            strategy_for(&fixture("group-with-data-schema.xml"), "TestMessage1")?,
            LengthStrategy::Staged,
        );
        assert_eq!(
            strategy_for(&fixture("nested-group-schema.xml"), "Top")?,
            LengthStrategy::Staged,
        );
        assert_eq!(
            strategy_for(&fixture("l3-orderbook-schema.xml"), "L3Book")?,
            LengthStrategy::Staged,
        );
        Ok(())
    }
}

//! Source-name tracking and de-duplicated parse warnings.
//!
//! The source name (file name, or `"<xml>"` for in-memory input) lives in
//! [`WarnState`]. Thread one `WarnState` through a parse and every warning
//! names the right file.

use std::collections::HashSet;

/// Warning de-duplication for one parse. Each parse owns its state, so
/// concurrent parses never suppress each other's warnings. A `RefCell` is
/// enough because the parser is single-threaded recursive descent.
pub(crate) struct WarnState {
    seen: std::cell::RefCell<HashSet<String>>,
    /// The name warnings and errors are reported against
    /// (e.g. `"orderbook-schema.xml"` or `"<xml>"`).
    pub(crate) name: String,
}

impl WarnState {
    pub(crate) fn new(name: String) -> Self {
        Self {
            seen: std::cell::RefCell::new(HashSet::new()),
            name,
        }
    }
}

/// Print a parser warning once per parse. `xi:include` inlines a shared
/// schema (such as `common-types.xml`) into every consuming file, so a plain
/// `eprintln!` would repeat one warning for each of N schemas that include
/// the same type. The key is byte offset plus message, so distinct warnings
/// are never suppressed.
///
/// When `node` is provided the warning includes the source file, line,
/// column, and the relevant XML line.
pub(crate) fn warn_once(message: &str, node: Option<roxmltree::Node<'_, '_>>, state: &WarnState) {
    let dedup_key = if let Some(n) = node {
        format!("{}:{}", n.range().start, message)
    } else {
        message.to_string()
    };
    if state.seen.borrow_mut().insert(dedup_key) {
        if let Some(n) = node {
            let pos = n.range().start;
            let text = n.document().input_text();
            let line = text[..pos].matches('\n').count() + 1;
            let last_nl = text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let col = pos - last_nl + 1;
            let line_end = text[pos..]
                .find('\n')
                .map(|i| pos + i)
                .unwrap_or(text.len());
            let snippet = text[last_nl..line_end].trim();
            eprintln!(
                "{}:{}:{}: {message}\n  |\n  | {snippet}\n  |",
                state.name, line, col,
            );
        } else {
            eprintln!("warning: {message}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each `WarnState` carries its own name.
    #[test]
    fn warn_state_remembers_the_file_it_was_built_with() -> Result<(), Box<dyn std::error::Error>> {
        let state = WarnState::new("orderbook-schema.xml".into());
        assert_eq!(
            state.name, "orderbook-schema.xml",
            "warnings must name the file that was actually parsed"
        );

        let state2 = WarnState::new("market-data.xml".into());
        assert_eq!(
            state2.name, "market-data.xml",
            "separate WarnStates carry separate names"
        );
        assert_eq!(
            state.name, "orderbook-schema.xml",
            "first WarnState is unchanged by the second"
        );
        Ok(())
    }
}

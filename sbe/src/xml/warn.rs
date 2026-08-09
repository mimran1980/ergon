//! Source-name tracking and de-duplicated parse warnings.
//!
//! The source name (filename or `"<xml>"` for in-memory input) lives in
//! [`WarnState`] — no global static. Thread a single `WarnState` through the
//! parse and every warning reports the correct file.

use std::collections::HashSet;

/// Per-invocation warning dedup — no global static, so concurrent parses
/// never suppress each other's warnings. Wrapped in `RefCell` because the
/// recursive-descent parser doesn't cross any await point.
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

/// De-duplicates parser warnings within a single parse call. `xi:include`
/// inlines a shared schema (e.g. `common-types.xml`) into every consuming
/// file, so a naive `eprintln!` fires once per consumer — N sibling schema
/// files sharing one included type multiply the same warning N times.
/// Each parse invocation creates its own [`WarnState`], so separate parse
/// calls do not suppress each other even when concurrent. Keyed on byte
/// offset + message, so distinct warnings are never suppressed within a
/// parse.
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

    /// Each `WarnState` carries its own name — no global static.
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
            "separate WarnStates carry separate names — no global leak"
        );
        assert_eq!(
            state.name, "orderbook-schema.xml",
            "first WarnState is unchanged by the second"
        );
        Ok(())
    }
}

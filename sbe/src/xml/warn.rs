//! Source-name tracking and de-duplicated parse warnings.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock, PoisonError};

/// The name warnings are reported against.
///
/// Declared once at module scope. A `static` inside a function body is local to
/// *that* body, so a setter and a getter that each declared their own would
/// touch two unrelated cells: the setter would appear to work and every warning
/// would still report the placeholder.
static SOURCE: OnceLock<Mutex<String>> = OnceLock::new();

/// Placeholder used until a real source name is known (in-memory XML input).
const UNNAMED_SOURCE: &str = "<xml>";

fn source_cell() -> &'static Mutex<String> {
    SOURCE.get_or_init(|| Mutex::new(String::from(UNNAMED_SOURCE)))
}

/// Tracks the source name so warnings can reference the real file
/// instead of a hardcoded `"schema.xml"`.
pub(crate) fn set_source_name(name: String) {
    *source_cell().lock().unwrap_or_else(PoisonError::into_inner) = name;
}

pub(crate) fn source_name() -> String {
    source_cell()
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Per-invocation warning dedup — no global static, so concurrent parses
/// never suppress each other's warnings. Wrapped in `RefCell` because the
/// recursive-descent parser doesn't cross any await point.
pub(crate) struct WarnState {
    seen: std::cell::RefCell<HashSet<String>>,
}

impl WarnState {
    pub(crate) fn new() -> Self {
        Self {
            seen: std::cell::RefCell::new(HashSet::new()),
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
                source_name(),
                line,
                col,
            );
        } else {
            eprintln!("warning: {message}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The setter and the getter must reach the same cell.
    ///
    /// They once declared a `static` each, inside their own function bodies —
    /// two distinct cells. The setter wrote one, the getter read the other, so
    /// every parse warning reported the placeholder no matter which file was
    /// being parsed. Nothing failed; the diagnostics were just quietly wrong.
    #[test]
    fn set_source_name_is_what_warnings_report() -> Result<(), Box<dyn std::error::Error>> {
        let previous = source_name();

        set_source_name("orderbook-schema.xml".into());
        assert_eq!(
            source_name(),
            "orderbook-schema.xml",
            "warnings must name the file that was actually parsed"
        );
        assert_ne!(
            source_name(),
            UNNAMED_SOURCE,
            "the placeholder means the setter never reached the cell the getter reads"
        );

        set_source_name(previous);
        Ok(())
    }
}

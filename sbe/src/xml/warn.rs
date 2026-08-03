//! Source-name tracking and de-duplicated parse warnings.

use std::collections::HashSet;

/// Tracks the source name so warnings can reference the real file
/// instead of a hardcoded `"schema.xml"`.
pub(crate) fn set_source_name(name: String) {
    use std::sync::Mutex;
    use std::sync::OnceLock;
    static SOURCE: OnceLock<Mutex<String>> = OnceLock::new();
    *SOURCE
        .get_or_init(|| Mutex::new(String::new()))
        .lock()
        .unwrap() = name;
}

pub(crate) fn source_name() -> String {
    use std::sync::Mutex;
    use std::sync::OnceLock;
    static SOURCE: OnceLock<Mutex<String>> = OnceLock::new();
    SOURCE
        .get_or_init(|| Mutex::new(String::from("<xml>")))
        .lock()
        .unwrap()
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

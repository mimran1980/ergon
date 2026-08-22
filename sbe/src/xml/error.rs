//! Parse errors and internal fault staging.

use std::ops::Range;
use std::path::PathBuf;

use roxmltree::Node;

/// Why an `xi:include` failed.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IncludeCause {
    /// The include graph contains a cycle.
    #[error("cyclic include: {}", cycle_display(chain))]
    Cycle {
        /// Canonical paths in visit order, ending at the repeated file.
        chain: Vec<PathBuf>,
    },
    /// A candidate path existed but could not be read.
    #[error("cannot read {}: {source}", path.display())]
    Io {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// No candidate path existed.
    #[error("include file not found")]
    NotFound,
}

fn cycle_display(chain: &[PathBuf]) -> String {
    chain
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(" -> ")
}

/// Errors raised while parsing an SBE schema. Carries a [`miette`] source span
/// so the offending XML element is highlighted in the rendered diagnostic.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
#[non_exhaustive]
pub enum ParseError {
    /// The XML document itself was malformed.
    #[error("malformed XML: {message}")]
    #[diagnostic(code(ergo_sbe::schema_parse::malformed_xml))]
    MalformedXml {
        /// What went wrong.
        message: String,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The offending location, when available.
        #[label("here")]
        span: Option<miette::SourceSpan>,
    },
    /// A required attribute or element was missing.
    #[error("missing {what}")]
    #[diagnostic(code(ergo_sbe::schema_parse::missing))]
    Missing {
        /// What was missing (element/attribute context).
        what: String,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The offending location, when a node was available.
        #[label("missing here")]
        span: Option<miette::SourceSpan>,
    },
    /// An attribute value was invalid.
    #[error("invalid {what}: {value}")]
    #[diagnostic(code(ergo_sbe::schema_parse::invalid))]
    Invalid {
        /// What was invalid.
        what: String,
        /// The offending value.
        value: String,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The offending location.
        #[label("invalid here")]
        span: Option<miette::SourceSpan>,
    },
    /// A schema resolution or validation error occurred.
    #[error("resolution error: {error}")]
    #[diagnostic(code(ergo_sbe::schema_parse::resolve))]
    Resolve {
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The primary offending location.
        #[label("here")]
        span: Option<miette::SourceSpan>,
        /// Secondary label (e.g. for duplicate definitions).
        #[label("related")]
        second_label: Option<miette::SourceSpan>,
        /// The underlying resolution error.
        #[source]
        error: Box<crate::resolve::ResolveError>,
    },
    /// Root schema file could not be read.
    #[error("cannot read {}: {source}", path.display())]
    #[diagnostic(code(ergo_sbe::schema_parse::io))]
    Io {
        /// Path that failed to read.
        path: PathBuf,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// An include (`xi:include`) resolution error occurred.
    #[error("include error for '{href}': {cause}")]
    #[diagnostic(code(ergo_sbe::schema_parse::include))]
    Include {
        /// The `href` attribute from the include element.
        href: String,
        /// Candidate paths that were tried, in order.
        attempted: Vec<PathBuf>,
        /// Machine-readable failure kind.
        #[source]
        cause: IncludeCause,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The offending include element, when available.
        #[label("include error here")]
        span: Option<miette::SourceSpan>,
    },
}

impl ParseError {
    pub(crate) fn malformed_xml(name: &str, message: impl Into<String>, xml: &str) -> Self {
        Self::MalformedXml {
            message: message.into(),
            source_code: named_source(name, xml),
            span: None,
        }
    }

    pub(crate) fn io(path: impl Into<PathBuf>, source: std::io::Error) -> Self {
        Self::Io {
            path: path.into(),
            source,
        }
    }

    /// Lift an internal [`Fault`] into a span-bearing [`ParseError`], attaching
    /// the parsed source so `miette` can render the highlight.
    pub(crate) fn from_fault(name: &str, fault: Fault, input: &str) -> Self {
        let source_code = named_source(name, input);
        let span = fault.span.map(miette::SourceSpan::from);
        match fault.kind {
            FaultKind::Missing { what } => Self::Missing {
                what,
                source_code,
                span,
            },
            FaultKind::Invalid { what, value } => Self::Invalid {
                what,
                value,
                source_code,
                span,
            },
            FaultKind::Include {
                href,
                attempted,
                cause,
            } => Self::Include {
                href,
                attempted,
                cause,
                source_code,
                span,
            },
        }
    }
}

impl From<crate::resolve::ResolveError> for ParseError {
    fn from(mut e: crate::resolve::ResolveError) -> Self {
        let source_code = e
            .take_source_code()
            .unwrap_or_else(|| miette::NamedSource::new("schema.xml", String::new()));
        let (span, second_label) = e.take_spans();
        Self::Resolve {
            source_code,
            span,
            second_label,
            error: Box::new(e),
        }
    }
}

/// Build a [`miette::NamedSource`] with the real source name, not the
/// old hardcoded `"schema.xml"`. Callers thread the name from [`WarnState`]
/// (set by the entry point — file path or `"<xml>"`).
pub(crate) fn named_source(name: &str, xml: &str) -> miette::NamedSource<String> {
    miette::NamedSource::new(name, xml.to_owned())
}

/// Internal, source-free error — converted to [`ParseError`] at the boundary,
/// where the parsed source text is known. Keeps the recursive helpers cheap and
/// free of source-cloning on the success path.
#[derive(Debug)]
pub(crate) struct Fault {
    pub(crate) kind: FaultKind,
    pub(crate) span: Option<Range<usize>>,
}

#[derive(Debug)]
pub(crate) enum FaultKind {
    Missing {
        what: String,
    },
    Invalid {
        what: String,
        value: String,
    },
    Include {
        href: String,
        attempted: Vec<PathBuf>,
        cause: IncludeCause,
    },
}

impl Fault {
    pub(crate) fn missing(node: Node<'_, '_>, what: impl Into<String>) -> Self {
        Self {
            kind: FaultKind::Missing { what: what.into() },
            span: Some(node.range()),
        }
    }
    pub(crate) fn missing_no_node(what: impl Into<String>) -> Self {
        Self {
            kind: FaultKind::Missing { what: what.into() },
            span: None,
        }
    }

    pub(crate) fn invalid(
        node: Node<'_, '_>,
        what: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            kind: FaultKind::Invalid {
                what: what.into(),
                value: value.into(),
            },
            span: Some(node.range()),
        }
    }

    pub(crate) fn include(
        href: impl Into<String>,
        attempted: Vec<PathBuf>,
        cause: IncludeCause,
    ) -> Self {
        Self {
            kind: FaultKind::Include {
                href: href.into(),
                attempted,
                cause,
            },
            span: None,
        }
    }
}

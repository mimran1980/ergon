//! Parse errors and internal fault staging.

use std::ops::Range;

use roxmltree::Node;

/// Errors raised while parsing an SBE schema. Carries a [`miette`] source span
/// so the offending XML element is highlighted in the rendered diagnostic.
#[derive(Debug, thiserror::Error, miette::Diagnostic)]
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
    /// An include (xi:include) resolution error occurred.
    #[error("include error: {message}")]
    #[diagnostic(code(ergo_sbe::schema_parse::include))]
    IncludeError {
        /// What went wrong.
        message: String,
        /// The source document, for span rendering.
        #[source_code]
        source_code: miette::NamedSource<String>,
        /// The offending location.
        #[label("include error here")]
        span: Option<miette::SourceSpan>,
    },
}

impl ParseError {
    pub(crate) fn malformed_xml(message: impl Into<String>, xml: &str) -> Self {
        Self::MalformedXml {
            message: message.into(),
            source_code: named_source(xml),
            span: None,
        }
    }

    /// Lift an internal [`Fault`] into a span-bearing [`ParseError`], attaching
    /// the parsed source so `miette` can render the highlight.
    pub(crate) fn from_fault(fault: Fault, input: &str) -> Self {
        let source_code = named_source(input);
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
            FaultKind::IncludeError { message } => Self::IncludeError {
                message,
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

pub(crate) fn named_source(xml: &str) -> miette::NamedSource<String> {
    miette::NamedSource::new("schema.xml", xml.to_owned())
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
    Missing { what: String },
    Invalid { what: String, value: String },
    IncludeError { message: String },
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

    pub(crate) fn include_error(message: impl Into<String>) -> Self {
        Self {
            kind: FaultKind::IncludeError {
                message: message.into(),
            },
            span: None,
        }
    }
}

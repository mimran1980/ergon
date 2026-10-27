//! The parse front door: [`parse`] and its input, [`SchemaSource`].

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use roxmltree::{Document, Node};

use crate::ir::{Ir, Signal};

use super::error::{Fault, ParseError, named_source};
use super::registry::TypeRegistry;
use super::schema::{IncludeWalk, parse_schema};
use super::warn::WarnState;

/// Where a schema comes from, plus the two parse options.
///
/// | Constructor / option | Effect |
/// |---|---|
/// | [`SchemaSource::file`] (or `From<&Path>` / `From<PathBuf>`) | Read a file; `xi:include` resolves relative to it; [`Ir::sources`] lists the file and every include |
/// | [`SchemaSource::xml`] | Inline XML text (e.g. `include_str!`); [`Ir::sources`] lists no file of its own |
/// | [`with_shared`](Self::with_shared) | Resolve type references against an already-parsed schema, and copy its types (not its messages) into the result |
/// | [`with_xsd_validation`](Self::with_xsd_validation) | Run [`crate::validate_against_sbe_xsd`] on the root document first |
///
/// A bare `&str` is deliberately not accepted: it could mean a path or XML
/// text. Say which with [`SchemaSource::file`] or [`SchemaSource::xml`].
///
/// ```compile_fail
/// // Does not compile: `&str` is not a `SchemaSource`.
/// let _ = ergo_sbe::parse("schemas/messages.xml");
/// ```
#[derive(Clone, Debug)]
pub struct SchemaSource<'a> {
    input: Input<'a>,
    shared: Option<&'a Ir>,
    xsd: bool,
}

#[derive(Clone, Debug)]
enum Input<'a> {
    File(PathBuf),
    Xml(&'a str),
}

impl<'a> SchemaSource<'a> {
    /// A schema file on disk.
    #[must_use]
    pub fn file(path: impl AsRef<Path>) -> Self {
        Self::from_input(Input::File(path.as_ref().to_path_buf()))
    }

    /// Inline schema XML text.
    #[must_use]
    pub const fn xml(xml: &'a str) -> Self {
        Self::from_input(Input::Xml(xml))
    }

    const fn from_input(input: Input<'a>) -> Self {
        Self {
            input,
            shared: None,
            xsd: false,
        }
    }

    /// Resolve type references against `shared` first, so this schema need
    /// not `<include>` or redeclare them.
    ///
    /// The parsed [`Ir`] then also carries `shared`'s composites, enums and
    /// sets (not its messages) and its [`Ir::sources`], so it generates on its
    /// own or as a consumer in [`crate::Generator::generate_multi`].
    ///
    /// Bare top-level `<type>` typedefs are inlined and dropped during
    /// parsing, so share them through a `<composite>`/`<enum>`/`<set>`.
    #[must_use]
    pub const fn with_shared(mut self, shared: &'a Ir) -> Self {
        self.shared = Some(shared);
        self
    }

    /// Check the root document against the embedded SBE XSD before parsing.
    ///
    /// A shape check for schema authors' CI, not a full W3C XSD engine (see
    /// [`crate::xsd`]). Included files are not XSD-checked. [`parse`] alone
    /// already rejects malformed XML, a bad root, unexpected elements and
    /// unknown attributes.
    #[must_use]
    pub const fn with_xsd_validation(mut self) -> Self {
        self.xsd = true;
        self
    }
}

impl From<&Path> for SchemaSource<'_> {
    fn from(path: &Path) -> Self {
        Self::file(path)
    }
}

impl From<&PathBuf> for SchemaSource<'_> {
    fn from(path: &PathBuf) -> Self {
        Self::file(path)
    }
}

impl From<PathBuf> for SchemaSource<'_> {
    fn from(path: PathBuf) -> Self {
        Self::from_input(Input::File(path))
    }
}

/// Parse an SBE schema into a resolved token [`Ir`], ready for
/// [`crate::Schema::from_ir`].
///
/// ```rust
/// use std::path::Path;
/// use ergo_sbe::{SchemaSource, parse};
///
/// # fn main() -> Result<(), ergo_sbe::ParseError> {
/// let xml = r#"<messageSchema package="t" id="1" version="0" byteOrder="littleEndian">
///   <types><composite name="messageHeader">
///     <type name="blockLength" primitiveType="uint16"/>
///     <type name="templateId" primitiveType="uint16"/>
///     <type name="schemaId" primitiveType="uint16"/>
///     <type name="version" primitiveType="uint16"/>
///   </composite></types>
/// </messageSchema>"#;
/// let ir = parse(SchemaSource::xml(xml))?;
/// assert_eq!(ir.id, 1);
/// assert!(ir.sources.is_empty());
/// assert!(parse(Path::new("missing.xml")).is_err());
/// # Ok(())
/// # }
/// ```
///
/// # Errors
///
/// [`ParseError`] with source spans when the file cannot be read, the XML is
/// malformed, the XSD shape check fails (when requested), or the schema is
/// structurally invalid.
#[allow(clippy::result_large_err)]
pub fn parse<'a>(source: impl Into<SchemaSource<'a>>) -> Result<Ir, ParseError> {
    let source = source.into();
    let registry = source
        .shared
        .map_or_else(TypeRegistry::new, TypeRegistry::from_parsed_schema);
    let mut ir = match &source.input {
        Input::Xml(xml) => {
            if source.xsd {
                xsd_check("<xml>", xml)?;
            }
            let warn_state = WarnState::new("<xml>".into());
            parse_with_context(
                xml,
                None,
                &mut IncludeWalk::new(),
                registry,
                &warn_state,
                &mut Vec::new(),
            )?
        }
        Input::File(path) => parse_path(path, registry, source.xsd)?,
    };
    if let Some(shared) = source.shared {
        merge_shared(&mut ir, shared);
    }
    Ok(ir)
}

#[allow(clippy::result_large_err)]
fn xsd_check(name: &str, xml: &str) -> Result<(), ParseError> {
    match crate::xsd::validate_against_sbe_xsd(xml) {
        Ok(()) => Ok(()),
        Err(e @ crate::xsd::XsdValidationError::MalformedXml(_)) => {
            Err(ParseError::malformed_xml(name, e.to_string(), xml))
        }
        Err(e) => Err(ParseError::Invalid {
            what: "SBE schema".into(),
            value: e.to_string(),
            source_code: named_source(name, xml),
            span: None,
        }),
    }
}

/// Prepend `shared`'s type tokens (everything outside its messages) and
/// append its source files, so the consumer IR is self-contained.
fn merge_shared(consumer: &mut Ir, shared: &Ir) {
    let mut tokens = Vec::with_capacity(shared.tokens.len() + consumer.tokens.len());
    let mut in_message = false;
    for token in &shared.tokens {
        match token.signal {
            Signal::BeginMessage => in_message = true,
            Signal::EndMessage => in_message = false,
            _ if !in_message => tokens.push(token.clone()),
            _ => {}
        }
    }
    tokens.append(&mut consumer.tokens);
    consumer.tokens = tokens;
    for path in &shared.sources {
        if !consumer.sources.contains(path) {
            consumer.sources.push(path.clone());
        }
    }
}

#[allow(clippy::result_large_err)]
fn parse_path(path: &Path, initial_registry: TypeRegistry, xsd: bool) -> Result<Ir, ParseError> {
    let name = path.display().to_string();
    let warn_state = WarnState::new(name.clone());
    let xml = std::fs::read_to_string(path).map_err(|e| ParseError::io(path, e))?;
    if xsd {
        xsd_check(&name, &xml)?;
    }
    let base_dir = path.parent();
    let mut dependencies = Vec::new();
    // Seed the include stack with the main file so an include targeting it
    // is a cycle (self-include or mutual A→B→A).
    let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    dependencies.push(root.clone());
    let mut walk = IncludeWalk::with_root(root);
    let mut ir = parse_with_context(
        &xml,
        base_dir,
        &mut walk,
        initial_registry,
        &warn_state,
        &mut dependencies,
    )?;
    ir.sources = stabilize_dependencies(path, dependencies);
    Ok(ir)
}

/// Root first (canonical when possible), then remaining unique paths in
/// sorted order so Cargo directives are stable across runs.
fn stabilize_dependencies(root: &Path, dependencies: Vec<PathBuf>) -> Vec<PathBuf> {
    let root_key = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    seen.insert(root_key.clone());
    out.push(root_key);
    let mut rest = dependencies;
    rest.sort();
    for path in rest {
        let key = path.canonicalize().unwrap_or(path);
        if seen.insert(key.clone()) {
            out.push(key);
        }
    }
    out
}

/// Internal: parse with optional base directory for include resolution and
/// an initial type registry (seeded from a shared schema, or empty).
pub(crate) fn parse_with_context(
    xml: &str,
    base_dir: Option<&Path>,
    walk: &mut IncludeWalk,
    initial_registry: TypeRegistry,
    warn_state: &WarnState,
    dependencies: &mut Vec<PathBuf>,
) -> Result<Ir, ParseError> {
    let doc = match Document::parse(xml) {
        Ok(d) => d,
        Err(e) => {
            return Err(ParseError::malformed_xml(
                &warn_state.name,
                e.to_string(),
                xml,
            ));
        }
    };
    let input = doc.input_text();
    let root = doc
        .root()
        .children()
        .find(Node::is_element)
        .ok_or_else(|| Fault::missing_no_node("root <messageSchema> element"));
    let root = match root {
        Ok(n) => n,
        Err(fault) => return Err(ParseError::from_fault(&warn_state.name, fault, input)),
    };
    if root.tag_name().name() != "messageSchema" {
        return Err(ParseError::from_fault(
            &warn_state.name,
            Fault::missing(root, "root <messageSchema> element"),
            input,
        ));
    }
    let mut ir = parse_schema(
        root,
        base_dir,
        walk,
        initial_registry,
        warn_state,
        dependencies,
    )
    .map_err(|fault| ParseError::from_fault(&warn_state.name, fault, input))?;
    crate::resolve::resolve_schema(&mut ir, Some(input))?;
    Ok(ir)
}

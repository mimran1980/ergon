//! Public parse entry points.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use roxmltree::{Document, Node};

use crate::ir::Ir;

use super::error::{Fault, ParseError, named_source};
use super::registry::TypeRegistry;
use super::schema::{IncludeWalk, parse_schema};
use super::warn::WarnState;

/// Parse an SBE schema from a string. Returns a token [`Ir`] ready for
/// [`crate::Schema::from_ir`].
///
/// # Errors
/// [`ParseError`] with source spans when the XML is malformed or the schema
/// is structurally invalid.
pub fn parse(xml: &str) -> Result<Ir, ParseError> {
    let warn_state = WarnState::new("<xml>".into());
    parse_with_context(
        xml,
        None,
        &mut IncludeWalk::new(),
        TypeRegistry::new(),
        &warn_state,
        &mut Vec::new(),
    )
}

/// [`parse`], resolving type references against an already-parsed shared
/// schema's composites/enums/sets first — so `xml` need not `<include>` or
/// redeclare them.
///
/// Only composites, enums, and sets round-trip through `shared`'s [`Ir`];
/// bare top-level `<type>` typedefs are inlined and dropped during parsing,
/// so reference those via a `<composite>`/`<enum>`/`<set>` in the shared
/// schema instead.
///
/// # Errors
///
/// Same as [`parse`].
#[allow(clippy::result_large_err)]
pub fn parse_with_shared(xml: &str, shared: &Ir) -> Result<Ir, ParseError> {
    let warn_state = WarnState::new("<xml>".into());
    parse_with_context(
        xml,
        None,
        &mut IncludeWalk::new(),
        TypeRegistry::from_parsed_schema(shared),
        &warn_state,
        &mut Vec::new(),
    )
}

/// [`parse`] after [`crate::validate_against_sbe_xsd`].
///
/// Use in CI for schema authors. Still not a full W3C XSD engine — see
/// [`crate::xsd`]. [`parse`] alone already rejects malformed XML, a bad
/// root, unexpected elements, and unknown attributes; this adds the XSD's
/// wider element/attribute shape check on top.
///
/// # Errors
///
/// XSD structural failures or any [`parse`] error.
#[allow(clippy::result_large_err)]
pub fn parse_with_xsd_validation(xml: &str) -> Result<Ir, ParseError> {
    if let Err(e) = crate::xsd::validate_against_sbe_xsd(xml) {
        return Err(match &e {
            crate::xsd::XsdValidationError::MalformedXml(_) => {
                ParseError::malformed_xml("<xml>", e.to_string(), xml)
            }
            _ => ParseError::Invalid {
                what: "SBE schema".into(),
                value: e.to_string(),
                source_code: named_source("<xml>", xml),
                span: None,
            },
        });
    }
    parse(xml)
}

/// Schema parse result plus every file that contributed to it.
///
/// `parse_file` keeps returning [`Ir`] only. Build helpers use this so Cargo
/// watches the root and every resolved include.
pub(crate) struct ParsedFile {
    pub ir: Ir,
    /// Root first, then remaining resolved includes, each path once, sorted.
    pub dependencies: Vec<PathBuf>,
}

/// Parse a schema file; resolve `xi:include` relative to the file's directory.
///
/// # Errors
///
/// I/O, XML, or schema validation failures as [`ParseError`].
#[allow(clippy::result_large_err)]
pub fn parse_file(path: impl AsRef<Path>) -> Result<Ir, ParseError> {
    Ok(parse_file_with_deps(path)?.ir)
}

/// [`parse_file`], also returning the canonical root and every resolved include.
#[allow(clippy::result_large_err)]
pub(crate) fn parse_file_with_deps(path: impl AsRef<Path>) -> Result<ParsedFile, ParseError> {
    parse_path_with_registry(path.as_ref(), TypeRegistry::new())
}

/// [`parse_file`], resolving type references against an already-parsed
/// shared schema first — see [`parse_with_shared`].
///
/// # Errors
///
/// Same as [`parse_file`].
#[allow(clippy::result_large_err)]
pub fn parse_file_with_shared(path: impl AsRef<Path>, shared: &Ir) -> Result<Ir, ParseError> {
    Ok(parse_file_with_shared_deps(path, shared)?.ir)
}

/// [`parse_file_with_shared`], also returning watched schema files.
#[allow(clippy::result_large_err)]
pub(crate) fn parse_file_with_shared_deps(
    path: impl AsRef<Path>,
    shared: &Ir,
) -> Result<ParsedFile, ParseError> {
    parse_path_with_registry(path.as_ref(), TypeRegistry::from_parsed_schema(shared))
}

#[allow(clippy::result_large_err)]
fn parse_path_with_registry(
    path: &Path,
    initial_registry: TypeRegistry,
) -> Result<ParsedFile, ParseError> {
    let name = path.display().to_string();
    let warn_state = WarnState::new(name.clone());
    let xml = std::fs::read_to_string(path).map_err(|e| ParseError::io(path, e))?;
    let base_dir = path.parent();
    let mut dependencies = Vec::new();
    // Seed the include stack with the main file so an include targeting it
    // is a cycle (self-include or mutual A→B→A).
    let root = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    dependencies.push(root.clone());
    let mut walk = IncludeWalk::with_root(root);
    let ir = parse_with_context(
        &xml,
        base_dir,
        &mut walk,
        initial_registry,
        &warn_state,
        &mut dependencies,
    )?;
    Ok(ParsedFile {
        ir,
        dependencies: stabilize_dependencies(path, dependencies),
    })
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

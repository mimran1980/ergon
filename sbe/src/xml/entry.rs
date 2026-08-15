//! Public parse entry points.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use roxmltree::{Document, Node};

use crate::ir::Ir;

use super::error::{Fault, ParseError, named_source};
use super::registry::TypeRegistry;
use super::schema::parse_schema;
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
        &mut HashSet::new(),
        TypeRegistry::new(),
        &warn_state,
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
        &mut HashSet::new(),
        TypeRegistry::from_parsed_schema(shared),
        &warn_state,
    )
}

/// [`parse`] after [`crate::validate_against_sbe_xsd`].
///
/// Use in CI for schema authors. Still not a full W3C XSD engine — see
/// [`crate::xsd`].
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

/// Parse a schema file; resolve `xi:include` relative to the file's directory.
///
/// # Errors
///
/// I/O, XML, or schema validation failures as [`ParseError`].
#[allow(clippy::result_large_err)]
pub fn parse_file(path: impl AsRef<Path>) -> Result<Ir, ParseError> {
    let path = path.as_ref();
    let name = path.display().to_string();
    let warn_state = WarnState::new(name.clone());
    let xml = std::fs::read_to_string(path).map_err(|e| {
        ParseError::malformed_xml(&name, format!("cannot read {}: {e}", path.display()), "")
    })?;
    let base_dir = path.parent();
    let mut seen = HashSet::new();
    // Seed `seen` with the main file so that any include targeting it is
    // detected as a cycle (self-include or mutual A→B→A).
    if let Ok(canon) = path.canonicalize() {
        seen.insert(canon);
    }
    parse_with_context(&xml, base_dir, &mut seen, TypeRegistry::new(), &warn_state)
}

/// [`parse_file`], resolving type references against an already-parsed
/// shared schema first — see [`parse_with_shared`].
///
/// # Errors
///
/// Same as [`parse_file`].
#[allow(clippy::result_large_err)]
pub fn parse_file_with_shared(path: impl AsRef<Path>, shared: &Ir) -> Result<Ir, ParseError> {
    let path = path.as_ref();
    let name = path.display().to_string();
    let warn_state = WarnState::new(name.clone());
    let xml = std::fs::read_to_string(path).map_err(|e| {
        ParseError::malformed_xml(&name, format!("cannot read {}: {e}", path.display()), "")
    })?;
    let base_dir = path.parent();
    let mut seen = HashSet::new();
    if let Ok(canon) = path.canonicalize() {
        seen.insert(canon);
    }
    parse_with_context(
        &xml,
        base_dir,
        &mut seen,
        TypeRegistry::from_parsed_schema(shared),
        &warn_state,
    )
}

/// Internal: parse with optional base directory for include resolution and
/// an initial type registry (seeded from a shared schema, or empty).
pub(crate) fn parse_with_context(
    xml: &str,
    base_dir: Option<&Path>,
    seen: &mut HashSet<PathBuf>,
    initial_registry: TypeRegistry,
    warn_state: &WarnState,
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
    let mut ir = parse_schema(root, base_dir, seen, initial_registry, warn_state)
        .map_err(|fault| ParseError::from_fault(&warn_state.name, fault, input))?;
    crate::resolve::resolve_schema(&mut ir, Some(input))?;
    Ok(ir)
}

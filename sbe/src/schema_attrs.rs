//! Canonical SBE attribute allow-lists.
//!
//! Shared by the always-on parser checks ([`crate::parse`]) and the opt-in
//! XSD-shaped validator ([`crate::validate_against_sbe_xsd`]) so the two
//! cannot drift apart — a divergence here means one of them rejects schemas
//! the other accepts.
//!
//! Each list is the union of the published SBE XSD grammar with the
//! attributes real-world schemas actually carry. **Neither source alone is
//! complete:** the XSD does not declare `constantValue`, `length`,
//! `nullValue`, or `characterEncoding`, all of which sbe-tool accepts and
//! checked-in schemas use; and no schema corpus exercises every attribute
//! the grammar allows. A list derived from either source on its own rejects
//! valid input.

/// `<messageSchema>` root element.
pub const MESSAGE_SCHEMA: &[&str] = &[
    "package",
    "id",
    "version",
    "semanticVersion",
    "description",
    "byteOrder",
    "headerType",
];

/// `<message>`.
pub const MESSAGE: &[&str] = &[
    "name",
    "id",
    "description",
    "blockLength",
    "semanticType",
    "sinceVersion",
    "deprecated",
];

/// `<field>` and `<data>`.
///
/// One list because the SBE XSD types both elements as `sbe:fieldType` —
/// hand-writing a narrower list for `<data>` drops `epoch`, `offset`,
/// `timeUnit`, and `valueRef`.
pub const FIELD_LIKE: &[&str] = &[
    "name",
    "id",
    "type",
    "description",
    "offset",
    "length",
    "presence",
    "valueRef",
    "constantValue",
    "nullValue",
    "minValue",
    "maxValue",
    "characterEncoding",
    "semanticType",
    "sinceVersion",
    "deprecated",
    "epoch",
    "timeUnit",
];

/// `<group>`.
pub const GROUP: &[&str] = &[
    "name",
    "id",
    "description",
    "dimensionType",
    "blockLength",
    "semanticType",
    "sinceVersion",
    "deprecated",
];

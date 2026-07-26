//! Token intermediate representation for an SBE schema.
//!
//! After [`crate::parse`], you usually only need [`Ir`] via [`crate::Schema`].
//! The flat [`Token`] stream (sbe-tool style) uses [`Signal`] brackets for
//! messages, fields, composites, enums, sets, groups, and var-data; [`Encoding`]
//! holds wire layout. [`crate::resolve_schema`] fills offsets and defaults.
//!
//! Most application code never inspects IR directly — use generated codecs.

/// Byte order declared by the schema; applies to every primitive encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteOrder {
    /// Little-endian — the SBE default.
    LittleEndian,
    /// Big-endian.
    BigEndian,
}

/// Structural role of a token in the IR stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Signal {
    /// Opens a message definition.
    BeginMessage,
    /// Closes a message definition.
    EndMessage,
    /// Opens a field — a message field or a composite member.
    BeginField,
    /// Closes a field.
    EndField,
    /// Opens a composite type definition.
    BeginComposite,
    /// Closes a composite type definition.
    EndComposite,
    /// Opens an enum type definition.
    BeginEnum,
    /// Closes an enum type definition.
    EndEnum,
    /// Opens a bitset (choice/set) type definition.
    BeginSet,
    /// Closes a bitset (choice/set) type definition.
    EndSet,
    /// Opens a repeating group.
    BeginGroup,
    /// Closes a repeating group.
    EndGroup,
    /// Opens a variable-length data field.
    BeginVarData,
    /// Closes a variable-length data field.
    EndVarData,
    /// A leaf encoding token (primitive within a composite, enum, or set).
    Encoding,
}

/// Field presence semantics.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Presence {
    /// Always present on the wire.
    #[default]
    Required,
    /// May be absent; encoded as the type's null value.
    Optional,
    /// Not encoded — the value is fixed by the schema.
    Constant,
}

/// SBE primitive wire types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimitiveType {
    /// Single ASCII byte.
    Char,
    /// Signed 8-bit integer.
    Int8,
    /// Unsigned 8-bit integer.
    UInt8,
    /// Signed 16-bit integer.
    Int16,
    /// Unsigned 16-bit integer.
    UInt16,
    /// Signed 32-bit integer.
    Int32,
    /// Unsigned 32-bit integer.
    UInt32,
    /// Signed 64-bit integer.
    Int64,
    /// Unsigned 64-bit integer.
    UInt64,
    /// IEEE-754 single-precision float.
    Float,
    /// IEEE-754 double-precision float.
    Double,
}

impl PrimitiveType {
    /// On-wire size in bytes.
    #[must_use]
    pub const fn size(self) -> usize {
        match self {
            Self::Char | Self::Int8 | Self::UInt8 => 1,
            Self::Int16 | Self::UInt16 => 2,
            Self::Int32 | Self::UInt32 | Self::Float => 4,
            Self::Int64 | Self::UInt64 | Self::Double => 8,
        }
    }
}

/// How a token is encoded on the wire.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Encoding {
    /// Primitive type for leaf tokens; `None` for structural tokens.
    pub primitive_type: Option<PrimitiveType>,
    /// Byte offset within the enclosing block; `None` when not declared.
    pub offset: Option<usize>,
    /// Presence of the value.
    pub presence: Presence,
    /// Schema version in which this token was introduced.
    pub since_version: u16,
    /// Null sentinel for optional fields; `None` when not applicable.
    pub null_value: Option<u64>,
    /// Character encoding for string fields (e.g. `"UTF-8"`, `"ASCII"`); `None` for non-string fields.
    pub character_encoding: Option<String>,
    /// SBE semantic type annotation (e.g. `"Price"`, `"Qty"`); `None` when not declared.
    pub semantic_type: Option<String>,
    /// Minimum valid value for the type; `None` when not declared.
    pub min_value: Option<u64>,
    /// Maximum valid value for the type; `None` when not declared.
    pub max_value: Option<u64>,
    /// Human-readable description from XML; `None` when absent.
    pub description: Option<String>,
    /// Constant value for `presence="constant"` fields; `None` otherwise.
    pub constant_value: Option<String>,
    /// Array length for fixed-size primitive arrays.
    pub length: Option<usize>,
    /// Epoch for timestamp encoding (e.g. "unix"); `None` when not declared.
    pub epoch: Option<String>,
    /// Time unit for timestamp encoding (e.g. "nanoseconds"); `None` when not declared.
    pub time_unit: Option<String>,
    /// Whether this token's wire size is variable; used for var-data composites.
    pub is_variable_length: bool,
    /// Whether this type or field is marked as deprecated in the schema.
    pub deprecated: bool,
}

/// One token in the flat IR stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Token {
    /// SBE field/message/type id; `None` for structural tokens that don't carry an id.
    pub id: Option<u16>,
    /// Name of the declared entity (message, field, composite, …).
    pub name: String,
    /// Structural role.
    pub signal: Signal,
    /// Wire-encoding metadata. Populated on `BeginField`; default elsewhere.
    pub encoding: Encoding,
}

/// The parsed schema IR: schema-level metadata plus the token stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Ir {
    /// SBE package name.
    pub package: String,
    /// SBE schema id.
    pub id: u16,
    /// SBE schema version.
    pub version: u16,
    /// Schema-declared byte order.
    pub byte_order: ByteOrder,
    /// Schema-level description from XML; `None` when absent.
    pub description: Option<String>,
    /// Semantic version string from XML; `None` when absent.
    pub semantic_version: Option<String>,
    /// Name of the composite type used as the message header (default `"messageHeader"`).
    pub header_type: String,
    /// Flat token stream.
    pub tokens: Vec<Token>,
}

//! Token intermediate representation for an SBE schema.
//!
//! After [`crate::parse`], you usually only need [`Ir`] via [`crate::Schema`].
//! The flat [`Token`] stream (sbe-tool style) uses [`Signal`] brackets for
//! messages, fields, composites, enums, sets, groups, and var-data; [`Encoding`]
//! holds wire layout. [`crate::resolve_schema`] fills offsets and defaults.
//!
//! Application code uses the generated codecs and rarely needs the IR.

/// Byte order declared by the schema; applies to every primitive encoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ByteOrder {
    /// Little-endian, the SBE default.
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
    /// Opens a field: a message field or a composite member.
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
    /// Not encoded; the schema fixes the value.
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

    /// Unsigned integer primitives plus `char` (one octet).
    pub(crate) const fn is_unsigned_int(self) -> bool {
        matches!(
            self,
            Self::Char | Self::UInt8 | Self::UInt16 | Self::UInt32 | Self::UInt64
        )
    }

    /// Signed integer primitives.
    pub(crate) const fn is_signed_int(self) -> bool {
        matches!(self, Self::Int8 | Self::Int16 | Self::Int32 | Self::Int64)
    }

    /// Inclusive range for a signed integer primitive.
    pub(crate) const fn signed_range(self) -> Option<(i64, i64)> {
        match self {
            Self::Int8 => Some((i8::MIN as i64, i8::MAX as i64)),
            Self::Int16 => Some((i16::MIN as i64, i16::MAX as i64)),
            Self::Int32 => Some((i32::MIN as i64, i32::MAX as i64)),
            Self::Int64 => Some((i64::MIN, i64::MAX)),
            _ => None,
        }
    }

    /// Maximum inclusive value for an unsigned integer primitive (or `char`).
    pub(crate) const fn unsigned_max(self) -> Option<u64> {
        match self {
            Self::Char | Self::UInt8 => Some(u8::MAX as u64),
            Self::UInt16 => Some(u16::MAX as u64),
            Self::UInt32 => Some(u32::MAX as u64),
            Self::UInt64 => Some(u64::MAX),
            _ => None,
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
    /// Minimum valid value: the schema's `minValue`, or the SBE primitive
    /// default that [`crate::resolve_schema`] fills in. `None` only before
    /// resolution or for a non-primitive token.
    pub min_value: Option<u64>,
    /// Maximum valid value: the schema's `maxValue`, or the SBE primitive
    /// default that [`crate::resolve_schema`] fills in. `None` only before
    /// resolution or for a non-primitive token.
    pub max_value: Option<u64>,
    /// Whether [`Self::min_value`] came from the schema's `minValue` (set by
    /// the XML parser) rather than the primitive default.
    pub min_value_declared: bool,
    /// Whether [`Self::max_value`] came from the schema's `maxValue` (set by
    /// the XML parser) rather than the primitive default.
    pub max_value_declared: bool,
    /// The top-level `<type>` a primitive field (or composite member) refers
    /// to, e.g. `Timestamp` for `<type name="Timestamp" primitiveType="uint64"/>`;
    /// `None` for a built-in primitive and for every other token.
    pub referenced_type: Option<String>,
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
    /// Schema version the type or field was deprecated in (XML `deprecated`);
    /// `None` when not deprecated.
    pub deprecated: Option<u16>,
}

/// Extension content retained from one schema node: attributes the SBE grammar
/// does not define (namespaced or not) and foreign-namespace child elements. Empty for every
/// node a schema does not annotate. Build-time data only; never read by code
/// generation unless the consumer opts in.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Extensions {
    /// Extension attributes in document order. Never `xmlns*` declarations
    /// (roxmltree resolves those into the namespace table).
    pub attributes: Vec<ExtensionAttr>,
    /// Foreign-namespace child elements in document order.
    pub elements: Vec<ExtensionElement>,
}

/// One attribute the SBE grammar does not own: namespaced (`x:scaleBy`) or an
/// unprefixed name the grammar does not define (`jsonValue`, `unit`, `scale`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionAttr {
    /// Local name (the part after the colon) — the match key; any prefix works.
    pub local_name: String,
    /// Resolved namespace URI; `None` for an unprefixed attribute.
    pub namespace: Option<String>,
    /// Verbatim attribute value.
    pub value: String,
}

/// One foreign-namespace element with its whole subtree, captured verbatim.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtensionElement {
    /// Local name (prefix is not retained: matching is prefix-independent).
    pub local_name: String,
    /// Resolved namespace URI of this element.
    pub namespace: Option<String>,
    /// All attributes of the extension element (its subtree is opaque, so the
    /// SBE allow-list does not apply). Never `xmlns*`.
    pub attributes: Vec<ExtensionAttr>,
    /// Concatenated direct text nodes, verbatim (whitespace preserved);
    /// `None` when the element has no text-node children. Consumers trim.
    pub text: Option<String>,
    /// Child elements, document order, recursively captured. XML comments
    /// inside extension subtrees are dropped.
    pub elements: Vec<ExtensionElement>,
}

impl Extensions {
    /// True when nothing was retained on this node.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.attributes.is_empty() && self.elements.is_empty()
    }

    /// First attribute with this local name, in any namespace — the
    /// “any prefix works” lookup.
    ///
    /// ```
    /// # use ergo_sbe::{ExtensionAttr, Extensions};
    /// let ext = Extensions {
    ///     attributes: vec![ExtensionAttr {
    ///         local_name: "exponent".into(),
    ///         namespace: Some("urn:example".into()),
    ///         value: "priceExponent".into(),
    ///     }],
    ///     elements: vec![],
    /// };
    /// assert_eq!(ext.attr("exponent"), Some("priceExponent"));
    /// ```
    #[must_use]
    pub fn attr(&self, local_name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.local_name == local_name)
            .map(|a| a.value.as_str())
    }

    /// First attribute with this exact (namespace URI, local name).
    #[must_use]
    pub fn attr_in(&self, namespace: &str, local_name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.namespace.as_deref() == Some(namespace) && a.local_name == local_name)
            .map(|a| a.value.as_str())
    }

    /// First child element with this local name, in any namespace.
    #[must_use]
    pub fn child(&self, local_name: &str) -> Option<&ExtensionElement> {
        self.elements.iter().find(|e| e.local_name == local_name)
    }
}

impl ExtensionElement {
    /// First attribute with this local name, in any namespace.
    #[must_use]
    pub fn attr(&self, local_name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.local_name == local_name)
            .map(|a| a.value.as_str())
    }

    /// First attribute with this exact (namespace URI, local name).
    #[must_use]
    pub fn attr_in(&self, namespace: &str, local_name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|a| a.namespace.as_deref() == Some(namespace) && a.local_name == local_name)
            .map(|a| a.value.as_str())
    }

    /// First child element with this local name, in any namespace.
    #[must_use]
    pub fn child(&self, local_name: &str) -> Option<&ExtensionElement> {
        self.elements.iter().find(|e| e.local_name == local_name)
    }

    /// Direct text content, verbatim; `None` when there were no text nodes.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        self.text.as_deref()
    }
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
    /// Source span into the input XML text, for miette diagnostics.
    /// `None` for synthetic tokens not originating from the schema source.
    pub span: Option<std::ops::Range<usize>>,
    /// Extension content of the schema element this token represents.
    /// Empty unless the schema annotates that element; never read by
    /// code generation without an explicit opt-in.
    pub extensions: Extensions,
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
    /// Schema files read to build this IR, each once and canonical where
    /// possible: the root file, then its resolved includes (sorted), then the
    /// shared schema's files (see `SchemaSource::with_shared`). Empty for
    /// inline XML without a shared file schema.
    /// [`crate::GeneratedModuleSet::write_to`] watches these with
    /// `cargo::rerun-if-changed`.
    pub sources: Vec<std::path::PathBuf>,
    /// Extension content at schema level: `messageSchema` root and `<types>`
    /// sections (including those pulled in through `xi:include`), merged in
    /// walk order: the root's own content first, then section/included
    /// content as the parse encounters it.
    pub extensions: Extensions,
}

//! Storage and wire value schemas.
//!
//! [`RowSchema`] describes the ordered columns of a prepared table layout;
//! [`ValueSchema`] describes one value's storage type. Both are cheap
//! wire-friendly descriptors: registration records carry their compact
//! encoding, and every recording call resolves lengths from them without
//! string lookups.

use crate::persist::EncodeError;

/// Storage type code, matching the registration wire encoding.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub enum TypeCode {
    /// 0/1 boolean, stored as `UInt8`.
    Bool = 1,
    I8 = 2,
    I16 = 3,
    I32 = 4,
    I64 = 5,
    U8 = 6,
    U16 = 7,
    U32 = 8,
    U64 = 9,
    F32 = 10,
    F64 = 11,
    /// Fixed-point decimal with explicit scale; stored as `i128` mantissa.
    Decimal = 12,
    /// UTF-8 text (validated on write).
    Utf8 = 13,
    /// Arbitrary bytes.
    Bytes = 14,
    /// UTC nanoseconds since epoch, `u64`.
    TimestampNs = 15,
}

impl TypeCode {
    /// Fixed byte width on the row wire; `None` for variable-length types.
    #[must_use]
    pub const fn fixed_width(self) -> Option<usize> {
        match self {
            Self::Bool | Self::I8 | Self::U8 => Some(1),
            Self::I16 | Self::U16 => Some(2),
            Self::I32 | Self::U32 | Self::F32 => Some(4),
            Self::I64 | Self::U64 | Self::F64 => Some(8),
            Self::Decimal => Some(16),
            Self::Utf8 | Self::Bytes | Self::TimestampNs => None,
        }
    }
}

/// Column flags carried by the registration wire encoding.
pub mod flags {
    /// Value may be absent (nullable storage).
    pub const NULLABLE: u8 = 1;
    /// Value is an array of the declared element type.
    pub const IS_ARRAY: u8 = 2;
}

/// One value's storage type.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ValueSchema {
    /// Base storage type.
    pub ty: TypeCode,
    /// Decimal precision (decimal type only).
    pub precision: u8,
    /// Decimal scale (decimal type only).
    pub scale: i8,
    /// Value may be absent.
    pub nullable: bool,
    /// Value is an array of the base type.
    pub is_array: bool,
}

impl ValueSchema {
    /// Non-nullable scalar schema.
    #[must_use]
    pub const fn scalar(ty: TypeCode) -> Self {
        Self {
            ty,
            precision: 0,
            scale: 0,
            nullable: false,
            is_array: false,
        }
    }

    /// Nullable scalar schema.
    #[must_use]
    pub const fn optional(ty: TypeCode) -> Self {
        Self {
            ty,
            precision: 0,
            scale: 0,
            nullable: true,
            is_array: false,
        }
    }

    /// Array-of-scalar schema.
    #[must_use]
    pub const fn array(ty: TypeCode) -> Self {
        Self {
            ty,
            precision: 0,
            scale: 0,
            nullable: false,
            is_array: true,
        }
    }

    /// Exact decimal schema with precision/scale.
    #[must_use]
    pub const fn decimal(precision: u8, scale: i8) -> Self {
        Self {
            ty: TypeCode::Decimal,
            precision,
            scale,
            nullable: false,
            is_array: false,
        }
    }

    /// Compact wire flags byte.
    #[must_use]
    pub const fn wire_flags(&self) -> u8 {
        let mut f = 0;
        if self.nullable {
            f |= flags::NULLABLE;
        }
        if self.is_array {
            f |= flags::IS_ARRAY;
        }
        f
    }

    /// Encoded length of one value of this schema, where runtime-length
    /// values (text/bytes/arrays) report their prefix and `len` bytes.
    #[must_use]
    pub fn encoded_len(&self, len: usize) -> Option<usize> {
        let base = match (self.ty.fixed_width(), self.is_array) {
            (Some(w), false) => w,
            (None, false) => 4usize.checked_add(len)?,
            (Some(w), true) => 4usize.checked_add(len.checked_mul(w)?)?,
            (None, true) => return None,
        };
        if self.nullable {
            // nullmap bit is amortized per-row; a nullable scalar occupies the
            // same width and absence is recorded in the row bitmap.
            Some(base)
        } else {
            Some(base)
        }
    }
}

/// Ordered column schema for a table layout.
///
/// `columns` is a slice so application row types can declare their schema
/// as a `static`; runtime-built (dynamic) schemas leak once at prepare
/// time, bounded by the layout dictionary limit.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub struct RowSchema {
    /// Ordered column schemas. Order defines the row wire order.
    pub columns: &'static [ValueSchema],
}

impl RowSchema {
    /// Const constructor for static schemas.
    #[must_use]
    pub const fn new(columns: &'static [ValueSchema]) -> Self {
        Self { columns }
    }

    /// Build a runtime schema from owned columns (leaks once at prepare
    /// time; call from the control thread only).
    #[must_use]
    pub fn from_vec(columns: Vec<ValueSchema>) -> Self {
        Self {
            columns: Box::leak(columns.into_boxed_slice()),
        }
    }

    /// Number of columns.
    #[must_use]
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    /// Whether the layout has no columns.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    /// Byte width of the leading null bitmap for this schema.
    #[must_use]
    pub fn nullmap_len(&self) -> usize {
        self.columns
            .iter()
            .filter(|c| c.nullable)
            .count()
            .div_ceil(8)
    }

    /// Minimum encoded length with every variable-length column empty.
    pub fn min_encoded_len(&self) -> Result<usize, EncodeError> {
        let mut total = self.nullmap_len();
        for (i, col) in self.columns.iter().enumerate() {
            if col.is_array {
                total = total.saturating_add(4);
            } else if let Some(w) = col.ty.fixed_width() {
                total = total.saturating_add(w);
            } else {
                // Utf8/Bytes: length prefix only when empty.
                let _ = i;
                total = total.saturating_add(4);
            }
        }
        Ok(total)
    }

    /// FNV-1a hash over the ordered schema; combined with provenance to form
    /// a layout fingerprint.
    #[must_use]
    pub fn fingerprint(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for col in self.columns {
            h = mix(h, col.ty as u8 as u64);
            h = mix(h, u64::from(col.wire_flags()));
            h = mix(h, u64::from(col.precision));
            h = mix(h, (col.scale as i64) as u64);
        }
        h
    }
}

fn mix(mut h: u64, b: u64) -> u64 {
    h ^= b;
    h = h.wrapping_mul(0x100_0000_01b3);
    h
}

/// SBE identity plus immutable mapping revision for a raw-message
/// projection. `schema_id/template_id/version` alone cannot identify
/// unrelated SBE schema files, so the descriptor also carries a fingerprint
/// over the complete descriptor content and its provenance.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ProjectionDescriptor {
    /// Logical table name the projection writes.
    pub table: &'static str,
    /// SBE schema id from the message header.
    pub sbe_schema_id: u16,
    /// SBE template id from the message header.
    pub sbe_template_id: u16,
    /// SBE acting version.
    pub sbe_version: u16,
    /// Fingerprint over descriptor content and provenance.
    pub schema_fingerprint: u64,
    /// Deterministic mapping revision; replay must use the original value.
    pub projection_revision: u32,
    /// Storage schema of the projected row.
    pub row_schema: RowSchema,
}

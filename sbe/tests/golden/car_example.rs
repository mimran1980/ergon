/// Generated from SBE schema package `baseline` id 1 version 0.
#[allow(
    clippy::absurd_extreme_comparisons,
    clippy::double_must_use,
    clippy::erasing_op,
    clippy::identity_op,
    clippy::unnecessary_cast
)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(clippy::eq_op)]
#[allow(clippy::manual_range_contains)]
pub mod sbe_rt {
    ///Generated enum `DecodeError`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DecodeError {
        /// Buffer shorter than needed for `field` (`needed` vs `available` bytes).
        BufferTooShort {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `needed`.
            needed: usize,
            ///Generated field `available`.
            available: usize,
        },
        /// Wire `schemaId` does not match this codec (`expected` name/id vs `actual`).
        WrongSchema {
            ///Generated field `expected`.
            expected: u16,
            ///Generated field `actual`.
            actual: u16,
            ///Generated field `expected_name`.
            expected_name: &'static str,
        },
        /// Wire `templateId` does not match this message (`expected` name/id vs `actual`).
        WrongTemplate {
            ///Generated field `expected`.
            expected: u16,
            ///Generated field `actual`.
            actual: u16,
            ///Generated field `expected_name`.
            expected_name: &'static str,
        },
        /// Multi-template stream saw an id with no registered length/decoder.
        UnknownTemplateLength {
            ///Generated field `template_id`.
            template_id: u16,
        },
        /// Header field value exceeds the supported maximum for this platform.
        InvalidHeaderValue {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `value`.
            value: u64,
            ///Generated field `maximum`.
            maximum: u64,
        },
        /// Length-prefix for var-data exceeds schema max or platform size.
        InvalidVarDataLength {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `length`.
            length: u64,
            ///Generated field `max_length`.
            max_length: u64,
        },
        /// Field/group/data was added in a schema version later than the wire message.
        FieldNotInVersion {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `wire_version`.
            wire_version: u16,
            ///Generated field `since_version`.
            since_version: u16,
        },
        /// Text var-data is not valid UTF-8.
        InvalidUtf8 {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `error`.
            error: core::str::Utf8Error,
        },
        /// Text var-data is not valid ASCII.
        InvalidAscii {
            ///Generated field `field`.
            field: &'static str,
        },
        /// Boolean wire enum was `NullVal` or an unknown discriminant.
        InvalidBoolean {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `discriminant`.
            discriminant: u64,
        },
        /// Domain `try_*` conversion failed.
        DomainConversionFailed {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `reason`.
            reason: &'static str,
        },
        /// Mutable ordered decoder called a dynamic tail out of schema order.
        OutOfOrder {
            ///Generated field `owner`.
            owner: &'static str,
            ///Generated field `expected`.
            expected: &'static str,
            ///Generated field `requested`.
            requested: &'static str,
        },
    }
    impl core::fmt::Display for DecodeError {
        #[cold]
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::BufferTooShort { field, needed, available } => {
                    write!(
                        f, "field '{}': needed {} bytes, {} available", field, needed,
                        available
                    )
                }
                Self::WrongSchema { expected, actual, expected_name } => {
                    write!(
                        f, "wrong schema: expected id {} ({}), got id {}", expected,
                        expected_name, actual
                    )
                }
                Self::WrongTemplate { expected, actual, expected_name } => {
                    write!(
                        f, "wrong template: expected id {} ({}), got id {}", expected,
                        expected_name, actual
                    )
                }
                Self::UnknownTemplateLength { template_id } => {
                    write!(
                        f,
                        "unknown template id {}: SBE messages do not carry length. Use decode_frame() with an external frame length.",
                        template_id
                    )
                }
                Self::InvalidHeaderValue { field, value, maximum } => {
                    write!(
                        f,
                        "message header field '{}': value {} exceeds supported maximum {}",
                        field, value, maximum
                    )
                }
                Self::InvalidVarDataLength { field, length, max_length } => {
                    write!(
                        f, "var data field '{}': length {} exceeds max {}", field,
                        length, max_length
                    )
                }
                Self::FieldNotInVersion { field, wire_version, since_version } => {
                    write!(
                        f, "field '{}' not in wire version {} (added in version {})",
                        field, wire_version, since_version
                    )
                }
                Self::InvalidUtf8 { field, error } => {
                    write!(f, "field '{}': invalid UTF-8: {}", field, error)
                }
                Self::InvalidAscii { field } => {
                    write!(f, "field '{}': invalid ASCII", field)
                }
                Self::InvalidBoolean { field, discriminant } => {
                    write!(
                        f,
                        "field '{}': invalid boolean (discriminant {discriminant:#x})",
                        field
                    )
                }
                Self::DomainConversionFailed { field, reason } => {
                    write!(f, "field '{}': domain conversion failed: {}", field, reason)
                }
                Self::OutOfOrder { owner, expected, requested } => {
                    write!(f, "{owner}: expected '{expected}', requested '{requested}'")
                }
            }
        }
    }
    impl core::error::Error for DecodeError {
        fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
            match self {
                Self::InvalidUtf8 { error, .. } => Some(error),
                _ => None,
            }
        }
    }
    ///Generated enum `EncodeError`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EncodeError {
        /// Encode buffer shorter than needed for `field` (`needed` vs `available`).
        BufferTooShort {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `needed`.
            needed: usize,
            ///Generated field `available`.
            available: usize,
        },
        /// Claim buffer length does not match ENCODED_LENGTH.
        ClaimLengthMismatch {
            ///Generated field `expected`.
            expected: usize,
            ///Generated field `actual`.
            actual: usize,
        },
        /// Var-data payload longer than the schema max for `field`.
        VarDataTooLong {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `max_length`.
            max_length: usize,
            ///Generated field `actual`.
            actual: usize,
        },
        /// Fixed char/byte array source longer than the schema length.
        FixedArrayTooLong {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `max_length`.
            max_length: usize,
            ///Generated field `actual`.
            actual: usize,
        },
        /// ASCII fixed-array `*_str` received a non-ASCII `&str`.
        InvalidAscii {
            ///Generated field `field`.
            field: &'static str,
        },
        /// Domain/DTO value outside the schema min/max range.
        ValueOutOfRange {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `min`.
            min: i128,
            ///Generated field `max`.
            max: i128,
            ///Generated field `actual`.
            actual: i128,
        },
        /// Tried to write more group entries than the declared count.
        GroupFull {
            ///Generated field `declared`.
            declared: u32,
            ///Generated field `attempted`.
            attempted: u32,
        },
        /// Known-size group closure returned without adding enough entries.
        GroupCountMismatch {
            ///Generated field `declared`.
            declared: u32,
            ///Generated field `actual`.
            actual: u32,
        },
        /// Unknown-size group entry count does not fit in `numInGroup`.
        GroupCountOverflow {
            ///Generated field `maximum`.
            maximum: u32,
            ///Generated field `actual`.
            actual: u32,
        },
        /// Checked arithmetic overflow in encoded length computation.
        EncodedLengthOverflow,
        /// Domain `try_*` conversion failed.
        DomainConversionFailed {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `reason`.
            reason: &'static str,
        },
        /// Nested decode failure during encode/verify paths.
        Decode(DecodeError),
    }
    impl core::fmt::Display for EncodeError {
        #[cold]
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::BufferTooShort { field, needed, available } => {
                    write!(
                        f,
                        "buffer too short for {field}: needed {needed}, available {available}"
                    )
                }
                Self::ClaimLengthMismatch { expected, actual } => {
                    write!(
                        f, "claim buffer length mismatch: expected {}, got {}", expected,
                        actual
                    )
                }
                Self::VarDataTooLong { field, max_length, actual } => {
                    write!(
                        f, "var data too long for field {}: max {}, actual {}", field,
                        max_length, actual
                    )
                }
                Self::FixedArrayTooLong { field, max_length, actual } => {
                    write!(
                        f, "fixed array too long for field {}: max {}, actual {}", field,
                        max_length, actual
                    )
                }
                Self::InvalidAscii { field } => {
                    write!(f, "field '{}': invalid ASCII", field)
                }
                Self::ValueOutOfRange { field, min, max, actual } => {
                    write!(
                        f, "value out of range for field {}: min {}, max {}, actual {}",
                        field, min, max, actual
                    )
                }
                Self::GroupFull { declared, attempted } => {
                    write!(
                        f, "group full: declared count {}, attempted to write {}",
                        declared, attempted
                    )
                }
                Self::GroupCountMismatch { declared, actual } => {
                    write!(
                        f, "group count mismatch: declared {declared}, wrote {actual}"
                    )
                }
                Self::GroupCountOverflow { maximum, actual } => {
                    write!(f, "group count overflow: max {maximum}, actual {actual}")
                }
                Self::EncodedLengthOverflow => {
                    write!(f, "encoded length computation overflowed")
                }
                Self::DomainConversionFailed { field, reason } => {
                    write!(f, "domain conversion failed for field {field}: {reason}")
                }
                Self::Decode(e) => write!(f, "decode error: {e}"),
            }
        }
    }
    impl core::error::Error for EncodeError {
        fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
            match self {
                Self::Decode(e) => Some(e),
                _ => None,
            }
        }
    }
    impl From<DecodeError> for EncodeError {
        #[inline]
        fn from(e: DecodeError) -> Self {
            Self::Decode(e)
        }
    }
    /// Meta attribute selector (Java `MetaAttribute` parity).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum MetaAttribute {
        /// Epoch / start of time (e.g. `"unix"`).
        Epoch,
        /// Time unit applied to the epoch (e.g. `"nanosecond"`).
        TimeUnit,
        /// SBE semantic type / FIX-tag relationship.
        SemanticType,
        /// Field presence: `"required"`, `"optional"`, or `"constant"`.
        Presence,
    }
    ///Generated enum `VerifyError`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VerifyError {
        /// Buffer shorter than the message header.
        HeaderTooShort,
        /// Wire block length below the minimum readable for this version.
        InvalidBlockLength {
            ///Generated field `expected_min`.
            expected_min: usize,
            ///Generated field `actual`.
            actual: usize,
        },
        /// Group dimension header for `field` lies past the buffer end.
        GroupDimOutOfBounds {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `offset`.
            offset: usize,
        },
        /// Var-data region for `field` lies past the buffer end.
        VarDataOutOfBounds {
            ///Generated field `field`.
            field: &'static str,
            ///Generated field `offset`.
            offset: usize,
            ///Generated field `length`.
            length: u64,
        },
        /// Full message (header + tails) longer than available bytes.
        MessageTooShort {
            ///Generated field `needed`.
            needed: usize,
            ///Generated field `available`.
            available: usize,
        },
        /// Nested decode error while verifying.
        DecodeError(DecodeError),
    }
    impl core::fmt::Display for VerifyError {
        #[cold]
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::HeaderTooShort => {
                    write!(f, "buffer too short to contain message header")
                }
                Self::InvalidBlockLength { expected_min, actual } => {
                    write!(
                        f, "invalid block length: expected at least {}, actual {}",
                        expected_min, actual
                    )
                }
                Self::GroupDimOutOfBounds { field, offset } => {
                    write!(
                        f, "group dimension header for '{}' out of bounds at offset {}",
                        field, offset
                    )
                }
                Self::VarDataOutOfBounds { field, offset, length } => {
                    write!(
                        f, "var-data for '{}' out of bounds at offset {} with length {}",
                        field, offset, length
                    )
                }
                Self::MessageTooShort { needed, available } => {
                    write!(
                        f, "message too short: needed {} bytes, {} available", needed,
                        available
                    )
                }
                Self::DecodeError(e) => {
                    write!(f, "decode error during verification: {e}")
                }
            }
        }
    }
    impl From<DecodeError> for VerifyError {
        #[inline]
        fn from(e: DecodeError) -> Self {
            VerifyError::DecodeError(e)
        }
    }
    impl core::error::Error for VerifyError {
        fn source(&self) -> Option<&(dyn core::error::Error + 'static)> {
            match self {
                Self::DecodeError(e) => Some(e),
                _ => None,
            }
        }
    }
    /// Convert a wire var-data length without truncation and validate
    /// the complete region with overflow-safe offset arithmetic.
    #[inline]
    pub(crate) fn checked_var_data_bounds(
        field: &'static str,
        offset: usize,
        prefix_size: usize,
        wire_length: u64,
        buffer_length: usize,
    ) -> Result<(usize, usize), DecodeError> {
        let length = usize::try_from(wire_length)
            .map_err(|_| {
                DecodeError::InvalidVarDataLength {
                    field,
                    length: wire_length,
                    max_length: usize::MAX as u64,
                }
            })?;
        let data_start = offset
            .checked_add(prefix_size)
            .ok_or(DecodeError::BufferTooShort {
                field,
                needed: usize::MAX,
                available: buffer_length.saturating_sub(offset),
            })?;
        let data_end = data_start
            .checked_add(length)
            .ok_or(DecodeError::BufferTooShort {
                field,
                needed: usize::MAX,
                available: buffer_length.saturating_sub(offset),
            })?;
        if data_end > buffer_length {
            return Err(DecodeError::BufferTooShort {
                field,
                needed: prefix_size.saturating_add(length),
                available: buffer_length.saturating_sub(offset),
            });
        }
        Ok((data_start, data_end))
    }
    #[inline]
    pub(crate) fn checked_header_u16(
        field: &'static str,
        value: u64,
    ) -> Result<u16, DecodeError> {
        u16::try_from(value)
            .map_err(|_| DecodeError::InvalidHeaderValue {
                field,
                value,
                maximum: u16::MAX as u64,
            })
    }
    #[inline]
    pub(crate) fn checked_header_usize(
        field: &'static str,
        value: u64,
    ) -> Result<usize, DecodeError> {
        usize::try_from(value)
            .map_err(|_| DecodeError::InvalidHeaderValue {
                field,
                value,
                maximum: usize::MAX as u64,
            })
    }
    /// Group `numInGroup` must fit both `usize` and the implementation
    /// iteration ceiling (`u32::MAX`). Never truncate a parser-accepted
    /// unsigned dimension into a narrower diagnostic or loop count.
    #[inline]
    pub(crate) fn checked_group_count(
        field: &'static str,
        value: u64,
    ) -> Result<usize, DecodeError> {
        if value > u32::MAX as u64 {
            return Err(DecodeError::InvalidHeaderValue {
                field,
                value,
                maximum: u32::MAX as u64,
            });
        }
        checked_header_usize(field, value)
    }
    /// Narrow a group count for `GroupFull` / mismatch diagnostics.
    /// Errors instead of truncating when the count exceeds `u32::MAX`.
    #[inline]
    pub(crate) const fn group_diag_count(count: u64) -> Result<u32, EncodeError> {
        if count > u32::MAX as u64 {
            Err(EncodeError::GroupCountOverflow {
                maximum: u32::MAX,
                actual: u32::MAX,
            })
        } else {
            Ok(count as u32)
        }
    }
    /// Convert a wire/user group count to `usize` without truncation.
    #[inline]
    pub(crate) const fn count_to_usize(count: u64) -> Result<usize, EncodeError> {
        if count > usize::MAX as u64 {
            Err(EncodeError::EncodedLengthOverflow)
        } else {
            Ok(count as usize)
        }
    }
    /// `a * b` for encoded-length / verify arithmetic, never wrapping.
    #[inline]
    pub(crate) const fn checked_len_mul(
        a: usize,
        b: usize,
    ) -> Result<usize, EncodeError> {
        match a.checked_mul(b) {
            Some(v) => Ok(v),
            None => Err(EncodeError::EncodedLengthOverflow),
        }
    }
    /// `a + b` for encoded-length / verify arithmetic, never wrapping.
    #[inline]
    pub(crate) const fn checked_len_add(
        a: usize,
        b: usize,
    ) -> Result<usize, EncodeError> {
        match a.checked_add(b) {
            Some(v) => Ok(v),
            None => Err(EncodeError::EncodedLengthOverflow),
        }
    }
    /// Compile-time metadata for a generated SBE message.
    ///
    /// Sealed: the supertrait lives in a private child of the generated
    /// module, so only code generated alongside these types can name
    /// it. Generic framing code can therefore trust that a
    /// `T: SbeMessage` carries this schema's real template id, block
    /// length, schema id, and version.
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is not a generated SBE message type",
        note = "SbeMessage is a sealed trait — only types generated by `ergo_sbe::Generator` can implement it. Import the generated module and use the provided decoder/encoder types directly."
    )]
    pub trait SbeMessage: super::__sbe_message_sealed::Sealed {
        ///Generated constant `TEMPLATE_ID`.
        const TEMPLATE_ID: u16;
        ///Generated constant `BLOCK_LENGTH`.
        const BLOCK_LENGTH: usize;
        ///Generated constant `SCHEMA_ID`.
        const SCHEMA_ID: u16;
        ///Generated constant `SCHEMA_VERSION`.
        const SCHEMA_VERSION: u16;
    }
    mod private {
        pub trait Sealed {}
    }
    /// How a group decoder was obtained.
    ///
    /// A group reached through its message's tail is **attached**: the
    /// decoder knows the real parent body, so completing it can hand
    /// back the next message stage. A group wrapped standalone is
    /// **detached**: it iterates, random-accesses, and rewinds, but has
    /// no parent to return to, so it exposes no message-stage
    /// completion. The distinction is a zero-sized type parameter —
    /// no runtime field, branch, or allocation.
    pub trait GroupContext: private::Sealed {}
    /// Standalone group: no parent message stage. Default context.
    #[doc(hidden)]
    pub struct Detached(());
    impl private::Sealed for Detached {}
    impl GroupContext for Detached {}
    /// Group reached through a message tail; completion is available.
    ///
    /// There is deliberately no safe public constructor: attachment is
    /// a proof, not a claim a consumer can make.
    #[doc(hidden)]
    pub struct Attached(());
    impl private::Sealed for Attached {}
    impl GroupContext for Attached {}
    ///Generated trait `HeaderState`.
    pub trait HeaderState: private::Sealed {}
    ///Generated struct `HeaderPresent`.
    pub struct HeaderPresent;
    impl private::Sealed for HeaderPresent {}
    impl HeaderState for HeaderPresent {}
    ///Generated struct `HeaderAbsent`.
    pub struct HeaderAbsent;
    impl private::Sealed for HeaderAbsent {}
    impl HeaderState for HeaderAbsent {}
    /// Typestate for whether fixed fields have been committed via
    /// `fixed(&FixedFields)`. Tail (group/var-data) methods are only
    /// available on [`FieldsFixed`].
    pub trait FieldsState: private::Sealed {}
    /// Initial encoder stage — fixed fields / `fixed()` / `raw_fixed()` only.
    pub struct FieldsUnfixed;
    impl private::Sealed for FieldsUnfixed {}
    impl FieldsState for FieldsUnfixed {}
    /// After `fixed(&FixedFields)` — ordered group/var-data tails only.
    pub struct FieldsFixed;
    impl private::Sealed for FieldsFixed {}
    impl FieldsState for FieldsFixed {}
    /// Return type for group closures (`add`, `bids`, …).
    /// Closures return `Result<(), EncodeError>`; `?` just works.
    pub type GroupResult = Result<(), EncodeError>;
    ///Generated trait `IntoGroupResult`.
    pub trait IntoGroupResult {
        ///Generated method `into_group_result`.
        fn into_group_result(self) -> GroupResult;
    }
    impl IntoGroupResult for () {
        #[inline]
        fn into_group_result(self) -> GroupResult {
            Ok(())
        }
    }
    impl IntoGroupResult for GroupResult {
        #[inline]
        fn into_group_result(self) -> GroupResult {
            self
        }
    }
}
/// Sealing marker for [`sbe_rt::SbeMessage`]. Private to this generated
/// module: no consumer can name it, so no consumer can forge message
/// metadata by implementing `SbeMessage` for its own type.
pub(crate) mod __sbe_message_sealed {
    pub trait Sealed {}
}
///Boolean Type.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BooleanType {
    ///False value representation.
    F = 0,
    ///True value representation.
    T = 1,
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal = 255,
}
impl BooleanType {
    /// Wire discriminant.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn raw(self) -> u8 {
        self as u8
    }
    /// Reconstruct from a wire discriminant (`NullVal` for unknown).
    #[inline]
    pub const fn from_raw(val: u8) -> Self {
        match val {
            0 => Self::F,
            1 => Self::T,
            _ => Self::NullVal,
        }
    }
    /// Map [`Self::NullVal`] → [`None`], any other variant → [`Some`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn as_option(self) -> Option<Self> {
        if matches!(self, Self::NullVal) { None } else { Some(self) }
    }
    /// Variant name as a `&'static str` — no allocation, unlike
    /// `.to_string()` through [`core::fmt::Display`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::F => stringify!(F),
            Self::T => stringify!(T),
            Self::NullVal => "NullVal",
        }
    }
    /// Returns `Some(true)` / `Some(false)` for the valid boolean
    /// values. Returns `None` for `NullVal` or any unknown raw
    /// discriminant — the SBE boolean wire type is tri-state
    /// (F, T, null). Prefer this (or `TryFrom`) over treating the
    /// raw discriminant as a Rust `bool`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn as_bool(self) -> Option<bool> {
        match self {
            Self::F => Some(false),
            Self::T => Some(true),
            _ => None,
        }
    }
}
impl From<BooleanType> for u8 {
    #[inline]
    fn from(val: BooleanType) -> Self {
        val as u8
    }
}
impl From<u8> for BooleanType {
    #[inline]
    fn from(val: u8) -> Self {
        Self::from_raw(val)
    }
}
impl core::fmt::Display for BooleanType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl core::str::FromStr for BooleanType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            stringify!(F) => Ok(Self::F),
            stringify!(T) => Ok(Self::T),
            "NullVal" => Ok(Self::NullVal),
            _ => Err(()),
        }
    }
}
impl From<bool> for BooleanType {
    #[inline]
    fn from(val: bool) -> Self {
        if val { Self::T } else { Self::F }
    }
}
impl TryFrom<BooleanType> for bool {
    type Error = ();
    #[inline]
    fn try_from(val: BooleanType) -> Result<Self, Self::Error> {
        val.as_bool().ok_or(())
    }
}
/// SBE enum `Model` — wire discriminant u8.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Model {
    ///`A` = 65.
    A = b'A',
    ///`B` = 66.
    B = b'B',
    ///`C` = 67.
    C = b'C',
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal = 0,
}
impl Model {
    /// Wire discriminant.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn raw(self) -> u8 {
        self as u8
    }
    /// Reconstruct from a wire discriminant (`NullVal` for unknown).
    #[inline]
    pub const fn from_raw(val: u8) -> Self {
        match val {
            b'A' => Self::A,
            b'B' => Self::B,
            b'C' => Self::C,
            _ => Self::NullVal,
        }
    }
    /// Map [`Self::NullVal`] → [`None`], any other variant → [`Some`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn as_option(self) -> Option<Self> {
        if matches!(self, Self::NullVal) { None } else { Some(self) }
    }
    /// Variant name as a `&'static str` — no allocation, unlike
    /// `.to_string()` through [`core::fmt::Display`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::A => stringify!(A),
            Self::B => stringify!(B),
            Self::C => stringify!(C),
            Self::NullVal => "NullVal",
        }
    }
}
impl From<Model> for u8 {
    #[inline]
    fn from(val: Model) -> Self {
        val as u8
    }
}
impl From<u8> for Model {
    #[inline]
    fn from(val: u8) -> Self {
        Self::from_raw(val)
    }
}
impl core::fmt::Display for Model {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl core::str::FromStr for Model {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            stringify!(A) => Ok(Self::A),
            stringify!(B) => Ok(Self::B),
            stringify!(C) => Ok(Self::C),
            "NullVal" => Ok(Self::NullVal),
            _ => Err(()),
        }
    }
}
/// SBE enum `BoostType` — wire discriminant u8.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BoostType {
    ///`TURBO` = 84.
    TURBO = b'T',
    ///`SUPERCHARGER` = 83.
    SUPERCHARGER = b'S',
    ///`NITROUS` = 78.
    NITROUS = b'N',
    ///`KERS` = 75.
    KERS = b'K',
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal = 0,
}
impl BoostType {
    /// Wire discriminant.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn raw(self) -> u8 {
        self as u8
    }
    /// Reconstruct from a wire discriminant (`NullVal` for unknown).
    #[inline]
    pub const fn from_raw(val: u8) -> Self {
        match val {
            b'T' => Self::TURBO,
            b'S' => Self::SUPERCHARGER,
            b'N' => Self::NITROUS,
            b'K' => Self::KERS,
            _ => Self::NullVal,
        }
    }
    /// Map [`Self::NullVal`] → [`None`], any other variant → [`Some`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn as_option(self) -> Option<Self> {
        if matches!(self, Self::NullVal) { None } else { Some(self) }
    }
    /// Variant name as a `&'static str` — no allocation, unlike
    /// `.to_string()` through [`core::fmt::Display`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::TURBO => stringify!(TURBO),
            Self::SUPERCHARGER => stringify!(SUPERCHARGER),
            Self::NITROUS => stringify!(NITROUS),
            Self::KERS => stringify!(KERS),
            Self::NullVal => "NullVal",
        }
    }
}
impl From<BoostType> for u8 {
    #[inline]
    fn from(val: BoostType) -> Self {
        val as u8
    }
}
impl From<u8> for BoostType {
    #[inline]
    fn from(val: u8) -> Self {
        Self::from_raw(val)
    }
}
impl core::fmt::Display for BoostType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}
impl core::str::FromStr for BoostType {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            stringify!(TURBO) => Ok(Self::TURBO),
            stringify!(SUPERCHARGER) => Ok(Self::SUPERCHARGER),
            stringify!(NITROUS) => Ok(Self::NITROUS),
            stringify!(KERS) => Ok(Self::KERS),
            "NullVal" => Ok(Self::NullVal),
            _ => Err(()),
        }
    }
}
/// SBE bitset `OptionalExtras` — wire type u8.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct OptionalExtras(pub u8);
impl OptionalExtras {
    ///Generated method `raw`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn raw(self) -> u8 {
        self.0
    }
    ///Generated method `default`.
    #[inline]
    pub const fn default() -> Self {
        Self(0)
    }
    ///Generated method `is_sun_roof`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn is_sun_roof(self) -> bool {
        (self.0 & (1 << 0)) != 0
    }
    ///Generated method `sun_roof`.
    #[inline]
    pub fn sun_roof(&mut self, val: bool) -> &mut Self {
        if val {
            self.0 |= 1 << 0;
        } else {
            self.0 &= !(1 << 0);
        }
        self
    }
    ///Generated method `is_sports_pack`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn is_sports_pack(self) -> bool {
        (self.0 & (1 << 1)) != 0
    }
    ///Generated method `sports_pack`.
    #[inline]
    pub fn sports_pack(&mut self, val: bool) -> &mut Self {
        if val {
            self.0 |= 1 << 1;
        } else {
            self.0 &= !(1 << 1);
        }
        self
    }
    ///Generated method `is_cruise_control`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn is_cruise_control(self) -> bool {
        (self.0 & (1 << 2)) != 0
    }
    ///Generated method `cruise_control`.
    #[inline]
    pub fn cruise_control(&mut self, val: bool) -> &mut Self {
        if val {
            self.0 |= 1 << 2;
        } else {
            self.0 &= !(1 << 2);
        }
        self
    }
}
impl From<u8> for OptionalExtras {
    #[inline]
    fn from(val: u8) -> Self {
        Self(val)
    }
}
impl From<OptionalExtras> for u8 {
    #[inline]
    fn from(val: OptionalExtras) -> Self {
        val.0
    }
}
impl core::fmt::Display for OptionalExtras {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut first = true;
        if self.is_sun_roof() {
            if !first {
                f.write_str("|")?;
            }
            f.write_str("sunRoof")?;
            first = false;
        }
        if self.is_sports_pack() {
            if !first {
                f.write_str("|")?;
            }
            f.write_str("sportsPack")?;
            first = false;
        }
        if self.is_cruise_control() {
            if !first {
                f.write_str("|")?;
            }
            f.write_str("cruiseControl")?;
            first = false;
        }
        Ok(())
    }
}
impl core::str::FromStr for OptionalExtras {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let mut v = Self::default();
        if s.is_empty() {
            return Ok(v);
        }
        for part in s.split('|') {
            let part = part.trim();
            let mut matched = false;
            if part == "sunRoof" {
                v.sun_roof(true);
                matched = true;
            }
            if part == "sportsPack" {
                v.sports_pack(true);
                matched = true;
            }
            if part == "cruiseControl" {
                v.cruise_control(true);
                matched = true;
            }
            if !matched {
                return Err(());
            }
        }
        Ok(v)
    }
}
///Message identifiers and length of message root.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MessageHeader(pub [u8; 8]);
impl core::fmt::Debug for MessageHeader {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(stringify!(MessageHeader))
            .field("blockLength", &self.block_length())
            .field("templateId", &self.template_id())
            .field("schemaId", &self.schema_id())
            .field("version", &self.version())
            .finish()
    }
}
impl MessageHeader {
    ///Generated method `block_length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn block_length(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 0))
    }
    ///Generated method `template_id`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn template_id(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 2))
    }
    ///Generated method `schema_id`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn schema_id(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 4))
    }
    ///Generated method `version`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn version(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 6))
    }
    ///Generated method `new`.
    #[inline]
    pub fn new(
        block_length: u16,
        template_id: u16,
        schema_id: u16,
        version: u16,
    ) -> Self {
        let mut bytes = [0u8; 8];
        let val_bytes = block_length.to_le_bytes();
        write_bytes::<2>(&mut bytes, 0, &val_bytes);
        let val_bytes = template_id.to_le_bytes();
        write_bytes::<2>(&mut bytes, 2, &val_bytes);
        let val_bytes = schema_id.to_le_bytes();
        write_bytes::<2>(&mut bytes, 4, &val_bytes);
        let val_bytes = version.to_le_bytes();
        write_bytes::<2>(&mut bytes, 6, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < MessageHeader > () == 8);
/// Canonical wire size of the SBE message header.
pub const MESSAGE_HEADER_ENCODED_LENGTH: usize = 8;
/// Parsed `(template_id, schema_id)` from [`MessageHeader::peek_header`].
/// Named fields prevent silent transposition of the two `u16` values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PeekedHeader {
    ///Generated field `template_id`.
    pub template_id: u16,
    ///Generated field `schema_id`.
    pub schema_id: u16,
}
impl MessageHeader {
    /// Read the header fields from a buffer without constructing a
    /// full `MessageHeader`. Returns `None` when the buffer is
    /// shorter than the header.
    #[must_use = "the peeked header identity is unused; ignoring it skips dispatch"]
    #[inline]
    pub fn peek_header(data: &[u8]) -> Option<PeekedHeader> {
        if data.len() < 8 {
            return None;
        }
        let mut hdr = [0u8; 8];
        hdr.copy_from_slice(&data[..8]);
        let this = Self(hdr);
        let template_id = u16::try_from(this.template_id() as u64).ok()?;
        let schema_id = u16::try_from(this.schema_id() as u64).ok()?;
        Some(PeekedHeader {
            template_id,
            schema_id,
        })
    }
    /// Read `template_id` from a frame without constructing a full
    /// `MessageHeader`. Returns `None` when the buffer is shorter
    /// than the header. For correct multi-schema dispatch,
    /// prefer [`Self::peek_header`] which also returns `schema_id`.
    #[must_use = "the peeked template id is unused; ignoring it skips dispatch"]
    #[inline]
    pub fn peek_template_id(data: &[u8]) -> Option<u16> {
        if data.len() < 8 {
            return None;
        }
        let mut hdr = [0u8; 8];
        hdr.copy_from_slice(&data[..8]);
        u16::try_from(Self(hdr).template_id() as u64).ok()
    }
    /// Validate `schema_id` and return `template_id`. Returns
    /// `None` when the buffer is too short or the schema doesn't
    /// match. Use this for correct multi-schema dispatch.
    #[must_use = "the schema-matched template id is unused; ignoring it skips dispatch"]
    #[inline]
    pub fn peek_for_schema(data: &[u8], expected_schema_id: u16) -> Option<u16> {
        let header = Self::peek_header(data)?;
        if header.schema_id == expected_schema_id {
            Some(header.template_id)
        } else {
            None
        }
    }
}
/// Flyweight decoder for the `messageHeader` composite.
#[derive(Clone, Copy)]
pub struct MessageHeaderDecoder<'a> {
    pub(crate) buf: &'a [u8],
    /// Byte offset of the composite body within `self.buf`.
    pub(crate) offset: usize,
}
impl<'a> MessageHeaderDecoder<'a> {
    ///Generated method `block_length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn block_length(&self) -> u16 {
        u16::from_le_bytes(unsafe {
            read_bytes_unchecked::<2>(self.buf, self.offset + 0)
        })
    }
    ///Generated method `template_id`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn template_id(&self) -> u16 {
        u16::from_le_bytes(unsafe {
            read_bytes_unchecked::<2>(self.buf, self.offset + 2)
        })
    }
    ///Generated method `schema_id`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn schema_id(&self) -> u16 {
        u16::from_le_bytes(unsafe {
            read_bytes_unchecked::<2>(self.buf, self.offset + 4)
        })
    }
    ///Generated method `version`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn version(&self) -> u16 {
        u16::from_le_bytes(unsafe {
            read_bytes_unchecked::<2>(self.buf, self.offset + 6)
        })
    }
}
///Repeating group dimensions.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct GroupSizeEncoding(pub [u8; 4]);
impl core::fmt::Debug for GroupSizeEncoding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(stringify!(GroupSizeEncoding))
            .field("blockLength", &self.block_length())
            .field("numInGroup", &self.num_in_group())
            .finish()
    }
}
impl GroupSizeEncoding {
    ///Generated method `block_length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn block_length(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 0))
    }
    ///Generated method `num_in_group`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn num_in_group(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 2))
    }
    ///Generated method `new`.
    #[inline]
    pub fn new(block_length: u16, num_in_group: u16) -> Self {
        let mut bytes = [0u8; 4];
        let val_bytes = block_length.to_le_bytes();
        write_bytes::<2>(&mut bytes, 0, &val_bytes);
        let val_bytes = num_in_group.to_le_bytes();
        write_bytes::<2>(&mut bytes, 2, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < GroupSizeEncoding > () == 4);
/// Flyweight decoder for the `groupSizeEncoding` composite.
#[derive(Clone, Copy)]
pub struct GroupSizeEncodingDecoder<'a> {
    pub(crate) buf: &'a [u8],
    /// Byte offset of the composite body within `self.buf`.
    pub(crate) offset: usize,
}
impl<'a> GroupSizeEncodingDecoder<'a> {
    ///Generated method `block_length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn block_length(&self) -> u16 {
        u16::from_le_bytes(unsafe {
            read_bytes_unchecked::<2>(self.buf, self.offset + 0)
        })
    }
    ///Generated method `num_in_group`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn num_in_group(&self) -> u16 {
        u16::from_le_bytes(unsafe {
            read_bytes_unchecked::<2>(self.buf, self.offset + 2)
        })
    }
}
///Variable length UTF-8 String.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarStringEncoding(pub [u8; 4]);
impl core::fmt::Debug for VarStringEncoding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(stringify!(VarStringEncoding))
            .field("length", &self.length())
            .field("varData", &self.var_data())
            .finish()
    }
}
impl VarStringEncoding {
    ///Generated method `length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes(read_bytes::<4>(&self.0, 0))
    }
    ///Generated method `var_data`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
    ///Generated method `new`.
    #[inline]
    pub fn new(length: u32, var_data: [u8; 0]) -> Self {
        let mut bytes = [0u8; 4];
        let val_bytes = length.to_le_bytes();
        write_bytes::<4>(&mut bytes, 0, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < VarStringEncoding > () == 4);
/// Flyweight decoder for the `varStringEncoding` composite.
#[derive(Clone, Copy)]
pub struct VarStringEncodingDecoder<'a> {
    pub(crate) buf: &'a [u8],
    /// Byte offset of the composite body within `self.buf`.
    pub(crate) offset: usize,
}
impl<'a> VarStringEncodingDecoder<'a> {
    ///Generated method `length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes(unsafe {
            read_bytes_unchecked::<4>(self.buf, self.offset + 0)
        })
    }
    ///Generated method `var_data`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
///Variable length ASCII String.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarAsciiEncoding(pub [u8; 4]);
impl core::fmt::Debug for VarAsciiEncoding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(stringify!(VarAsciiEncoding))
            .field("length", &self.length())
            .field("varData", &self.var_data())
            .finish()
    }
}
impl VarAsciiEncoding {
    ///Generated method `length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes(read_bytes::<4>(&self.0, 0))
    }
    ///Generated method `var_data`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
    ///Generated method `new`.
    #[inline]
    pub fn new(length: u32, var_data: [u8; 0]) -> Self {
        let mut bytes = [0u8; 4];
        let val_bytes = length.to_le_bytes();
        write_bytes::<4>(&mut bytes, 0, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < VarAsciiEncoding > () == 4);
/// Flyweight decoder for the `varAsciiEncoding` composite.
#[derive(Clone, Copy)]
pub struct VarAsciiEncodingDecoder<'a> {
    pub(crate) buf: &'a [u8],
    /// Byte offset of the composite body within `self.buf`.
    pub(crate) offset: usize,
}
impl<'a> VarAsciiEncodingDecoder<'a> {
    ///Generated method `length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes(unsafe {
            read_bytes_unchecked::<4>(self.buf, self.offset + 0)
        })
    }
    ///Generated method `var_data`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
///Variable length binary blob.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarDataEncoding(pub [u8; 4]);
impl core::fmt::Debug for VarDataEncoding {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(stringify!(VarDataEncoding))
            .field("length", &self.length())
            .field("varData", &self.var_data())
            .finish()
    }
}
impl VarDataEncoding {
    ///Generated method `length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes(read_bytes::<4>(&self.0, 0))
    }
    ///Generated method `var_data`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
    ///Generated method `new`.
    #[inline]
    pub fn new(length: u32, var_data: [u8; 0]) -> Self {
        let mut bytes = [0u8; 4];
        let val_bytes = length.to_le_bytes();
        write_bytes::<4>(&mut bytes, 0, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < VarDataEncoding > () == 4);
/// Flyweight decoder for the `varDataEncoding` composite.
#[derive(Clone, Copy)]
pub struct VarDataEncodingDecoder<'a> {
    pub(crate) buf: &'a [u8],
    /// Byte offset of the composite body within `self.buf`.
    pub(crate) offset: usize,
}
impl<'a> VarDataEncodingDecoder<'a> {
    ///Generated method `length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes(unsafe {
            read_bytes_unchecked::<4>(self.buf, self.offset + 0)
        })
    }
    ///Generated method `var_data`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
/// SBE composite `Booster` — 2 byte wire image.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Booster(pub [u8; 2]);
impl core::fmt::Debug for Booster {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(stringify!(Booster))
            .field("BoostType", &self.boost_type())
            .field("horsePower", &self.horse_power())
            .finish()
    }
}
impl Booster {
    ///Generated method `boost_type`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn boost_type(&self) -> BoostType {
        BoostType::from_raw(u8::from_le_bytes(read_bytes::<1>(&self.0, 0)))
    }
    /// Raw wire discriminant — bypasses enum mapping.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn raw_boost_type(&self) -> u8 {
        u8::from_le_bytes(read_bytes::<1>(&self.0, 0))
    }
    ///Generated method `horse_power`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn horse_power(&self) -> u8 {
        u8::from_le_bytes(read_bytes::<1>(&self.0, 1))
    }
    ///Generated method `new`.
    #[inline]
    pub fn new(boost_type: BoostType, horse_power: u8) -> Self {
        let mut bytes = [0u8; 2];
        let val_bytes = (boost_type as u8).to_le_bytes();
        write_bytes::<1>(&mut bytes, 0, &val_bytes);
        let val_bytes = horse_power.to_le_bytes();
        write_bytes::<1>(&mut bytes, 1, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < Booster > () == 2);
/// Flyweight decoder for the `Booster` composite.
#[derive(Clone, Copy)]
pub struct BoosterDecoder<'a> {
    pub(crate) buf: &'a [u8],
    /// Byte offset of the composite body within `self.buf`.
    pub(crate) offset: usize,
}
impl<'a> BoosterDecoder<'a> {
    ///Generated method `boost_type`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn boost_type(&self) -> BoostType {
        BoostType::from_raw(
            u8::from_le_bytes(unsafe {
                read_bytes_unchecked::<1>(self.buf, self.offset + 0)
            }),
        )
    }
    ///Generated method `horse_power`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn horse_power(&self) -> u8 {
        u8::from_le_bytes(unsafe {
            read_bytes_unchecked::<1>(self.buf, self.offset + 1)
        })
    }
}
/// SBE composite `Engine` — 10 byte wire image.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Engine(pub [u8; 10]);
impl core::fmt::Debug for Engine {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct(stringify!(Engine))
            .field("capacity", &self.capacity())
            .field("numCylinders", &self.num_cylinders())
            .field("maxRpm", &self.max_rpm())
            .field("manufacturerCode", &self.manufacturer_code())
            .field("fuel", &self.fuel())
            .field("efficiency", &self.efficiency())
            .field("boosterEnabled", &self.booster_enabled())
            .field("booster", &self.booster())
            .finish()
    }
}
impl Engine {
    ///Generated method `capacity`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn capacity(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 0))
    }
    ///Generated method `num_cylinders`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn num_cylinders(&self) -> u8 {
        u8::from_le_bytes(read_bytes::<1>(&self.0, 2))
    }
    ///Generated method `max_rpm`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn max_rpm(&self) -> u16 {
        9000
    }
    ///Generated method `manufacturer_code`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn manufacturer_code(&self) -> [u8; 3] {
        let mut res = [0 as u8; 3];
        let mut idx = 0;
        while idx < 3 {
            let offset = 3 + idx * 1;
            res[idx] = u8::from_le_bytes(read_bytes::<1>(&self.0, offset));
            idx += 1;
        }
        res
    }
    ///Generated method `fuel`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn fuel(&self) -> &'static str {
        "Petrol"
    }
    ///Generated method `efficiency`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn efficiency(&self) -> i8 {
        i8::from_le_bytes(read_bytes::<1>(&self.0, 6))
    }
    ///Generated method `booster_enabled`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn booster_enabled(&self) -> BooleanType {
        BooleanType::from_raw(u8::from_le_bytes(read_bytes::<1>(&self.0, 7)))
    }
    /// Raw wire discriminant — bypasses enum mapping.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn raw_booster_enabled(&self) -> u8 {
        u8::from_le_bytes(read_bytes::<1>(&self.0, 7))
    }
    ///Generated method `booster`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn booster(&self) -> Booster {
        Booster(read_bytes::<2>(&self.0, 8))
    }
    ///Generated method `new`.
    #[inline]
    pub fn new(
        capacity: u16,
        num_cylinders: u8,
        manufacturer_code: [u8; 3],
        efficiency: i8,
        booster_enabled: BooleanType,
        booster: Booster,
    ) -> Self {
        let mut bytes = [0u8; 10];
        let val_bytes = capacity.to_le_bytes();
        write_bytes::<2>(&mut bytes, 0, &val_bytes);
        let val_bytes = num_cylinders.to_le_bytes();
        write_bytes::<1>(&mut bytes, 2, &val_bytes);
        let mut idx = 0;
        while idx < 3 {
            let val_bytes = manufacturer_code[idx].to_le_bytes();
            write_bytes::<1>(&mut bytes, 3 + idx * 1, &val_bytes);
            idx += 1;
        }
        let val_bytes = efficiency.to_le_bytes();
        write_bytes::<1>(&mut bytes, 6, &val_bytes);
        let val_bytes = (booster_enabled as u8).to_le_bytes();
        write_bytes::<1>(&mut bytes, 7, &val_bytes);
        write_bytes::<2>(&mut bytes, 8, &booster.0);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < Engine > () == 10);
/// Flyweight decoder for the `Engine` composite.
#[derive(Clone, Copy)]
pub struct EngineDecoder<'a> {
    pub(crate) buf: &'a [u8],
    /// Byte offset of the composite body within `self.buf`.
    pub(crate) offset: usize,
}
impl<'a> EngineDecoder<'a> {
    ///Generated method `capacity`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn capacity(&self) -> u16 {
        u16::from_le_bytes(unsafe {
            read_bytes_unchecked::<2>(self.buf, self.offset + 0)
        })
    }
    ///Generated method `num_cylinders`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn num_cylinders(&self) -> u8 {
        u8::from_le_bytes(unsafe {
            read_bytes_unchecked::<1>(self.buf, self.offset + 2)
        })
    }
    ///Generated method `max_rpm`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn max_rpm(&self) -> u16 {
        9000
    }
    ///Generated method `manufacturer_code`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn manufacturer_code(&self) -> [u8; 3] {
        let mut res = [0 as u8; 3];
        let mut idx = 0;
        while idx < 3 {
            res[idx] = u8::from_le_bytes(unsafe {
                read_bytes_unchecked::<1>(self.buf, self.offset + 3 + idx * 1)
            });
            idx += 1;
        }
        res
    }
    ///Generated method `fuel`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn fuel(&self) -> &'static str {
        "Petrol"
    }
    ///Generated method `efficiency`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn efficiency(&self) -> i8 {
        i8::from_le_bytes(unsafe {
            read_bytes_unchecked::<1>(self.buf, self.offset + 6)
        })
    }
    ///Generated method `booster_enabled`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn booster_enabled(&self) -> BooleanType {
        BooleanType::from_raw(
            u8::from_le_bytes(unsafe {
                read_bytes_unchecked::<1>(self.buf, self.offset + 7)
            }),
        )
    }
    ///Generated method `booster`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn booster(&self) -> Booster {
        Booster(unsafe { read_bytes_unchecked::<2>(self.buf, self.offset + 8) })
    }
}
#[doc = concat!(
    "Schema constants for the `", stringify!(CarSchema),
    "` message: `SCHEMA_ID`, `SCHEMA_VERSION`, `TEMPLATE_ID`, `BLOCK_LENGTH`, `HEADER_LENGTH`."
)]
pub struct CarSchema;
impl CarSchema {
    ///`SCHEMA_ID` = 1.
    pub const SCHEMA_ID: u16 = 1;
    ///`SCHEMA_VERSION` = 0.
    pub const SCHEMA_VERSION: u16 = 0;
    ///`TEMPLATE_ID` = 1.
    pub const TEMPLATE_ID: u16 = 1;
    ///`BLOCK_LENGTH` = 45.
    pub const BLOCK_LENGTH: usize = 45;
    ///`HEADER_LENGTH` = 8.
    pub const HEADER_LENGTH: usize = 8;
    /// Full structural verification of a buffer: validates header,
    /// block-length extent, group dimension headers, entry strides,
    /// and var-data bounds. Use **before** construction when the
    /// entire frame must be proven valid without building a decoder.
    #[inline]
    pub fn verify(buf: &[u8]) -> Result<(), sbe_rt::VerifyError> {
        CarDecoder::verify(buf)
    }
}
///Description of a basic Car
#[must_use = "decoder must be read or advanced; dropping is fine only after use"]
pub struct CarDecoder<'a> {
    pub(crate) buf: &'a [u8],
    /// Byte offset of the message body within `self.buf`.
    pub(crate) offset: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
/// Buffer-placement and wire-frame metadata. Holds a reference to the
/// parent decoder — zero-copy. Utility methods live here so no schema
/// field can collide with them. Byte views on this facet span the
/// **acting fixed block only**; complete frames use the complete stage
/// or the decoder's tail-rescan helpers when the message has groups
/// or var-data.
#[derive(Clone, Copy)]
pub struct CarDecoderMetadata<'m, 'a> {
    decoder: &'m CarDecoder<'a>,
}
impl<'m, 'a> CarDecoderMetadata<'m, 'a> {
    /// Absolute offset of this message's frame start (first header byte)
    /// within the underlying buffer.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn message_offset(&self) -> usize {
        self.decoder.byte_offset().saturating_sub(CarDecoder::HEADER_LENGTH)
    }
    /// End of the **acting fixed block** (body start + acting block length).
    /// Not the full message end when groups/var-data follow — use a complete
    /// stage or inherent `encoded_length_with_header` after walking tails.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn limit(&self) -> usize {
        self.decoder.byte_offset() + self.decoder.acting_block_length
    }
    /// The full underlying buffer slice this decoder was wrapped on.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn buffer(&self) -> &'a [u8] {
        self.decoder.buf
    }
    /// Bytes after the acting fixed block end. May still contain unread
    /// groups/var-data of **this** message until the consuming walk finishes.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        let end = (self.decoder.byte_offset() + self.decoder.acting_block_length)
            .min(self.decoder.buf.len());
        &self.decoder.buf[end..]
    }
    /// Fixed-block body only (groups/var-data not included).
    /// For a complete frame walk tails then use the complete stage's
    /// `as_bytes_with_header`, or the decoder's inherent
    /// `as_bytes_with_header` which rescans tails without consuming
    /// the stage.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_fixed_body_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let start = self.decoder.byte_offset();
        let end = self.decoder.byte_offset() + self.decoder.acting_block_length;
        if start > self.decoder.buf.len() || end > self.decoder.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "body",
                needed: end.saturating_sub(start),
                available: self.decoder.buf.len().saturating_sub(start),
            });
        }
        Ok(&self.decoder.buf[start..end])
    }
    /// Header + fixed block only — **not** a complete SBE message when
    /// groups or var-data remain. Prefer the complete stage's
    /// `as_bytes_with_header` after finishing the walk.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_fixed_region_with_header(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let start = self.message_offset();
        let end = self.decoder.byte_offset() + self.decoder.acting_block_length;
        if start > self.decoder.buf.len() || end > self.decoder.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "frame",
                needed: end.saturating_sub(start),
                available: self.decoder.buf.len().saturating_sub(start),
            });
        }
        Ok(&self.decoder.buf[start..end])
    }
    /// Schema version from the message header (or wrap args), not the
    /// compiled schema constant. Fields with `sinceVersion` and optional
    /// presence depend on this value.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn acting_version(&self) -> u16 {
        self.decoder.acting_version
    }
    /// Block length from the wire header / wrap args. Tail offsets use
    /// this acting length, not only the compiled `BLOCK_LENGTH`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn acting_block_length(&self) -> usize {
        self.decoder.acting_block_length
    }
}
impl<'a> CarDecoder<'a> {
    /// Metadata accessor: buffer positions, wire-frame boundaries,
    /// version/block-length state. Returns a zero-copy reference to
    /// the parent decoder — no fields are copied. All utility methods
    /// are scoped here so no schema field name can collide with them.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn get_metadata(&self) -> CarDecoderMetadata<'_, 'a> {
        CarDecoderMetadata {
            decoder: self,
        }
    }
    ///`SCHEMA_ID` = 1.
    pub const SCHEMA_ID: u16 = 1;
    ///`SCHEMA_VERSION` = 0.
    pub const SCHEMA_VERSION: u16 = 0;
    ///`TEMPLATE_ID` = 1.
    pub const TEMPLATE_ID: u16 = 1;
    ///`BLOCK_LENGTH` = 45.
    pub const BLOCK_LENGTH: usize = 45;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 45);
    /// Schema-declared message header size in bytes.
    pub const HEADER_LENGTH: usize = 8;
    /// Minimum body bytes needed to safely read every fixed field present
    /// at `acting_version` (version-aware; not always full `BLOCK_LENGTH`).
    #[must_use = "this extent is the minimum readable body size; ignoring it skips a bounds check"]
    #[inline]
    pub const fn min_readable_fixed_extent(acting_version: u16) -> usize {
        let mut m = 45;
        m
    }
    /// Wrap a buffer for decoding at **message start** with bounds checks.
    /// Fields are at `message_offset + HEADER_LENGTH + field_offset`.
    ///
    /// Validates that the body holds `max(acting_block_length,
    /// min_readable_fixed_extent(acting_version))` bytes so required
    /// accessors never read out of bounds from safe code.
    ///
    /// # Migration from sbe-tool
    ///
    /// sbe-tool Rust `wrap` takes the **body** offset (usually
    /// `message_start + 8`). ergo-sbe takes the **message** start so the
    /// same offset works for `wrap`, `decode`, and claim buffers.
    #[inline]
    pub fn try_wrap(
        buf: &'a [u8],
        message_offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let Some(body_offset) = message_offset.checked_add(Self::HEADER_LENGTH) else {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: Self::HEADER_LENGTH,
                available: buf.len().saturating_sub(message_offset),
            });
        };
        let available_body = buf.len().saturating_sub(body_offset);
        let min_fixed = Self::min_readable_fixed_extent(acting_version);
        let body_need = if acting_block_length > min_fixed {
            acting_block_length
        } else {
            min_fixed
        };
        if body_need > available_body {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message body",
                needed: Self::HEADER_LENGTH.saturating_add(body_need),
                available: buf.len().saturating_sub(message_offset),
            });
        }
        Ok(unsafe {
            Self::wrap_unchecked(
                buf,
                message_offset,
                acting_block_length,
                acting_version,
            )
        })
    }
    /// Trusted external-metadata wrap. Proves version-aware fixed extent
    /// then constructs; **panics** if the buffer is too short. Field
    /// accessors use unchecked reads justified by that proof.
    ///
    /// Prefer [`Self::try_wrap`] at untrusted boundaries. Uses a direct
    /// extent check (not `try_wrap` + match) so the hot success path does
    /// not construct a `Result` — same contract as encoder bare `wrap`.
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        message_offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        let Some(body_offset) = message_offset.checked_add(Self::HEADER_LENGTH) else {
            panic!("buffer too short for message header");
        };
        let available_body = buf.len().saturating_sub(body_offset);
        let min_fixed = Self::min_readable_fixed_extent(acting_version);
        let body_need = if acting_block_length > min_fixed {
            acting_block_length
        } else {
            min_fixed
        };
        if body_need > available_body {
            panic!("buffer too short for message body");
        }
        unsafe {
            Self::wrap_unchecked(
                buf,
                message_offset,
                acting_block_length,
                acting_version,
            )
        }
    }
    /// Zero-check wrap — raw pointer accessors, **UB** on OOB.
    /// Only for proven-tight hot loops after an external extent proof.
    ///
    /// # Safety
    /// `message_offset + HEADER_LENGTH + max(acting_block_length,
    /// min_readable_fixed_extent(acting_version))` must not overflow
    /// and must be ≤ `buf.len()`.
    #[inline]
    pub unsafe fn wrap_unchecked(
        buf: &'a [u8],
        message_offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        let body_offset = message_offset + Self::HEADER_LENGTH;
        Self {
            buf,
            offset: body_offset,
            acting_block_length,
            acting_version,
        }
    }
    /// Decode a framed message at **message start** (`offset` = first
    /// byte of the header). Validates header fields and the
    /// version-aware fixed body extent. See [`Self::wrap`] for the
    /// message-start coordinate system.
    #[inline]
    pub fn try_decode(
        buf: &'a [u8],
        offset: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if 8 > buf.len().saturating_sub(offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: buf.len().saturating_sub(offset),
            });
        }
        let header_bytes: [u8; 8] = read_bytes::<8>(buf, offset);
        let header = MessageHeader(header_bytes);
        let template_id = sbe_rt::checked_header_u16(
            "templateId",
            header.template_id() as u64,
        )?;
        if template_id != Self::TEMPLATE_ID {
            return Err(sbe_rt::DecodeError::WrongTemplate {
                expected: Self::TEMPLATE_ID,
                actual: template_id,
                expected_name: "Car",
            });
        }
        let schema_id = sbe_rt::checked_header_u16(
            "schemaId",
            header.schema_id() as u64,
        )?;
        if schema_id != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: schema_id,
                expected_name: "baseline",
            });
        }
        let acting_block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let acting_version = sbe_rt::checked_header_u16(
            "version",
            header.version() as u64,
        )?;
        Self::try_wrap(buf, offset, acting_block_length, acting_version)
    }
    /// Trusted framed decode — **hybrid return** (freeze-friendly):
    ///
    /// - **Extent (short buffer):** panics after the same proof as
    ///   [`Self::wrap`] (trusted tier).
    /// - **Identity (wrong template/schema):** still returns `Err`
    ///   so session demux can recover without catch_unwind.
    ///
    /// Signature therefore looks like [`Self::try_decode`], but short
    /// buffers do **not** yield `BufferTooShort` — they panic. Prefer
    /// [`Self::try_decode`] at untrusted boundaries when every failure
    /// must be a `Result`.
    #[inline]
    pub fn decode(buf: &'a [u8], offset: usize) -> Result<Self, sbe_rt::DecodeError> {
        let header_bytes: [u8; 8] = read_bytes::<8>(buf, offset);
        let header = MessageHeader(header_bytes);
        let template_id = sbe_rt::checked_header_u16(
            "templateId",
            header.template_id() as u64,
        )?;
        if template_id != Self::TEMPLATE_ID {
            return Err(sbe_rt::DecodeError::WrongTemplate {
                expected: Self::TEMPLATE_ID,
                actual: template_id,
                expected_name: "Car",
            });
        }
        let schema_id = sbe_rt::checked_header_u16(
            "schemaId",
            header.schema_id() as u64,
        )?;
        if schema_id != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: schema_id,
                expected_name: "baseline",
            });
        }
        let acting_block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let acting_version = sbe_rt::checked_header_u16(
            "version",
            header.version() as u64,
        )?;
        Ok(Self::wrap(buf, offset, acting_block_length, acting_version))
    }
    /// Unchecked **extent**, checked **identity**.
    ///
    /// Header/body bytes are read without bounds checks (**UB** if the
    /// caller has not proven the frame fits). Template/schema identity
    /// still returns `Err` (same hybrid policy as [`Self::decode`]).
    ///
    /// # Safety
    /// Header and version-readable fixed body for this template must
    /// be fully in-bounds at `offset`.
    #[inline]
    pub unsafe fn decode_unchecked(
        buf: &'a [u8],
        offset: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let header_bytes: [u8; 8] = unsafe { read_bytes_unchecked::<8>(buf, offset) };
        let header = MessageHeader(header_bytes);
        let template_id = sbe_rt::checked_header_u16(
            "templateId",
            header.template_id() as u64,
        )?;
        if template_id != Self::TEMPLATE_ID {
            return Err(sbe_rt::DecodeError::WrongTemplate {
                expected: Self::TEMPLATE_ID,
                actual: template_id,
                expected_name: "Car",
            });
        }
        let schema_id = sbe_rt::checked_header_u16(
            "schemaId",
            header.schema_id() as u64,
        )?;
        if schema_id != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: schema_id,
                expected_name: "baseline",
            });
        }
        let acting_block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let acting_version = sbe_rt::checked_header_u16(
            "version",
            header.version() as u64,
        )?;
        Ok(unsafe {
            Self::wrap_unchecked(buf, offset, acting_block_length, acting_version)
        })
    }
    /// Schema version from the message header (or wrap args), not the
    /// compiled schema constant. Fields with `sinceVersion` and optional
    /// presence depend on this value.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    /// Block length from the wire header / wrap args. Tail offsets use
    /// this acting length, not only the compiled `BLOCK_LENGTH`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
    ///Generated method `serial_number`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline(always)]
    pub fn serial_number(&self) -> u64 {
        u64::from_le_bytes(unsafe {
            read_bytes_unchecked::<8>(self.buf, self.offset + 0)
        })
    }
    ///`SERIAL_NUMBER_ID` = 1.
    pub const SERIAL_NUMBER_ID: u16 = 1;
    ///`SERIAL_NUMBER_SINCE_VERSION` = 0.
    pub const SERIAL_NUMBER_SINCE_VERSION: u16 = 0;
    ///`SERIAL_NUMBER_ENCODING_OFFSET` = 0.
    pub const SERIAL_NUMBER_ENCODING_OFFSET: usize = 0;
    ///`SERIAL_NUMBER_ENCODING_LENGTH` = 8.
    pub const SERIAL_NUMBER_ENCODING_LENGTH: usize = 8;
    ///Generated method `serial_number_meta_attribute`.
    #[inline]
    pub const fn serial_number_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`SERIAL_NUMBER_NULL` = 18446744073709551615.
    pub const SERIAL_NUMBER_NULL: u64 = 18446744073709551615_u64;
    ///`SERIAL_NUMBER_MIN` = 0.
    pub const SERIAL_NUMBER_MIN: u64 = 0_u64;
    ///`SERIAL_NUMBER_MAX` = 18446744073709551614.
    pub const SERIAL_NUMBER_MAX: u64 = 18446744073709551614_u64;
    ///Generated method `model_year`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline(always)]
    pub fn model_year(&self) -> u16 {
        u16::from_le_bytes(unsafe {
            read_bytes_unchecked::<2>(self.buf, self.offset + 8)
        })
    }
    ///`MODEL_YEAR_ID` = 2.
    pub const MODEL_YEAR_ID: u16 = 2;
    ///`MODEL_YEAR_SINCE_VERSION` = 0.
    pub const MODEL_YEAR_SINCE_VERSION: u16 = 0;
    ///`MODEL_YEAR_ENCODING_OFFSET` = 8.
    pub const MODEL_YEAR_ENCODING_OFFSET: usize = 8;
    ///`MODEL_YEAR_ENCODING_LENGTH` = 2.
    pub const MODEL_YEAR_ENCODING_LENGTH: usize = 2;
    ///Generated method `model_year_meta_attribute`.
    #[inline]
    pub const fn model_year_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`MODEL_YEAR_NULL` = 65535.
    pub const MODEL_YEAR_NULL: u16 = 65535_u16;
    ///`MODEL_YEAR_MIN` = 0.
    pub const MODEL_YEAR_MIN: u16 = 0_u16;
    ///`MODEL_YEAR_MAX` = 65534.
    pub const MODEL_YEAR_MAX: u16 = 65534_u16;
    ///Generated method `available`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn available(&self) -> BooleanType {
        BooleanType::from_raw(
            u8::from_le_bytes(unsafe {
                read_bytes_unchecked::<1>(self.buf, self.offset + 10)
            }),
        )
    }
    /// Raw wire discriminant — bypasses enum mapping.
    /// Use to inspect unknown/forward enum values without losing the original byte.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn raw_available(&self) -> u8 {
        u8::from_le_bytes(unsafe {
            read_bytes_unchecked::<1>(self.buf, self.offset + 10)
        })
    }
    /// Returns `true` / `false` for valid boolean values.
    /// Rejects `NullVal` or unknown raw discriminants —
    /// the SBE boolean wire type is tri-state (F, T, null).
    #[inline]
    pub fn try_available_bool(&self) -> Result<bool, sbe_rt::DecodeError> {
        self.available()
            .as_bool()
            .ok_or(sbe_rt::DecodeError::InvalidBoolean {
                field: stringify!(available),
                discriminant: self.raw_available() as u64,
            })
    }
    ///`AVAILABLE_ID` = 3.
    pub const AVAILABLE_ID: u16 = 3;
    ///`AVAILABLE_SINCE_VERSION` = 0.
    pub const AVAILABLE_SINCE_VERSION: u16 = 0;
    ///`AVAILABLE_ENCODING_OFFSET` = 10.
    pub const AVAILABLE_ENCODING_OFFSET: usize = 10;
    ///`AVAILABLE_ENCODING_LENGTH` = 1.
    pub const AVAILABLE_ENCODING_LENGTH: usize = 1;
    ///Generated method `available_meta_attribute`.
    #[inline]
    pub const fn available_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`AVAILABLE_NULL` = BooleanType::NullVal.
    pub const AVAILABLE_NULL: BooleanType = BooleanType::NullVal;
    ///Generated method `code`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn code(&self) -> Model {
        Model::from_raw(
            u8::from_le_bytes(unsafe {
                read_bytes_unchecked::<1>(self.buf, self.offset + 11)
            }),
        )
    }
    /// Raw wire discriminant — bypasses enum mapping.
    /// Use to inspect unknown/forward enum values without losing the original byte.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn raw_code(&self) -> u8 {
        u8::from_le_bytes(unsafe {
            read_bytes_unchecked::<1>(self.buf, self.offset + 11)
        })
    }
    ///`CODE_ID` = 4.
    pub const CODE_ID: u16 = 4;
    ///`CODE_SINCE_VERSION` = 0.
    pub const CODE_SINCE_VERSION: u16 = 0;
    ///`CODE_ENCODING_OFFSET` = 11.
    pub const CODE_ENCODING_OFFSET: usize = 11;
    ///`CODE_ENCODING_LENGTH` = 1.
    pub const CODE_ENCODING_LENGTH: usize = 1;
    ///Generated method `code_meta_attribute`.
    #[inline]
    pub const fn code_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`CODE_NULL` = Model::NullVal.
    pub const CODE_NULL: Model = Model::NullVal;
    ///Generated method `some_numbers`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn some_numbers(&self) -> [u32; 4] {
        if 28 > self.acting_block_length {
            return [0 as u32; 4];
        }
        let all: [u8; 16] = unsafe {
            read_bytes_unchecked::<16>(self.buf, self.offset + 12)
        };
        [
            u32::from_le_bytes([all[0usize], all[1usize], all[2usize], all[3usize]]),
            u32::from_le_bytes([all[4usize], all[5usize], all[6usize], all[7usize]]),
            u32::from_le_bytes([all[8usize], all[9usize], all[10usize], all[11usize]]),
            u32::from_le_bytes([all[12usize], all[13usize], all[14usize], all[15usize]]),
        ]
    }
    ///`SOME_NUMBERS_ID` = 5.
    pub const SOME_NUMBERS_ID: u16 = 5;
    ///`SOME_NUMBERS_SINCE_VERSION` = 0.
    pub const SOME_NUMBERS_SINCE_VERSION: u16 = 0;
    ///`SOME_NUMBERS_ENCODING_OFFSET` = 12.
    pub const SOME_NUMBERS_ENCODING_OFFSET: usize = 12;
    ///`SOME_NUMBERS_ENCODING_LENGTH` = 16.
    pub const SOME_NUMBERS_ENCODING_LENGTH: usize = 16;
    ///Generated method `some_numbers_meta_attribute`.
    #[inline]
    pub const fn some_numbers_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`SOME_NUMBERS_NULL` = 4294967295.
    pub const SOME_NUMBERS_NULL: u32 = 4294967295_u32;
    ///`SOME_NUMBERS_MIN` = 0.
    pub const SOME_NUMBERS_MIN: u32 = 0_u32;
    ///`SOME_NUMBERS_MAX` = 4294967294.
    pub const SOME_NUMBERS_MAX: u32 = 4294967294_u32;
    ///Generated method `vehicle_code`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn vehicle_code(&self) -> [u8; 6] {
        if 34 > self.acting_block_length {
            return [0 as u8; 6];
        }
        let all: [u8; 6] = unsafe {
            read_bytes_unchecked::<6>(self.buf, self.offset + 28)
        };
        all
    }
    ///Generated method `copy_vehicle_code`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn copy_vehicle_code(&self, dst: &mut [u8]) -> usize {
        let src = self.vehicle_code();
        let n = src.len().min(dst.len());
        let mut i = 0usize;
        while i < n {
            dst[i] = src[i] as u8;
            i += 1;
        }
        n
    }
    ///`VEHICLE_CODE_ID` = 6.
    pub const VEHICLE_CODE_ID: u16 = 6;
    ///`VEHICLE_CODE_SINCE_VERSION` = 0.
    pub const VEHICLE_CODE_SINCE_VERSION: u16 = 0;
    ///`VEHICLE_CODE_ENCODING_OFFSET` = 28.
    pub const VEHICLE_CODE_ENCODING_OFFSET: usize = 28;
    ///`VEHICLE_CODE_ENCODING_LENGTH` = 6.
    pub const VEHICLE_CODE_ENCODING_LENGTH: usize = 6;
    ///Generated method `vehicle_code_meta_attribute`.
    #[inline]
    pub const fn vehicle_code_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`VEHICLE_CODE_NULL` = 0.
    pub const VEHICLE_CODE_NULL: u8 = 0_u8;
    ///`VEHICLE_CODE_MIN` = 32.
    pub const VEHICLE_CODE_MIN: u8 = 32_u8;
    ///`VEHICLE_CODE_MAX` = 126.
    pub const VEHICLE_CODE_MAX: u8 = 126_u8;
    ///Generated method `extras`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn extras(&self) -> OptionalExtras {
        OptionalExtras(
            u8::from_le_bytes(unsafe {
                read_bytes_unchecked::<1>(self.buf, self.offset + 34)
            }),
        )
    }
    ///`EXTRAS_ID` = 7.
    pub const EXTRAS_ID: u16 = 7;
    ///`EXTRAS_SINCE_VERSION` = 0.
    pub const EXTRAS_SINCE_VERSION: u16 = 0;
    ///`EXTRAS_ENCODING_OFFSET` = 34.
    pub const EXTRAS_ENCODING_OFFSET: usize = 34;
    ///`EXTRAS_ENCODING_LENGTH` = 1.
    pub const EXTRAS_ENCODING_LENGTH: usize = 1;
    ///Generated method `extras_meta_attribute`.
    #[inline]
    pub const fn extras_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///Generated method `discounted_model`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn discounted_model(&self) -> Model {
        Model::C
    }
    ///`DISCOUNTED_MODEL_ID` = 8.
    pub const DISCOUNTED_MODEL_ID: u16 = 8;
    ///`DISCOUNTED_MODEL_SINCE_VERSION` = 0.
    pub const DISCOUNTED_MODEL_SINCE_VERSION: u16 = 0;
    ///`DISCOUNTED_MODEL_ENCODING_OFFSET` = 35.
    pub const DISCOUNTED_MODEL_ENCODING_OFFSET: usize = 35;
    ///`DISCOUNTED_MODEL_ENCODING_LENGTH` = 1.
    pub const DISCOUNTED_MODEL_ENCODING_LENGTH: usize = 1;
    ///Generated method `discounted_model_meta_attribute`.
    #[inline]
    pub const fn discounted_model_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("constant"),
        }
    }
    ///`DISCOUNTED_MODEL_NULL` = Model::NullVal.
    pub const DISCOUNTED_MODEL_NULL: Model = Model::NullVal;
    ///Generated method `engine`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn engine(&self) -> EngineDecoder<'_> {
        EngineDecoder {
            buf: self.buf,
            offset: self.offset + 35,
        }
    }
    ///Generated method `engine_value`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn engine_value(&self) -> Engine {
        Engine(unsafe { read_bytes_unchecked::<10>(self.buf, self.offset + 35) })
    }
    ///`ENGINE_ID` = 9.
    pub const ENGINE_ID: u16 = 9;
    ///`ENGINE_SINCE_VERSION` = 0.
    pub const ENGINE_SINCE_VERSION: u16 = 0;
    ///`ENGINE_ENCODING_OFFSET` = 35.
    pub const ENGINE_ENCODING_OFFSET: usize = 35;
    ///`ENGINE_ENCODING_LENGTH` = 10.
    pub const ENGINE_ENCODING_LENGTH: usize = 10;
    ///Generated method `engine_meta_attribute`.
    #[inline]
    pub const fn engine_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    /// Byte offset of the message body within `self.buf`.
    #[inline]
    fn byte_offset(&self) -> usize {
        self.offset
    }
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.byte_offset() + self.acting_block_length)
    }
    #[inline]
    fn walk_tail_0(&self, start: usize) -> Result<usize, sbe_rt::DecodeError> {
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_len = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let mut offset = start + 4;
        let mut idx = 0;
        while idx < count {
            offset = FuelFiguresEntryDecoder::skip(
                self.buf,
                offset,
                block_len,
                self.acting_version,
            )?;
            idx += 1;
        }
        Ok(offset)
    }
    #[inline]
    fn walk_tail_1(&self, start: usize) -> Result<usize, sbe_rt::DecodeError> {
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_len = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let mut offset = start + 4;
        let mut idx = 0;
        while idx < count {
            offset = PerformanceFiguresEntryDecoder::skip(
                self.buf,
                offset,
                block_len,
                self.acting_version,
            )?;
            idx += 1;
        }
        Ok(offset)
    }
    #[inline]
    fn walk_tail_2(&self, start: usize) -> Result<usize, sbe_rt::DecodeError> {
        if 4 > self.buf.len().saturating_sub(start) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "manufacturer",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarStringEncoding(bytes);
        let wire_length = header.length() as u64;
        let (_, data_end) = sbe_rt::checked_var_data_bounds(
            "manufacturer",
            start,
            4,
            wire_length,
            self.buf.len(),
        )?;
        Ok(data_end)
    }
    #[inline]
    fn walk_tail_3(&self, start: usize) -> Result<usize, sbe_rt::DecodeError> {
        if 4 > self.buf.len().saturating_sub(start) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "model",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarStringEncoding(bytes);
        let wire_length = header.length() as u64;
        let (_, data_end) = sbe_rt::checked_var_data_bounds(
            "model",
            start,
            4,
            wire_length,
            self.buf.len(),
        )?;
        Ok(data_end)
    }
    #[inline]
    fn walk_tail_4(&self, start: usize) -> Result<usize, sbe_rt::DecodeError> {
        if 4 > self.buf.len().saturating_sub(start) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "activationCode",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarAsciiEncoding(bytes);
        let wire_length = header.length() as u64;
        let (_, data_end) = sbe_rt::checked_var_data_bounds(
            "activationCode",
            start,
            4,
            wire_length,
            self.buf.len(),
        )?;
        Ok(data_end)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        self.walk_tail_0(start)
    }
    #[inline]
    fn tail_offset_2(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_1()?;
        self.walk_tail_1(start)
    }
    #[inline]
    fn tail_offset_3(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_2()?;
        self.walk_tail_2(start)
    }
    #[inline]
    fn tail_offset_4(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_3()?;
        self.walk_tail_3(start)
    }
    #[inline]
    fn tail_offset_5(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_4()?;
        self.walk_tail_4(start)
    }
    ///Generated method `fuel_figures`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn fuel_figures(&self) -> Result<FuelFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        FuelFiguresDecoder::wrap(self.buf, offset, self.acting_version)
    }
    ///Generated method `performance_figures`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn performance_figures(
        &self,
    ) -> Result<PerformanceFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        PerformanceFiguresDecoder::wrap(self.buf, offset, self.acting_version)
    }
    ///Generated method `manufacturer`.
    #[inline]
    pub fn manufacturer(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: stringify!(manufacturer),
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let header = VarStringEncoding(bytes);
        let wire_length = header.length() as u64;
        if wire_length > 1073741824 as u64 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(manufacturer),
                length: wire_length,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            stringify!(manufacturer),
            offset,
            4,
            wire_length,
            self.buf.len(),
        )?;
        Ok(&self.buf[data_start..data_end])
    }
    /// View this UTF-8 var-data field as `&str`.
    #[inline]
    pub fn manufacturer_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.manufacturer()?;
        core::str::from_utf8(bytes)
            .map_err(|e| sbe_rt::DecodeError::InvalidUtf8 {
                field: "manufacturer",
                error: e,
            })
    }
    /// View this text var-data field as `&str` without character
    /// encoding validation. Structural bounds are still checked.
    ///
    /// # Safety
    ///
    /// The wire bytes must be valid UTF-8.
    #[inline]
    pub unsafe fn manufacturer_as_str_unchecked(
        &self,
    ) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.manufacturer()?;
        Ok(unsafe { core::str::from_utf8_unchecked(bytes) })
    }
    ///Generated method `model`.
    #[inline]
    pub fn model(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_3()?;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: stringify!(model),
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let header = VarStringEncoding(bytes);
        let wire_length = header.length() as u64;
        if wire_length > 1073741824 as u64 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(model),
                length: wire_length,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            stringify!(model),
            offset,
            4,
            wire_length,
            self.buf.len(),
        )?;
        Ok(&self.buf[data_start..data_end])
    }
    /// View this UTF-8 var-data field as `&str`.
    #[inline]
    pub fn model_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.model()?;
        core::str::from_utf8(bytes)
            .map_err(|e| sbe_rt::DecodeError::InvalidUtf8 {
                field: "model",
                error: e,
            })
    }
    /// View this text var-data field as `&str` without character
    /// encoding validation. Structural bounds are still checked.
    ///
    /// # Safety
    ///
    /// The wire bytes must be valid UTF-8.
    #[inline]
    pub unsafe fn model_as_str_unchecked(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.model()?;
        Ok(unsafe { core::str::from_utf8_unchecked(bytes) })
    }
    ///Generated method `activation_code`.
    #[inline]
    pub fn activation_code(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_4()?;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: stringify!(activation_code),
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let header = VarAsciiEncoding(bytes);
        let wire_length = header.length() as u64;
        if wire_length > 1073741824 as u64 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(activation_code),
                length: wire_length,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            stringify!(activation_code),
            offset,
            4,
            wire_length,
            self.buf.len(),
        )?;
        Ok(&self.buf[data_start..data_end])
    }
    /// View this ASCII var-data field as `&str`.
    #[inline]
    pub fn activation_code_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.activation_code()?;
        if bytes.iter().any(|b| *b > 0x7F) {
            return Err(sbe_rt::DecodeError::InvalidAscii {
                field: "activation_code",
            });
        }
        Ok(unsafe { core::str::from_utf8_unchecked(bytes) })
    }
    /// View this text var-data field as `&str` without ASCII
    /// validation. Structural bounds remain fallible.
    ///
    /// # Safety
    ///
    /// The wire bytes must be 7-bit ASCII. For ASCII-declared
    /// fields from a trusted source this is always true.
    #[inline]
    pub unsafe fn activation_code_as_str_unchecked(
        &self,
    ) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.activation_code()?;
        Ok(unsafe { core::str::from_utf8_unchecked(bytes) })
    }
    /// Consume this stage and return a fresh decoder at the initial
    /// message position. The consumed stage cannot be reused.
    #[inline]
    pub fn rewind(self) -> Self {
        Self {
            buf: self.buf,
            offset: self.offset,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        }
    }
    ///Generated method `encoded_length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        let end = self.tail_offset_5()?;
        Ok(end - self.byte_offset())
    }
    ///Generated method `encoded_length_with_header`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
        let len = self.encoded_length()?;
        Ok(len + 8)
    }
    ///Generated method `as_body_bytes`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_body_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let end = self.tail_offset_5()?;
        let start = self.byte_offset();
        Ok(&self.buf[start..end])
    }
    ///Generated method `as_bytes_with_header`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_bytes_with_header(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let end = self.tail_offset_5()?;
        let start = self.byte_offset().saturating_sub(Self::HEADER_LENGTH);
        Ok(&self.buf[start..end])
    }
    ///Generated method `verify`.
    #[inline]
    pub fn verify(buf: &[u8]) -> Result<(), sbe_rt::VerifyError> {
        if buf.len() < 8 {
            return Err(sbe_rt::VerifyError::HeaderTooShort);
        }
        let header_bytes: [u8; 8] = read_bytes::<8>(buf, 0);
        let header = MessageHeader(header_bytes);
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        if block_length < Self::BLOCK_LENGTH {
            return Err(sbe_rt::VerifyError::InvalidBlockLength {
                expected_min: Self::BLOCK_LENGTH,
                actual: block_length,
            });
        }
        let body_end = (8 as usize)
            .checked_add(block_length)
            .ok_or(sbe_rt::VerifyError::MessageTooShort {
                needed: usize::MAX,
                available: buf.len(),
            })?;
        if body_end > buf.len() {
            return Err(sbe_rt::VerifyError::MessageTooShort {
                needed: body_end,
                available: buf.len(),
            });
        }
        let mut offset = body_end;
        {
            if offset + 4 > buf.len() {
                return Err(sbe_rt::VerifyError::GroupDimOutOfBounds {
                    field: "fuel_figures",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSizeEncoding(bytes);
            let count = match sbe_rt::checked_group_count(
                "numInGroup",
                dim.num_in_group() as u64,
            ) {
                Ok(count) => count,
                Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
            };
            let mut entry_offset = match offset.checked_add(4) {
                Some(v) => v,
                None => {
                    return Err(sbe_rt::VerifyError::GroupDimOutOfBounds {
                        field: "fuel_figures",
                        offset,
                    });
                }
            };
            for _ in 0..count {
                match FuelFiguresEntryDecoder::skip(buf, entry_offset, 6, 0) {
                    Ok(next) => entry_offset = next,
                    Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
                }
            }
            offset = entry_offset;
        }
        {
            if offset + 4 > buf.len() {
                return Err(sbe_rt::VerifyError::GroupDimOutOfBounds {
                    field: "performance_figures",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSizeEncoding(bytes);
            let count = match sbe_rt::checked_group_count(
                "numInGroup",
                dim.num_in_group() as u64,
            ) {
                Ok(count) => count,
                Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
            };
            let mut entry_offset = match offset.checked_add(4) {
                Some(v) => v,
                None => {
                    return Err(sbe_rt::VerifyError::GroupDimOutOfBounds {
                        field: "performance_figures",
                        offset,
                    });
                }
            };
            for _ in 0..count {
                match PerformanceFiguresEntryDecoder::skip(buf, entry_offset, 1, 0) {
                    Ok(next) => entry_offset = next,
                    Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
                }
            }
            offset = entry_offset;
        }
        {
            if 4 > buf.len().saturating_sub(offset) {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "manufacturer",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let var_header = VarStringEncoding(bytes);
            let len = var_header.length() as u64;
            let (_, data_end) = match sbe_rt::checked_var_data_bounds(
                "manufacturer",
                offset,
                4,
                len,
                buf.len(),
            ) {
                Ok(bounds) => bounds,
                Err(_) => {
                    return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                        field: "manufacturer",
                        offset,
                        length: len,
                    });
                }
            };
            offset = data_end;
        }
        {
            if 4 > buf.len().saturating_sub(offset) {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "model",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let var_header = VarStringEncoding(bytes);
            let len = var_header.length() as u64;
            let (_, data_end) = match sbe_rt::checked_var_data_bounds(
                "model",
                offset,
                4,
                len,
                buf.len(),
            ) {
                Ok(bounds) => bounds,
                Err(_) => {
                    return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                        field: "model",
                        offset,
                        length: len,
                    });
                }
            };
            offset = data_end;
        }
        {
            if 4 > buf.len().saturating_sub(offset) {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "activation_code",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let var_header = VarAsciiEncoding(bytes);
            let len = var_header.length() as u64;
            let (_, data_end) = match sbe_rt::checked_var_data_bounds(
                "activation_code",
                offset,
                4,
                len,
                buf.len(),
            ) {
                Ok(bounds) => bounds,
                Err(_) => {
                    return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                        field: "activation_code",
                        offset,
                        length: len,
                    });
                }
            };
            offset = data_end;
        }
        Ok(())
    }
}
impl<'a> TryFrom<&'a [u8]> for CarDecoder<'a> {
    type Error = sbe_rt::DecodeError;
    #[inline]
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_decode(buf, 0)
    }
}
impl<'a> __sbe_message_sealed::Sealed for CarDecoder<'a> {}
impl<'a> sbe_rt::SbeMessage for CarDecoder<'a> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 45;
    const SCHEMA_ID: u16 = 1;
    const SCHEMA_VERSION: u16 = 0;
}
impl<'a> core::fmt::Display for CarDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}
impl<'a> core::fmt::Debug for CarDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut d = f.debug_struct("CarDecoder");
        if self.byte_offset().saturating_add(8) <= self.buf.len()
            && 8 <= self.acting_block_length
        {
            let v = self.serial_number();
            d.field("serialNumber", &v);
        }
        if self.byte_offset().saturating_add(10) <= self.buf.len()
            && 10 <= self.acting_block_length
        {
            let v = self.model_year();
            d.field("modelYear", &v);
        }
        if self.byte_offset().saturating_add(11) <= self.buf.len()
            && 11 <= self.acting_block_length
        {
            let v = self.available();
            d.field("available", &v);
        }
        if self.byte_offset().saturating_add(12) <= self.buf.len()
            && 12 <= self.acting_block_length
        {
            let v = self.code();
            d.field("code", &v);
        }
        if self.byte_offset().saturating_add(35) <= self.buf.len()
            && 35 <= self.acting_block_length
        {
            let v = self.extras();
            d.field("extras", &format_args!("{}", v));
        }
        if self.byte_offset().saturating_add(45) <= self.buf.len()
            && 45 <= self.acting_block_length
        {
            let v = self.engine_value();
            d.field("engine", &v);
        }
        if let Ok(_g) = self.fuel_figures() {
            let entries: Vec<String> = _g
                .filter_map(|r| r.ok())
                .map(|e| format!("{e}"))
                .collect();
            d.field("fuelFigures", &entries);
        }
        if let Ok(_g) = self.performance_figures() {
            let entries: Vec<String> = _g
                .filter_map(|r| r.ok())
                .map(|e| format!("{e}"))
                .collect();
            d.field("performanceFigures", &entries);
        }
        if let Ok(_data) = self.manufacturer() {
            match std::str::from_utf8(_data) {
                Ok(_s) => d.field("manufacturer", &_s),
                Err(_) => d.field("manufacturer", &format!("<{} bytes>", _data.len())),
            };
        }
        if let Ok(_data) = self.model() {
            match std::str::from_utf8(_data) {
                Ok(_s) => d.field("model", &_s),
                Err(_) => d.field("model", &format!("<{} bytes>", _data.len())),
            };
        }
        if let Ok(_data) = self.activation_code() {
            match std::str::from_utf8(_data) {
                Ok(_s) => d.field("activationCode", &_s),
                Err(_) => d.field("activationCode", &format!("<{} bytes>", _data.len())),
            };
        }
        d.finish()
    }
}
#[doc = concat!(
    "Group `", stringify!(FuelFiguresDecoder),
    "` decoder — iterate entries in wire order."
)]
/// This group has entries with nested groups or var-data —
/// there is no constant stride, so `entry_at` (O(1) random
/// access) is **not** available. Use the [`Iterator`]
/// implementation, [`Self::scan_entry_at`], or
/// [`Self::skip_n`] to advance positionally instead.
pub struct FuelFiguresDecoder<'a, C: sbe_rt::GroupContext = sbe_rt::Detached> {
    buf: &'a [u8],
    offset: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
    acting_block_length: usize,
    parent_pos: usize,
    parent_block_length: usize,
    poisoned: Option<sbe_rt::DecodeError>,
    min_entry_extent: usize,
    _context: core::marker::PhantomData<C>,
}
impl<'a, C: sbe_rt::GroupContext> FuelFiguresDecoder<'a, C> {
    /// Proof-dependent constructor: like `wrap()` but remembers the
    /// parent message body position and acting block length so
    /// `finish()` can rebuild the next stage.
    ///
    /// Private to the generated module — a caller outside it cannot
    /// invent parent state and then `finish()` into a message stage
    /// that never existed.
    ///
    /// # Safety
    /// `parent_pos` and `parent_block_length` must describe the message
    /// body this group is genuinely nested in, and `offset` must be that
    /// message's real dimension-header offset for this group. The
    /// dimension header, the acting block length, and the group extent
    /// are still validated here and may be untrusted.
    #[inline]
    unsafe fn wrap_with_parent(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Result<FuelFiguresDecoder<'a, sbe_rt::Attached>, sbe_rt::DecodeError> {
        if 4 > buf.len().saturating_sub(offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: 4,
                available: buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let entries_start = offset + 4;
        let min_fixed = <FuelFiguresDecoder<
            '_,
            sbe_rt::Detached,
        >>::min_readable_fixed_extent(acting_version);
        if count > 0 && block_length < min_fixed {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: min_fixed,
                available: block_length,
            });
        }
        let min_entry_extent = if block_length > min_fixed {
            block_length
        } else {
            min_fixed
        };
        Ok(FuelFiguresDecoder {
            buf,
            offset: entries_start,
            count,
            start: entries_start,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
            poisoned: None,
            min_entry_extent,
            _context: core::marker::PhantomData,
        })
    }
    /// Attached decoder for a group that is not in the acting version:
    /// zero entries, zero bytes, immediately complete.
    ///
    /// # Safety
    /// `parent_pos` and `parent_block_length` must describe the message
    /// body this group is nested in, and `offset` must be the byte
    /// position where this group would have started had it been present.
    #[inline]
    unsafe fn wrap_absent_parent(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> FuelFiguresDecoder<'a, sbe_rt::Attached> {
        FuelFiguresDecoder {
            buf,
            offset,
            count: 0,
            start: offset,
            total: 0,
            acting_version,
            acting_block_length: 0,
            parent_pos,
            parent_block_length,
            poisoned: None,
            min_entry_extent: 0,
            _context: core::marker::PhantomData,
        }
    }
    ///Generated method `is_empty`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    /// Wire-declared entries not yet consumed.
    ///
    /// O(1): `into_*` already read the SBE dimension header containing
    /// `numInGroup`. This does not promise that remaining entries will
    /// decode, so dynamic groups are not [`core::iter::ExactSizeIterator`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn remaining_entries(&self) -> usize {
        self.count
    }
}
impl<'a> FuelFiguresDecoder<'a, sbe_rt::Detached> {
    ///`ENTRY_BLOCK_LENGTH` = 6.
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    /// Minimum entry bytes needed to safely read every **required**
    /// fixed field present at `acting_version`.
    ///
    /// Version-aware, and not always the compiled
    /// `ENTRY_BLOCK_LENGTH`: a forward-compatible reader accepts a
    /// wire block length it does not recognise, but never one too
    /// small for the fields it will actually read.
    #[must_use = "this extent is the minimum readable body size; ignoring it skips a bounds check"]
    #[inline]
    pub const fn min_readable_fixed_extent(acting_version: u16) -> usize {
        let mut m = 6;
        m
    }
    /// Wrap a standalone group at its dimension header, with bounds
    /// checks.
    ///
    /// This is the only public constructor. It validates the dimension
    /// header, rejects a wire block length too small to hold the
    /// required fixed fields active at `acting_version`, and — for
    /// fixed-stride groups — proves the whole entry region at once.
    ///
    /// The result is *detached*: it iterates, random-accesses, and
    /// rewinds, but has no parent message to complete into, so it has
    /// no `finish` / `skip_remaining`.
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
    ) -> Result<FuelFiguresDecoder<'a, sbe_rt::Detached>, sbe_rt::DecodeError> {
        let attached = unsafe {
            <FuelFiguresDecoder<
                'a,
                sbe_rt::Attached,
            >>::wrap_with_parent(buf, offset, acting_version, 0, 0)?
        };
        Ok(FuelFiguresDecoder {
            buf: attached.buf,
            offset: attached.offset,
            count: attached.count,
            start: attached.start,
            total: attached.total,
            acting_version: attached.acting_version,
            acting_block_length: attached.acting_block_length,
            parent_pos: attached.parent_pos,
            parent_block_length: attached.parent_block_length,
            poisoned: None,
            min_entry_extent: attached.min_entry_extent,
            _context: core::marker::PhantomData,
        })
    }
}
impl<'a, C: sbe_rt::GroupContext> FuelFiguresDecoder<'a, C> {
    /// Entries not yet advanced (count), not a byte slice.
    /// For message-level byte tails use `get_metadata().remaining()`.
    /// Prefer [`Self::remaining_entries`] at call sites that mean
    /// group cardinality rather than a byte tail.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.remaining_entries()
    }
    /// Dimension wrap after the caller has proven
    /// the dimension header (and, for fixed groups, the full entry
    /// region) is in-bounds. Prefer [`Self::wrap`] / [`Self::wrap_with_parent`].
    ///
    /// # Safety
    /// `offset + dimension_header_size` must not overflow and must be
    /// ≤ `buf.len()`. For fixed-block groups (no nested tail),
    /// `offset + dim + count * acting_block_length` must also fit. Entry
    /// accessors then use unchecked fixed-field reads under that proof.
    #[inline]
    pub(crate) unsafe fn wrap_trusted(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = unsafe { read_bytes_unchecked::<4>(buf, offset) };
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let min_fixed = <FuelFiguresDecoder<
            '_,
            sbe_rt::Detached,
        >>::min_readable_fixed_extent(acting_version);
        let min_entry_extent = if block_length > min_fixed {
            block_length
        } else {
            min_fixed
        };
        Ok(Self {
            buf,
            offset: offset + 4,
            count,
            start: offset + 4,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
            poisoned: None,
            min_entry_extent,
            _context: core::marker::PhantomData,
        })
    }
    /// Restart iteration from the group's proven start.
    ///
    /// This is the one operation that clears a poisoned group: the
    /// start offset was validated at wrap time, so retrying from there
    /// is sound even after an entry failed.
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        self.offset = self.start;
        self.count = self.total;
        self.poisoned = None;
        self
    }
}
impl<'a, C: sbe_rt::GroupContext> FuelFiguresDecoder<'a, C> {
    ///Generated method `skip_n`.
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: n
                    .saturating_mul(
                        <FuelFiguresDecoder<'_, sbe_rt::Detached>>::ENTRY_BLOCK_LENGTH,
                    ),
                available: self
                    .count
                    .saturating_mul(
                        <FuelFiguresDecoder<'_, sbe_rt::Detached>>::ENTRY_BLOCK_LENGTH,
                    ),
            });
        }
        for _ in 0..n {
            let __available = self.buf.len().saturating_sub(self.offset);
            if self.min_entry_extent > __available {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "fuelFigures",
                    needed: self.min_entry_extent,
                    available: __available,
                });
            }
            let entry = unsafe {
                FuelFiguresEntryDecoder::wrap(
                    self.buf,
                    self.offset,
                    self.acting_block_length,
                    self.acting_version,
                )
            };
            self.offset += entry.encoded_length()?;
            self.count -= 1;
        }
        Ok(())
    }
}
impl<'a, C: sbe_rt::GroupContext> FuelFiguresDecoder<'a, C> {
    ///Generated method `scan_entry_at`.
    #[inline]
    pub fn scan_entry_at(
        &self,
        idx: usize,
    ) -> Result<FuelFiguresEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: idx
                    .saturating_add(1)
                    .saturating_mul(
                        <FuelFiguresDecoder<'_, sbe_rt::Detached>>::ENTRY_BLOCK_LENGTH,
                    ),
                available: self
                    .total
                    .saturating_mul(
                        <FuelFiguresDecoder<'_, sbe_rt::Detached>>::ENTRY_BLOCK_LENGTH,
                    ),
            });
        }
        let mut offset = self.start;
        for _ in 0..idx {
            offset = FuelFiguresEntryDecoder::skip(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            )?;
        }
        let available = self.buf.len().saturating_sub(offset);
        if self.min_entry_extent > available {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: self.min_entry_extent,
                available,
            });
        }
        let entry = unsafe {
            FuelFiguresEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            )
        };
        entry.encoded_length()?;
        Ok(entry)
    }
}
impl<'a, C: sbe_rt::GroupContext> Iterator for FuelFiguresDecoder<'a, C> {
    type Item = Result<FuelFiguresEntryDecoder<'a>, sbe_rt::DecodeError>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.poisoned.is_some() || self.count == 0 {
            return None;
        }
        let available = self.buf.len().saturating_sub(self.offset);
        if self.min_entry_extent > available {
            let error = sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: self.min_entry_extent,
                available,
            };
            self.poisoned = Some(error);
            self.count = 0;
            return Some(Err(error));
        }
        let entry = unsafe {
            FuelFiguresEntryDecoder::wrap(
                self.buf,
                self.offset,
                self.acting_block_length,
                self.acting_version,
            )
        };
        let size = match entry.encoded_length() {
            Ok(s) => s,
            Err(e) => {
                self.poisoned = Some(e);
                self.count = 0;
                return Some(Err(e));
            }
        };
        self.offset += size;
        self.count -= 1;
        Some(Ok(entry))
    }
    /// Conservative: the declared count is an upper bound, but a
    /// malformed entry can end iteration early, so the lower bound
    /// is zero. This group is deliberately **not**
    /// `ExactSizeIterator` — a size-based allocation must not trust
    /// a count the wire has not yet justified.
    ///
    /// Poisoning zeroes the count, so this collapses to `(0, Some(0))`
    /// on a broken group without a separate branch.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.count))
    }
}
/// Exhausted by count or by poison, `next()` keeps returning `None`.
///
/// [`Self::rewind`] is the documented exception: it is not `next`,
/// and it deliberately restarts a *new* iteration from the start
/// offset proven at wrap time. Do not call it partway through an
/// adaptor that has cached this fuse.
impl<'a, C: sbe_rt::GroupContext> core::iter::FusedIterator
for FuelFiguresDecoder<'a, C> {}
#[doc = concat!(
    "Entry decoder for the `", stringify!(FuelFiguresEntryDecoder),
    "` group — access fixed fields and var-data for one entry."
)]
pub struct FuelFiguresEntryDecoder<'a> {
    buf: &'a [u8],
    offset: usize,
    acting_version: u16,
    acting_block_length: usize,
    /// One-shot entry-extent cache: filled by `encoded_length`,
    /// reused by the last var-data accessor. `Cell` keeps `&self`
    /// getters and makes the entry `Send` + `!Sync`.
    tail_end: core::cell::Cell<Option<usize>>,
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    ///`ENTRY_BLOCK_LENGTH` = 6.
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    /// Private entry wrap after the group iterator has proven extents.
    ///
    /// # Safety
    /// Fixed block at `offset` and every dynamic tail extent this entry
    /// will traverse must be fully in-bounds in `buf`.
    #[inline]
    unsafe fn wrap(
        buf: &'a [u8],
        offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        Self {
            buf,
            offset,
            acting_version,
            acting_block_length,
            tail_end: core::cell::Cell::new(None),
        }
    }
    ///Generated method `speed`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn speed(&self) -> u16 {
        let offset = self.offset + 0;
        u16::from_le_bytes(unsafe { read_bytes_unchecked::<2>(self.buf, offset) })
    }
    ///`SPEED_ID` = 11.
    pub const SPEED_ID: u16 = 11;
    ///`SPEED_SINCE_VERSION` = 0.
    pub const SPEED_SINCE_VERSION: u16 = 0;
    ///`SPEED_ENCODING_OFFSET` = 0.
    pub const SPEED_ENCODING_OFFSET: usize = 0;
    ///`SPEED_ENCODING_LENGTH` = 2.
    pub const SPEED_ENCODING_LENGTH: usize = 2;
    ///Generated method `speed_meta_attribute`.
    #[inline]
    pub const fn speed_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`SPEED_NULL` = 65535.
    pub const SPEED_NULL: u16 = 65535_u16;
    ///`SPEED_MIN` = 0.
    pub const SPEED_MIN: u16 = 0_u16;
    ///`SPEED_MAX` = 65534.
    pub const SPEED_MAX: u16 = 65534_u16;
    ///Generated method `mpg`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn mpg(&self) -> f32 {
        let offset = self.offset + 2;
        f32::from_le_bytes(unsafe { read_bytes_unchecked::<4>(self.buf, offset) })
    }
    ///`MPG_ID` = 12.
    pub const MPG_ID: u16 = 12;
    ///`MPG_SINCE_VERSION` = 0.
    pub const MPG_SINCE_VERSION: u16 = 0;
    ///`MPG_ENCODING_OFFSET` = 2.
    pub const MPG_ENCODING_OFFSET: usize = 2;
    ///`MPG_ENCODING_LENGTH` = 4.
    pub const MPG_ENCODING_LENGTH: usize = 4;
    ///Generated method `mpg_meta_attribute`.
    #[inline]
    pub const fn mpg_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///Generated constant `MPG_NULL`.
    pub const MPG_NULL: f32 = f32::from_bits(2143289344u32);
    ///Generated constant `MPG_MIN`.
    pub const MPG_MIN: f32 = f32::from_bits(4286578687u32);
    ///Generated constant `MPG_MAX`.
    pub const MPG_MAX: f32 = f32::from_bits(2139095039u32);
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        if self.acting_block_length > self.buf.len().saturating_sub(self.offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: self.acting_block_length,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        Ok(self.offset + self.acting_block_length)
    }
    #[inline]
    fn walk_tail_0(&self, start: usize) -> Result<usize, sbe_rt::DecodeError> {
        if 4 > self.buf.len().saturating_sub(start) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "usageDescription",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarAsciiEncoding(bytes);
        let wire_length = header.length() as u64;
        let (_, data_end) = sbe_rt::checked_var_data_bounds(
            "usageDescription",
            start,
            4,
            wire_length,
            self.buf.len(),
        )?;
        Ok(data_end)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        self.walk_tail_0(start)
    }
    ///Generated method `usage_description`.
    #[inline]
    pub fn usage_description(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        if let Some(end) = self.tail_end.get() {
            let data_offset = self.offset + self.acting_block_length + 4;
            return Ok(unsafe { self.buf.get_unchecked(data_offset..end) });
        }
        let offset = self.tail_offset_0()?;
        if let Some(end) = self.tail_end.get() {
            let data_offset = offset
                .checked_add(4)
                .ok_or(sbe_rt::DecodeError::BufferTooShort {
                    field: stringify!(usage_description),
                    needed: usize::MAX,
                    available: self.buf.len().saturating_sub(offset),
                })?;
            return Ok(unsafe { self.buf.get_unchecked(data_offset..end) });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarAsciiEncoding(bytes);
        let wire_length = header.length() as u64;
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            stringify!(usage_description),
            offset,
            4,
            wire_length,
            self.buf.len(),
        )?;
        Ok(&self.buf[data_start..data_end])
    }
    ///Generated method `encoded_length`.
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        if let Some(end) = self.tail_end.get() {
            return Ok(end - self.offset);
        }
        let end = self.tail_offset_1()?;
        self.tail_end.set(Some(end));
        Ok(end - self.offset)
    }
    ///Generated method `skip`.
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        offset: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        if block_len > buf.len().saturating_sub(offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: block_len,
                available: buf.len().saturating_sub(offset),
            });
        }
        let entry = unsafe { Self::wrap(buf, offset, block_len, acting_version) };
        entry.tail_offset_1()
    }
}
impl<'a> core::fmt::Display for FuelFiguresEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.speed();
            write!(f, "speed: {:?}", v)?;
        }
        {
            let v = self.mpg();
            write!(f, ", mpg: {:?}", v)?;
        }
        if let Ok(d) = self.usage_description() {
            match std::str::from_utf8(d) {
                Ok(s) => write!(f, ", usageDescription: {}", s)?,
                Err(_) => write!(f, ", usageDescription: <{} bytes>", d.len())?,
            }
        }
        write!(f, " }}")
    }
}
/// Consuming decoder stage — drop without `into_*` / `finish` skips
/// remaining wire tails.
#[must_use = "decoder stage must be advanced with into_*/finish or tails are skipped"]
pub struct FuelFiguresEntryDecoderComplete<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) offset: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
impl<'a> FuelFiguresEntryDecoderComplete<'a> {
    /// Schema version from the message header (or wrap args), not the
    /// compiled schema constant. Fields with `sinceVersion` and optional
    /// presence depend on this value.
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    /// Block length from the wire header / wrap args. Tail offsets use
    /// this acting length, not only the compiled `BLOCK_LENGTH`.
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_usage_description(
        self,
    ) -> Result<(&'a [u8], FuelFiguresEntryDecoderComplete<'a>), sbe_rt::DecodeError> {
        let offset = self.offset + self.acting_block_length;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "usageDescription",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let len = u32::from_le_bytes(bytes) as u64;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "usageDescription",
                length: len,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            "usageDescription",
            offset,
            4,
            len,
            self.buf.len(),
        )?;
        let data = &self.buf[data_start..data_end];
        let next = FuelFiguresEntryDecoderComplete {
            buf: self.buf,
            offset: self.offset,
            tail_start: data_end,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
    ///Non-consuming variant: read this var-data field as `&[u8]` without advancing or constructing the next stage.
    ///
    ///Cheaper than [`Self::into_usage_description`] when only the bytes are needed.
    #[inline]
    pub fn usage_description_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.offset + self.acting_block_length;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "usageDescription",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let len = u32::from_le_bytes(bytes) as u64;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "usageDescription",
                length: len,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            "usageDescription",
            offset,
            4,
            len,
            self.buf.len(),
        )?;
        Ok(&self.buf[data_start..data_end])
    }
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    /// Consume this stage, read the next ASCII var-data
    /// field as a validated `&str`, and advance.
    #[inline]
    pub fn into_usage_description_as_str(
        self,
    ) -> Result<(&'a str, FuelFiguresEntryDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_usage_description()?;
        if !bytes.is_ascii() {
            return Err(sbe_rt::DecodeError::InvalidAscii {
                field: "usageDescription",
            });
        }
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        Ok((s, next))
    }
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    /// Consume this stage, read the next text var-data field as
    /// a `&str` without encoding validation, and advance.
    ///
    /// Structural bounds (truncated payload, overflowing length)
    /// remain fallible — only character validation is skipped.
    ///
    /// # Safety
    /// The wire bytes must be valid for the schema-declared
    /// character encoding (UTF-8 or ASCII).
    #[inline]
    pub unsafe fn into_usage_description_as_str_unchecked(
        self,
    ) -> Result<(&'a str, FuelFiguresEntryDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_usage_description()?;
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        Ok((s, next))
    }
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_usage_description<E, F>(
        self,
        f: F,
    ) -> Result<FuelFiguresEntryDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_usage_description()?;
        f(data)?;
        Ok(next)
    }
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_usage_description_as_message(
        self,
    ) -> Result<
        (DecodedFrame<'a>, FuelFiguresEntryDecoderComplete<'a>),
        sbe_rt::DecodeError,
    > {
        let (data, next) = self.into_usage_description()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
    /// Fallible scoped nested-message accessor.
    #[inline]
    pub fn try_usage_description_as_message<E, F>(
        self,
        f: F,
    ) -> Result<FuelFiguresEntryDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_usage_description_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> FuelFiguresEntryDecoderComplete<'a> {
    /// Body bytes (excluding the message header; for entries this is the
    /// complete entry bytes).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_body_bytes(&self) -> &'a [u8] {
        &self.buf[self.offset..self.tail_start]
    }
    /// Complete SBE frame (header + body) for message stages.
    /// For entry stages (`HEADER_LENGTH == 0`) this equals [`Self::as_body_bytes`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_bytes_with_header(&self) -> &'a [u8] {
        &self.buf[self.offset - 0..self.tail_start]
    }
    /// Body length (excluding header).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.tail_start - self.offset
    }
    /// Total message length including the schema-declared header.
    /// Pure arithmetic: body length + `HEADER_LENGTH`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.tail_start - self.offset + 0
    }
    /// Bytes after this message/entry.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.tail_start..]
    }
}
#[doc = concat!(
    "Group `", stringify!(PerformanceFiguresDecoder),
    "` decoder — iterate entries in wire order."
)]
/// This group has entries with nested groups or var-data —
/// there is no constant stride, so `entry_at` (O(1) random
/// access) is **not** available. Use the [`Iterator`]
/// implementation, [`Self::scan_entry_at`], or
/// [`Self::skip_n`] to advance positionally instead.
pub struct PerformanceFiguresDecoder<'a, C: sbe_rt::GroupContext = sbe_rt::Detached> {
    buf: &'a [u8],
    offset: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
    acting_block_length: usize,
    parent_pos: usize,
    parent_block_length: usize,
    poisoned: Option<sbe_rt::DecodeError>,
    min_entry_extent: usize,
    _context: core::marker::PhantomData<C>,
}
impl<'a, C: sbe_rt::GroupContext> PerformanceFiguresDecoder<'a, C> {
    /// Proof-dependent constructor: like `wrap()` but remembers the
    /// parent message body position and acting block length so
    /// `finish()` can rebuild the next stage.
    ///
    /// Private to the generated module — a caller outside it cannot
    /// invent parent state and then `finish()` into a message stage
    /// that never existed.
    ///
    /// # Safety
    /// `parent_pos` and `parent_block_length` must describe the message
    /// body this group is genuinely nested in, and `offset` must be that
    /// message's real dimension-header offset for this group. The
    /// dimension header, the acting block length, and the group extent
    /// are still validated here and may be untrusted.
    #[inline]
    unsafe fn wrap_with_parent(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Result<PerformanceFiguresDecoder<'a, sbe_rt::Attached>, sbe_rt::DecodeError> {
        if 4 > buf.len().saturating_sub(offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: 4,
                available: buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let entries_start = offset + 4;
        let min_fixed = <PerformanceFiguresDecoder<
            '_,
            sbe_rt::Detached,
        >>::min_readable_fixed_extent(acting_version);
        if count > 0 && block_length < min_fixed {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: min_fixed,
                available: block_length,
            });
        }
        let min_entry_extent = if block_length > min_fixed {
            block_length
        } else {
            min_fixed
        };
        Ok(PerformanceFiguresDecoder {
            buf,
            offset: entries_start,
            count,
            start: entries_start,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
            poisoned: None,
            min_entry_extent,
            _context: core::marker::PhantomData,
        })
    }
    /// Attached decoder for a group that is not in the acting version:
    /// zero entries, zero bytes, immediately complete.
    ///
    /// # Safety
    /// `parent_pos` and `parent_block_length` must describe the message
    /// body this group is nested in, and `offset` must be the byte
    /// position where this group would have started had it been present.
    #[inline]
    unsafe fn wrap_absent_parent(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> PerformanceFiguresDecoder<'a, sbe_rt::Attached> {
        PerformanceFiguresDecoder {
            buf,
            offset,
            count: 0,
            start: offset,
            total: 0,
            acting_version,
            acting_block_length: 0,
            parent_pos,
            parent_block_length,
            poisoned: None,
            min_entry_extent: 0,
            _context: core::marker::PhantomData,
        }
    }
    ///Generated method `is_empty`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    /// Wire-declared entries not yet consumed.
    ///
    /// O(1): `into_*` already read the SBE dimension header containing
    /// `numInGroup`. This does not promise that remaining entries will
    /// decode, so dynamic groups are not [`core::iter::ExactSizeIterator`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn remaining_entries(&self) -> usize {
        self.count
    }
}
impl<'a> PerformanceFiguresDecoder<'a, sbe_rt::Detached> {
    ///`ENTRY_BLOCK_LENGTH` = 1.
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    /// Minimum entry bytes needed to safely read every **required**
    /// fixed field present at `acting_version`.
    ///
    /// Version-aware, and not always the compiled
    /// `ENTRY_BLOCK_LENGTH`: a forward-compatible reader accepts a
    /// wire block length it does not recognise, but never one too
    /// small for the fields it will actually read.
    #[must_use = "this extent is the minimum readable body size; ignoring it skips a bounds check"]
    #[inline]
    pub const fn min_readable_fixed_extent(acting_version: u16) -> usize {
        let mut m = 1;
        m
    }
    /// Wrap a standalone group at its dimension header, with bounds
    /// checks.
    ///
    /// This is the only public constructor. It validates the dimension
    /// header, rejects a wire block length too small to hold the
    /// required fixed fields active at `acting_version`, and — for
    /// fixed-stride groups — proves the whole entry region at once.
    ///
    /// The result is *detached*: it iterates, random-accesses, and
    /// rewinds, but has no parent message to complete into, so it has
    /// no `finish` / `skip_remaining`.
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
    ) -> Result<PerformanceFiguresDecoder<'a, sbe_rt::Detached>, sbe_rt::DecodeError> {
        let attached = unsafe {
            <PerformanceFiguresDecoder<
                'a,
                sbe_rt::Attached,
            >>::wrap_with_parent(buf, offset, acting_version, 0, 0)?
        };
        Ok(PerformanceFiguresDecoder {
            buf: attached.buf,
            offset: attached.offset,
            count: attached.count,
            start: attached.start,
            total: attached.total,
            acting_version: attached.acting_version,
            acting_block_length: attached.acting_block_length,
            parent_pos: attached.parent_pos,
            parent_block_length: attached.parent_block_length,
            poisoned: None,
            min_entry_extent: attached.min_entry_extent,
            _context: core::marker::PhantomData,
        })
    }
}
impl<'a, C: sbe_rt::GroupContext> PerformanceFiguresDecoder<'a, C> {
    /// Entries not yet advanced (count), not a byte slice.
    /// For message-level byte tails use `get_metadata().remaining()`.
    /// Prefer [`Self::remaining_entries`] at call sites that mean
    /// group cardinality rather than a byte tail.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.remaining_entries()
    }
    /// Dimension wrap after the caller has proven
    /// the dimension header (and, for fixed groups, the full entry
    /// region) is in-bounds. Prefer [`Self::wrap`] / [`Self::wrap_with_parent`].
    ///
    /// # Safety
    /// `offset + dimension_header_size` must not overflow and must be
    /// ≤ `buf.len()`. For fixed-block groups (no nested tail),
    /// `offset + dim + count * acting_block_length` must also fit. Entry
    /// accessors then use unchecked fixed-field reads under that proof.
    #[inline]
    pub(crate) unsafe fn wrap_trusted(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = unsafe { read_bytes_unchecked::<4>(buf, offset) };
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let min_fixed = <PerformanceFiguresDecoder<
            '_,
            sbe_rt::Detached,
        >>::min_readable_fixed_extent(acting_version);
        let min_entry_extent = if block_length > min_fixed {
            block_length
        } else {
            min_fixed
        };
        Ok(Self {
            buf,
            offset: offset + 4,
            count,
            start: offset + 4,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
            poisoned: None,
            min_entry_extent,
            _context: core::marker::PhantomData,
        })
    }
    /// Restart iteration from the group's proven start.
    ///
    /// This is the one operation that clears a poisoned group: the
    /// start offset was validated at wrap time, so retrying from there
    /// is sound even after an entry failed.
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        self.offset = self.start;
        self.count = self.total;
        self.poisoned = None;
        self
    }
}
impl<'a, C: sbe_rt::GroupContext> PerformanceFiguresDecoder<'a, C> {
    ///Generated method `skip_n`.
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: n
                    .saturating_mul(
                        <PerformanceFiguresDecoder<
                            '_,
                            sbe_rt::Detached,
                        >>::ENTRY_BLOCK_LENGTH,
                    ),
                available: self
                    .count
                    .saturating_mul(
                        <PerformanceFiguresDecoder<
                            '_,
                            sbe_rt::Detached,
                        >>::ENTRY_BLOCK_LENGTH,
                    ),
            });
        }
        for _ in 0..n {
            let __available = self.buf.len().saturating_sub(self.offset);
            if self.min_entry_extent > __available {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "performanceFigures",
                    needed: self.min_entry_extent,
                    available: __available,
                });
            }
            let entry = unsafe {
                PerformanceFiguresEntryDecoder::wrap(
                    self.buf,
                    self.offset,
                    self.acting_block_length,
                    self.acting_version,
                )
            };
            self.offset += entry.encoded_length()?;
            self.count -= 1;
        }
        Ok(())
    }
}
impl<'a, C: sbe_rt::GroupContext> PerformanceFiguresDecoder<'a, C> {
    ///Generated method `scan_entry_at`.
    #[inline]
    pub fn scan_entry_at(
        &self,
        idx: usize,
    ) -> Result<PerformanceFiguresEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: idx
                    .saturating_add(1)
                    .saturating_mul(
                        <PerformanceFiguresDecoder<
                            '_,
                            sbe_rt::Detached,
                        >>::ENTRY_BLOCK_LENGTH,
                    ),
                available: self
                    .total
                    .saturating_mul(
                        <PerformanceFiguresDecoder<
                            '_,
                            sbe_rt::Detached,
                        >>::ENTRY_BLOCK_LENGTH,
                    ),
            });
        }
        let mut offset = self.start;
        for _ in 0..idx {
            offset = PerformanceFiguresEntryDecoder::skip(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            )?;
        }
        let available = self.buf.len().saturating_sub(offset);
        if self.min_entry_extent > available {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: self.min_entry_extent,
                available,
            });
        }
        let entry = unsafe {
            PerformanceFiguresEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            )
        };
        entry.encoded_length()?;
        Ok(entry)
    }
}
impl<'a, C: sbe_rt::GroupContext> Iterator for PerformanceFiguresDecoder<'a, C> {
    type Item = Result<PerformanceFiguresEntryDecoder<'a>, sbe_rt::DecodeError>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.poisoned.is_some() || self.count == 0 {
            return None;
        }
        let available = self.buf.len().saturating_sub(self.offset);
        if self.min_entry_extent > available {
            let error = sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: self.min_entry_extent,
                available,
            };
            self.poisoned = Some(error);
            self.count = 0;
            return Some(Err(error));
        }
        let entry = unsafe {
            PerformanceFiguresEntryDecoder::wrap(
                self.buf,
                self.offset,
                self.acting_block_length,
                self.acting_version,
            )
        };
        let size = match entry.encoded_length() {
            Ok(s) => s,
            Err(e) => {
                self.poisoned = Some(e);
                self.count = 0;
                return Some(Err(e));
            }
        };
        self.offset += size;
        self.count -= 1;
        Some(Ok(entry))
    }
    /// Conservative: the declared count is an upper bound, but a
    /// malformed entry can end iteration early, so the lower bound
    /// is zero. This group is deliberately **not**
    /// `ExactSizeIterator` — a size-based allocation must not trust
    /// a count the wire has not yet justified.
    ///
    /// Poisoning zeroes the count, so this collapses to `(0, Some(0))`
    /// on a broken group without a separate branch.
    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, Some(self.count))
    }
}
/// Exhausted by count or by poison, `next()` keeps returning `None`.
///
/// [`Self::rewind`] is the documented exception: it is not `next`,
/// and it deliberately restarts a *new* iteration from the start
/// offset proven at wrap time. Do not call it partway through an
/// adaptor that has cached this fuse.
impl<'a, C: sbe_rt::GroupContext> core::iter::FusedIterator
for PerformanceFiguresDecoder<'a, C> {}
#[doc = concat!(
    "Entry decoder for the `", stringify!(PerformanceFiguresEntryDecoder),
    "` group — access fixed fields and var-data for one entry."
)]
pub struct PerformanceFiguresEntryDecoder<'a> {
    buf: &'a [u8],
    offset: usize,
    acting_version: u16,
    acting_block_length: usize,
    /// One-shot entry-extent cache: filled by `encoded_length`,
    /// reused by the last var-data accessor. `Cell` keeps `&self`
    /// getters and makes the entry `Send` + `!Sync`.
    tail_end: core::cell::Cell<Option<usize>>,
}
impl<'a> PerformanceFiguresEntryDecoder<'a> {
    ///`ENTRY_BLOCK_LENGTH` = 1.
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    /// Private entry wrap after the group iterator has proven extents.
    ///
    /// # Safety
    /// Fixed block at `offset` and every dynamic tail extent this entry
    /// will traverse must be fully in-bounds in `buf`.
    #[inline]
    unsafe fn wrap(
        buf: &'a [u8],
        offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        Self {
            buf,
            offset,
            acting_version,
            acting_block_length,
            tail_end: core::cell::Cell::new(None),
        }
    }
    ///Generated method `octane_rating`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn octane_rating(&self) -> u8 {
        let offset = self.offset + 0;
        u8::from_le_bytes(unsafe { read_bytes_unchecked::<1>(self.buf, offset) })
    }
    ///`OCTANE_RATING_ID` = 14.
    pub const OCTANE_RATING_ID: u16 = 14;
    ///`OCTANE_RATING_SINCE_VERSION` = 0.
    pub const OCTANE_RATING_SINCE_VERSION: u16 = 0;
    ///`OCTANE_RATING_ENCODING_OFFSET` = 0.
    pub const OCTANE_RATING_ENCODING_OFFSET: usize = 0;
    ///`OCTANE_RATING_ENCODING_LENGTH` = 1.
    pub const OCTANE_RATING_ENCODING_LENGTH: usize = 1;
    ///Generated method `octane_rating_meta_attribute`.
    #[inline]
    pub const fn octane_rating_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`OCTANE_RATING_NULL` = 255.
    pub const OCTANE_RATING_NULL: u8 = 255_u8;
    ///`OCTANE_RATING_MIN` = 90.
    pub const OCTANE_RATING_MIN: u8 = 90_u8;
    ///`OCTANE_RATING_MAX` = 110.
    pub const OCTANE_RATING_MAX: u8 = 110_u8;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        if self.acting_block_length > self.buf.len().saturating_sub(self.offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: self.acting_block_length,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        Ok(self.offset + self.acting_block_length)
    }
    #[inline]
    fn walk_tail_0(&self, start: usize) -> Result<usize, sbe_rt::DecodeError> {
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_len = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let mut offset = start + 4;
        let mut idx = 0;
        while idx < count {
            offset = PerformanceFiguresAccelerationEntryDecoder::skip(
                self.buf,
                offset,
                block_len,
                self.acting_version,
            )?;
            idx += 1;
        }
        Ok(offset)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        self.walk_tail_0(start)
    }
    ///Generated method `acceleration`.
    #[inline]
    pub fn acceleration(
        &self,
    ) -> Result<PerformanceFiguresAccelerationDecoder<'a>, sbe_rt::DecodeError> {
        if self.tail_end.get().is_some() {
            let offset = self.offset + self.acting_block_length;
            return unsafe {
                PerformanceFiguresAccelerationDecoder::wrap_trusted(
                    self.buf,
                    offset,
                    self.acting_version,
                    0,
                    0,
                )
            };
        }
        let offset = self.tail_offset_0()?;
        if self.tail_end.get().is_some() {
            return unsafe {
                PerformanceFiguresAccelerationDecoder::wrap_trusted(
                    self.buf,
                    offset,
                    self.acting_version,
                    0,
                    0,
                )
            };
        }
        PerformanceFiguresAccelerationDecoder::wrap(
            self.buf,
            offset,
            self.acting_version,
        )
    }
    ///Generated method `encoded_length`.
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        if let Some(end) = self.tail_end.get() {
            return Ok(end - self.offset);
        }
        let end = self.tail_offset_1()?;
        self.tail_end.set(Some(end));
        Ok(end - self.offset)
    }
    ///Generated method `skip`.
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        offset: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        if block_len > buf.len().saturating_sub(offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: block_len,
                available: buf.len().saturating_sub(offset),
            });
        }
        let entry = unsafe { Self::wrap(buf, offset, block_len, acting_version) };
        entry.tail_offset_1()
    }
}
impl<'a> core::fmt::Display for PerformanceFiguresEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.octane_rating();
            write!(f, "octaneRating: {:?}", v)?;
        }
        write!(f, ", acceleration: [")?;
        if let Ok(ng_decoder) = self.acceleration() {
            for (i, entry) in ng_decoder.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
        }
        write!(f, "]")?;
        write!(f, " }}")
    }
}
#[doc = concat!(
    "Group `", stringify!(PerformanceFiguresAccelerationDecoder),
    "` decoder — iterate entries in wire order."
)]
pub struct PerformanceFiguresAccelerationDecoder<
    'a,
    C: sbe_rt::GroupContext = sbe_rt::Detached,
> {
    buf: &'a [u8],
    offset: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
    acting_block_length: usize,
    parent_pos: usize,
    parent_block_length: usize,
    _context: core::marker::PhantomData<C>,
}
impl<'a, C: sbe_rt::GroupContext> PerformanceFiguresAccelerationDecoder<'a, C> {
    /// Proof-dependent constructor: like `wrap()` but remembers the
    /// parent message body position and acting block length so
    /// `finish()` can rebuild the next stage.
    ///
    /// Private to the generated module — a caller outside it cannot
    /// invent parent state and then `finish()` into a message stage
    /// that never existed.
    ///
    /// # Safety
    /// `parent_pos` and `parent_block_length` must describe the message
    /// body this group is genuinely nested in, and `offset` must be that
    /// message's real dimension-header offset for this group. The
    /// dimension header, the acting block length, and the group extent
    /// are still validated here and may be untrusted.
    #[inline]
    unsafe fn wrap_with_parent(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Result<
        PerformanceFiguresAccelerationDecoder<'a, sbe_rt::Attached>,
        sbe_rt::DecodeError,
    > {
        if 4 > buf.len().saturating_sub(offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: 4,
                available: buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let entries_start = offset + 4;
        let min_fixed = <PerformanceFiguresAccelerationDecoder<
            '_,
            sbe_rt::Detached,
        >>::min_readable_fixed_extent(acting_version);
        if count > 0 && block_length < min_fixed {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: min_fixed,
                available: block_length,
            });
        }
        let entries_length = count
            .checked_mul(block_length)
            .ok_or(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: usize::MAX,
                available: buf.len().saturating_sub(entries_start),
            })?;
        if entries_length > buf.len().saturating_sub(entries_start) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: entries_length,
                available: buf.len().saturating_sub(entries_start),
            });
        }
        Ok(PerformanceFiguresAccelerationDecoder {
            buf,
            offset: entries_start,
            count,
            start: entries_start,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
            _context: core::marker::PhantomData,
        })
    }
    /// Attached decoder for a group that is not in the acting version:
    /// zero entries, zero bytes, immediately complete.
    ///
    /// # Safety
    /// `parent_pos` and `parent_block_length` must describe the message
    /// body this group is nested in, and `offset` must be the byte
    /// position where this group would have started had it been present.
    #[inline]
    unsafe fn wrap_absent_parent(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> PerformanceFiguresAccelerationDecoder<'a, sbe_rt::Attached> {
        PerformanceFiguresAccelerationDecoder {
            buf,
            offset,
            count: 0,
            start: offset,
            total: 0,
            acting_version,
            acting_block_length: 0,
            parent_pos,
            parent_block_length,
            _context: core::marker::PhantomData,
        }
    }
    ///Generated method `is_empty`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    /// Wire-declared entries not yet consumed.
    ///
    /// O(1): `into_*` already read the SBE dimension header containing
    /// `numInGroup`. This does not promise that remaining entries will
    /// decode, so dynamic groups are not [`core::iter::ExactSizeIterator`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn remaining_entries(&self) -> usize {
        self.count
    }
}
impl<'a> PerformanceFiguresAccelerationDecoder<'a, sbe_rt::Detached> {
    ///`ENTRY_BLOCK_LENGTH` = 6.
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    /// Minimum entry bytes needed to safely read every **required**
    /// fixed field present at `acting_version`.
    ///
    /// Version-aware, and not always the compiled
    /// `ENTRY_BLOCK_LENGTH`: a forward-compatible reader accepts a
    /// wire block length it does not recognise, but never one too
    /// small for the fields it will actually read.
    #[must_use = "this extent is the minimum readable body size; ignoring it skips a bounds check"]
    #[inline]
    pub const fn min_readable_fixed_extent(acting_version: u16) -> usize {
        let mut m = 6;
        m
    }
    /// Wrap a standalone group at its dimension header, with bounds
    /// checks.
    ///
    /// This is the only public constructor. It validates the dimension
    /// header, rejects a wire block length too small to hold the
    /// required fixed fields active at `acting_version`, and — for
    /// fixed-stride groups — proves the whole entry region at once.
    ///
    /// The result is *detached*: it iterates, random-accesses, and
    /// rewinds, but has no parent message to complete into, so it has
    /// no `finish` / `skip_remaining`.
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
    ) -> Result<
        PerformanceFiguresAccelerationDecoder<'a, sbe_rt::Detached>,
        sbe_rt::DecodeError,
    > {
        let attached = unsafe {
            <PerformanceFiguresAccelerationDecoder<
                'a,
                sbe_rt::Attached,
            >>::wrap_with_parent(buf, offset, acting_version, 0, 0)?
        };
        Ok(PerformanceFiguresAccelerationDecoder {
            buf: attached.buf,
            offset: attached.offset,
            count: attached.count,
            start: attached.start,
            total: attached.total,
            acting_version: attached.acting_version,
            acting_block_length: attached.acting_block_length,
            parent_pos: attached.parent_pos,
            parent_block_length: attached.parent_block_length,
            _context: core::marker::PhantomData,
        })
    }
}
impl<'a, C: sbe_rt::GroupContext> PerformanceFiguresAccelerationDecoder<'a, C> {
    /// Entries not yet advanced (count), not a byte slice.
    /// For message-level byte tails use `get_metadata().remaining()`.
    /// Prefer [`Self::remaining_entries`] at call sites that mean
    /// group cardinality rather than a byte tail.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.remaining_entries()
    }
    /// Dimension wrap after the caller has proven
    /// the dimension header (and, for fixed groups, the full entry
    /// region) is in-bounds. Prefer [`Self::wrap`] / [`Self::wrap_with_parent`].
    ///
    /// # Safety
    /// `offset + dimension_header_size` must not overflow and must be
    /// ≤ `buf.len()`. For fixed-block groups (no nested tail),
    /// `offset + dim + count * acting_block_length` must also fit. Entry
    /// accessors then use unchecked fixed-field reads under that proof.
    #[inline]
    pub(crate) unsafe fn wrap_trusted(
        buf: &'a [u8],
        offset: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = unsafe { read_bytes_unchecked::<4>(buf, offset) };
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let min_fixed = <PerformanceFiguresAccelerationDecoder<
            '_,
            sbe_rt::Detached,
        >>::min_readable_fixed_extent(acting_version);
        Ok(Self {
            buf,
            offset: offset + 4,
            count,
            start: offset + 4,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
            _context: core::marker::PhantomData,
        })
    }
    /// Restart iteration from the group's proven start.
    ///
    /// This is the one operation that clears a poisoned group: the
    /// start offset was validated at wrap time, so retrying from there
    /// is sound even after an entry failed.
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        self.offset = self.start;
        self.count = self.total;
        self
    }
}
impl<'a, C: sbe_rt::GroupContext> PerformanceFiguresAccelerationDecoder<'a, C> {
    ///Generated method `skip_n`.
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: n.saturating_mul(self.acting_block_length),
                available: self.count.saturating_mul(self.acting_block_length),
            });
        }
        self.offset += n.saturating_mul(self.acting_block_length);
        self.count -= n;
        Ok(())
    }
    /// Bulk-decode all remaining entries into a caller-owned `Vec`.
    /// Zero-allocation after warm-up — the caller reuses the
    /// destination buffer across messages.
    #[inline]
    pub fn bulk_decode_into(
        &mut self,
        dst: &mut Vec<PerformanceFiguresAccelerationEntry>,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let needed = self
            .count
            .checked_mul(self.acting_block_length)
            .ok_or(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: usize::MAX,
                available: 0,
            })?;
        if self.offset + needed > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let cap = self.count;
        dst.clear();
        dst.reserve(cap);
        for _ in 0..cap {
            let offset = self.offset;
            self.offset += self.acting_block_length;
            dst.push(PerformanceFiguresAccelerationEntry {
                mph: u16::from_le_bytes(
                    self.buf[offset + 0..offset + 0 + 2].try_into().unwrap(),
                ),
                seconds: f32::from_le_bytes(
                    self.buf[offset + 2..offset + 2 + 4].try_into().unwrap(),
                ),
            });
        }
        self.count = 0;
        Ok(cap)
    }
    /// Bulk-decode all remaining entries into a new `Vec`.
    /// Convenience wrapper around [`Self::bulk_decode_into`].
    /// One bounds check for the whole batch — faster than
    /// iterating with [`Iterator::next`] when materialising
    /// the entire group (DTO construction, snapshots).
    #[inline]
    pub fn bulk_decode(
        &mut self,
    ) -> Result<Vec<PerformanceFiguresAccelerationEntry>, sbe_rt::DecodeError> {
        let mut out = Vec::new();
        self.bulk_decode_into(&mut out)?;
        Ok(out)
    }
}
impl<'a, C: sbe_rt::GroupContext> PerformanceFiguresAccelerationDecoder<'a, C> {
    ///Generated method `entry_at`.
    #[inline]
    pub fn entry_at(
        &self,
        idx: usize,
    ) -> Result<PerformanceFiguresAccelerationEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: idx.saturating_add(1).saturating_mul(self.acting_block_length),
                available: self.total.saturating_mul(self.acting_block_length),
            });
        }
        let byte_offset = idx
            .checked_mul(self.acting_block_length)
            .ok_or(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: usize::MAX,
                available: self.buf.len().saturating_sub(self.start),
            })?;
        let offset = self
            .start
            .checked_add(byte_offset)
            .ok_or(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: usize::MAX,
                available: self.buf.len().saturating_sub(self.start),
            })?;
        if self.acting_block_length > self.buf.len().saturating_sub(offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: self.acting_block_length,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        Ok(unsafe {
            PerformanceFiguresAccelerationEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            )
        })
    }
}
impl<'a, C: sbe_rt::GroupContext> Iterator
for PerformanceFiguresAccelerationDecoder<'a, C> {
    type Item = PerformanceFiguresAccelerationEntryDecoder<'a>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = unsafe {
            PerformanceFiguresAccelerationEntryDecoder::wrap(
                self.buf,
                self.offset,
                self.acting_block_length,
                self.acting_version,
            )
        };
        self.offset += self.acting_block_length;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a, C: sbe_rt::GroupContext> ExactSizeIterator
for PerformanceFiguresAccelerationDecoder<'a, C> {
    #[inline]
    fn len(&self) -> usize {
        self.count
    }
}
#[doc = concat!(
    "Entry decoder for the `", stringify!(PerformanceFiguresAccelerationEntryDecoder),
    "` group — access fixed fields and var-data for one entry."
)]
pub struct PerformanceFiguresAccelerationEntryDecoder<'a> {
    buf: &'a [u8],
    offset: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> PerformanceFiguresAccelerationEntryDecoder<'a> {
    ///`ENTRY_BLOCK_LENGTH` = 6.
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    /// Private entry wrap after the group iterator (or equivalent)
    /// has proven the acting fixed block is in-bounds at `offset`.
    ///
    /// # Safety
    /// `offset + max(acting_block_length, ENTRY_BLOCK_LENGTH)` (and any
    /// field offset used by accessors) must not overflow and must be
    /// ≤ `buf.len()`. Fixed-field getters may then use unchecked reads.
    #[inline]
    unsafe fn wrap(
        buf: &'a [u8],
        offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        Self {
            buf,
            offset,
            acting_version,
            acting_block_length,
        }
    }
    ///Generated method `mph`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn mph(&self) -> u16 {
        let offset = self.offset + 0;
        u16::from_le_bytes(unsafe { read_bytes_unchecked::<2>(self.buf, offset) })
    }
    ///`MPH_ID` = 16.
    pub const MPH_ID: u16 = 16;
    ///`MPH_SINCE_VERSION` = 0.
    pub const MPH_SINCE_VERSION: u16 = 0;
    ///`MPH_ENCODING_OFFSET` = 0.
    pub const MPH_ENCODING_OFFSET: usize = 0;
    ///`MPH_ENCODING_LENGTH` = 2.
    pub const MPH_ENCODING_LENGTH: usize = 2;
    ///Generated method `mph_meta_attribute`.
    #[inline]
    pub const fn mph_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`MPH_NULL` = 65535.
    pub const MPH_NULL: u16 = 65535_u16;
    ///`MPH_MIN` = 0.
    pub const MPH_MIN: u16 = 0_u16;
    ///`MPH_MAX` = 65534.
    pub const MPH_MAX: u16 = 65534_u16;
    ///Generated method `seconds`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn seconds(&self) -> f32 {
        let offset = self.offset + 2;
        f32::from_le_bytes(unsafe { read_bytes_unchecked::<4>(self.buf, offset) })
    }
    ///`SECONDS_ID` = 17.
    pub const SECONDS_ID: u16 = 17;
    ///`SECONDS_SINCE_VERSION` = 0.
    pub const SECONDS_SINCE_VERSION: u16 = 0;
    ///`SECONDS_ENCODING_OFFSET` = 2.
    pub const SECONDS_ENCODING_OFFSET: usize = 2;
    ///`SECONDS_ENCODING_LENGTH` = 4.
    pub const SECONDS_ENCODING_LENGTH: usize = 4;
    ///Generated method `seconds_meta_attribute`.
    #[inline]
    pub const fn seconds_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///Generated constant `SECONDS_NULL`.
    pub const SECONDS_NULL: f32 = f32::from_bits(2143289344u32);
    ///Generated constant `SECONDS_MIN`.
    pub const SECONDS_MIN: f32 = f32::from_bits(4286578687u32);
    ///Generated constant `SECONDS_MAX`.
    pub const SECONDS_MAX: f32 = f32::from_bits(2139095039u32);
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        if self.acting_block_length > self.buf.len().saturating_sub(self.offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: self.acting_block_length,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        Ok(self.offset + self.acting_block_length)
    }
    ///Generated method `encoded_length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.acting_block_length
    }
    ///Generated method `skip`.
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        offset: usize,
        block_len: usize,
        _acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        if block_len > buf.len().saturating_sub(offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: block_len,
                available: buf.len().saturating_sub(offset),
            });
        }
        Ok(offset + block_len)
    }
}
impl<'a> core::fmt::Display for PerformanceFiguresAccelerationEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.mph();
            write!(f, "mph: {:?}", v)?;
        }
        {
            let v = self.seconds();
            write!(f, ", seconds: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
/// Consuming decoder stage — drop without `into_*` / `finish` skips
/// remaining wire tails.
#[must_use = "decoder stage must be advanced with into_*/finish or tails are skipped"]
pub struct PerformanceFiguresEntryDecoderComplete<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) offset: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
impl<'a> PerformanceFiguresEntryDecoderComplete<'a> {
    /// Schema version from the message header (or wrap args), not the
    /// compiled schema constant. Fields with `sinceVersion` and optional
    /// presence depend on this value.
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    /// Block length from the wire header / wrap args. Tail offsets use
    /// this acting length, not only the compiled `BLOCK_LENGTH`.
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> PerformanceFiguresEntryDecoder<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_acceleration(
        self,
    ) -> Result<
        PerformanceFiguresAccelerationDecoder<'a, sbe_rt::Attached>,
        sbe_rt::DecodeError,
    > {
        let group_start = self.offset + self.acting_block_length;
        unsafe {
            <PerformanceFiguresAccelerationDecoder<
                'a,
                sbe_rt::Attached,
            >>::wrap_with_parent(
                self.buf,
                group_start,
                self.acting_version,
                self.offset,
                self.acting_block_length,
            )
        }
    }
}
impl<'a> PerformanceFiguresAccelerationDecoder<'a, sbe_rt::Attached> {
    #[inline]
    fn into_parent_stage(
        self,
        tail_start: usize,
    ) -> PerformanceFiguresEntryDecoderComplete<'a> {
        PerformanceFiguresEntryDecoderComplete {
            buf: self.buf,
            offset: self.parent_pos,
            tail_start,
            acting_version: self.acting_version,
            acting_block_length: self.parent_block_length,
        }
    }
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    ///
    /// Only an *attached* group — one reached through its message's
    /// tail — can complete into a message stage. A standalone
    /// [`Self::wrap`] has no parent to return to.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<PerformanceFiguresEntryDecoderComplete<'a>, sbe_rt::DecodeError> {
        let mut offset = self.offset;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            offset = PerformanceFiguresAccelerationEntryDecoder::skip(
                self.buf,
                offset,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(self.into_parent_stage(offset))
    }
    /// Explicit sequential spelling of "advance past the rest of this group".
    #[inline]
    pub fn skip_remaining(
        self,
    ) -> Result<PerformanceFiguresEntryDecoderComplete<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
    /// Consume every remaining entry in one pass and return the next
    /// parent stage.
    ///
    /// Fixed-stride entries advance by the acting block length.
    /// Empty groups invoke the callback zero times.
    ///
    /// A callback or decoding error consumes this ordered stage and
    /// returns no continuation.
    #[inline]
    pub fn visit_entries<E, F>(
        mut self,
        mut visit: F,
    ) -> Result<PerformanceFiguresEntryDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnMut(PerformanceFiguresAccelerationEntryDecoder<'a>) -> Result<(), E>,
    {
        while self.count > 0 {
            let entry = unsafe {
                PerformanceFiguresAccelerationEntryDecoder::wrap(
                    self.buf,
                    self.offset,
                    self.acting_block_length,
                    self.acting_version,
                )
            };
            visit(entry)?;
            self.offset += self.acting_block_length;
            self.count -= 1;
        }
        let tail_start = self.offset;
        Ok(self.into_parent_stage(tail_start))
    }
}
impl<'a> PerformanceFiguresEntryDecoderComplete<'a> {
    /// Body bytes (excluding the message header; for entries this is the
    /// complete entry bytes).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_body_bytes(&self) -> &'a [u8] {
        &self.buf[self.offset..self.tail_start]
    }
    /// Complete SBE frame (header + body) for message stages.
    /// For entry stages (`HEADER_LENGTH == 0`) this equals [`Self::as_body_bytes`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_bytes_with_header(&self) -> &'a [u8] {
        &self.buf[self.offset - 0..self.tail_start]
    }
    /// Body length (excluding header).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.tail_start - self.offset
    }
    /// Total message length including the schema-declared header.
    /// Pure arithmetic: body length + `HEADER_LENGTH`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.tail_start - self.offset + 0
    }
    /// Bytes after this message/entry.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.tail_start..]
    }
}
/// Consuming decoder stage — drop without `into_*` / `finish` skips
/// remaining wire tails.
#[must_use = "decoder stage must be advanced with into_*/finish or tails are skipped"]
pub struct CarDecoderAfterFuelFigures<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) offset: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
/// Consuming decoder stage — drop without `into_*` / `finish` skips
/// remaining wire tails.
#[must_use = "decoder stage must be advanced with into_*/finish or tails are skipped"]
pub struct CarDecoderAfterPerformanceFigures<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) offset: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
/// Consuming decoder stage — drop without `into_*` / `finish` skips
/// remaining wire tails.
#[must_use = "decoder stage must be advanced with into_*/finish or tails are skipped"]
pub struct CarDecoderAfterManufacturer<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) offset: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
/// Consuming decoder stage — drop without `into_*` / `finish` skips
/// remaining wire tails.
#[must_use = "decoder stage must be advanced with into_*/finish or tails are skipped"]
pub struct CarDecoderAfterModel<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) offset: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
/// Consuming decoder stage — drop without `into_*` / `finish` skips
/// remaining wire tails.
#[must_use = "decoder stage must be advanced with into_*/finish or tails are skipped"]
pub struct CarDecoderComplete<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) offset: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
impl<'a> CarDecoderAfterFuelFigures<'a> {
    /// Schema version from the message header (or wrap args), not the
    /// compiled schema constant. Fields with `sinceVersion` and optional
    /// presence depend on this value.
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    /// Block length from the wire header / wrap args. Tail offsets use
    /// this acting length, not only the compiled `BLOCK_LENGTH`.
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> CarDecoderAfterPerformanceFigures<'a> {
    /// Schema version from the message header (or wrap args), not the
    /// compiled schema constant. Fields with `sinceVersion` and optional
    /// presence depend on this value.
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    /// Block length from the wire header / wrap args. Tail offsets use
    /// this acting length, not only the compiled `BLOCK_LENGTH`.
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> CarDecoderAfterManufacturer<'a> {
    /// Schema version from the message header (or wrap args), not the
    /// compiled schema constant. Fields with `sinceVersion` and optional
    /// presence depend on this value.
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    /// Block length from the wire header / wrap args. Tail offsets use
    /// this acting length, not only the compiled `BLOCK_LENGTH`.
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> CarDecoderAfterModel<'a> {
    /// Schema version from the message header (or wrap args), not the
    /// compiled schema constant. Fields with `sinceVersion` and optional
    /// presence depend on this value.
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    /// Block length from the wire header / wrap args. Tail offsets use
    /// this acting length, not only the compiled `BLOCK_LENGTH`.
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> CarDecoderComplete<'a> {
    /// Schema version from the message header (or wrap args), not the
    /// compiled schema constant. Fields with `sinceVersion` and optional
    /// presence depend on this value.
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    /// Block length from the wire header / wrap args. Tail offsets use
    /// this acting length, not only the compiled `BLOCK_LENGTH`.
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> CarDecoder<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_fuel_figures(
        self,
    ) -> Result<FuelFiguresDecoder<'a, sbe_rt::Attached>, sbe_rt::DecodeError> {
        let group_start = self.byte_offset() + self.acting_block_length;
        unsafe {
            <FuelFiguresDecoder<
                'a,
                sbe_rt::Attached,
            >>::wrap_with_parent(
                self.buf,
                group_start,
                self.acting_version,
                self.byte_offset(),
                self.acting_block_length,
            )
        }
    }
}
impl<'a> CarDecoderAfterFuelFigures<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_performance_figures(
        self,
    ) -> Result<PerformanceFiguresDecoder<'a, sbe_rt::Attached>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        unsafe {
            <PerformanceFiguresDecoder<
                'a,
                sbe_rt::Attached,
            >>::wrap_with_parent(
                self.buf,
                group_start,
                self.acting_version,
                self.offset,
                self.acting_block_length,
            )
        }
    }
}
impl<'a> CarDecoderAfterPerformanceFigures<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_manufacturer(
        self,
    ) -> Result<(&'a [u8], CarDecoderAfterManufacturer<'a>), sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "manufacturer",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let len = u32::from_le_bytes(bytes) as u64;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "manufacturer",
                length: len,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            "manufacturer",
            offset,
            4,
            len,
            self.buf.len(),
        )?;
        let data = &self.buf[data_start..data_end];
        let next = CarDecoderAfterManufacturer {
            buf: self.buf,
            offset: self.offset,
            tail_start: data_end,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
    ///Non-consuming variant: read this var-data field as `&[u8]` without advancing or constructing the next stage.
    ///
    ///Cheaper than [`Self::into_manufacturer`] when only the bytes are needed.
    #[inline]
    pub fn manufacturer_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "manufacturer",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let len = u32::from_le_bytes(bytes) as u64;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "manufacturer",
                length: len,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            "manufacturer",
            offset,
            4,
            len,
            self.buf.len(),
        )?;
        Ok(&self.buf[data_start..data_end])
    }
}
impl<'a> CarDecoderAfterPerformanceFigures<'a> {
    /// Consume this stage, read the next UTF-8 var-data
    /// field as a validated `&str`, and advance.
    #[inline]
    pub fn into_manufacturer_as_str(
        self,
    ) -> Result<(&'a str, CarDecoderAfterManufacturer<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_manufacturer()?;
        let s = core::str::from_utf8(bytes)
            .map_err(|e| {
                sbe_rt::DecodeError::InvalidUtf8 {
                    field: "manufacturer",
                    error: e,
                }
            })?;
        Ok((s, next))
    }
}
impl<'a> CarDecoderAfterPerformanceFigures<'a> {
    /// Consume this stage, read the next text var-data field as
    /// a `&str` without encoding validation, and advance.
    ///
    /// Structural bounds (truncated payload, overflowing length)
    /// remain fallible — only character validation is skipped.
    ///
    /// # Safety
    /// The wire bytes must be valid for the schema-declared
    /// character encoding (UTF-8 or ASCII).
    #[inline]
    pub unsafe fn into_manufacturer_as_str_unchecked(
        self,
    ) -> Result<(&'a str, CarDecoderAfterManufacturer<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_manufacturer()?;
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        Ok((s, next))
    }
}
impl<'a> CarDecoderAfterPerformanceFigures<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_manufacturer<E, F>(
        self,
        f: F,
    ) -> Result<CarDecoderAfterManufacturer<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_manufacturer()?;
        f(data)?;
        Ok(next)
    }
}
impl<'a> CarDecoderAfterPerformanceFigures<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_manufacturer_as_message(
        self,
    ) -> Result<
        (DecodedFrame<'a>, CarDecoderAfterManufacturer<'a>),
        sbe_rt::DecodeError,
    > {
        let (data, next) = self.into_manufacturer()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
    /// Fallible scoped nested-message accessor.
    #[inline]
    pub fn try_manufacturer_as_message<E, F>(
        self,
        f: F,
    ) -> Result<CarDecoderAfterManufacturer<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_manufacturer_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> CarDecoderAfterManufacturer<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_model(
        self,
    ) -> Result<(&'a [u8], CarDecoderAfterModel<'a>), sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "model",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let len = u32::from_le_bytes(bytes) as u64;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "model",
                length: len,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            "model",
            offset,
            4,
            len,
            self.buf.len(),
        )?;
        let data = &self.buf[data_start..data_end];
        let next = CarDecoderAfterModel {
            buf: self.buf,
            offset: self.offset,
            tail_start: data_end,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
    ///Non-consuming variant: read this var-data field as `&[u8]` without advancing or constructing the next stage.
    ///
    ///Cheaper than [`Self::into_model`] when only the bytes are needed.
    #[inline]
    pub fn model_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "model",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let len = u32::from_le_bytes(bytes) as u64;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "model",
                length: len,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            "model",
            offset,
            4,
            len,
            self.buf.len(),
        )?;
        Ok(&self.buf[data_start..data_end])
    }
}
impl<'a> CarDecoderAfterManufacturer<'a> {
    /// Consume this stage, read the next UTF-8 var-data
    /// field as a validated `&str`, and advance.
    #[inline]
    pub fn into_model_as_str(
        self,
    ) -> Result<(&'a str, CarDecoderAfterModel<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_model()?;
        let s = core::str::from_utf8(bytes)
            .map_err(|e| {
                sbe_rt::DecodeError::InvalidUtf8 {
                    field: "model",
                    error: e,
                }
            })?;
        Ok((s, next))
    }
}
impl<'a> CarDecoderAfterManufacturer<'a> {
    /// Consume this stage, read the next text var-data field as
    /// a `&str` without encoding validation, and advance.
    ///
    /// Structural bounds (truncated payload, overflowing length)
    /// remain fallible — only character validation is skipped.
    ///
    /// # Safety
    /// The wire bytes must be valid for the schema-declared
    /// character encoding (UTF-8 or ASCII).
    #[inline]
    pub unsafe fn into_model_as_str_unchecked(
        self,
    ) -> Result<(&'a str, CarDecoderAfterModel<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_model()?;
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        Ok((s, next))
    }
}
impl<'a> CarDecoderAfterManufacturer<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_model<E, F>(self, f: F) -> Result<CarDecoderAfterModel<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_model()?;
        f(data)?;
        Ok(next)
    }
}
impl<'a> CarDecoderAfterManufacturer<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_model_as_message(
        self,
    ) -> Result<(DecodedFrame<'a>, CarDecoderAfterModel<'a>), sbe_rt::DecodeError> {
        let (data, next) = self.into_model()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
    /// Fallible scoped nested-message accessor.
    #[inline]
    pub fn try_model_as_message<E, F>(self, f: F) -> Result<CarDecoderAfterModel<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_model_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> CarDecoderAfterModel<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_activation_code(
        self,
    ) -> Result<(&'a [u8], CarDecoderComplete<'a>), sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "activationCode",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let len = u32::from_le_bytes(bytes) as u64;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "activationCode",
                length: len,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            "activationCode",
            offset,
            4,
            len,
            self.buf.len(),
        )?;
        let data = &self.buf[data_start..data_end];
        let next = CarDecoderComplete {
            buf: self.buf,
            offset: self.offset,
            tail_start: data_end,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
    ///Non-consuming variant: read this var-data field as `&[u8]` without advancing or constructing the next stage.
    ///
    ///Cheaper than [`Self::into_activation_code`] when only the bytes are needed.
    #[inline]
    pub fn activation_code_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "activationCode",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = unsafe {
            core::ptr::read_unaligned(self.buf.as_ptr().add(offset) as *const [u8; 4])
        };
        let len = u32::from_le_bytes(bytes) as u64;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "activationCode",
                length: len,
                max_length: 1073741824 as u64,
            });
        }
        let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
            "activationCode",
            offset,
            4,
            len,
            self.buf.len(),
        )?;
        Ok(&self.buf[data_start..data_end])
    }
}
impl<'a> CarDecoderAfterModel<'a> {
    /// Consume this stage, read the next ASCII var-data
    /// field as a validated `&str`, and advance.
    #[inline]
    pub fn into_activation_code_as_str(
        self,
    ) -> Result<(&'a str, CarDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_activation_code()?;
        if !bytes.is_ascii() {
            return Err(sbe_rt::DecodeError::InvalidAscii {
                field: "activationCode",
            });
        }
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        Ok((s, next))
    }
}
impl<'a> CarDecoderAfterModel<'a> {
    /// Consume this stage, read the next text var-data field as
    /// a `&str` without encoding validation, and advance.
    ///
    /// Structural bounds (truncated payload, overflowing length)
    /// remain fallible — only character validation is skipped.
    ///
    /// # Safety
    /// The wire bytes must be valid for the schema-declared
    /// character encoding (UTF-8 or ASCII).
    #[inline]
    pub unsafe fn into_activation_code_as_str_unchecked(
        self,
    ) -> Result<(&'a str, CarDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_activation_code()?;
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        Ok((s, next))
    }
}
impl<'a> CarDecoderAfterModel<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_activation_code<E, F>(self, f: F) -> Result<CarDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_activation_code()?;
        f(data)?;
        Ok(next)
    }
}
impl<'a> CarDecoderAfterModel<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_activation_code_as_message(
        self,
    ) -> Result<(DecodedFrame<'a>, CarDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (data, next) = self.into_activation_code()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
    /// Fallible scoped nested-message accessor.
    #[inline]
    pub fn try_activation_code_as_message<E, F>(
        self,
        f: F,
    ) -> Result<CarDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_activation_code_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> FuelFiguresDecoder<'a, sbe_rt::Attached> {
    #[inline]
    fn into_parent_stage(self, tail_start: usize) -> CarDecoderAfterFuelFigures<'a> {
        CarDecoderAfterFuelFigures {
            buf: self.buf,
            offset: self.parent_pos,
            tail_start,
            acting_version: self.acting_version,
            acting_block_length: self.parent_block_length,
        }
    }
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    ///
    /// Only an *attached* group — one reached through its message's
    /// tail — can complete into a message stage. A standalone
    /// [`Self::wrap`] has no parent to return to.
    #[inline]
    pub fn finish(self) -> Result<CarDecoderAfterFuelFigures<'a>, sbe_rt::DecodeError> {
        if let Some(error) = self.poisoned {
            return Err(error);
        }
        let mut offset = self.offset;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            offset = FuelFiguresEntryDecoder::skip(
                self.buf,
                offset,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(self.into_parent_stage(offset))
    }
    /// Explicit sequential spelling of "advance past the rest of this group".
    #[inline]
    pub fn skip_remaining(
        self,
    ) -> Result<CarDecoderAfterFuelFigures<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
    /// Consume every remaining entry in one pass and return the next
    /// parent stage.
    ///
    /// The callback must return this entry's generated completion
    /// stage. Dynamic `visit_entries` does not pre-scan
    /// `encoded_length()`; the next cursor comes from that
    /// completion. Empty groups invoke the callback zero times.
    ///
    /// A callback or decoding error consumes this ordered stage and
    /// returns no continuation. Returning a completion that does
    /// not belong to the supplied entry panics.
    #[inline]
    pub fn visit_entries<E, F>(
        mut self,
        mut visit: F,
    ) -> Result<CarDecoderAfterFuelFigures<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnMut(
            FuelFiguresEntryDecoder<'a>,
        ) -> Result<FuelFiguresEntryDecoderComplete<'a>, E>,
    {
        if let Some(error) = self.poisoned {
            return Err(E::from(error));
        }
        while self.count > 0 {
            let available = self.buf.len().saturating_sub(self.offset);
            if self.min_entry_extent > available {
                return Err(
                    E::from(sbe_rt::DecodeError::BufferTooShort {
                        field: "fuelFigures",
                        needed: self.min_entry_extent,
                        available,
                    }),
                );
            }
            let entry = unsafe {
                FuelFiguresEntryDecoder::wrap(
                    self.buf,
                    self.offset,
                    self.acting_block_length,
                    self.acting_version,
                )
            };
            let complete = visit(entry)?;
            if !core::ptr::eq(complete.buf.as_ptr(), self.buf.as_ptr())
                || complete.buf.len() != self.buf.len() || complete.offset != self.offset
                || complete.acting_version != self.acting_version
                || complete.acting_block_length != self.acting_block_length
            {
                panic!(
                    "visit_entries callback returned a completion that does not belong to the supplied entry"
                );
            }
            self.offset = complete.tail_start;
            self.count -= 1;
        }
        let tail_start = self.offset;
        Ok(self.into_parent_stage(tail_start))
    }
}
impl<'a> PerformanceFiguresDecoder<'a, sbe_rt::Attached> {
    #[inline]
    fn into_parent_stage(
        self,
        tail_start: usize,
    ) -> CarDecoderAfterPerformanceFigures<'a> {
        CarDecoderAfterPerformanceFigures {
            buf: self.buf,
            offset: self.parent_pos,
            tail_start,
            acting_version: self.acting_version,
            acting_block_length: self.parent_block_length,
        }
    }
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    ///
    /// Only an *attached* group — one reached through its message's
    /// tail — can complete into a message stage. A standalone
    /// [`Self::wrap`] has no parent to return to.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<CarDecoderAfterPerformanceFigures<'a>, sbe_rt::DecodeError> {
        if let Some(error) = self.poisoned {
            return Err(error);
        }
        let mut offset = self.offset;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            offset = PerformanceFiguresEntryDecoder::skip(
                self.buf,
                offset,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(self.into_parent_stage(offset))
    }
    /// Explicit sequential spelling of "advance past the rest of this group".
    #[inline]
    pub fn skip_remaining(
        self,
    ) -> Result<CarDecoderAfterPerformanceFigures<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
    /// Consume every remaining entry in one pass and return the next
    /// parent stage.
    ///
    /// The callback must return this entry's generated completion
    /// stage. Dynamic `visit_entries` does not pre-scan
    /// `encoded_length()`; the next cursor comes from that
    /// completion. Empty groups invoke the callback zero times.
    ///
    /// A callback or decoding error consumes this ordered stage and
    /// returns no continuation. Returning a completion that does
    /// not belong to the supplied entry panics.
    #[inline]
    pub fn visit_entries<E, F>(
        mut self,
        mut visit: F,
    ) -> Result<CarDecoderAfterPerformanceFigures<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnMut(
            PerformanceFiguresEntryDecoder<'a>,
        ) -> Result<PerformanceFiguresEntryDecoderComplete<'a>, E>,
    {
        if let Some(error) = self.poisoned {
            return Err(E::from(error));
        }
        while self.count > 0 {
            let available = self.buf.len().saturating_sub(self.offset);
            if self.min_entry_extent > available {
                return Err(
                    E::from(sbe_rt::DecodeError::BufferTooShort {
                        field: "performanceFigures",
                        needed: self.min_entry_extent,
                        available,
                    }),
                );
            }
            let entry = unsafe {
                PerformanceFiguresEntryDecoder::wrap(
                    self.buf,
                    self.offset,
                    self.acting_block_length,
                    self.acting_version,
                )
            };
            let complete = visit(entry)?;
            if !core::ptr::eq(complete.buf.as_ptr(), self.buf.as_ptr())
                || complete.buf.len() != self.buf.len() || complete.offset != self.offset
                || complete.acting_version != self.acting_version
                || complete.acting_block_length != self.acting_block_length
            {
                panic!(
                    "visit_entries callback returned a completion that does not belong to the supplied entry"
                );
            }
            self.offset = complete.tail_start;
            self.count -= 1;
        }
        let tail_start = self.offset;
        Ok(self.into_parent_stage(tail_start))
    }
}
impl<'a> CarDecoderComplete<'a> {
    /// Body bytes (excluding the message header; for entries this is the
    /// complete entry bytes).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_body_bytes(&self) -> &'a [u8] {
        &self.buf[self.offset..self.tail_start]
    }
    /// Complete SBE frame (header + body) for message stages.
    /// For entry stages (`HEADER_LENGTH == 0`) this equals [`Self::as_body_bytes`].
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_bytes_with_header(&self) -> &'a [u8] {
        &self.buf[self.offset - 8..self.tail_start]
    }
    /// Body length (excluding header).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.tail_start - self.offset
    }
    /// Total message length including the schema-declared header.
    /// Pure arithmetic: body length + `HEADER_LENGTH`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.tail_start - self.offset + 8
    }
    /// Bytes after this message/entry.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.tail_start..]
    }
}
impl<'a> CarDecoder<'a> {
    /// Convert this flyweight into a mutable ordered cursor.
    ///
    /// Group and var-data methods must then be called in schema order;
    /// a wrong call returns [`sbe_rt::DecodeError::OutOfOrder`] and
    /// leaves the cursor unchanged. Fixed fields stay random-access.
    #[inline]
    pub fn ordered(self) -> CarOrderedDecoder<'a> {
        let tail_offset = self.offset + self.acting_block_length;
        CarOrderedDecoder {
            inner: self,
            tail_offset,
            next_ordinal: 0,
        }
    }
}
/// Mutable ordered decoder — sequential dynamic tails, random-access
/// fixed fields, runtime order checks.
#[must_use = "decoder must be read or advanced; dropping is fine only after use"]
pub struct CarOrderedDecoder<'a> {
    inner: CarDecoder<'a>,
    tail_offset: usize,
    next_ordinal: u16,
}
impl<'a> CarOrderedDecoder<'a> {
    /// Schema version from the message header (or wrap args).
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.inner.acting_version()
    }
    /// Block length from the wire header / wrap args.
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.inner.acting_block_length()
    }
    /// Placement utilities. Does not expose random-access dynamic tails.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn get_metadata(&self) -> CarDecoderMetadata<'_, 'a> {
        self.inner.get_metadata()
    }
    ///Generated method `serial_number`.
    #[inline]
    pub fn serial_number(&self) -> u64 {
        self.inner.serial_number()
    }
    ///Generated method `model_year`.
    #[inline]
    pub fn model_year(&self) -> u16 {
        self.inner.model_year()
    }
    ///Generated method `available`.
    #[inline]
    pub fn available(&self) -> BooleanType {
        self.inner.available()
    }
    ///Generated method `code`.
    #[inline]
    pub fn code(&self) -> Model {
        self.inner.code()
    }
    ///Generated method `some_numbers`.
    #[inline]
    pub fn some_numbers(&self) -> [u32; 4] {
        self.inner.some_numbers()
    }
    ///Generated method `vehicle_code`.
    #[inline]
    pub fn vehicle_code(&self) -> [u8; 6] {
        self.inner.vehicle_code()
    }
    ///Generated method `extras`.
    #[inline]
    pub fn extras(&self) -> OptionalExtras {
        self.inner.extras()
    }
    ///Generated method `discounted_model`.
    #[inline]
    pub fn discounted_model(&self) -> Model {
        self.inner.discounted_model()
    }
    ///Generated method `engine`.
    #[inline]
    pub fn engine(&self) -> EngineDecoder<'_> {
        self.inner.engine()
    }
    ///Generated method `engine_value`.
    #[inline]
    pub fn engine_value(&self) -> Engine {
        self.inner.engine_value()
    }
}
impl<'a> CarOrderedDecoder<'a> {
    #[inline]
    fn expect(
        &self,
        ordinal: u16,
        requested: &'static str,
    ) -> Result<(), sbe_rt::DecodeError> {
        const NAMES: &[&str] = &[
            "fuelFigures",
            "performanceFigures",
            "manufacturer",
            "model",
            "activationCode",
        ];
        let expected = if (self.next_ordinal as usize) < NAMES.len() {
            NAMES[self.next_ordinal as usize]
        } else {
            "<complete>"
        };
        if self.next_ordinal != ordinal {
            return Err(sbe_rt::DecodeError::OutOfOrder {
                owner: "Car",
                expected,
                requested,
            });
        }
        Ok(())
    }
    ///Generated method `fuel_figures`.
    #[inline]
    pub fn fuel_figures(
        &mut self,
    ) -> Result<FuelFiguresOrderedDecoder<'_, 'a>, sbe_rt::DecodeError> {
        self.expect(0, "fuelFigures")?;
        FuelFiguresOrderedDecoder::begin(self)
    }
    ///Generated method `performance_figures`.
    #[inline]
    pub fn performance_figures(
        &mut self,
    ) -> Result<PerformanceFiguresOrderedDecoder<'_, 'a>, sbe_rt::DecodeError> {
        self.expect(1, "performanceFigures")?;
        PerformanceFiguresOrderedDecoder::begin(self)
    }
    ///Generated method `manufacturer`.
    #[inline]
    pub fn manufacturer(&mut self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.expect(2, "manufacturer")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "manufacturer",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "manufacturer",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "manufacturer",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(data)
    }
    ///Generated method `manufacturer_as_str`.
    #[inline]
    pub fn manufacturer_as_str(&mut self) -> Result<&'a str, sbe_rt::DecodeError> {
        self.expect(2, "manufacturer")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "manufacturer",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "manufacturer",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "manufacturer",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        let s = core::str::from_utf8(data)
            .map_err(|e| {
                sbe_rt::DecodeError::InvalidUtf8 {
                    field: "manufacturer",
                    error: e,
                }
            })?;
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(s)
    }
    ///Generated method `manufacturer_as_message`.
    #[inline]
    pub fn manufacturer_as_message(
        &mut self,
    ) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
        self.expect(2, "manufacturer")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "manufacturer",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "manufacturer",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "manufacturer",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(frame)
    }
    ///Generated method `model`.
    #[inline]
    pub fn model(&mut self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.expect(3, "model")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "model",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "model",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "model",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(data)
    }
    ///Generated method `model_as_str`.
    #[inline]
    pub fn model_as_str(&mut self) -> Result<&'a str, sbe_rt::DecodeError> {
        self.expect(3, "model")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "model",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "model",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "model",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        let s = core::str::from_utf8(data)
            .map_err(|e| {
                sbe_rt::DecodeError::InvalidUtf8 {
                    field: "model",
                    error: e,
                }
            })?;
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(s)
    }
    ///Generated method `model_as_message`.
    #[inline]
    pub fn model_as_message(&mut self) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
        self.expect(3, "model")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "model",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "model",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "model",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(frame)
    }
    ///Generated method `activation_code`.
    #[inline]
    pub fn activation_code(&mut self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.expect(4, "activationCode")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "activationCode",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "activationCode",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "activationCode",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(data)
    }
    ///Generated method `activation_code_as_str`.
    #[inline]
    pub fn activation_code_as_str(&mut self) -> Result<&'a str, sbe_rt::DecodeError> {
        self.expect(4, "activationCode")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "activationCode",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "activationCode",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "activationCode",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        if !data.is_ascii() {
            return Err(sbe_rt::DecodeError::InvalidAscii {
                field: "activationCode",
            });
        }
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(unsafe { core::str::from_utf8_unchecked(data) })
    }
    ///Generated method `activation_code_as_message`.
    #[inline]
    pub fn activation_code_as_message(
        &mut self,
    ) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
        self.expect(4, "activationCode")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "activationCode",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "activationCode",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "activationCode",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(frame)
    }
    /// Skip any unconsumed suffix and return the complete stage.
    #[inline]
    pub fn finish(mut self) -> Result<CarDecoderComplete<'a>, sbe_rt::DecodeError> {
        while (self.next_ordinal as usize) < 5 {
            match self.next_ordinal {
                0 => self.fuel_figures()?.skip_remaining()?,
                1 => self.performance_figures()?.skip_remaining()?,
                2 => {
                    let _ = self.manufacturer()?;
                }
                3 => {
                    let _ = self.model()?;
                }
                4 => {
                    let _ = self.activation_code()?;
                }
                _ => break,
            }
        }
        Ok(CarDecoderComplete {
            buf: self.inner.buf,
            offset: self.inner.offset,
            tail_start: self.tail_offset,
            acting_version: self.inner.acting_version,
            acting_block_length: self.inner.acting_block_length,
        })
    }
}
///Generated struct `FuelFiguresOrderedDecoder`.
pub struct FuelFiguresOrderedDecoder<'p, 'a> {
    buf: &'a [u8],
    offset: usize,
    count: usize,
    acting_block_length: usize,
    acting_version: u16,
    min_entry_extent: usize,
    parent: &'p mut CarOrderedDecoder<'a>,
}
impl<'p, 'a> FuelFiguresOrderedDecoder<'p, 'a> {
    #[inline]
    fn begin(
        parent: &'p mut CarOrderedDecoder<'a>,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let start = parent.tail_offset;
        if 4 > parent.inner.buf.len().saturating_sub(start) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: 4,
                available: parent.inner.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(parent.inner.buf, start);
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let min_fixed = <FuelFiguresDecoder<
            '_,
            sbe_rt::Detached,
        >>::min_readable_fixed_extent(parent.inner.acting_version);
        let min_entry_extent = if block_length > min_fixed {
            block_length
        } else {
            min_fixed
        };
        if count > 0 && block_length < min_fixed {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: min_fixed,
                available: block_length,
            });
        }
        Ok(Self {
            buf: parent.inner.buf,
            offset: start + 4,
            count,
            acting_block_length: block_length,
            acting_version: parent.inner.acting_version,
            min_entry_extent,
            parent,
        })
    }
    ///Generated method `remaining_entries`.
    #[inline]
    pub const fn remaining_entries(&self) -> usize {
        self.count
    }
    ///Generated method `is_empty`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
    ///Generated method `visit_entries`.
    #[inline]
    pub fn visit_entries<E, F>(mut self, mut visit: F) -> Result<(), E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnMut(&mut FuelFiguresEntryOrderedDecoder<'a>) -> Result<(), E>,
    {
        while self.count > 0 {
            let available = self.buf.len().saturating_sub(self.offset);
            if self.min_entry_extent > available {
                return Err(
                    E::from(sbe_rt::DecodeError::BufferTooShort {
                        field: "fuelFigures",
                        needed: self.min_entry_extent,
                        available,
                    }),
                );
            }
            let mut entry = FuelFiguresEntryOrderedDecoder::at(
                self.buf,
                self.offset,
                self.acting_block_length,
                self.acting_version,
            );
            visit(&mut entry)?;
            self.offset = entry.finish_unread()?;
            self.count -= 1;
        }
        self.commit();
        Ok(())
    }
    ///Generated method `finish`.
    #[inline]
    pub fn finish(mut self) -> Result<(), sbe_rt::DecodeError> {
        while self.count > 0 {
            self.offset = FuelFiguresEntryDecoder::skip(
                self.buf,
                self.offset,
                self.acting_block_length,
                self.acting_version,
            )?;
            self.count -= 1;
        }
        self.commit();
        Ok(())
    }
    ///Generated method `skip_remaining`.
    #[inline]
    pub fn skip_remaining(self) -> Result<(), sbe_rt::DecodeError> {
        self.finish()
    }
    #[inline]
    fn commit(self) {
        self.parent.tail_offset = self.offset;
        self.parent.next_ordinal = self.parent.next_ordinal.saturating_add(1);
    }
}
///Generated struct `FuelFiguresEntryOrderedDecoder`.
pub struct FuelFiguresEntryOrderedDecoder<'a> {
    inner: FuelFiguresEntryDecoder<'a>,
    tail_offset: usize,
    next_ordinal: u16,
}
impl<'a> FuelFiguresEntryOrderedDecoder<'a> {
    #[inline]
    fn at(
        buf: &'a [u8],
        offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        Self {
            inner: unsafe {
                FuelFiguresEntryDecoder::wrap(
                    buf,
                    offset,
                    acting_block_length,
                    acting_version,
                )
            },
            tail_offset: offset + acting_block_length,
            next_ordinal: 0,
        }
    }
    #[inline]
    fn expect(
        &self,
        ordinal: u16,
        requested: &'static str,
    ) -> Result<(), sbe_rt::DecodeError> {
        const NAMES: &[&str] = &["usageDescription"];
        let expected = if (self.next_ordinal as usize) < NAMES.len() {
            NAMES[self.next_ordinal as usize]
        } else {
            "<complete>"
        };
        if self.next_ordinal != ordinal {
            return Err(sbe_rt::DecodeError::OutOfOrder {
                owner: "FuelFigures",
                expected,
                requested,
            });
        }
        Ok(())
    }
    ///Generated method `speed`.
    #[inline]
    pub fn speed(&self) -> u16 {
        self.inner.speed()
    }
    ///Generated method `mpg`.
    #[inline]
    pub fn mpg(&self) -> f32 {
        self.inner.mpg()
    }
    ///Generated method `usage_description`.
    #[inline]
    pub fn usage_description(&mut self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.expect(0, "usageDescription")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "usageDescription",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "usageDescription",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "usageDescription",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(data)
    }
    ///Generated method `usage_description_as_str`.
    #[inline]
    pub fn usage_description_as_str(&mut self) -> Result<&'a str, sbe_rt::DecodeError> {
        self.expect(0, "usageDescription")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "usageDescription",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "usageDescription",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "usageDescription",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        if !data.is_ascii() {
            return Err(sbe_rt::DecodeError::InvalidAscii {
                field: "usageDescription",
            });
        }
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(unsafe { core::str::from_utf8_unchecked(data) })
    }
    ///Generated method `usage_description_as_message`.
    #[inline]
    pub fn usage_description_as_message(
        &mut self,
    ) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
        self.expect(0, "usageDescription")?;
        let (data, end) = {
            let offset = self.tail_offset;
            if offset + 4 > self.inner.buf.len() {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "usageDescription",
                    needed: 4,
                    available: self.inner.buf.len().saturating_sub(offset),
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(self.inner.buf, offset);
            let len = u32::from_le_bytes(bytes) as u64;
            if len > 1073741824 {
                return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                    field: "usageDescription",
                    length: len,
                    max_length: 1073741824 as u64,
                });
            }
            let (data_start, data_end) = sbe_rt::checked_var_data_bounds(
                "usageDescription",
                offset,
                4,
                len,
                self.inner.buf.len(),
            )?;
            (&self.inner.buf[data_start..data_end], data_end)
        };
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        self.tail_offset = end;
        self.next_ordinal = self.next_ordinal.saturating_add(1);
        Ok(frame)
    }
    #[inline]
    fn finish_unread(mut self) -> Result<usize, sbe_rt::DecodeError> {
        while (self.next_ordinal as usize) < 1 {
            match self.next_ordinal {
                0 => {
                    let _ = self.usage_description()?;
                }
                _ => break,
            }
        }
        Ok(self.tail_offset)
    }
}
///Generated struct `PerformanceFiguresOrderedDecoder`.
pub struct PerformanceFiguresOrderedDecoder<'p, 'a> {
    buf: &'a [u8],
    offset: usize,
    count: usize,
    acting_block_length: usize,
    acting_version: u16,
    min_entry_extent: usize,
    parent: &'p mut CarOrderedDecoder<'a>,
}
impl<'p, 'a> PerformanceFiguresOrderedDecoder<'p, 'a> {
    #[inline]
    fn begin(
        parent: &'p mut CarOrderedDecoder<'a>,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let start = parent.tail_offset;
        if 4 > parent.inner.buf.len().saturating_sub(start) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: 4,
                available: parent.inner.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(parent.inner.buf, start);
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let min_fixed = <PerformanceFiguresDecoder<
            '_,
            sbe_rt::Detached,
        >>::min_readable_fixed_extent(parent.inner.acting_version);
        let min_entry_extent = if block_length > min_fixed {
            block_length
        } else {
            min_fixed
        };
        if count > 0 && block_length < min_fixed {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: min_fixed,
                available: block_length,
            });
        }
        Ok(Self {
            buf: parent.inner.buf,
            offset: start + 4,
            count,
            acting_block_length: block_length,
            acting_version: parent.inner.acting_version,
            min_entry_extent,
            parent,
        })
    }
    ///Generated method `remaining_entries`.
    #[inline]
    pub const fn remaining_entries(&self) -> usize {
        self.count
    }
    ///Generated method `is_empty`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
    ///Generated method `visit_entries`.
    #[inline]
    pub fn visit_entries<E, F>(mut self, mut visit: F) -> Result<(), E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnMut(&mut PerformanceFiguresEntryOrderedDecoder<'a>) -> Result<(), E>,
    {
        while self.count > 0 {
            let available = self.buf.len().saturating_sub(self.offset);
            if self.min_entry_extent > available {
                return Err(
                    E::from(sbe_rt::DecodeError::BufferTooShort {
                        field: "performanceFigures",
                        needed: self.min_entry_extent,
                        available,
                    }),
                );
            }
            let mut entry = PerformanceFiguresEntryOrderedDecoder::at(
                self.buf,
                self.offset,
                self.acting_block_length,
                self.acting_version,
            );
            visit(&mut entry)?;
            self.offset = entry.finish_unread()?;
            self.count -= 1;
        }
        self.commit();
        Ok(())
    }
    ///Generated method `finish`.
    #[inline]
    pub fn finish(mut self) -> Result<(), sbe_rt::DecodeError> {
        while self.count > 0 {
            self.offset = PerformanceFiguresEntryDecoder::skip(
                self.buf,
                self.offset,
                self.acting_block_length,
                self.acting_version,
            )?;
            self.count -= 1;
        }
        self.commit();
        Ok(())
    }
    ///Generated method `skip_remaining`.
    #[inline]
    pub fn skip_remaining(self) -> Result<(), sbe_rt::DecodeError> {
        self.finish()
    }
    #[inline]
    fn commit(self) {
        self.parent.tail_offset = self.offset;
        self.parent.next_ordinal = self.parent.next_ordinal.saturating_add(1);
    }
}
///Generated struct `PerformanceFiguresEntryOrderedDecoder`.
pub struct PerformanceFiguresEntryOrderedDecoder<'a> {
    inner: PerformanceFiguresEntryDecoder<'a>,
    tail_offset: usize,
    next_ordinal: u16,
}
impl<'a> PerformanceFiguresEntryOrderedDecoder<'a> {
    #[inline]
    fn at(
        buf: &'a [u8],
        offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        Self {
            inner: unsafe {
                PerformanceFiguresEntryDecoder::wrap(
                    buf,
                    offset,
                    acting_block_length,
                    acting_version,
                )
            },
            tail_offset: offset + acting_block_length,
            next_ordinal: 0,
        }
    }
    #[inline]
    fn expect(
        &self,
        ordinal: u16,
        requested: &'static str,
    ) -> Result<(), sbe_rt::DecodeError> {
        const NAMES: &[&str] = &["acceleration"];
        let expected = if (self.next_ordinal as usize) < NAMES.len() {
            NAMES[self.next_ordinal as usize]
        } else {
            "<complete>"
        };
        if self.next_ordinal != ordinal {
            return Err(sbe_rt::DecodeError::OutOfOrder {
                owner: "PerformanceFigures",
                expected,
                requested,
            });
        }
        Ok(())
    }
    ///Generated method `octane_rating`.
    #[inline]
    pub fn octane_rating(&self) -> u8 {
        self.inner.octane_rating()
    }
    ///Generated method `acceleration`.
    #[inline]
    pub fn acceleration(
        &mut self,
    ) -> Result<
        PerformanceFiguresAccelerationOrderedDecoder<'_, 'a>,
        sbe_rt::DecodeError,
    > {
        self.expect(0, "acceleration")?;
        PerformanceFiguresAccelerationOrderedDecoder::begin_entry(self)
    }
    #[inline]
    fn finish_unread(mut self) -> Result<usize, sbe_rt::DecodeError> {
        while (self.next_ordinal as usize) < 1 {
            match self.next_ordinal {
                0 => {
                    self.acceleration()?.skip_remaining()?;
                }
                _ => break,
            }
        }
        Ok(self.tail_offset)
    }
}
///Generated struct `PerformanceFiguresAccelerationOrderedDecoder`.
pub struct PerformanceFiguresAccelerationOrderedDecoder<'p, 'a> {
    buf: &'a [u8],
    offset: usize,
    count: usize,
    acting_block_length: usize,
    acting_version: u16,
    min_entry_extent: usize,
    parent: &'p mut PerformanceFiguresEntryOrderedDecoder<'a>,
}
impl<'p, 'a> PerformanceFiguresAccelerationOrderedDecoder<'p, 'a> {
    #[inline]
    fn begin_entry(
        parent: &'p mut PerformanceFiguresEntryOrderedDecoder<'a>,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let start = parent.tail_offset;
        if 4 > parent.inner.buf.len().saturating_sub(start) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: 4,
                available: parent.inner.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(parent.inner.buf, start);
        let header = GroupSizeEncoding(bytes);
        let count = sbe_rt::checked_group_count(
            "numInGroup",
            header.num_in_group() as u64,
        )?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let min_entry_extent = 0usize;
        Ok(Self {
            buf: parent.inner.buf,
            offset: start + 4,
            count,
            acting_block_length: block_length,
            acting_version: parent.inner.acting_version,
            min_entry_extent,
            parent,
        })
    }
    ///Generated method `remaining_entries`.
    #[inline]
    pub const fn remaining_entries(&self) -> usize {
        self.count
    }
    ///Generated method `is_empty`.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.count == 0
    }
    ///Generated method `visit_entries`.
    #[inline]
    pub fn visit_entries<E, F>(mut self, mut visit: F) -> Result<(), E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnMut(
            &mut PerformanceFiguresAccelerationEntryOrderedDecoder<'a>,
        ) -> Result<(), E>,
    {
        while self.count > 0 {
            let mut entry = PerformanceFiguresAccelerationEntryOrderedDecoder::at(
                self.buf,
                self.offset,
                self.acting_block_length,
                self.acting_version,
            );
            visit(&mut entry)?;
            self.offset += self.acting_block_length;
            self.count -= 1;
        }
        self.commit();
        Ok(())
    }
    ///Generated method `finish`.
    #[inline]
    pub fn finish(mut self) -> Result<(), sbe_rt::DecodeError> {
        while self.count > 0 {
            self.offset = PerformanceFiguresAccelerationEntryDecoder::skip(
                self.buf,
                self.offset,
                self.acting_block_length,
                self.acting_version,
            )?;
            self.count -= 1;
        }
        self.commit();
        Ok(())
    }
    ///Generated method `skip_remaining`.
    #[inline]
    pub fn skip_remaining(self) -> Result<(), sbe_rt::DecodeError> {
        self.finish()
    }
    #[inline]
    fn commit(self) {
        self.parent.tail_offset = self.offset;
        self.parent.next_ordinal = self.parent.next_ordinal.saturating_add(1);
    }
}
///Generated struct `PerformanceFiguresAccelerationEntryOrderedDecoder`.
pub struct PerformanceFiguresAccelerationEntryOrderedDecoder<'a> {
    inner: PerformanceFiguresAccelerationEntryDecoder<'a>,
}
impl<'a> PerformanceFiguresAccelerationEntryOrderedDecoder<'a> {
    #[inline]
    fn at(
        buf: &'a [u8],
        offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        Self {
            inner: unsafe {
                PerformanceFiguresAccelerationEntryDecoder::wrap(
                    buf,
                    offset,
                    acting_block_length,
                    acting_version,
                )
            },
        }
    }
    ///Generated method `mph`.
    #[inline]
    pub fn mph(&self) -> u16 {
        self.inner.mph()
    }
    ///Generated method `seconds`.
    #[inline]
    pub fn seconds(&self) -> f32 {
        self.inner.seconds()
    }
}
/// Owned domain object — application-layer counterpart to the flyweight decoder.
///
/// Owned domain object — application-layer counterpart to the flyweight decoder.
///
/// Materialise with [`Self::try_from_decoder`] (from a decoder).
/// This is an inherent method, not `TryFrom`/`From`: conversion is never
/// infallible (groups, var-data, converters).
#[derive(Debug, Clone, PartialEq)]
pub struct CarFuelFiguresEntryDomain {
    ///Generated field `speed`.
    pub speed: u16,
    ///Generated field `mpg`.
    pub mpg: f32,
    ///Generated field `usage_description`.
    pub usage_description: Vec<u8>,
}
impl CarFuelFiguresEntryDomain {
    /// Fallible conversion from a flyweight decoder.
    ///
    /// Propagates decode errors from malformed group entries and var-data
    /// instead of panicking. Prefer this over `From`/`TryFrom`.
    #[inline]
    pub fn try_from_decoder(
        dec: FuelFiguresEntryDecoder<'_>,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Ok(Self {
            speed: dec.speed(),
            mpg: dec.mpg(),
            usage_description: match dec.usage_description() {
                Ok(data) => data.to_vec(),
                Err(e) => return Err(e),
            },
        })
    }
}
impl CarFuelFiguresEntryDomain {
    /// Encode this domain entry into a by-value entry encoder,
    /// returning the completeness proof required by dynamic
    /// group [`add`](crate).
    #[inline]
    pub fn encode_into<'a>(
        &self,
        mut enc: FuelFiguresEntryEncoder<'a>,
    ) -> Result<FuelFiguresEntryComplete<'a>, sbe_rt::EncodeError> {
        {
            let __v = self.speed as i128;
            if __v < 0 || __v > 65534 {
                return Err(sbe_rt::EncodeError::ValueOutOfRange {
                    field: "speed",
                    min: 0,
                    max: 65534,
                    actual: __v,
                });
            }
        }
        enc.speed(self.speed);
        enc.mpg(self.mpg);
        let enc = enc.usage_description(&self.usage_description)?;
        Ok(enc)
    }
    /// Compute this entry's contribution to the total encoded length
    /// (entry block + nested groups + entry var-data).
    #[inline]
    pub fn length_contribution(&self) -> Result<usize, sbe_rt::EncodeError> {
        let mut len: usize = 6;
        if self.usage_description.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "usageDescription",
                max_length: 1073741824,
                actual: self.usage_description.len(),
            });
        }
        len = len.checked_add(4).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        len = len
            .checked_add(self.usage_description.len())
            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        Ok(len)
    }
}
/// Owned domain object — application-layer counterpart to the flyweight decoder.
///
/// Owned domain object — application-layer counterpart to the flyweight decoder.
///
/// Materialise with [`Self::try_from_decoder`] (from a decoder).
/// This is an inherent method, not `TryFrom`/`From`: conversion is never
/// infallible (groups, var-data, converters).
#[derive(Debug, Clone, PartialEq)]
pub struct CarPerformanceFiguresEntryAccelerationEntryDomain {
    ///Generated field `mph`.
    pub mph: u16,
    ///Generated field `seconds`.
    pub seconds: f32,
}
impl CarPerformanceFiguresEntryAccelerationEntryDomain {
    /// Fallible conversion from a flyweight decoder.
    ///
    /// Propagates decode errors from malformed group entries and var-data
    /// instead of panicking. Prefer this over `From`/`TryFrom`.
    #[inline]
    pub fn try_from_decoder(
        dec: PerformanceFiguresAccelerationEntryDecoder<'_>,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Ok(Self {
            mph: dec.mph(),
            seconds: dec.seconds(),
        })
    }
}
impl CarPerformanceFiguresEntryAccelerationEntryDomain {
    ///Generated method `encode_into`.
    #[inline]
    pub fn encode_into<'a>(
        &self,
        enc: &mut PerformanceFiguresAccelerationEntryEncoder<'a>,
    ) -> Result<(), sbe_rt::EncodeError> {
        {
            let __v = self.mph as i128;
            if __v < 0 || __v > 65534 {
                return Err(sbe_rt::EncodeError::ValueOutOfRange {
                    field: "mph",
                    min: 0,
                    max: 65534,
                    actual: __v,
                });
            }
        }
        enc.mph(self.mph);
        enc.seconds(self.seconds);
        Ok(())
    }
    /// Compute this entry's contribution to the total encoded length
    /// (entry block + nested groups + entry var-data).
    #[inline]
    pub fn length_contribution(&self) -> Result<usize, sbe_rt::EncodeError> {
        let mut len: usize = 6;
        Ok(len)
    }
}
impl CarPerformanceFiguresEntryAccelerationEntryDomain {
    /// Convert to the wire entry struct for bulk encoding.
    #[must_use = "the converted wire entry is unused; ignoring it skips encoding"]
    #[inline]
    pub fn to_wire_entry(&self) -> PerformanceFiguresAccelerationEntry {
        PerformanceFiguresAccelerationEntry {
            mph: self.mph,
            seconds: self.seconds,
        }
    }
}
impl<'a> PerformanceFiguresAccelerationEncoder<'a> {
    /// Encode flat domain entries with one complete-region bounds check
    /// and no temporary wire-entry allocation.
    #[inline]
    pub fn bulk_add_domain(
        &mut self,
        entries: &[CarPerformanceFiguresEntryAccelerationEntryDomain],
    ) -> Result<(), sbe_rt::EncodeError> {
        self.bulk_add_with(
            entries,
            |entry, slot| {
                {
                    let __v = entry.mph as i128;
                    if __v < 0 || __v > 65534 {
                        return Err(sbe_rt::EncodeError::ValueOutOfRange {
                            field: "mph",
                            min: 0,
                            max: 65534,
                            actual: __v,
                        });
                    }
                }
                slot[0..0 + 2].copy_from_slice(&entry.mph.to_le_bytes());
                slot[2..2 + 4].copy_from_slice(&entry.seconds.to_le_bytes());
                Ok(())
            },
        )
    }
}
/// Owned domain object — application-layer counterpart to the flyweight decoder.
///
/// Owned domain object — application-layer counterpart to the flyweight decoder.
///
/// Materialise with [`Self::try_from_decoder`] (from a decoder).
/// This is an inherent method, not `TryFrom`/`From`: conversion is never
/// infallible (groups, var-data, converters).
#[derive(Debug, Clone, PartialEq)]
pub struct CarPerformanceFiguresEntryDomain {
    ///Generated field `octane_rating`.
    pub octane_rating: u8,
    ///Generated field `acceleration`.
    pub acceleration: Vec<CarPerformanceFiguresEntryAccelerationEntryDomain>,
}
impl CarPerformanceFiguresEntryDomain {
    /// Fallible conversion from a flyweight decoder.
    ///
    /// Propagates decode errors from malformed group entries and var-data
    /// instead of panicking. Prefer this over `From`/`TryFrom`.
    #[inline]
    pub fn try_from_decoder(
        dec: PerformanceFiguresEntryDecoder<'_>,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Ok(Self {
            octane_rating: dec.octane_rating(),
            acceleration: dec
                .acceleration()
                .map(|g| {
                    g.map(
                            CarPerformanceFiguresEntryAccelerationEntryDomain::try_from_decoder,
                        )
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap_or_else(|e| Err(e))?,
        })
    }
}
impl CarPerformanceFiguresEntryDomain {
    /// Encode this domain entry into a by-value entry encoder,
    /// returning the completeness proof required by dynamic
    /// group [`add`](crate).
    #[inline]
    pub fn encode_into<'a>(
        &self,
        mut enc: PerformanceFiguresEntryEncoder<'a>,
    ) -> Result<PerformanceFiguresEntryComplete<'a>, sbe_rt::EncodeError> {
        {
            let __v = self.octane_rating as i128;
            if __v < 90 || __v > 110 {
                return Err(sbe_rt::EncodeError::ValueOutOfRange {
                    field: "octaneRating",
                    min: 90,
                    max: 110,
                    actual: __v,
                });
            }
        }
        enc.octane_rating(self.octane_rating);
        let count = <u16>::try_from(self.acceleration.len())
            .map_err(|_| {
                sbe_rt::EncodeError::ValueOutOfRange {
                    field: "group count",
                    min: 0,
                    max: u16::MAX as i128,
                    actual: self.acceleration.len() as i128,
                }
            })?;
        let enc = enc.acceleration(count, |g| g.bulk_add_domain(&self.acceleration))?;
        Ok(enc)
    }
    /// Compute this entry's contribution to the total encoded length
    /// (entry block + nested groups + entry var-data).
    #[inline]
    pub fn length_contribution(&self) -> Result<usize, sbe_rt::EncodeError> {
        let mut len: usize = 6;
        len = len.checked_add(4).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        for entry in &self.acceleration {
            len = len
                .checked_add(entry.length_contribution()?)
                .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        }
        Ok(len)
    }
}
/// Owned domain object — application-layer counterpart to the flyweight decoder.
///
/// Materialise with [`Self::try_from_decoder`] (from a decoder) or
/// [`Self::try_from_slice_with_header`] (from framed bytes).
/// These are inherent methods, not `TryFrom`/`From`: there are two fallible
/// sources, and conversion is never infallible (groups, var-data, converters).
#[derive(Debug, Clone, PartialEq)]
pub struct CarDomain {
    ///Generated field `serial_number`.
    pub serial_number: u64,
    ///Generated field `model_year`.
    pub model_year: u16,
    ///Generated field `available`.
    pub available: bool,
    ///Generated field `code`.
    pub code: Model,
    ///Generated field `some_numbers`.
    pub some_numbers: [u32; 4],
    ///Generated field `vehicle_code`.
    pub vehicle_code: [u8; 6],
    ///Generated field `extras`.
    pub extras: OptionalExtras,
    ///Generated field `engine`.
    pub engine: Engine,
    ///Generated field `fuel_figures`.
    pub fuel_figures: Vec<CarFuelFiguresEntryDomain>,
    ///Generated field `performance_figures`.
    pub performance_figures: Vec<CarPerformanceFiguresEntryDomain>,
    ///Generated field `manufacturer`.
    pub manufacturer: Vec<u8>,
    ///Generated field `model`.
    pub model: Vec<u8>,
    ///Generated field `activation_code`.
    pub activation_code: Vec<u8>,
}
impl CarDomain {
    /// Fallible conversion from a flyweight decoder.
    ///
    /// Propagates decode errors from malformed group entries and var-data
    /// instead of panicking. Prefer this over `From`/`TryFrom`: the companion
    /// entry point is [`Self::try_from_slice_with_header`] (when generated),
    /// and named methods make the two sources unambiguous.
    #[inline]
    pub fn try_from_decoder(dec: CarDecoder<'_>) -> Result<Self, sbe_rt::DecodeError> {
        Ok(Self {
            serial_number: dec.serial_number(),
            model_year: dec.model_year(),
            available: dec.try_available_bool()?,
            code: dec.code(),
            some_numbers: dec.some_numbers(),
            vehicle_code: dec.vehicle_code(),
            extras: dec.extras(),
            engine: dec.engine_value(),
            fuel_figures: dec
                .fuel_figures()
                .map(|g| {
                    g.map(|r| {
                            r.and_then(|entry| CarFuelFiguresEntryDomain::try_from_decoder(
                                entry,
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap_or_else(|e| Err(e))?,
            performance_figures: dec
                .performance_figures()
                .map(|g| {
                    g.map(|r| {
                            r.and_then(|entry| CarPerformanceFiguresEntryDomain::try_from_decoder(
                                entry,
                            ))
                        })
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap_or_else(|e| Err(e))?,
            manufacturer: match dec.manufacturer() {
                Ok(data) => data.to_vec(),
                Err(e) => return Err(e),
            },
            model: match dec.model() {
                Ok(data) => data.to_vec(),
                Err(e) => return Err(e),
            },
            activation_code: match dec.activation_code() {
                Ok(data) => data.to_vec(),
                Err(e) => return Err(e),
            },
        })
    }
    /// Decode from a framed byte slice: validate the message header, then
    /// materialise the full domain object.
    ///
    /// Distinct from [`Self::try_from_decoder`]: this path owns header
    /// validation + offset; that path starts from an already-wrapped decoder.
    /// Named methods (not `TryFrom`/`From`) keep the two sources obvious.
    #[inline]
    pub fn try_from_slice_with_header(
        buf: &[u8],
        message_offset: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Self::try_from_decoder(CarDecoder::decode(buf, message_offset)?)
    }
}
impl CarDomain {
    ///Generated method `encode`.
    #[inline]
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
        let mut enc = CarEncoder::try_wrap_and_apply_header(buf, 0)?;
        {
            let __v = self.serial_number as i128;
            if __v < 0 || __v > 18446744073709551614 {
                return Err(sbe_rt::EncodeError::ValueOutOfRange {
                    field: "serialNumber",
                    min: 0,
                    max: 18446744073709551614,
                    actual: __v,
                });
            }
        }
        enc.serial_number(self.serial_number);
        {
            let __v = self.model_year as i128;
            if __v < 0 || __v > 65534 {
                return Err(sbe_rt::EncodeError::ValueOutOfRange {
                    field: "modelYear",
                    min: 0,
                    max: 65534,
                    actual: __v,
                });
            }
        }
        enc.model_year(self.model_year);
        enc.available_bool(self.available);
        enc.code(self.code);
        enc.some_numbers(self.some_numbers);
        enc.vehicle_code(self.vehicle_code);
        enc.extras(self.extras);
        enc.engine(self.engine);
        let enc = CarEncoder {
            buf: enc.buf,
            msg_offset: enc.msg_offset,
            offset: enc.offset,
            _header: core::marker::PhantomData::<sbe_rt::HeaderPresent>,
            _fields: core::marker::PhantomData::<sbe_rt::FieldsFixed>,
        };
        let count = <u16>::try_from(self.fuel_figures.len())
            .map_err(|_| {
                sbe_rt::EncodeError::ValueOutOfRange {
                    field: "group count",
                    min: 0,
                    max: u16::MAX as i128,
                    actual: self.fuel_figures.len() as i128,
                }
            })?;
        let enc = enc
            .fuel_figures(
                count,
                |g| -> Result<(), sbe_rt::EncodeError> {
                    for e in &self.fuel_figures {
                        g.add(|entry| e.encode_into(entry))?;
                    }
                    Ok(())
                },
            )?;
        let count = <u16>::try_from(self.performance_figures.len())
            .map_err(|_| {
                sbe_rt::EncodeError::ValueOutOfRange {
                    field: "group count",
                    min: 0,
                    max: u16::MAX as i128,
                    actual: self.performance_figures.len() as i128,
                }
            })?;
        let enc = enc
            .performance_figures(
                count,
                |g| -> Result<(), sbe_rt::EncodeError> {
                    for e in &self.performance_figures {
                        g.add(|entry| e.encode_into(entry))?;
                    }
                    Ok(())
                },
            )?;
        let enc = enc.manufacturer(&self.manufacturer)?;
        let enc = enc.model(&self.model)?;
        let enc = enc.activation_code(&self.activation_code)?;
        Ok(enc.encoded_length() + CarEncoder::HEADER_LENGTH)
    }
    /// Compute the exact SBE message **body** length (no header)
    /// from this domain object. [`Self::encode`] always writes
    /// the header too, so its return value is
    /// [`Self::encoded_length_with_header`], not this — sizing a
    /// buffer from `encoded_length()` alone under-allocates by
    /// the message header size.
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::EncodeError> {
        let mut len: usize = 45;
        len = len.checked_add(4).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        for entry in &self.fuel_figures {
            len = len
                .checked_add(entry.length_contribution()?)
                .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        }
        len = len.checked_add(4).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        for entry in &self.performance_figures {
            len = len
                .checked_add(entry.length_contribution()?)
                .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        }
        if self.manufacturer.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "manufacturer",
                max_length: 1073741824,
                actual: self.manufacturer.len(),
            });
        }
        len = len.checked_add(4).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        len = len
            .checked_add(self.manufacturer.len())
            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        if self.model.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "model",
                max_length: 1073741824,
                actual: self.model.len(),
            });
        }
        len = len.checked_add(4).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        len = len
            .checked_add(self.model.len())
            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        if self.activation_code.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "activationCode",
                max_length: 1073741824,
                actual: self.activation_code.len(),
            });
        }
        len = len.checked_add(4).ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        len = len
            .checked_add(self.activation_code.len())
            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        Ok(len)
    }
    /// Compute the exact buffer size [`Self::encode`] needs and
    /// exactly what it returns on success, for both fixed and
    /// dynamic (group/var-data-bearing) messages.
    #[inline]
    pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::EncodeError> {
        Ok(self.encoded_length()? + CarEncoder::HEADER_LENGTH)
    }
}
///Description of a basic Car
#[doc = concat!(
    "Encoder stage `", "CarEncoder", "` — call `fixed(&FixedFields)` before tails."
)]
#[must_use = "encoder must be consumed to write the message"]
pub struct CarEncoder<
    'a,
    H: sbe_rt::HeaderState = sbe_rt::HeaderPresent,
    F: sbe_rt::FieldsState = sbe_rt::FieldsUnfixed,
> {
    buf: &'a mut [u8],
    msg_offset: usize,
    offset: usize,
    _header: core::marker::PhantomData<H>,
    _fields: core::marker::PhantomData<F>,
}
impl<'a, H: sbe_rt::HeaderState, F: sbe_rt::FieldsState> core::fmt::Display
for CarEncoder<'a, H, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarEncoder"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState, F: sbe_rt::FieldsState> core::fmt::Debug
for CarEncoder<'a, H, F> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarEncoder")
                    .field("msg_offset", &self.msg_offset)
                    .field("offset", &self.offset)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[doc = concat!(
    "Encoder stage `", "CarAfterFuelFigures", "` — write tail elements in wire order."
)]
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterFuelFigures<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    offset: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for CarAfterFuelFigures<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarAfterFuelFigures"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarAfterFuelFigures<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarAfterFuelFigures")
                    .field("msg_offset", &self.msg_offset)
                    .field("offset", &self.offset)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[doc = concat!(
    "Encoder stage `", "CarAfterPerformanceFigures",
    "` — write tail elements in wire order."
)]
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterPerformanceFigures<
    'a,
    H: sbe_rt::HeaderState = sbe_rt::HeaderPresent,
> {
    buf: &'a mut [u8],
    msg_offset: usize,
    offset: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display
for CarAfterPerformanceFigures<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarAfterPerformanceFigures"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarAfterPerformanceFigures<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarAfterPerformanceFigures")
                    .field("msg_offset", &self.msg_offset)
                    .field("offset", &self.offset)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[doc = concat!(
    "Encoder stage `", "CarAfterManufacturer", "` — write tail elements in wire order."
)]
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterManufacturer<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    offset: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for CarAfterManufacturer<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarAfterManufacturer"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarAfterManufacturer<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarAfterManufacturer")
                    .field("msg_offset", &self.msg_offset)
                    .field("offset", &self.offset)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[doc = concat!(
    "Encoder stage `", "CarAfterModel", "` — write tail elements in wire order."
)]
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterModel<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    offset: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for CarAfterModel<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarAfterModel"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarAfterModel<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarAfterModel")
                    .field("msg_offset", &self.msg_offset)
                    .field("offset", &self.offset)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[doc = concat!(
    "Encoder stage `", "CarComplete", "` — write tail elements in wire order."
)]
#[must_use = "encoder must be consumed to write the message"]
pub struct CarComplete<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    offset: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for CarComplete<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarComplete"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarComplete<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::decode(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarComplete")
                    .field("msg_offset", &self.msg_offset)
                    .field("offset", &self.offset)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
/// Complete set of latest-version fixed fields for this message.
/// Required fields (including `sinceVersion` fields) are concrete
/// values; only presence-optional fields are `Option<T>`. Constants
/// are excluded.
///
/// This struct is **intentionally exhaustive** (not
/// `#[non_exhaustive]`): when the schema adds a fixed field, every
/// `fixed(&…)` call site must be updated. That is a feature — schema
/// changes surface as compile errors rather than silent defaults.
#[derive(Debug, Clone)]
pub struct CarFixedFields {
    ///Generated field `serial_number`.
    pub serial_number: u64,
    ///Generated field `model_year`.
    pub model_year: u16,
    ///Generated field `available`.
    pub available: BooleanType,
    ///Generated field `code`.
    pub code: Model,
    ///Generated field `some_numbers`.
    pub some_numbers: [u32; 4],
    ///Generated field `vehicle_code`.
    pub vehicle_code: [u8; 6],
    ///Generated field `extras`.
    pub extras: OptionalExtras,
    ///Generated field `engine`.
    pub engine: Engine,
}
///Raw fixed-field writer. Individual field setters are available only on this writer. When done, embed the fields in a [`CarFixedFields`] and call the encoder's `fixed()`.
#[must_use = "raw fixed writer must be embedded in FixedFields"]
pub struct CarRawFixedWriter<'a> {
    buf: &'a mut [u8],
    msg_offset: usize,
    offset: usize,
}
impl<'a> CarRawFixedWriter<'a> {
    ///Generated method `serial_number`.
    #[inline]
    pub fn serial_number(&mut self, val: u64) -> &mut Self {
        let offset = self.msg_offset + 0;
        unsafe {
            self.buf
                .get_unchecked_mut(offset..offset + 8)
                .copy_from_slice(&val.to_le_bytes());
        }
        self
    }
    ///Generated method `model_year`.
    #[inline]
    pub fn model_year(&mut self, val: u16) -> &mut Self {
        let offset = self.msg_offset + 8;
        unsafe {
            self.buf
                .get_unchecked_mut(offset..offset + 2)
                .copy_from_slice(&val.to_le_bytes());
        }
        self
    }
    ///Generated method `available`.
    #[inline]
    pub fn available(&mut self, val: BooleanType) -> &mut Self {
        let offset = self.msg_offset + 10;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    ///Generated method `available_bool`.
    #[inline]
    pub fn available_bool(&mut self, val: bool) -> &mut Self {
        self.buf[self.msg_offset + 10] = val as u8;
        self
    }
    ///Generated method `code`.
    #[inline]
    pub fn code(&mut self, val: Model) -> &mut Self {
        let offset = self.msg_offset + 11;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    ///Generated method `some_numbers`.
    #[inline]
    pub fn some_numbers(&mut self, val: [u32; 4]) -> &mut Self {
        let offset = self.msg_offset + 12;
        let mut idx = 0usize;
        while idx < 4 {
            unsafe {
                self.buf
                    .get_unchecked_mut(offset + idx * 4..offset + (idx + 1) * 4)
                    .copy_from_slice(&val[idx].to_le_bytes());
            }
            idx += 1;
        }
        self
    }
    ///Generated method `put_some_numbers`.
    #[inline]
    pub fn put_some_numbers(&mut self, v0: u32, v1: u32, v2: u32, v3: u32) -> &mut Self {
        self.some_numbers([v0, v1, v2, v3])
    }
    ///Generated method `vehicle_code`.
    #[inline]
    pub fn vehicle_code(&mut self, val: [u8; 6]) -> &mut Self {
        let offset = self.msg_offset + 28;
        unsafe {
            let dst = self.buf.get_unchecked_mut(offset..offset + 6);
            let src = core::slice::from_raw_parts(val.as_ptr() as *const u8, 6);
            dst.copy_from_slice(src);
        }
        self
    }
    ///Generated method `vehicle_code_str`.
    #[inline]
    pub fn vehicle_code_str(
        &mut self,
        src: &str,
    ) -> Result<&mut Self, sbe_rt::EncodeError> {
        if !src.is_ascii() {
            return Err(sbe_rt::EncodeError::InvalidAscii {
                field: "vehicleCode",
            });
        }
        if src.len() > 6 {
            return Err(sbe_rt::EncodeError::FixedArrayTooLong {
                field: "vehicleCode",
                max_length: 6,
                actual: src.len(),
            });
        }
        let mut tmp = [0 as u8; 6];
        let bytes = src.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            tmp[i] = bytes[i] as u8;
            i += 1;
        }
        Ok(self.vehicle_code(tmp))
    }
    ///Generated method `put_vehicle_code`.
    #[inline]
    pub fn put_vehicle_code(
        &mut self,
        v0: u8,
        v1: u8,
        v2: u8,
        v3: u8,
        v4: u8,
        v5: u8,
    ) -> &mut Self {
        self.vehicle_code([v0, v1, v2, v3, v4, v5])
    }
    ///Generated method `extras`.
    #[inline]
    pub fn extras(&mut self, val: OptionalExtras) -> &mut Self {
        let offset = self.msg_offset + 34;
        self.buf[offset..offset + 1].copy_from_slice(&val.0.to_le_bytes());
        self
    }
    ///Generated method `engine`.
    #[inline]
    pub fn engine(&mut self, val: Engine) -> &mut Self {
        let offset = self.msg_offset + 35;
        self.buf[offset..offset + 10].copy_from_slice(&val.0);
        self
    }
}
impl<'a> CarEncoder<'a> {
    ///`SCHEMA_ID` = 1.
    pub const SCHEMA_ID: u16 = 1;
    ///`SCHEMA_VERSION` = 0.
    pub const SCHEMA_VERSION: u16 = 0;
    ///`TEMPLATE_ID` = 1.
    pub const TEMPLATE_ID: u16 = 1;
    ///`BLOCK_LENGTH` = 45.
    pub const BLOCK_LENGTH: usize = 45;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 45);
    /// Schema-declared message header size in bytes.
    pub const HEADER_LENGTH: usize = 8;
    ///Generated constant `HEADER_TEMPLATE`.
    pub const HEADER_TEMPLATE: [u8; 8] = [45, 0, 1, 0, 1, 0, 0, 0];
    const _HEADER_TEMPLATE_LEN: () = assert!(Self::HEADER_TEMPLATE.len() == 8);
    ///Generated method `compute_length`.
    #[inline]
    pub const fn compute_length() -> CarEncodedLength {
        CarEncodedLength::new()
    }
    /// Cold error constructor — never inlined into the hot path.
    #[cold]
    #[inline(never)]
    fn buffer_too_short(
        buf: &[u8],
        offset: usize,
        needed: usize,
    ) -> sbe_rt::EncodeError {
        sbe_rt::EncodeError::BufferTooShort {
            field: "message header+body",
            needed,
            available: buf.len().saturating_sub(offset),
        }
    }
    /// Wrap a mutable buffer for encoding with one bounds/overflow check.
    /// Does **not** write the message header (`HeaderAbsent`).
    ///
    /// `msg_offset` is the **message start** (first byte of the SBE frame),
    /// not the body. sbe-tool Rust `wrap` takes the body offset instead.
    ///
    /// Prefer [`Self::wrap_and_apply_header`] when encoding a full frame.
    #[inline]
    pub fn try_wrap(
        buf: &'a mut [u8],
        msg_offset: usize,
    ) -> Result<CarEncoder<'a, sbe_rt::HeaderAbsent>, sbe_rt::EncodeError> {
        if 53 > buf.len().saturating_sub(msg_offset) {
            return Err(Self::buffer_too_short(buf, msg_offset, 53));
        }
        Ok(unsafe { Self::wrap_unchecked(buf, msg_offset) })
    }
    /// Trusted body-only wrap. Proves header + fixed-body extent then
    /// constructs; **panics** if the buffer is too short. Field setters
    /// use unchecked writes justified by that proof.
    ///
    /// Prefer [`Self::try_wrap`] at untrusted boundaries.
    #[inline]
    pub fn wrap(
        buf: &'a mut [u8],
        msg_offset: usize,
    ) -> CarEncoder<'a, sbe_rt::HeaderAbsent> {
        if 53 > buf.len().saturating_sub(msg_offset) {
            panic!("{}", Self::buffer_too_short(buf, msg_offset, 53));
        }
        unsafe { Self::wrap_unchecked(buf, msg_offset) }
    }
    /// Zero-check body-only wrap — raw pointer ops, **UB** on OOB.
    /// Only for proven-tight hot loops where the panic machinery is
    /// measurable in the critical path.
    ///
    /// # Safety
    /// `msg_offset + HEADER_LENGTH + BLOCK_LENGTH` must not overflow
    /// and must be ≤ `buf.len()` for the lifetime of the encoder.
    #[inline]
    pub unsafe fn wrap_unchecked(
        buf: &'a mut [u8],
        msg_offset: usize,
    ) -> CarEncoder<'a, sbe_rt::HeaderAbsent> {
        let body_offset = msg_offset + 8;
        CarEncoder {
            buf,
            msg_offset,
            offset: body_offset + 45,
            _header: core::marker::PhantomData,
            _fields: core::marker::PhantomData,
        }
    }
    /// Wrap a mutable buffer, write the header, with one bounds/overflow check.
    /// `offset` is the **message start** (see [`Self::wrap`]).
    ///
    /// Optional-field nullification is **not** applied by default — call
    /// `apply_nulls()` if you want null sentinels.
    #[inline]
    pub fn try_wrap_and_apply_header(
        buf: &'a mut [u8],
        offset: usize,
    ) -> Result<CarEncoder<'a, sbe_rt::HeaderPresent>, sbe_rt::EncodeError> {
        if 53 > buf.len().saturating_sub(offset) {
            return Err(Self::buffer_too_short(buf, offset, 53));
        }
        Ok(unsafe { Self::wrap_and_apply_header_unchecked(buf, offset) })
    }
    /// Trusted full-frame wrap + header. Proves header + fixed-body extent
    /// then writes the header; **panics** if the buffer is too short.
    /// Field setters use unchecked writes justified by that proof.
    ///
    /// Prefer [`Self::try_wrap_and_apply_header`] at untrusted boundaries.
    /// Call [`Self::wrap_and_apply_header_unchecked`] only with a proven
    /// extent when even panic machinery must be avoided.
    #[inline]
    pub fn wrap_and_apply_header(
        buf: &'a mut [u8],
        offset: usize,
    ) -> CarEncoder<'a, sbe_rt::HeaderPresent> {
        if 53 > buf.len().saturating_sub(offset) {
            panic!("{}", Self::buffer_too_short(buf, offset, 53));
        }
        unsafe { Self::wrap_and_apply_header_unchecked(buf, offset) }
    }
    /// Zero-check full-frame wrap + header — `copy_nonoverlapping`, **UB**
    /// on OOB. Only for proven-tight hot loops.
    ///
    /// # Safety
    /// `offset + HEADER_LENGTH + BLOCK_LENGTH` must not overflow and must be
    /// ≤ `buf.len()` for the lifetime of the encoder.
    #[inline]
    pub unsafe fn wrap_and_apply_header_unchecked(
        buf: &'a mut [u8],
        offset: usize,
    ) -> CarEncoder<'a, sbe_rt::HeaderPresent> {
        unsafe {
            core::ptr::copy_nonoverlapping(
                Self::HEADER_TEMPLATE.as_ptr(),
                buf.as_mut_ptr().add(offset),
                8,
            );
        }
        let body_offset = offset + 8;
        CarEncoder {
            buf,
            msg_offset: offset,
            offset: body_offset + 45,
            _header: core::marker::PhantomData,
            _fields: core::marker::PhantomData,
        }
    }
    ///`SERIAL_NUMBER_ID` = 1.
    pub const SERIAL_NUMBER_ID: u16 = 1;
    ///`SERIAL_NUMBER_SINCE_VERSION` = 0.
    pub const SERIAL_NUMBER_SINCE_VERSION: u16 = 0;
    ///`SERIAL_NUMBER_ENCODING_OFFSET` = 0.
    pub const SERIAL_NUMBER_ENCODING_OFFSET: usize = 0;
    ///`SERIAL_NUMBER_ENCODING_LENGTH` = 8.
    pub const SERIAL_NUMBER_ENCODING_LENGTH: usize = 8;
    ///Generated method `serial_number_meta_attribute`.
    #[inline]
    pub const fn serial_number_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`SERIAL_NUMBER_NULL` = 18446744073709551615.
    pub const SERIAL_NUMBER_NULL: u64 = 18446744073709551615_u64;
    ///`SERIAL_NUMBER_MIN` = 0.
    pub const SERIAL_NUMBER_MIN: u64 = 0_u64;
    ///`SERIAL_NUMBER_MAX` = 18446744073709551614.
    pub const SERIAL_NUMBER_MAX: u64 = 18446744073709551614_u64;
    ///`MODEL_YEAR_ID` = 2.
    pub const MODEL_YEAR_ID: u16 = 2;
    ///`MODEL_YEAR_SINCE_VERSION` = 0.
    pub const MODEL_YEAR_SINCE_VERSION: u16 = 0;
    ///`MODEL_YEAR_ENCODING_OFFSET` = 8.
    pub const MODEL_YEAR_ENCODING_OFFSET: usize = 8;
    ///`MODEL_YEAR_ENCODING_LENGTH` = 2.
    pub const MODEL_YEAR_ENCODING_LENGTH: usize = 2;
    ///Generated method `model_year_meta_attribute`.
    #[inline]
    pub const fn model_year_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`MODEL_YEAR_NULL` = 65535.
    pub const MODEL_YEAR_NULL: u16 = 65535_u16;
    ///`MODEL_YEAR_MIN` = 0.
    pub const MODEL_YEAR_MIN: u16 = 0_u16;
    ///`MODEL_YEAR_MAX` = 65534.
    pub const MODEL_YEAR_MAX: u16 = 65534_u16;
    ///`AVAILABLE_ID` = 3.
    pub const AVAILABLE_ID: u16 = 3;
    ///`AVAILABLE_SINCE_VERSION` = 0.
    pub const AVAILABLE_SINCE_VERSION: u16 = 0;
    ///`AVAILABLE_ENCODING_OFFSET` = 10.
    pub const AVAILABLE_ENCODING_OFFSET: usize = 10;
    ///`AVAILABLE_ENCODING_LENGTH` = 1.
    pub const AVAILABLE_ENCODING_LENGTH: usize = 1;
    ///Generated method `available_meta_attribute`.
    #[inline]
    pub const fn available_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`AVAILABLE_NULL` = BooleanType::NullVal.
    pub const AVAILABLE_NULL: BooleanType = BooleanType::NullVal;
    ///`CODE_ID` = 4.
    pub const CODE_ID: u16 = 4;
    ///`CODE_SINCE_VERSION` = 0.
    pub const CODE_SINCE_VERSION: u16 = 0;
    ///`CODE_ENCODING_OFFSET` = 11.
    pub const CODE_ENCODING_OFFSET: usize = 11;
    ///`CODE_ENCODING_LENGTH` = 1.
    pub const CODE_ENCODING_LENGTH: usize = 1;
    ///Generated method `code_meta_attribute`.
    #[inline]
    pub const fn code_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`CODE_NULL` = Model::NullVal.
    pub const CODE_NULL: Model = Model::NullVal;
    ///`SOME_NUMBERS_ID` = 5.
    pub const SOME_NUMBERS_ID: u16 = 5;
    ///`SOME_NUMBERS_SINCE_VERSION` = 0.
    pub const SOME_NUMBERS_SINCE_VERSION: u16 = 0;
    ///`SOME_NUMBERS_ENCODING_OFFSET` = 12.
    pub const SOME_NUMBERS_ENCODING_OFFSET: usize = 12;
    ///`SOME_NUMBERS_ENCODING_LENGTH` = 16.
    pub const SOME_NUMBERS_ENCODING_LENGTH: usize = 16;
    ///Generated method `some_numbers_meta_attribute`.
    #[inline]
    pub const fn some_numbers_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`SOME_NUMBERS_NULL` = 4294967295.
    pub const SOME_NUMBERS_NULL: u32 = 4294967295_u32;
    ///`SOME_NUMBERS_MIN` = 0.
    pub const SOME_NUMBERS_MIN: u32 = 0_u32;
    ///`SOME_NUMBERS_MAX` = 4294967294.
    pub const SOME_NUMBERS_MAX: u32 = 4294967294_u32;
    ///`VEHICLE_CODE_ID` = 6.
    pub const VEHICLE_CODE_ID: u16 = 6;
    ///`VEHICLE_CODE_SINCE_VERSION` = 0.
    pub const VEHICLE_CODE_SINCE_VERSION: u16 = 0;
    ///`VEHICLE_CODE_ENCODING_OFFSET` = 28.
    pub const VEHICLE_CODE_ENCODING_OFFSET: usize = 28;
    ///`VEHICLE_CODE_ENCODING_LENGTH` = 6.
    pub const VEHICLE_CODE_ENCODING_LENGTH: usize = 6;
    ///Generated method `vehicle_code_meta_attribute`.
    #[inline]
    pub const fn vehicle_code_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`VEHICLE_CODE_NULL` = 0.
    pub const VEHICLE_CODE_NULL: u8 = 0_u8;
    ///`VEHICLE_CODE_MIN` = 32.
    pub const VEHICLE_CODE_MIN: u8 = 32_u8;
    ///`VEHICLE_CODE_MAX` = 126.
    pub const VEHICLE_CODE_MAX: u8 = 126_u8;
    ///`EXTRAS_ID` = 7.
    pub const EXTRAS_ID: u16 = 7;
    ///`EXTRAS_SINCE_VERSION` = 0.
    pub const EXTRAS_SINCE_VERSION: u16 = 0;
    ///`EXTRAS_ENCODING_OFFSET` = 34.
    pub const EXTRAS_ENCODING_OFFSET: usize = 34;
    ///`EXTRAS_ENCODING_LENGTH` = 1.
    pub const EXTRAS_ENCODING_LENGTH: usize = 1;
    ///Generated method `extras_meta_attribute`.
    #[inline]
    pub const fn extras_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
    ///`ENGINE_ID` = 9.
    pub const ENGINE_ID: u16 = 9;
    ///`ENGINE_SINCE_VERSION` = 0.
    pub const ENGINE_SINCE_VERSION: u16 = 0;
    ///`ENGINE_ENCODING_OFFSET` = 35.
    pub const ENGINE_ENCODING_OFFSET: usize = 35;
    ///`ENGINE_ENCODING_LENGTH` = 10.
    pub const ENGINE_ENCODING_LENGTH: usize = 10;
    ///Generated method `engine_meta_attribute`.
    #[inline]
    pub const fn engine_meta_attribute(
        attr: sbe_rt::MetaAttribute,
    ) -> Option<&'static str> {
        match attr {
            sbe_rt::MetaAttribute::Epoch => None,
            sbe_rt::MetaAttribute::TimeUnit => None,
            sbe_rt::MetaAttribute::SemanticType => None,
            sbe_rt::MetaAttribute::Presence => Some("required"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> CarEncoder<'a, H, sbe_rt::FieldsUnfixed> {
    ///Generated method `serial_number`.
    #[inline]
    pub fn serial_number(&mut self, val: u64) -> &mut Self {
        let offset = self.msg_offset + 8;
        unsafe {
            self.buf
                .get_unchecked_mut(offset..offset + 8)
                .copy_from_slice(&val.to_le_bytes());
        }
        self
    }
    ///Generated method `model_year`.
    #[inline]
    pub fn model_year(&mut self, val: u16) -> &mut Self {
        let offset = self.msg_offset + 16;
        unsafe {
            self.buf
                .get_unchecked_mut(offset..offset + 2)
                .copy_from_slice(&val.to_le_bytes());
        }
        self
    }
    ///Generated method `available`.
    #[inline]
    pub fn available(&mut self, val: BooleanType) -> &mut Self {
        let offset = self.msg_offset + 18;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    ///Generated method `available_bool`.
    #[inline]
    pub fn available_bool(&mut self, val: bool) -> &mut Self {
        self.buf[self.msg_offset + 18] = val as u8;
        self
    }
    ///Generated method `code`.
    #[inline]
    pub fn code(&mut self, val: Model) -> &mut Self {
        let offset = self.msg_offset + 19;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    ///Generated method `some_numbers`.
    #[inline]
    pub fn some_numbers(&mut self, val: [u32; 4]) -> &mut Self {
        let offset = self.msg_offset + 20;
        let mut idx = 0usize;
        while idx < 4 {
            unsafe {
                self.buf
                    .get_unchecked_mut(offset + idx * 4..offset + (idx + 1) * 4)
                    .copy_from_slice(&val[idx].to_le_bytes());
            }
            idx += 1;
        }
        self
    }
    ///Generated method `put_some_numbers`.
    #[inline]
    pub fn put_some_numbers(&mut self, v0: u32, v1: u32, v2: u32, v3: u32) -> &mut Self {
        self.some_numbers([v0, v1, v2, v3])
    }
    ///Generated method `vehicle_code`.
    #[inline]
    pub fn vehicle_code(&mut self, val: [u8; 6]) -> &mut Self {
        let offset = self.msg_offset + 36;
        unsafe {
            let dst = self.buf.get_unchecked_mut(offset..offset + 6);
            let src = core::slice::from_raw_parts(val.as_ptr() as *const u8, 6);
            dst.copy_from_slice(src);
        }
        self
    }
    ///Generated method `vehicle_code_str`.
    #[inline]
    pub fn vehicle_code_str(
        &mut self,
        src: &str,
    ) -> Result<&mut Self, sbe_rt::EncodeError> {
        if !src.is_ascii() {
            return Err(sbe_rt::EncodeError::InvalidAscii {
                field: "vehicleCode",
            });
        }
        if src.len() > 6 {
            return Err(sbe_rt::EncodeError::FixedArrayTooLong {
                field: "vehicleCode",
                max_length: 6,
                actual: src.len(),
            });
        }
        let mut tmp = [0 as u8; 6];
        let bytes = src.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            tmp[i] = bytes[i] as u8;
            i += 1;
        }
        Ok(self.vehicle_code(tmp))
    }
    ///Generated method `put_vehicle_code`.
    #[inline]
    pub fn put_vehicle_code(
        &mut self,
        v0: u8,
        v1: u8,
        v2: u8,
        v3: u8,
        v4: u8,
        v5: u8,
    ) -> &mut Self {
        self.vehicle_code([v0, v1, v2, v3, v4, v5])
    }
    ///Generated method `extras`.
    #[inline]
    pub fn extras(&mut self, val: OptionalExtras) -> &mut Self {
        let offset = self.msg_offset + 42;
        self.buf[offset..offset + 1].copy_from_slice(&val.0.to_le_bytes());
        self
    }
    ///Generated method `engine`.
    #[inline]
    pub fn engine(&mut self, val: Engine) -> &mut Self {
        let offset = self.msg_offset + 43;
        self.buf[offset..offset + 10].copy_from_slice(&val.0);
        self
    }
    ///Set all fixed fields at once from a [`CarFixedFields`] value.
    ///
    ///Required fields are always written; optional fields write the schema null wire image when `None` (including nested optional composite members). Returns the encoder ready for ordered tail methods.
    #[inline(always)]
    #[must_use]
    pub fn fixed(
        mut self,
        fixed: &CarFixedFields,
    ) -> CarEncoder<'a, H, sbe_rt::FieldsFixed> {
        self.serial_number(fixed.serial_number);
        self.model_year(fixed.model_year);
        self.available(fixed.available);
        self.code(fixed.code);
        self.some_numbers(fixed.some_numbers);
        self.vehicle_code(fixed.vehicle_code);
        self.extras(fixed.extras);
        self.engine(fixed.engine);
        CarEncoder {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: self.offset,
            _header: core::marker::PhantomData,
            _fields: core::marker::PhantomData,
        }
    }
    ///Return a dedicated raw fixed-field writer. All individual field setters are available on the writer. To advance to tail stages, collect the values into a [`CarFixedFields`] and call `fixed()`.
    #[inline]
    #[must_use]
    pub fn raw_fixed(self) -> CarRawFixedWriter<'a> {
        let body_start = self.msg_offset + 8;
        CarRawFixedWriter {
            buf: &mut self.buf[body_start..],
            msg_offset: 0,
            offset: self.offset - body_start,
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> CarEncoder<'a, H, sbe_rt::FieldsFixed> {
    ///Generated method `serial_number`.
    #[inline]
    pub fn serial_number(&mut self, val: u64) -> &mut Self {
        let offset = self.msg_offset + 8;
        unsafe {
            self.buf
                .get_unchecked_mut(offset..offset + 8)
                .copy_from_slice(&val.to_le_bytes());
        }
        self
    }
    ///Generated method `model_year`.
    #[inline]
    pub fn model_year(&mut self, val: u16) -> &mut Self {
        let offset = self.msg_offset + 16;
        unsafe {
            self.buf
                .get_unchecked_mut(offset..offset + 2)
                .copy_from_slice(&val.to_le_bytes());
        }
        self
    }
    ///Generated method `available`.
    #[inline]
    pub fn available(&mut self, val: BooleanType) -> &mut Self {
        let offset = self.msg_offset + 18;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    ///Generated method `available_bool`.
    #[inline]
    pub fn available_bool(&mut self, val: bool) -> &mut Self {
        self.buf[self.msg_offset + 18] = val as u8;
        self
    }
    ///Generated method `code`.
    #[inline]
    pub fn code(&mut self, val: Model) -> &mut Self {
        let offset = self.msg_offset + 19;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    ///Generated method `some_numbers`.
    #[inline]
    pub fn some_numbers(&mut self, val: [u32; 4]) -> &mut Self {
        let offset = self.msg_offset + 20;
        let mut idx = 0usize;
        while idx < 4 {
            unsafe {
                self.buf
                    .get_unchecked_mut(offset + idx * 4..offset + (idx + 1) * 4)
                    .copy_from_slice(&val[idx].to_le_bytes());
            }
            idx += 1;
        }
        self
    }
    ///Generated method `put_some_numbers`.
    #[inline]
    pub fn put_some_numbers(&mut self, v0: u32, v1: u32, v2: u32, v3: u32) -> &mut Self {
        self.some_numbers([v0, v1, v2, v3])
    }
    ///Generated method `vehicle_code`.
    #[inline]
    pub fn vehicle_code(&mut self, val: [u8; 6]) -> &mut Self {
        let offset = self.msg_offset + 36;
        unsafe {
            let dst = self.buf.get_unchecked_mut(offset..offset + 6);
            let src = core::slice::from_raw_parts(val.as_ptr() as *const u8, 6);
            dst.copy_from_slice(src);
        }
        self
    }
    ///Generated method `vehicle_code_str`.
    #[inline]
    pub fn vehicle_code_str(
        &mut self,
        src: &str,
    ) -> Result<&mut Self, sbe_rt::EncodeError> {
        if !src.is_ascii() {
            return Err(sbe_rt::EncodeError::InvalidAscii {
                field: "vehicleCode",
            });
        }
        if src.len() > 6 {
            return Err(sbe_rt::EncodeError::FixedArrayTooLong {
                field: "vehicleCode",
                max_length: 6,
                actual: src.len(),
            });
        }
        let mut tmp = [0 as u8; 6];
        let bytes = src.as_bytes();
        let mut i = 0usize;
        while i < bytes.len() {
            tmp[i] = bytes[i] as u8;
            i += 1;
        }
        Ok(self.vehicle_code(tmp))
    }
    ///Generated method `put_vehicle_code`.
    #[inline]
    pub fn put_vehicle_code(
        &mut self,
        v0: u8,
        v1: u8,
        v2: u8,
        v3: u8,
        v4: u8,
        v5: u8,
    ) -> &mut Self {
        self.vehicle_code([v0, v1, v2, v3, v4, v5])
    }
    ///Generated method `extras`.
    #[inline]
    pub fn extras(&mut self, val: OptionalExtras) -> &mut Self {
        let offset = self.msg_offset + 42;
        self.buf[offset..offset + 1].copy_from_slice(&val.0.to_le_bytes());
        self
    }
    ///Generated method `engine`.
    #[inline]
    pub fn engine(&mut self, val: Engine) -> &mut Self {
        let offset = self.msg_offset + 43;
        self.buf[offset..offset + 10].copy_from_slice(&val.0);
        self
    }
}
/// Buffer-placement metadata. Holds a reference to the parent encoder
/// — zero-copy. Utility methods live here so no schema field can
/// collide with them.
#[derive(Clone, Copy)]
pub struct CarEncoderMetadata<'m, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    encoder_msg_offset: usize,
    encoder_offset: usize,
    encoder_buf: &'m [u8],
    _h: core::marker::PhantomData<H>,
}
impl<'m, H: sbe_rt::HeaderState> CarEncoderMetadata<'m, H> {
    /// Fixed-block body bytes only (groups/var-data not yet written).
    /// For a complete frame use the terminal stage's
    /// `as_bytes_with_header`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_fixed_body_bytes(&self) -> &[u8] {
        &self.encoder_buf[self.encoder_msg_offset + 8..self.encoder_offset]
    }
    /// Header + fixed block only — **not** a complete SBE message when
    /// groups or var-data remain. Prefer the complete stage's
    /// `as_bytes_with_header`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_fixed_region_with_header(&self) -> &[u8] {
        &self.encoder_buf[self.encoder_msg_offset..self.encoder_offset]
    }
    /// Absolute offset of this message within the original buffer
    /// (the `msg_offset` argument passed to `wrap`).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn message_offset(&self) -> usize {
        self.encoder_msg_offset
    }
    /// Absolute current write cursor within the original buffer.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn limit(&self) -> usize {
        self.encoder_offset
    }
    /// The complete original buffer this encoder wraps.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn buffer(&self) -> &[u8] {
        self.encoder_buf
    }
}
impl<'a, H: sbe_rt::HeaderState, F: sbe_rt::FieldsState> CarEncoder<'a, H, F> {
    ///Generated method `get_metadata`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn get_metadata(&self) -> CarEncoderMetadata<'_, H> {
        CarEncoderMetadata {
            encoder_msg_offset: self.msg_offset,
            encoder_offset: self.offset,
            encoder_buf: self.buf,
            _h: core::marker::PhantomData,
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> CarEncoder<'a, H, sbe_rt::FieldsFixed> {
    /// Encode this group with a known count up front.
    /// Closures return [`sbe_rt::GroupResult`]
    /// (`Result<(), EncodeError>`); `?` works — there is no
    /// separate `try_*` method name.
    #[inline]
    #[must_use]
    pub fn fuel_figures<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<CarAfterFuelFigures<'a, H>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut FuelFiguresEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.offset + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: stringify!(fuel_figures),
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        self.buf[self.offset..self.offset + 4]
            .copy_from_slice(&FuelFiguresEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.offset + 2..self.offset + 2 + 2]
            .copy_from_slice(&count.to_le_bytes());
        let mut group = FuelFiguresEncoder::wrap(self.buf, self.offset + 4, count);
        f(&mut group)?;
        let written = group.written();
        if written != count {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared: sbe_rt::group_diag_count(count as u64)?,
                actual: sbe_rt::group_diag_count(written as u64)?,
            });
        }
        Ok(CarAfterFuelFigures {
            buf: group.buf,
            msg_offset: self.msg_offset,
            offset: group.offset,
            _header: core::marker::PhantomData,
        })
    }
    ///Encode this group without knowing the count up front.
    ///
    ///The dimension header is written with a zero placeholder; after the closure returns, the actual entry count is back-patched into the header. No `GroupFull` check — overflow is the caller's responsibility.
    ///
    ///Prefer [`Self::fuel_figures`] when the count is known at compile time or from a small input.
    #[inline]
    #[must_use]
    pub fn fuel_figures_unknown_size<F>(
        mut self,
        f: F,
    ) -> Result<CarAfterFuelFigures<'a, H>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut FuelFiguresEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.offset + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: stringify!(fuel_figures),
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        self.buf[self.offset..self.offset + 4]
            .copy_from_slice(&FuelFiguresEncoder::GROUP_DIM_TEMPLATE);
        let count_offset = self.offset + 2;
        self.buf[count_offset..count_offset + 2].fill(0);
        let (buf, offset, actual) = {
            let mut group = FuelFiguresEncoder::wrap(
                self.buf,
                self.offset + 4,
                u16::MAX,
            );
            f(&mut group)?;
            let n = group.written();
            (group.buf, group.offset, n)
        };
        buf[count_offset..count_offset + 2].copy_from_slice(&actual.to_le_bytes());
        Ok(CarAfterFuelFigures {
            buf,
            msg_offset: self.msg_offset,
            offset,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarAfterFuelFigures<'a, H> {
    /// Encode this group with a known count up front.
    /// Closures return [`sbe_rt::GroupResult`]
    /// (`Result<(), EncodeError>`); `?` works — there is no
    /// separate `try_*` method name.
    #[inline]
    #[must_use]
    pub fn performance_figures<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<CarAfterPerformanceFigures<'a, H>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.offset + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: stringify!(performance_figures),
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        self.buf[self.offset..self.offset + 4]
            .copy_from_slice(&PerformanceFiguresEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.offset + 2..self.offset + 2 + 2]
            .copy_from_slice(&count.to_le_bytes());
        let mut group = PerformanceFiguresEncoder::wrap(
            self.buf,
            self.offset + 4,
            count,
        );
        f(&mut group)?;
        let written = group.written();
        if written != count {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared: sbe_rt::group_diag_count(count as u64)?,
                actual: sbe_rt::group_diag_count(written as u64)?,
            });
        }
        Ok(CarAfterPerformanceFigures {
            buf: group.buf,
            msg_offset: self.msg_offset,
            offset: group.offset,
            _header: core::marker::PhantomData,
        })
    }
    ///Encode this group without knowing the count up front.
    ///
    ///The dimension header is written with a zero placeholder; after the closure returns, the actual entry count is back-patched into the header. No `GroupFull` check — overflow is the caller's responsibility.
    ///
    ///Prefer [`Self::performance_figures`] when the count is known at compile time or from a small input.
    #[inline]
    #[must_use]
    pub fn performance_figures_unknown_size<F>(
        mut self,
        f: F,
    ) -> Result<CarAfterPerformanceFigures<'a, H>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.offset + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: stringify!(performance_figures),
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        self.buf[self.offset..self.offset + 4]
            .copy_from_slice(&PerformanceFiguresEncoder::GROUP_DIM_TEMPLATE);
        let count_offset = self.offset + 2;
        self.buf[count_offset..count_offset + 2].fill(0);
        let (buf, offset, actual) = {
            let mut group = PerformanceFiguresEncoder::wrap(
                self.buf,
                self.offset + 4,
                u16::MAX,
            );
            f(&mut group)?;
            let n = group.written();
            (group.buf, group.offset, n)
        };
        buf[count_offset..count_offset + 2].copy_from_slice(&actual.to_le_bytes());
        Ok(CarAfterPerformanceFigures {
            buf,
            msg_offset: self.msg_offset,
            offset,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarAfterPerformanceFigures<'a, H> {
    ///Generated method `manufacturer`.
    #[inline]
    #[must_use]
    pub fn manufacturer(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterManufacturer<'a, H>, sbe_rt::EncodeError> {
        if data.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "manufacturer",
                max_length: 1073741824,
                actual: data.len(),
            });
        }
        let needed = 4 + data.len();
        if self.offset + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: stringify!(manufacturer),
                needed,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let wire_length = <u32>::try_from(data.len())
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: stringify!(manufacturer),
                    max_length: <u32>::MAX as usize,
                    actual: data.len(),
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterManufacturer {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: start + data.len(),
            _header: core::marker::PhantomData,
        })
    }
    ///Generated method `manufacturer_unchecked`.
    #[inline]
    #[must_use]
    pub fn manufacturer_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterManufacturer<'a, H>, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.offset + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: stringify!(manufacturer),
                needed,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let wire_length = <u32>::try_from(data.len())
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: stringify!(manufacturer),
                    max_length: <u32>::MAX as usize,
                    actual: data.len(),
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterManufacturer {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: start + data.len(),
            _header: core::marker::PhantomData,
        })
    }
    /// Lend exactly `exact_len` bytes of the var-data region
    /// to a closure for nested-message encoding. Zero-copy:
    /// the closure writes directly into the outer buffer.
    ///
    /// Canonical nested-SBE pattern (AppMessage → L2Book):
    /// ```text
    /// let inner_len = InnerEncoder::compute_length_with_header(...);
    /// after.payload_with(inner_len, |payload| {
    ///     let len = InnerEncoder::wrap_and_apply_header(payload, 0)?
    ///         .field(value)
    ///         // continue the single encoder chain through all tail stages
    ///         .encoded_length_with_header();
    ///     debug_assert_eq!(len, inner_len);
    ///     Ok(())
    /// })?;
    /// ```
    /// Returns the next stage on success; on failure the
    /// caller error propagates unchanged and no partial
    /// data is published.
    #[inline]
    #[must_use]
    pub fn manufacturer_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<CarAfterManufacturer<'a, H>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 1073741824 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "manufacturer",
                    max_length: 1073741824,
                    actual: exact_len,
                }
                    .into(),
            );
        }
        let needed = 4 + exact_len;
        if self.offset + needed > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: stringify!(manufacturer),
                    needed,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        let wire_length = <u32>::try_from(exact_len)
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: stringify!(manufacturer),
                    max_length: <u32>::MAX as usize,
                    actual: exact_len,
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(CarAfterManufacturer {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: start + exact_len,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarAfterManufacturer<'a, H> {
    ///Generated method `model`.
    #[inline]
    #[must_use]
    pub fn model(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterModel<'a, H>, sbe_rt::EncodeError> {
        if data.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "model",
                max_length: 1073741824,
                actual: data.len(),
            });
        }
        let needed = 4 + data.len();
        if self.offset + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: stringify!(model),
                needed,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let wire_length = <u32>::try_from(data.len())
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: stringify!(model),
                    max_length: <u32>::MAX as usize,
                    actual: data.len(),
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterModel {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: start + data.len(),
            _header: core::marker::PhantomData,
        })
    }
    ///Generated method `model_unchecked`.
    #[inline]
    #[must_use]
    pub fn model_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterModel<'a, H>, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.offset + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: stringify!(model),
                needed,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let wire_length = <u32>::try_from(data.len())
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: stringify!(model),
                    max_length: <u32>::MAX as usize,
                    actual: data.len(),
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterModel {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: start + data.len(),
            _header: core::marker::PhantomData,
        })
    }
    /// Lend exactly `exact_len` bytes of the var-data region
    /// to a closure for nested-message encoding. Zero-copy:
    /// the closure writes directly into the outer buffer.
    ///
    /// Canonical nested-SBE pattern (AppMessage → L2Book):
    /// ```text
    /// let inner_len = InnerEncoder::compute_length_with_header(...);
    /// after.payload_with(inner_len, |payload| {
    ///     let len = InnerEncoder::wrap_and_apply_header(payload, 0)?
    ///         .field(value)
    ///         // continue the single encoder chain through all tail stages
    ///         .encoded_length_with_header();
    ///     debug_assert_eq!(len, inner_len);
    ///     Ok(())
    /// })?;
    /// ```
    /// Returns the next stage on success; on failure the
    /// caller error propagates unchanged and no partial
    /// data is published.
    #[inline]
    #[must_use]
    pub fn model_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<CarAfterModel<'a, H>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 1073741824 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "model",
                    max_length: 1073741824,
                    actual: exact_len,
                }
                    .into(),
            );
        }
        let needed = 4 + exact_len;
        if self.offset + needed > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: stringify!(model),
                    needed,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        let wire_length = <u32>::try_from(exact_len)
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: stringify!(model),
                    max_length: <u32>::MAX as usize,
                    actual: exact_len,
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(CarAfterModel {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: start + exact_len,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarAfterModel<'a, H> {
    ///Generated method `activation_code`.
    #[inline]
    #[must_use]
    pub fn activation_code(
        mut self,
        data: &[u8],
    ) -> Result<CarComplete<'a, H>, sbe_rt::EncodeError> {
        if data.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "activationCode",
                max_length: 1073741824,
                actual: data.len(),
            });
        }
        let needed = 4 + data.len();
        if self.offset + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: stringify!(activation_code),
                needed,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let wire_length = <u32>::try_from(data.len())
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: stringify!(activation_code),
                    max_length: <u32>::MAX as usize,
                    actual: data.len(),
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarComplete {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: start + data.len(),
            _header: core::marker::PhantomData,
        })
    }
    ///Generated method `activation_code_unchecked`.
    #[inline]
    #[must_use]
    pub fn activation_code_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarComplete<'a, H>, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.offset + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: stringify!(activation_code),
                needed,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let wire_length = <u32>::try_from(data.len())
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: stringify!(activation_code),
                    max_length: <u32>::MAX as usize,
                    actual: data.len(),
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarComplete {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: start + data.len(),
            _header: core::marker::PhantomData,
        })
    }
    /// Lend exactly `exact_len` bytes of the var-data region
    /// to a closure for nested-message encoding. Zero-copy:
    /// the closure writes directly into the outer buffer.
    ///
    /// Canonical nested-SBE pattern (AppMessage → L2Book):
    /// ```text
    /// let inner_len = InnerEncoder::compute_length_with_header(...);
    /// after.payload_with(inner_len, |payload| {
    ///     let len = InnerEncoder::wrap_and_apply_header(payload, 0)?
    ///         .field(value)
    ///         // continue the single encoder chain through all tail stages
    ///         .encoded_length_with_header();
    ///     debug_assert_eq!(len, inner_len);
    ///     Ok(())
    /// })?;
    /// ```
    /// Returns the next stage on success; on failure the
    /// caller error propagates unchanged and no partial
    /// data is published.
    #[inline]
    #[must_use]
    pub fn activation_code_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<CarComplete<'a, H>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 1073741824 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "activationCode",
                    max_length: 1073741824,
                    actual: exact_len,
                }
                    .into(),
            );
        }
        let needed = 4 + exact_len;
        if self.offset + needed > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: stringify!(activation_code),
                    needed,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        let wire_length = <u32>::try_from(exact_len)
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: stringify!(activation_code),
                    max_length: <u32>::MAX as usize,
                    actual: exact_len,
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(CarComplete {
            buf: self.buf,
            msg_offset: self.msg_offset,
            offset: start + exact_len,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarComplete<'a, H> {
    /// SBE message body bytes (excluding the message header).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_body_bytes(&self) -> &[u8] {
        let body_start = self.msg_offset + 8;
        &self.buf[body_start..self.offset]
    }
    /// SBE message body length (excluding the message header).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.offset - self.msg_offset - 8
    }
    /// Total SBE message length including the header region.
    /// Pure arithmetic — available for body-only wraps too.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.offset - self.msg_offset
    }
    /// Unwritten region after this message's write cursor to the end of
    /// the original buffer. Use for multi-message packing, e.g.
    /// `NextEncoder::wrap_and_apply_header(remaining, 0)`. This is **not**
    /// the payload of the current message — for the absolute write
    /// cursor while keeping the encoder alive, use
    /// `get_metadata().limit()`.
    #[inline]
    pub fn into_remaining_mut(self) -> &'a mut [u8] {
        &mut self.buf[self.offset..]
    }
}
impl<'a> CarComplete<'a, sbe_rt::HeaderPresent> {
    /// Header-inclusive bytes. Only available when the encoder was
    /// constructed via `wrap_and_apply_header` (not raw `wrap`).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn as_bytes_with_header(&self) -> &[u8] {
        &self.buf[self.msg_offset..self.offset]
    }
}
impl<'a, H: sbe_rt::HeaderState, F: sbe_rt::FieldsState> __sbe_message_sealed::Sealed
for CarEncoder<'a, H, F> {}
impl<'a, H: sbe_rt::HeaderState, F: sbe_rt::FieldsState> sbe_rt::SbeMessage
for CarEncoder<'a, H, F> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 45;
    const SCHEMA_ID: u16 = 1;
    const SCHEMA_VERSION: u16 = 0;
}
/// Pre-`fixed()` root encoder stage. Individual fixed-field setters
/// and [`fixed`](Self::fixed) live here; group/var-data tails are
/// only available on the [`sbe_rt::FieldsFixed`] phase after
/// `fixed(&FixedFields)`.
pub type CarUnfixedEncoder<'a, H = sbe_rt::HeaderPresent> = CarEncoder<
    'a,
    H,
    sbe_rt::FieldsUnfixed,
>;
#[doc = concat!(
    "Encoder for the `", stringify!(FuelFiguresEncoder),
    "` group — call `add()` to write entries."
)]
#[must_use = "group encoder must call add() to write entries"]
pub struct FuelFiguresEncoder<'a> {
    buf: &'a mut [u8],
    offset: usize,
    count: u16,
    written: u16,
}
impl<'a> FuelFiguresEncoder<'a> {
    /// Wire size in bytes of one entry's fixed block — every entry
    /// `add()` writes advances `offset` by exactly this much before
    /// any entry-level var-data or nested groups.
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    /// The dimension header this group's entries expect, as raw
    /// wire bytes (block length + count fields, group-header byte
    /// order). `wrap()` does not write this — a caller assembling a
    /// standalone group writes it at `offset - GROUP_DIM_TEMPLATE.len()`
    /// before calling `wrap()`, with the count field set to the
    /// declared entry count. `add()` calls must write exactly that
    /// many entries; the parent message stage rejects a mismatch as
    /// [`sbe_rt::EncodeError::GroupCountMismatch`].
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [6, 0, 0, 0];
    const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == 4);
    /// Low-level entries-only constructor for a standalone group.
    ///
    /// `offset` is the position of the **first entry**, immediately
    /// after a dimension header the caller has already written —
    /// this constructor neither writes nor back-patches that header.
    /// `count` bounds how many entries `add()` accepts before it
    /// returns [`sbe_rt::EncodeError::GroupFull`].
    ///
    /// Normal generated code reaches this only through the parent
    /// message's own group-writing stage (e.g. the `fuel_figures(
    /// count, |g| { .. })` method on the message encoder), which
    /// writes the dimension header for you and hands you this type
    /// already positioned at the first entry. Call this directly
    /// only when assembling a standalone group outside a full
    /// message — get the framing (header contents, `offset`, order)
    /// wrong and the result is a malformed group image.
    #[inline]
    pub fn wrap(buf: &'a mut [u8], offset: usize, count: u16) -> Self {
        Self {
            buf,
            offset,
            count,
            written: 0,
        }
    }
    /// Write one group entry, proving required tails are complete.
    ///
    /// The closure takes the entry encoder **by value** and must return
    /// the entry-complete proof — reachable only by writing every
    /// required nested group and var-data field in wire order.
    #[inline]
    #[must_use]
    pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(
            FuelFiguresEntryEncoder<'b>,
        ) -> Result<FuelFiguresEntryComplete<'b>, sbe_rt::EncodeError>,
    {
        if self.written >= self.count {
            return Err(
                sbe_rt::EncodeError::GroupFull {
                    declared: sbe_rt::group_diag_count(self.count as u64)?,
                    attempted: sbe_rt::group_diag_count(self.written as u64)?
                        .checked_add(1)
                        .ok_or(sbe_rt::EncodeError::GroupCountOverflow {
                            maximum: u32::MAX,
                            actual: u32::MAX,
                        })?,
                }
                    .into(),
            );
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.offset + block_len > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: "group entry",
                    needed: block_len,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let __entry = unsafe { FuelFiguresEntryEncoder::wrap(__buf, self.offset) };
            let __complete = f(__entry)?;
            self.offset = __complete.into_cursor();
        }
        self.written += 1;
        Ok(())
    }
    /// Number of entries written so far (for `_unknown_size` back-patch).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn written(&self) -> u16 {
        self.written
    }
}
#[doc = concat!(
    "Proven-complete entry for the `", stringify!(FuelFiguresEntryComplete), "` group."
)]
pub struct FuelFiguresEntryComplete<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    offset: usize,
}
impl<'a> FuelFiguresEntryComplete<'a> {
    pub(crate) fn into_cursor(self) -> usize {
        self.offset
    }
}
#[doc = concat!(
    "Entry encoder for the `", stringify!(FuelFiguresEntryEncoder), "` group",
    " — write required tails in wire order to reach EntryComplete", "."
)]
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct FuelFiguresEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    offset: usize,
}
impl<'a> FuelFiguresEntryEncoder<'a> {
    ///`ENTRY_BLOCK_LENGTH` = 6.
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    /// Private entry wrap after the group encoder proved the fixed block
    /// region fits (via `add` / `start_entry` capacity checks).
    ///
    /// # Safety
    /// `offset + ENTRY_BLOCK_LENGTH` must not overflow and must be ≤ `buf.len()`
    /// for the lifetime of the returned encoder.
    #[inline]
    unsafe fn wrap(buf: &'a mut [u8], offset: usize) -> Self {
        Self {
            buf,
            entry_start: offset,
            offset: offset + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    ///Generated method `speed`.
    #[inline]
    pub fn speed(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    ///Generated method `mpg`.
    #[inline]
    pub fn mpg(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        self.buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        self
    }
    ///Generated method `usage_description`.
    #[inline]
    #[must_use]
    pub fn usage_description(
        mut self,
        data: &[u8],
    ) -> Result<FuelFiguresEntryComplete<'a>, sbe_rt::EncodeError> {
        if data.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "usageDescription",
                max_length: 1073741824,
                actual: data.len(),
            });
        }
        let needed = 4 + data.len();
        if self.offset + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: "group entry",
                needed,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let wire_length = u32::try_from(data.len())
            .map_err(|_| {
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "usageDescription",
                    max_length: u32::MAX as usize,
                    actual: data.len(),
                }
            })?;
        let len_bytes = wire_length.to_le_bytes();
        self.buf[self.offset..self.offset + 4].copy_from_slice(&len_bytes);
        let start = self.offset + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        self.offset = start + data.len();
        Ok(FuelFiguresEntryComplete {
            buf: self.buf,
            entry_start: self.entry_start,
            offset: self.offset,
        })
    }
}
#[doc = concat!(
    "Encoder for the `", stringify!(PerformanceFiguresEncoder),
    "` group — call `add()` to write entries."
)]
#[must_use = "group encoder must call add() to write entries"]
pub struct PerformanceFiguresEncoder<'a> {
    buf: &'a mut [u8],
    offset: usize,
    count: u16,
    written: u16,
}
impl<'a> PerformanceFiguresEncoder<'a> {
    /// Wire size in bytes of one entry's fixed block — every entry
    /// `add()` writes advances `offset` by exactly this much before
    /// any entry-level var-data or nested groups.
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    /// The dimension header this group's entries expect, as raw
    /// wire bytes (block length + count fields, group-header byte
    /// order). `wrap()` does not write this — a caller assembling a
    /// standalone group writes it at `offset - GROUP_DIM_TEMPLATE.len()`
    /// before calling `wrap()`, with the count field set to the
    /// declared entry count. `add()` calls must write exactly that
    /// many entries; the parent message stage rejects a mismatch as
    /// [`sbe_rt::EncodeError::GroupCountMismatch`].
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [1, 0, 0, 0];
    const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == 4);
    /// Low-level entries-only constructor for a standalone group.
    ///
    /// `offset` is the position of the **first entry**, immediately
    /// after a dimension header the caller has already written —
    /// this constructor neither writes nor back-patches that header.
    /// `count` bounds how many entries `add()` accepts before it
    /// returns [`sbe_rt::EncodeError::GroupFull`].
    ///
    /// Normal generated code reaches this only through the parent
    /// message's own group-writing stage (e.g. the `fuel_figures(
    /// count, |g| { .. })` method on the message encoder), which
    /// writes the dimension header for you and hands you this type
    /// already positioned at the first entry. Call this directly
    /// only when assembling a standalone group outside a full
    /// message — get the framing (header contents, `offset`, order)
    /// wrong and the result is a malformed group image.
    #[inline]
    pub fn wrap(buf: &'a mut [u8], offset: usize, count: u16) -> Self {
        Self {
            buf,
            offset,
            count,
            written: 0,
        }
    }
    /// Write one group entry, proving required tails are complete.
    ///
    /// The closure takes the entry encoder **by value** and must return
    /// the entry-complete proof — reachable only by writing every
    /// required nested group and var-data field in wire order.
    #[inline]
    #[must_use]
    pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(
            PerformanceFiguresEntryEncoder<'b>,
        ) -> Result<PerformanceFiguresEntryComplete<'b>, sbe_rt::EncodeError>,
    {
        if self.written >= self.count {
            return Err(
                sbe_rt::EncodeError::GroupFull {
                    declared: sbe_rt::group_diag_count(self.count as u64)?,
                    attempted: sbe_rt::group_diag_count(self.written as u64)?
                        .checked_add(1)
                        .ok_or(sbe_rt::EncodeError::GroupCountOverflow {
                            maximum: u32::MAX,
                            actual: u32::MAX,
                        })?,
                }
                    .into(),
            );
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.offset + block_len > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: "group entry",
                    needed: block_len,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let __entry = unsafe {
                PerformanceFiguresEntryEncoder::wrap(__buf, self.offset)
            };
            let __complete = f(__entry)?;
            self.offset = __complete.into_cursor();
        }
        self.written += 1;
        Ok(())
    }
    /// Number of entries written so far (for `_unknown_size` back-patch).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn written(&self) -> u16 {
        self.written
    }
}
#[doc = concat!(
    "Proven-complete entry for the `", stringify!(PerformanceFiguresEntryComplete),
    "` group."
)]
pub struct PerformanceFiguresEntryComplete<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    offset: usize,
}
impl<'a> PerformanceFiguresEntryComplete<'a> {
    pub(crate) fn into_cursor(self) -> usize {
        self.offset
    }
}
#[doc = concat!(
    "Entry encoder for the `", stringify!(PerformanceFiguresEntryEncoder), "` group",
    " — write required tails in wire order to reach EntryComplete", "."
)]
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct PerformanceFiguresEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    offset: usize,
}
impl<'a> PerformanceFiguresEntryEncoder<'a> {
    ///`ENTRY_BLOCK_LENGTH` = 1.
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    /// Private entry wrap after the group encoder proved the fixed block
    /// region fits (via `add` / `start_entry` capacity checks).
    ///
    /// # Safety
    /// `offset + ENTRY_BLOCK_LENGTH` must not overflow and must be ≤ `buf.len()`
    /// for the lifetime of the returned encoder.
    #[inline]
    unsafe fn wrap(buf: &'a mut [u8], offset: usize) -> Self {
        Self {
            buf,
            entry_start: offset,
            offset: offset + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    ///Generated method `octane_rating`.
    #[inline]
    pub fn octane_rating(&mut self, val: u8) -> &mut Self {
        self.buf[self.entry_start + 0] = val as u8;
        self
    }
    ///Generated method `acceleration`.
    #[inline]
    #[must_use]
    pub fn acceleration<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<PerformanceFiguresEntryComplete<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresAccelerationEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.offset + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: "group entry",
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        self.buf[self.offset..self.offset + 4]
            .copy_from_slice(&PerformanceFiguresAccelerationEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.offset + 2..self.offset + 2 + 2]
            .copy_from_slice(&count.to_le_bytes());
        let __offset;
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut group = PerformanceFiguresAccelerationEncoder::wrap(
                __buf,
                self.offset + 4,
                count,
            );
            f(&mut group)?;
            let written = group.written();
            if written != count {
                return Err(sbe_rt::EncodeError::GroupCountMismatch {
                    declared: sbe_rt::group_diag_count(count as u64)?,
                    actual: sbe_rt::group_diag_count(written as u64)?,
                });
            }
            __offset = group.offset;
        }
        Ok(PerformanceFiguresEntryComplete {
            buf: self.buf,
            entry_start: self.entry_start,
            offset: __offset,
        })
    }
    /// Nested-group `_unknown_size` variant — back-patches count.
    #[inline]
    pub fn acceleration_unknown_size<F>(
        mut self,
        f: F,
    ) -> Result<PerformanceFiguresEntryComplete<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresAccelerationEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.offset + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: "group entry",
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        self.buf[self.offset..self.offset + 4]
            .copy_from_slice(&PerformanceFiguresAccelerationEncoder::GROUP_DIM_TEMPLATE);
        let count_offset = self.offset + 2;
        self.buf[count_offset..count_offset + 2].fill(0);
        let __offset;
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut group = PerformanceFiguresAccelerationEncoder::wrap(
                __buf,
                self.offset + 4,
                u16::MAX,
            );
            f(&mut group)?;
            let actual: u16 = group.written();
            __offset = group.offset;
            group
                .buf[count_offset..count_offset + 2]
                .copy_from_slice(&actual.to_le_bytes());
        }
        Ok(PerformanceFiguresEntryComplete {
            buf: self.buf,
            entry_start: self.entry_start,
            offset: __offset,
        })
    }
}
#[doc = concat!(
    "Encoder for the `", stringify!(PerformanceFiguresAccelerationEncoder),
    "` group — call `add()` to write entries."
)]
#[must_use = "group encoder must call add() to write entries"]
pub struct PerformanceFiguresAccelerationEncoder<'a> {
    buf: &'a mut [u8],
    offset: usize,
    count: u16,
    written: u16,
}
impl<'a> PerformanceFiguresAccelerationEncoder<'a> {
    /// Wire size in bytes of one entry's fixed block — every entry
    /// `add()` writes advances `offset` by exactly this much before
    /// any entry-level var-data or nested groups.
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    /// The dimension header this group's entries expect, as raw
    /// wire bytes (block length + count fields, group-header byte
    /// order). `wrap()` does not write this — a caller assembling a
    /// standalone group writes it at `offset - GROUP_DIM_TEMPLATE.len()`
    /// before calling `wrap()`, with the count field set to the
    /// declared entry count. `add()` calls must write exactly that
    /// many entries; the parent message stage rejects a mismatch as
    /// [`sbe_rt::EncodeError::GroupCountMismatch`].
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [6, 0, 0, 0];
    const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == 4);
    /// Low-level entries-only constructor for a standalone group.
    ///
    /// `offset` is the position of the **first entry**, immediately
    /// after a dimension header the caller has already written —
    /// this constructor neither writes nor back-patches that header.
    /// `count` bounds how many entries `add()` accepts before it
    /// returns [`sbe_rt::EncodeError::GroupFull`].
    ///
    /// Normal generated code reaches this only through the parent
    /// message's own group-writing stage (e.g. the `fuel_figures(
    /// count, |g| { .. })` method on the message encoder), which
    /// writes the dimension header for you and hands you this type
    /// already positioned at the first entry. Call this directly
    /// only when assembling a standalone group outside a full
    /// message — get the framing (header contents, `offset`, order)
    /// wrong and the result is a malformed group image.
    #[inline]
    pub fn wrap(buf: &'a mut [u8], offset: usize, count: u16) -> Self {
        Self {
            buf,
            offset,
            count,
            written: 0,
        }
    }
    /// Write one group entry. The closure may return `()` or
    /// `Result<(), sbe_rt::EncodeError>` (both satisfy
    /// [`sbe_rt::GroupResult`]), so `?` works without a `try_add`.
    #[inline]
    #[must_use]
    pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut PerformanceFiguresAccelerationEntryEncoder<'b>,
        ) -> sbe_rt::GroupResult,
    {
        if self.written >= self.count {
            return Err(
                sbe_rt::EncodeError::GroupFull {
                    declared: sbe_rt::group_diag_count(self.count as u64)?,
                    attempted: sbe_rt::group_diag_count(self.written as u64)?
                        .checked_add(1)
                        .ok_or(sbe_rt::EncodeError::GroupCountOverflow {
                            maximum: u32::MAX,
                            actual: u32::MAX,
                        })?,
                }
                    .into(),
            );
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.offset + block_len > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: "group entry",
                    needed: block_len,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut __entry = unsafe {
                PerformanceFiguresAccelerationEntryEncoder::wrap(__buf, self.offset)
            };
            f(&mut __entry)?;
            self.offset = __entry.offset;
        }
        self.written += 1;
        Ok(())
    }
    ///Write one group entry, proving completeness in the type system.
    ///
    ///The closure takes the entry encoder **by value** and must return `PerformanceFiguresAccelerationEntryComplete` — reachable only by writing every required tail in wire order. An entry that skips, reorders, or repeats a tail cannot produce that type, so it fails to compile rather than producing a short entry at run time.
    ///
    ///[`Self::add`] stays available for entries whose tails are already checked elsewhere.
    #[inline]
    pub fn add_checked<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(
            PerformanceFiguresAccelerationEntryEncoder<'b>,
        ) -> Result<
                PerformanceFiguresAccelerationEntryComplete<'b>,
                sbe_rt::EncodeError,
            >,
    {
        if self.written >= self.count {
            return Err(
                sbe_rt::EncodeError::GroupFull {
                    declared: sbe_rt::group_diag_count(self.count as u64)?,
                    attempted: sbe_rt::group_diag_count(self.written as u64)?
                        .checked_add(1)
                        .ok_or(sbe_rt::EncodeError::GroupCountOverflow {
                            maximum: u32::MAX,
                            actual: u32::MAX,
                        })?,
                }
                    .into(),
            );
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.offset + block_len > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    field: "group entry",
                    needed: block_len,
                    available: self.buf.len().saturating_sub(self.offset),
                }
                    .into(),
            );
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let __entry = unsafe {
                PerformanceFiguresAccelerationEntryEncoder::wrap(__buf, self.offset)
            };
            let __complete = f(__entry)?;
            self.offset = __complete.into_cursor();
        }
        self.written += 1;
        Ok(())
    }
    /// Manual entry creation: returns a borrowed entry encoder.
    /// The entry writes fixed fields directly into the group buffer.
    /// Drop the entry or let it go out of scope to commit it.
    #[must_use]
    #[inline]
    pub fn start_entry(
        &mut self,
    ) -> Result<PerformanceFiguresAccelerationEntryEncoder<'_>, sbe_rt::EncodeError> {
        if self.written as u64 >= self.count as u64 {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: sbe_rt::group_diag_count(self.count as u64)?,
                attempted: sbe_rt::group_diag_count(self.written as u64)?
                    .checked_add(1)
                    .ok_or(sbe_rt::EncodeError::GroupCountOverflow {
                        maximum: u32::MAX,
                        actual: u32::MAX,
                    })?,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self
            .offset
            .checked_add(block_len)
            .map(|end| end > self.buf.len())
            .unwrap_or(true)
        {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: "group entry",
                needed: block_len,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let entry_offset = self.offset;
        self.offset += block_len;
        self.written += 1;
        let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
        Ok(unsafe {
            PerformanceFiguresAccelerationEntryEncoder::wrap(__buf, entry_offset)
        })
    }
    /// Number of entries written so far (for `_unknown_size` back-patch).
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub fn written(&self) -> u16 {
        self.written
    }
}
/// Value struct for this group's entries.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceFiguresAccelerationEntry {
    ///Generated field `mph`.
    pub mph: u16,
    ///Generated field `seconds`.
    pub seconds: f32,
}
impl<'a> PerformanceFiguresAccelerationEncoder<'a> {
    /// Write one entry from a struct. Faster than [`Self::add`] when
    /// the entry has no nested groups or var-data.
    #[inline]
    pub fn add_struct(
        &mut self,
        entry: &PerformanceFiguresAccelerationEntry,
    ) -> Result<(), sbe_rt::EncodeError> {
        if self.written as u64 >= self.count as u64 {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: sbe_rt::group_diag_count(self.count as u64)?,
                attempted: sbe_rt::group_diag_count(self.written as u64)?
                    .checked_add(1)
                    .ok_or(sbe_rt::EncodeError::GroupCountOverflow {
                        maximum: u32::MAX,
                        actual: u32::MAX,
                    })?,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.offset + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: "group entry",
                needed: block_len,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        let offset = self.offset;
        self.offset += block_len;
        self.written += 1;
        self.buf[offset + 0..offset + 0 + 2].copy_from_slice(&entry.mph.to_le_bytes());
        self.buf[offset + 2..offset + 2 + 4]
            .copy_from_slice(&entry.seconds.to_le_bytes());
        Ok(())
    }
    #[inline]
    fn bulk_add_with<T, F>(
        &mut self,
        entries: &[T],
        mut write_entry: F,
    ) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnMut(&T, &mut [u8]) -> Result<(), sbe_rt::EncodeError>,
    {
        let count = entries.len();
        if count == 0 {
            return Ok(());
        }
        let attempted = (self.written as usize)
            .checked_add(count)
            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        if attempted > sbe_rt::count_to_usize(self.count as u64)? {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: sbe_rt::group_diag_count(self.count as u64)?,
                attempted: sbe_rt::group_diag_count(attempted as u64)?,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if block_len == 0 {
            self.written = attempted as u16;
            return Ok(());
        }
        let needed = count
            .checked_mul(block_len)
            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        let end = self
            .offset
            .checked_add(needed)
            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        if end > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                field: "group entry",
                needed,
                available: self.buf.len().saturating_sub(self.offset),
            });
        }
        {
            let region = &mut self.buf[self.offset..end];
            for (entry, slot) in entries.iter().zip(region.chunks_exact_mut(block_len)) {
                write_entry(entry, slot)?;
            }
        }
        self.offset = end;
        self.written = attempted as u16;
        Ok(())
    }
    /// Encode a slice of fixed-size entries after validating the
    /// complete destination region once.
    #[inline]
    pub fn bulk_add(
        &mut self,
        entries: &[PerformanceFiguresAccelerationEntry],
    ) -> Result<(), sbe_rt::EncodeError> {
        self.bulk_add_with(
            entries,
            |entry, slot| {
                slot[0..0 + 2].copy_from_slice(&entry.mph.to_le_bytes());
                slot[2..2 + 4].copy_from_slice(&entry.seconds.to_le_bytes());
                Ok(())
            },
        )
    }
}
#[doc = concat!(
    "Proven-complete entry for the `",
    stringify!(PerformanceFiguresAccelerationEntryComplete), "` group."
)]
pub struct PerformanceFiguresAccelerationEntryComplete<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    offset: usize,
}
impl<'a> PerformanceFiguresAccelerationEntryComplete<'a> {
    pub(crate) fn into_cursor(self) -> usize {
        self.offset
    }
}
#[doc = concat!(
    "Entry encoder for the `", stringify!(PerformanceFiguresAccelerationEntryEncoder),
    "` group", " — set fields then call `complete()`", "."
)]
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct PerformanceFiguresAccelerationEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    offset: usize,
}
impl<'a> PerformanceFiguresAccelerationEntryEncoder<'a> {
    ///`ENTRY_BLOCK_LENGTH` = 6.
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    /// Private entry wrap after the group encoder proved the fixed block
    /// region fits (via `add` / `start_entry` capacity checks).
    ///
    /// # Safety
    /// `offset + ENTRY_BLOCK_LENGTH` must not overflow and must be ≤ `buf.len()`
    /// for the lifetime of the returned encoder.
    #[inline]
    unsafe fn wrap(buf: &'a mut [u8], offset: usize) -> Self {
        Self {
            buf,
            entry_start: offset,
            offset: offset + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    ///Finish a flat entry, producing the `PerformanceFiguresAccelerationEntryComplete` that [`PerformanceFiguresAccelerationEncoder::add_checked`] requires.
    ///
    ///Only for entries with no required tails — an entry that has them reaches this type through its last tail method instead.
    #[inline]
    pub fn complete(self) -> PerformanceFiguresAccelerationEntryComplete<'a> {
        PerformanceFiguresAccelerationEntryComplete {
            buf: self.buf,
            entry_start: self.entry_start,
            offset: self.offset,
        }
    }
    ///Generated method `mph`.
    #[inline]
    pub fn mph(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    ///Generated method `seconds`.
    #[inline]
    pub fn seconds(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        self.buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        self
    }
}
/// Exact-length calculator for this message.
#[must_use = "length builder must be consumed"]
pub struct CarEncodedLength {
    state: EncodedLengthAccumulator,
}
impl CarEncodedLength {
    ///`BLOCK_LENGTH` = 45.
    pub const BLOCK_LENGTH: usize = 45;
    ///`HEADER_LENGTH` = 8.
    pub const HEADER_LENGTH: usize = 8;
    /// Start computing the encoded length.
    #[inline]
    pub const fn new() -> Self {
        Self {
            state: EncodedLengthAccumulator::new(Self::BLOCK_LENGTH),
        }
    }
}
impl CarEncodedLength {
    ///`FUELFIGURES_USAGEDESCRIPTION_PREFIX` = 4.
    pub const FUELFIGURES_USAGEDESCRIPTION_PREFIX: usize = 4;
    ///`PERFORMANCEFIGURES_ACCELERATION_GROUP_DIM` = 4.
    pub const PERFORMANCEFIGURES_ACCELERATION_GROUP_DIM: usize = 4;
    ///`PERFORMANCEFIGURES_ACCELERATION_ENTRY_BLOCK` = 6.
    pub const PERFORMANCEFIGURES_ACCELERATION_ENTRY_BLOCK: usize = 6;
    ///`MANUFACTURER_PREFIX` = 4.
    pub const MANUFACTURER_PREFIX: usize = 4;
    ///`MODEL_PREFIX` = 4.
    pub const MODEL_PREFIX: usize = 4;
    ///`ACTIVATIONCODE_PREFIX` = 4.
    pub const ACTIVATIONCODE_PREFIX: usize = 4;
}
/// Schema-specific ragged entry builder — field-named methods bake in
/// the wire layout (dim/block/prefix). Chain: `b.add()?.field(len)?`.
#[must_use = "ragged builder must be consumed to advance the length"]
pub struct CarFuelFiguresRaggedBuilder<'a> {
    b: &'a mut RaggedEntryBuilder,
}
impl<'a> CarFuelFiguresRaggedBuilder<'a> {
    /// Register one entry. Returns `&mut Self` for chaining.
    #[inline]
    pub fn add(&mut self) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.add()?;
        Ok(self)
    }
    /// Register `count` identical entries at once (uniform shape — no
    /// per-entry var-data or nested-group differences). Shortcut for
    /// calling `add()` in a loop.
    #[inline]
    pub fn uniform(&mut self, count: usize) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.entries(count)?;
        Ok(self)
    }
    /// Record a var-data field's length for the current entry.
    /// The prefix size is baked in — just pass the data length.
    #[inline]
    pub fn usage_description(
        &mut self,
        len: usize,
    ) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.var_data(4, len)?;
        Ok(self)
    }
}
/// Schema-specific ragged entry builder — field-named methods bake in
/// the wire layout (dim/block/prefix). Chain: `b.add()?.field(len)?`.
#[must_use = "ragged builder must be consumed to advance the length"]
pub struct CarPerformanceFiguresRaggedBuilder<'a> {
    b: &'a mut RaggedEntryBuilder,
}
/// Schema-specific ragged entry builder — field-named methods bake in
/// the wire layout (dim/block/prefix). Chain: `b.add()?.field(len)?`.
#[must_use = "ragged builder must be consumed to advance the length"]
pub struct CarPerformanceFiguresAccelerationRaggedBuilder<'a> {
    b: &'a mut RaggedEntryBuilder,
}
impl<'a> CarPerformanceFiguresAccelerationRaggedBuilder<'a> {
    /// Register one entry. Returns `&mut Self` for chaining.
    #[inline]
    pub fn add(&mut self) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.add()?;
        Ok(self)
    }
    /// Register `count` identical entries at once (uniform shape — no
    /// per-entry var-data or nested-group differences). Shortcut for
    /// calling `add()` in a loop.
    #[inline]
    pub fn uniform(&mut self, count: usize) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.entries(count)?;
        Ok(self)
    }
}
impl<'a> CarPerformanceFiguresRaggedBuilder<'a> {
    /// Register one entry. Returns `&mut Self` for chaining.
    #[inline]
    pub fn add(&mut self) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.add()?;
        Ok(self)
    }
    /// Register `count` identical entries at once (uniform shape — no
    /// per-entry var-data or nested-group differences). Shortcut for
    /// calling `add()` in a loop.
    #[inline]
    pub fn uniform(&mut self, count: usize) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.entries(count)?;
        Ok(self)
    }
    /// Enter a nested ragged group. The closure receives a sub-builder
    /// with field-named methods for the nested entries.
    #[inline]
    pub fn acceleration<F>(&mut self, f: F) -> Result<&mut Self, sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut CarPerformanceFiguresAccelerationRaggedBuilder<'_>,
        ) -> Result<(), sbe_rt::EncodeError>,
    {
        self.b
            .group_ragged(
                4,
                6,
                |inner| {
                    let mut sub = CarPerformanceFiguresAccelerationRaggedBuilder {
                        b: inner,
                    };
                    f(&mut sub)
                },
            )?;
        Ok(self)
    }
}
#[doc(hidden)]
#[must_use = "length builder must be completed"]
pub struct CarEncodedLengthAfterFuelFigures {
    state: EncodedLengthAccumulator,
}
#[doc(hidden)]
#[must_use = "length builder must be completed"]
pub struct CarEncodedLengthAfterPerformanceFigures {
    state: EncodedLengthAccumulator,
}
#[doc(hidden)]
#[must_use = "length builder must be completed"]
pub struct CarEncodedLengthAfterManufacturer {
    state: EncodedLengthAccumulator,
}
#[doc(hidden)]
#[must_use = "length builder must be completed"]
pub struct CarEncodedLengthAfterModel {
    state: EncodedLengthAccumulator,
}
#[doc(hidden)]
#[must_use = "length builder must be completed"]
pub struct CarEncodedLengthComplete {
    state: EncodedLengthAccumulator,
}
#[doc(hidden)]
#[must_use = "complete the nested shape or call finish_empty()"]
pub struct CarFuelFiguresUniformEncodedLength {
    state: EncodedLengthAccumulator,
    parent_multiplier: usize,
    declared_count: u32,
}
impl CarFuelFiguresUniformEncodedLength {
    ///Generated method `usage_description`.
    #[inline]
    pub const fn usage_description(
        mut self,
        byte_len: usize,
    ) -> Result<CarEncodedLengthAfterFuelFigures, sbe_rt::EncodeError> {
        if byte_len > 1073741824 {
            self.state
                .fail(sbe_rt::EncodeError::VarDataTooLong {
                    field: "usageDescription",
                    max_length: 1073741824,
                    actual: byte_len,
                });
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "usageDescription",
                max_length: 1073741824,
                actual: byte_len,
            });
        }
        let m = self.state.multiplier();
        self.state.add_scaled(4 as usize, m);
        self.state.add_scaled(byte_len, m);
        self.state.leave_group(self.parent_multiplier);
        match self.state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterFuelFigures {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
    /// Complete this group when the entry count is zero.
    /// Returns an error if the declared count is non-zero.
    #[inline]
    pub fn finish_empty(
        self,
    ) -> Result<CarEncodedLengthAfterFuelFigures, sbe_rt::EncodeError> {
        if self.declared_count != 0 {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared: self.declared_count,
                actual: 0,
            });
        }
        let mut state = self.state;
        state.leave_group(self.parent_multiplier);
        match state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterFuelFigures {
                    state,
                })
            }
            Err(e) => Err(e),
        }
    }
}
impl CarEncodedLength {
    /// **Uniform** group — every one of the `count` entries shares
    /// exactly the same wire shape (same fixed block AND the same
    /// nested-group counts / var-data lengths). The length is the
    /// single entry shape multiplied by `count`, so no per-entry
    /// description is needed. This is the fastest path; prefer it
    /// whenever all entries are identical.
    #[inline]
    pub const fn fuel_figures(self, count: u16) -> CarFuelFiguresUniformEncodedLength {
        let mut state = self.state;
        let count_usize = match sbe_rt::count_to_usize(count as u64) {
            Ok(c) => c,
            Err(e) => {
                state.fail(e);
                0
            }
        };
        let declared_count = match sbe_rt::group_diag_count(count as u64) {
            Ok(c) => c,
            Err(e) => {
                state.fail(e);
                0
            }
        };
        let pm = state.enter_group(count_usize, 4 as usize, 6 as usize);
        CarFuelFiguresUniformEncodedLength {
            state,
            parent_multiplier: pm,
            declared_count,
        }
    }
    /// **Ragged** group (known count) — entries may have *different*
    /// shapes: e.g. each bid has a different number of orders, or
    /// each entry carries var-data of a different length. The total
    /// entry count is known up-front (`count`); the closure describes
    /// each entry's *variable* contribution (nested groups via
    /// `builder.group(dim, block, count)` and var-data via
    /// `builder.var_data(prefix, len)`), calling `builder.add()` once
    /// per entry. The builder verifies `add()` was called exactly
    /// `count` times. Each entry's fixed block is pre-counted, so
    /// `add()` only registers the entry — describe its variable tail
    /// with `group()`/`var_data()`.
    #[inline]
    pub fn fuel_figures_ragged<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<CarEncodedLengthAfterFuelFigures, sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut CarFuelFiguresRaggedBuilder<'_>,
        ) -> Result<(), sbe_rt::EncodeError>,
    {
        let count_usize = sbe_rt::count_to_usize(count as u64)?;
        let declared = sbe_rt::group_diag_count(count as u64)?;
        let pm = self.state.enter_group(count_usize, 4 as usize, 6 as usize);
        self.state.leave_group(pm);
        let mut builder = RaggedEntryBuilder::new(self.state, pm, 0);
        let mut wrapper = CarFuelFiguresRaggedBuilder {
            b: &mut builder,
        };
        f(&mut wrapper)?;
        let actual = sbe_rt::group_diag_count(builder.written as u64)?;
        if actual != declared {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared,
                actual,
            });
        }
        self.state = builder.state;
        self.state.leave_group(pm);
        match self.state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterFuelFigures {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
    ///**Unknown-size** group — the entry count is discovered from the data (e.g. draining an iterator), not known up front.
    ///
    ///Like the ragged path but without a declared `count`: call `builder.add()` (or `builder.entries(n)`) once per entry; the builder counts completed entries and rejects overflow of the wire count type (`u16`). Each `add()` contributes the entry's fixed block, plus any `group()`/`var_data()` recorded for that entry.
    #[inline]
    pub fn fuel_figures_unknown_size<F>(
        mut self,
        f: F,
    ) -> Result<CarEncodedLengthAfterFuelFigures, sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut CarFuelFiguresRaggedBuilder<'_>,
        ) -> Result<(), sbe_rt::EncodeError>,
    {
        let max_count = sbe_rt::count_to_usize(u16::MAX as u64)?;
        let pm = self.state.multiplier();
        self.state.add_scaled(4 as usize, pm);
        let mut builder = RaggedEntryBuilder::new(self.state, pm, 6 as usize);
        let mut wrapper = CarFuelFiguresRaggedBuilder {
            b: &mut builder,
        };
        f(&mut wrapper)?;
        if builder.written > max_count {
            return Err(sbe_rt::EncodeError::GroupCountOverflow {
                maximum: sbe_rt::group_diag_count(u16::MAX as u64)?,
                actual: sbe_rt::group_diag_count(builder.written as u64)?,
            });
        }
        self.state = builder.state;
        match self.state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterFuelFigures {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
}
#[doc(hidden)]
#[must_use = "complete the nested shape or call finish_empty()"]
pub struct CarPerformanceFiguresUniformEncodedLength {
    state: EncodedLengthAccumulator,
    parent_multiplier: usize,
    declared_count: u32,
}
impl CarPerformanceFiguresUniformEncodedLength {
    ///Generated method `acceleration`.
    #[inline]
    pub const fn acceleration(
        mut self,
        count: u16,
    ) -> Result<CarEncodedLengthAfterPerformanceFigures, sbe_rt::EncodeError> {
        let count = match sbe_rt::count_to_usize(count as u64) {
            Ok(c) => c,
            Err(e) => return Err(e),
        };
        let pm = self.state.enter_group(count, 4 as usize, 6 as usize);
        self.state.leave_group(pm);
        match self.state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterPerformanceFigures {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
    /// Complete this group when the entry count is zero.
    /// Returns an error if the declared count is non-zero.
    #[inline]
    pub fn finish_empty(
        self,
    ) -> Result<CarEncodedLengthAfterPerformanceFigures, sbe_rt::EncodeError> {
        if self.declared_count != 0 {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared: self.declared_count,
                actual: 0,
            });
        }
        let mut state = self.state;
        state.leave_group(self.parent_multiplier);
        match state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterPerformanceFigures {
                    state,
                })
            }
            Err(e) => Err(e),
        }
    }
}
impl CarEncodedLengthAfterFuelFigures {
    /// **Uniform** group — every one of the `count` entries shares
    /// exactly the same wire shape (same fixed block AND the same
    /// nested-group counts / var-data lengths). The length is the
    /// single entry shape multiplied by `count`, so no per-entry
    /// description is needed. This is the fastest path; prefer it
    /// whenever all entries are identical.
    #[inline]
    pub const fn performance_figures(
        self,
        count: u16,
    ) -> CarPerformanceFiguresUniformEncodedLength {
        let mut state = self.state;
        let count_usize = match sbe_rt::count_to_usize(count as u64) {
            Ok(c) => c,
            Err(e) => {
                state.fail(e);
                0
            }
        };
        let declared_count = match sbe_rt::group_diag_count(count as u64) {
            Ok(c) => c,
            Err(e) => {
                state.fail(e);
                0
            }
        };
        let pm = state.enter_group(count_usize, 4 as usize, 1 as usize);
        CarPerformanceFiguresUniformEncodedLength {
            state,
            parent_multiplier: pm,
            declared_count,
        }
    }
    /// **Ragged** group (known count) — entries may have *different*
    /// shapes: e.g. each bid has a different number of orders, or
    /// each entry carries var-data of a different length. The total
    /// entry count is known up-front (`count`); the closure describes
    /// each entry's *variable* contribution (nested groups via
    /// `builder.group(dim, block, count)` and var-data via
    /// `builder.var_data(prefix, len)`), calling `builder.add()` once
    /// per entry. The builder verifies `add()` was called exactly
    /// `count` times. Each entry's fixed block is pre-counted, so
    /// `add()` only registers the entry — describe its variable tail
    /// with `group()`/`var_data()`.
    #[inline]
    pub fn performance_figures_ragged<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<CarEncodedLengthAfterPerformanceFigures, sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut CarPerformanceFiguresRaggedBuilder<'_>,
        ) -> Result<(), sbe_rt::EncodeError>,
    {
        let count_usize = sbe_rt::count_to_usize(count as u64)?;
        let declared = sbe_rt::group_diag_count(count as u64)?;
        let pm = self.state.enter_group(count_usize, 4 as usize, 1 as usize);
        self.state.leave_group(pm);
        let mut builder = RaggedEntryBuilder::new(self.state, pm, 0);
        let mut wrapper = CarPerformanceFiguresRaggedBuilder {
            b: &mut builder,
        };
        f(&mut wrapper)?;
        let actual = sbe_rt::group_diag_count(builder.written as u64)?;
        if actual != declared {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared,
                actual,
            });
        }
        self.state = builder.state;
        self.state.leave_group(pm);
        match self.state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterPerformanceFigures {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
    ///**Unknown-size** group — the entry count is discovered from the data (e.g. draining an iterator), not known up front.
    ///
    ///Like the ragged path but without a declared `count`: call `builder.add()` (or `builder.entries(n)`) once per entry; the builder counts completed entries and rejects overflow of the wire count type (`u16`). Each `add()` contributes the entry's fixed block, plus any `group()`/`var_data()` recorded for that entry.
    #[inline]
    pub fn performance_figures_unknown_size<F>(
        mut self,
        f: F,
    ) -> Result<CarEncodedLengthAfterPerformanceFigures, sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut CarPerformanceFiguresRaggedBuilder<'_>,
        ) -> Result<(), sbe_rt::EncodeError>,
    {
        let max_count = sbe_rt::count_to_usize(u16::MAX as u64)?;
        let pm = self.state.multiplier();
        self.state.add_scaled(4 as usize, pm);
        let mut builder = RaggedEntryBuilder::new(self.state, pm, 1 as usize);
        let mut wrapper = CarPerformanceFiguresRaggedBuilder {
            b: &mut builder,
        };
        f(&mut wrapper)?;
        if builder.written > max_count {
            return Err(sbe_rt::EncodeError::GroupCountOverflow {
                maximum: sbe_rt::group_diag_count(u16::MAX as u64)?,
                actual: sbe_rt::group_diag_count(builder.written as u64)?,
            });
        }
        self.state = builder.state;
        match self.state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterPerformanceFigures {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
}
impl CarEncodedLengthAfterPerformanceFigures {
    ///Generated method `manufacturer`.
    #[inline]
    pub const fn manufacturer(
        self,
        byte_len: usize,
    ) -> Result<CarEncodedLengthAfterManufacturer, sbe_rt::EncodeError> {
        if byte_len > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "manufacturer",
                max_length: 1073741824,
                actual: byte_len,
            });
        }
        let len = match self.state.len.checked_add(4 as usize) {
            Some(v) => v,
            None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
        };
        let len = match len.checked_add(byte_len) {
            Some(v) => v,
            None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
        };
        Ok(CarEncodedLengthAfterManufacturer {
            state: EncodedLengthAccumulator {
                len,
                multiplier: 1,
                error: None,
            },
        })
    }
}
impl CarEncodedLengthAfterManufacturer {
    ///Generated method `model`.
    #[inline]
    pub const fn model(
        self,
        byte_len: usize,
    ) -> Result<CarEncodedLengthAfterModel, sbe_rt::EncodeError> {
        if byte_len > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "model",
                max_length: 1073741824,
                actual: byte_len,
            });
        }
        let len = match self.state.len.checked_add(4 as usize) {
            Some(v) => v,
            None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
        };
        let len = match len.checked_add(byte_len) {
            Some(v) => v,
            None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
        };
        Ok(CarEncodedLengthAfterModel {
            state: EncodedLengthAccumulator {
                len,
                multiplier: 1,
                error: None,
            },
        })
    }
}
impl CarEncodedLengthAfterModel {
    ///Generated method `activation_code`.
    #[inline]
    pub const fn activation_code(
        self,
        byte_len: usize,
    ) -> Result<CarEncodedLengthComplete, sbe_rt::EncodeError> {
        if byte_len > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "activationCode",
                max_length: 1073741824,
                actual: byte_len,
            });
        }
        let len = match self.state.len.checked_add(4 as usize) {
            Some(v) => v,
            None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
        };
        let len = match len.checked_add(byte_len) {
            Some(v) => v,
            None => return Err(sbe_rt::EncodeError::EncodedLengthOverflow),
        };
        Ok(CarEncodedLengthComplete {
            state: EncodedLengthAccumulator {
                len,
                multiplier: 1,
                error: None,
            },
        })
    }
}
impl CarEncodedLengthComplete {
    ///Generated method `encoded_length`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn encoded_length(&self) -> usize {
        self.state.len
    }
    ///Generated method `encoded_length_with_header`.
    #[must_use = "discarding this value is almost always a mistake"]
    #[inline]
    pub const fn encoded_length_with_header(&self) -> usize {
        self.state.len + 8 as usize
    }
}
///Generated module `car_field_meta`.
pub mod car_field_meta {
    ///Generated struct `FieldInfo`.
    pub struct FieldInfo {
        ///Generated field `name`.
        pub name: &'static str,
        ///Generated field `id`.
        pub id: u16,
        ///Generated field `offset`.
        pub offset: usize,
        ///Generated field `since_version`.
        pub since_version: u16,
        ///Generated field `field_type`.
        pub field_type: &'static str,
        ///Generated field `presence`.
        pub presence: &'static str,
        ///Generated field `null_value`.
        pub null_value: Option<&'static str>,
        ///Generated field `semantic_type`.
        pub semantic_type: Option<&'static str>,
        ///Generated field `description`.
        pub description: Option<&'static str>,
    }
    ///Generated constant `FIELDS`.
    pub const FIELDS: &[FieldInfo] = &[
        FieldInfo {
            name: "serialNumber",
            id: 1,
            offset: 0,
            since_version: 0,
            field_type: "u64",
            presence: "required",
            null_value: Some("18446744073709551615"),
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "modelYear",
            id: 2,
            offset: 8,
            since_version: 0,
            field_type: "u16",
            presence: "required",
            null_value: Some("65535"),
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "available",
            id: 3,
            offset: 10,
            since_version: 0,
            field_type: "BooleanType",
            presence: "required",
            null_value: None,
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "code",
            id: 4,
            offset: 11,
            since_version: 0,
            field_type: "Model",
            presence: "required",
            null_value: None,
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "someNumbers",
            id: 5,
            offset: 12,
            since_version: 0,
            field_type: "u32",
            presence: "required",
            null_value: Some("4294967295"),
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "vehicleCode",
            id: 6,
            offset: 28,
            since_version: 0,
            field_type: "u8",
            presence: "required",
            null_value: Some("0"),
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "extras",
            id: 7,
            offset: 34,
            since_version: 0,
            field_type: "OptionalExtras",
            presence: "required",
            null_value: None,
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "discountedModel",
            id: 8,
            offset: 35,
            since_version: 0,
            field_type: "Model",
            presence: "constant",
            null_value: None,
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "engine",
            id: 9,
            offset: 35,
            since_version: 0,
            field_type: "Engine",
            presence: "required",
            null_value: None,
            semantic_type: None,
            description: None,
        },
    ];
}
#[doc(hidden)]
pub(crate) struct EncodedLengthAccumulator {
    len: usize,
    multiplier: usize,
    error: Option<sbe_rt::EncodeError>,
}
impl EncodedLengthAccumulator {
    pub(crate) const fn new(block_length: usize) -> Self {
        Self {
            len: block_length,
            multiplier: 1,
            error: None,
        }
    }
    pub(crate) const fn multiplier(&self) -> usize {
        self.multiplier
    }
    /// Add `unit_len * repetitions * times` in one checked step.
    ///
    /// Identical fixed-width entries contribute the same amount each,
    /// so `times` repetitions of `add_scaled` are one multiplication.
    /// The overflow boundary is unchanged: every term is non-negative,
    /// so the single checked add overflows exactly when some partial
    /// sum of the loop would have.
    pub(crate) const fn add_scaled_repeated(
        &mut self,
        unit_len: usize,
        repetitions: usize,
        times: usize,
    ) {
        if self.error.is_some() {
            return;
        }
        let Some(scaled) = repetitions.checked_mul(times) else {
            self.error = Some(sbe_rt::EncodeError::EncodedLengthOverflow);
            return;
        };
        self.add_scaled(unit_len, scaled);
    }
    pub(crate) const fn add_scaled(&mut self, unit_len: usize, repetitions: usize) {
        if self.error.is_some() {
            return;
        }
        let contribution = match unit_len.checked_mul(repetitions) {
            Some(c) => c,
            None => {
                self.error = Some(sbe_rt::EncodeError::EncodedLengthOverflow);
                return;
            }
        };
        self.len = match self.len.checked_add(contribution) {
            Some(l) => l,
            None => {
                self.error = Some(sbe_rt::EncodeError::EncodedLengthOverflow);
                self.len
            }
        };
    }
    pub(crate) const fn enter_group(
        &mut self,
        count: usize,
        dimension_length: usize,
        entry_block_length: usize,
    ) -> usize {
        let parent_multiplier = self.multiplier;
        self.add_scaled(dimension_length, parent_multiplier);
        self.multiplier = match parent_multiplier.checked_mul(count) {
            Some(m) => m,
            None => {
                self.error = Some(sbe_rt::EncodeError::EncodedLengthOverflow);
                0
            }
        };
        self.add_scaled(entry_block_length, self.multiplier);
        parent_multiplier
    }
    pub(crate) const fn leave_group(&mut self, parent_multiplier: usize) {
        self.multiplier = parent_multiplier;
    }
    pub(crate) const fn fail(&mut self, error: sbe_rt::EncodeError) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }
    pub(crate) const fn check(&self) -> Result<(), sbe_rt::EncodeError> {
        match self.error {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }
    pub(crate) const fn finish(
        self,
        header_length: usize,
    ) -> Result<(usize, usize), sbe_rt::EncodeError> {
        if let Err(e) = self.check() {
            return Err(e);
        }
        match self.len.checked_add(header_length) {
            Some(full) => Ok((self.len, full)),
            None => Err(sbe_rt::EncodeError::EncodedLengthOverflow),
        }
    }
}
/// Builder for ragged/unknown-size entries.
/// `entry_block_length` is 0 for known-size ragged (blocks already
/// counted by `enter_group`) and the actual block length for
/// unknown-size (blocks added per-entry via `add()`/`entries()`).
#[doc(hidden)]
pub struct RaggedEntryBuilder {
    state: EncodedLengthAccumulator,
    parent_multiplier: usize,
    entry_block_length: usize,
    ///Generated field `written`.
    pub written: usize,
}
impl RaggedEntryBuilder {
    fn new(
        state: EncodedLengthAccumulator,
        parent_multiplier: usize,
        entry_block_length: usize,
    ) -> Self {
        Self {
            state,
            parent_multiplier,
            entry_block_length,
            written: 0,
        }
    }
    /// Register one entry (adds entry block for unknown-size groups).
    #[inline]
    pub fn add(&mut self) -> sbe_rt::GroupResult {
        self.state.add_scaled(self.entry_block_length, self.parent_multiplier);
        self.bump_written(1)
    }
    /// Register N flat entries at once (for fixed-width unknown-size groups).
    ///
    /// Equivalent to `n` successful [`Self::add`] calls, including the
    /// boundary at which the entry count overflows.
    #[inline]
    pub fn entries(&mut self, n: usize) -> sbe_rt::GroupResult {
        self.state
            .add_scaled_repeated(self.entry_block_length, self.parent_multiplier, n);
        self.bump_written(n)
    }
    /// Checked entry-count update. An unchecked `written += n` could
    /// wrap and report a count the caller never asked for, which the
    /// group's declared-count check would then silently accept.
    #[inline]
    fn bump_written(&mut self, n: usize) -> sbe_rt::GroupResult {
        match self.written.checked_add(n) {
            Some(total) => {
                self.written = total;
                Ok(())
            }
            None => {
                let error = sbe_rt::EncodeError::GroupCountOverflow {
                    maximum: u32::MAX,
                    actual: u32::MAX,
                };
                self.state.fail(error);
                Err(error)
            }
        }
    }
    /// Add a nested group dimension + entries.
    #[inline]
    pub fn group(
        &mut self,
        dim: usize,
        block: usize,
        count: usize,
    ) -> sbe_rt::GroupResult {
        let pm = self.state.enter_group(count, dim, block);
        self.state.leave_group(pm);
        self.state.check()?;
        Ok(())
    }
    /// Add a nested **ragged** group — entries may differ (e.g. var-data
    /// of differing length per entry). Adds the group dimension once,
    /// then the closure describes each entry (`sub.add()` for the entry
    /// block, `sub.var_data(...)` for per-entry var-data). The closure
    /// receives a sub-builder scoped to this group's parent multiplier.
    #[inline]
    pub fn group_ragged<F>(
        &mut self,
        dim: usize,
        entry_block: usize,
        f: F,
    ) -> sbe_rt::GroupResult
    where
        F: FnOnce(&mut RaggedEntryBuilder) -> sbe_rt::GroupResult,
    {
        let pm = self.state.multiplier();
        self.state.add_scaled(dim, pm);
        let state = core::mem::replace(
            &mut self.state,
            EncodedLengthAccumulator::new(0),
        );
        let mut sub = RaggedEntryBuilder::new(state, pm, entry_block);
        f(&mut sub)?;
        self.state = sub.state;
        self.state.check()?;
        Ok(())
    }
    /// Add a varData field for the current entry.
    #[inline]
    pub fn var_data(&mut self, prefix: usize, byte_len: usize) -> sbe_rt::GroupResult {
        self.state.add_scaled(prefix, self.parent_multiplier);
        self.state.add_scaled(byte_len, self.parent_multiplier);
        self.state.check()?;
        Ok(())
    }
}
///`SEMANTIC_VERSION` = "5.2".
pub const SEMANTIC_VERSION: &str = "5.2";
///`SCHEMA_HASH` = 11133254787130522899.
pub const SCHEMA_HASH: u64 = 11133254787130522899;
///Generated constant `SCHEMA_SHA256`.
pub const SCHEMA_SHA256: [u8; 32] = [
    0x78, 0x48, 0x97, 0x0c, 0x36, 0x8e, 0x7a, 0xf4, 0x8e, 0xdd, 0xc0, 0x8e, 0x75, 0xef,
    0x6d, 0x66, 0xa4, 0xf5, 0xc3, 0x03, 0x4d, 0xc7, 0x4d, 0x37, 0xed, 0x93, 0x11, 0xb0,
    0xf9, 0x87, 0xa2, 0x51,
];
///`SCHEMA_SHA256_HEX` = "7848970c368e7af48eddc08e75ef6d66a4f5c3034dc74d37ed9311b0f987a251".
pub const SCHEMA_SHA256_HEX: &str = "7848970c368e7af48eddc08e75ef6d66a4f5c3034dc74d37ed9311b0f987a251";
///`SCHEMA_ID` = 1.
pub const SCHEMA_ID: u16 = 1;
///`SCHEMA_VERSION` = 0.
pub const SCHEMA_VERSION: u16 = 0;
///Generated module `prelude`.
pub mod prelude {
    ///Generated import `use`.
    pub use super::sbe_rt::{
        DecodeError, EncodeError, VerifyError, MetaAttribute, SbeMessage,
    };
    ///Generated import `use`.
    pub use super::{
        AnyMessage, DecodedFrame, FrameCursor, FramingPolicy, MessageVisitor,
        MessageHeader, MessageHeaderDecoder, GroupSizeEncoding, GroupSizeEncodingDecoder,
        VarStringEncoding, VarStringEncodingDecoder, VarAsciiEncoding,
        VarAsciiEncodingDecoder, VarDataEncoding, VarDataEncodingDecoder, Booster,
        BoosterDecoder, Engine, EngineDecoder, BooleanType, Model, BoostType,
        OptionalExtras, CarDecoder, CarEncoder,
    };
}
/// Read `N` bytes from `buf` at `offset` into a fixed-size array.
///
/// Bounds-checked slice indexing. LLVM elides the check when the
/// slice length is known (stack buffer with visible size).
#[inline]
pub fn read_bytes<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
    buf[offset..offset + N].try_into().expect("read_bytes: buffer too short")
}
///Generated function `write_bytes`.
#[inline]
pub fn write_bytes<const N: usize>(buf: &mut [u8], offset: usize, bytes: &[u8; N]) {
    buf[offset..offset + N].copy_from_slice(bytes);
}
/// Unchecked companion to [`read_bytes`] — zero bounds checks.
///
/// # Safety
/// Caller guarantees `offset + N` does not overflow and
/// `offset + N <= buf.len()`.
#[inline(always)]
#[allow(dead_code)]
unsafe fn read_bytes_unchecked<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
    unsafe { core::ptr::read_unaligned(buf.as_ptr().add(offset) as *const [u8; N]) }
}
/// Unchecked companion to [`write_bytes`] — zero bounds checks.
///
/// # Safety
/// Caller guarantees `offset + N` does not overflow and
/// `offset + N <= buf.len()`.
#[inline]
#[allow(dead_code)]
unsafe fn write_bytes_unchecked<const N: usize>(
    buf: &mut [u8],
    offset: usize,
    bytes: &[u8; N],
) {
    unsafe {
        core::ptr::write_unaligned(buf.as_mut_ptr().add(offset) as *mut [u8; N], *bytes)
    }
}
/// Read `schemaId` from a message header at the start of `buf`.
/// Returns [`None`] if `buf` is shorter than the header field.
#[must_use = "the header schema id is unused; ignoring it skips dispatch"]
#[inline]
pub fn schema_id_from_header(buf: &[u8]) -> Option<u16> {
    if buf.len() < 4 + 2 {
        return None;
    }
    let bytes = read_bytes::<2>(buf, 4);
    let value = u16::from_le_bytes(bytes) as u64;
    u16::try_from(value).ok()
}
/// Tagged union of every message type in the schema — decode once,
/// then `match` to access the typed decoder.
#[non_exhaustive]
pub enum AnyMessage<'a> {
    ///Generated variant `Car`.
    Car(CarDecoder<'a>),
    ///Generated variant `Unknown`.
    Unknown {
        ///Generated field `header`.
        header: MessageHeader,
        /// The complete frame: schema-declared message header
        /// followed by the unparsed body. Not the body alone.
        frame: &'a [u8],
    },
}
/// One decoded message with its buffer range and length.
pub struct DecodedFrame<'a> {
    ///Generated field `message`.
    pub message: AnyMessage<'a>,
    ///Generated field `range`.
    pub range: core::ops::Range<usize>,
    ///Generated field `len`.
    pub len: usize,
}
/// How frames are delimited in a stream: length-prefixed or fixed-size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramingPolicy {
    ///Generated variant `LengthPrefixU32Le`.
    LengthPrefixU32Le,
    ///Generated variant `LengthPrefixU16Le`.
    LengthPrefixU16Le,
    ///Generated variant `Fixed`.
    Fixed(usize),
}
/// Iterator that yields [`DecodedFrame`]s from a byte buffer according
/// to a [`FramingPolicy`].
pub struct FrameCursor<'a> {
    buf: &'a [u8],
    offset: usize,
    framing: FramingPolicy,
}
impl<'a> FrameCursor<'a> {
    ///Generated method `new`.
    #[inline]
    pub const fn new(buf: &'a [u8], framing: FramingPolicy) -> Self {
        Self { buf, offset: 0, framing }
    }
}
impl<'a> core::iter::FusedIterator for FrameCursor<'a> {}
impl<'a> Iterator for FrameCursor<'a> {
    type Item = Result<DecodedFrame<'a>, sbe_rt::DecodeError>;
    /// Fused after the first error.
    ///
    /// A framing error means the boundary between this frame and the
    /// next is no longer known, so every later offset would be a guess.
    /// The cursor moves to the terminal boundary instead: one `Err`,
    /// then permanent `None`. Re-polling a broken stream used to return
    /// the same error forever.
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.offset >= self.buf.len() {
            return None;
        }
        let terminal = self.buf.len();
        let (header_len, frame_len) = match self.framing {
            FramingPolicy::LengthPrefixU32Le => {
                if 4 > self.buf.len().saturating_sub(self.offset) {
                    let available = self.buf.len().saturating_sub(self.offset);
                    self.offset = terminal;
                    return Some(
                        Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "length prefix",
                            needed: 4,
                            available,
                        }),
                    );
                }
                let bytes: [u8; 4] = read_bytes::<4>(self.buf, self.offset);
                let len = u32::from_le_bytes(bytes) as usize;
                (4, len)
            }
            FramingPolicy::LengthPrefixU16Le => {
                if 2 > self.buf.len().saturating_sub(self.offset) {
                    let available = self.buf.len().saturating_sub(self.offset);
                    self.offset = terminal;
                    return Some(
                        Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "length prefix",
                            needed: 2,
                            available,
                        }),
                    );
                }
                let bytes: [u8; 2] = read_bytes::<2>(self.buf, self.offset);
                let len = u16::from_le_bytes(bytes) as usize;
                (2, len)
            }
            FramingPolicy::Fixed(len) => (0, len),
        };
        let available = self.buf.len().saturating_sub(self.offset);
        let frame_start = match self.offset.checked_add(header_len) {
            Some(value) => value,
            None => {
                self.offset = terminal;
                return Some(
                    Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "frame bounds",
                        needed: usize::MAX,
                        available,
                    }),
                );
            }
        };
        let frame_end = match frame_start.checked_add(frame_len) {
            Some(value) => value,
            None => {
                self.offset = terminal;
                return Some(
                    Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "frame bounds",
                        needed: usize::MAX,
                        available,
                    }),
                );
            }
        };
        if frame_end > self.buf.len() {
            self.offset = terminal;
            return Some(
                Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "frame bounds",
                    needed: header_len.saturating_add(frame_len),
                    available,
                }),
            );
        }
        match AnyMessage::decode_frame(self.buf, frame_start, frame_len) {
            Ok(frame) => {
                self.offset = frame_end;
                Some(Ok(frame))
            }
            Err(e) => {
                self.offset = terminal;
                Some(Err(e))
            }
        }
    }
}
impl<'a> AnyMessage<'a> {
    /// Dispatch a framed message with header + version-aware fixed-extent checks.
    #[inline]
    pub fn try_decode(
        buf: &'a [u8],
        offset: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if 8 > buf.len().saturating_sub(offset) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: buf.len().saturating_sub(offset),
            });
        }
        let header_bytes = read_bytes::<8>(buf, offset);
        let header = MessageHeader(header_bytes);
        let template_id = sbe_rt::checked_header_u16(
            "templateId",
            header.template_id() as u64,
        )?;
        let schema_id = sbe_rt::checked_header_u16(
            "schemaId",
            header.schema_id() as u64,
        )?;
        let version = sbe_rt::checked_header_u16("version", header.version() as u64)?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        if schema_id != 1 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 1,
                actual: schema_id,
                expected_name: "baseline",
            });
        }
        match template_id {
            CarSchema::TEMPLATE_ID => {
                Ok(Self::Car(CarDecoder::try_wrap(buf, offset, block_length, version)?))
            }
            _ => {
                Err(sbe_rt::DecodeError::UnknownTemplateLength {
                    template_id,
                })
            }
        }
    }
    /// Trusted multi-template dispatch. Same checks as
    /// [`Self::try_decode`]; prefer `try_decode` at untrusted
    /// boundaries. Dynamic tails remain checked on consume.
    #[inline]
    pub fn decode(buf: &'a [u8], offset: usize) -> Result<Self, sbe_rt::DecodeError> {
        Self::try_decode(buf, offset)
    }
}
impl<'a> AnyMessage<'a> {
    /// Decode one externally framed message.
    ///
    /// `frame_len` is header-inclusive: it is the size of the whole
    /// SBE frame starting at `offset` (message header plus body),
    /// not a body-only length. The declared range
    /// `offset .. offset + frame_len` must fit in `buf`, and
    /// `frame_len` must be at least the schema message-header size,
    /// **before** any header field is read. A shorter declared
    /// length returns [`sbe_rt::DecodeError::BufferTooShort`] with
    /// `field: "message header"` so one frame cannot source
    /// identity bytes from the next framing unit.
    #[inline]
    pub fn decode_frame(
        buf: &'a [u8],
        offset: usize,
        frame_len: usize,
    ) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
        let available = buf.len().saturating_sub(offset);
        let frame_end = match offset.checked_add(frame_len) {
            Some(end) => end,
            None => {
                return Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "message header",
                    needed: 8,
                    available,
                });
            }
        };
        if frame_end > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: frame_len,
                available,
            });
        }
        if frame_len < 8 {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: frame_len,
            });
        }
        let frame = &buf[offset..frame_end];
        let header_bytes: [u8; 8] = read_bytes::<8>(frame, 0);
        let header = MessageHeader(header_bytes);
        let template_id = sbe_rt::checked_header_u16(
            "templateId",
            header.template_id() as u64,
        )?;
        let schema_id = sbe_rt::checked_header_u16(
            "schemaId",
            header.schema_id() as u64,
        )?;
        let version = sbe_rt::checked_header_u16("version", header.version() as u64)?;
        let block_length = sbe_rt::checked_header_usize(
            "blockLength",
            header.block_length() as u64,
        )?;
        let body_offset = offset + 8;
        if schema_id != 1 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 1,
                actual: schema_id,
                expected_name: "baseline",
            });
        }
        match template_id {
            CarSchema::TEMPLATE_ID => {
                let frame_end = offset
                    .checked_add(frame_len)
                    .ok_or(sbe_rt::DecodeError::BufferTooShort {
                        field: "Car",
                        needed: frame_len,
                        available: buf.len().saturating_sub(offset),
                    })?;
                if frame_end > buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "Car",
                        needed: frame_len,
                        available: buf.len().saturating_sub(offset),
                    });
                }
                let decoder = CarDecoder::try_decode(&buf[..frame_end], offset)?;
                Ok(DecodedFrame {
                    message: Self::Car(decoder),
                    range: offset..frame_end,
                    len: frame_len,
                })
            }
            _ => {
                Ok(DecodedFrame {
                    message: Self::Unknown { header, frame },
                    range: offset..frame_end,
                    len: frame_len,
                })
            }
        }
    }
}
impl<'a> AnyMessage<'a> {
    /// Header-inclusive encoded length of this variant.
    /// [`Self::Unknown`] reports the matched frame length.
    #[inline]
    pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
        match self {
            Self::Car(d) => d.encoded_length_with_header(),
            Self::Unknown { frame, .. } => Ok(frame.len()),
        }
    }
}
impl<'a> AnyMessage<'a> {
    /// Complete SBE frame (message header + body) — for
    /// [`Self::Unknown`] this is the same header-plus-body range
    /// the cursor matched.
    #[inline]
    pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        match self {
            Self::Car(d) => d.as_bytes_with_header(),
            Self::Unknown { frame, .. } => Ok(frame),
        }
    }
}
impl<'a> AnyMessage<'a> {
    /// Copy this message's header-inclusive frame into `buf`.
    /// Unknown templates copy the matched frame bytes.
    #[inline]
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
        match self {
            Self::Car(d) => {
                let len = d.encoded_length_with_header()?;
                if len > buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        field: "AnyMessage::encode",
                        needed: len,
                        available: buf.len(),
                    });
                }
                let bytes = d.as_bytes_with_header()?;
                buf[..len].copy_from_slice(bytes);
                Ok(len)
            }
            Self::Unknown { frame, .. } => {
                if frame.len() > buf.len() {
                    return Err(sbe_rt::EncodeError::BufferTooShort {
                        field: "AnyMessage::encode",
                        needed: frame.len(),
                        available: buf.len(),
                    });
                }
                buf[..frame.len()].copy_from_slice(frame);
                Ok(frame.len())
            }
        }
    }
}
///Generated trait `MessageVisitor`.
pub trait MessageVisitor {
    ///Generated type `Output`.
    type Output;
    ///Generated method `visit_car`.
    fn visit_car(&mut self, decoder: &CarDecoder<'_>) -> Self::Output;
    /// Called for unknown template IDs (not in this schema).
    ///
    /// `header` is the parsed schema-declared MessageHeader.
    /// `frame` is the complete frame — message header followed by
    /// the unparsed body — not the body alone. Must be implemented;
    /// there is no panicking default, because an unknown template
    /// is application policy rather than a crash.
    fn visit_unknown(&mut self, header: &MessageHeader, frame: &[u8]) -> Self::Output;
}
impl<'a> AnyMessage<'a> {
    ///Generated method `visit`.
    #[inline]
    pub fn visit<V: MessageVisitor>(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::Car(d) => visitor.visit_car(d),
            Self::Unknown { header, frame } => visitor.visit_unknown(header, frame),
        }
    }
}

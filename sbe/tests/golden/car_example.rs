/// Generated from SBE schema package `baseline` id 1 version 0.
#[allow(
    clippy::absurd_extreme_comparisons,
    clippy::double_must_use,
    clippy::erasing_op,
    clippy::identity_op,
    clippy::unnecessary_cast,
    unused_assignments,
    unused_comparisons
)]
#[allow(non_camel_case_types)]
#[allow(non_snake_case)]
#[allow(clippy::identity_op)]
#[allow(clippy::eq_op)]
#[allow(clippy::needless_borrow)]
#[allow(clippy::manual_range_contains)]
#[allow(unused_imports)]
#[allow(unused_variables)]
#[allow(unused_mut)]
#[allow(dead_code)]
pub mod sbe_rt {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum DecodeError {
        BufferTooShort { field: &'static str, needed: usize, available: usize },
        WrongSchema { expected: u16, actual: u16, expected_name: &'static str },
        UnknownTemplateLength { template_id: u16 },
        InvalidHeaderValue { field: &'static str, value: u64, maximum: u64 },
        InvalidVarDataLength { field: &'static str, length: u64, max_length: u64 },
        /// Field/group/data was added in a schema version later than the wire message.
        FieldNotInVersion { field: &'static str, wire_version: u16, since_version: u16 },
        InvalidUtf8 { field: &'static str, error: core::str::Utf8Error },
        InvalidAscii { field: &'static str },
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
                        f, "var data field '{}: length {} exceeds max {}", field, length,
                        max_length
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
            }
        }
    }
    impl core::error::Error for DecodeError {}
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EncodeError {
        BufferTooShort { needed: usize, available: usize },
        VarDataTooLong { field: &'static str, max_length: usize, actual: usize },
        /// Fixed char/byte array source longer than the schema length.
        FixedArrayTooLong { field: &'static str, max_length: usize, actual: usize },
        /// Domain/DTO value outside the schema min/max range.
        ValueOutOfRange { field: &'static str, min: i128, max: i128, actual: i128 },
        GroupFull { declared: u32, attempted: u32 },
        /// Known-size group closure returned without adding enough entries.
        GroupCountMismatch { declared: u32, actual: u32 },
        /// Unknown-size group entry count does not fit in `numInGroup`.
        GroupCountOverflow { maximum: u32, actual: u32 },
        /// Checked arithmetic overflow in encoded length computation.
        EncodedLengthOverflow,
        Decode(DecodeError),
    }
    impl core::fmt::Display for EncodeError {
        #[cold]
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::BufferTooShort { needed, available } => {
                    write!(
                        f, "buffer too short: needed {}, available {}", needed, available
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
                Self::Decode(e) => write!(f, "decode error: {e}"),
            }
        }
    }
    impl core::error::Error for EncodeError {}
    impl From<DecodeError> for EncodeError {
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VerifyError {
        HeaderTooShort,
        InvalidBlockLength { expected_min: usize, actual: usize },
        GroupDimOutOfBounds { field: &'static str, offset: usize },
        VarDataOutOfBounds { field: &'static str, offset: usize, length: u64 },
        MessageTooShort { needed: usize, available: usize },
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
        fn from(e: DecodeError) -> Self {
            VerifyError::DecodeError(e)
        }
    }
    impl core::error::Error for VerifyError {}
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
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is not a generated SBE message type",
        note = "SbeMessage is a sealed trait — only types generated by `ergo_sbe::Generator` can implement it. Import the generated module and use the provided decoder/encoder types directly."
    )]
    pub trait SbeMessage {
        const TEMPLATE_ID: u16;
        const BLOCK_LENGTH: usize;
        const SCHEMA_ID: u16;
        const SCHEMA_VERSION: u16;
    }
    pub mod private {
        pub trait Sealed {}
    }
    pub trait HeaderState: private::Sealed {}
    pub struct HeaderPresent;
    impl private::Sealed for HeaderPresent {}
    impl HeaderState for HeaderPresent {}
    pub struct HeaderAbsent;
    impl private::Sealed for HeaderAbsent {}
    impl HeaderState for HeaderAbsent {}
    /// Return type for group closures (`add`, `bids`, …).
    /// Closures return `Result<(), EncodeError>`; `?` just works.
    pub type GroupResult = Result<(), EncodeError>;
    pub trait IntoGroupResult {
        fn into_group_result(self) -> GroupResult;
    }
    impl IntoGroupResult for () {
        fn into_group_result(self) -> GroupResult {
            Ok(())
        }
    }
    impl IntoGroupResult for GroupResult {
        fn into_group_result(self) -> GroupResult {
            self
        }
    }
}
///Boolean Type.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BooleanType {
    F = 0,
    T = 1,
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal = 255,
}
impl BooleanType {
    pub fn raw(self) -> u8 {
        self as u8
    }
    pub const fn from_raw(val: u8) -> Self {
        match val {
            0 => Self::F,
            1 => Self::T,
            _ => Self::NullVal,
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
        match self {
            Self::F => f.write_str(stringify!(F)),
            Self::T => f.write_str(stringify!(T)),
            Self::NullVal => f.write_str("NullVal"),
        }
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
impl From<BooleanType> for bool {
    #[inline]
    fn from(val: BooleanType) -> bool {
        val as u8 != 0
    }
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Model {
    A = b'A',
    B = b'B',
    C = b'C',
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal = 0,
}
impl Model {
    pub fn raw(self) -> u8 {
        self as u8
    }
    pub const fn from_raw(val: u8) -> Self {
        match val {
            b'A' => Self::A,
            b'B' => Self::B,
            b'C' => Self::C,
            _ => Self::NullVal,
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
        match self {
            Self::A => f.write_str(stringify!(A)),
            Self::B => f.write_str(stringify!(B)),
            Self::C => f.write_str(stringify!(C)),
            Self::NullVal => f.write_str("NullVal"),
        }
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
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BoostType {
    TURBO = b'T',
    SUPERCHARGER = b'S',
    NITROUS = b'N',
    KERS = b'K',
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal = 0,
}
impl BoostType {
    pub fn raw(self) -> u8 {
        self as u8
    }
    pub const fn from_raw(val: u8) -> Self {
        match val {
            b'T' => Self::TURBO,
            b'S' => Self::SUPERCHARGER,
            b'N' => Self::NITROUS,
            b'K' => Self::KERS,
            _ => Self::NullVal,
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
        match self {
            Self::TURBO => f.write_str(stringify!(TURBO)),
            Self::SUPERCHARGER => f.write_str(stringify!(SUPERCHARGER)),
            Self::NITROUS => f.write_str(stringify!(NITROUS)),
            Self::KERS => f.write_str(stringify!(KERS)),
            Self::NullVal => f.write_str("NullVal"),
        }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[repr(transparent)]
pub struct OptionalExtras(pub u8);
impl OptionalExtras {
    #[inline]
    pub const fn raw(self) -> u8 {
        self.0
    }
    #[inline]
    pub const fn default() -> Self {
        Self(0)
    }
    #[inline]
    pub const fn is_sun_roof(self) -> bool {
        (self.0 & (1 << 0)) != 0
    }
    #[inline]
    pub fn sun_roof(&mut self, val: bool) -> &mut Self {
        if val {
            self.0 |= 1 << 0;
        } else {
            self.0 &= !(1 << 0);
        }
        self
    }
    #[inline]
    pub const fn is_sports_pack(self) -> bool {
        (self.0 & (1 << 1)) != 0
    }
    #[inline]
    pub fn sports_pack(&mut self, val: bool) -> &mut Self {
        if val {
            self.0 |= 1 << 1;
        } else {
            self.0 &= !(1 << 1);
        }
        self
    }
    #[inline]
    pub const fn is_cruise_control(self) -> bool {
        (self.0 & (1 << 2)) != 0
    }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct MessageHeader(pub [u8; 8]);
impl MessageHeader {
    #[inline]
    pub fn block_length(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 0))
    }
    #[inline]
    pub fn template_id(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 2))
    }
    #[inline]
    pub fn schema_id(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 4))
    }
    #[inline]
    pub fn version(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 6))
    }
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
impl MessageHeader {
    /// Read `(template_id, schema_id)` from a frame without
    /// constructing a full `MessageHeader`. Returns `None`
    /// when the buffer is shorter than the header.
    #[inline]
    pub fn peek_header(data: &[u8]) -> Option<(u16, u16)> {
        if data.len() < 8 {
            return None;
        }
        let mut hdr = [0u8; 8];
        hdr.copy_from_slice(&data[..8]);
        let this = Self(hdr);
        let template_id = u16::try_from(this.template_id() as u64).ok()?;
        let schema_id = u16::try_from(this.schema_id() as u64).ok()?;
        Some((template_id, schema_id))
    }
    /// Read `template_id` from a frame without constructing a full
    /// `MessageHeader`. Returns `None` when the buffer is shorter
    /// than the header. For correct multi-schema dispatch,
    /// prefer [`Self::peek_header`] which also returns `schema_id`.
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
    #[inline]
    pub fn peek_for_schema(data: &[u8], expected_schema_id: u16) -> Option<u16> {
        let (tid, sid) = Self::peek_header(data)?;
        if sid == expected_schema_id { Some(tid) } else { None }
    }
}
#[derive(Clone, Copy)]
pub struct MessageHeaderDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
}
impl<'a> MessageHeaderDecoder<'a> {
    #[inline]
    pub fn block_length(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
    #[inline]
    pub fn template_id(&self) -> u16 {
        let offset = self.pos + 2;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
    #[inline]
    pub fn schema_id(&self) -> u16 {
        let offset = self.pos + 4;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
    #[inline]
    pub fn version(&self) -> u16 {
        let offset = self.pos + 6;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
}
///Repeating group dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct GroupSizeEncoding(pub [u8; 4]);
impl GroupSizeEncoding {
    #[inline]
    pub fn block_length(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 0))
    }
    #[inline]
    pub fn num_in_group(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 2))
    }
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
#[derive(Clone, Copy)]
pub struct GroupSizeEncodingDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
}
impl<'a> GroupSizeEncodingDecoder<'a> {
    #[inline]
    pub fn block_length(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
    #[inline]
    pub fn num_in_group(&self) -> u16 {
        let offset = self.pos + 2;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
}
///Variable length UTF-8 String.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarStringEncoding(pub [u8; 4]);
impl VarStringEncoding {
    #[inline]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes(read_bytes::<4>(&self.0, 0))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
    pub fn new(length: u32, var_data: [u8; 0]) -> Self {
        let mut bytes = [0u8; 4];
        let val_bytes = length.to_le_bytes();
        write_bytes::<4>(&mut bytes, 0, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < VarStringEncoding > () == 4);
#[derive(Clone, Copy)]
pub struct VarStringEncodingDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
}
impl<'a> VarStringEncodingDecoder<'a> {
    #[inline]
    pub fn length(&self) -> u32 {
        let offset = self.pos + 0;
        u32::from_le_bytes(read_bytes_unchecked::<4>(self.buf, offset))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
///Variable length ASCII String.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarAsciiEncoding(pub [u8; 4]);
impl VarAsciiEncoding {
    #[inline]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes(read_bytes::<4>(&self.0, 0))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
    pub fn new(length: u32, var_data: [u8; 0]) -> Self {
        let mut bytes = [0u8; 4];
        let val_bytes = length.to_le_bytes();
        write_bytes::<4>(&mut bytes, 0, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < VarAsciiEncoding > () == 4);
#[derive(Clone, Copy)]
pub struct VarAsciiEncodingDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
}
impl<'a> VarAsciiEncodingDecoder<'a> {
    #[inline]
    pub fn length(&self) -> u32 {
        let offset = self.pos + 0;
        u32::from_le_bytes(read_bytes_unchecked::<4>(self.buf, offset))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
///Variable length binary blob.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarDataEncoding(pub [u8; 4]);
impl VarDataEncoding {
    #[inline]
    pub fn length(&self) -> u32 {
        u32::from_le_bytes(read_bytes::<4>(&self.0, 0))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
    pub fn new(length: u32, var_data: [u8; 0]) -> Self {
        let mut bytes = [0u8; 4];
        let val_bytes = length.to_le_bytes();
        write_bytes::<4>(&mut bytes, 0, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < VarDataEncoding > () == 4);
#[derive(Clone, Copy)]
pub struct VarDataEncodingDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
}
impl<'a> VarDataEncodingDecoder<'a> {
    #[inline]
    pub fn length(&self) -> u32 {
        let offset = self.pos + 0;
        u32::from_le_bytes(read_bytes_unchecked::<4>(self.buf, offset))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Booster(pub [u8; 2]);
impl Booster {
    #[inline]
    pub fn boost_type(&self) -> BoostType {
        BoostType::from_raw(u8::from_le_bytes(read_bytes::<1>(&self.0, 0)))
    }
    #[inline]
    pub fn horse_power(&self) -> u8 {
        u8::from_le_bytes(read_bytes::<1>(&self.0, 1))
    }
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
#[derive(Clone, Copy)]
pub struct BoosterDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
}
impl<'a> BoosterDecoder<'a> {
    #[inline]
    pub fn boost_type(&self) -> BoostType {
        let offset = self.pos + 0;
        BoostType::from_raw(
            u8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset)),
        )
    }
    #[inline]
    pub fn horse_power(&self) -> u8 {
        let offset = self.pos + 1;
        u8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Engine(pub [u8; 10]);
impl Engine {
    #[inline]
    pub fn capacity(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 0))
    }
    #[inline]
    pub fn num_cylinders(&self) -> u8 {
        u8::from_le_bytes(read_bytes::<1>(&self.0, 2))
    }
    #[inline]
    pub const fn max_rpm(&self) -> u16 {
        9000
    }
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
    #[inline]
    pub const fn fuel(&self) -> &'static str {
        "Petrol"
    }
    #[inline]
    pub fn efficiency(&self) -> i8 {
        i8::from_le_bytes(read_bytes::<1>(&self.0, 6))
    }
    #[inline]
    pub fn booster_enabled(&self) -> BooleanType {
        BooleanType::from_raw(u8::from_le_bytes(read_bytes::<1>(&self.0, 7)))
    }
    #[inline]
    pub fn booster(&self) -> Booster {
        Booster(read_bytes::<2>(&self.0, 8))
    }
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
#[derive(Clone, Copy)]
pub struct EngineDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
}
impl<'a> EngineDecoder<'a> {
    #[inline]
    pub fn capacity(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
    #[inline]
    pub fn num_cylinders(&self) -> u8 {
        let offset = self.pos + 2;
        u8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset))
    }
    #[inline]
    pub const fn max_rpm(&self) -> u16 {
        9000
    }
    #[inline]
    pub fn manufacturer_code(&self) -> [u8; 3] {
        let mut res = [0 as u8; 3];
        let mut idx = 0;
        while idx < 3 {
            let offset = self.pos + 3 + idx * 1;
            res[idx] = u8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset));
            idx += 1;
        }
        res
    }
    #[inline]
    pub const fn fuel(&self) -> &'static str {
        "Petrol"
    }
    #[inline]
    pub fn efficiency(&self) -> i8 {
        let offset = self.pos + 6;
        i8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset))
    }
    #[inline]
    pub fn booster_enabled(&self) -> BooleanType {
        let offset = self.pos + 7;
        BooleanType::from_raw(
            u8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset)),
        )
    }
    #[inline]
    pub fn booster(&self) -> Booster {
        let offset = self.pos + 8;
        Booster(read_bytes_unchecked::<2>(self.buf, offset))
    }
}
///Description of a basic Car
pub struct CarSchema;
impl CarSchema {
    pub const SCHEMA_ID: u16 = 1;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 45;
    pub const HEADER_LENGTH: usize = 8;
}
pub struct CarDecoder<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) msg_offset: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
impl<'a> CarDecoder<'a> {
    pub const SCHEMA_ID: u16 = 1;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 45;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 45);
    /// Schema-declared message header size in bytes.
    pub const HEADER_LENGTH: usize = 8;
    ///MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        message_offset: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        let body_pos = message_offset + Self::HEADER_LENGTH;
        Self {
            buf,
            pos: body_pos,
            msg_offset: message_offset,
            acting_block_length,
            acting_version,
        }
    }
    #[inline]
    pub fn try_wrap_and_apply_header(
        buf: &'a [u8],
        pos: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if 8 > buf.len().saturating_sub(pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: buf.len().saturating_sub(pos),
            });
        }
        let header_bytes: [u8; 8] = read_bytes::<8>(buf, pos);
        let header = MessageHeader(header_bytes);
        let template_id = sbe_rt::checked_header_u16(
            "templateId",
            header.template_id() as u64,
        )?;
        if template_id != Self::TEMPLATE_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::TEMPLATE_ID,
                actual: template_id,
                expected_name: "baseline",
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
        let body_pos = pos + 8;
        if acting_block_length > buf.len().saturating_sub(body_pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message body",
                needed: (8 as usize).saturating_add(acting_block_length),
                available: buf.len().saturating_sub(pos),
            });
        }
        let acting_version = sbe_rt::checked_header_u16(
            "version",
            header.version() as u64,
        )?;
        let mut dec = Self::wrap(buf, pos, acting_block_length, acting_version);
        Ok(dec)
    }
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
    #[inline]
    pub fn serial_number(&self) -> u64 {
        let offset = self.pos + 0;
        u64::from_le_bytes(read_bytes_unchecked::<8>(self.buf, offset))
    }
    pub const SERIAL_NUMBER_ID: u16 = 1;
    pub const SERIAL_NUMBER_SINCE_VERSION: u16 = 0;
    pub const SERIAL_NUMBER_ENCODING_OFFSET: usize = 0;
    pub const SERIAL_NUMBER_ENCODING_LENGTH: usize = 8;
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
    pub const SERIAL_NUMBER_NULL: u64 = 18446744073709551615_u64;
    pub const SERIAL_NUMBER_MIN: u64 = 0_u64;
    pub const SERIAL_NUMBER_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    pub fn model_year(&self) -> u16 {
        let offset = self.pos + 8;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
    pub const MODEL_YEAR_ID: u16 = 2;
    pub const MODEL_YEAR_SINCE_VERSION: u16 = 0;
    pub const MODEL_YEAR_ENCODING_OFFSET: usize = 8;
    pub const MODEL_YEAR_ENCODING_LENGTH: usize = 2;
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
    pub const MODEL_YEAR_NULL: u16 = 65535_u16;
    pub const MODEL_YEAR_MIN: u16 = 0_u16;
    pub const MODEL_YEAR_MAX: u16 = 65534_u16;
    #[inline]
    pub fn available(&self) -> BooleanType {
        let offset = self.pos + 10;
        BooleanType::from_raw(
            u8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset)),
        )
    }
    #[inline]
    pub fn available_bool(&self) -> bool {
        bool::from(self.available())
    }
    pub const AVAILABLE_ID: u16 = 3;
    pub const AVAILABLE_SINCE_VERSION: u16 = 0;
    pub const AVAILABLE_ENCODING_OFFSET: usize = 10;
    pub const AVAILABLE_ENCODING_LENGTH: usize = 1;
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
    pub const AVAILABLE_NULL: BooleanType = BooleanType::NullVal;
    #[inline]
    pub fn code(&self) -> Model {
        let offset = self.pos + 11;
        Model::from_raw(u8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset)))
    }
    pub const CODE_ID: u16 = 4;
    pub const CODE_SINCE_VERSION: u16 = 0;
    pub const CODE_ENCODING_OFFSET: usize = 11;
    pub const CODE_ENCODING_LENGTH: usize = 1;
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
    pub const CODE_NULL: Model = Model::NullVal;
    #[inline]
    pub fn some_numbers(&self) -> [u32; 4] {
        if 28 > self.acting_block_length {
            return [0 as u32; 4];
        }
        let offset = self.pos + 12;
        let all: [u8; 16] = read_bytes_unchecked::<16>(self.buf, offset);
        [
            u32::from_le_bytes([all[0usize], all[1usize], all[2usize], all[3usize]]),
            u32::from_le_bytes([all[4usize], all[5usize], all[6usize], all[7usize]]),
            u32::from_le_bytes([all[8usize], all[9usize], all[10usize], all[11usize]]),
            u32::from_le_bytes([all[12usize], all[13usize], all[14usize], all[15usize]]),
        ]
    }
    pub const SOME_NUMBERS_ID: u16 = 5;
    pub const SOME_NUMBERS_SINCE_VERSION: u16 = 0;
    pub const SOME_NUMBERS_ENCODING_OFFSET: usize = 12;
    pub const SOME_NUMBERS_ENCODING_LENGTH: usize = 16;
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
    pub const SOME_NUMBERS_NULL: u32 = 4294967295_u32;
    pub const SOME_NUMBERS_MIN: u32 = 0_u32;
    pub const SOME_NUMBERS_MAX: u32 = 4294967294_u32;
    #[inline]
    pub fn vehicle_code(&self) -> [u8; 6] {
        if 34 > self.acting_block_length {
            return [0 as u8; 6];
        }
        let offset = self.pos + 28;
        let all: [u8; 6] = read_bytes_unchecked::<6>(self.buf, offset);
        [
            u8::from_le_bytes([all[0usize]]),
            u8::from_le_bytes([all[1usize]]),
            u8::from_le_bytes([all[2usize]]),
            u8::from_le_bytes([all[3usize]]),
            u8::from_le_bytes([all[4usize]]),
            u8::from_le_bytes([all[5usize]]),
        ]
    }
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
    pub const VEHICLE_CODE_ID: u16 = 6;
    pub const VEHICLE_CODE_SINCE_VERSION: u16 = 0;
    pub const VEHICLE_CODE_ENCODING_OFFSET: usize = 28;
    pub const VEHICLE_CODE_ENCODING_LENGTH: usize = 6;
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
    pub const VEHICLE_CODE_NULL: u8 = 0_u8;
    pub const VEHICLE_CODE_MIN: u8 = 32_u8;
    pub const VEHICLE_CODE_MAX: u8 = 126_u8;
    #[inline]
    pub fn extras(&self) -> OptionalExtras {
        let offset = self.pos + 34;
        OptionalExtras(u8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset)))
    }
    pub const EXTRAS_ID: u16 = 7;
    pub const EXTRAS_SINCE_VERSION: u16 = 0;
    pub const EXTRAS_ENCODING_OFFSET: usize = 34;
    pub const EXTRAS_ENCODING_LENGTH: usize = 1;
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
    #[inline]
    pub const fn discounted_model(&self) -> Model {
        Model::C
    }
    pub const DISCOUNTED_MODEL_ID: u16 = 8;
    pub const DISCOUNTED_MODEL_SINCE_VERSION: u16 = 0;
    pub const DISCOUNTED_MODEL_ENCODING_OFFSET: usize = 35;
    pub const DISCOUNTED_MODEL_ENCODING_LENGTH: usize = 1;
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
    pub const DISCOUNTED_MODEL_NULL: Model = Model::NullVal;
    #[inline]
    pub fn engine(&self) -> EngineDecoder<'_> {
        let offset = self.pos + 35;
        EngineDecoder {
            buf: self.buf,
            pos: offset,
        }
    }
    #[inline]
    pub fn engine_value(&self) -> Engine {
        let offset = self.pos + 35;
        Engine(read_bytes_unchecked::<10>(self.buf, offset))
    }
    pub const ENGINE_ID: u16 = 9;
    pub const ENGINE_SINCE_VERSION: u16 = 0;
    pub const ENGINE_ENCODING_OFFSET: usize = 35;
    pub const ENGINE_ENCODING_LENGTH: usize = 10;
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
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = FuelFiguresEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            idx += 1;
        }
        Ok(pos)
    }
    #[inline]
    fn tail_offset_2(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_1()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = PerformanceFiguresEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            idx += 1;
        }
        Ok(pos)
    }
    #[inline]
    fn tail_offset_3(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_2()?;
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
    fn tail_offset_4(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_3()?;
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
    fn tail_offset_5(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_4()?;
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
    fn fuel_figures(&self) -> Result<FuelFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        FuelFiguresDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn performance_figures(
        &self,
    ) -> Result<PerformanceFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        PerformanceFiguresDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn manufacturer(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
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
    #[inline]
    fn manufacturer_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.manufacturer()?;
        core::str::from_utf8(bytes)
            .map_err(|e| sbe_rt::DecodeError::InvalidUtf8 {
                field: "manufacturer",
                error: e,
            })
    }
    /// View this text var-data field as `&str` without UTF-8
    /// validation.
    ///
    /// # Safety
    ///
    /// The wire bytes must be valid UTF-8. For schema-declared
    /// ASCII encoding this is always true (ASCII ⊂ UTF-8).
    #[inline]
    pub unsafe fn manufacturer_as_str_unchecked(&self) -> &'a str {
        let bytes = unsafe { self.manufacturer().unwrap_unchecked() };
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }
    #[inline]
    pub fn model(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_3()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
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
    #[inline]
    fn model_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.model()?;
        core::str::from_utf8(bytes)
            .map_err(|e| sbe_rt::DecodeError::InvalidUtf8 {
                field: "model",
                error: e,
            })
    }
    /// View this text var-data field as `&str` without UTF-8
    /// validation.
    ///
    /// # Safety
    ///
    /// The wire bytes must be valid UTF-8. For schema-declared
    /// ASCII encoding this is always true (ASCII ⊂ UTF-8).
    #[inline]
    pub unsafe fn model_as_str_unchecked(&self) -> &'a str {
        let bytes = unsafe { self.model().unwrap_unchecked() };
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }
    #[inline]
    pub fn activation_code(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_4()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
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
    #[inline]
    fn activation_code_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.activation_code()?;
        core::str::from_utf8(bytes)
            .map_err(|e| sbe_rt::DecodeError::InvalidUtf8 {
                field: "activation_code",
                error: e,
            })
    }
    /// View this text var-data field as `&str` without UTF-8
    /// validation.
    ///
    /// # Safety
    ///
    /// The wire bytes must be valid UTF-8. For schema-declared
    /// ASCII encoding this is always true (ASCII ⊂ UTF-8).
    #[inline]
    pub unsafe fn activation_code_as_str_unchecked(&self) -> &'a str {
        let bytes = unsafe { self.activation_code().unwrap_unchecked() };
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }
    /// Consume this stage and return a fresh decoder at the initial
    /// message position. The consumed stage cannot be reused.
    #[inline]
    pub fn rewind(self) -> Self {
        self
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        let end = self.tail_offset_5()?;
        Ok(end - self.pos)
    }
    #[inline]
    pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
        let len = self.encoded_length()?;
        Ok(len + 8)
    }
    #[inline]
    pub fn as_body_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let end = self.tail_offset_5()?;
        Ok(&self.buf[self.pos..end])
    }
    #[inline]
    pub fn as_bytes_with_header(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let end = self.tail_offset_5()?;
        Ok(&self.buf[self.msg_offset..end])
    }
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
            let count = dim.num_in_group() as usize;
            let mut entry_pos = offset + 4;
            for _ in 0..count {
                match FuelFiguresEntryDecoder::skip(buf, entry_pos, 6, 0) {
                    Ok(next) => entry_pos = next,
                    Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
                }
            }
            offset = entry_pos;
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
            let count = dim.num_in_group() as usize;
            let mut entry_pos = offset + 4;
            for _ in 0..count {
                match PerformanceFiguresEntryDecoder::skip(buf, entry_pos, 1, 0) {
                    Ok(next) => entry_pos = next,
                    Err(e) => return Err(sbe_rt::VerifyError::DecodeError(e)),
                }
            }
            offset = entry_pos;
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
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::try_wrap_and_apply_header(buf, 0)
    }
}
impl<'a> sbe_rt::private::Sealed for CarDecoder<'a> {}
impl<'a> sbe_rt::SbeMessage for CarDecoder<'a> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 45;
    const SCHEMA_ID: u16 = 1;
    const SCHEMA_VERSION: u16 = 0;
}
impl<'a> CarDecoder<'a> {
    /// Fallible byte view of the complete SBE frame (header + body).
    /// Returns `None` if the buffer is malformed or truncated.
    /// Prefer [`Self::as_bytes_with_header`] for explicit error handling.
    pub fn as_ref_opt(&self) -> Option<&[u8]> {
        self.as_bytes_with_header().ok()
    }
}
impl<'a> core::fmt::Display for CarDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(self, f)
    }
}
impl<'a> core::fmt::Debug for CarDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let mut d = f.debug_struct("CarDecoder");
        if self.pos.saturating_add(8) <= self.buf.len() && 8 <= self.acting_block_length
        {
            let v = self.serial_number();
            d.field("serialNumber", &v);
        }
        if self.pos.saturating_add(10) <= self.buf.len()
            && 10 <= self.acting_block_length
        {
            let v = self.model_year();
            d.field("modelYear", &v);
        }
        if self.pos.saturating_add(11) <= self.buf.len()
            && 11 <= self.acting_block_length
        {
            let v = self.available();
            d.field("available", &v);
        }
        if self.pos.saturating_add(12) <= self.buf.len()
            && 12 <= self.acting_block_length
        {
            let v = self.code();
            d.field("code", &v);
        }
        if self.pos.saturating_add(35) <= self.buf.len()
            && 35 <= self.acting_block_length
        {
            let v = self.extras();
            d.field("extras", &format_args!("{}", v));
        }
        if self.pos.saturating_add(45) <= self.buf.len()
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
pub struct FuelFiguresDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
    acting_block_length: usize,
    parent_pos: usize,
    parent_block_length: usize,
}
impl<'a> FuelFiguresDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Self::wrap_with_parent(buf, pos, acting_version, 0, 0)
    }
    /// Like `wrap()` but remembers the parent message body position and
    /// acting block length so `finish()` can rebuild the next stage.
    #[inline]
    pub fn wrap_with_parent(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if 4 > buf.len().saturating_sub(pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_length = header.block_length() as usize;
        let entries_start = pos + 4;
        Ok(Self {
            buf,
            pos: entries_start,
            count,
            start: entries_start,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}
impl<'a> FuelFiguresDecoder<'a> {
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.count
    }
    /// Dimension wrap (trusted position): the caller has
    /// proven `pos` is within a validated extent.
    #[inline]
    pub fn wrap_trusted(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Self {
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_length = header.block_length() as usize;
        Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
        }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        self.pos = self.start;
        self.count = self.total;
        self
    }
}
impl<'a> FuelFiguresDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: n.saturating_mul(Self::ENTRY_BLOCK_LENGTH),
                available: self.count.saturating_mul(Self::ENTRY_BLOCK_LENGTH),
            });
        }
        for _ in 0..n {
            let entry = FuelFiguresEntryDecoder::wrap(
                self.buf,
                self.pos,
                self.acting_block_length,
                self.acting_version,
            );
            self.pos += entry.encoded_length()?;
            self.count -= 1;
        }
        Ok(())
    }
}
impl<'a> FuelFiguresDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<FuelFiguresEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: idx.saturating_add(1).saturating_mul(Self::ENTRY_BLOCK_LENGTH),
                available: self.total.saturating_mul(Self::ENTRY_BLOCK_LENGTH),
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
        let entry = FuelFiguresEntryDecoder::wrap(
            self.buf,
            offset,
            self.acting_block_length,
            self.acting_version,
        );
        entry.encoded_length()?;
        Ok(entry)
    }
}
impl<'a> Iterator for FuelFiguresDecoder<'a> {
    type Item = Result<FuelFiguresEntryDecoder<'a>, sbe_rt::DecodeError>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = FuelFiguresEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_block_length,
            self.acting_version,
        );
        let size = match entry.encoded_length() {
            Ok(s) => s,
            Err(e) => {
                self.count = 0;
                return Some(Err(e));
            }
        };
        self.pos += size;
        self.count -= 1;
        Some(Ok(entry))
    }
}
impl<'a> ExactSizeIterator for FuelFiguresDecoder<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.count
    }
}
pub struct FuelFiguresEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
    /// One-shot entry-extent cache: filled by
    /// `encoded_length`, reused by the last var-data accessor.
    tail_end: core::cell::Cell<Option<usize>>,
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        Self {
            buf,
            pos,
            acting_version,
            acting_block_length,
            tail_end: core::cell::Cell::new(None),
        }
    }
    #[inline]
    pub fn speed(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
    pub const SPEED_ID: u16 = 11;
    pub const SPEED_SINCE_VERSION: u16 = 0;
    pub const SPEED_ENCODING_OFFSET: usize = 0;
    pub const SPEED_ENCODING_LENGTH: usize = 2;
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
    pub const SPEED_NULL: u16 = 65535_u16;
    pub const SPEED_MIN: u16 = 0_u16;
    pub const SPEED_MAX: u16 = 65534_u16;
    #[inline]
    pub fn mpg(&self) -> f32 {
        let offset = self.pos + 2;
        f32::from_le_bytes(read_bytes_unchecked::<4>(self.buf, offset))
    }
    pub const MPG_ID: u16 = 12;
    pub const MPG_SINCE_VERSION: u16 = 0;
    pub const MPG_ENCODING_OFFSET: usize = 2;
    pub const MPG_ENCODING_LENGTH: usize = 4;
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
    pub const MPG_NULL: f32 = f32::from_bits(2139095041u32);
    pub const MPG_MIN: f32 = f32::from_bits(4286578687u32);
    pub const MPG_MAX: f32 = f32::from_bits(2139095039u32);
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        if self.acting_block_length > self.buf.len().saturating_sub(self.pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: self.acting_block_length,
                available: self.buf.len().saturating_sub(self.pos),
            });
        }
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
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
    pub fn usage_description(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        if let Some(end) = self.tail_end.get() {
            let data_offset = self.pos + self.acting_block_length + 4;
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
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        if let Some(end) = self.tail_end.get() {
            return Ok(end - self.pos);
        }
        let end = self.tail_offset_1()?;
        self.tail_end.set(Some(end));
        Ok(end - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, block_len, acting_version);
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
pub struct FuelFiguresEntryDecoderComplete<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
impl<'a> FuelFiguresEntryDecoderComplete<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
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
        let offset = self.pos + self.acting_block_length;
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
            pos: self.pos,
            tail_start: data_end,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
    /// Non-consuming variant: read this var-data field as `&[u8]`
    /// without advancing or constructing the next stage. Cheaper
    /// than [`Self::#into_ident`] when only the bytes are needed.
    #[inline]
    pub fn usage_description_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.pos + self.acting_block_length;
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
    /// Consume this stage, read the next text var-data field as
    /// a validated `&str`, and advance to the following stage.
    #[inline]
    pub fn into_usage_description_as_str(
        self,
    ) -> Result<(&'a str, FuelFiguresEntryDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_usage_description()?;
        let s = core::str::from_utf8(bytes)
            .map_err(|e| {
                sbe_rt::DecodeError::InvalidUtf8 {
                    field: "usageDescription",
                    error: e,
                }
            })?;
        Ok((s, next))
    }
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    /// Consume this stage, read the next text var-data field as
    /// a `&str` without UTF-8 validation, and advance to the
    /// following stage.
    ///
    /// # Safety
    ///
    /// The wire bytes must be valid UTF-8. For schema-declared
    /// ASCII encoding this is always true (ASCII ⊂ UTF-8).
    #[inline]
    pub unsafe fn into_usage_description_as_str_unchecked(
        self,
    ) -> (&'a str, FuelFiguresEntryDecoderComplete<'a>) {
        let (bytes, next) = unsafe { self.into_usage_description().unwrap_unchecked() };
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        (s, next)
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
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
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
    #[inline]
    pub fn as_body_bytes(&self) -> &'a [u8] {
        &self.buf[self.pos..self.tail_start]
    }
    /// Complete SBE frame (header + body) for message stages.
    /// For entry stages (`HEADER_LENGTH == 0`) this equals [`Self::as_body_bytes`].
    #[inline]
    pub fn as_bytes_with_header(&self) -> &'a [u8] {
        &self.buf[self.pos - 0..self.tail_start]
    }
    /// Body length (excluding header).
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.tail_start - self.pos
    }
    /// Total message length including the schema-declared header.
    /// Pure arithmetic: body length + `HEADER_LENGTH`.
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.tail_start - self.pos + 0
    }
    /// Bytes after this message/entry.
    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.tail_start..]
    }
}
pub struct PerformanceFiguresDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
    acting_block_length: usize,
    parent_pos: usize,
    parent_block_length: usize,
}
impl<'a> PerformanceFiguresDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Self::wrap_with_parent(buf, pos, acting_version, 0, 0)
    }
    /// Like `wrap()` but remembers the parent message body position and
    /// acting block length so `finish()` can rebuild the next stage.
    #[inline]
    pub fn wrap_with_parent(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if 4 > buf.len().saturating_sub(pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_length = header.block_length() as usize;
        let entries_start = pos + 4;
        Ok(Self {
            buf,
            pos: entries_start,
            count,
            start: entries_start,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}
impl<'a> PerformanceFiguresDecoder<'a> {
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.count
    }
    /// Dimension wrap (trusted position): the caller has
    /// proven `pos` is within a validated extent.
    #[inline]
    pub fn wrap_trusted(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Self {
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_length = header.block_length() as usize;
        Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
        }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        self.pos = self.start;
        self.count = self.total;
        self
    }
}
impl<'a> PerformanceFiguresDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: n.saturating_mul(Self::ENTRY_BLOCK_LENGTH),
                available: self.count.saturating_mul(Self::ENTRY_BLOCK_LENGTH),
            });
        }
        for _ in 0..n {
            let entry = PerformanceFiguresEntryDecoder::wrap(
                self.buf,
                self.pos,
                self.acting_block_length,
                self.acting_version,
            );
            self.pos += entry.encoded_length()?;
            self.count -= 1;
        }
        Ok(())
    }
}
impl<'a> PerformanceFiguresDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<PerformanceFiguresEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: idx.saturating_add(1).saturating_mul(Self::ENTRY_BLOCK_LENGTH),
                available: self.total.saturating_mul(Self::ENTRY_BLOCK_LENGTH),
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
        let entry = PerformanceFiguresEntryDecoder::wrap(
            self.buf,
            offset,
            self.acting_block_length,
            self.acting_version,
        );
        entry.encoded_length()?;
        Ok(entry)
    }
}
impl<'a> Iterator for PerformanceFiguresDecoder<'a> {
    type Item = Result<PerformanceFiguresEntryDecoder<'a>, sbe_rt::DecodeError>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = PerformanceFiguresEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_block_length,
            self.acting_version,
        );
        let size = match entry.encoded_length() {
            Ok(s) => s,
            Err(e) => {
                self.count = 0;
                return Some(Err(e));
            }
        };
        self.pos += size;
        self.count -= 1;
        Some(Ok(entry))
    }
}
impl<'a> ExactSizeIterator for PerformanceFiguresDecoder<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.count
    }
}
pub struct PerformanceFiguresEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
    /// One-shot entry-extent cache: filled by
    /// `encoded_length`, reused by the last var-data accessor.
    tail_end: core::cell::Cell<Option<usize>>,
}
impl<'a> PerformanceFiguresEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        Self {
            buf,
            pos,
            acting_version,
            acting_block_length,
            tail_end: core::cell::Cell::new(None),
        }
    }
    #[inline]
    pub fn octane_rating(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes_unchecked::<1>(self.buf, offset))
    }
    pub const OCTANE_RATING_ID: u16 = 14;
    pub const OCTANE_RATING_SINCE_VERSION: u16 = 0;
    pub const OCTANE_RATING_ENCODING_OFFSET: usize = 0;
    pub const OCTANE_RATING_ENCODING_LENGTH: usize = 1;
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
    pub const OCTANE_RATING_NULL: u8 = 255_u8;
    pub const OCTANE_RATING_MIN: u8 = 90_u8;
    pub const OCTANE_RATING_MAX: u8 = 110_u8;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        if self.acting_block_length > self.buf.len().saturating_sub(self.pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: self.acting_block_length,
                available: self.buf.len().saturating_sub(self.pos),
            });
        }
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: 4,
                available: self.buf.len().saturating_sub(start),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = PerformanceFiguresAccelerationEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            idx += 1;
        }
        Ok(pos)
    }
    #[inline]
    pub fn acceleration(
        &self,
    ) -> Result<PerformanceFiguresAccelerationDecoder<'a>, sbe_rt::DecodeError> {
        if self.tail_end.get().is_some() {
            let offset = self.pos + self.acting_block_length;
            return Ok(
                PerformanceFiguresAccelerationDecoder::wrap_trusted(
                    self.buf,
                    offset,
                    self.acting_version,
                    0,
                    0,
                ),
            );
        }
        let offset = self.tail_offset_0()?;
        if self.tail_end.get().is_some() {
            return Ok(
                PerformanceFiguresAccelerationDecoder::wrap_trusted(
                    self.buf,
                    offset,
                    self.acting_version,
                    0,
                    0,
                ),
            );
        }
        PerformanceFiguresAccelerationDecoder::wrap(
            self.buf,
            offset,
            self.acting_version,
        )
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        if let Some(end) = self.tail_end.get() {
            return Ok(end - self.pos);
        }
        let end = self.tail_offset_1()?;
        self.tail_end.set(Some(end));
        Ok(end - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, block_len, acting_version);
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
pub struct PerformanceFiguresAccelerationDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
    acting_block_length: usize,
    parent_pos: usize,
    parent_block_length: usize,
}
impl<'a> PerformanceFiguresAccelerationDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Self::wrap_with_parent(buf, pos, acting_version, 0, 0)
    }
    /// Like `wrap()` but remembers the parent message body position and
    /// acting block length so `finish()` can rebuild the next stage.
    #[inline]
    pub fn wrap_with_parent(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if 4 > buf.len().saturating_sub(pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_length = header.block_length() as usize;
        let entries_start = pos + 4;
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
        Ok(Self {
            buf,
            pos: entries_start,
            count,
            start: entries_start,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
}
impl<'a> PerformanceFiguresAccelerationDecoder<'a> {
    #[inline]
    pub const fn remaining(&self) -> usize {
        self.count
    }
    /// Dimension wrap (trusted position): the caller has
    /// proven `pos` is within a validated extent.
    #[inline]
    pub fn wrap_trusted(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
        parent_pos: usize,
        parent_block_length: usize,
    ) -> Self {
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_length = header.block_length() as usize;
        Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
            acting_block_length: block_length,
            parent_pos,
            parent_block_length,
        }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        self.pos = self.start;
        self.count = self.total;
        self
    }
}
impl<'a> PerformanceFiguresAccelerationDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: n.saturating_mul(self.acting_block_length),
                available: self.count.saturating_mul(self.acting_block_length),
            });
        }
        self.pos += n.saturating_mul(self.acting_block_length);
        self.count -= n;
        Ok(())
    }
    /// Bulk-decode all remaining entries into a `Vec`.
    /// One bounds check for the whole batch — faster than
    /// iterating with [`Iterator::next`] when materialising
    /// the entire group (DTO construction, snapshots).
    pub fn bulk_decode(
        &mut self,
    ) -> Result<Vec<PerformanceFiguresAccelerationEntry>, sbe_rt::DecodeError> {
        let needed = self
            .count
            .checked_mul(self.acting_block_length)
            .ok_or(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: usize::MAX,
                available: 0,
            })?;
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed,
                available: self.buf.len().saturating_sub(self.pos),
            });
        }
        let cap = self.count;
        let mut out = Vec::with_capacity(cap);
        for _ in 0..cap {
            let pos = self.pos;
            self.pos += self.acting_block_length;
            out.push(PerformanceFiguresAccelerationEntry {
                mph: u16::from_le_bytes(
                    self.buf[pos + 0..pos + 0 + 2].try_into().unwrap(),
                ),
                seconds: f32::from_le_bytes(
                    self.buf[pos + 2..pos + 2 + 4].try_into().unwrap(),
                ),
            });
        }
        self.count = 0;
        Ok(out)
    }
}
impl<'a> PerformanceFiguresAccelerationDecoder<'a> {
    #[inline]
    pub fn nth(
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
        Ok(
            PerformanceFiguresAccelerationEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for PerformanceFiguresAccelerationDecoder<'a> {
    type Item = PerformanceFiguresAccelerationEntryDecoder<'a>;
    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = PerformanceFiguresAccelerationEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_block_length,
            self.acting_version,
        );
        self.pos += self.acting_block_length;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for PerformanceFiguresAccelerationDecoder<'a> {
    #[inline]
    fn len(&self) -> usize {
        self.count
    }
}
pub struct PerformanceFiguresAccelerationEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> PerformanceFiguresAccelerationEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_block_length: usize,
        acting_version: u16,
    ) -> Self {
        Self {
            buf,
            pos,
            acting_version,
            acting_block_length,
        }
    }
    #[inline]
    pub fn mph(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes_unchecked::<2>(self.buf, offset))
    }
    pub const MPH_ID: u16 = 16;
    pub const MPH_SINCE_VERSION: u16 = 0;
    pub const MPH_ENCODING_OFFSET: usize = 0;
    pub const MPH_ENCODING_LENGTH: usize = 2;
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
    pub const MPH_NULL: u16 = 65535_u16;
    pub const MPH_MIN: u16 = 0_u16;
    pub const MPH_MAX: u16 = 65534_u16;
    #[inline]
    pub fn seconds(&self) -> f32 {
        let offset = self.pos + 2;
        f32::from_le_bytes(read_bytes_unchecked::<4>(self.buf, offset))
    }
    pub const SECONDS_ID: u16 = 17;
    pub const SECONDS_SINCE_VERSION: u16 = 0;
    pub const SECONDS_ENCODING_OFFSET: usize = 2;
    pub const SECONDS_ENCODING_LENGTH: usize = 4;
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
    pub const SECONDS_NULL: f32 = f32::from_bits(2139095041u32);
    pub const SECONDS_MIN: f32 = f32::from_bits(4286578687u32);
    pub const SECONDS_MAX: f32 = f32::from_bits(2139095039u32);
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        if self.acting_block_length > self.buf.len().saturating_sub(self.pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: self.acting_block_length,
                available: self.buf.len().saturating_sub(self.pos),
            });
        }
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.acting_block_length
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        _acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        if block_len > buf.len().saturating_sub(pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "group entry",
                needed: block_len,
                available: buf.len().saturating_sub(pos),
            });
        }
        Ok(pos + block_len)
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
pub struct PerformanceFiguresEntryDecoderComplete<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
impl<'a> PerformanceFiguresEntryDecoderComplete<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
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
    ) -> Result<PerformanceFiguresAccelerationDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.pos + self.acting_block_length;
        PerformanceFiguresAccelerationDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> PerformanceFiguresAccelerationDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<PerformanceFiguresEntryDecoderComplete<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = PerformanceFiguresAccelerationEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(PerformanceFiguresEntryDecoderComplete {
            buf: self.buf,
            pos: self.parent_pos,
            tail_start: pos,
            acting_version: self.acting_version,
            acting_block_length: self.parent_block_length,
        })
    }
    /// Explicit sequential spelling of "advance past the rest of this group".
    #[inline]
    pub fn skip_remaining(
        self,
    ) -> Result<PerformanceFiguresEntryDecoderComplete<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> PerformanceFiguresEntryDecoderComplete<'a> {
    /// Body bytes (excluding the message header; for entries this is the
    /// complete entry bytes).
    #[inline]
    pub fn as_body_bytes(&self) -> &'a [u8] {
        &self.buf[self.pos..self.tail_start]
    }
    /// Complete SBE frame (header + body) for message stages.
    /// For entry stages (`HEADER_LENGTH == 0`) this equals [`Self::as_body_bytes`].
    #[inline]
    pub fn as_bytes_with_header(&self) -> &'a [u8] {
        &self.buf[self.pos - 0..self.tail_start]
    }
    /// Body length (excluding header).
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.tail_start - self.pos
    }
    /// Total message length including the schema-declared header.
    /// Pure arithmetic: body length + `HEADER_LENGTH`.
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.tail_start - self.pos + 0
    }
    /// Bytes after this message/entry.
    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.tail_start..]
    }
}
pub struct CarDecoderAfterFuelFigures<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
pub struct CarDecoderAfterPerformanceFigures<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
pub struct CarDecoderAfterManufacturer<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
pub struct CarDecoderAfterModel<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
pub struct CarDecoderComplete<'a> {
    pub(crate) buf: &'a [u8],
    pub(crate) pos: usize,
    pub(crate) tail_start: usize,
    pub(crate) acting_version: u16,
    pub(crate) acting_block_length: usize,
}
impl<'a> CarDecoderAfterFuelFigures<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> CarDecoderAfterPerformanceFigures<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> CarDecoderAfterManufacturer<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> CarDecoderAfterModel<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> CarDecoderComplete<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
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
    ) -> Result<FuelFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.pos + self.acting_block_length;
        FuelFiguresDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> CarDecoderAfterFuelFigures<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_performance_figures(
        self,
    ) -> Result<PerformanceFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        PerformanceFiguresDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
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
            pos: self.pos,
            tail_start: data_end,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
    /// Non-consuming variant: read this var-data field as `&[u8]`
    /// without advancing or constructing the next stage. Cheaper
    /// than [`Self::#into_ident`] when only the bytes are needed.
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
    /// Consume this stage, read the next text var-data field as
    /// a validated `&str`, and advance to the following stage.
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
    /// a `&str` without UTF-8 validation, and advance to the
    /// following stage.
    ///
    /// # Safety
    ///
    /// The wire bytes must be valid UTF-8. For schema-declared
    /// ASCII encoding this is always true (ASCII ⊂ UTF-8).
    #[inline]
    pub unsafe fn into_manufacturer_as_str_unchecked(
        self,
    ) -> (&'a str, CarDecoderAfterManufacturer<'a>) {
        let (bytes, next) = unsafe { self.into_manufacturer().unwrap_unchecked() };
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        (s, next)
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
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
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
            pos: self.pos,
            tail_start: data_end,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
    /// Non-consuming variant: read this var-data field as `&[u8]`
    /// without advancing or constructing the next stage. Cheaper
    /// than [`Self::#into_ident`] when only the bytes are needed.
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
    /// Consume this stage, read the next text var-data field as
    /// a validated `&str`, and advance to the following stage.
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
    /// a `&str` without UTF-8 validation, and advance to the
    /// following stage.
    ///
    /// # Safety
    ///
    /// The wire bytes must be valid UTF-8. For schema-declared
    /// ASCII encoding this is always true (ASCII ⊂ UTF-8).
    #[inline]
    pub unsafe fn into_model_as_str_unchecked(
        self,
    ) -> (&'a str, CarDecoderAfterModel<'a>) {
        let (bytes, next) = unsafe { self.into_model().unwrap_unchecked() };
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        (s, next)
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
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
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
            pos: self.pos,
            tail_start: data_end,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
    /// Non-consuming variant: read this var-data field as `&[u8]`
    /// without advancing or constructing the next stage. Cheaper
    /// than [`Self::#into_ident`] when only the bytes are needed.
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
    /// Consume this stage, read the next text var-data field as
    /// a validated `&str`, and advance to the following stage.
    #[inline]
    pub fn into_activation_code_as_str(
        self,
    ) -> Result<(&'a str, CarDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (bytes, next) = self.into_activation_code()?;
        let s = core::str::from_utf8(bytes)
            .map_err(|e| {
                sbe_rt::DecodeError::InvalidUtf8 {
                    field: "activationCode",
                    error: e,
                }
            })?;
        Ok((s, next))
    }
}
impl<'a> CarDecoderAfterModel<'a> {
    /// Consume this stage, read the next text var-data field as
    /// a `&str` without UTF-8 validation, and advance to the
    /// following stage.
    ///
    /// # Safety
    ///
    /// The wire bytes must be valid UTF-8. For schema-declared
    /// ASCII encoding this is always true (ASCII ⊂ UTF-8).
    #[inline]
    pub unsafe fn into_activation_code_as_str_unchecked(
        self,
    ) -> (&'a str, CarDecoderComplete<'a>) {
        let (bytes, next) = unsafe { self.into_activation_code().unwrap_unchecked() };
        let s = unsafe { core::str::from_utf8_unchecked(bytes) };
        (s, next)
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
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
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
impl<'a> FuelFiguresDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(self) -> Result<CarDecoderAfterFuelFigures<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = FuelFiguresEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(CarDecoderAfterFuelFigures {
            buf: self.buf,
            pos: self.parent_pos,
            tail_start: pos,
            acting_version: self.acting_version,
            acting_block_length: self.parent_block_length,
        })
    }
    /// Explicit sequential spelling of "advance past the rest of this group".
    #[inline]
    pub fn skip_remaining(
        self,
    ) -> Result<CarDecoderAfterFuelFigures<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> PerformanceFiguresDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<CarDecoderAfterPerformanceFigures<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = PerformanceFiguresEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(CarDecoderAfterPerformanceFigures {
            buf: self.buf,
            pos: self.parent_pos,
            tail_start: pos,
            acting_version: self.acting_version,
            acting_block_length: self.parent_block_length,
        })
    }
    /// Explicit sequential spelling of "advance past the rest of this group".
    #[inline]
    pub fn skip_remaining(
        self,
    ) -> Result<CarDecoderAfterPerformanceFigures<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> CarDecoderComplete<'a> {
    /// Body bytes (excluding the message header; for entries this is the
    /// complete entry bytes).
    #[inline]
    pub fn as_body_bytes(&self) -> &'a [u8] {
        &self.buf[self.pos..self.tail_start]
    }
    /// Complete SBE frame (header + body) for message stages.
    /// For entry stages (`HEADER_LENGTH == 0`) this equals [`Self::as_body_bytes`].
    #[inline]
    pub fn as_bytes_with_header(&self) -> &'a [u8] {
        &self.buf[self.pos - 8..self.tail_start]
    }
    /// Body length (excluding header).
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.tail_start - self.pos
    }
    /// Total message length including the schema-declared header.
    /// Pure arithmetic: body length + `HEADER_LENGTH`.
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.tail_start - self.pos + 8
    }
    /// Bytes after this message/entry.
    #[inline]
    pub fn remaining(&self) -> &'a [u8] {
        &self.buf[self.tail_start..]
    }
}
/// Owned domain object — application-layer counterpart to the flyweight decoder.
/// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
#[derive(Debug, Clone, PartialEq)]
pub struct CarFuelFiguresEntryDomain {
    pub speed: u16,
    pub mpg: f32,
    pub usage_description: Vec<u8>,
}
impl CarFuelFiguresEntryDomain {
    /// Fallible conversion from a decoder. Propagates decode errors
    /// from malformed group entries instead of silently dropping them.
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
impl<'a> From<FuelFiguresEntryDecoder<'a>> for CarFuelFiguresEntryDomain {
    fn from(dec: FuelFiguresEntryDecoder<'a>) -> Self {
        Self::try_from_decoder(dec)
            .expect(
                "domain conversion failed — use try_from_decoder for fallible conversion",
            )
    }
}
impl CarFuelFiguresEntryDomain {
    pub fn encode_into<'a>(
        &self,
        enc: &mut FuelFiguresEntryEncoder<'a>,
    ) -> Result<(), sbe_rt::EncodeError> {
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
        Ok(())
    }
    /// Compute this entry's contribution to the total encoded length
    /// (entry block + nested groups + entry var-data).
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
/// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
#[derive(Debug, Clone, PartialEq)]
pub struct CarPerformanceFiguresEntryAccelerationEntryDomain {
    pub mph: u16,
    pub seconds: f32,
}
impl CarPerformanceFiguresEntryAccelerationEntryDomain {
    /// Fallible conversion from a decoder. Propagates decode errors
    /// from malformed group entries instead of silently dropping them.
    pub fn try_from_decoder(
        dec: PerformanceFiguresAccelerationEntryDecoder<'_>,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Ok(Self {
            mph: dec.mph(),
            seconds: dec.seconds(),
        })
    }
}
impl<'a> From<PerformanceFiguresAccelerationEntryDecoder<'a>>
for CarPerformanceFiguresEntryAccelerationEntryDomain {
    fn from(dec: PerformanceFiguresAccelerationEntryDecoder<'a>) -> Self {
        Self::try_from_decoder(dec)
            .expect(
                "domain conversion failed — use try_from_decoder for fallible conversion",
            )
    }
}
impl CarPerformanceFiguresEntryAccelerationEntryDomain {
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
    pub fn length_contribution(&self) -> Result<usize, sbe_rt::EncodeError> {
        let mut len: usize = 6;
        Ok(len)
    }
}
impl CarPerformanceFiguresEntryAccelerationEntryDomain {
    /// Convert to the wire entry struct for bulk encoding.
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
/// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
#[derive(Debug, Clone, PartialEq)]
pub struct CarPerformanceFiguresEntryDomain {
    pub octane_rating: u8,
    pub acceleration: Vec<CarPerformanceFiguresEntryAccelerationEntryDomain>,
}
impl CarPerformanceFiguresEntryDomain {
    /// Fallible conversion from a decoder. Propagates decode errors
    /// from malformed group entries instead of silently dropping them.
    pub fn try_from_decoder(
        dec: PerformanceFiguresEntryDecoder<'_>,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Ok(Self {
            octane_rating: dec.octane_rating(),
            acceleration: dec
                .acceleration()
                .map(|g| Ok(
                    g
                        .map(CarPerformanceFiguresEntryAccelerationEntryDomain::from)
                        .collect(),
                ))
                .unwrap_or_else(|e| Err(e))?,
        })
    }
}
impl<'a> From<PerformanceFiguresEntryDecoder<'a>> for CarPerformanceFiguresEntryDomain {
    fn from(dec: PerformanceFiguresEntryDecoder<'a>) -> Self {
        Self::try_from_decoder(dec)
            .expect(
                "domain conversion failed — use try_from_decoder for fallible conversion",
            )
    }
}
impl CarPerformanceFiguresEntryDomain {
    pub fn encode_into<'a>(
        &self,
        enc: &mut PerformanceFiguresEntryEncoder<'a>,
    ) -> Result<(), sbe_rt::EncodeError> {
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
        let enc = enc
            .acceleration(
                self.acceleration.len() as u16,
                |g| g.bulk_add_domain(&self.acceleration),
            )?;
        Ok(())
    }
    /// Compute this entry's contribution to the total encoded length
    /// (entry block + nested groups + entry var-data).
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
/// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
#[derive(Debug, Clone, PartialEq)]
pub struct CarDomain {
    pub serial_number: u64,
    pub model_year: u16,
    pub available: bool,
    pub code: Model,
    pub some_numbers: [u32; 4],
    pub vehicle_code: [u8; 6],
    pub extras: OptionalExtras,
    pub engine: Engine,
    pub fuel_figures: Vec<CarFuelFiguresEntryDomain>,
    pub performance_figures: Vec<CarPerformanceFiguresEntryDomain>,
    pub manufacturer: Vec<u8>,
    pub model: Vec<u8>,
    pub activation_code: Vec<u8>,
}
impl CarDomain {
    /// Fallible conversion from a decoder. Propagates decode errors
    /// from malformed group entries instead of silently dropping them.
    pub fn try_from_decoder(dec: CarDecoder<'_>) -> Result<Self, sbe_rt::DecodeError> {
        Ok(Self {
            serial_number: dec.serial_number(),
            model_year: dec.model_year(),
            available: dec.available_bool(),
            code: dec.code(),
            some_numbers: dec.some_numbers(),
            vehicle_code: dec.vehicle_code(),
            extras: dec.extras(),
            engine: dec.engine_value(),
            fuel_figures: dec
                .fuel_figures()
                .map(|g| {
                    g.map(|r| r.map(CarFuelFiguresEntryDomain::from))
                        .collect::<Result<Vec<_>, _>>()
                })
                .unwrap_or_else(|e| Err(e))?,
            performance_figures: dec
                .performance_figures()
                .map(|g| {
                    g.map(|r| r.map(CarPerformanceFiguresEntryDomain::from))
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
    /// Decode directly from a byte slice — validates the header
    /// and materialises the full domain object in one call.
    pub fn try_from_slice_with_header(
        buf: &[u8],
        message_offset: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        Self::try_from_decoder(
            CarDecoder::try_wrap_and_apply_header(buf, message_offset)?,
        )
    }
}
impl<'a> From<CarDecoder<'a>> for CarDomain {
    fn from(dec: CarDecoder<'a>) -> Self {
        Self::try_from_decoder(dec)
            .expect(
                "domain conversion failed — use try_from_decoder for fallible conversion",
            )
    }
}
impl CarDomain {
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
        let enc = enc
            .fuel_figures(
                self.fuel_figures.len() as u16,
                |g| -> Result<(), sbe_rt::EncodeError> {
                    for e in &self.fuel_figures {
                        g.add(|entry| -> Result<(), sbe_rt::EncodeError> {
                            e.encode_into(entry)
                        })?;
                    }
                    Ok(())
                },
            )?;
        let enc = enc
            .performance_figures(
                self.performance_figures.len() as u16,
                |g| -> Result<(), sbe_rt::EncodeError> {
                    for e in &self.performance_figures {
                        g.add(|entry| -> Result<(), sbe_rt::EncodeError> {
                            e.encode_into(entry)
                        })?;
                    }
                    Ok(())
                },
            )?;
        let enc = enc.manufacturer(&self.manufacturer)?;
        let enc = enc.model(&self.model)?;
        let enc = enc.activation_code(&self.activation_code)?;
        Ok(enc.encoded_length() + CarEncoder::HEADER_LENGTH)
    }
    /// Compute the exact SBE message body length from this domain object.
    /// Matches the length returned by [`Self::encode`].
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
    /// Compute the exact SBE message length including the message header.
    /// Matches `encode()` return value for non-fixed messages.
    pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::EncodeError> {
        Ok(self.encoded_length()? + CarEncoder::HEADER_LENGTH)
    }
}
///Description of a basic Car
#[must_use = "encoder must be consumed to write the message"]
pub struct CarEncoder<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    pos: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for CarEncoder<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarEncoder"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarEncoder<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarEncoder")
                    .field("msg_offset", &self.msg_offset)
                    .field("pos", &self.pos)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterFuelFigures<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    pos: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for CarAfterFuelFigures<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarAfterFuelFigures"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarAfterFuelFigures<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarAfterFuelFigures")
                    .field("msg_offset", &self.msg_offset)
                    .field("pos", &self.pos)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterPerformanceFigures<
    'a,
    H: sbe_rt::HeaderState = sbe_rt::HeaderPresent,
> {
    buf: &'a mut [u8],
    msg_offset: usize,
    pos: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display
for CarAfterPerformanceFigures<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarAfterPerformanceFigures"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarAfterPerformanceFigures<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarAfterPerformanceFigures")
                    .field("msg_offset", &self.msg_offset)
                    .field("pos", &self.pos)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterManufacturer<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    pos: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for CarAfterManufacturer<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarAfterManufacturer"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarAfterManufacturer<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarAfterManufacturer")
                    .field("msg_offset", &self.msg_offset)
                    .field("pos", &self.pos)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterModel<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    pos: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for CarAfterModel<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarAfterModel"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarAfterModel<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarAfterModel")
                    .field("msg_offset", &self.msg_offset)
                    .field("pos", &self.pos)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarComplete<'a, H: sbe_rt::HeaderState = sbe_rt::HeaderPresent> {
    buf: &'a mut [u8],
    msg_offset: usize,
    pos: usize,
    _header: core::marker::PhantomData<H>,
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Display for CarComplete<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Display::fmt(&dec, f),
            Err(_) => write!(f, "<partial {}>", "CarComplete"),
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> core::fmt::Debug for CarComplete<'a, H> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match CarDecoder::try_wrap_and_apply_header(self.buf, self.msg_offset) {
            Ok(dec) => core::fmt::Debug::fmt(&dec, f),
            Err(_) => {
                f.debug_struct("CarComplete")
                    .field("msg_offset", &self.msg_offset)
                    .field("pos", &self.pos)
                    .field("buf_len", &self.buf.len())
                    .finish()
            }
        }
    }
}
/// Complete set of latest-version fixed fields for this message.
/// Required fields are concrete values; optional/versioned fields
/// are `Option<T>`. Constants are excluded.
#[derive(Debug, Clone)]
pub struct CarFixedFields {
    pub serial_number: u64,
    pub model_year: u16,
    pub available: BooleanType,
    pub code: Model,
    pub some_numbers: [u32; 4],
    pub vehicle_code: [u8; 6],
    pub extras: OptionalExtras,
    pub engine: Engine,
}
/// Raw fixed-field writer. Individual field setters are available
/// only on this writer. When done, embed the fields in a
/// `#fixed_name` and call the encoder's `fixed()`.
#[must_use = "raw fixed writer must be embedded in FixedFields"]
pub struct CarRawFixedWriter<'a> {
    buf: &'a mut [u8],
    msg_offset: usize,
    pos: usize,
}
impl<'a> CarEncoder<'a> {
    pub const SCHEMA_ID: u16 = 1;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 45;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 45);
    /// Schema-declared message header size in bytes.
    pub const HEADER_LENGTH: usize = 8;
    ///MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [45, 0, 1, 0, 1, 0, 0, 0];
    const _HEADER_TEMPLATE_LEN: () = assert!(Self::HEADER_TEMPLATE.len() == 8);
    #[inline]
    pub const fn compute_length() -> CarEncodedLength {
        CarEncodedLength::new()
    }
    /// Cold error constructor — never inlined into the hot path.
    #[cold]
    #[inline(never)]
    fn buffer_too_short(buf: &[u8], pos: usize, needed: usize) -> sbe_rt::EncodeError {
        sbe_rt::EncodeError::BufferTooShort {
            needed,
            available: buf.len().saturating_sub(pos),
        }
    }
    /// Wrap a mutable buffer for encoding with bounds validation.
    /// Does **not** write the message header (`HeaderAbsent`).
    /// Prefer [`Self::wrap`] for the fast path when the buffer size is known.
    /// Prefer [`Self::wrap_and_apply_header`] when encoding a full frame.
    #[inline]
    pub fn try_wrap(
        buf: &'a mut [u8],
        msg_offset: usize,
    ) -> Result<CarEncoder<'a, sbe_rt::HeaderAbsent>, sbe_rt::EncodeError> {
        if 53 > buf.len().saturating_sub(msg_offset) {
            return Err(Self::buffer_too_short(buf, msg_offset, 53));
        }
        let body_pos = msg_offset + 8;
        Ok(CarEncoder {
            buf,
            msg_offset,
            pos: body_pos + 45,
            _header: core::marker::PhantomData,
        })
    }
    /// Wrap a mutable buffer, write the header, with bounds validation.
    /// Returns an error if the buffer is too short.
    /// Prefer [`Self::wrap_and_apply_header`] for the fast path.
    #[inline]
    pub fn try_wrap_and_apply_header(
        buf: &'a mut [u8],
        pos: usize,
    ) -> Result<CarEncoder<'a, sbe_rt::HeaderPresent>, sbe_rt::EncodeError> {
        if 53 > buf.len().saturating_sub(pos) {
            return Err(Self::buffer_too_short(buf, pos, 53));
        }
        buf[pos..pos + 8].copy_from_slice(&Self::HEADER_TEMPLATE);
        let body_pos = pos + 8;
        Ok(CarEncoder {
            buf,
            msg_offset: pos,
            pos: body_pos + 45,
            _header: core::marker::PhantomData,
        })
    }
    pub const SERIAL_NUMBER_ID: u16 = 1;
    pub const SERIAL_NUMBER_SINCE_VERSION: u16 = 0;
    pub const SERIAL_NUMBER_ENCODING_OFFSET: usize = 0;
    pub const SERIAL_NUMBER_ENCODING_LENGTH: usize = 8;
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
    pub const SERIAL_NUMBER_NULL: u64 = 18446744073709551615_u64;
    pub const SERIAL_NUMBER_MIN: u64 = 0_u64;
    pub const SERIAL_NUMBER_MAX: u64 = 18446744073709551614_u64;
    pub const MODEL_YEAR_ID: u16 = 2;
    pub const MODEL_YEAR_SINCE_VERSION: u16 = 0;
    pub const MODEL_YEAR_ENCODING_OFFSET: usize = 8;
    pub const MODEL_YEAR_ENCODING_LENGTH: usize = 2;
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
    pub const MODEL_YEAR_NULL: u16 = 65535_u16;
    pub const MODEL_YEAR_MIN: u16 = 0_u16;
    pub const MODEL_YEAR_MAX: u16 = 65534_u16;
    pub const AVAILABLE_ID: u16 = 3;
    pub const AVAILABLE_SINCE_VERSION: u16 = 0;
    pub const AVAILABLE_ENCODING_OFFSET: usize = 10;
    pub const AVAILABLE_ENCODING_LENGTH: usize = 1;
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
    pub const AVAILABLE_NULL: BooleanType = BooleanType::NullVal;
    pub const CODE_ID: u16 = 4;
    pub const CODE_SINCE_VERSION: u16 = 0;
    pub const CODE_ENCODING_OFFSET: usize = 11;
    pub const CODE_ENCODING_LENGTH: usize = 1;
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
    pub const CODE_NULL: Model = Model::NullVal;
    pub const SOME_NUMBERS_ID: u16 = 5;
    pub const SOME_NUMBERS_SINCE_VERSION: u16 = 0;
    pub const SOME_NUMBERS_ENCODING_OFFSET: usize = 12;
    pub const SOME_NUMBERS_ENCODING_LENGTH: usize = 16;
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
    pub const SOME_NUMBERS_NULL: u32 = 4294967295_u32;
    pub const SOME_NUMBERS_MIN: u32 = 0_u32;
    pub const SOME_NUMBERS_MAX: u32 = 4294967294_u32;
    pub const VEHICLE_CODE_ID: u16 = 6;
    pub const VEHICLE_CODE_SINCE_VERSION: u16 = 0;
    pub const VEHICLE_CODE_ENCODING_OFFSET: usize = 28;
    pub const VEHICLE_CODE_ENCODING_LENGTH: usize = 6;
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
    pub const VEHICLE_CODE_NULL: u8 = 0_u8;
    pub const VEHICLE_CODE_MIN: u8 = 32_u8;
    pub const VEHICLE_CODE_MAX: u8 = 126_u8;
    pub const EXTRAS_ID: u16 = 7;
    pub const EXTRAS_SINCE_VERSION: u16 = 0;
    pub const EXTRAS_ENCODING_OFFSET: usize = 34;
    pub const EXTRAS_ENCODING_LENGTH: usize = 1;
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
    pub const ENGINE_ID: u16 = 9;
    pub const ENGINE_SINCE_VERSION: u16 = 0;
    pub const ENGINE_ENCODING_OFFSET: usize = 35;
    pub const ENGINE_ENCODING_LENGTH: usize = 10;
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
impl<'a, H: sbe_rt::HeaderState> CarEncoder<'a, H> {
    /// Absolute offset of this message within the original buffer
    /// (the `msg_offset` argument passed to `wrap`).
    #[inline]
    pub const fn message_offset(&self) -> usize {
        self.msg_offset
    }
    /// Absolute current write cursor within the original buffer.
    #[inline]
    pub const fn limit(&self) -> usize {
        self.pos
    }
    /// The complete original buffer this encoder wraps.
    #[inline]
    pub fn buffer(&self) -> &[u8] {
        self.buf
    }
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
    #[inline]
    pub fn available(&mut self, val: BooleanType) -> &mut Self {
        let offset = self.msg_offset + 18;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    #[inline]
    pub fn available_bool(&mut self, val: bool) -> &mut Self {
        self.buf[self.msg_offset + 18] = val as u8;
        self
    }
    #[inline]
    pub fn code(&mut self, val: Model) -> &mut Self {
        let offset = self.msg_offset + 19;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
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
    #[inline]
    pub fn put_some_numbers(&mut self, v0: u32, v1: u32, v2: u32, v3: u32) -> &mut Self {
        self.some_numbers([v0, v1, v2, v3])
    }
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
    #[inline]
    pub fn vehicle_code_str(
        &mut self,
        src: &str,
    ) -> Result<&mut Self, sbe_rt::EncodeError> {
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
    #[inline]
    pub fn extras(&mut self, val: OptionalExtras) -> &mut Self {
        let offset = self.msg_offset + 42;
        self.buf[offset..offset + 1].copy_from_slice(&val.0.to_le_bytes());
        self
    }
    #[inline]
    pub fn engine(&mut self, val: Engine) -> &mut Self {
        let offset = self.msg_offset + 43;
        self.buf[offset..offset + 10].copy_from_slice(&val.0);
        self
    }
    /// Set all fixed fields at once from a [`#fixed_name`] value.
    /// Required fields are always written; optional fields are
    /// written when `Some`. Returns the encoder for tail methods.
    #[inline]
    #[must_use]
    pub fn fixed(mut self, fixed: &CarFixedFields) -> Self {
        self.serial_number(fixed.serial_number);
        self.model_year(fixed.model_year);
        self.available(fixed.available);
        self.code(fixed.code);
        self.some_numbers(fixed.some_numbers);
        self.vehicle_code(fixed.vehicle_code);
        self.extras(fixed.extras);
        self.engine(fixed.engine);
        self
    }
    /// Return a dedicated raw fixed-field writer. All individual field
    /// setters are available on the writer. To advance to tail stages,
    /// collect the values into a `#fixed_name` and call `fixed()`.
    #[inline]
    #[must_use]
    pub fn raw_fixed(self) -> CarRawFixedWriter<'a> {
        let body_start = self.msg_offset + 8;
        CarRawFixedWriter {
            buf: &mut self.buf[body_start..],
            msg_offset: 0,
            pos: self.pos - body_start,
        }
    }
}
impl<'a, H: sbe_rt::HeaderState> CarEncoder<'a, H> {
    /// Encode this group with a known count up front. Closure may
    /// return `()` or `Result<(), E>` (via
    /// Closures return `GroupResult`; `?` just works. a
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
        if self.pos + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.pos),
                }
                    .into(),
            );
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&FuelFiguresEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = FuelFiguresEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group)?;
        let written = group.written();
        if written != count {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared: count as u32,
                actual: written as u32,
            });
        }
        Ok(CarAfterFuelFigures {
            buf: group.buf,
            msg_offset: self.msg_offset,
            pos: group.pos,
            _header: core::marker::PhantomData,
        })
    }
    /// Encode this group without knowing the count up front.
    /// The dimension header is written with a zero placeholder;
    /// after the closure returns, the actual entry count is
    /// back-patched into the header. No `GroupFull` check —
    /// overflow is the caller's responsibility.
    ///
    /// Prefer [`Self::#g_snake`] when the count is known at
    /// compile time or from a small input.
    #[inline]
    #[must_use]
    pub fn fuel_figures_unknown_size<F>(
        mut self,
        f: F,
    ) -> Result<CarAfterFuelFigures<'a, H>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut FuelFiguresEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.pos + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.pos),
                }
                    .into(),
            );
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&FuelFiguresEncoder::GROUP_DIM_TEMPLATE);
        let count_offset = self.pos + 2;
        self.buf[count_offset..count_offset + 2].fill(0);
        let (buf, pos, actual) = {
            let mut group = FuelFiguresEncoder::wrap(self.buf, self.pos + 4, u16::MAX);
            f(&mut group)?;
            let n = group.written();
            (group.buf, group.pos, n)
        };
        buf[count_offset..count_offset + 2].copy_from_slice(&actual.to_le_bytes());
        Ok(CarAfterFuelFigures {
            buf,
            msg_offset: self.msg_offset,
            pos,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarAfterFuelFigures<'a, H> {
    /// Encode this group with a known count up front. Closure may
    /// return `()` or `Result<(), E>` (via
    /// Closures return `GroupResult`; `?` just works. a
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
        if self.pos + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.pos),
                }
                    .into(),
            );
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&PerformanceFiguresEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = PerformanceFiguresEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group)?;
        let written = group.written();
        if written != count {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared: count as u32,
                actual: written as u32,
            });
        }
        Ok(CarAfterPerformanceFigures {
            buf: group.buf,
            msg_offset: self.msg_offset,
            pos: group.pos,
            _header: core::marker::PhantomData,
        })
    }
    /// Encode this group without knowing the count up front.
    /// The dimension header is written with a zero placeholder;
    /// after the closure returns, the actual entry count is
    /// back-patched into the header. No `GroupFull` check —
    /// overflow is the caller's responsibility.
    ///
    /// Prefer [`Self::#g_snake`] when the count is known at
    /// compile time or from a small input.
    #[inline]
    #[must_use]
    pub fn performance_figures_unknown_size<F>(
        mut self,
        f: F,
    ) -> Result<CarAfterPerformanceFigures<'a, H>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.pos + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.pos),
                }
                    .into(),
            );
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&PerformanceFiguresEncoder::GROUP_DIM_TEMPLATE);
        let count_offset = self.pos + 2;
        self.buf[count_offset..count_offset + 2].fill(0);
        let (buf, pos, actual) = {
            let mut group = PerformanceFiguresEncoder::wrap(
                self.buf,
                self.pos + 4,
                u16::MAX,
            );
            f(&mut group)?;
            let n = group.written();
            (group.buf, group.pos, n)
        };
        buf[count_offset..count_offset + 2].copy_from_slice(&actual.to_le_bytes());
        Ok(CarAfterPerformanceFigures {
            buf,
            msg_offset: self.msg_offset,
            pos,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarAfterPerformanceFigures<'a, H> {
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
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterManufacturer {
            buf: self.buf,
            msg_offset: self.msg_offset,
            pos: start + data.len(),
            _header: core::marker::PhantomData,
        })
    }
    #[inline]
    #[must_use]
    pub fn manufacturer_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterManufacturer<'a, H>, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterManufacturer {
            buf: self.buf,
            msg_offset: self.msg_offset,
            pos: start + data.len(),
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
    ///     let len = InnerEncoder::try_wrap_and_apply_header(payload, 0)?
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
        if self.pos + needed > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed,
                    available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(CarAfterManufacturer {
            buf: self.buf,
            msg_offset: self.msg_offset,
            pos: start + exact_len,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarAfterManufacturer<'a, H> {
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
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterModel {
            buf: self.buf,
            msg_offset: self.msg_offset,
            pos: start + data.len(),
            _header: core::marker::PhantomData,
        })
    }
    #[inline]
    #[must_use]
    pub fn model_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterModel<'a, H>, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterModel {
            buf: self.buf,
            msg_offset: self.msg_offset,
            pos: start + data.len(),
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
    ///     let len = InnerEncoder::try_wrap_and_apply_header(payload, 0)?
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
        if self.pos + needed > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed,
                    available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(CarAfterModel {
            buf: self.buf,
            msg_offset: self.msg_offset,
            pos: start + exact_len,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarAfterModel<'a, H> {
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
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarComplete {
            buf: self.buf,
            msg_offset: self.msg_offset,
            pos: start + data.len(),
            _header: core::marker::PhantomData,
        })
    }
    #[inline]
    #[must_use]
    pub fn activation_code_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarComplete<'a, H>, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarComplete {
            buf: self.buf,
            msg_offset: self.msg_offset,
            pos: start + data.len(),
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
    ///     let len = InnerEncoder::try_wrap_and_apply_header(payload, 0)?
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
        if self.pos + needed > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed,
                    available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(CarComplete {
            buf: self.buf,
            msg_offset: self.msg_offset,
            pos: start + exact_len,
            _header: core::marker::PhantomData,
        })
    }
}
impl<'a, H: sbe_rt::HeaderState> CarComplete<'a, H> {
    /// SBE message body bytes (excluding the message header).
    #[inline]
    pub fn as_body_bytes(&self) -> &[u8] {
        let body_start = self.msg_offset + 8;
        &self.buf[body_start..self.pos]
    }
    /// SBE message body length (excluding the message header).
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.pos - self.msg_offset - 8
    }
    /// Total SBE message length including the header region.
    /// Pure arithmetic — available for body-only wraps too.
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.pos - self.msg_offset
    }
    /// Unwritten region after this message.
    #[inline]
    pub fn into_remaining_mut(self) -> &'a mut [u8] {
        &mut self.buf[self.pos..]
    }
}
impl<'a> CarComplete<'a, sbe_rt::HeaderPresent> {
    /// Header-inclusive bytes. Only available when the encoder was
    /// constructed via `wrap_and_apply_header` (not raw `wrap`).
    #[inline]
    pub fn as_bytes_with_header(&self) -> &[u8] {
        &self.buf[self.msg_offset..self.pos]
    }
}
impl<'a> sbe_rt::private::Sealed for CarEncoder<'a> {}
impl<'a> sbe_rt::SbeMessage for CarEncoder<'a> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 45;
    const SCHEMA_ID: u16 = 1;
    const SCHEMA_VERSION: u16 = 0;
}
#[must_use = "group encoder must call add() to write entries"]
pub struct FuelFiguresEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> FuelFiguresEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [6, 0, 0, 0];
    const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == 4);
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize, count: u16) -> Self {
        Self {
            buf,
            pos,
            count,
            written: 0,
        }
    }
    /// Write one group entry. Closure may return `()` or `Result<(), E>`
    /// ([`sbe_rt::GroupEncodeResult`]) so `?` works without `try_add`.
    #[inline]
    #[must_use]
    pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(&mut FuelFiguresEntryEncoder<'b>) -> sbe_rt::GroupResult,
    {
        if self.written >= self.count {
            return Err(
                sbe_rt::EncodeError::GroupFull {
                    declared: self.count as u32,
                    attempted: self.written as u32 + 1,
                }
                    .into(),
            );
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: block_len,
                    available: self.buf.len().saturating_sub(self.pos),
                }
                    .into(),
            );
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut __entry = FuelFiguresEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry)?;
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
    /// Manual entry creation: returns a borrowed entry encoder.
    /// The entry writes fixed fields directly into the group buffer.
    /// Drop the entry or let it go out of scope to commit it.
    /// The group position is pre-advanced, so fields are written
    /// to the correct offset.
    #[must_use]
    #[inline]
    pub fn start_entry(
        &mut self,
    ) -> Result<FuelFiguresEntryEncoder<'_>, sbe_rt::EncodeError> {
        if self.written as u32 >= self.count as u32 {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: (self.written as u32) + 1,
            });
        }
        let entry_pos = self.pos;
        self.pos += 6;
        self.written += 1;
        Ok(FuelFiguresEntryEncoder::wrap(&mut self.buf[entry_pos..], 0))
    }
}
impl<'a> FuelFiguresEncoder<'a> {
    /// Number of entries written so far (for `_unknown_size` back-patch).
    #[inline]
    pub fn written(&self) -> u16 {
        self.written
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct FuelFiguresEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> FuelFiguresEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[inline]
    pub fn speed(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[inline]
    pub fn mpg(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        self.buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[inline]
    #[must_use]
    pub fn usage_description(
        &mut self,
        data: &[u8],
    ) -> Result<&mut Self, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len().saturating_sub(self.pos),
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
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        self.pos = start + data.len();
        Ok(self)
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct PerformanceFiguresEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> PerformanceFiguresEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [1, 0, 0, 0];
    const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == 4);
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize, count: u16) -> Self {
        Self {
            buf,
            pos,
            count,
            written: 0,
        }
    }
    /// Write one group entry. Closure may return `()` or `Result<(), E>`
    /// ([`sbe_rt::GroupEncodeResult`]) so `?` works without `try_add`.
    #[inline]
    #[must_use]
    pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresEntryEncoder<'b>) -> sbe_rt::GroupResult,
    {
        if self.written >= self.count {
            return Err(
                sbe_rt::EncodeError::GroupFull {
                    declared: self.count as u32,
                    attempted: self.written as u32 + 1,
                }
                    .into(),
            );
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: block_len,
                    available: self.buf.len().saturating_sub(self.pos),
                }
                    .into(),
            );
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut __entry = PerformanceFiguresEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry)?;
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
    /// Manual entry creation: returns a borrowed entry encoder.
    /// The entry writes fixed fields directly into the group buffer.
    /// Drop the entry or let it go out of scope to commit it.
    /// The group position is pre-advanced, so fields are written
    /// to the correct offset.
    #[must_use]
    #[inline]
    pub fn start_entry(
        &mut self,
    ) -> Result<PerformanceFiguresEntryEncoder<'_>, sbe_rt::EncodeError> {
        if self.written as u32 >= self.count as u32 {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: (self.written as u32) + 1,
            });
        }
        let entry_pos = self.pos;
        self.pos += 1;
        self.written += 1;
        Ok(PerformanceFiguresEntryEncoder::wrap(&mut self.buf[entry_pos..], 0))
    }
}
impl<'a> PerformanceFiguresEncoder<'a> {
    /// Number of entries written so far (for `_unknown_size` back-patch).
    #[inline]
    pub fn written(&self) -> u16 {
        self.written
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct PerformanceFiguresEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> PerformanceFiguresEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[inline]
    pub fn octane_rating(&mut self, val: u8) -> &mut Self {
        self.buf[self.entry_start + 0] = val as u8;
        self
    }
    #[inline]
    #[must_use]
    pub fn acceleration<F>(
        &mut self,
        count: u16,
        f: F,
    ) -> Result<&mut Self, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresAccelerationEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.pos + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.pos),
                }
                    .into(),
            );
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&PerformanceFiguresAccelerationEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let __pos;
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut group = PerformanceFiguresAccelerationEncoder::wrap(
                __buf,
                self.pos + 4,
                count,
            );
            f(&mut group)?;
            let written = group.written();
            if written != count {
                return Err(sbe_rt::EncodeError::GroupCountMismatch {
                    declared: count as u32,
                    actual: written as u32,
                });
            }
            __pos = group.pos;
        }
        self.pos = __pos;
        Ok(self)
    }
    /// Nested-group `_unknown_size` variant — back-patches count.
    #[inline]
    pub fn acceleration_unknown_size<F>(
        &mut self,
        f: F,
    ) -> Result<&mut Self, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresAccelerationEncoder<'a>) -> sbe_rt::GroupResult,
    {
        if self.pos + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: 4,
                    available: self.buf.len().saturating_sub(self.pos),
                }
                    .into(),
            );
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&PerformanceFiguresAccelerationEncoder::GROUP_DIM_TEMPLATE);
        let count_offset = self.pos + 2;
        self.buf[count_offset..count_offset + 2].fill(0);
        let __pos;
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut group = PerformanceFiguresAccelerationEncoder::wrap(
                __buf,
                self.pos + 4,
                u16::MAX,
            );
            f(&mut group)?;
            let actual: u16 = group.written();
            __pos = group.pos;
            group
                .buf[count_offset..count_offset + 2]
                .copy_from_slice(&actual.to_le_bytes());
        }
        self.pos = __pos;
        Ok(self)
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct PerformanceFiguresAccelerationEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> PerformanceFiguresAccelerationEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [6, 0, 0, 0];
    const _GROUP_DIM_TEMPLATE_LEN: () = assert!(Self::GROUP_DIM_TEMPLATE.len() == 4);
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize, count: u16) -> Self {
        Self {
            buf,
            pos,
            count,
            written: 0,
        }
    }
    /// Write one group entry. Closure may return `()` or `Result<(), E>`
    /// ([`sbe_rt::GroupEncodeResult`]) so `?` works without `try_add`.
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
                    declared: self.count as u32,
                    attempted: self.written as u32 + 1,
                }
                    .into(),
            );
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: block_len,
                    available: self.buf.len().saturating_sub(self.pos),
                }
                    .into(),
            );
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut __entry = PerformanceFiguresAccelerationEntryEncoder::wrap(
                __buf,
                self.pos,
            );
            f(&mut __entry)?;
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
    /// Manual entry creation: returns a borrowed entry encoder.
    /// The entry writes fixed fields directly into the group buffer.
    /// Drop the entry or let it go out of scope to commit it.
    /// The group position is pre-advanced, so fields are written
    /// to the correct offset.
    #[must_use]
    #[inline]
    pub fn start_entry(
        &mut self,
    ) -> Result<PerformanceFiguresAccelerationEntryEncoder<'_>, sbe_rt::EncodeError> {
        if self.written as u32 >= self.count as u32 {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: (self.written as u32) + 1,
            });
        }
        let entry_pos = self.pos;
        self.pos += 6;
        self.written += 1;
        Ok(
            PerformanceFiguresAccelerationEntryEncoder::wrap(
                &mut self.buf[entry_pos..],
                0,
            ),
        )
    }
}
impl<'a> PerformanceFiguresAccelerationEncoder<'a> {
    /// Number of entries written so far (for `_unknown_size` back-patch).
    #[inline]
    pub fn written(&self) -> u16 {
        self.written
    }
}
/// Value struct for this group's entries.
#[derive(Debug, Clone, PartialEq)]
pub struct PerformanceFiguresAccelerationEntry {
    pub mph: u16,
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
        if self.written as u32 >= self.count as u32 {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: (self.written as u32) + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len().saturating_sub(self.pos),
            });
        }
        let pos = self.pos;
        self.pos += block_len;
        self.written += 1;
        self.buf[pos + 0..pos + 0 + 2].copy_from_slice(&entry.mph.to_le_bytes());
        self.buf[pos + 2..pos + 2 + 4].copy_from_slice(&entry.seconds.to_le_bytes());
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
        if attempted > self.count as usize {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: attempted.min(u32::MAX as usize) as u32,
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
            .pos
            .checked_add(needed)
            .ok_or(sbe_rt::EncodeError::EncodedLengthOverflow)?;
        if end > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len().saturating_sub(self.pos),
            });
        }
        {
            let region = &mut self.buf[self.pos..end];
            for (entry, slot) in entries.iter().zip(region.chunks_exact_mut(block_len)) {
                write_entry(entry, slot)?;
            }
        }
        self.pos = end;
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
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct PerformanceFiguresAccelerationEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> PerformanceFiguresAccelerationEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[inline]
    pub fn mph(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[inline]
    pub fn seconds(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        self.buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        self
    }
}
impl<'a> CarEncoder<'a> {
    /// Wrap a mutable buffer for encoding — no bounds check.
    /// Does **not** write the message header (`HeaderAbsent`).
    /// Caller guarantees the buffer is large enough.
    #[inline]
    pub fn wrap(
        buf: &'a mut [u8],
        msg_offset: usize,
    ) -> CarEncoder<'a, sbe_rt::HeaderAbsent> {
        CarEncoder {
            buf,
            msg_offset,
            pos: msg_offset + 53,
            _header: core::marker::PhantomData,
        }
    }
    /// Wrap a mutable buffer, write the header, and return the encoder.
    /// No bounds check — caller guarantees the buffer is large enough.
    /// This is the default fast path (matching sbe-tool's `wrap`).
    #[inline]
    pub fn wrap_and_apply_header(
        buf: &'a mut [u8],
        msg_offset: usize,
    ) -> CarEncoder<'a, sbe_rt::HeaderPresent> {
        buf[msg_offset..msg_offset + 8].copy_from_slice(&CarEncoder::HEADER_TEMPLATE);
        CarEncoder {
            buf,
            msg_offset,
            pos: msg_offset + 53,
            _header: core::marker::PhantomData,
        }
    }
}
/// Exact-length calculator for this message.
#[must_use = "length builder must be consumed"]
pub struct CarEncodedLength {
    state: EncodedLengthAccumulator,
}
impl CarEncodedLength {
    pub const BLOCK_LENGTH: usize = 45;
    pub const HEADER_LENGTH: usize = 8;
    /// Start computing the encoded length.
    pub const fn new() -> Self {
        Self {
            state: EncodedLengthAccumulator::new(Self::BLOCK_LENGTH),
        }
    }
}
impl CarEncodedLength {
    pub const FUELFIGURES_USAGEDESCRIPTION_PREFIX: usize = 4;
    pub const PERFORMANCEFIGURES_ACCELERATION_GROUP_DIM: usize = 4;
    pub const PERFORMANCEFIGURES_ACCELERATION_ENTRY_BLOCK: usize = 6;
    pub const MANUFACTURER_PREFIX: usize = 4;
    pub const MODEL_PREFIX: usize = 4;
    pub const ACTIVATIONCODE_PREFIX: usize = 4;
}
/// Schema-specific ragged entry builder — field-named methods bake in
/// the wire layout (dim/block/prefix). Chain: `b.add()?.field(len)?`.
pub struct CarFuelFiguresRaggedBuilder<'a> {
    b: &'a mut RaggedEntryBuilder,
}
impl<'a> CarFuelFiguresRaggedBuilder<'a> {
    /// Register one entry. Returns `&mut Self` for chaining.
    pub fn add(&mut self) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.add()?;
        Ok(self)
    }
    /// Register `count` identical entries at once (uniform shape — no
    /// per-entry var-data or nested-group differences). Shortcut for
    /// calling `add()` in a loop.
    pub fn uniform(&mut self, count: usize) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.entries(count)?;
        Ok(self)
    }
    /// Record a var-data field's length for the current entry.
    /// The prefix size is baked in — just pass the data length.
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
pub struct CarPerformanceFiguresRaggedBuilder<'a> {
    b: &'a mut RaggedEntryBuilder,
}
/// Schema-specific ragged entry builder — field-named methods bake in
/// the wire layout (dim/block/prefix). Chain: `b.add()?.field(len)?`.
pub struct CarPerformanceFiguresAccelerationRaggedBuilder<'a> {
    b: &'a mut RaggedEntryBuilder,
}
impl<'a> CarPerformanceFiguresAccelerationRaggedBuilder<'a> {
    /// Register one entry. Returns `&mut Self` for chaining.
    pub fn add(&mut self) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.add()?;
        Ok(self)
    }
    /// Register `count` identical entries at once (uniform shape — no
    /// per-entry var-data or nested-group differences). Shortcut for
    /// calling `add()` in a loop.
    pub fn uniform(&mut self, count: usize) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.entries(count)?;
        Ok(self)
    }
}
impl<'a> CarPerformanceFiguresRaggedBuilder<'a> {
    /// Register one entry. Returns `&mut Self` for chaining.
    pub fn add(&mut self) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.add()?;
        Ok(self)
    }
    /// Register `count` identical entries at once (uniform shape — no
    /// per-entry var-data or nested-group differences). Shortcut for
    /// calling `add()` in a loop.
    pub fn uniform(&mut self, count: usize) -> Result<&mut Self, sbe_rt::EncodeError> {
        self.b.entries(count)?;
        Ok(self)
    }
    /// Enter a nested ragged group. The closure receives a sub-builder
    /// with field-named methods for the nested entries.
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
pub struct CarEncodedLengthAfterPerformanceFigures {
    state: EncodedLengthAccumulator,
}
#[doc(hidden)]
pub struct CarEncodedLengthAfterManufacturer {
    state: EncodedLengthAccumulator,
}
#[doc(hidden)]
pub struct CarEncodedLengthAfterModel {
    state: EncodedLengthAccumulator,
}
#[doc(hidden)]
pub struct CarEncodedLengthAfterActivationCode {
    state: EncodedLengthAccumulator,
}
#[doc(hidden)]
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
    pub const fn usage_description(
        mut self,
        byte_len: usize,
    ) -> Result<CarEncodedLengthAfterPerformanceFigures, sbe_rt::EncodeError> {
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
                Ok(CarEncodedLengthAfterPerformanceFigures {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
    /// Complete this group when the entry count is zero.
    /// Returns an error if the declared count is non-zero.
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
impl CarFuelFiguresUniformEncodedLength {
    pub fn performance_figures(
        self,
        count: u16,
    ) -> CarEncodedLengthAfterPerformanceFigures {
        if self.declared_count != 0 {
            let mut state = self.state;
            state
                .fail(sbe_rt::EncodeError::GroupCountMismatch {
                    declared: self.declared_count,
                    actual: 0,
                });
            return CarEncodedLengthAfterPerformanceFigures {
                state,
            };
        }
        let mut state = self.state;
        state.leave_group(self.parent_multiplier);
        match state.check() {
            Ok(()) => {
                CarEncodedLengthAfterPerformanceFigures {
                    state,
                }
            }
            Err(e) => {
                state.fail(e);
                CarEncodedLengthAfterPerformanceFigures {
                    state,
                }
            }
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
    pub const fn fuel_figures(self, count: u16) -> CarFuelFiguresUniformEncodedLength {
        let mut state = self.state;
        let pm = state.enter_group(count as usize, 4 as usize, 6 as usize);
        CarFuelFiguresUniformEncodedLength {
            state,
            parent_multiplier: pm,
            declared_count: count as u32,
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
    pub fn fuel_figures_ragged<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<CarEncodedLengthAfterPerformanceFigures, sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut CarFuelFiguresRaggedBuilder<'_>,
        ) -> Result<(), sbe_rt::EncodeError>,
    {
        let pm = self.state.enter_group(count as usize, 4 as usize, 6 as usize);
        self.state.leave_group(pm);
        let mut builder = RaggedEntryBuilder::new(self.state, pm, 0);
        let mut wrapper = CarFuelFiguresRaggedBuilder {
            b: &mut builder,
        };
        f(&mut wrapper)?;
        if builder.written != count as usize {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared: count as u32,
                actual: builder.written as u32,
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
    /// **Unknown-size** group — the entry count is discovered from
    /// the data (e.g. draining an iterator), not known up-front.
    /// Like the ragged path but without a declared `count`: call
    /// `builder.add()` (or `builder.entries(n)`) once per entry;
    /// the builder counts completed entries and rejects overflow
    /// of the wire count type (`#count_ty`). Each `add()` contributes
    /// the entry's fixed block, plus any `group()`/`var_data()` you
    /// record for that entry.
    pub fn fuel_figures_unknown_size<F>(
        mut self,
        f: F,
    ) -> Result<CarEncodedLengthAfterPerformanceFigures, sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut CarFuelFiguresRaggedBuilder<'_>,
        ) -> Result<(), sbe_rt::EncodeError>,
    {
        let max_count = u16::MAX as usize;
        let pm = self.state.multiplier();
        self.state.add_scaled(4 as usize, pm);
        let mut builder = RaggedEntryBuilder::new(self.state, pm, 6 as usize);
        let mut wrapper = CarFuelFiguresRaggedBuilder {
            b: &mut builder,
        };
        f(&mut wrapper)?;
        if builder.written > max_count {
            return Err(sbe_rt::EncodeError::GroupCountOverflow {
                maximum: u16::MAX as u32,
                actual: builder.written as u32,
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
#[doc(hidden)]
#[must_use = "complete the nested shape or call finish_empty()"]
pub struct CarPerformanceFiguresUniformEncodedLength {
    state: EncodedLengthAccumulator,
    parent_multiplier: usize,
    declared_count: u32,
}
impl CarPerformanceFiguresUniformEncodedLength {
    pub const fn acceleration(
        mut self,
        count: u16,
    ) -> Result<CarEncodedLengthAfterManufacturer, sbe_rt::EncodeError> {
        let pm = self.state.enter_group(count as usize, 4 as usize, 6 as usize);
        self.state.leave_group(pm);
        match self.state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterManufacturer {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
    /// Complete this group when the entry count is zero.
    /// Returns an error if the declared count is non-zero.
    pub fn finish_empty(
        self,
    ) -> Result<CarEncodedLengthAfterManufacturer, sbe_rt::EncodeError> {
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
                Ok(CarEncodedLengthAfterManufacturer {
                    state,
                })
            }
            Err(e) => Err(e),
        }
    }
}
impl CarPerformanceFiguresUniformEncodedLength {
    pub fn manufacturer(self, byte_len: usize) -> CarEncodedLengthAfterManufacturer {
        if self.declared_count != 0 {
            let mut state = self.state;
            state
                .fail(sbe_rt::EncodeError::GroupCountMismatch {
                    declared: self.declared_count,
                    actual: 0,
                });
            return CarEncodedLengthAfterManufacturer {
                state,
            };
        }
        let mut state = self.state;
        state.leave_group(self.parent_multiplier);
        match state.check() {
            Ok(()) => {
                CarEncodedLengthAfterManufacturer {
                    state,
                }
            }
            Err(e) => {
                state.fail(e);
                CarEncodedLengthAfterManufacturer {
                    state,
                }
            }
        }
    }
}
impl CarEncodedLengthAfterPerformanceFigures {
    /// **Uniform** group — every one of the `count` entries shares
    /// exactly the same wire shape (same fixed block AND the same
    /// nested-group counts / var-data lengths). The length is the
    /// single entry shape multiplied by `count`, so no per-entry
    /// description is needed. This is the fastest path; prefer it
    /// whenever all entries are identical.
    pub const fn performance_figures(
        self,
        count: u16,
    ) -> CarPerformanceFiguresUniformEncodedLength {
        let mut state = self.state;
        let pm = state.enter_group(count as usize, 4 as usize, 1 as usize);
        CarPerformanceFiguresUniformEncodedLength {
            state,
            parent_multiplier: pm,
            declared_count: count as u32,
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
    pub fn performance_figures_ragged<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<CarEncodedLengthAfterManufacturer, sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut CarPerformanceFiguresRaggedBuilder<'_>,
        ) -> Result<(), sbe_rt::EncodeError>,
    {
        let pm = self.state.enter_group(count as usize, 4 as usize, 1 as usize);
        self.state.leave_group(pm);
        let mut builder = RaggedEntryBuilder::new(self.state, pm, 0);
        let mut wrapper = CarPerformanceFiguresRaggedBuilder {
            b: &mut builder,
        };
        f(&mut wrapper)?;
        if builder.written != count as usize {
            return Err(sbe_rt::EncodeError::GroupCountMismatch {
                declared: count as u32,
                actual: builder.written as u32,
            });
        }
        self.state = builder.state;
        self.state.leave_group(pm);
        match self.state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterManufacturer {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
    /// **Unknown-size** group — the entry count is discovered from
    /// the data (e.g. draining an iterator), not known up-front.
    /// Like the ragged path but without a declared `count`: call
    /// `builder.add()` (or `builder.entries(n)`) once per entry;
    /// the builder counts completed entries and rejects overflow
    /// of the wire count type (`#count_ty`). Each `add()` contributes
    /// the entry's fixed block, plus any `group()`/`var_data()` you
    /// record for that entry.
    pub fn performance_figures_unknown_size<F>(
        mut self,
        f: F,
    ) -> Result<CarEncodedLengthAfterManufacturer, sbe_rt::EncodeError>
    where
        F: FnOnce(
            &mut CarPerformanceFiguresRaggedBuilder<'_>,
        ) -> Result<(), sbe_rt::EncodeError>,
    {
        let max_count = u16::MAX as usize;
        let pm = self.state.multiplier();
        self.state.add_scaled(4 as usize, pm);
        let mut builder = RaggedEntryBuilder::new(self.state, pm, 1 as usize);
        let mut wrapper = CarPerformanceFiguresRaggedBuilder {
            b: &mut builder,
        };
        f(&mut wrapper)?;
        if builder.written > max_count {
            return Err(sbe_rt::EncodeError::GroupCountOverflow {
                maximum: u16::MAX as u32,
                actual: builder.written as u32,
            });
        }
        self.state = builder.state;
        match self.state.check() {
            Ok(()) => {
                Ok(CarEncodedLengthAfterManufacturer {
                    state: self.state,
                })
            }
            Err(e) => Err(e),
        }
    }
}
impl CarEncodedLengthAfterManufacturer {
    pub const fn manufacturer(
        self,
        byte_len: usize,
    ) -> Result<CarEncodedLengthAfterModel, sbe_rt::EncodeError> {
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
    pub const fn model(
        self,
        byte_len: usize,
    ) -> Result<CarEncodedLengthAfterActivationCode, sbe_rt::EncodeError> {
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
        Ok(CarEncodedLengthAfterActivationCode {
            state: EncodedLengthAccumulator {
                len,
                multiplier: 1,
                error: None,
            },
        })
    }
}
impl CarEncodedLengthAfterActivationCode {
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
    pub const fn encoded_length(&self) -> usize {
        self.state.len
    }
    pub const fn encoded_length_with_header(&self) -> usize {
        self.state.len + 8 as usize
    }
}
pub mod car_field_meta {
    pub struct FieldInfo {
        pub name: &'static str,
        pub id: u16,
        pub offset: usize,
        pub since_version: u16,
        pub field_type: &'static str,
        pub presence: &'static str,
        pub null_value: Option<&'static str>,
        pub semantic_type: Option<&'static str>,
        pub description: Option<&'static str>,
    }
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
    pub fn add(&mut self) -> sbe_rt::GroupResult {
        self.state.add_scaled(self.entry_block_length, self.parent_multiplier);
        self.written += 1;
        Ok(())
    }
    /// Register N flat entries at once (for fixed-width unknown-size groups).
    pub fn entries(&mut self, n: usize) -> sbe_rt::GroupResult {
        for _ in 0..n {
            self.state.add_scaled(self.entry_block_length, self.parent_multiplier);
        }
        self.written += n;
        Ok(())
    }
    /// Add a nested group dimension + entries.
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
    pub fn var_data(&mut self, prefix: usize, byte_len: usize) -> sbe_rt::GroupResult {
        self.state.add_scaled(prefix, self.parent_multiplier);
        self.state.add_scaled(byte_len, self.parent_multiplier);
        self.state.check()?;
        Ok(())
    }
}
pub const SEMANTIC_VERSION: &str = "5.2";
pub const SCHEMA_HASH: u64 = 11133254787130522899;
pub const SCHEMA_SHA256: [u8; 32] = [
    0x24, 0x95, 0xbf, 0xf8, 0x72, 0x90, 0x99, 0x27, 0x2f, 0xbd, 0x82, 0xb5, 0xc1, 0xba,
    0x60, 0xc6, 0xfe, 0x51, 0xb2, 0x0f, 0x36, 0xe5, 0x5a, 0x02, 0x0a, 0x42, 0xf4, 0xd4,
    0xa4, 0x88, 0x36, 0x80,
];
pub const SCHEMA_SHA256_HEX: &str = "2495bff8729099272fbd82b5c1ba60c6fe51b20f36e55a020a42f4d4a4883680";
pub const SCHEMA_ID: u16 = 1;
pub const SCHEMA_VERSION: u16 = 0;
pub mod prelude {
    pub use super::sbe_rt::{
        DecodeError, EncodeError, VerifyError, MetaAttribute, SbeMessage,
    };
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
/// Prefer [`read_bytes_unchecked`] when the caller has already
/// validated bounds.
#[inline]
pub fn read_bytes<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
    buf[offset..offset + N].try_into().expect("read_bytes: buffer too short")
}
#[inline]
pub fn write_bytes<const N: usize>(buf: &mut [u8], offset: usize, bytes: &[u8; N]) {
    buf[offset..offset + N].copy_from_slice(bytes);
}
/// Unchecked companion to [`read_bytes`] — zero bounds checks.
/// Caller guarantees `offset + N <= buf.len()`.
#[inline]
pub fn read_bytes_unchecked<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
    unsafe { core::ptr::read_unaligned(buf.as_ptr().add(offset) as *const [u8; N]) }
}
/// Unchecked companion to [`write_bytes`] — zero bounds checks.
/// Caller guarantees `offset + N <= buf.len()`.
#[inline]
pub fn write_bytes_unchecked<const N: usize>(
    buf: &mut [u8],
    offset: usize,
    bytes: &[u8; N],
) {
    unsafe {
        core::ptr::write_unaligned(buf.as_mut_ptr().add(offset) as *mut [u8; N], *bytes)
    }
}
#[inline]
pub fn schema_id_from_header(buf: &[u8]) -> Option<u16> {
    if buf.len() < 4 + 2 {
        return None;
    }
    let bytes = read_bytes::<2>(buf, 4);
    let value = u16::from_le_bytes(bytes) as u64;
    u16::try_from(value).ok()
}
#[non_exhaustive]
pub enum AnyMessage<'a> {
    Car(CarDecoder<'a>),
    Unknown { header: MessageHeader, payload: &'a [u8] },
}
pub struct DecodedFrame<'a> {
    pub message: AnyMessage<'a>,
    pub range: core::ops::Range<usize>,
    pub len: usize,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramingPolicy {
    LengthPrefixU32,
    LengthPrefixU16,
    Fixed(usize),
}
pub struct FrameCursor<'a> {
    buf: &'a [u8],
    pos: usize,
    framing: FramingPolicy,
}
impl<'a> FrameCursor<'a> {
    #[inline]
    pub const fn new(buf: &'a [u8], framing: FramingPolicy) -> Self {
        Self { buf, pos: 0, framing }
    }
}
impl<'a> Iterator for FrameCursor<'a> {
    type Item = Result<DecodedFrame<'a>, sbe_rt::DecodeError>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.pos >= self.buf.len() {
            return None;
        }
        let (header_len, frame_len) = match self.framing {
            FramingPolicy::LengthPrefixU32 => {
                if 4 > self.buf.len().saturating_sub(self.pos) {
                    return Some(
                        Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "length prefix",
                            needed: 4,
                            available: self.buf.len().saturating_sub(self.pos),
                        }),
                    );
                }
                let bytes: [u8; 4] = read_bytes::<4>(self.buf, self.pos);
                let len = u32::from_le_bytes(bytes) as usize;
                (4, len)
            }
            FramingPolicy::LengthPrefixU16 => {
                if 2 > self.buf.len().saturating_sub(self.pos) {
                    return Some(
                        Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "length prefix",
                            needed: 2,
                            available: self.buf.len().saturating_sub(self.pos),
                        }),
                    );
                }
                let bytes: [u8; 2] = read_bytes::<2>(self.buf, self.pos);
                let len = u16::from_le_bytes(bytes) as usize;
                (2, len)
            }
            FramingPolicy::Fixed(len) => (0, len),
        };
        let frame_start = match self.pos.checked_add(header_len) {
            Some(value) => value,
            None => {
                return Some(
                    Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "frame bounds",
                        needed: usize::MAX,
                        available: self.buf.len().saturating_sub(self.pos),
                    }),
                );
            }
        };
        let frame_end = match frame_start.checked_add(frame_len) {
            Some(value) => value,
            None => {
                return Some(
                    Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "frame bounds",
                        needed: usize::MAX,
                        available: self.buf.len().saturating_sub(self.pos),
                    }),
                );
            }
        };
        if frame_end > self.buf.len() {
            return Some(
                Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "frame bounds",
                    needed: header_len.saturating_add(frame_len),
                    available: self.buf.len().saturating_sub(self.pos),
                }),
            );
        }
        let res = AnyMessage::decode_frame(self.buf, frame_start, frame_len);
        match res {
            Ok(frame) => {
                self.pos = frame_end;
                Some(Ok(frame))
            }
            Err(e) => Some(Err(e)),
        }
    }
}
impl<'a> AnyMessage<'a> {
    #[inline]
    pub fn decode(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
        if 8 > buf.len().saturating_sub(pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: buf.len().saturating_sub(pos),
            });
        }
        let header_bytes = read_bytes::<8>(buf, pos);
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
        let body_pos = pos + 8;
        if schema_id != 1 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 1,
                actual: schema_id,
                expected_name: "baseline",
            });
        }
        match template_id {
            CarSchema::TEMPLATE_ID => {
                Ok(Self::Car(CarDecoder::wrap(buf, pos, block_length, version)))
            }
            _ => {
                Err(sbe_rt::DecodeError::UnknownTemplateLength {
                    template_id,
                })
            }
        }
    }
}
impl<'a> AnyMessage<'a> {
    #[inline]
    pub fn decode_frame(
        buf: &'a [u8],
        pos: usize,
        frame_len: usize,
    ) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
        if 8 > buf.len().saturating_sub(pos) {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: buf.len().saturating_sub(pos),
            });
        }
        let header_bytes: [u8; 8] = read_bytes::<8>(buf, pos);
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
        let body_pos = pos + 8;
        if schema_id != 1 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 1,
                actual: schema_id,
                expected_name: "baseline",
            });
        }
        match template_id {
            CarSchema::TEMPLATE_ID => {
                let decoder = CarDecoder::wrap(buf, pos, block_length, version);
                let total_len = decoder.encoded_length_with_header()?;
                if total_len > frame_len {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "Car",
                        needed: total_len,
                        available: frame_len,
                    });
                }
                Ok(DecodedFrame {
                    message: Self::Car(decoder),
                    range: pos..pos + total_len,
                    len: total_len,
                })
            }
            _ => {
                if frame_len > buf.len().saturating_sub(pos) {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "template body",
                        needed: frame_len,
                        available: buf.len().saturating_sub(pos),
                    });
                }
                let payload = &buf[pos..pos + frame_len];
                Ok(DecodedFrame {
                    message: Self::Unknown { header, payload },
                    range: pos..pos + frame_len,
                    len: frame_len,
                })
            }
        }
    }
}
impl<'a> AnyMessage<'a> {
    #[inline]
    pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
        match self {
            Self::Car(d) => d.encoded_length_with_header(),
            Self::Unknown { payload, .. } => Ok(payload.len()),
        }
    }
}
impl<'a> AnyMessage<'a> {
    /// Complete SBE frame (message header + body) for known variants;
    /// raw payload bytes for [`Self::Unknown`].
    #[inline]
    pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        match self {
            Self::Car(d) => d.as_bytes_with_header(),
            Self::Unknown { payload, .. } => Ok(payload),
        }
    }
}
impl<'a> AnyMessage<'a> {
    #[inline]
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
        match self {
            Self::Car(d) => {
                let len = d.encoded_length_with_header()?;
                let bytes = d.as_bytes_with_header()?;
                buf[..len].copy_from_slice(bytes);
                Ok(len)
            }
            Self::Unknown { payload, .. } => {
                buf[..payload.len()].copy_from_slice(payload);
                Ok(payload.len())
            }
        }
    }
}
pub trait MessageVisitor {
    type Output;
    fn visit_car(&mut self, decoder: &CarDecoder<'_>) -> Self::Output;
    /// Called for unknown template IDs (not in this schema).
    /// `header` is the raw schema-declared MessageHeader; `payload` is
    /// the bytes after the header. Default returns `unimplemented!()`.
    fn visit_unknown(&mut self, header: &MessageHeader, payload: &[u8]) -> Self::Output {
        unimplemented!(
            "unknown template id {} in schema {}", header.template_id(),
            stringify!("baseline")
        )
    }
}
impl<'a> AnyMessage<'a> {
    pub fn visit<V: MessageVisitor>(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::Car(d) => visitor.visit_car(d),
            Self::Unknown { header, payload } => visitor.visit_unknown(header, payload),
        }
    }
}

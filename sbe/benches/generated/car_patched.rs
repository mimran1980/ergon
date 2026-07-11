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
        InvalidVarDataLength { field: &'static str, length: u32, max_length: u32 },
        Utf8(core::str::Utf8Error),
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
                Self::InvalidVarDataLength { field, length, max_length } => {
                    write!(
                        f, "var data field '{}: length {} exceeds max {}", field, length,
                        max_length
                    )
                }
                Self::Utf8(err) => write!(f, "UTF-8 decode error: {}", err),
            }
        }
    }
    impl core::error::Error for DecodeError {}
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EncodeError {
        BufferTooShort { needed: usize, available: usize },
        VarDataTooLong { field: &'static str, max_length: usize, actual: usize },
        GroupFull { declared: u32, attempted: u32 },
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
                Self::GroupFull { declared, attempted } => {
                    write!(
                        f, "group full: declared count {}, attempted to write {}",
                        declared, attempted
                    )
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum VerifyError {
        HeaderTooShort,
        InvalidBlockLength { expected_min: usize, actual: usize },
        GroupDimOutOfBounds { field: &'static str, offset: usize },
        VarDataOutOfBounds { field: &'static str, offset: usize, length: u32 },
        MessageTooShort { needed: usize, available: usize },
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
            }
        }
    }
    impl core::error::Error for VerifyError {}
    #[diagnostic::on_unimplemented(
        message = "`{Self}` is not a generated SBE message type",
        note = "SbeMessage is a sealed trait — only types generated by `ergosbe::Generator` can implement it. Import the generated module and use the provided decoder/encoder types directly."
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
    pub trait EncodeGroupEntry<E> {
        fn encode(self, entry: &mut E);
    }
    impl<E, F> EncodeGroupEntry<E> for F
    where
        F: FnOnce(&mut E),
    {
        #[inline]
        fn encode(self, entry: &mut E) {
            self(entry);
        }
    }
}
///Boolean Type.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum BooleanType {
    F = 0,
    T = 1,
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal,
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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Model {
    A = b'A',
    B = b'B',
    C = b'C',
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal,
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct OptionalExtras(pub u8);
impl OptionalExtras {
    pub const fn raw(self) -> u8 {
        self.0
    }
    pub const fn default() -> Self {
        Self(0)
    }
    pub const fn sun_roof(self) -> bool {
        (self.0 & (1 << 0)) != 0
    }
    pub fn set_sun_roof(&mut self, val: bool) {
        if val {
            self.0 |= 1 << 0;
        } else {
            self.0 &= !(1 << 0);
        }
    }
    pub const fn sports_pack(self) -> bool {
        (self.0 & (1 << 1)) != 0
    }
    pub fn set_sports_pack(&mut self, val: bool) {
        if val {
            self.0 |= 1 << 1;
        } else {
            self.0 &= !(1 << 1);
        }
    }
    pub const fn cruise_control(self) -> bool {
        (self.0 & (1 << 2)) != 0
    }
    pub fn set_cruise_control(&mut self, val: bool) {
        if val {
            self.0 |= 1 << 2;
        } else {
            self.0 &= !(1 << 2);
        }
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
///Message identifiers and length of message root.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
#[derive(Clone, Copy)]
pub struct MessageHeaderDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}
impl<'a> MessageHeaderDecoder<'a> {
    #[inline]
    pub fn block_length(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    #[inline]
    pub fn template_id(&self) -> u16 {
        let offset = self.pos + 2;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    #[inline]
    pub fn schema_id(&self) -> u16 {
        let offset = self.pos + 4;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    #[inline]
    pub fn version(&self) -> u16 {
        let offset = self.pos + 6;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
}
///Repeating group dimensions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    buf: &'a [u8],
    pos: usize,
}
impl<'a> GroupSizeEncodingDecoder<'a> {
    #[inline]
    pub fn block_length(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    #[inline]
    pub fn num_in_group(&self) -> u16 {
        let offset = self.pos + 2;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
}
///Variable length UTF-8 String.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    buf: &'a [u8],
    pos: usize,
}
impl<'a> VarStringEncodingDecoder<'a> {
    #[inline]
    pub fn length(&self) -> u32 {
        let offset = self.pos + 0;
        u32::from_le_bytes(read_bytes::<4>(self.buf, offset))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
///Variable length ASCII String.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    buf: &'a [u8],
    pos: usize,
}
impl<'a> VarAsciiEncodingDecoder<'a> {
    #[inline]
    pub fn length(&self) -> u32 {
        let offset = self.pos + 0;
        u32::from_le_bytes(read_bytes::<4>(self.buf, offset))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
///Variable length binary blob.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
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
    buf: &'a [u8],
    pos: usize,
}
impl<'a> VarDataEncodingDecoder<'a> {
    #[inline]
    pub fn length(&self) -> u32 {
        let offset = self.pos + 0;
        u32::from_le_bytes(read_bytes::<4>(self.buf, offset))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct Booster(pub [u8; 1]);
impl Booster {
    #[inline]
    pub fn horse_power(&self) -> u8 {
        u8::from_le_bytes(read_bytes::<1>(&self.0, 0))
    }
    pub fn new(horse_power: u8) -> Self {
        let mut bytes = [0u8; 1];
        let val_bytes = horse_power.to_le_bytes();
        write_bytes::<1>(&mut bytes, 0, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < Booster > () == 1);
#[derive(Clone, Copy)]
pub struct BoosterDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}
impl<'a> BoosterDecoder<'a> {
    #[inline]
    pub fn horse_power(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct Engine(pub [u8; 6]);
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
    pub fn new(capacity: u16, num_cylinders: u8, manufacturer_code: [u8; 3]) -> Self {
        let mut bytes = [0u8; 6];
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
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < Engine > () == 6);
#[derive(Clone, Copy)]
pub struct EngineDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}
impl<'a> EngineDecoder<'a> {
    #[inline]
    pub fn capacity(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    #[inline]
    pub fn num_cylinders(&self) -> u8 {
        let offset = self.pos + 2;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
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
            res[idx] = u8::from_le_bytes(read_bytes::<1>(self.buf, offset));
            idx += 1;
        }
        res
    }
    #[inline]
    pub const fn fuel(&self) -> &'static str {
        "Petrol"
    }
}
///Description of a basic Car
#[derive(Clone, Copy)]
pub struct CarDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> CarDecoder<'a> {
    pub const SCHEMA_ID: u16 = 1;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 41;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 41);
    ///MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
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
            acting_block_length,
            acting_version,
        }
    }
    #[inline]
    pub fn wrap_and_apply_header(
        buf: &'a [u8],
        pos: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if pos + 8 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: buf.len().saturating_sub(pos),
            });
        }
        let header_bytes: [u8; 8] = read_bytes::<8>(buf, pos);
        let header = MessageHeader(header_bytes);
        if header.template_id() != Self::TEMPLATE_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::TEMPLATE_ID,
                actual: header.template_id(),
                expected_name: "baseline",
            });
        }
        if header.schema_id() != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: header.schema_id(),
                expected_name: "baseline",
            });
        }
        Ok(Self::wrap(buf, pos + 8, header.block_length() as usize, header.version()))
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
        u64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const SERIAL_NUMBER_NULL: u64 = 18446744073709551615_u64;
    pub const SERIAL_NUMBER_MIN: u64 = 0_u64;
    pub const SERIAL_NUMBER_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    pub fn model_year(&self) -> u16 {
        let offset = self.pos + 8;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    pub const MODEL_YEAR_NULL: u16 = 65535_u16;
    pub const MODEL_YEAR_MIN: u16 = 0_u16;
    pub const MODEL_YEAR_MAX: u16 = 65534_u16;
    #[inline]
    pub fn available(&self) -> BooleanType {
        let offset = self.pos + 10;
        BooleanType::from_raw(u8::from_le_bytes(read_bytes::<1>(self.buf, offset)))
    }
    pub const AVAILABLE_NULL: BooleanType = BooleanType::NullVal;
    #[inline]
    pub fn code(&self) -> Model {
        let offset = self.pos + 11;
        Model::from_raw(u8::from_le_bytes(read_bytes::<1>(self.buf, offset)))
    }
    pub const CODE_NULL: Model = Model::NullVal;
    #[inline]
    pub fn some_numbers(&self) -> [u32; 4] {
        if self.acting_version < 0 || 28 > self.acting_block_length {
            return [0 as u32; 4];
        }
        let offset = self.pos + 12;
        let all: [u8; 16] = read_bytes::<16>(self.buf, offset);
        [
            u32::from_le_bytes([all[0usize], all[1usize], all[2usize], all[3usize]]),
            u32::from_le_bytes([all[4usize], all[5usize], all[6usize], all[7usize]]),
            u32::from_le_bytes([all[8usize], all[9usize], all[10usize], all[11usize]]),
            u32::from_le_bytes([all[12usize], all[13usize], all[14usize], all[15usize]]),
        ]
    }
    pub const SOME_NUMBERS_NULL: u32 = 4294967295_u32;
    pub const SOME_NUMBERS_MIN: u32 = 0_u32;
    pub const SOME_NUMBERS_MAX: u32 = 4294967294_u32;
    #[inline]
    pub fn vehicle_code(&self) -> [u8; 6] {
        if self.acting_version < 0 || 34 > self.acting_block_length {
            return [0 as u8; 6];
        }
        let offset = self.pos + 28;
        let all: [u8; 6] = read_bytes::<6>(self.buf, offset);
        [
            u8::from_le_bytes([all[0usize]]),
            u8::from_le_bytes([all[1usize]]),
            u8::from_le_bytes([all[2usize]]),
            u8::from_le_bytes([all[3usize]]),
            u8::from_le_bytes([all[4usize]]),
            u8::from_le_bytes([all[5usize]]),
        ]
    }
    pub const VEHICLE_CODE_NULL: u8 = 0_u8;
    pub const VEHICLE_CODE_MIN: u8 = 32_u8;
    pub const VEHICLE_CODE_MAX: u8 = 126_u8;
    #[inline]
    pub fn extras(&self) -> OptionalExtras {
        let offset = self.pos + 34;
        OptionalExtras(u8::from_le_bytes(read_bytes::<1>(self.buf, offset)))
    }
    #[inline]
    pub const fn discounted_model(&self) -> Model {
        Model::C
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
    pub fn engine_as_struct(&self) -> Engine {
        let offset = self.pos + 35;
        Engine(read_bytes::<6>(self.buf, offset))
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
                available: self.buf.len() - start,
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
                available: self.buf.len() - start,
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
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "manufacturer",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "manufacturer",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn tail_offset_4(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_3()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "model",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "model",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn tail_offset_5(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_4()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "activationCode",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "activationCode",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
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
    fn manufacturer(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(manufacturer),
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn manufacturer_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.manufacturer()?;
        core::str::from_utf8(bytes).map_err(sbe_rt::DecodeError::Utf8)
    }
    #[inline]
    fn model(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_3()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(model),
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn model_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.model()?;
        core::str::from_utf8(bytes).map_err(sbe_rt::DecodeError::Utf8)
    }
    #[inline]
    fn activation_code(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_4()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(activation_code),
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn activation_code_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.activation_code()?;
        core::str::from_utf8(bytes).map_err(sbe_rt::DecodeError::Utf8)
    }
    /// Return a fresh copy of this decoder at the initial body position.
    /// The decoder is a stateless flyweight (Copy), so rewind is a no-op
    /// — it exists for API symmetry with cursor-based decoders.
    #[inline]
    pub fn rewind(&self) -> Self {
        *self
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
    pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let len = self.encoded_length_with_header()?;
        let start = self.pos - 8;
        Ok(&self.buf[start..start + len])
    }
    #[inline]
    pub fn verify(buf: &[u8]) -> Result<(), sbe_rt::VerifyError> {
        if buf.len() < 8 {
            return Err(sbe_rt::VerifyError::HeaderTooShort);
        }
        let header_bytes: [u8; 8] = read_bytes::<8>(buf, 0);
        let header = MessageHeader(header_bytes);
        let block_length = header.block_length() as usize;
        if block_length < Self::BLOCK_LENGTH {
            return Err(sbe_rt::VerifyError::InvalidBlockLength {
                expected_min: Self::BLOCK_LENGTH,
                actual: block_length,
            });
        }
        let body_end = 8 + block_length;
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
            let entries_end = offset + 4 + count * 6;
            if entries_end > buf.len() {
                return Err(sbe_rt::VerifyError::MessageTooShort {
                    needed: entries_end,
                    available: buf.len(),
                });
            }
            offset = entries_end;
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
            let entries_end = offset + 4 + count * 1;
            if entries_end > buf.len() {
                return Err(sbe_rt::VerifyError::MessageTooShort {
                    needed: entries_end,
                    available: buf.len(),
                });
            }
            offset = entries_end;
        }
        {
            if offset + 4 > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "manufacturer",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let var_header = VarStringEncoding(bytes);
            let len = var_header.length();
            let data_end = offset + 4 + len as usize;
            if data_end > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "manufacturer",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        {
            if offset + 4 > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "model",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let var_header = VarStringEncoding(bytes);
            let len = var_header.length();
            let data_end = offset + 4 + len as usize;
            if data_end > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "model",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        {
            if offset + 4 > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "activation_code",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let var_header = VarAsciiEncoding(bytes);
            let len = var_header.length();
            let data_end = offset + 4 + len as usize;
            if data_end > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "activation_code",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        Ok(())
    }
}
impl<'a> TryFrom<&'a [u8]> for CarDecoder<'a> {
    type Error = sbe_rt::DecodeError;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::wrap_and_apply_header(buf, 0)
    }
}
impl<'a> sbe_rt::private::Sealed for CarDecoder<'a> {}
impl<'a> sbe_rt::SbeMessage for CarDecoder<'a> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 41;
    const SCHEMA_ID: u16 = 1;
    const SCHEMA_VERSION: u16 = 0;
}
impl<'a> AsRef<[u8]> for CarDecoder<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes().unwrap_or(&[])
    }
}
impl<'a> CarDecoder<'a> {
    pub fn as_ref_opt(&self) -> Option<&[u8]> {
        self.as_bytes().ok()
    }
}
impl<'a> core::fmt::Display for CarDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Car {{ ")?;
        {
            let v = self.serial_number();
            write!(f, "serial_number: {:?}", v)?;
        }
        {
            let v = self.model_year();
            write!(f, ", model_year: {:?}", v)?;
        }
        {
            let e = self.available();
            write!(f, ", available: BooleanType::{e:?}")?;
        }
        {
            let e = self.code();
            write!(f, ", code: Model::{e:?}")?;
        }
        if let Ok(g) = self.fuel_figures() {
            write!(f, ", fuel_figures: [")?;
            for (i, result) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                match result {
                    Ok(entry) => write!(f, "{}", entry)?,
                    Err(_) => write!(f, "{{err}}")?,
                }
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.performance_figures() {
            write!(f, ", performance_figures: [")?;
            for (i, result) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                match result {
                    Ok(entry) => write!(f, "{}", entry)?,
                    Err(_) => write!(f, "{{err}}")?,
                }
            }
            write!(f, "]")?;
        }
        if let Ok(d) = self.manufacturer() {
            write!(f, ", manufacturer: {} bytes", d.len())?;
        }
        if let Ok(d) = self.model() {
            write!(f, ", model: {} bytes", d.len())?;
        }
        if let Ok(d) = self.activation_code() {
            write!(f, ", activation_code: {} bytes", d.len())?;
        }
        write!(f, " }}")
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
        if pos + 4 > buf.len() {
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
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
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
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        for _ in 0..n {
            let entry = FuelFiguresEntryDecoder::wrap(
                self.buf,
                self.pos,
                self.acting_block_length,
                self.acting_version,
            );
            if cfg!(not(feature = "bound-check-disabled")) {
                self.pos += entry.encoded_length()?;
            } else {
                self.pos += entry.encoded_length().unwrap();
            }
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
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            FuelFiguresEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for FuelFiguresDecoder<'a> {
    type Item = Result<FuelFiguresEntryDecoder<'a>, sbe_rt::DecodeError>;
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
        #[cfg(not(feature = "bound-check-disabled"))]
        let size = match entry.encoded_length() {
            Ok(s) => s,
            Err(e) => {
                self.count = 0;
                return Some(Err(e));
            }
        };
        #[cfg(feature = "bound-check-disabled")]
        let size = entry.encoded_length().unwrap();
        self.pos += size;
        self.count -= 1;
        Some(Ok(entry))
    }
}
impl<'a> ExactSizeIterator for FuelFiguresDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct FuelFiguresEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
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
        }
    }
    #[inline]
    pub fn speed(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    pub const SPEED_NULL: u16 = 65535_u16;
    pub const SPEED_MIN: u16 = 0_u16;
    pub const SPEED_MAX: u16 = 65534_u16;
    #[inline]
    pub fn mpg(&self) -> f32 {
        let offset = self.pos + 2;
        f32::from_le_bytes(read_bytes::<4>(self.buf, offset))
    }
    pub const MPG_NULL: f32 = f32::from_bits(2139095041u32);
    pub const MPG_MIN: f32 = f32::from_bits(4286578687u32);
    pub const MPG_MAX: f32 = f32::from_bits(2139095039u32);
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "usageDescription",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "usageDescription",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    pub fn usage_description(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_1()? - self.pos)
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
            write!(f, ", usageDescription: {} bytes", d.len())?;
        }
        write!(f, " }}")
    }
}
pub struct FuelFiguresEntryDecoderComplete<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
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
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "usageDescription",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "usageDescription",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = FuelFiguresEntryDecoderComplete {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
}
impl<'a> FuelFiguresEntryDecoderComplete<'a> {
    /// Header-inclusive bytes (for an entry, the entry bytes; header_size is 0).
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        &self.buf[self.pos - 0..self.tail_start]
    }
    /// Body length (excluding header).
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.tail_start - self.pos
    }
    /// Header-inclusive length.
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.tail_start - self.pos + 0
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
        if pos + 4 > buf.len() {
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
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
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
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        for _ in 0..n {
            let entry = PerformanceFiguresEntryDecoder::wrap(
                self.buf,
                self.pos,
                self.acting_block_length,
                self.acting_version,
            );
            if cfg!(not(feature = "bound-check-disabled")) {
                self.pos += entry.encoded_length()?;
            } else {
                self.pos += entry.encoded_length().unwrap();
            }
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
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            PerformanceFiguresEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for PerformanceFiguresDecoder<'a> {
    type Item = Result<PerformanceFiguresEntryDecoder<'a>, sbe_rt::DecodeError>;
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
        #[cfg(not(feature = "bound-check-disabled"))]
        let size = match entry.encoded_length() {
            Ok(s) => s,
            Err(e) => {
                self.count = 0;
                return Some(Err(e));
            }
        };
        #[cfg(feature = "bound-check-disabled")]
        let size = entry.encoded_length().unwrap();
        self.pos += size;
        self.count -= 1;
        Some(Ok(entry))
    }
}
impl<'a> ExactSizeIterator for PerformanceFiguresDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct PerformanceFiguresEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
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
        }
    }
    #[inline]
    pub fn octane_rating(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const OCTANE_RATING_NULL: u8 = 255_u8;
    pub const OCTANE_RATING_MIN: u8 = 90_u8;
    pub const OCTANE_RATING_MAX: u8 = 110_u8;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: 4,
                available: self.buf.len() - start,
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
        let offset = self.tail_offset_0()?;
        PerformanceFiguresAccelerationDecoder::wrap(
            self.buf,
            offset,
            self.acting_version,
        )
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_1()? - self.pos)
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
        if pos + 4 > buf.len() {
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
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
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
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
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
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
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
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    pub const MPH_NULL: u16 = 65535_u16;
    pub const MPH_MIN: u16 = 0_u16;
    pub const MPH_MAX: u16 = 65534_u16;
    #[inline]
    pub fn seconds(&self) -> f32 {
        let offset = self.pos + 2;
        f32::from_le_bytes(read_bytes::<4>(self.buf, offset))
    }
    pub const SECONDS_NULL: f32 = f32::from_bits(2139095041u32);
    pub const SECONDS_MIN: f32 = f32::from_bits(4286578687u32);
    pub const SECONDS_MAX: f32 = f32::from_bits(2139095039u32);
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
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
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
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
    /// Header-inclusive bytes (for an entry, the entry bytes; header_size is 0).
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        &self.buf[self.pos - 0..self.tail_start]
    }
    /// Body length (excluding header).
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.tail_start - self.pos
    }
    /// Header-inclusive length.
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.tail_start - self.pos + 0
    }
}
pub struct CarDecoderAfterFuelFigures<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct CarDecoderAfterPerformanceFigures<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct CarDecoderAfterManufacturer<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct CarDecoderAfterModel<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct CarDecoderComplete<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
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
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "manufacturer",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "manufacturer",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = CarDecoderAfterManufacturer {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
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
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "model",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "model",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = CarDecoderAfterModel {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
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
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "activationCode",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "activationCode",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = CarDecoderComplete {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
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
    /// Header-inclusive bytes (for an entry, the entry bytes; header_size is 0).
    #[inline]
    pub fn as_bytes(&self) -> &'a [u8] {
        &self.buf[self.pos - 8..self.tail_start]
    }
    /// Body length (excluding header).
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.tail_start - self.pos
    }
    /// Header-inclusive length.
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.tail_start - self.pos + 8
    }
}
/// Owned domain object — application-layer counterpart to the flyweight decoder.
/// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CarFuelFiguresEntryDomain {
    pub speed: u16,
    pub mpg: f32,
    pub usage_description: Vec<u8>,
}
impl<'a> From<FuelFiguresEntryDecoder<'a>> for CarFuelFiguresEntryDomain {
    fn from(dec: FuelFiguresEntryDecoder<'a>) -> Self {
        Self {
            speed: dec.speed(),
            mpg: dec.mpg(),
            usage_description: dec.usage_description().unwrap_or(&[]).to_vec(),
        }
    }
}
/// Owned domain object — application-layer counterpart to the flyweight decoder.
/// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CarPerformanceFiguresEntryAccelerationEntryDomain {
    pub mph: u16,
    pub seconds: f32,
}
impl<'a> From<PerformanceFiguresAccelerationEntryDecoder<'a>>
for CarPerformanceFiguresEntryAccelerationEntryDomain {
    fn from(dec: PerformanceFiguresAccelerationEntryDecoder<'a>) -> Self {
        Self {
            mph: dec.mph(),
            seconds: dec.seconds(),
        }
    }
}
/// Owned domain object — application-layer counterpart to the flyweight decoder.
/// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CarPerformanceFiguresEntryDomain {
    pub octane_rating: u8,
    pub acceleration: Vec<CarPerformanceFiguresEntryAccelerationEntryDomain>,
}
impl<'a> From<PerformanceFiguresEntryDecoder<'a>> for CarPerformanceFiguresEntryDomain {
    fn from(dec: PerformanceFiguresEntryDecoder<'a>) -> Self {
        Self {
            octane_rating: dec.octane_rating(),
            acceleration: dec
                .acceleration()
                .map(|g| {
                    g
                        .map(CarPerformanceFiguresEntryAccelerationEntryDomain::from)
                        .collect()
                })
                .unwrap_or_default(),
        }
    }
}
/// Owned domain object — application-layer counterpart to the flyweight decoder.
/// Use `MsgDomain::from(decoder)` or `decoder.into()` to convert.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct CarDomain {
    pub serial_number: u64,
    pub model_year: u16,
    pub available: BooleanType,
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
impl<'a> From<CarDecoder<'a>> for CarDomain {
    fn from(dec: CarDecoder<'a>) -> Self {
        Self {
            serial_number: dec.serial_number(),
            model_year: dec.model_year(),
            available: dec.available(),
            code: dec.code(),
            some_numbers: dec.some_numbers(),
            vehicle_code: dec.vehicle_code(),
            extras: dec.extras(),
            engine: dec.engine_as_struct(),
            fuel_figures: dec
                .fuel_figures()
                .map(|g| {
                    g
                        .filter_map(|e| e.ok())
                        .map(CarFuelFiguresEntryDomain::from)
                        .collect()
                })
                .unwrap_or_default(),
            performance_figures: dec
                .performance_figures()
                .map(|g| {
                    g
                        .filter_map(|e| e.ok())
                        .map(CarPerformanceFiguresEntryDomain::from)
                        .collect()
                })
                .unwrap_or_default(),
            manufacturer: dec.manufacturer().unwrap_or(&[]).to_vec(),
            model: dec.model().unwrap_or(&[]).to_vec(),
            activation_code: dec.activation_code().unwrap_or(&[]).to_vec(),
        }
    }
}
///Description of a basic Car
#[must_use = "encoder must be consumed to write the message"]
pub struct CarEncoder<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterFuelFigures<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterPerformanceFigures<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterManufacturer<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarAfterModel<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarComplete<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
impl<'a> CarEncoder<'a> {
    pub const SCHEMA_ID: u16 = 1;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 41;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 41);
    ///MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [41, 0, 1, 0, 1, 0, 0, 0];
    const _HEADER_TEMPLATE_LEN: () = assert!(Self::HEADER_TEMPLATE.len() == 8);
    /// Wrap a mutable buffer for encoding. Returns an error if the buffer
    /// is too short for the header + fixed block.
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Result<Self, sbe_rt::EncodeError> {
        let needed: usize = 8 + Self::BLOCK_LENGTH;
        let available: usize = buf.len().saturating_sub(pos);
        if available < needed {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available,
            });
        }
        Ok(Self {
            buf: &mut buf[pos..],
            message_start: 0,
            pos: needed,
        })
    }
    /// Wrap a mutable buffer and write the SBE message header.
    /// Returns an error if the buffer is too short.
    #[inline]
    pub fn wrap_and_apply_header(
        buf: &'a mut [u8],
        pos: usize,
    ) -> Result<Self, sbe_rt::EncodeError> {
        let needed: usize = 8 + Self::BLOCK_LENGTH;
        let available: usize = buf.len().saturating_sub(pos);
        if available < needed {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available,
            });
        }
        buf[pos..pos + 8].copy_from_slice(&Self::HEADER_TEMPLATE);
        Self::wrap(buf, pos)
    }
    #[must_use]
    #[inline]
    pub fn serial_number(&mut self, val: u64) -> &mut Self {
        let offset = 8;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    #[inline]
    pub fn model_year(&mut self, val: u16) -> &mut Self {
        let offset = 16;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn available(&mut self, val: BooleanType) -> &mut Self {
        let offset = 18;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    #[must_use]
    pub fn available_bool(&mut self, val: bool) -> &mut Self {
        let offset = 18;
        let enum_val: BooleanType = val.into();
        self.buf[offset..offset + 1].copy_from_slice(&(enum_val as u8).to_le_bytes());
        self
    }
    #[must_use]
    pub fn code(&mut self, val: Model) -> &mut Self {
        let offset = 19;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    #[must_use]
    #[inline]
    pub fn some_numbers(&mut self, val: [u32; 4]) -> &mut Self {
        let offset = 20;
        let mut idx = 0usize;
        while idx < 4 {
            self.buf[offset + idx * 4..offset + (idx + 1) * 4]
                .copy_from_slice(&val[idx].to_le_bytes());
            idx += 1;
        }
        self
    }
    #[must_use]
    #[inline]
    pub fn vehicle_code(&mut self, val: [u8; 6]) -> &mut Self {
        self.buf[36..][..6].copy_from_slice(&val);
        self
    }
    #[must_use]
    pub fn extras(&mut self, val: OptionalExtras) -> &mut Self {
        let offset = 42;
        self.buf[offset..offset + 1].copy_from_slice(&val.0.to_le_bytes());
        self
    }
    #[must_use]
    pub fn engine(&mut self, val: Engine) -> &mut Self {
        let offset = 43;
        self.buf[offset..offset + 6].copy_from_slice(&val.0);
        self
    }
    /// Return the encoded bytes written so far (partial — available before
    /// the tail is complete, for scalar-only inspection).
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.message_start..self.pos]
    }
    /// Compute the exact SBE message body length before encoding.
    /// Parameters: one `usize` per group (entry count) and one `usize` per var-data field (byte length).
    #[inline]
    pub const fn compute_encoded_length(
        fuel_figures_count: usize,
        performance_figures_count: usize,
        manufacturer_len: usize,
        model_len: usize,
        activation_code_len: usize,
    ) -> usize {
        let mut len = 41;
        len += 4 + fuel_figures_count * 6;
        len += 4 + performance_figures_count * 1;
        len += 4 + manufacturer_len;
        len += 4 + model_len;
        len += 4 + activation_code_len;
        len
    }
    /// Compute the exact SBE message length including the standard
    /// message header (header size + body). DECISIONS.md §2: callers
    /// must use this — not a hand-written `+ 8`.
    #[inline]
    pub const fn compute_encoded_length_with_message_header(
        fuel_figures_count: usize,
        performance_figures_count: usize,
        manufacturer_len: usize,
        model_len: usize,
        activation_code_len: usize,
    ) -> usize {
        8usize
            + Self::compute_encoded_length(
                fuel_figures_count,
                performance_figures_count,
                manufacturer_len,
                model_len,
                activation_code_len,
            )
    }
}
impl<'a> CarEncoder<'a> {
    #[must_use]
    pub fn fuel_figures<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<CarAfterFuelFigures<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut FuelFiguresEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&FuelFiguresEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = FuelFiguresEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(CarAfterFuelFigures {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> CarAfterFuelFigures<'a> {
    #[must_use]
    pub fn performance_figures<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<CarAfterPerformanceFigures<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&PerformanceFiguresEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = PerformanceFiguresEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(CarAfterPerformanceFigures {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> CarAfterPerformanceFigures<'a> {
    #[must_use]
    pub fn manufacturer(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterManufacturer<'a>, sbe_rt::EncodeError> {
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
                available: self.buf.len() - self.pos,
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterManufacturer {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn manufacturer_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterManufacturer<'a>, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len() - self.pos,
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterManufacturer {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
}
impl<'a> CarAfterManufacturer<'a> {
    #[must_use]
    pub fn model(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterModel<'a>, sbe_rt::EncodeError> {
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
                available: self.buf.len() - self.pos,
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterModel {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn model_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarAfterModel<'a>, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len() - self.pos,
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarAfterModel {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
}
impl<'a> CarAfterModel<'a> {
    #[must_use]
    pub fn activation_code(
        mut self,
        data: &[u8],
    ) -> Result<CarComplete<'a>, sbe_rt::EncodeError> {
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
                available: self.buf.len() - self.pos,
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn activation_code_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarComplete<'a>, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len() - self.pos,
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(CarComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
}
impl<'a> CarComplete<'a> {
    /// Returns the complete SBE message bytes (header + body).
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.message_start..self.pos]
    }
    /// Explicit header-inclusive view (alias for `as_bytes()`).
    /// DECISIONS.md §2: use this when header inclusion must be
    /// explicit rather than implied by the complete stage.
    #[inline]
    pub fn as_bytes_with_header(&self) -> &[u8] {
        self.as_bytes()
    }
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.pos - self.message_start - 8
    }
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.pos - self.message_start
    }
}
impl<'a> AsRef<[u8]> for CarComplete<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a> sbe_rt::private::Sealed for CarEncoder<'a> {}
impl<'a> sbe_rt::SbeMessage for CarEncoder<'a> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 41;
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
    #[must_use]
    pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(&mut FuelFiguresEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: self.written as u32 + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut __entry = FuelFiguresEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
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
    #[must_use]
    pub fn speed(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn mpg(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        self.buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn usage_description(
        &mut self,
        data: &[u8],
    ) -> Result<&mut Self, sbe_rt::EncodeError> {
        let needed = 4 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len() - self.pos,
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
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
    #[must_use]
    pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: self.written as u32 + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut __entry = PerformanceFiguresEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
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
    #[must_use]
    pub fn octane_rating(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn acceleration<F>(
        &mut self,
        count: u16,
        f: F,
    ) -> Result<&mut Self, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresAccelerationEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
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
            f(&mut group);
            __pos = group.pos;
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
    #[must_use]
    pub fn add<'b, F>(&'b mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresAccelerationEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count as u32,
                attempted: self.written as u32 + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut __entry = PerformanceFiguresAccelerationEntryEncoder::wrap(
                __buf,
                self.pos,
            );
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
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
    #[must_use]
    pub fn mph(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn seconds(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        self.buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        self
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
pub const SEMANTIC_VERSION: &str = "5.2";
pub const SCHEMA_HASH: u64 = 11133254787130522899;
pub const SCHEMA_SHA256: [u8; 32] = [
    0xad, 0xf6, 0x38, 0xad, 0x84, 0x97, 0xf8, 0x3b, 0x2b, 0x0b, 0x28, 0x50, 0x2e, 0xb2,
    0xd2, 0x4f, 0xea, 0x41, 0xd7, 0xfa, 0x6d, 0x21, 0x55, 0x2e, 0xcd, 0xba, 0xc2, 0x4b,
    0x35, 0x70, 0x74, 0x80,
];
pub const SCHEMA_SHA256_HEX: &str = "adf638ad8497f83b2b0b28502eb2d24fea41d7fa6d21552ecdbac24b35707480";
pub const SCHEMA_ID: u16 = 1;
pub const SCHEMA_VERSION: u16 = 0;
pub mod prelude {
    pub use super::sbe_rt::{DecodeError, EncodeError, VerifyError, SbeMessage};
    pub use super::{
        AnyMessage, DecodedFrame, FrameCursor, FramingPolicy, MessageVisitor,
        MessageHeader, MessageHeaderDecoder, GroupSizeEncoding, GroupSizeEncodingDecoder,
        VarStringEncoding, VarStringEncodingDecoder, VarAsciiEncoding,
        VarAsciiEncodingDecoder, VarDataEncoding, VarDataEncodingDecoder, Booster,
        BoosterDecoder, Engine, EngineDecoder, BooleanType, Model, OptionalExtras,
        CarDecoder, CarEncoder,
    };
}
/// Read `N` bytes from `buf` at `offset` into a fixed-size array.
///
/// Safe path uses slice indexing (bounds-checked, equivalent to Aeron's
/// `slice[index..index+N].try_into()`). With `bound-check-disabled`,
/// uses `core::ptr::read_unaligned` for zero-overhead access.
#[inline]
pub fn read_bytes<const N: usize>(buf: &[u8], offset: usize) -> [u8; N] {
    #[cfg(not(feature = "bound-check-disabled"))]
    { buf[offset..offset + N].try_into().expect("read_bytes: buffer too short") }
    #[cfg(feature = "bound-check-disabled")]
    unsafe { core::ptr::read_unaligned(buf.as_ptr().add(offset) as *const [u8; N]) }
}
/// Write `N` bytes from `bytes` into `buf` at `offset`.
///
/// Safe path uses `copy_from_slice`. With `bound-check-disabled`,
/// uses `core::ptr::write_unaligned` for zero-overhead write.
#[inline]
pub fn write_bytes<const N: usize>(buf: &mut [u8], offset: usize, bytes: &[u8; N]) {
    #[cfg(not(feature = "bound-check-disabled"))]
    {
        buf[offset..offset + N].copy_from_slice(bytes);
    }
    #[cfg(feature = "bound-check-disabled")]
    unsafe {
        core::ptr::write_unaligned(buf.as_mut_ptr().add(offset) as *mut [u8; N], *bytes);
    }
}
#[inline]
pub fn schema_id_from_header(buf: &[u8]) -> Option<u16> {
    if buf.len() < 4 + 2 {
        return None;
    }
    let bytes = read_bytes::<2>(buf, 4);
    Some(u16::from_le_bytes(bytes))
}
#[non_exhaustive]
#[derive(Clone, Copy)]
pub enum AnyMessage<'a> {
    Car(CarDecoder<'a>),
    Unknown { header: MessageHeader, payload: &'a [u8] },
}
#[derive(Clone)]
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
                if self.pos + 4 > self.buf.len() {
                    return Some(
                        Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "length prefix",
                            needed: 4,
                            available: self.buf.len() - self.pos,
                        }),
                    );
                }
                let bytes: [u8; 4] = read_bytes::<4>(self.buf, self.pos);
                let len = u32::from_le_bytes(bytes) as usize;
                (4, len)
            }
            FramingPolicy::LengthPrefixU16 => {
                if self.pos + 2 > self.buf.len() {
                    return Some(
                        Err(sbe_rt::DecodeError::BufferTooShort {
                            field: "length prefix",
                            needed: 2,
                            available: self.buf.len() - self.pos,
                        }),
                    );
                }
                let bytes: [u8; 2] = read_bytes::<2>(self.buf, self.pos);
                let len = u16::from_le_bytes(bytes) as usize;
                (2, len)
            }
            FramingPolicy::Fixed(len) => (0, len),
        };
        if self.pos + header_len + frame_len > self.buf.len() {
            return Some(
                Err(sbe_rt::DecodeError::BufferTooShort {
                    field: "frame bounds",
                    needed: header_len + frame_len,
                    available: self.buf.len() - self.pos,
                }),
            );
        }
        let off = self.pos + header_len;
        let res = AnyMessage::decode_frame(self.buf, off, frame_len);
        match res {
            Ok(frame) => {
                self.pos += header_len + frame_len;
                Some(Ok(frame))
            }
            Err(e) => Some(Err(e)),
        }
    }
}
impl<'a> AnyMessage<'a> {
    #[inline]
    pub fn decode(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
        if pos + 8 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: buf.len() - pos,
            });
        }
        let header_bytes = read_bytes::<8>(buf, pos);
        let header = MessageHeader(header_bytes);
        let template_id = header.template_id();
        let schema_id = header.schema_id();
        let version = header.version();
        let block_length = header.block_length() as usize;
        let body_pos = pos + 8;
        if schema_id != 1 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 1,
                actual: schema_id,
                expected_name: "baseline",
            });
        }
        match template_id {
            1 => Ok(Self::Car(CarDecoder::wrap(buf, body_pos, block_length, version))),
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
        if pos + 8 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: buf.len().saturating_sub(pos),
            });
        }
        let header_bytes: [u8; 8] = read_bytes::<8>(buf, pos);
        let header = MessageHeader(header_bytes);
        let template_id = header.template_id();
        let schema_id = header.schema_id();
        let version = header.version();
        let block_length = header.block_length() as usize;
        let body_pos = pos + 8;
        if schema_id != 1 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 1,
                actual: schema_id,
                expected_name: "baseline",
            });
        }
        match template_id {
            1 => {
                let decoder = CarDecoder::wrap(buf, body_pos, block_length, version);
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
                if pos + frame_len > buf.len() {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "template body",
                        needed: frame_len,
                        available: buf.len() - pos,
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
    #[inline]
    pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        match self {
            Self::Car(d) => d.as_bytes(),
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
                buf[..len].copy_from_slice(d.as_bytes()?);
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
}
impl<'a> AnyMessage<'a> {
    pub fn visit<V: MessageVisitor>(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::Car(d) => visitor.visit_car(d),
            Self::Unknown { .. } => unimplemented!(),
        }
    }
}

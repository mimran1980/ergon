/// Generated from SBE schema package `normalized_app` id 92 version 0.
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
                        f,
                        "field '{}': needed {} bytes, {} available",
                        field,
                        needed,
                        available,
                    )
                }
                Self::WrongSchema { expected, actual, expected_name } => {
                    write!(
                        f,
                        "wrong schema: expected id {} ({}), got id {}",
                        expected,
                        expected_name,
                        actual,
                    )
                }
                Self::UnknownTemplateLength { template_id } => {
                    write!(
                        f,
                        "unknown template id {}: SBE messages do not carry length. Use decode_frame() with an external frame length.",
                        template_id,
                    )
                }
                Self::InvalidVarDataLength { field, length, max_length } => {
                    write!(
                        f,
                        "var data field '{}: length {} exceeds max {}",
                        field,
                        length,
                        max_length,
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
                        f,
                        "buffer too short: needed {}, available {}",
                        needed,
                        available,
                    )
                }
                Self::VarDataTooLong { field, max_length, actual } => {
                    write!(
                        f,
                        "var data too long for field {}: max {}, actual {}",
                        field,
                        max_length,
                        actual,
                    )
                }
                Self::GroupFull { declared, attempted } => {
                    write!(
                        f,
                        "group full: declared count {}, attempted to write {}",
                        declared,
                        attempted,
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
                        f,
                        "invalid block length: expected at least {}, actual {}",
                        expected_min,
                        actual,
                    )
                }
                Self::GroupDimOutOfBounds { field, offset } => {
                    write!(
                        f,
                        "group dimension header for '{}' out of bounds at offset {}",
                        field,
                        offset,
                    )
                }
                Self::VarDataOutOfBounds { field, offset, length } => {
                    write!(
                        f,
                        "var-data for '{}' out of bounds at offset {} with length {}",
                        field,
                        offset,
                        length,
                    )
                }
                Self::MessageTooShort { needed, available } => {
                    write!(
                        f,
                        "message too short: needed {} bytes, {} available",
                        needed,
                        available,
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
pub trait SbeDecimal: Sized {
    type Error;
    fn try_from_sbe(mantissa: i64, exponent: i8) -> Result<Self, Self::Error>;
    fn try_into_sbe(self) -> Result<(i64, i8), Self::Error>;
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Source {
    Bitget = 1,
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal,
}
impl Source {
    pub fn raw(self) -> u8 {
        self as u8
    }
    pub const fn from_raw(val: u8) -> Self {
        match val {
            1 => Self::Bitget,
            _ => Self::NullVal,
        }
    }
}
impl From<Source> for u8 {
    #[inline]
    fn from(val: Source) -> Self {
        val as u8
    }
}
impl From<u8> for Source {
    #[inline]
    fn from(val: u8) -> Self {
        Self::from_raw(val)
    }
}
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub enum Side {
    Buy = 1,
    Sell = 2,
    /// Unknown enum value — the wire discriminant did not match any known variant.
    NullVal,
}
impl Side {
    pub fn raw(self) -> u8 {
        self as u8
    }
    pub const fn from_raw(val: u8) -> Self {
        match val {
            1 => Self::Buy,
            2 => Self::Sell,
            _ => Self::NullVal,
        }
    }
}
impl From<Side> for u8 {
    #[inline]
    fn from(val: Side) -> Self {
        val as u8
    }
}
impl From<u8> for Side {
    #[inline]
    fn from(val: u8) -> Self {
        Self::from_raw(val)
    }
}
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
const _: () = assert!(core::mem::size_of::<MessageHeader>() == 8);
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
const _: () = assert!(core::mem::size_of::<GroupSizeEncoding>() == 4);
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
const _: () = assert!(core::mem::size_of::<VarDataEncoding>() == 4);
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
const _: () = assert!(core::mem::size_of::<VarStringEncoding>() == 4);
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct Decimal(pub [u8; 9]);
impl Decimal {
    #[inline]
    pub fn mantissa(&self) -> i64 {
        i64::from_le_bytes(read_bytes::<8>(&self.0, 0))
    }
    #[inline]
    pub fn exponent(&self) -> i8 {
        i8::from_le_bytes(read_bytes::<1>(&self.0, 8))
    }
    pub fn new(mantissa: i64, exponent: i8) -> Self {
        let mut bytes = [0u8; 9];
        let val_bytes = mantissa.to_le_bytes();
        write_bytes::<8>(&mut bytes, 0, &val_bytes);
        let val_bytes = exponent.to_le_bytes();
        write_bytes::<1>(&mut bytes, 8, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of::<Decimal>() == 9);
#[derive(Clone, Copy)]
pub struct DecimalDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}
impl<'a> DecimalDecoder<'a> {
    #[inline]
    pub fn mantissa(&self) -> i64 {
        let offset = self.pos + 0;
        i64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    #[inline]
    pub fn exponent(&self) -> i8 {
        let offset = self.pos + 8;
        i8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
}
///attr:AppMessage description-child:AppMessage comment-child:AppMessage xml-comment:AppMessage
#[derive(Clone, Copy)]
pub struct AppMessageDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> AppMessageDecoder<'a> {
    pub const SCHEMA_ID: u16 = 92;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 8;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 8);
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
                expected_name: "normalized_app",
            });
        }
        if header.schema_id() != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: header.schema_id(),
                expected_name: "normalized_app",
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
    pub fn sent_ts(&self) -> u64 {
        let offset = self.pos + 0;
        u64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const SENT_TS_NULL: u64 = 18446744073709551615_u64;
    pub const SENT_TS_MIN: u64 = 0_u64;
    pub const SENT_TS_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "appName",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "appName",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn tail_offset_2(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_1()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "payload",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarDataEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "payload",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn app_name(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(app_name),
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn app_name_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.app_name()?;
        core::str::from_utf8(bytes).map_err(sbe_rt::DecodeError::Utf8)
    }
    #[inline]
    fn payload(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarDataEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(payload),
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn payload_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.payload()?;
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
        let end = self.tail_offset_2()?;
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
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "app_name",
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
                    field: "app_name",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        {
            if offset + 4 > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "payload",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let var_header = VarDataEncoding(bytes);
            let len = var_header.length();
            let data_end = offset + 4 + len as usize;
            if data_end > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "payload",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        Ok(())
    }
}
impl<'a> TryFrom<&'a [u8]> for AppMessageDecoder<'a> {
    type Error = sbe_rt::DecodeError;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::wrap_and_apply_header(buf, 0)
    }
}
impl<'a> sbe_rt::private::Sealed for AppMessageDecoder<'a> {}
impl<'a> sbe_rt::SbeMessage for AppMessageDecoder<'a> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 8;
    const SCHEMA_ID: u16 = 92;
    const SCHEMA_VERSION: u16 = 0;
}
impl<'a> AsRef<[u8]> for AppMessageDecoder<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes().unwrap_or(&[])
    }
}
impl<'a> AppMessageDecoder<'a> {
    pub fn as_ref_opt(&self) -> Option<&[u8]> {
        self.as_bytes().ok()
    }
}
impl<'a> core::fmt::Display for AppMessageDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "AppMessage {{ ")?;
        {
            let v = self.sent_ts();
            write!(f, "sent_ts: {:?}", v)?;
        }
        if let Ok(d) = self.app_name() {
            write!(f, ", app_name: {} bytes", d.len())?;
        }
        if let Ok(d) = self.payload() {
            write!(f, ", payload: {} bytes", d.len())?;
        }
        write!(f, " }}")
    }
}
pub struct AppMessageDecoderAfterAppName<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct AppMessageDecoderComplete<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> AppMessageDecoderAfterAppName<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> AppMessageDecoderComplete<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> AppMessageDecoder<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_app_name(
        self,
    ) -> Result<(&'a [u8], AppMessageDecoderAfterAppName<'a>), sbe_rt::DecodeError> {
        let offset = self.pos + self.acting_block_length;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "appName",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "appName",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "appName",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = AppMessageDecoderAfterAppName {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
}
impl<'a> AppMessageDecoder<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_app_name_as_message(
        self,
    ) -> Result<
        (DecodedFrame<'a>, AppMessageDecoderAfterAppName<'a>),
        sbe_rt::DecodeError,
    > {
        let (data, next) = self.into_app_name()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
}
impl<'a> AppMessageDecoder<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_app_name<E, F>(self, f: F) -> Result<AppMessageDecoderAfterAppName<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_app_name()?;
        f(data)?;
        Ok(next)
    }
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
    #[inline]
    pub fn try_app_name_as_message<E, F>(
        self,
        f: F,
    ) -> Result<AppMessageDecoderAfterAppName<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_app_name_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> AppMessageDecoderAfterAppName<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_payload(
        self,
    ) -> Result<(&'a [u8], AppMessageDecoderComplete<'a>), sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "payload",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarDataEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "payload",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "payload",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = AppMessageDecoderComplete {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
}
impl<'a> AppMessageDecoderAfterAppName<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_payload_as_message(
        self,
    ) -> Result<(DecodedFrame<'a>, AppMessageDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (data, next) = self.into_payload()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
}
impl<'a> AppMessageDecoderAfterAppName<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_payload<E, F>(self, f: F) -> Result<AppMessageDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_payload()?;
        f(data)?;
        Ok(next)
    }
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
    #[inline]
    pub fn try_payload_as_message<E, F>(
        self,
        f: F,
    ) -> Result<AppMessageDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_payload_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> AppMessageDecoderComplete<'a> {
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
///attr:AppMessage description-child:AppMessage comment-child:AppMessage xml-comment:AppMessage
#[must_use = "encoder must be consumed to write the message"]
pub struct AppMessageEncoder<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct AppMessageAfterAppName<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct AppMessageComplete<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
impl<'a> AppMessageEncoder<'a> {
    pub const SCHEMA_ID: u16 = 92;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 8;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 8);
    ///MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [8, 0, 1, 0, 92, 0, 0, 0];
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
    pub fn sent_ts(&mut self, val: u64) -> &mut Self {
        let offset = 8;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
    /// Compute the exact SBE message body length before encoding.
    /// Parameters: one `usize` per group (entry count) and one `usize` per var-data field (byte length).
    #[inline]
    pub const fn compute_encoded_length(
        app_name_len: usize,
        payload_len: usize,
    ) -> usize {
        let mut len = 8;
        len += 4 + app_name_len;
        len += 4 + payload_len;
        len
    }
    /// Compute the exact SBE message length including the standard
    /// message header (header size + body). DECISIONS.md §2: callers
    /// must use this — not a hand-written `+ 8`.
    #[inline]
    pub const fn compute_encoded_length_with_message_header(
        app_name_len: usize,
        payload_len: usize,
    ) -> usize {
        8usize + Self::compute_encoded_length(app_name_len, payload_len)
    }
    /// Run a fallible closure over the fixed-body fields. The closure
    /// receives `&mut Self` and can set/read fixed fields; tail
    /// transitions are unavailable inside the closure. Returns the
    /// same stage on success, or the caller's error on failure.
    #[inline]
    pub fn try_fixed<E, F>(mut self, f: F) -> Result<Self, E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        f(&mut self)?;
        Ok(self)
    }
}
impl<'a> AppMessageEncoder<'a> {
    #[must_use]
    pub fn app_name(
        mut self,
        data: &[u8],
    ) -> Result<AppMessageAfterAppName<'a>, sbe_rt::EncodeError> {
        if data.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "appName",
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
        Ok(AppMessageAfterAppName {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn app_name_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<AppMessageAfterAppName<'a>, sbe_rt::EncodeError> {
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
        Ok(AppMessageAfterAppName {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    /// Lend exactly `exact_len` bytes of the var-data region
    /// to a closure for nested-message encoding. Zero-copy:
    /// the closure writes directly into the outer buffer.
    /// Returns the next stage on success; on failure the
    /// caller error propagates unchanged and no partial
    /// data is published.
    #[must_use]
    pub fn app_name_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<AppMessageAfterAppName<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 1073741824 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "appName",
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
                    available: self.buf.len() - self.pos,
                }
                    .into(),
            );
        }
        let len_bytes = (exact_len as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(AppMessageAfterAppName {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + exact_len,
        })
    }
}
impl<'a> AppMessageAfterAppName<'a> {
    #[must_use]
    pub fn payload(
        mut self,
        data: &[u8],
    ) -> Result<AppMessageComplete<'a>, sbe_rt::EncodeError> {
        if data.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "payload",
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
        Ok(AppMessageComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn payload_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<AppMessageComplete<'a>, sbe_rt::EncodeError> {
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
        Ok(AppMessageComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    /// Lend exactly `exact_len` bytes of the var-data region
    /// to a closure for nested-message encoding. Zero-copy:
    /// the closure writes directly into the outer buffer.
    /// Returns the next stage on success; on failure the
    /// caller error propagates unchanged and no partial
    /// data is published.
    #[must_use]
    pub fn payload_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<AppMessageComplete<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 1073741824 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "payload",
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
                    available: self.buf.len() - self.pos,
                }
                    .into(),
            );
        }
        let len_bytes = (exact_len as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(AppMessageComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + exact_len,
        })
    }
}
impl<'a> AppMessageComplete<'a> {
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
impl<'a> AsRef<[u8]> for AppMessageComplete<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a> sbe_rt::private::Sealed for AppMessageEncoder<'a> {}
impl<'a> sbe_rt::SbeMessage for AppMessageEncoder<'a> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 8;
    const SCHEMA_ID: u16 = 92;
    const SCHEMA_VERSION: u16 = 0;
}
pub mod app_message_field_meta {
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
            name: "sentTs",
            id: 1,
            offset: 0,
            since_version: 0,
            field_type: "u64",
            presence: "required",
            null_value: Some("18446744073709551615"),
            semantic_type: None,
            description: None,
        },
    ];
}
#[derive(Clone, Copy)]
pub struct L2BookDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> L2BookDecoder<'a> {
    pub const SCHEMA_ID: u16 = 92;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 2;
    pub const BLOCK_LENGTH: usize = 25;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 25);
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
                expected_name: "normalized_app",
            });
        }
        if header.schema_id() != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: header.schema_id(),
                expected_name: "normalized_app",
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
    pub fn source(&self) -> Source {
        let offset = self.pos + 0;
        Source::from_raw(u8::from_le_bytes(read_bytes::<1>(self.buf, offset)))
    }
    pub const SOURCE_NULL: Source = Source::NullVal;
    #[inline]
    pub fn exchange_timestamp(&self) -> u64 {
        let offset = self.pos + 1;
        u64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const EXCHANGE_TIMESTAMP_NULL: u64 = 18446744073709551615_u64;
    pub const EXCHANGE_TIMESTAMP_MIN: u64 = 0_u64;
    pub const EXCHANGE_TIMESTAMP_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    pub fn receive_timestamp(&self) -> u64 {
        let offset = self.pos + 9;
        u64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const RECEIVE_TIMESTAMP_NULL: u64 = 18446744073709551615_u64;
    pub const RECEIVE_TIMESTAMP_MIN: u64 = 0_u64;
    pub const RECEIVE_TIMESTAMP_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    pub fn sequence(&self) -> u64 {
        let offset = self.pos + 17;
        u64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const SEQUENCE_NULL: u64 = 18446744073709551615_u64;
    pub const SEQUENCE_MIN: u64 = 0_u64;
    pub const SEQUENCE_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "bids",
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
            pos = L2BookBidsEntryDecoder::skip(
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
                field: "asks",
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
            pos = L2BookAsksEntryDecoder::skip(
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
                field: "symbol",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbol",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn bids(&self) -> Result<L2BookBidsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        L2BookBidsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn asks(&self) -> Result<L2BookAsksDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        L2BookAsksDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn symbol(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(symbol),
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn symbol_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.symbol()?;
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
        let end = self.tail_offset_3()?;
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
                    field: "bids",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSizeEncoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 18;
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
                    field: "asks",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSizeEncoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 18;
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
                    field: "symbol",
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
                    field: "symbol",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        Ok(())
    }
}
impl<'a> TryFrom<&'a [u8]> for L2BookDecoder<'a> {
    type Error = sbe_rt::DecodeError;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::wrap_and_apply_header(buf, 0)
    }
}
impl<'a> sbe_rt::private::Sealed for L2BookDecoder<'a> {}
impl<'a> sbe_rt::SbeMessage for L2BookDecoder<'a> {
    const TEMPLATE_ID: u16 = 2;
    const BLOCK_LENGTH: usize = 25;
    const SCHEMA_ID: u16 = 92;
    const SCHEMA_VERSION: u16 = 0;
}
impl<'a> AsRef<[u8]> for L2BookDecoder<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes().unwrap_or(&[])
    }
}
impl<'a> L2BookDecoder<'a> {
    pub fn as_ref_opt(&self) -> Option<&[u8]> {
        self.as_bytes().ok()
    }
}
impl<'a> core::fmt::Display for L2BookDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "L2Book {{ ")?;
        {
            let e = self.source();
            write!(f, "source: Source::{e:?}")?;
        }
        {
            let v = self.exchange_timestamp();
            write!(f, ", exchange_timestamp: {:?}", v)?;
        }
        {
            let v = self.receive_timestamp();
            write!(f, ", receive_timestamp: {:?}", v)?;
        }
        {
            let v = self.sequence();
            write!(f, ", sequence: {:?}", v)?;
        }
        if let Ok(g) = self.bids() {
            write!(f, ", bids: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.asks() {
            write!(f, ", asks: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(d) = self.symbol() {
            write!(f, ", symbol: {} bytes", d.len())?;
        }
        write!(f, " }}")
    }
}
pub struct L2BookBidsDecoder<'a> {
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
impl<'a> L2BookBidsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 18;
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
                field: "bids",
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
impl<'a> L2BookBidsDecoder<'a> {
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
impl<'a> L2BookBidsDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "bids",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> L2BookBidsDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<L2BookBidsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "bids",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "bids",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            L2BookBidsEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for L2BookBidsDecoder<'a> {
    type Item = L2BookBidsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = L2BookBidsEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for L2BookBidsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct L2BookBidsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> L2BookBidsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 18;
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
    pub fn price(&self) -> DecimalDecoder<'_> {
        let offset = self.pos + 0;
        DecimalDecoder {
            buf: self.buf,
            pos: offset,
        }
    }
    #[inline]
    pub fn price_as_struct(&self) -> Decimal {
        let offset = self.pos + 0;
        Decimal(read_bytes::<9>(self.buf, offset))
    }
    #[inline]
    pub const fn raw_price(&self) -> Decimal {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 9];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 9)
            });
        Decimal(bytes)
    }
    #[inline]
    pub fn size(&self) -> DecimalDecoder<'_> {
        let offset = self.pos + 9;
        DecimalDecoder {
            buf: self.buf,
            pos: offset,
        }
    }
    #[inline]
    pub fn size_as_struct(&self) -> Decimal {
        let offset = self.pos + 9;
        Decimal(read_bytes::<9>(self.buf, offset))
    }
    #[inline]
    pub const fn raw_size(&self) -> Decimal {
        let offset = self.pos + 9;
        let mut bytes = [0u8; 9];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 9)
            });
        Decimal(bytes)
    }
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
impl<'a> core::fmt::Display for L2BookBidsEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        write!(f, " }}")
    }
}
pub struct L2BookAsksDecoder<'a> {
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
impl<'a> L2BookAsksDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 18;
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
                field: "asks",
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
impl<'a> L2BookAsksDecoder<'a> {
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
impl<'a> L2BookAsksDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "asks",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> L2BookAsksDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<L2BookAsksEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "asks",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "asks",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            L2BookAsksEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for L2BookAsksDecoder<'a> {
    type Item = L2BookAsksEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = L2BookAsksEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for L2BookAsksDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct L2BookAsksEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> L2BookAsksEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 18;
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
    pub fn price(&self) -> DecimalDecoder<'_> {
        let offset = self.pos + 0;
        DecimalDecoder {
            buf: self.buf,
            pos: offset,
        }
    }
    #[inline]
    pub fn price_as_struct(&self) -> Decimal {
        let offset = self.pos + 0;
        Decimal(read_bytes::<9>(self.buf, offset))
    }
    #[inline]
    pub const fn raw_price(&self) -> Decimal {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 9];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 9)
            });
        Decimal(bytes)
    }
    #[inline]
    pub fn size(&self) -> DecimalDecoder<'_> {
        let offset = self.pos + 9;
        DecimalDecoder {
            buf: self.buf,
            pos: offset,
        }
    }
    #[inline]
    pub fn size_as_struct(&self) -> Decimal {
        let offset = self.pos + 9;
        Decimal(read_bytes::<9>(self.buf, offset))
    }
    #[inline]
    pub const fn raw_size(&self) -> Decimal {
        let offset = self.pos + 9;
        let mut bytes = [0u8; 9];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 9)
            });
        Decimal(bytes)
    }
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
impl<'a> core::fmt::Display for L2BookAsksEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        write!(f, " }}")
    }
}
pub struct L2BookDecoderAfterBids<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct L2BookDecoderAfterAsks<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct L2BookDecoderComplete<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> L2BookDecoderAfterBids<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> L2BookDecoderAfterAsks<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> L2BookDecoderComplete<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> L2BookDecoder<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_bids(self) -> Result<L2BookBidsDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.pos + self.acting_block_length;
        L2BookBidsDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> L2BookDecoderAfterBids<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_asks(self) -> Result<L2BookAsksDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        L2BookAsksDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> L2BookDecoderAfterAsks<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_symbol(
        self,
    ) -> Result<(&'a [u8], L2BookDecoderComplete<'a>), sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbol",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "symbol",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbol",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = L2BookDecoderComplete {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
}
impl<'a> L2BookDecoderAfterAsks<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_symbol_as_message(
        self,
    ) -> Result<(DecodedFrame<'a>, L2BookDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (data, next) = self.into_symbol()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
}
impl<'a> L2BookDecoderAfterAsks<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_symbol<E, F>(self, f: F) -> Result<L2BookDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_symbol()?;
        f(data)?;
        Ok(next)
    }
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
    #[inline]
    pub fn try_symbol_as_message<E, F>(
        self,
        f: F,
    ) -> Result<L2BookDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_symbol_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> L2BookBidsDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(self) -> Result<L2BookDecoderAfterBids<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = L2BookBidsEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(L2BookDecoderAfterBids {
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
    ) -> Result<L2BookDecoderAfterBids<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> L2BookAsksDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(self) -> Result<L2BookDecoderAfterAsks<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = L2BookAsksEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(L2BookDecoderAfterAsks {
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
    ) -> Result<L2BookDecoderAfterAsks<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> L2BookDecoderComplete<'a> {
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
#[must_use = "encoder must be consumed to write the message"]
pub struct L2BookEncoder<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct L2BookAfterBids<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct L2BookAfterAsks<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct L2BookComplete<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
impl<'a> L2BookEncoder<'a> {
    pub const SCHEMA_ID: u16 = 92;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 2;
    pub const BLOCK_LENGTH: usize = 25;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 25);
    ///MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [25, 0, 2, 0, 92, 0, 0, 0];
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
    pub fn source(&mut self, val: Source) -> &mut Self {
        let offset = 8;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    #[must_use]
    #[inline]
    pub fn exchange_timestamp(&mut self, val: u64) -> &mut Self {
        let offset = 9;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    #[inline]
    pub fn receive_timestamp(&mut self, val: u64) -> &mut Self {
        let offset = 17;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    #[inline]
    pub fn sequence(&mut self, val: u64) -> &mut Self {
        let offset = 25;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
    /// Compute the exact SBE message body length before encoding.
    /// Parameters: one `usize` per group (entry count) and one `usize` per var-data field (byte length).
    #[inline]
    pub const fn compute_encoded_length(
        bids_count: usize,
        asks_count: usize,
        symbol_len: usize,
    ) -> usize {
        let mut len = 25;
        len += 4 + bids_count * 18;
        len += 4 + asks_count * 18;
        len += 4 + symbol_len;
        len
    }
    /// Compute the exact SBE message length including the standard
    /// message header (header size + body). DECISIONS.md §2: callers
    /// must use this — not a hand-written `+ 8`.
    #[inline]
    pub const fn compute_encoded_length_with_message_header(
        bids_count: usize,
        asks_count: usize,
        symbol_len: usize,
    ) -> usize {
        8usize + Self::compute_encoded_length(bids_count, asks_count, symbol_len)
    }
    /// Run a fallible closure over the fixed-body fields. The closure
    /// receives `&mut Self` and can set/read fixed fields; tail
    /// transitions are unavailable inside the closure. Returns the
    /// same stage on success, or the caller's error on failure.
    #[inline]
    pub fn try_fixed<E, F>(mut self, f: F) -> Result<Self, E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        f(&mut self)?;
        Ok(self)
    }
}
impl<'a> L2BookEncoder<'a> {
    #[must_use]
    pub fn bids<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<L2BookAfterBids<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut L2BookBidsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&L2BookBidsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = L2BookBidsEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(L2BookAfterBids {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_bids<E, F>(mut self, count: u16, f: F) -> Result<L2BookAfterBids<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut L2BookBidsEncoder<'a>) -> Result<(), E>,
    {
        if self.pos + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: 4,
                    available: self.buf.len() - self.pos,
                }
                    .into(),
            );
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&L2BookBidsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = L2BookBidsEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group)?;
        Ok(L2BookAfterBids {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> L2BookAfterBids<'a> {
    #[must_use]
    pub fn asks<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<L2BookAfterAsks<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut L2BookAsksEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&L2BookAsksEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = L2BookAsksEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(L2BookAfterAsks {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_asks<E, F>(mut self, count: u16, f: F) -> Result<L2BookAfterAsks<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut L2BookAsksEncoder<'a>) -> Result<(), E>,
    {
        if self.pos + 4 > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed: 4,
                    available: self.buf.len() - self.pos,
                }
                    .into(),
            );
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&L2BookAsksEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = L2BookAsksEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group)?;
        Ok(L2BookAfterAsks {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> L2BookAfterAsks<'a> {
    #[must_use]
    pub fn symbol(
        mut self,
        data: &[u8],
    ) -> Result<L2BookComplete<'a>, sbe_rt::EncodeError> {
        if data.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "symbol",
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
        Ok(L2BookComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn symbol_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<L2BookComplete<'a>, sbe_rt::EncodeError> {
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
        Ok(L2BookComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    /// Lend exactly `exact_len` bytes of the var-data region
    /// to a closure for nested-message encoding. Zero-copy:
    /// the closure writes directly into the outer buffer.
    /// Returns the next stage on success; on failure the
    /// caller error propagates unchanged and no partial
    /// data is published.
    #[must_use]
    pub fn symbol_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<L2BookComplete<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 1073741824 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "symbol",
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
                    available: self.buf.len() - self.pos,
                }
                    .into(),
            );
        }
        let len_bytes = (exact_len as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(L2BookComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + exact_len,
        })
    }
}
impl<'a> L2BookComplete<'a> {
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
impl<'a> AsRef<[u8]> for L2BookComplete<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a> sbe_rt::private::Sealed for L2BookEncoder<'a> {}
impl<'a> sbe_rt::SbeMessage for L2BookEncoder<'a> {
    const TEMPLATE_ID: u16 = 2;
    const BLOCK_LENGTH: usize = 25;
    const SCHEMA_ID: u16 = 92;
    const SCHEMA_VERSION: u16 = 0;
}
#[must_use = "group encoder must call add() to write entries"]
pub struct L2BookBidsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> L2BookBidsEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 18;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [18, 0, 0, 0];
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
        F: FnOnce(&mut L2BookBidsEntryEncoder<'b>),
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
            let mut __entry = L2BookBidsEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct L2BookBidsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> L2BookBidsEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 18;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn price(&mut self, val: Decimal) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 9].copy_from_slice(&val.0);
        self
    }
    #[must_use]
    pub fn size(&mut self, val: Decimal) -> &mut Self {
        let offset = self.entry_start + 9;
        self.buf[offset..offset + 9].copy_from_slice(&val.0);
        self
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct L2BookAsksEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> L2BookAsksEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 18;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [18, 0, 0, 0];
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
        F: FnOnce(&mut L2BookAsksEntryEncoder<'b>),
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
            let mut __entry = L2BookAsksEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct L2BookAsksEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> L2BookAsksEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 18;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn price(&mut self, val: Decimal) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 9].copy_from_slice(&val.0);
        self
    }
    #[must_use]
    pub fn size(&mut self, val: Decimal) -> &mut Self {
        let offset = self.entry_start + 9;
        self.buf[offset..offset + 9].copy_from_slice(&val.0);
        self
    }
}
pub mod l2_book_field_meta {
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
            name: "source",
            id: 1,
            offset: 0,
            since_version: 0,
            field_type: "Source",
            presence: "required",
            null_value: None,
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "exchangeTimestamp",
            id: 2,
            offset: 1,
            since_version: 0,
            field_type: "u64",
            presence: "required",
            null_value: Some("18446744073709551615"),
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "receiveTimestamp",
            id: 3,
            offset: 9,
            since_version: 0,
            field_type: "u64",
            presence: "required",
            null_value: Some("18446744073709551615"),
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "sequence",
            id: 4,
            offset: 17,
            since_version: 0,
            field_type: "u64",
            presence: "required",
            null_value: Some("18446744073709551615"),
            semantic_type: None,
            description: None,
        },
    ];
}
#[derive(Clone, Copy)]
pub struct TradeDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> TradeDecoder<'a> {
    pub const SCHEMA_ID: u16 = 92;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 3;
    pub const BLOCK_LENGTH: usize = 44;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 44);
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
                expected_name: "normalized_app",
            });
        }
        if header.schema_id() != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: header.schema_id(),
                expected_name: "normalized_app",
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
    pub fn source(&self) -> Source {
        let offset = self.pos + 0;
        Source::from_raw(u8::from_le_bytes(read_bytes::<1>(self.buf, offset)))
    }
    pub const SOURCE_NULL: Source = Source::NullVal;
    #[inline]
    pub fn exchange_timestamp(&self) -> u64 {
        let offset = self.pos + 1;
        u64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const EXCHANGE_TIMESTAMP_NULL: u64 = 18446744073709551615_u64;
    pub const EXCHANGE_TIMESTAMP_MIN: u64 = 0_u64;
    pub const EXCHANGE_TIMESTAMP_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    pub fn receive_timestamp(&self) -> u64 {
        let offset = self.pos + 9;
        u64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const RECEIVE_TIMESTAMP_NULL: u64 = 18446744073709551615_u64;
    pub const RECEIVE_TIMESTAMP_MIN: u64 = 0_u64;
    pub const RECEIVE_TIMESTAMP_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    pub fn trade_id(&self) -> u64 {
        let offset = self.pos + 17;
        u64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const TRADE_ID_NULL: u64 = 18446744073709551615_u64;
    pub const TRADE_ID_MIN: u64 = 0_u64;
    pub const TRADE_ID_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    pub fn price(&self) -> DecimalDecoder<'_> {
        let offset = self.pos + 25;
        DecimalDecoder {
            buf: self.buf,
            pos: offset,
        }
    }
    #[inline]
    pub fn price_as_struct(&self) -> Decimal {
        let offset = self.pos + 25;
        Decimal(read_bytes::<9>(self.buf, offset))
    }
    #[inline]
    pub fn size(&self) -> DecimalDecoder<'_> {
        let offset = self.pos + 34;
        DecimalDecoder {
            buf: self.buf,
            pos: offset,
        }
    }
    #[inline]
    pub fn size_as_struct(&self) -> Decimal {
        let offset = self.pos + 34;
        Decimal(read_bytes::<9>(self.buf, offset))
    }
    #[inline]
    pub fn side(&self) -> Side {
        let offset = self.pos + 43;
        Side::from_raw(u8::from_le_bytes(read_bytes::<1>(self.buf, offset)))
    }
    pub const SIDE_NULL: Side = Side::NullVal;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbol",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbol",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn symbol(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(symbol),
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn symbol_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.symbol()?;
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
        let end = self.tail_offset_1()?;
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
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "symbol",
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
                    field: "symbol",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        Ok(())
    }
}
impl<'a> TryFrom<&'a [u8]> for TradeDecoder<'a> {
    type Error = sbe_rt::DecodeError;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::wrap_and_apply_header(buf, 0)
    }
}
impl<'a> sbe_rt::private::Sealed for TradeDecoder<'a> {}
impl<'a> sbe_rt::SbeMessage for TradeDecoder<'a> {
    const TEMPLATE_ID: u16 = 3;
    const BLOCK_LENGTH: usize = 44;
    const SCHEMA_ID: u16 = 92;
    const SCHEMA_VERSION: u16 = 0;
}
impl<'a> AsRef<[u8]> for TradeDecoder<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes().unwrap_or(&[])
    }
}
impl<'a> TradeDecoder<'a> {
    pub fn as_ref_opt(&self) -> Option<&[u8]> {
        self.as_bytes().ok()
    }
}
impl<'a> core::fmt::Display for TradeDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Trade {{ ")?;
        {
            let e = self.source();
            write!(f, "source: Source::{e:?}")?;
        }
        {
            let v = self.exchange_timestamp();
            write!(f, ", exchange_timestamp: {:?}", v)?;
        }
        {
            let v = self.receive_timestamp();
            write!(f, ", receive_timestamp: {:?}", v)?;
        }
        {
            let v = self.trade_id();
            write!(f, ", trade_id: {:?}", v)?;
        }
        {
            let e = self.side();
            write!(f, ", side: Side::{e:?}")?;
        }
        if let Ok(d) = self.symbol() {
            write!(f, ", symbol: {} bytes", d.len())?;
        }
        write!(f, " }}")
    }
}
pub struct TradeDecoderComplete<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> TradeDecoderComplete<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> TradeDecoder<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_symbol(
        self,
    ) -> Result<(&'a [u8], TradeDecoderComplete<'a>), sbe_rt::DecodeError> {
        let offset = self.pos + self.acting_block_length;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbol",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "symbol",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbol",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = TradeDecoderComplete {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
}
impl<'a> TradeDecoder<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_symbol_as_message(
        self,
    ) -> Result<(DecodedFrame<'a>, TradeDecoderComplete<'a>), sbe_rt::DecodeError> {
        let (data, next) = self.into_symbol()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
}
impl<'a> TradeDecoder<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_symbol<E, F>(self, f: F) -> Result<TradeDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_symbol()?;
        f(data)?;
        Ok(next)
    }
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
    #[inline]
    pub fn try_symbol_as_message<E, F>(self, f: F) -> Result<TradeDecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_symbol_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> TradeDecoderComplete<'a> {
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
#[must_use = "encoder must be consumed to write the message"]
pub struct TradeEncoder<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct TradeComplete<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
impl<'a> TradeEncoder<'a> {
    pub const SCHEMA_ID: u16 = 92;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 3;
    pub const BLOCK_LENGTH: usize = 44;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 44);
    ///MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [44, 0, 3, 0, 92, 0, 0, 0];
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
    pub fn source(&mut self, val: Source) -> &mut Self {
        let offset = 8;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    #[must_use]
    #[inline]
    pub fn exchange_timestamp(&mut self, val: u64) -> &mut Self {
        let offset = 9;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    #[inline]
    pub fn receive_timestamp(&mut self, val: u64) -> &mut Self {
        let offset = 17;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    #[inline]
    pub fn trade_id(&mut self, val: u64) -> &mut Self {
        let offset = 25;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn price(&mut self, val: Decimal) -> &mut Self {
        let offset = 33;
        self.buf[offset..offset + 9].copy_from_slice(&val.0);
        self
    }
    #[must_use]
    pub fn size(&mut self, val: Decimal) -> &mut Self {
        let offset = 42;
        self.buf[offset..offset + 9].copy_from_slice(&val.0);
        self
    }
    #[must_use]
    pub fn side(&mut self, val: Side) -> &mut Self {
        let offset = 51;
        self.buf[offset..offset + 1].copy_from_slice(&(val as u8).to_le_bytes());
        self
    }
    /// Compute the exact SBE message body length before encoding.
    /// Parameters: one `usize` per group (entry count) and one `usize` per var-data field (byte length).
    #[inline]
    pub const fn compute_encoded_length(symbol_len: usize) -> usize {
        let mut len = 44;
        len += 4 + symbol_len;
        len
    }
    /// Compute the exact SBE message length including the standard
    /// message header (header size + body). DECISIONS.md §2: callers
    /// must use this — not a hand-written `+ 8`.
    #[inline]
    pub const fn compute_encoded_length_with_message_header(symbol_len: usize) -> usize {
        8usize + Self::compute_encoded_length(symbol_len)
    }
    /// Run a fallible closure over the fixed-body fields. The closure
    /// receives `&mut Self` and can set/read fixed fields; tail
    /// transitions are unavailable inside the closure. Returns the
    /// same stage on success, or the caller's error on failure.
    #[inline]
    pub fn try_fixed<E, F>(mut self, f: F) -> Result<Self, E>
    where
        F: FnOnce(&mut Self) -> Result<(), E>,
    {
        f(&mut self)?;
        Ok(self)
    }
}
impl<'a> TradeEncoder<'a> {
    #[must_use]
    pub fn symbol(
        mut self,
        data: &[u8],
    ) -> Result<TradeComplete<'a>, sbe_rt::EncodeError> {
        if data.len() > 1073741824 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "symbol",
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
        Ok(TradeComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn symbol_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<TradeComplete<'a>, sbe_rt::EncodeError> {
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
        Ok(TradeComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    /// Lend exactly `exact_len` bytes of the var-data region
    /// to a closure for nested-message encoding. Zero-copy:
    /// the closure writes directly into the outer buffer.
    /// Returns the next stage on success; on failure the
    /// caller error propagates unchanged and no partial
    /// data is published.
    #[must_use]
    pub fn symbol_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<TradeComplete<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 1073741824 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "symbol",
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
                    available: self.buf.len() - self.pos,
                }
                    .into(),
            );
        }
        let len_bytes = (exact_len as u32).to_le_bytes();
        self.buf[self.pos..self.pos + 4].copy_from_slice(&len_bytes);
        let start = self.pos + 4;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(TradeComplete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + exact_len,
        })
    }
}
impl<'a> TradeComplete<'a> {
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
impl<'a> AsRef<[u8]> for TradeComplete<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a> sbe_rt::private::Sealed for TradeEncoder<'a> {}
impl<'a> sbe_rt::SbeMessage for TradeEncoder<'a> {
    const TEMPLATE_ID: u16 = 3;
    const BLOCK_LENGTH: usize = 44;
    const SCHEMA_ID: u16 = 92;
    const SCHEMA_VERSION: u16 = 0;
}
pub mod trade_field_meta {
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
            name: "source",
            id: 1,
            offset: 0,
            since_version: 0,
            field_type: "Source",
            presence: "required",
            null_value: None,
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "exchangeTimestamp",
            id: 2,
            offset: 1,
            since_version: 0,
            field_type: "u64",
            presence: "required",
            null_value: Some("18446744073709551615"),
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "receiveTimestamp",
            id: 3,
            offset: 9,
            since_version: 0,
            field_type: "u64",
            presence: "required",
            null_value: Some("18446744073709551615"),
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "tradeId",
            id: 4,
            offset: 17,
            since_version: 0,
            field_type: "u64",
            presence: "required",
            null_value: Some("18446744073709551615"),
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "price",
            id: 5,
            offset: 25,
            since_version: 0,
            field_type: "Decimal",
            presence: "required",
            null_value: None,
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "size",
            id: 6,
            offset: 34,
            since_version: 0,
            field_type: "Decimal",
            presence: "required",
            null_value: None,
            semantic_type: None,
            description: None,
        },
        FieldInfo {
            name: "side",
            id: 7,
            offset: 43,
            since_version: 0,
            field_type: "Side",
            presence: "required",
            null_value: None,
            semantic_type: None,
            description: None,
        },
    ];
}
pub const SCHEMA_HASH: u64 = 6577946258823559792;
pub const SCHEMA_SHA256: [u8; 32] = [
    0xe4, 0xc8, 0xa0, 0xc3, 0x7c, 0x7f, 0x57, 0x8b, 0xe6, 0xfa, 0x6f, 0x50, 0x0a, 0x95,
    0xd2, 0x1c, 0x7b, 0x5b, 0x3c, 0x06, 0x1a, 0x5f, 0xe7, 0x49, 0x82, 0x9b, 0x0a, 0xf7,
    0xf6, 0x87, 0xe0, 0x1f,
];
pub const SCHEMA_SHA256_HEX: &str = "e4c8a0c37c7f578be6fa6f500a95d21c7b5b3c061a5fe749829b0af7f687e01f";
pub const SCHEMA_ID: u16 = 92;
pub const SCHEMA_VERSION: u16 = 0;
pub mod prelude {
    pub use super::sbe_rt::{DecodeError, EncodeError, VerifyError, SbeMessage};
    pub use super::{
        AnyMessage, DecodedFrame, FrameCursor, FramingPolicy, MessageVisitor,
        MessageHeader, MessageHeaderDecoder, GroupSizeEncoding, GroupSizeEncodingDecoder,
        VarDataEncoding, VarDataEncodingDecoder, VarStringEncoding,
        VarStringEncodingDecoder, Decimal, DecimalDecoder, Source, Side,
        AppMessageDecoder, AppMessageEncoder, L2BookDecoder, L2BookEncoder, TradeDecoder,
        TradeEncoder,
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
    AppMessage(AppMessageDecoder<'a>),
    L2Book(L2BookDecoder<'a>),
    Trade(TradeDecoder<'a>),
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
        if schema_id != 92 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 92,
                actual: schema_id,
                expected_name: "normalized_app",
            });
        }
        match template_id {
            1 => {
                Ok(
                    Self::AppMessage(
                        AppMessageDecoder::wrap(buf, body_pos, block_length, version),
                    ),
                )
            }
            2 => {
                Ok(
                    Self::L2Book(
                        L2BookDecoder::wrap(buf, body_pos, block_length, version),
                    ),
                )
            }
            3 => {
                Ok(Self::Trade(TradeDecoder::wrap(buf, body_pos, block_length, version)))
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
        if schema_id != 92 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 92,
                actual: schema_id,
                expected_name: "normalized_app",
            });
        }
        match template_id {
            1 => {
                let decoder = AppMessageDecoder::wrap(
                    buf,
                    body_pos,
                    block_length,
                    version,
                );
                let total_len = decoder.encoded_length_with_header()?;
                if total_len > frame_len {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "AppMessage",
                        needed: total_len,
                        available: frame_len,
                    });
                }
                Ok(DecodedFrame {
                    message: Self::AppMessage(decoder),
                    range: pos..pos + total_len,
                    len: total_len,
                })
            }
            2 => {
                let decoder = L2BookDecoder::wrap(buf, body_pos, block_length, version);
                let total_len = decoder.encoded_length_with_header()?;
                if total_len > frame_len {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "L2Book",
                        needed: total_len,
                        available: frame_len,
                    });
                }
                Ok(DecodedFrame {
                    message: Self::L2Book(decoder),
                    range: pos..pos + total_len,
                    len: total_len,
                })
            }
            3 => {
                let decoder = TradeDecoder::wrap(buf, body_pos, block_length, version);
                let total_len = decoder.encoded_length_with_header()?;
                if total_len > frame_len {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "Trade",
                        needed: total_len,
                        available: frame_len,
                    });
                }
                Ok(DecodedFrame {
                    message: Self::Trade(decoder),
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
            Self::AppMessage(d) => d.encoded_length_with_header(),
            Self::L2Book(d) => d.encoded_length_with_header(),
            Self::Trade(d) => d.encoded_length_with_header(),
            Self::Unknown { payload, .. } => Ok(payload.len()),
        }
    }
}
impl<'a> AnyMessage<'a> {
    #[inline]
    pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        match self {
            Self::AppMessage(d) => d.as_bytes(),
            Self::L2Book(d) => d.as_bytes(),
            Self::Trade(d) => d.as_bytes(),
            Self::Unknown { payload, .. } => Ok(payload),
        }
    }
}
impl<'a> AnyMessage<'a> {
    #[inline]
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
        match self {
            Self::AppMessage(d) => {
                let len = d.encoded_length_with_header()?;
                buf[..len].copy_from_slice(d.as_bytes()?);
                Ok(len)
            }
            Self::L2Book(d) => {
                let len = d.encoded_length_with_header()?;
                buf[..len].copy_from_slice(d.as_bytes()?);
                Ok(len)
            }
            Self::Trade(d) => {
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
    fn visit_app_message(&mut self, decoder: &AppMessageDecoder<'_>) -> Self::Output;
    fn visit_l2_book(&mut self, decoder: &L2BookDecoder<'_>) -> Self::Output;
    fn visit_trade(&mut self, decoder: &TradeDecoder<'_>) -> Self::Output;
}
impl<'a> AnyMessage<'a> {
    pub fn visit<V: MessageVisitor>(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::AppMessage(d) => visitor.visit_app_message(d),
            Self::L2Book(d) => visitor.visit_l2_book(d),
            Self::Trade(d) => visitor.visit_trade(d),
            Self::Unknown { .. } => unimplemented!(),
        }
    }
}

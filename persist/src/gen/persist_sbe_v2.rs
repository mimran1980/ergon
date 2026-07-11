/// Generated from SBE schema package `org.ergo.sbe.persist.v2` id 1000 version 1.
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct GroupSize16Encoding(pub [u8; 4]);
impl GroupSize16Encoding {
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
const _: () = assert!(core::mem::size_of:: < GroupSize16Encoding > () == 4);
#[derive(Clone, Copy)]
pub struct GroupSize16EncodingDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}
impl<'a> GroupSize16EncodingDecoder<'a> {
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
pub struct VarString16Encoding(pub [u8; 2]);
impl VarString16Encoding {
    #[inline]
    pub fn length(&self) -> u16 {
        u16::from_le_bytes(read_bytes::<2>(&self.0, 0))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
    pub fn new(length: u16, var_data: [u8; 0]) -> Self {
        let mut bytes = [0u8; 2];
        let val_bytes = length.to_le_bytes();
        write_bytes::<2>(&mut bytes, 0, &val_bytes);
        Self(bytes)
    }
}
const _: () = assert!(core::mem::size_of:: < VarString16Encoding > () == 2);
#[derive(Clone, Copy)]
pub struct VarString16EncodingDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}
impl<'a> VarString16EncodingDecoder<'a> {
    #[inline]
    pub fn length(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    #[inline]
    pub fn var_data(&self) -> [u8; 0] {
        []
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[repr(transparent)]
pub struct SymbolTableEncoding(pub [u8; 4]);
impl SymbolTableEncoding {
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
const _: () = assert!(core::mem::size_of:: < SymbolTableEncoding > () == 4);
#[derive(Clone, Copy)]
pub struct SymbolTableEncodingDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
}
impl<'a> SymbolTableEncodingDecoder<'a> {
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
const _: () = assert!(core::mem::size_of:: < Decimal > () == 9);
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
///V2 schema with column type metadata
#[derive(Clone, Copy)]
pub struct DynamicSchemaV2Decoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicSchemaV2Decoder<'a> {
    pub const SCHEMA_ID: u16 = 1000;
    pub const SCHEMA_VERSION: u16 = 1;
    pub const TEMPLATE_ID: u16 = 3;
    pub const BLOCK_LENGTH: usize = 4;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 4);
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
                expected_name: "org.ergo.sbe.persist.v2",
            });
        }
        if header.schema_id() != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: header.schema_id(),
                expected_name: "org.ergo.sbe.persist.v2",
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
    pub fn schema_id(&self) -> u32 {
        let offset = self.pos + 0;
        u32::from_le_bytes(read_bytes::<4>(self.buf, offset))
    }
    pub const SCHEMA_ID_NULL: u32 = 4294967295_u32;
    pub const SCHEMA_ID_MIN: u32 = 0_u32;
    pub const SCHEMA_ID_MAX: u32 = 4294967294_u32;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "metadata",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicSchemaV2MetadataEntryDecoder::skip(
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
                field: "columns",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicSchemaV2ColumnsEntryDecoder::skip(
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
        if start + 2 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "tableName",
                needed: 2,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 2] = read_bytes::<2>(self.buf, start);
        let header = VarString16Encoding(bytes);
        let len = header.length() as usize;
        if start + 2 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "tableName",
                needed: 2 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 2 + len)
    }
    #[inline]
    fn tail_offset_4(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_3()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbolTable",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = SymbolTableEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbolTable",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn metadata(
        &self,
    ) -> Result<DynamicSchemaV2MetadataDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        DynamicSchemaV2MetadataDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn columns(&self) -> Result<DynamicSchemaV2ColumnsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        DynamicSchemaV2ColumnsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn table_name(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        let bytes: [u8; 2] = read_bytes::<2>(self.buf, offset);
        let header = VarString16Encoding(bytes);
        let len = header.length() as usize;
        if len > 65534 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(table_name),
                length: len as u32,
                max_length: 65534,
            });
        }
        let data_offset = offset + 2;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn table_name_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.table_name()?;
        core::str::from_utf8(bytes).map_err(sbe_rt::DecodeError::Utf8)
    }
    #[inline]
    fn symbol_table(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_3()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = SymbolTableEncoding(bytes);
        let len = header.length() as usize;
        if len > 4294967294 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(symbol_table),
                length: len as u32,
                max_length: 4294967294,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn symbol_table_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.symbol_table()?;
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
        let end = self.tail_offset_4()?;
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
                    field: "metadata",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 4;
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
                    field: "columns",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 7;
            if entries_end > buf.len() {
                return Err(sbe_rt::VerifyError::MessageTooShort {
                    needed: entries_end,
                    available: buf.len(),
                });
            }
            offset = entries_end;
        }
        {
            if offset + 2 > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "table_name",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 2] = read_bytes::<2>(buf, offset);
            let var_header = VarString16Encoding(bytes);
            let len = var_header.length();
            let data_end = offset + 2 + len as usize;
            if data_end > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "table_name",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        {
            if offset + 4 > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "symbol_table",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let var_header = SymbolTableEncoding(bytes);
            let len = var_header.length();
            let data_end = offset + 4 + len as usize;
            if data_end > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "symbol_table",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        Ok(())
    }
}
impl<'a> TryFrom<&'a [u8]> for DynamicSchemaV2Decoder<'a> {
    type Error = sbe_rt::DecodeError;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::wrap_and_apply_header(buf, 0)
    }
}
impl<'a> sbe_rt::private::Sealed for DynamicSchemaV2Decoder<'a> {}
impl<'a> sbe_rt::SbeMessage for DynamicSchemaV2Decoder<'a> {
    const TEMPLATE_ID: u16 = 3;
    const BLOCK_LENGTH: usize = 4;
    const SCHEMA_ID: u16 = 1000;
    const SCHEMA_VERSION: u16 = 1;
}
impl<'a> AsRef<[u8]> for DynamicSchemaV2Decoder<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes().unwrap_or(&[])
    }
}
impl<'a> DynamicSchemaV2Decoder<'a> {
    pub fn as_ref_opt(&self) -> Option<&[u8]> {
        self.as_bytes().ok()
    }
}
impl<'a> core::fmt::Display for DynamicSchemaV2Decoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicSchemaV2 {{ ")?;
        {
            let v = self.schema_id();
            write!(f, "schema_id: {:?}", v)?;
        }
        if let Ok(g) = self.metadata() {
            write!(f, ", metadata: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.columns() {
            write!(f, ", columns: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(d) = self.table_name() {
            write!(f, ", table_name: {} bytes", d.len())?;
        }
        if let Ok(d) = self.symbol_table() {
            write!(f, ", symbol_table: {} bytes", d.len())?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicSchemaV2MetadataDecoder<'a> {
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
impl<'a> DynamicSchemaV2MetadataDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
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
                field: "metadata",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicSchemaV2MetadataDecoder<'a> {
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
impl<'a> DynamicSchemaV2MetadataDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "metadata",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicSchemaV2MetadataDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicSchemaV2MetadataEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "metadata",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "metadata",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicSchemaV2MetadataEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicSchemaV2MetadataDecoder<'a> {
    type Item = DynamicSchemaV2MetadataEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicSchemaV2MetadataEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicSchemaV2MetadataDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicSchemaV2MetadataEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicSchemaV2MetadataEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
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
    pub fn key_len(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    pub const KEY_LEN_NULL: u16 = 65535_u16;
    pub const KEY_LEN_MIN: u16 = 0_u16;
    pub const KEY_LEN_MAX: u16 = 65534_u16;
    #[inline]
    pub fn val_len(&self) -> u16 {
        let offset = self.pos + 2;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    pub const VAL_LEN_NULL: u16 = 65535_u16;
    pub const VAL_LEN_MIN: u16 = 0_u16;
    pub const VAL_LEN_MAX: u16 = 65534_u16;
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
impl<'a> core::fmt::Display for DynamicSchemaV2MetadataEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.key_len();
            write!(f, "keyLen: {:?}", v)?;
        }
        {
            let v = self.val_len();
            write!(f, ", valLen: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicSchemaV2ColumnsDecoder<'a> {
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
impl<'a> DynamicSchemaV2ColumnsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 7;
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
                field: "columns",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicSchemaV2ColumnsDecoder<'a> {
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
impl<'a> DynamicSchemaV2ColumnsDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "columns",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicSchemaV2ColumnsDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicSchemaV2ColumnsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "columns",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "columns",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicSchemaV2ColumnsEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicSchemaV2ColumnsDecoder<'a> {
    type Item = DynamicSchemaV2ColumnsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicSchemaV2ColumnsEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicSchemaV2ColumnsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicSchemaV2ColumnsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicSchemaV2ColumnsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 7;
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
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn name_len(&self) -> u16 {
        let offset = self.pos + 1;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    pub const NAME_LEN_NULL: u16 = 65535_u16;
    pub const NAME_LEN_MIN: u16 = 0_u16;
    pub const NAME_LEN_MAX: u16 = 65534_u16;
    #[inline]
    pub fn outer_type(&self) -> u8 {
        let offset = self.pos + 3;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const OUTER_TYPE_NULL: u8 = 255_u8;
    pub const OUTER_TYPE_MIN: u8 = 0_u8;
    pub const OUTER_TYPE_MAX: u8 = 254_u8;
    #[inline]
    pub fn inner_type(&self) -> u8 {
        let offset = self.pos + 4;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const INNER_TYPE_NULL: u8 = 255_u8;
    pub const INNER_TYPE_MIN: u8 = 0_u8;
    pub const INNER_TYPE_MAX: u8 = 254_u8;
    #[inline]
    pub fn precision(&self) -> u8 {
        let offset = self.pos + 5;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const PRECISION_NULL: u8 = 255_u8;
    pub const PRECISION_MIN: u8 = 0_u8;
    pub const PRECISION_MAX: u8 = 254_u8;
    #[inline]
    pub fn scale(&self) -> u8 {
        let offset = self.pos + 6;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const SCALE_NULL: u8 = 255_u8;
    pub const SCALE_MIN: u8 = 0_u8;
    pub const SCALE_MAX: u8 = 254_u8;
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
impl<'a> core::fmt::Display for DynamicSchemaV2ColumnsEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.field_id();
            write!(f, "fieldId: {:?}", v)?;
        }
        {
            let v = self.name_len();
            write!(f, ", nameLen: {:?}", v)?;
        }
        {
            let v = self.outer_type();
            write!(f, ", outerType: {:?}", v)?;
        }
        {
            let v = self.inner_type();
            write!(f, ", innerType: {:?}", v)?;
        }
        {
            let v = self.precision();
            write!(f, ", precision: {:?}", v)?;
        }
        {
            let v = self.scale();
            write!(f, ", scale: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicSchemaV2DecoderAfterMetadata<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicSchemaV2DecoderAfterColumns<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicSchemaV2DecoderAfterTableName<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicSchemaV2DecoderComplete<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicSchemaV2DecoderAfterMetadata<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicSchemaV2DecoderAfterColumns<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicSchemaV2DecoderAfterTableName<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicSchemaV2DecoderComplete<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicSchemaV2Decoder<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_metadata(
        self,
    ) -> Result<DynamicSchemaV2MetadataDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.pos + self.acting_block_length;
        DynamicSchemaV2MetadataDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicSchemaV2DecoderAfterMetadata<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_columns(
        self,
    ) -> Result<DynamicSchemaV2ColumnsDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        DynamicSchemaV2ColumnsDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicSchemaV2DecoderAfterColumns<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_table_name(
        self,
    ) -> Result<
        (&'a [u8], DynamicSchemaV2DecoderAfterTableName<'a>),
        sbe_rt::DecodeError,
    > {
        let offset = self.tail_start;
        if offset + 2 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "tableName",
                needed: 2,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 2] = read_bytes::<2>(self.buf, offset);
        let header = VarString16Encoding(bytes);
        let len = header.length() as usize;
        if len > 65534 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "tableName",
                length: len as u32,
                max_length: 65534,
            });
        }
        let data_start = offset + 2;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "tableName",
                needed: 2 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = DynamicSchemaV2DecoderAfterTableName {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
}
impl<'a> DynamicSchemaV2DecoderAfterColumns<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_table_name_as_message(
        self,
    ) -> Result<
        (DecodedFrame<'a>, DynamicSchemaV2DecoderAfterTableName<'a>),
        sbe_rt::DecodeError,
    > {
        let (data, next) = self.into_table_name()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
}
impl<'a> DynamicSchemaV2DecoderAfterColumns<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_table_name<E, F>(
        self,
        f: F,
    ) -> Result<DynamicSchemaV2DecoderAfterTableName<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_table_name()?;
        f(data)?;
        Ok(next)
    }
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
    #[inline]
    pub fn try_table_name_as_message<E, F>(
        self,
        f: F,
    ) -> Result<DynamicSchemaV2DecoderAfterTableName<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_table_name_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> DynamicSchemaV2DecoderAfterTableName<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_symbol_table(
        self,
    ) -> Result<(&'a [u8], DynamicSchemaV2DecoderComplete<'a>), sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbolTable",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = SymbolTableEncoding(bytes);
        let len = header.length() as usize;
        if len > 4294967294 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "symbolTable",
                length: len as u32,
                max_length: 4294967294,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbolTable",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = DynamicSchemaV2DecoderComplete {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
}
impl<'a> DynamicSchemaV2DecoderAfterTableName<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_symbol_table_as_message(
        self,
    ) -> Result<
        (DecodedFrame<'a>, DynamicSchemaV2DecoderComplete<'a>),
        sbe_rt::DecodeError,
    > {
        let (data, next) = self.into_symbol_table()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
}
impl<'a> DynamicSchemaV2DecoderAfterTableName<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_symbol_table<E, F>(
        self,
        f: F,
    ) -> Result<DynamicSchemaV2DecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_symbol_table()?;
        f(data)?;
        Ok(next)
    }
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
    #[inline]
    pub fn try_symbol_table_as_message<E, F>(
        self,
        f: F,
    ) -> Result<DynamicSchemaV2DecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_symbol_table_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> DynamicSchemaV2MetadataDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicSchemaV2DecoderAfterMetadata<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicSchemaV2MetadataEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicSchemaV2DecoderAfterMetadata {
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
    ) -> Result<DynamicSchemaV2DecoderAfterMetadata<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicSchemaV2ColumnsDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicSchemaV2DecoderAfterColumns<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicSchemaV2ColumnsEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicSchemaV2DecoderAfterColumns {
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
    ) -> Result<DynamicSchemaV2DecoderAfterColumns<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicSchemaV2DecoderComplete<'a> {
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
///V2 schema with column type metadata
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicSchemaV2Encoder<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicSchemaV2AfterMetadata<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicSchemaV2AfterColumns<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicSchemaV2AfterTableName<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicSchemaV2Complete<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
impl<'a> DynamicSchemaV2Encoder<'a> {
    pub const SCHEMA_ID: u16 = 1000;
    pub const SCHEMA_VERSION: u16 = 1;
    pub const TEMPLATE_ID: u16 = 3;
    pub const BLOCK_LENGTH: usize = 4;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 4);
    ///MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [4, 0, 3, 0, 232, 3, 1, 0];
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
    pub fn schema_id(&mut self, val: u32) -> &mut Self {
        let offset = 8;
        self.buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        self
    }
    /// Compute the exact SBE message body length before encoding.
    /// Parameters: one `usize` per group (entry count) and one `usize` per var-data field (byte length).
    #[inline]
    pub const fn compute_encoded_length(
        metadata_count: usize,
        columns_count: usize,
        table_name_len: usize,
        symbol_table_len: usize,
    ) -> usize {
        let mut len = 4;
        len += 4 + metadata_count * 4;
        len += 4 + columns_count * 7;
        len += 2 + table_name_len;
        len += 4 + symbol_table_len;
        len
    }
    /// Compute the exact SBE message length including the standard
    /// message header (header size + body). DECISIONS.md §2: callers
    /// must use this — not a hand-written `+ 8`.
    #[inline]
    pub const fn compute_encoded_length_with_message_header(
        metadata_count: usize,
        columns_count: usize,
        table_name_len: usize,
        symbol_table_len: usize,
    ) -> usize {
        8usize
            + Self::compute_encoded_length(
                metadata_count,
                columns_count,
                table_name_len,
                symbol_table_len,
            )
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
impl<'a> DynamicSchemaV2Encoder<'a> {
    #[must_use]
    pub fn metadata<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicSchemaV2AfterMetadata<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicSchemaV2MetadataEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicSchemaV2MetadataEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicSchemaV2MetadataEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicSchemaV2AfterMetadata {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_metadata<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicSchemaV2AfterMetadata<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicSchemaV2MetadataEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicSchemaV2MetadataEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicSchemaV2MetadataEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicSchemaV2AfterMetadata {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicSchemaV2AfterMetadata<'a> {
    #[must_use]
    pub fn columns<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicSchemaV2AfterColumns<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicSchemaV2ColumnsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicSchemaV2ColumnsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicSchemaV2ColumnsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicSchemaV2AfterColumns {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_columns<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicSchemaV2AfterColumns<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicSchemaV2ColumnsEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicSchemaV2ColumnsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicSchemaV2ColumnsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicSchemaV2AfterColumns {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicSchemaV2AfterColumns<'a> {
    #[must_use]
    pub fn table_name(
        mut self,
        data: &[u8],
    ) -> Result<DynamicSchemaV2AfterTableName<'a>, sbe_rt::EncodeError> {
        if data.len() > 65534 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "tableName",
                max_length: 65534,
                actual: data.len(),
            });
        }
        let needed = 2 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len() - self.pos,
            });
        }
        let len_bytes = (data.len() as u16).to_le_bytes();
        self.buf[self.pos..self.pos + 2].copy_from_slice(&len_bytes);
        let start = self.pos + 2;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(DynamicSchemaV2AfterTableName {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn table_name_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<DynamicSchemaV2AfterTableName<'a>, sbe_rt::EncodeError> {
        let needed = 2 + data.len();
        if self.pos + needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len() - self.pos,
            });
        }
        let len_bytes = (data.len() as u16).to_le_bytes();
        self.buf[self.pos..self.pos + 2].copy_from_slice(&len_bytes);
        let start = self.pos + 2;
        self.buf[start..start + data.len()].copy_from_slice(data);
        Ok(DynamicSchemaV2AfterTableName {
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
    pub fn table_name_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<DynamicSchemaV2AfterTableName<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 65534 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "tableName",
                    max_length: 65534,
                    actual: exact_len,
                }
                    .into(),
            );
        }
        let needed = 2 + exact_len;
        if self.pos + needed > self.buf.len() {
            return Err(
                sbe_rt::EncodeError::BufferTooShort {
                    needed,
                    available: self.buf.len() - self.pos,
                }
                    .into(),
            );
        }
        let len_bytes = (exact_len as u16).to_le_bytes();
        self.buf[self.pos..self.pos + 2].copy_from_slice(&len_bytes);
        let start = self.pos + 2;
        f(&mut self.buf[start..start + exact_len])?;
        Ok(DynamicSchemaV2AfterTableName {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + exact_len,
        })
    }
}
impl<'a> DynamicSchemaV2AfterTableName<'a> {
    #[must_use]
    pub fn symbol_table(
        mut self,
        data: &[u8],
    ) -> Result<DynamicSchemaV2Complete<'a>, sbe_rt::EncodeError> {
        if data.len() > 4294967294 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "symbolTable",
                max_length: 4294967294,
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
        Ok(DynamicSchemaV2Complete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn symbol_table_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<DynamicSchemaV2Complete<'a>, sbe_rt::EncodeError> {
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
        Ok(DynamicSchemaV2Complete {
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
    pub fn symbol_table_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<DynamicSchemaV2Complete<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 4294967294 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "symbolTable",
                    max_length: 4294967294,
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
        Ok(DynamicSchemaV2Complete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + exact_len,
        })
    }
}
impl<'a> DynamicSchemaV2Complete<'a> {
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
impl<'a> AsRef<[u8]> for DynamicSchemaV2Complete<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a> sbe_rt::private::Sealed for DynamicSchemaV2Encoder<'a> {}
impl<'a> sbe_rt::SbeMessage for DynamicSchemaV2Encoder<'a> {
    const TEMPLATE_ID: u16 = 3;
    const BLOCK_LENGTH: usize = 4;
    const SCHEMA_ID: u16 = 1000;
    const SCHEMA_VERSION: u16 = 1;
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicSchemaV2MetadataEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicSchemaV2MetadataEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [4, 0, 0, 0];
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
        F: FnOnce(&mut DynamicSchemaV2MetadataEntryEncoder<'b>),
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
            let mut __entry = DynamicSchemaV2MetadataEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct DynamicSchemaV2MetadataEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicSchemaV2MetadataEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn key_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn val_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 2;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicSchemaV2ColumnsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicSchemaV2ColumnsEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 7;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [7, 0, 0, 0];
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
        F: FnOnce(&mut DynamicSchemaV2ColumnsEntryEncoder<'b>),
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
            let mut __entry = DynamicSchemaV2ColumnsEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct DynamicSchemaV2ColumnsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicSchemaV2ColumnsEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 7;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn field_id(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn name_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 1;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn outer_type(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 3;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn inner_type(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 4;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn precision(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 5;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn scale(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 6;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
}
pub mod dynamic_schema_v2_field_meta {
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
            name: "schemaId",
            id: 1,
            offset: 0,
            since_version: 0,
            field_type: "u32",
            presence: "required",
            null_value: Some("4294967295"),
            semantic_type: None,
            description: None,
        },
    ];
}
///V2 row with decimal array fields
#[derive(Clone, Copy)]
pub struct DynamicRowV2Decoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2Decoder<'a> {
    pub const SCHEMA_ID: u16 = 1000;
    pub const SCHEMA_VERSION: u16 = 1;
    pub const TEMPLATE_ID: u16 = 4;
    pub const BLOCK_LENGTH: usize = 4;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 4);
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
                expected_name: "org.ergo.sbe.persist.v2",
            });
        }
        if header.schema_id() != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: header.schema_id(),
                expected_name: "org.ergo.sbe.persist.v2",
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
    pub fn schema_id(&self) -> u32 {
        let offset = self.pos + 0;
        u32::from_le_bytes(read_bytes::<4>(self.buf, offset))
    }
    pub const SCHEMA_ID_NULL: u32 = 4294967295_u32;
    pub const SCHEMA_ID_MIN: u32 = 0_u32;
    pub const SCHEMA_ID_MAX: u32 = 4294967294_u32;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "rowMetadata",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicRowV2RowMetadataEntryDecoder::skip(
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
                field: "int64Fields",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicRowV2Int64FieldsEntryDecoder::skip(
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
                field: "uint64Fields",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicRowV2Uint64FieldsEntryDecoder::skip(
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
    fn tail_offset_4(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_3()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "float64Fields",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicRowV2Float64FieldsEntryDecoder::skip(
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
    fn tail_offset_5(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_4()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "boolFields",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicRowV2BoolFieldsEntryDecoder::skip(
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
    fn tail_offset_6(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_5()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "stringFields",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicRowV2StringFieldsEntryDecoder::skip(
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
    fn tail_offset_7(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_6()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "nullFields",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicRowV2NullFieldsEntryDecoder::skip(
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
    fn tail_offset_8(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_7()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "decimalArrayFields",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicRowV2DecimalArrayFieldsEntryDecoder::skip(
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
    fn tail_offset_9(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_8()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbolTable",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = SymbolTableEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbolTable",
                needed: 4 + len,
                available: self.buf.len() - start,
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn row_metadata(
        &self,
    ) -> Result<DynamicRowV2RowMetadataDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        DynamicRowV2RowMetadataDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn int64_fields(
        &self,
    ) -> Result<DynamicRowV2Int64FieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        DynamicRowV2Int64FieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn uint64_fields(
        &self,
    ) -> Result<DynamicRowV2Uint64FieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        DynamicRowV2Uint64FieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn float64_fields(
        &self,
    ) -> Result<DynamicRowV2Float64FieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_3()?;
        DynamicRowV2Float64FieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn bool_fields(
        &self,
    ) -> Result<DynamicRowV2BoolFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_4()?;
        DynamicRowV2BoolFieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn string_fields(
        &self,
    ) -> Result<DynamicRowV2StringFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_5()?;
        DynamicRowV2StringFieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn null_fields(
        &self,
    ) -> Result<DynamicRowV2NullFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_6()?;
        DynamicRowV2NullFieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    fn decimal_array_fields(
        &self,
    ) -> Result<DynamicRowV2DecimalArrayFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_7()?;
        DynamicRowV2DecimalArrayFieldsDecoder::wrap(
            self.buf,
            offset,
            self.acting_version,
        )
    }
    #[inline]
    fn symbol_table(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_8()?;
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = SymbolTableEncoding(bytes);
        let len = header.length() as usize;
        if len > 4294967294 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: stringify!(symbol_table),
                length: len as u32,
                max_length: 4294967294,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    fn symbol_table_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.symbol_table()?;
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
        let end = self.tail_offset_9()?;
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
                    field: "row_metadata",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 4;
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
                    field: "int64_fields",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 9;
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
                    field: "uint64_fields",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 9;
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
                    field: "float64_fields",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 9;
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
                    field: "bool_fields",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 2;
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
                    field: "string_fields",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
            let count = dim.num_in_group() as usize;
            let entries_end = offset + 4 + count * 3;
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
                    field: "null_fields",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
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
                return Err(sbe_rt::VerifyError::GroupDimOutOfBounds {
                    field: "decimal_array_fields",
                    offset,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let dim = GroupSize16Encoding(bytes);
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
                    field: "symbol_table",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 4] = read_bytes::<4>(buf, offset);
            let var_header = SymbolTableEncoding(bytes);
            let len = var_header.length();
            let data_end = offset + 4 + len as usize;
            if data_end > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "symbol_table",
                    offset,
                    length: len as u32,
                });
            }
            offset = data_end;
        }
        Ok(())
    }
}
impl<'a> TryFrom<&'a [u8]> for DynamicRowV2Decoder<'a> {
    type Error = sbe_rt::DecodeError;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::wrap_and_apply_header(buf, 0)
    }
}
impl<'a> sbe_rt::private::Sealed for DynamicRowV2Decoder<'a> {}
impl<'a> sbe_rt::SbeMessage for DynamicRowV2Decoder<'a> {
    const TEMPLATE_ID: u16 = 4;
    const BLOCK_LENGTH: usize = 4;
    const SCHEMA_ID: u16 = 1000;
    const SCHEMA_VERSION: u16 = 1;
}
impl<'a> AsRef<[u8]> for DynamicRowV2Decoder<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes().unwrap_or(&[])
    }
}
impl<'a> DynamicRowV2Decoder<'a> {
    pub fn as_ref_opt(&self) -> Option<&[u8]> {
        self.as_bytes().ok()
    }
}
impl<'a> core::fmt::Display for DynamicRowV2Decoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicRowV2 {{ ")?;
        {
            let v = self.schema_id();
            write!(f, "schema_id: {:?}", v)?;
        }
        if let Ok(g) = self.row_metadata() {
            write!(f, ", row_metadata: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.int64_fields() {
            write!(f, ", int64_fields: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.uint64_fields() {
            write!(f, ", uint64_fields: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.float64_fields() {
            write!(f, ", float64_fields: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.bool_fields() {
            write!(f, ", bool_fields: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.string_fields() {
            write!(f, ", string_fields: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.null_fields() {
            write!(f, ", null_fields: [")?;
            for (i, entry) in g.enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{}", entry)?;
            }
            write!(f, "]")?;
        }
        if let Ok(g) = self.decimal_array_fields() {
            write!(f, ", decimal_array_fields: [")?;
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
        if let Ok(d) = self.symbol_table() {
            write!(f, ", symbol_table: {} bytes", d.len())?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicRowV2RowMetadataDecoder<'a> {
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
impl<'a> DynamicRowV2RowMetadataDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
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
                field: "rowMetadata",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicRowV2RowMetadataDecoder<'a> {
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
impl<'a> DynamicRowV2RowMetadataDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "rowMetadata",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicRowV2RowMetadataDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicRowV2RowMetadataEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "rowMetadata",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "rowMetadata",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicRowV2RowMetadataEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicRowV2RowMetadataDecoder<'a> {
    type Item = DynamicRowV2RowMetadataEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicRowV2RowMetadataEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicRowV2RowMetadataDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicRowV2RowMetadataEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2RowMetadataEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
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
    pub fn key_len(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    pub const KEY_LEN_NULL: u16 = 65535_u16;
    pub const KEY_LEN_MIN: u16 = 0_u16;
    pub const KEY_LEN_MAX: u16 = 65534_u16;
    #[inline]
    pub fn val_len(&self) -> u16 {
        let offset = self.pos + 2;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    pub const VAL_LEN_NULL: u16 = 65535_u16;
    pub const VAL_LEN_MIN: u16 = 0_u16;
    pub const VAL_LEN_MAX: u16 = 65534_u16;
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
impl<'a> core::fmt::Display for DynamicRowV2RowMetadataEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.key_len();
            write!(f, "keyLen: {:?}", v)?;
        }
        {
            let v = self.val_len();
            write!(f, ", valLen: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicRowV2Int64FieldsDecoder<'a> {
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
impl<'a> DynamicRowV2Int64FieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
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
                field: "int64Fields",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicRowV2Int64FieldsDecoder<'a> {
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
impl<'a> DynamicRowV2Int64FieldsDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "int64Fields",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicRowV2Int64FieldsDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicRowV2Int64FieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "int64Fields",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "int64Fields",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicRowV2Int64FieldsEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicRowV2Int64FieldsDecoder<'a> {
    type Item = DynamicRowV2Int64FieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicRowV2Int64FieldsEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicRowV2Int64FieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicRowV2Int64FieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2Int64FieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
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
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn value(&self) -> i64 {
        let offset = self.pos + 1;
        i64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const VALUE_NULL: i64 = -9223372036854775808_i64;
    pub const VALUE_MIN: i64 = -9223372036854775807_i64;
    pub const VALUE_MAX: i64 = 9223372036854775807_i64;
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
impl<'a> core::fmt::Display for DynamicRowV2Int64FieldsEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.field_id();
            write!(f, "fieldId: {:?}", v)?;
        }
        {
            let v = self.value();
            write!(f, ", value: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicRowV2Uint64FieldsDecoder<'a> {
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
impl<'a> DynamicRowV2Uint64FieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
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
                field: "uint64Fields",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicRowV2Uint64FieldsDecoder<'a> {
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
impl<'a> DynamicRowV2Uint64FieldsDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "uint64Fields",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicRowV2Uint64FieldsDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicRowV2Uint64FieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "uint64Fields",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "uint64Fields",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicRowV2Uint64FieldsEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicRowV2Uint64FieldsDecoder<'a> {
    type Item = DynamicRowV2Uint64FieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicRowV2Uint64FieldsEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicRowV2Uint64FieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicRowV2Uint64FieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2Uint64FieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
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
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn value(&self) -> u64 {
        let offset = self.pos + 1;
        u64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const VALUE_NULL: u64 = 18446744073709551615_u64;
    pub const VALUE_MIN: u64 = 0_u64;
    pub const VALUE_MAX: u64 = 18446744073709551614_u64;
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
impl<'a> core::fmt::Display for DynamicRowV2Uint64FieldsEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.field_id();
            write!(f, "fieldId: {:?}", v)?;
        }
        {
            let v = self.value();
            write!(f, ", value: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicRowV2Float64FieldsDecoder<'a> {
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
impl<'a> DynamicRowV2Float64FieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
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
                field: "float64Fields",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicRowV2Float64FieldsDecoder<'a> {
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
impl<'a> DynamicRowV2Float64FieldsDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "float64Fields",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicRowV2Float64FieldsDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicRowV2Float64FieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "float64Fields",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "float64Fields",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicRowV2Float64FieldsEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicRowV2Float64FieldsDecoder<'a> {
    type Item = DynamicRowV2Float64FieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicRowV2Float64FieldsEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicRowV2Float64FieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicRowV2Float64FieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2Float64FieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
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
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn value(&self) -> f64 {
        let offset = self.pos + 1;
        f64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const VALUE_NULL: f64 = f64::from_bits(9221120237041090561);
    pub const VALUE_MIN: f64 = f64::from_bits(18442240474082181119);
    pub const VALUE_MAX: f64 = f64::from_bits(9218868437227405311);
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
impl<'a> core::fmt::Display for DynamicRowV2Float64FieldsEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.field_id();
            write!(f, "fieldId: {:?}", v)?;
        }
        {
            let v = self.value();
            write!(f, ", value: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicRowV2BoolFieldsDecoder<'a> {
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
impl<'a> DynamicRowV2BoolFieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 2;
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
                field: "boolFields",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicRowV2BoolFieldsDecoder<'a> {
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
impl<'a> DynamicRowV2BoolFieldsDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "boolFields",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicRowV2BoolFieldsDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicRowV2BoolFieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "boolFields",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "boolFields",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicRowV2BoolFieldsEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicRowV2BoolFieldsDecoder<'a> {
    type Item = DynamicRowV2BoolFieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicRowV2BoolFieldsEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicRowV2BoolFieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicRowV2BoolFieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2BoolFieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 2;
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
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn value(&self) -> u8 {
        let offset = self.pos + 1;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const VALUE_NULL: u8 = 255_u8;
    pub const VALUE_MIN: u8 = 0_u8;
    pub const VALUE_MAX: u8 = 254_u8;
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
impl<'a> core::fmt::Display for DynamicRowV2BoolFieldsEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.field_id();
            write!(f, "fieldId: {:?}", v)?;
        }
        {
            let v = self.value();
            write!(f, ", value: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicRowV2StringFieldsDecoder<'a> {
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
impl<'a> DynamicRowV2StringFieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 3;
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
                field: "stringFields",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicRowV2StringFieldsDecoder<'a> {
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
impl<'a> DynamicRowV2StringFieldsDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "stringFields",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicRowV2StringFieldsDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicRowV2StringFieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "stringFields",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "stringFields",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicRowV2StringFieldsEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicRowV2StringFieldsDecoder<'a> {
    type Item = DynamicRowV2StringFieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicRowV2StringFieldsEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicRowV2StringFieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicRowV2StringFieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2StringFieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 3;
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
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn str_len(&self) -> u16 {
        let offset = self.pos + 1;
        u16::from_le_bytes(read_bytes::<2>(self.buf, offset))
    }
    pub const STR_LEN_NULL: u16 = 65535_u16;
    pub const STR_LEN_MIN: u16 = 0_u16;
    pub const STR_LEN_MAX: u16 = 65534_u16;
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
impl<'a> core::fmt::Display for DynamicRowV2StringFieldsEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.field_id();
            write!(f, "fieldId: {:?}", v)?;
        }
        {
            let v = self.str_len();
            write!(f, ", strLen: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicRowV2NullFieldsDecoder<'a> {
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
impl<'a> DynamicRowV2NullFieldsDecoder<'a> {
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
                field: "nullFields",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicRowV2NullFieldsDecoder<'a> {
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
impl<'a> DynamicRowV2NullFieldsDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "nullFields",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicRowV2NullFieldsDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicRowV2NullFieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "nullFields",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "nullFields",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicRowV2NullFieldsEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicRowV2NullFieldsDecoder<'a> {
    type Item = DynamicRowV2NullFieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicRowV2NullFieldsEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicRowV2NullFieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicRowV2NullFieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2NullFieldsEntryDecoder<'a> {
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
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
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
impl<'a> core::fmt::Display for DynamicRowV2NullFieldsEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.field_id();
            write!(f, "fieldId: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicRowV2DecimalArrayFieldsDecoder<'a> {
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
impl<'a> DynamicRowV2DecimalArrayFieldsDecoder<'a> {
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
                field: "decimalArrayFields",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicRowV2DecimalArrayFieldsDecoder<'a> {
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
impl<'a> DynamicRowV2DecimalArrayFieldsDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "decimalArrayFields",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        for _ in 0..n {
            let entry = DynamicRowV2DecimalArrayFieldsEntryDecoder::wrap(
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
impl<'a> DynamicRowV2DecimalArrayFieldsDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<DynamicRowV2DecimalArrayFieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "decimalArrayFields",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "decimalArrayFields",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicRowV2DecimalArrayFieldsEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicRowV2DecimalArrayFieldsDecoder<'a> {
    type Item = Result<
        DynamicRowV2DecimalArrayFieldsEntryDecoder<'a>,
        sbe_rt::DecodeError,
    >;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicRowV2DecimalArrayFieldsEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicRowV2DecimalArrayFieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicRowV2DecimalArrayFieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2DecimalArrayFieldsEntryDecoder<'a> {
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
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + self.acting_block_length)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "values",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, start);
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = DynamicRowV2DecimalArrayFieldsValuesEntryDecoder::skip(
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
    pub fn values(
        &self,
    ) -> Result<DynamicRowV2DecimalArrayFieldsValuesDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        DynamicRowV2DecimalArrayFieldsValuesDecoder::wrap(
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
impl<'a> core::fmt::Display for DynamicRowV2DecimalArrayFieldsEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.field_id();
            write!(f, "fieldId: {:?}", v)?;
        }
        write!(f, ", values: [")?;
        if let Ok(ng_decoder) = self.values() {
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
pub struct DynamicRowV2DecimalArrayFieldsValuesDecoder<'a> {
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
impl<'a> DynamicRowV2DecimalArrayFieldsValuesDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
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
                field: "values",
                needed: 4,
                available: buf.len().saturating_sub(pos),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(buf, pos);
        let header = GroupSize16Encoding(bytes);
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
impl<'a> DynamicRowV2DecimalArrayFieldsValuesDecoder<'a> {
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
impl<'a> DynamicRowV2DecimalArrayFieldsValuesDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if cfg!(not(feature = "bound-check-disabled")) && n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "values",
                needed: n * self.acting_block_length,
                available: self.count * self.acting_block_length,
            });
        }
        self.pos += n * self.acting_block_length;
        self.count -= n;
        Ok(())
    }
}
impl<'a> DynamicRowV2DecimalArrayFieldsValuesDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<
        DynamicRowV2DecimalArrayFieldsValuesEntryDecoder<'a>,
        sbe_rt::DecodeError,
    > {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "values",
                needed: (idx + 1) * self.acting_block_length,
                available: self.total * self.acting_block_length,
            });
        }
        let offset = self.start + idx * self.acting_block_length;
        if offset + self.acting_block_length > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "values",
                needed: self.acting_block_length,
                available: self.buf.len() - offset,
            });
        }
        Ok(
            DynamicRowV2DecimalArrayFieldsValuesEntryDecoder::wrap(
                self.buf,
                offset,
                self.acting_block_length,
                self.acting_version,
            ),
        )
    }
}
impl<'a> Iterator for DynamicRowV2DecimalArrayFieldsValuesDecoder<'a> {
    type Item = DynamicRowV2DecimalArrayFieldsValuesEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = DynamicRowV2DecimalArrayFieldsValuesEntryDecoder::wrap(
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
impl<'a> ExactSizeIterator for DynamicRowV2DecimalArrayFieldsValuesDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct DynamicRowV2DecimalArrayFieldsValuesEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2DecimalArrayFieldsValuesEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
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
    pub fn mantissa(&self) -> i64 {
        let offset = self.pos + 0;
        i64::from_le_bytes(read_bytes::<8>(self.buf, offset))
    }
    pub const MANTISSA_NULL: i64 = -9223372036854775808_i64;
    pub const MANTISSA_MIN: i64 = -9223372036854775807_i64;
    pub const MANTISSA_MAX: i64 = 9223372036854775807_i64;
    #[inline]
    pub fn exponent(&self) -> i8 {
        let offset = self.pos + 8;
        i8::from_le_bytes(read_bytes::<1>(self.buf, offset))
    }
    pub const EXPONENT_NULL: i8 = -128_i8;
    pub const EXPONENT_MIN: i8 = -127_i8;
    pub const EXPONENT_MAX: i8 = 127_i8;
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
impl<'a> core::fmt::Display for DynamicRowV2DecimalArrayFieldsValuesEntryDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{{ ")?;
        {
            let v = self.mantissa();
            write!(f, "mantissa: {:?}", v)?;
        }
        {
            let v = self.exponent();
            write!(f, ", exponent: {:?}", v)?;
        }
        write!(f, " }}")
    }
}
pub struct DynamicRowV2DecimalArrayFieldsEntryDecoderComplete<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2DecimalArrayFieldsEntryDecoderComplete<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2DecimalArrayFieldsEntryDecoder<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_values(
        self,
    ) -> Result<DynamicRowV2DecimalArrayFieldsValuesDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.pos + self.acting_block_length;
        DynamicRowV2DecimalArrayFieldsValuesDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicRowV2DecimalArrayFieldsValuesDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<
        DynamicRowV2DecimalArrayFieldsEntryDecoderComplete<'a>,
        sbe_rt::DecodeError,
    > {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicRowV2DecimalArrayFieldsValuesEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicRowV2DecimalArrayFieldsEntryDecoderComplete {
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
    ) -> Result<
        DynamicRowV2DecimalArrayFieldsEntryDecoderComplete<'a>,
        sbe_rt::DecodeError,
    > {
        self.finish()
    }
}
impl<'a> DynamicRowV2DecimalArrayFieldsEntryDecoderComplete<'a> {
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
pub struct DynamicRowV2DecoderAfterRowMetadata<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicRowV2DecoderAfterInt64Fields<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicRowV2DecoderAfterUint64Fields<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicRowV2DecoderAfterFloat64Fields<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicRowV2DecoderAfterBoolFields<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicRowV2DecoderAfterStringFields<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicRowV2DecoderAfterNullFields<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicRowV2DecoderAfterDecimalArrayFields<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
pub struct DynamicRowV2DecoderComplete<'a> {
    buf: &'a [u8],
    pos: usize,
    tail_start: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowV2DecoderAfterRowMetadata<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2DecoderAfterInt64Fields<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2DecoderAfterUint64Fields<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2DecoderAfterFloat64Fields<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2DecoderAfterBoolFields<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2DecoderAfterStringFields<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2DecoderAfterNullFields<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2DecoderAfterDecimalArrayFields<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2DecoderComplete<'a> {
    #[inline]
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    #[inline]
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
}
impl<'a> DynamicRowV2Decoder<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_row_metadata(
        self,
    ) -> Result<DynamicRowV2RowMetadataDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.pos + self.acting_block_length;
        DynamicRowV2RowMetadataDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicRowV2DecoderAfterRowMetadata<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_int64_fields(
        self,
    ) -> Result<DynamicRowV2Int64FieldsDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        DynamicRowV2Int64FieldsDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicRowV2DecoderAfterInt64Fields<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_uint64_fields(
        self,
    ) -> Result<DynamicRowV2Uint64FieldsDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        DynamicRowV2Uint64FieldsDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicRowV2DecoderAfterUint64Fields<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_float64_fields(
        self,
    ) -> Result<DynamicRowV2Float64FieldsDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        DynamicRowV2Float64FieldsDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicRowV2DecoderAfterFloat64Fields<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_bool_fields(
        self,
    ) -> Result<DynamicRowV2BoolFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        DynamicRowV2BoolFieldsDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicRowV2DecoderAfterBoolFields<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_string_fields(
        self,
    ) -> Result<DynamicRowV2StringFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        DynamicRowV2StringFieldsDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicRowV2DecoderAfterStringFields<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_null_fields(
        self,
    ) -> Result<DynamicRowV2NullFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        DynamicRowV2NullFieldsDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicRowV2DecoderAfterNullFields<'a> {
    /// Consume this stage and start decoding the next tail group,
    /// enforcing wire order. The returned group decoder owns the
    /// right to advance to the following stage via `finish()`.
    #[inline]
    pub fn into_decimal_array_fields(
        self,
    ) -> Result<DynamicRowV2DecimalArrayFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let group_start = self.tail_start;
        DynamicRowV2DecimalArrayFieldsDecoder::wrap_with_parent(
            self.buf,
            group_start,
            self.acting_version,
            self.pos,
            self.acting_block_length,
        )
    }
}
impl<'a> DynamicRowV2DecoderAfterDecimalArrayFields<'a> {
    /// Consume this stage, read the next var-data field, and advance
    /// to the following stage. Wire order is enforced by consumption.
    #[inline]
    pub fn into_symbol_table(
        self,
    ) -> Result<(&'a [u8], DynamicRowV2DecoderComplete<'a>), sbe_rt::DecodeError> {
        let offset = self.tail_start;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbolTable",
                needed: 4,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let bytes: [u8; 4] = read_bytes::<4>(self.buf, offset);
        let header = SymbolTableEncoding(bytes);
        let len = header.length() as usize;
        if len > 4294967294 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "symbolTable",
                length: len as u32,
                max_length: 4294967294,
            });
        }
        let data_start = offset + 4;
        if data_start + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "symbolTable",
                needed: 4 + len,
                available: self.buf.len().saturating_sub(offset),
            });
        }
        let data = &self.buf[data_start..data_start + len];
        let next = DynamicRowV2DecoderComplete {
            buf: self.buf,
            pos: self.pos,
            tail_start: data_start + len,
            acting_version: self.acting_version,
            acting_block_length: self.acting_block_length,
        };
        Ok((data, next))
    }
}
impl<'a> DynamicRowV2DecoderAfterDecimalArrayFields<'a> {
    /// Consume this stage, decode the var-data field as a nested
    /// SBE message via `AnyMessage::decode_frame`, and advance
    /// to the next stage.
    #[inline]
    pub fn into_symbol_table_as_message(
        self,
    ) -> Result<
        (DecodedFrame<'a>, DynamicRowV2DecoderComplete<'a>),
        sbe_rt::DecodeError,
    > {
        let (data, next) = self.into_symbol_table()?;
        let frame = AnyMessage::decode_frame(data, 0, data.len())?;
        Ok((frame, next))
    }
}
impl<'a> DynamicRowV2DecoderAfterDecimalArrayFields<'a> {
    /// Fallible scoped var-data accessor. Calls the closure with
    /// the decoded bytes and returns the next stage on success.
    #[inline]
    pub fn try_symbol_table<E, F>(
        self,
        f: F,
    ) -> Result<DynamicRowV2DecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(&[u8]) -> Result<(), E>,
    {
        let (data, next) = self.into_symbol_table()?;
        f(data)?;
        Ok(next)
    }
    /// Fallible scoped nested-message accessor. Decodes the
    /// var-data as an SBE message, calls the closure with the
    /// decoded frame, and returns the next stage on success.
    #[inline]
    pub fn try_symbol_table_as_message<E, F>(
        self,
        f: F,
    ) -> Result<DynamicRowV2DecoderComplete<'a>, E>
    where
        E: From<sbe_rt::DecodeError>,
        F: FnOnce(DecodedFrame<'a>) -> Result<(), E>,
    {
        let (frame, next) = self.into_symbol_table_as_message()?;
        f(frame)?;
        Ok(next)
    }
}
impl<'a> DynamicRowV2RowMetadataDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicRowV2DecoderAfterRowMetadata<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicRowV2RowMetadataEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicRowV2DecoderAfterRowMetadata {
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
    ) -> Result<DynamicRowV2DecoderAfterRowMetadata<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicRowV2Int64FieldsDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicRowV2DecoderAfterInt64Fields<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicRowV2Int64FieldsEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicRowV2DecoderAfterInt64Fields {
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
    ) -> Result<DynamicRowV2DecoderAfterInt64Fields<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicRowV2Uint64FieldsDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicRowV2DecoderAfterUint64Fields<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicRowV2Uint64FieldsEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicRowV2DecoderAfterUint64Fields {
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
    ) -> Result<DynamicRowV2DecoderAfterUint64Fields<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicRowV2Float64FieldsDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicRowV2DecoderAfterFloat64Fields<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicRowV2Float64FieldsEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicRowV2DecoderAfterFloat64Fields {
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
    ) -> Result<DynamicRowV2DecoderAfterFloat64Fields<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicRowV2BoolFieldsDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicRowV2DecoderAfterBoolFields<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicRowV2BoolFieldsEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicRowV2DecoderAfterBoolFields {
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
    ) -> Result<DynamicRowV2DecoderAfterBoolFields<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicRowV2StringFieldsDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicRowV2DecoderAfterStringFields<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicRowV2StringFieldsEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicRowV2DecoderAfterStringFields {
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
    ) -> Result<DynamicRowV2DecoderAfterStringFields<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicRowV2NullFieldsDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicRowV2DecoderAfterNullFields<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicRowV2NullFieldsEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicRowV2DecoderAfterNullFields {
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
    ) -> Result<DynamicRowV2DecoderAfterNullFields<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicRowV2DecimalArrayFieldsDecoder<'a> {
    /// Scan past any unread entries (including nested tails) in wire
    /// order and return the next decoder stage.
    #[inline]
    pub fn finish(
        self,
    ) -> Result<DynamicRowV2DecoderAfterDecimalArrayFields<'a>, sbe_rt::DecodeError> {
        let mut pos = self.pos;
        let mut remaining = self.count;
        let block_len = self.acting_block_length;
        while remaining > 0 {
            pos = DynamicRowV2DecimalArrayFieldsEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            remaining -= 1;
        }
        Ok(DynamicRowV2DecoderAfterDecimalArrayFields {
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
    ) -> Result<DynamicRowV2DecoderAfterDecimalArrayFields<'a>, sbe_rt::DecodeError> {
        self.finish()
    }
}
impl<'a> DynamicRowV2DecoderComplete<'a> {
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
///V2 row with decimal array fields
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2Encoder<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2AfterRowMetadata<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2AfterInt64Fields<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2AfterUint64Fields<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2AfterFloat64Fields<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2AfterBoolFields<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2AfterStringFields<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2AfterNullFields<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2AfterDecimalArrayFields<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
#[must_use = "encoder must be consumed to write the message"]
pub struct DynamicRowV2Complete<'a> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2Encoder<'a> {
    pub const SCHEMA_ID: u16 = 1000;
    pub const SCHEMA_VERSION: u16 = 1;
    pub const TEMPLATE_ID: u16 = 4;
    pub const BLOCK_LENGTH: usize = 4;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 4);
    ///MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [4, 0, 4, 0, 232, 3, 1, 0];
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
    pub fn schema_id(&mut self, val: u32) -> &mut Self {
        let offset = 8;
        self.buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
        self
    }
    /// Compute the exact SBE message body length before encoding.
    /// Parameters: one `usize` per group (entry count) and one `usize` per var-data field (byte length).
    #[inline]
    pub const fn compute_encoded_length(
        row_metadata_count: usize,
        int64_fields_count: usize,
        uint64_fields_count: usize,
        float64_fields_count: usize,
        bool_fields_count: usize,
        string_fields_count: usize,
        null_fields_count: usize,
        decimal_array_fields_count: usize,
        symbol_table_len: usize,
    ) -> usize {
        let mut len = 4;
        len += 4 + row_metadata_count * 4;
        len += 4 + int64_fields_count * 9;
        len += 4 + uint64_fields_count * 9;
        len += 4 + float64_fields_count * 9;
        len += 4 + bool_fields_count * 2;
        len += 4 + string_fields_count * 3;
        len += 4 + null_fields_count * 1;
        len += 4 + decimal_array_fields_count * 1;
        len += 4 + symbol_table_len;
        len
    }
    /// Compute the exact SBE message length including the standard
    /// message header (header size + body). DECISIONS.md §2: callers
    /// must use this — not a hand-written `+ 8`.
    #[inline]
    pub const fn compute_encoded_length_with_message_header(
        row_metadata_count: usize,
        int64_fields_count: usize,
        uint64_fields_count: usize,
        float64_fields_count: usize,
        bool_fields_count: usize,
        string_fields_count: usize,
        null_fields_count: usize,
        decimal_array_fields_count: usize,
        symbol_table_len: usize,
    ) -> usize {
        8usize
            + Self::compute_encoded_length(
                row_metadata_count,
                int64_fields_count,
                uint64_fields_count,
                float64_fields_count,
                bool_fields_count,
                string_fields_count,
                null_fields_count,
                decimal_array_fields_count,
                symbol_table_len,
            )
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
impl<'a> DynamicRowV2Encoder<'a> {
    #[must_use]
    pub fn row_metadata<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterRowMetadata<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicRowV2RowMetadataEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicRowV2RowMetadataEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2RowMetadataEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicRowV2AfterRowMetadata {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_row_metadata<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterRowMetadata<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicRowV2RowMetadataEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicRowV2RowMetadataEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2RowMetadataEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicRowV2AfterRowMetadata {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicRowV2AfterRowMetadata<'a> {
    #[must_use]
    pub fn int64_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterInt64Fields<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicRowV2Int64FieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicRowV2Int64FieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2Int64FieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicRowV2AfterInt64Fields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_int64_fields<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterInt64Fields<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicRowV2Int64FieldsEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicRowV2Int64FieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2Int64FieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicRowV2AfterInt64Fields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicRowV2AfterInt64Fields<'a> {
    #[must_use]
    pub fn uint64_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterUint64Fields<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicRowV2Uint64FieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicRowV2Uint64FieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2Uint64FieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicRowV2AfterUint64Fields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_uint64_fields<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterUint64Fields<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicRowV2Uint64FieldsEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicRowV2Uint64FieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2Uint64FieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicRowV2AfterUint64Fields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicRowV2AfterUint64Fields<'a> {
    #[must_use]
    pub fn float64_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterFloat64Fields<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicRowV2Float64FieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicRowV2Float64FieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2Float64FieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicRowV2AfterFloat64Fields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_float64_fields<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterFloat64Fields<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicRowV2Float64FieldsEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicRowV2Float64FieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2Float64FieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicRowV2AfterFloat64Fields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicRowV2AfterFloat64Fields<'a> {
    #[must_use]
    pub fn bool_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterBoolFields<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicRowV2BoolFieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicRowV2BoolFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2BoolFieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicRowV2AfterBoolFields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_bool_fields<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterBoolFields<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicRowV2BoolFieldsEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicRowV2BoolFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2BoolFieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicRowV2AfterBoolFields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicRowV2AfterBoolFields<'a> {
    #[must_use]
    pub fn string_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterStringFields<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicRowV2StringFieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicRowV2StringFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2StringFieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicRowV2AfterStringFields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_string_fields<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterStringFields<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicRowV2StringFieldsEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicRowV2StringFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2StringFieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicRowV2AfterStringFields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicRowV2AfterStringFields<'a> {
    #[must_use]
    pub fn null_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterNullFields<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicRowV2NullFieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicRowV2NullFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2NullFieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicRowV2AfterNullFields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_null_fields<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterNullFields<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicRowV2NullFieldsEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicRowV2NullFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2NullFieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicRowV2AfterNullFields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicRowV2AfterNullFields<'a> {
    #[must_use]
    pub fn decimal_array_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterDecimalArrayFields<'a>, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicRowV2DecimalArrayFieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&DynamicRowV2DecimalArrayFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2DecimalArrayFieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group);
        Ok(DynamicRowV2AfterDecimalArrayFields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
    /// Fallible group: propagates caller `?` errors via `E: From<EncodeError>`.
    #[must_use]
    pub fn try_decimal_array_fields<E, F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<DynamicRowV2AfterDecimalArrayFields<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut DynamicRowV2DecimalArrayFieldsEncoder<'a>) -> Result<(), E>,
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
            .copy_from_slice(&DynamicRowV2DecimalArrayFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = DynamicRowV2DecimalArrayFieldsEncoder::wrap(
            self.buf,
            self.pos + 4,
            count,
        );
        f(&mut group)?;
        Ok(DynamicRowV2AfterDecimalArrayFields {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
        })
    }
}
impl<'a> DynamicRowV2AfterDecimalArrayFields<'a> {
    #[must_use]
    pub fn symbol_table(
        mut self,
        data: &[u8],
    ) -> Result<DynamicRowV2Complete<'a>, sbe_rt::EncodeError> {
        if data.len() > 4294967294 {
            return Err(sbe_rt::EncodeError::VarDataTooLong {
                field: "symbolTable",
                max_length: 4294967294,
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
        Ok(DynamicRowV2Complete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
        })
    }
    #[must_use]
    pub fn symbol_table_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<DynamicRowV2Complete<'a>, sbe_rt::EncodeError> {
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
        Ok(DynamicRowV2Complete {
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
    pub fn symbol_table_with<E, F>(
        mut self,
        exact_len: usize,
        f: F,
    ) -> Result<DynamicRowV2Complete<'a>, E>
    where
        E: From<sbe_rt::EncodeError>,
        F: FnOnce(&mut [u8]) -> Result<(), E>,
    {
        if exact_len > 4294967294 {
            return Err(
                sbe_rt::EncodeError::VarDataTooLong {
                    field: "symbolTable",
                    max_length: 4294967294,
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
        Ok(DynamicRowV2Complete {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + exact_len,
        })
    }
}
impl<'a> DynamicRowV2Complete<'a> {
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
impl<'a> AsRef<[u8]> for DynamicRowV2Complete<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a> sbe_rt::private::Sealed for DynamicRowV2Encoder<'a> {}
impl<'a> sbe_rt::SbeMessage for DynamicRowV2Encoder<'a> {
    const TEMPLATE_ID: u16 = 4;
    const BLOCK_LENGTH: usize = 4;
    const SCHEMA_ID: u16 = 1000;
    const SCHEMA_VERSION: u16 = 1;
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicRowV2RowMetadataEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicRowV2RowMetadataEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [4, 0, 0, 0];
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
        F: FnOnce(&mut DynamicRowV2RowMetadataEntryEncoder<'b>),
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
            let mut __entry = DynamicRowV2RowMetadataEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct DynamicRowV2RowMetadataEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2RowMetadataEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn key_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn val_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 2;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicRowV2Int64FieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicRowV2Int64FieldsEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [9, 0, 0, 0];
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
        F: FnOnce(&mut DynamicRowV2Int64FieldsEntryEncoder<'b>),
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
            let mut __entry = DynamicRowV2Int64FieldsEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct DynamicRowV2Int64FieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2Int64FieldsEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn field_id(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn value(&mut self, val: i64) -> &mut Self {
        let offset = self.entry_start + 1;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicRowV2Uint64FieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicRowV2Uint64FieldsEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [9, 0, 0, 0];
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
        F: FnOnce(&mut DynamicRowV2Uint64FieldsEntryEncoder<'b>),
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
            let mut __entry = DynamicRowV2Uint64FieldsEntryEncoder::wrap(
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
pub struct DynamicRowV2Uint64FieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2Uint64FieldsEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn field_id(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn value(&mut self, val: u64) -> &mut Self {
        let offset = self.entry_start + 1;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicRowV2Float64FieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicRowV2Float64FieldsEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [9, 0, 0, 0];
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
        F: FnOnce(&mut DynamicRowV2Float64FieldsEntryEncoder<'b>),
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
            let mut __entry = DynamicRowV2Float64FieldsEntryEncoder::wrap(
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
pub struct DynamicRowV2Float64FieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2Float64FieldsEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn field_id(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn value(&mut self, val: f64) -> &mut Self {
        let offset = self.entry_start + 1;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicRowV2BoolFieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicRowV2BoolFieldsEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 2;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [2, 0, 0, 0];
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
        F: FnOnce(&mut DynamicRowV2BoolFieldsEntryEncoder<'b>),
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
            let mut __entry = DynamicRowV2BoolFieldsEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct DynamicRowV2BoolFieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2BoolFieldsEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 2;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn field_id(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn value(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 1;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicRowV2StringFieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicRowV2StringFieldsEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 3;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [3, 0, 0, 0];
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
        F: FnOnce(&mut DynamicRowV2StringFieldsEntryEncoder<'b>),
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
            let mut __entry = DynamicRowV2StringFieldsEntryEncoder::wrap(
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
pub struct DynamicRowV2StringFieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2StringFieldsEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 3;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn field_id(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn str_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 1;
        self.buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
        self
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicRowV2NullFieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicRowV2NullFieldsEncoder<'a> {
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
        F: FnOnce(&mut DynamicRowV2NullFieldsEntryEncoder<'b>),
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
            let mut __entry = DynamicRowV2NullFieldsEntryEncoder::wrap(__buf, self.pos);
            f(&mut __entry);
            self.pos = __entry.pos;
        }
        self.written += 1;
        Ok(())
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct DynamicRowV2NullFieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2NullFieldsEntryEncoder<'a> {
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
    pub fn field_id(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct DynamicRowV2DecimalArrayFieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicRowV2DecimalArrayFieldsEncoder<'a> {
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
        F: FnOnce(&mut DynamicRowV2DecimalArrayFieldsEntryEncoder<'b>),
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
            let mut __entry = DynamicRowV2DecimalArrayFieldsEntryEncoder::wrap(
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
pub struct DynamicRowV2DecimalArrayFieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2DecimalArrayFieldsEntryEncoder<'a> {
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
    pub fn field_id(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn values<F>(
        &mut self,
        count: u16,
        f: F,
    ) -> Result<&mut Self, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut DynamicRowV2DecimalArrayFieldsValuesEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(
                &DynamicRowV2DecimalArrayFieldsValuesEncoder::GROUP_DIM_TEMPLATE,
            );
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let __pos;
        {
            let __buf: &'a mut [u8] = unsafe { &mut *(self.buf as *mut [u8]) };
            let mut group = DynamicRowV2DecimalArrayFieldsValuesEncoder::wrap(
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
pub struct DynamicRowV2DecimalArrayFieldsValuesEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> DynamicRowV2DecimalArrayFieldsValuesEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    pub const GROUP_DIM_TEMPLATE: [u8; 4] = [9, 0, 0, 0];
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
        F: FnOnce(&mut DynamicRowV2DecimalArrayFieldsValuesEntryEncoder<'b>),
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
            let mut __entry = DynamicRowV2DecimalArrayFieldsValuesEntryEncoder::wrap(
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
pub struct DynamicRowV2DecimalArrayFieldsValuesEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> DynamicRowV2DecimalArrayFieldsValuesEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    #[must_use]
    pub fn mantissa(&mut self, val: i64) -> &mut Self {
        let offset = self.entry_start + 0;
        self.buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
        self
    }
    #[must_use]
    pub fn exponent(&mut self, val: i8) -> &mut Self {
        let offset = self.entry_start + 8;
        self.buf[offset..offset + 1].copy_from_slice(&val.to_le_bytes());
        self
    }
}
pub mod dynamic_row_v2_field_meta {
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
            name: "schemaId",
            id: 1,
            offset: 0,
            since_version: 0,
            field_type: "u32",
            presence: "required",
            null_value: Some("4294967295"),
            semantic_type: None,
            description: None,
        },
    ];
}
pub const SEMANTIC_VERSION: &str = "2.0.0";
pub const SCHEMA_HASH: u64 = 13794582440173905638;
pub const SCHEMA_SHA256: [u8; 32] = [
    0xaa, 0xd9, 0x8a, 0x79, 0x09, 0x51, 0x67, 0xe1, 0xbd, 0x4c, 0xdf, 0x34, 0xd8, 0x35,
    0x15, 0xb7, 0x26, 0x65, 0xc7, 0x22, 0x6d, 0xed, 0x2c, 0x04, 0xdc, 0xe6, 0xe2, 0xdf,
    0x9c, 0x23, 0x18, 0x9c,
];
pub const SCHEMA_SHA256_HEX: &str = "aad98a79095167e1bd4cdf34d83515b72665c7226ded2c04dce6e2df9c23189c";
pub const SCHEMA_ID: u16 = 1000;
pub const SCHEMA_VERSION: u16 = 1;
pub mod prelude {
    pub use super::sbe_rt::{DecodeError, EncodeError, VerifyError, SbeMessage};
    pub use super::{
        AnyMessage, DecodedFrame, FrameCursor, FramingPolicy, MessageVisitor,
        MessageHeader, MessageHeaderDecoder, GroupSize16Encoding,
        GroupSize16EncodingDecoder, VarString16Encoding, VarString16EncodingDecoder,
        SymbolTableEncoding, SymbolTableEncodingDecoder, Decimal, DecimalDecoder,
        DynamicSchemaV2Decoder, DynamicSchemaV2Encoder, DynamicRowV2Decoder,
        DynamicRowV2Encoder,
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
    DynamicSchemaV2(DynamicSchemaV2Decoder<'a>),
    DynamicRowV2(DynamicRowV2Decoder<'a>),
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
        if schema_id != 1000 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 1000,
                actual: schema_id,
                expected_name: "org.ergo.sbe.persist.v2",
            });
        }
        match template_id {
            3 => {
                Ok(
                    Self::DynamicSchemaV2(
                        DynamicSchemaV2Decoder::wrap(
                            buf,
                            body_pos,
                            block_length,
                            version,
                        ),
                    ),
                )
            }
            4 => {
                Ok(
                    Self::DynamicRowV2(
                        DynamicRowV2Decoder::wrap(buf, body_pos, block_length, version),
                    ),
                )
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
        if schema_id != 1000 {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: 1000,
                actual: schema_id,
                expected_name: "org.ergo.sbe.persist.v2",
            });
        }
        match template_id {
            3 => {
                let decoder = DynamicSchemaV2Decoder::wrap(
                    buf,
                    body_pos,
                    block_length,
                    version,
                );
                let total_len = decoder.encoded_length_with_header()?;
                if total_len > frame_len {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "DynamicSchemaV2",
                        needed: total_len,
                        available: frame_len,
                    });
                }
                Ok(DecodedFrame {
                    message: Self::DynamicSchemaV2(decoder),
                    range: pos..pos + total_len,
                    len: total_len,
                })
            }
            4 => {
                let decoder = DynamicRowV2Decoder::wrap(
                    buf,
                    body_pos,
                    block_length,
                    version,
                );
                let total_len = decoder.encoded_length_with_header()?;
                if total_len > frame_len {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "DynamicRowV2",
                        needed: total_len,
                        available: frame_len,
                    });
                }
                Ok(DecodedFrame {
                    message: Self::DynamicRowV2(decoder),
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
            Self::DynamicSchemaV2(d) => d.encoded_length_with_header(),
            Self::DynamicRowV2(d) => d.encoded_length_with_header(),
            Self::Unknown { payload, .. } => Ok(payload.len()),
        }
    }
}
impl<'a> AnyMessage<'a> {
    #[inline]
    pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        match self {
            Self::DynamicSchemaV2(d) => d.as_bytes(),
            Self::DynamicRowV2(d) => d.as_bytes(),
            Self::Unknown { payload, .. } => Ok(payload),
        }
    }
}
impl<'a> AnyMessage<'a> {
    #[inline]
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
        match self {
            Self::DynamicSchemaV2(d) => {
                let len = d.encoded_length_with_header()?;
                buf[..len].copy_from_slice(d.as_bytes()?);
                Ok(len)
            }
            Self::DynamicRowV2(d) => {
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
    fn visit_dynamic_schema_v2(
        &mut self,
        decoder: &DynamicSchemaV2Decoder<'_>,
    ) -> Self::Output;
    fn visit_dynamic_row_v2(
        &mut self,
        decoder: &DynamicRowV2Decoder<'_>,
    ) -> Self::Output;
}
impl<'a> AnyMessage<'a> {
    pub fn visit<V: MessageVisitor>(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::DynamicSchemaV2(d) => visitor.visit_dynamic_schema_v2(d),
            Self::DynamicRowV2(d) => visitor.visit_dynamic_row_v2(d),
            Self::Unknown { .. } => unimplemented!(),
        }
    }
}

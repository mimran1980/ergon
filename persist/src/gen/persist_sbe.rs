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
        GroupFull { declared: u16, attempted: u16 },
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
#[repr(transparent)]
pub struct MessageHeader(pub [u8; 8]);
impl MessageHeader {
    #[inline]
    pub const fn block_length(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn template_id(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[2 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn schema_id(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[4 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn version(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[6 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    pub const fn new(
        block_length: u16,
        template_id: u16,
        schema_id: u16,
        version: u16,
    ) -> Self {
        let mut bytes = [0u8; 8];
        let val_bytes = block_length.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            bytes[0 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = template_id.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            bytes[2 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = schema_id.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            bytes[4 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = version.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            bytes[6 + j] = val_bytes[j];
            j += 1;
        }
        Self(bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct GroupSize16Encoding(pub [u8; 4]);
impl GroupSize16Encoding {
    #[inline]
    pub const fn block_length(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn num_in_group(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[2 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    pub const fn new(block_length: u16, num_in_group: u16) -> Self {
        let mut bytes = [0u8; 4];
        let val_bytes = block_length.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            bytes[0 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = num_in_group.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            bytes[2 + j] = val_bytes[j];
            j += 1;
        }
        Self(bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarString16Encoding(pub [u8; 2]);
impl VarString16Encoding {
    #[inline]
    pub const fn length(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn var_data(&self) -> [u8; 0] {
        let mut res = [0 as u8; 0];
        let mut idx = 0;
        while idx < 0 {
            let offset = 2 + idx * 1;
            let mut bytes = [0u8; 1];
            let mut j = 0;
            while j < 1 {
                bytes[j] = self.0[offset + j];
                j += 1;
            }
            res[idx] = u8::from_le_bytes(bytes);
            idx += 1;
        }
        res
    }
    pub const fn new(length: u16, var_data: [u8; 0]) -> Self {
        let mut bytes = [0u8; 2];
        let val_bytes = length.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            bytes[0 + j] = val_bytes[j];
            j += 1;
        }
        let mut idx = 0;
        while idx < 0 {
            let val_bytes = var_data[idx].to_le_bytes();
            let mut j = 0;
            while j < 1 {
                bytes[2 + idx * 1 + j] = val_bytes[j];
                j += 1;
            }
            idx += 1;
        }
        Self(bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct SymbolTableEncoding(pub [u8; 4]);
impl SymbolTableEncoding {
    #[inline]
    pub const fn length(&self) -> u32 {
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u32::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn var_data(&self) -> [u8; 0] {
        let mut res = [0 as u8; 0];
        let mut idx = 0;
        while idx < 0 {
            let offset = 4 + idx * 1;
            let mut bytes = [0u8; 1];
            let mut j = 0;
            while j < 1 {
                bytes[j] = self.0[offset + j];
                j += 1;
            }
            res[idx] = u8::from_le_bytes(bytes);
            idx += 1;
        }
        res
    }
    pub const fn new(length: u32, var_data: [u8; 0]) -> Self {
        let mut bytes = [0u8; 4];
        let val_bytes = length.to_le_bytes();
        let mut j = 0;
        while j < 4 {
            bytes[0 + j] = val_bytes[j];
            j += 1;
        }
        let mut idx = 0;
        while idx < 0 {
            let val_bytes = var_data[idx].to_le_bytes();
            let mut j = 0;
            while j < 1 {
                bytes[4 + idx * 1 + j] = val_bytes[j];
                j += 1;
            }
            idx += 1;
        }
        Self(bytes)
    }
}
/// Dynamic table schema registration
#[derive(Clone, Copy)]
pub struct DynamicSchemaDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicSchemaDecoder<'a> {
    pub const SCHEMA_ID: u16 = 1000;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 4;
    /// MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    #[inline]
    pub const fn wrap(
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
        #[cfg(not(feature = "bound-check-disabled"))]
        let header_bytes: [u8; 8] = buf
            .get(pos..pos + 8)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "message header",
                    needed: 8,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        #[cfg(feature = "bound-check-disabled")]
        let header_bytes: [u8; 8] = unsafe {
            core::ptr::read_unaligned(buf.as_ptr().add(pos) as *const [u8; 8])
        };
        let header = MessageHeader(header_bytes);
        if header.schema_id() != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: header.schema_id(),
                expected_name: "org.ergo.sbe.persist",
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
        u32::from_le_bytes(self.buf[offset..][..4].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn schema_id_unchecked(&self) -> u32 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 4];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 4)
            });
        u32::from_le_bytes(bytes)
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = MetadataEntryDecoder::skip(
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = ColumnsEntryDecoder::skip(
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
        let bytes: [u8; 2] = self.buf[start..start + 2].try_into().unwrap();
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
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
    pub fn metadata(&self) -> Result<MetadataDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        MetadataDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn columns(&self) -> Result<ColumnsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        ColumnsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn table_name(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        let bytes: [u8; 2] = self.buf[offset..offset + 2].try_into().unwrap();
        let header = VarString16Encoding(bytes);
        let len = header.length() as usize;
        if len > 65534 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "table_name",
                length: len as u32,
                max_length: 65534,
            });
        }
        let data_offset = offset + 2;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    pub fn table_name_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.table_name()?;
        core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))
    }
    #[inline]
    pub unsafe fn table_name_as_str_unchecked(&self) -> &'a str {
        let data = self.table_name().unwrap_or(&[]);
        unsafe { core::str::from_utf8_unchecked(data) }
    }
    #[inline]
    pub fn table_name_as_string(&self) -> Result<String, sbe_rt::DecodeError> {
        Ok(self.table_name_as_str()?.to_string())
    }
    #[inline]
    pub fn table_name_as_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.table_name()
    }
    #[inline]
    pub fn symbol_table(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_3()?;
        let bytes: [u8; 4] = self.buf[offset..offset + 4].try_into().unwrap();
        let header = SymbolTableEncoding(bytes);
        let len = header.length() as usize;
        if len > 4294967294 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "symbol_table",
                length: len as u32,
                max_length: 4294967294,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    pub fn symbol_table_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.symbol_table()?;
        core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))
    }
    #[inline]
    pub unsafe fn symbol_table_as_str_unchecked(&self) -> &'a str {
        let data = self.symbol_table().unwrap_or(&[]);
        unsafe { core::str::from_utf8_unchecked(data) }
    }
    #[inline]
    pub fn symbol_table_as_string(&self) -> Result<String, sbe_rt::DecodeError> {
        Ok(self.symbol_table_as_str()?.to_string())
    }
    #[inline]
    pub fn symbol_table_as_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.symbol_table()
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
        let header_bytes: [u8; 8] = buf[..8].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            if offset + 2 > buf.len() {
                return Err(sbe_rt::VerifyError::VarDataOutOfBounds {
                    field: "table_name",
                    offset,
                    length: 0,
                });
            }
            let bytes: [u8; 2] = buf[offset..offset + 2].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
impl<'a> TryFrom<&'a [u8]> for DynamicSchemaDecoder<'a> {
    type Error = sbe_rt::DecodeError;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::wrap_and_apply_header(buf, 0)
    }
}
impl<'a> sbe_rt::private::Sealed for DynamicSchemaDecoder<'a> {}
impl<'a> sbe_rt::SbeMessage for DynamicSchemaDecoder<'a> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 4;
    const SCHEMA_ID: u16 = 1000;
    const SCHEMA_VERSION: u16 = 0;
}
impl<'a> AsRef<[u8]> for DynamicSchemaDecoder<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes().unwrap_or(&[])
    }
}
impl<'a> DynamicSchemaDecoder<'a> {
    pub fn as_ref_opt(&self) -> Option<&[u8]> {
        self.as_bytes().ok()
    }
}
impl<'a> core::fmt::Display for DynamicSchemaDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicSchema {{ ")?;
        {
            let v = self.schema_id();
            write!(f, "schema_id: {}", v)?;
        }
        if let Ok(g) = self.metadata() {
            write!(f, ", metadata: {} entries", g.len())?;
        }
        if let Ok(g) = self.columns() {
            write!(f, ", columns: {} entries", g.len())?;
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
pub struct MetadataDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> MetadataDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = buf
            .get(pos..pos + 4)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "metadata",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline]
    pub const fn remaining(&self) -> usize {
        { self.count }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        {
            self.pos = self.start;
            self.count = self.total;
            self
        }
    }
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "metadata",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<MetadataEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "metadata",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "metadata",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(MetadataEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 4]], sbe_rt::DecodeError> {
        let len = self.count * 4;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "metadata",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<4>();
        Ok(chunks)
    }
}
impl<'a> Iterator for MetadataDecoder<'a> {
    type Item = MetadataEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = MetadataEntryDecoder::wrap(self.buf, self.pos, self.acting_version);
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for MetadataDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct MetadataEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> MetadataEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    #[inline]
    pub fn key_len(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn key_len_unchecked(&self) -> u16 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 2];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 2)
            });
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_key_len(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.key_len_unchecked() }
    }
    pub const KEY_LEN_NULL: u16 = 65535_u16;
    pub const KEY_LEN_MIN: u16 = 0_u16;
    pub const KEY_LEN_MAX: u16 = 65534_u16;
    #[inline]
    pub fn val_len(&self) -> u16 {
        let offset = self.pos + 2;
        u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn val_len_unchecked(&self) -> u16 {
        let offset = self.pos + 2;
        let mut bytes = [0u8; 2];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 2)
            });
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_val_len(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.val_len_unchecked() }
    }
    pub const VAL_LEN_NULL: u16 = 65535_u16;
    pub const VAL_LEN_MIN: u16 = 0_u16;
    pub const VAL_LEN_MAX: u16 = 65534_u16;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_0()? - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_0()
    }
}
pub struct ColumnsDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> ColumnsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = buf
            .get(pos..pos + 4)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "columns",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline]
    pub const fn remaining(&self) -> usize {
        { self.count }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        {
            self.pos = self.start;
            self.count = self.total;
            self
        }
    }
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "columns",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<ColumnsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "columns",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "columns",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(ColumnsEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 4]], sbe_rt::DecodeError> {
        let len = self.count * 4;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "columns",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<4>();
        Ok(chunks)
    }
}
impl<'a> Iterator for ColumnsDecoder<'a> {
    type Item = ColumnsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = ColumnsEntryDecoder::wrap(self.buf, self.pos, self.acting_version);
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for ColumnsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct ColumnsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> ColumnsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    #[inline]
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn field_id_unchecked(&self) -> u8 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_field_id(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.field_id_unchecked() }
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn name_len(&self) -> u16 {
        let offset = self.pos + 1;
        u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn name_len_unchecked(&self) -> u16 {
        let offset = self.pos + 1;
        let mut bytes = [0u8; 2];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 2)
            });
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_name_len(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.name_len_unchecked() }
    }
    pub const NAME_LEN_NULL: u16 = 65535_u16;
    pub const NAME_LEN_MIN: u16 = 0_u16;
    pub const NAME_LEN_MAX: u16 = 65534_u16;
    #[inline]
    pub fn type_tag(&self) -> u8 {
        let offset = self.pos + 3;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn type_tag_unchecked(&self) -> u8 {
        let offset = self.pos + 3;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_type_tag(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.type_tag_unchecked() }
    }
    pub const TYPE_TAG_NULL: u8 = 255_u8;
    pub const TYPE_TAG_MIN: u8 = 0_u8;
    pub const TYPE_TAG_MAX: u8 = 254_u8;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_0()? - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_0()
    }
}
pub mod dynamic_schema_encoder_state {
    pub struct NeedsMetadata;
    pub struct NeedsColumns;
    pub struct NeedsTableName;
    pub struct NeedsSymbolTable;
    pub struct Complete;
}
#[must_use]
pub struct DynamicSchemaEncoder<
    'a,
    State = dynamic_schema_encoder_state::NeedsMetadata,
> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
    _phantom: core::marker::PhantomData<State>,
}
impl<'a, State> DynamicSchemaEncoder<'a, State> {
    pub const SCHEMA_ID: u16 = 1000;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 4;
    /// MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [4, 0, 1, 0, 232, 3, 0, 0];
    const _HEADER_TEMPLATE_LEN: () = assert!(Self::HEADER_TEMPLATE.len() == 8);
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            message_start: pos,
            pos: pos + 8 + 4,
            _phantom: core::marker::PhantomData,
        }
    }
    #[inline]
    pub fn wrap_and_apply_header(
        buf: &'a mut [u8],
        pos: usize,
    ) -> Result<Self, sbe_rt::EncodeError> {
        let needed = 8 + 4;
        if pos + needed > buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: buf.len() - pos,
            });
        }
        buf[pos..pos + 8].copy_from_slice(&Self::HEADER_TEMPLATE);
        Ok(Self::wrap(buf, pos))
    }
    #[must_use]
    pub fn schema_id(&mut self, val: u32) -> &mut Self {
        let offset = self.message_start + 8 + 0;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 4].copy_from_slice(&val_bytes);
        self
    }
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.pos - (self.message_start + 8)
    }
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.pos - self.message_start
    }
}
impl<'a> DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::NeedsMetadata> {
    #[must_use]
    pub fn metadata<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::NeedsColumns>,
        sbe_rt::EncodeError,
    >
    where
        F: FnOnce(&mut MetadataEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&MetadataEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = MetadataEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(DynamicSchemaEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::NeedsColumns> {
    #[must_use]
    pub fn columns<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::NeedsTableName>,
        sbe_rt::EncodeError,
    >
    where
        F: FnOnce(&mut ColumnsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&ColumnsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = ColumnsEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(DynamicSchemaEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::NeedsTableName> {
    #[must_use]
    pub fn table_name(
        mut self,
        data: &[u8],
    ) -> Result<
        DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::NeedsSymbolTable>,
        sbe_rt::EncodeError,
    > {
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
        Ok(DynamicSchemaEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
    #[must_use]
    pub fn table_name_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<
        DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::NeedsSymbolTable>,
        sbe_rt::EncodeError,
    > {
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
        Ok(DynamicSchemaEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::NeedsSymbolTable> {
    #[must_use]
    pub fn symbol_table(
        mut self,
        data: &[u8],
    ) -> Result<
        DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::Complete>,
        sbe_rt::EncodeError,
    > {
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
        Ok(DynamicSchemaEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
    #[must_use]
    pub fn symbol_table_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<
        DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::Complete>,
        sbe_rt::EncodeError,
    > {
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
        Ok(DynamicSchemaEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::Complete> {
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.message_start..self.pos]
    }
}
impl<'a> AsRef<[u8]>
for DynamicSchemaEncoder<'a, dynamic_schema_encoder_state::Complete> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a, State> sbe_rt::private::Sealed for DynamicSchemaEncoder<'a, State> {}
impl<'a, State> sbe_rt::SbeMessage for DynamicSchemaEncoder<'a, State> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 4;
    const SCHEMA_ID: u16 = 1000;
    const SCHEMA_VERSION: u16 = 0;
}
#[must_use]
pub struct MetadataEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> MetadataEncoder<'a> {
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
        F: FnOnce(&mut MetadataEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count,
                attempted: self.written + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        let mut entry = MetadataEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use]
pub struct MetadataEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> MetadataEntryEncoder<'a> {
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 2].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn val_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 2;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 2].copy_from_slice(&val_bytes);
        self
    }
}
#[must_use]
pub struct ColumnsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> ColumnsEncoder<'a> {
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
        F: FnOnce(&mut ColumnsEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count,
                attempted: self.written + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        let mut entry = ColumnsEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use]
pub struct ColumnsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> ColumnsEntryEncoder<'a> {
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
    pub fn field_id(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn name_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 1;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 2].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn type_tag(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 3;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
}
pub mod dynamic_schema_field_meta {
    pub struct FieldInfo {
        pub name: &'static str,
        pub id: u16,
        pub offset: usize,
        pub since_version: u16,
        pub field_type: &'static str,
    }
    pub const FIELDS: &[FieldInfo] = &[
        FieldInfo {
            name: "schemaId",
            id: 1,
            offset: 0,
            since_version: 0,
            field_type: "u32",
        },
    ];
}
/// Dynamic row values
#[derive(Clone, Copy)]
pub struct DynamicRowDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
    acting_block_length: usize,
}
impl<'a> DynamicRowDecoder<'a> {
    pub const SCHEMA_ID: u16 = 1000;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 2;
    pub const BLOCK_LENGTH: usize = 4;
    /// MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    #[inline]
    pub const fn wrap(
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
        #[cfg(not(feature = "bound-check-disabled"))]
        let header_bytes: [u8; 8] = buf
            .get(pos..pos + 8)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "message header",
                    needed: 8,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        #[cfg(feature = "bound-check-disabled")]
        let header_bytes: [u8; 8] = unsafe {
            core::ptr::read_unaligned(buf.as_ptr().add(pos) as *const [u8; 8])
        };
        let header = MessageHeader(header_bytes);
        if header.schema_id() != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: header.schema_id(),
                expected_name: "org.ergo.sbe.persist",
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
        u32::from_le_bytes(self.buf[offset..][..4].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn schema_id_unchecked(&self) -> u32 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 4];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 4)
            });
        u32::from_le_bytes(bytes)
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = RowMetadataEntryDecoder::skip(
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = Int64FieldsEntryDecoder::skip(
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = Uint64FieldsEntryDecoder::skip(
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = Float64FieldsEntryDecoder::skip(
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = BoolFieldsEntryDecoder::skip(
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = StringFieldsEntryDecoder::skip(
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = NullFieldsEntryDecoder::skip(
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
                field: "symbolTable",
                needed: 4,
                available: self.buf.len() - start,
            });
        }
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
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
    pub fn row_metadata(&self) -> Result<RowMetadataDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        RowMetadataDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn int64_fields(&self) -> Result<Int64FieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        Int64FieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn uint64_fields(&self) -> Result<Uint64FieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        Uint64FieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn float64_fields(
        &self,
    ) -> Result<Float64FieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_3()?;
        Float64FieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn bool_fields(&self) -> Result<BoolFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_4()?;
        BoolFieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn string_fields(&self) -> Result<StringFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_5()?;
        StringFieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn null_fields(&self) -> Result<NullFieldsDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_6()?;
        NullFieldsDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn symbol_table(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_7()?;
        let bytes: [u8; 4] = self.buf[offset..offset + 4].try_into().unwrap();
        let header = SymbolTableEncoding(bytes);
        let len = header.length() as usize;
        if len > 4294967294 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "symbol_table",
                length: len as u32,
                max_length: 4294967294,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    pub fn symbol_table_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.symbol_table()?;
        core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))
    }
    #[inline]
    pub unsafe fn symbol_table_as_str_unchecked(&self) -> &'a str {
        let data = self.symbol_table().unwrap_or(&[]);
        unsafe { core::str::from_utf8_unchecked(data) }
    }
    #[inline]
    pub fn symbol_table_as_string(&self) -> Result<String, sbe_rt::DecodeError> {
        Ok(self.symbol_table_as_str()?.to_string())
    }
    #[inline]
    pub fn symbol_table_as_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.symbol_table()
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        let end = self.tail_offset_8()?;
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
        let header_bytes: [u8; 8] = buf[..8].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
impl<'a> TryFrom<&'a [u8]> for DynamicRowDecoder<'a> {
    type Error = sbe_rt::DecodeError;
    fn try_from(buf: &'a [u8]) -> Result<Self, Self::Error> {
        Self::wrap_and_apply_header(buf, 0)
    }
}
impl<'a> sbe_rt::private::Sealed for DynamicRowDecoder<'a> {}
impl<'a> sbe_rt::SbeMessage for DynamicRowDecoder<'a> {
    const TEMPLATE_ID: u16 = 2;
    const BLOCK_LENGTH: usize = 4;
    const SCHEMA_ID: u16 = 1000;
    const SCHEMA_VERSION: u16 = 0;
}
impl<'a> AsRef<[u8]> for DynamicRowDecoder<'a> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes().unwrap_or(&[])
    }
}
impl<'a> DynamicRowDecoder<'a> {
    pub fn as_ref_opt(&self) -> Option<&[u8]> {
        self.as_bytes().ok()
    }
}
impl<'a> core::fmt::Display for DynamicRowDecoder<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "DynamicRow {{ ")?;
        {
            let v = self.schema_id();
            write!(f, "schema_id: {}", v)?;
        }
        if let Ok(g) = self.row_metadata() {
            write!(f, ", row_metadata: {} entries", g.len())?;
        }
        if let Ok(g) = self.int64_fields() {
            write!(f, ", int64_fields: {} entries", g.len())?;
        }
        if let Ok(g) = self.uint64_fields() {
            write!(f, ", uint64_fields: {} entries", g.len())?;
        }
        if let Ok(g) = self.float64_fields() {
            write!(f, ", float64_fields: {} entries", g.len())?;
        }
        if let Ok(g) = self.bool_fields() {
            write!(f, ", bool_fields: {} entries", g.len())?;
        }
        if let Ok(g) = self.string_fields() {
            write!(f, ", string_fields: {} entries", g.len())?;
        }
        if let Ok(g) = self.null_fields() {
            write!(f, ", null_fields: {} entries", g.len())?;
        }
        if let Ok(d) = self.symbol_table() {
            write!(f, ", symbol_table: {} bytes", d.len())?;
        }
        write!(f, " }}")
    }
}
pub struct RowMetadataDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> RowMetadataDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = buf
            .get(pos..pos + 4)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "rowMetadata",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline]
    pub const fn remaining(&self) -> usize {
        { self.count }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        {
            self.pos = self.start;
            self.count = self.total;
            self
        }
    }
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "rowMetadata",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<RowMetadataEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "rowMetadata",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "rowMetadata",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(RowMetadataEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 4]], sbe_rt::DecodeError> {
        let len = self.count * 4;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "rowMetadata",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<4>();
        Ok(chunks)
    }
}
impl<'a> Iterator for RowMetadataDecoder<'a> {
    type Item = RowMetadataEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = RowMetadataEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for RowMetadataDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct RowMetadataEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> RowMetadataEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 4;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    #[inline]
    pub fn key_len(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn key_len_unchecked(&self) -> u16 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 2];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 2)
            });
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_key_len(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.key_len_unchecked() }
    }
    pub const KEY_LEN_NULL: u16 = 65535_u16;
    pub const KEY_LEN_MIN: u16 = 0_u16;
    pub const KEY_LEN_MAX: u16 = 65534_u16;
    #[inline]
    pub fn val_len(&self) -> u16 {
        let offset = self.pos + 2;
        u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn val_len_unchecked(&self) -> u16 {
        let offset = self.pos + 2;
        let mut bytes = [0u8; 2];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 2)
            });
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_val_len(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.val_len_unchecked() }
    }
    pub const VAL_LEN_NULL: u16 = 65535_u16;
    pub const VAL_LEN_MIN: u16 = 0_u16;
    pub const VAL_LEN_MAX: u16 = 65534_u16;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_0()? - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_0()
    }
}
pub struct Int64FieldsDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> Int64FieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = buf
            .get(pos..pos + 4)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "int64Fields",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline]
    pub const fn remaining(&self) -> usize {
        { self.count }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        {
            self.pos = self.start;
            self.count = self.total;
            self
        }
    }
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "int64Fields",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<Int64FieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "int64Fields",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "int64Fields",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(Int64FieldsEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 9]], sbe_rt::DecodeError> {
        let len = self.count * 9;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "int64Fields",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<9>();
        Ok(chunks)
    }
}
impl<'a> Iterator for Int64FieldsDecoder<'a> {
    type Item = Int64FieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = Int64FieldsEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for Int64FieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct Int64FieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> Int64FieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    #[inline]
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn field_id_unchecked(&self) -> u8 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_field_id(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.field_id_unchecked() }
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn value(&self) -> i64 {
        let offset = self.pos + 1;
        i64::from_le_bytes(self.buf[offset..][..8].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn value_unchecked(&self) -> i64 {
        let offset = self.pos + 1;
        let mut bytes = [0u8; 8];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 8)
            });
        i64::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_value(&self) -> i64 {
        #[allow(unused_unsafe)] unsafe { self.value_unchecked() }
    }
    pub const VALUE_NULL: i64 = -9223372036854775808_i64;
    pub const VALUE_MIN: i64 = -9223372036854775807_i64;
    pub const VALUE_MAX: i64 = 9223372036854775807_i64;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_0()? - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_0()
    }
}
pub struct Uint64FieldsDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> Uint64FieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = buf
            .get(pos..pos + 4)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "uint64Fields",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline]
    pub const fn remaining(&self) -> usize {
        { self.count }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        {
            self.pos = self.start;
            self.count = self.total;
            self
        }
    }
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "uint64Fields",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<Uint64FieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "uint64Fields",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "uint64Fields",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(Uint64FieldsEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 9]], sbe_rt::DecodeError> {
        let len = self.count * 9;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "uint64Fields",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<9>();
        Ok(chunks)
    }
}
impl<'a> Iterator for Uint64FieldsDecoder<'a> {
    type Item = Uint64FieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = Uint64FieldsEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for Uint64FieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct Uint64FieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> Uint64FieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    #[inline]
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn field_id_unchecked(&self) -> u8 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_field_id(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.field_id_unchecked() }
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn value(&self) -> u64 {
        let offset = self.pos + 1;
        u64::from_le_bytes(self.buf[offset..][..8].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn value_unchecked(&self) -> u64 {
        let offset = self.pos + 1;
        let mut bytes = [0u8; 8];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 8)
            });
        u64::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_value(&self) -> u64 {
        #[allow(unused_unsafe)] unsafe { self.value_unchecked() }
    }
    pub const VALUE_NULL: u64 = 18446744073709551615_u64;
    pub const VALUE_MIN: u64 = 0_u64;
    pub const VALUE_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_0()? - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_0()
    }
}
pub struct Float64FieldsDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> Float64FieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = buf
            .get(pos..pos + 4)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "float64Fields",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline]
    pub const fn remaining(&self) -> usize {
        { self.count }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        {
            self.pos = self.start;
            self.count = self.total;
            self
        }
    }
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "float64Fields",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<Float64FieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "float64Fields",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "float64Fields",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(Float64FieldsEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 9]], sbe_rt::DecodeError> {
        let len = self.count * 9;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "float64Fields",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<9>();
        Ok(chunks)
    }
}
impl<'a> Iterator for Float64FieldsDecoder<'a> {
    type Item = Float64FieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = Float64FieldsEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for Float64FieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct Float64FieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> Float64FieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 9;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    #[inline]
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn field_id_unchecked(&self) -> u8 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_field_id(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.field_id_unchecked() }
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn value(&self) -> f64 {
        let offset = self.pos + 1;
        f64::from_le_bytes(self.buf[offset..][..8].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn value_unchecked(&self) -> f64 {
        let offset = self.pos + 1;
        let mut bytes = [0u8; 8];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 8)
            });
        f64::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_value(&self) -> f64 {
        #[allow(unused_unsafe)] unsafe { self.value_unchecked() }
    }
    pub const VALUE_NULL: f64 = f64::from_bits(9221120237041090561);
    pub const VALUE_MIN: f64 = f64::from_bits(18442240474082181119);
    pub const VALUE_MAX: f64 = f64::from_bits(9218868437227405311);
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_0()? - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_0()
    }
}
pub struct BoolFieldsDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> BoolFieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 2;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = buf
            .get(pos..pos + 4)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "boolFields",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline]
    pub const fn remaining(&self) -> usize {
        { self.count }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        {
            self.pos = self.start;
            self.count = self.total;
            self
        }
    }
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "boolFields",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<BoolFieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "boolFields",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "boolFields",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(BoolFieldsEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 2]], sbe_rt::DecodeError> {
        let len = self.count * 2;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "boolFields",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<2>();
        Ok(chunks)
    }
}
impl<'a> Iterator for BoolFieldsDecoder<'a> {
    type Item = BoolFieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = BoolFieldsEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for BoolFieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct BoolFieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> BoolFieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 2;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    #[inline]
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn field_id_unchecked(&self) -> u8 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_field_id(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.field_id_unchecked() }
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn value(&self) -> u8 {
        let offset = self.pos + 1;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn value_unchecked(&self) -> u8 {
        let offset = self.pos + 1;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_value(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.value_unchecked() }
    }
    pub const VALUE_NULL: u8 = 255_u8;
    pub const VALUE_MIN: u8 = 0_u8;
    pub const VALUE_MAX: u8 = 254_u8;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_0()? - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_0()
    }
}
pub struct StringFieldsDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> StringFieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 3;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = buf
            .get(pos..pos + 4)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "stringFields",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline]
    pub const fn remaining(&self) -> usize {
        { self.count }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        {
            self.pos = self.start;
            self.count = self.total;
            self
        }
    }
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "stringFields",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<StringFieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "stringFields",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "stringFields",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(StringFieldsEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 3]], sbe_rt::DecodeError> {
        let len = self.count * 3;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "stringFields",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<3>();
        Ok(chunks)
    }
}
impl<'a> Iterator for StringFieldsDecoder<'a> {
    type Item = StringFieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = StringFieldsEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for StringFieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct StringFieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> StringFieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 3;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    #[inline]
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn field_id_unchecked(&self) -> u8 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_field_id(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.field_id_unchecked() }
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    pub fn str_len(&self) -> u16 {
        let offset = self.pos + 1;
        u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn str_len_unchecked(&self) -> u16 {
        let offset = self.pos + 1;
        let mut bytes = [0u8; 2];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 2)
            });
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_str_len(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.str_len_unchecked() }
    }
    pub const STR_LEN_NULL: u16 = 65535_u16;
    pub const STR_LEN_MIN: u16 = 0_u16;
    pub const STR_LEN_MAX: u16 = 65534_u16;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_0()? - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_0()
    }
}
pub struct NullFieldsDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> NullFieldsDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    #[inline]
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        let bytes: [u8; 4] = buf
            .get(pos..pos + 4)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "nullFields",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSize16Encoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            start: pos + 4,
            total: count,
            acting_version,
        })
    }
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    #[inline]
    pub const fn remaining(&self) -> usize {
        { self.count }
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        {
            self.pos = self.start;
            self.count = self.total;
            self
        }
    }
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "nullFields",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<NullFieldsEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "nullFields",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "nullFields",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(NullFieldsEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 1]], sbe_rt::DecodeError> {
        let len = self.count * 1;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "nullFields",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<1>();
        Ok(chunks)
    }
    #[inline]
    pub fn field_id_as_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let len = self.count * 1;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "nullFields",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        Ok(unsafe {
            core::slice::from_raw_parts(
                self.buf.as_ptr().add(self.pos) as *const u8,
                self.count,
            )
        })
    }
}
impl<'a> Iterator for NullFieldsDecoder<'a> {
    type Item = NullFieldsEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = NullFieldsEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for NullFieldsDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct NullFieldsEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> NullFieldsEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    #[inline]
    pub fn field_id(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn field_id_unchecked(&self) -> u8 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_field_id(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.field_id_unchecked() }
    }
    pub const FIELD_ID_NULL: u8 = 255_u8;
    pub const FIELD_ID_MIN: u8 = 0_u8;
    pub const FIELD_ID_MAX: u8 = 254_u8;
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.tail_offset_0()? - self.pos)
    }
    #[inline]
    pub fn skip(
        buf: &'a [u8],
        pos: usize,
        block_len: usize,
        acting_version: u16,
    ) -> Result<usize, sbe_rt::DecodeError> {
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_0()
    }
}
pub mod dynamic_row_encoder_state {
    pub struct NeedsRowMetadata;
    pub struct NeedsInt64Fields;
    pub struct NeedsUint64Fields;
    pub struct NeedsFloat64Fields;
    pub struct NeedsBoolFields;
    pub struct NeedsStringFields;
    pub struct NeedsNullFields;
    pub struct NeedsSymbolTable;
    pub struct Complete;
}
#[must_use]
pub struct DynamicRowEncoder<'a, State = dynamic_row_encoder_state::NeedsRowMetadata> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
    _phantom: core::marker::PhantomData<State>,
}
impl<'a, State> DynamicRowEncoder<'a, State> {
    pub const SCHEMA_ID: u16 = 1000;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 2;
    pub const BLOCK_LENGTH: usize = 4;
    /// MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [4, 0, 2, 0, 232, 3, 0, 0];
    const _HEADER_TEMPLATE_LEN: () = assert!(Self::HEADER_TEMPLATE.len() == 8);
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            message_start: pos,
            pos: pos + 8 + 4,
            _phantom: core::marker::PhantomData,
        }
    }
    #[inline]
    pub fn wrap_and_apply_header(
        buf: &'a mut [u8],
        pos: usize,
    ) -> Result<Self, sbe_rt::EncodeError> {
        let needed = 8 + 4;
        if pos + needed > buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: buf.len() - pos,
            });
        }
        buf[pos..pos + 8].copy_from_slice(&Self::HEADER_TEMPLATE);
        Ok(Self::wrap(buf, pos))
    }
    #[must_use]
    pub fn schema_id(&mut self, val: u32) -> &mut Self {
        let offset = self.message_start + 8 + 0;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 4].copy_from_slice(&val_bytes);
        self
    }
    #[inline]
    pub fn encoded_length(&self) -> usize {
        self.pos - (self.message_start + 8)
    }
    #[inline]
    pub fn encoded_length_with_header(&self) -> usize {
        self.pos - self.message_start
    }
}
impl<'a> DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsRowMetadata> {
    #[must_use]
    pub fn row_metadata<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsInt64Fields>,
        sbe_rt::EncodeError,
    >
    where
        F: FnOnce(&mut RowMetadataEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&RowMetadataEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = RowMetadataEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(DynamicRowEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsInt64Fields> {
    #[must_use]
    pub fn int64_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsUint64Fields>,
        sbe_rt::EncodeError,
    >
    where
        F: FnOnce(&mut Int64FieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&Int64FieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = Int64FieldsEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(DynamicRowEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsUint64Fields> {
    #[must_use]
    pub fn uint64_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsFloat64Fields>,
        sbe_rt::EncodeError,
    >
    where
        F: FnOnce(&mut Uint64FieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&Uint64FieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = Uint64FieldsEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(DynamicRowEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsFloat64Fields> {
    #[must_use]
    pub fn float64_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsBoolFields>,
        sbe_rt::EncodeError,
    >
    where
        F: FnOnce(&mut Float64FieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&Float64FieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = Float64FieldsEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(DynamicRowEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsBoolFields> {
    #[must_use]
    pub fn bool_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsStringFields>,
        sbe_rt::EncodeError,
    >
    where
        F: FnOnce(&mut BoolFieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&BoolFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = BoolFieldsEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(DynamicRowEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsStringFields> {
    #[must_use]
    pub fn string_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsNullFields>,
        sbe_rt::EncodeError,
    >
    where
        F: FnOnce(&mut StringFieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&StringFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = StringFieldsEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(DynamicRowEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsNullFields> {
    #[must_use]
    pub fn null_fields<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsSymbolTable>,
        sbe_rt::EncodeError,
    >
    where
        F: FnOnce(&mut NullFieldsEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&NullFieldsEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = NullFieldsEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(DynamicRowEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicRowEncoder<'a, dynamic_row_encoder_state::NeedsSymbolTable> {
    #[must_use]
    pub fn symbol_table(
        mut self,
        data: &[u8],
    ) -> Result<
        DynamicRowEncoder<'a, dynamic_row_encoder_state::Complete>,
        sbe_rt::EncodeError,
    > {
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
        Ok(DynamicRowEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
    #[must_use]
    pub fn symbol_table_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<
        DynamicRowEncoder<'a, dynamic_row_encoder_state::Complete>,
        sbe_rt::EncodeError,
    > {
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
        Ok(DynamicRowEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> DynamicRowEncoder<'a, dynamic_row_encoder_state::Complete> {
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.message_start..self.pos]
    }
}
impl<'a> AsRef<[u8]> for DynamicRowEncoder<'a, dynamic_row_encoder_state::Complete> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a, State> sbe_rt::private::Sealed for DynamicRowEncoder<'a, State> {}
impl<'a, State> sbe_rt::SbeMessage for DynamicRowEncoder<'a, State> {
    const TEMPLATE_ID: u16 = 2;
    const BLOCK_LENGTH: usize = 4;
    const SCHEMA_ID: u16 = 1000;
    const SCHEMA_VERSION: u16 = 0;
}
#[must_use]
pub struct RowMetadataEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> RowMetadataEncoder<'a> {
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
        F: FnOnce(&mut RowMetadataEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count,
                attempted: self.written + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        let mut entry = RowMetadataEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use]
pub struct RowMetadataEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> RowMetadataEntryEncoder<'a> {
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 2].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn val_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 2;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 2].copy_from_slice(&val_bytes);
        self
    }
}
#[must_use]
pub struct Int64FieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> Int64FieldsEncoder<'a> {
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
        F: FnOnce(&mut Int64FieldsEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count,
                attempted: self.written + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        let mut entry = Int64FieldsEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use]
pub struct Int64FieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> Int64FieldsEntryEncoder<'a> {
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn value(&mut self, val: i64) -> &mut Self {
        let offset = self.entry_start + 1;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 8].copy_from_slice(&val_bytes);
        self
    }
}
#[must_use]
pub struct Uint64FieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> Uint64FieldsEncoder<'a> {
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
        F: FnOnce(&mut Uint64FieldsEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count,
                attempted: self.written + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        let mut entry = Uint64FieldsEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use]
pub struct Uint64FieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> Uint64FieldsEntryEncoder<'a> {
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn value(&mut self, val: u64) -> &mut Self {
        let offset = self.entry_start + 1;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 8].copy_from_slice(&val_bytes);
        self
    }
}
#[must_use]
pub struct Float64FieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> Float64FieldsEncoder<'a> {
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
        F: FnOnce(&mut Float64FieldsEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count,
                attempted: self.written + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        let mut entry = Float64FieldsEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use]
pub struct Float64FieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> Float64FieldsEntryEncoder<'a> {
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn value(&mut self, val: f64) -> &mut Self {
        let offset = self.entry_start + 1;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 8].copy_from_slice(&val_bytes);
        self
    }
}
#[must_use]
pub struct BoolFieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> BoolFieldsEncoder<'a> {
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
        F: FnOnce(&mut BoolFieldsEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count,
                attempted: self.written + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        let mut entry = BoolFieldsEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use]
pub struct BoolFieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> BoolFieldsEntryEncoder<'a> {
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn value(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 1;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
}
#[must_use]
pub struct StringFieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> StringFieldsEncoder<'a> {
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
        F: FnOnce(&mut StringFieldsEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count,
                attempted: self.written + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        let mut entry = StringFieldsEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use]
pub struct StringFieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> StringFieldsEntryEncoder<'a> {
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn str_len(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 1;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 2].copy_from_slice(&val_bytes);
        self
    }
}
#[must_use]
pub struct NullFieldsEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> NullFieldsEncoder<'a> {
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
        F: FnOnce(&mut NullFieldsEntryEncoder<'b>),
    {
        if self.written >= self.count {
            return Err(sbe_rt::EncodeError::GroupFull {
                declared: self.count,
                attempted: self.written + 1,
            });
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: block_len,
                available: self.buf.len() - self.pos,
            });
        }
        let mut entry = NullFieldsEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use]
pub struct NullFieldsEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> NullFieldsEntryEncoder<'a> {
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
}
pub mod dynamic_row_field_meta {
    pub struct FieldInfo {
        pub name: &'static str,
        pub id: u16,
        pub offset: usize,
        pub since_version: u16,
        pub field_type: &'static str,
    }
    pub const FIELDS: &[FieldInfo] = &[
        FieldInfo {
            name: "schemaId",
            id: 1,
            offset: 0,
            since_version: 0,
            field_type: "u32",
        },
    ];
}
pub const SEMANTIC_VERSION: &str = "1.0.0";
pub const SCHEMA_HASH: u64 = 17568668021833792841;
pub const SCHEMA_SHA256: [u8; 32] = [
    0xd4, 0xad, 0x7a, 0xa6, 0xa6, 0x88, 0xca, 0x12, 0xbf, 0x33, 0x6e, 0xcb, 0xb6, 0xf8,
    0x02, 0xc1, 0xfa, 0x64, 0xdd, 0x73, 0x3e, 0xce, 0xda, 0xb3, 0x59, 0x5b, 0x33, 0xe7,
    0x0d, 0x67, 0x49, 0x3e,
];
pub const SCHEMA_SHA256_HEX: &str = "d4ad7aa6a688ca12bf336ecbb6f802c1fa64dd733ecedab3595b33e70d67493e";
#[inline]
pub const fn schema_id_from_header(buf: &[u8]) -> Option<u16> {
    if buf.len() < 4 + 2 {
        return None;
    }
    let mut bytes = [0u8; 2];
    let mut j = 0;
    while j < 2 {
        bytes[j] = buf[4 + j];
        j += 1;
    }
    Some(u16::from_le_bytes(bytes))
}
#[non_exhaustive]
#[derive(Clone, Copy)]
pub enum AnyMessage<'a> {
    DynamicSchema(DynamicSchemaDecoder<'a>),
    DynamicRow(DynamicRowDecoder<'a>),
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
                let bytes: [u8; 4] = self
                    .buf[self.pos..self.pos + 4]
                    .try_into()
                    .unwrap();
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
                let bytes: [u8; 2] = self
                    .buf[self.pos..self.pos + 2]
                    .try_into()
                    .unwrap();
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
    pub const fn decode(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
        if pos + 8 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "message header",
                needed: 8,
                available: buf.len() - pos,
            });
        }
        let mut header_bytes = [0u8; 8];
        let mut j = 0;
        while j < 8 {
            header_bytes[j] = buf[pos + j];
            j += 1;
        }
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
                expected_name: "org.ergo.sbe.persist",
            });
        }
        match template_id {
            1 => {
                Ok(
                    Self::DynamicSchema(
                        DynamicSchemaDecoder::wrap(buf, body_pos, block_length, version),
                    ),
                )
            }
            2 => {
                Ok(
                    Self::DynamicRow(
                        DynamicRowDecoder::wrap(buf, body_pos, block_length, version),
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
    #[inline]
    pub fn decode_frame(
        buf: &'a [u8],
        pos: usize,
        frame_len: usize,
    ) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
        let header_bytes: [u8; 8] = buf
            .get(pos..pos + 8)
            .ok_or_else(|| {
                sbe_rt::DecodeError::BufferTooShort {
                    field: "decoded frame",
                    needed: 8,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
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
                expected_name: "org.ergo.sbe.persist",
            });
        }
        match template_id {
            1 => {
                let decoder = DynamicSchemaDecoder::wrap(
                    buf,
                    body_pos,
                    block_length,
                    version,
                );
                let total_len = match decoder.encoded_length_with_header() {
                    Ok(len) => len,
                    Err(e) => return Err(e),
                };
                if total_len > frame_len {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "DynamicSchema",
                        needed: total_len,
                        available: frame_len,
                    });
                }
                Ok(DecodedFrame {
                    message: Self::DynamicSchema(decoder),
                    range: pos..pos + total_len,
                    len: total_len,
                })
            }
            2 => {
                let decoder = DynamicRowDecoder::wrap(
                    buf,
                    body_pos,
                    block_length,
                    version,
                );
                let total_len = match decoder.encoded_length_with_header() {
                    Ok(len) => len,
                    Err(e) => return Err(e),
                };
                if total_len > frame_len {
                    return Err(sbe_rt::DecodeError::BufferTooShort {
                        field: "DynamicRow",
                        needed: total_len,
                        available: frame_len,
                    });
                }
                Ok(DecodedFrame {
                    message: Self::DynamicRow(decoder),
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
    #[inline]
    pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
        match self {
            Self::DynamicSchema(d) => d.encoded_length_with_header(),
            Self::DynamicRow(d) => d.encoded_length_with_header(),
            Self::Unknown { payload, .. } => Ok(payload.len()),
        }
    }
    #[inline]
    pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        match self {
            Self::DynamicSchema(d) => d.as_bytes(),
            Self::DynamicRow(d) => d.as_bytes(),
            Self::Unknown { payload, .. } => Ok(payload),
        }
    }
    #[inline]
    pub fn encode(&self, buf: &mut [u8]) -> Result<usize, sbe_rt::EncodeError> {
        match self {
            Self::DynamicSchema(d) => {
                let len = d.encoded_length_with_header()?;
                buf[..len].copy_from_slice(d.as_bytes()?);
                Ok(len)
            }
            Self::DynamicRow(d) => {
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
    fn visit_dynamic_schema(
        &mut self,
        decoder: &DynamicSchemaDecoder<'_>,
    ) -> Self::Output;
    fn visit_dynamic_row(&mut self, decoder: &DynamicRowDecoder<'_>) -> Self::Output;
}
impl<'a> AnyMessage<'a> {
    pub fn visit<V: MessageVisitor>(&self, visitor: &mut V) -> V::Output {
        match self {
            Self::DynamicSchema(d) => visitor.visit_dynamic_schema(d),
            Self::DynamicRow(d) => visitor.visit_dynamic_row(d),
            Self::Unknown { .. } => unimplemented!(),
        }
    }
}
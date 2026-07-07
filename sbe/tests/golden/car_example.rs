//! Generated from SBE schema package `baseline` id 1 version 0.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(clippy::identity_op)]
#![allow(clippy::eq_op)]
#![allow(clippy::needless_borrow)]
#![allow(clippy::manual_range_contains)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(dead_code)]
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
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum BooleanType {
    F = 0,
    T = 1,
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
    #[inline(always)]
    fn from(val: BooleanType) -> Self {
        val as u8
    }
}
impl From<u8> for BooleanType {
    #[inline(always)]
    fn from(val: u8) -> Self {
        Self::from_raw(val)
    }
}
impl From<bool> for BooleanType {
    #[inline(always)]
    fn from(val: bool) -> Self {
        if val { Self::T } else { Self::F }
    }
}
impl From<BooleanType> for bool {
    #[inline(always)]
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
    #[inline(always)]
    fn from(val: Model) -> Self {
        val as u8
    }
}
impl From<u8> for Model {
    #[inline(always)]
    fn from(val: u8) -> Self {
        Self::from_raw(val)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
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
    #[inline(always)]
    fn from(val: u8) -> Self {
        Self(val)
    }
}
impl From<OptionalExtras> for u8 {
    #[inline(always)]
    fn from(val: OptionalExtras) -> Self {
        val.0
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
pub struct GroupSizeEncoding(pub [u8; 4]);
impl GroupSizeEncoding {
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
pub struct VarStringEncoding(pub [u8; 4]);
impl VarStringEncoding {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarAsciiEncoding(pub [u8; 4]);
impl VarAsciiEncoding {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct VarDataEncoding(pub [u8; 4]);
impl VarDataEncoding {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Booster(pub [u8; 1]);
impl Booster {
    #[inline]
    pub const fn horse_power(&self) -> u8 {
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u8::from_le_bytes(bytes)
    }
    pub const fn new(horse_power: u8) -> Self {
        let mut bytes = [0u8; 1];
        let val_bytes = horse_power.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            bytes[0 + j] = val_bytes[j];
            j += 1;
        }
        Self(bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct Engine(pub [u8; 6]);
impl Engine {
    #[inline]
    pub const fn capacity(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn num_cylinders(&self) -> u8 {
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.0[2 + j];
            j += 1;
        }
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn max_rpm(&self) -> u16 {
        9000
    }
    #[inline]
    pub const fn manufacturer_code(&self) -> [u8; 3] {
        let mut res = [0 as u8; 3];
        let mut idx = 0;
        while idx < 3 {
            let offset = 3 + idx * 1;
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
    #[inline]
    pub const fn fuel(&self) -> &'static str {
        "Petrol"
    }
    pub const fn new(
        capacity: u16,
        num_cylinders: u8,
        manufacturer_code: [u8; 3],
    ) -> Self {
        let mut bytes = [0u8; 6];
        let val_bytes = capacity.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            bytes[0 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = num_cylinders.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            bytes[2 + j] = val_bytes[j];
            j += 1;
        }
        let mut idx = 0;
        while idx < 3 {
            let val_bytes = manufacturer_code[idx].to_le_bytes();
            let mut j = 0;
            while j < 1 {
                bytes[3 + idx * 1 + j] = val_bytes[j];
                j += 1;
            }
            idx += 1;
        }
        Self(bytes)
    }
}
/// Description of a basic Car
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
        u64::from_le_bytes(self.buf[offset..][..8].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn serial_number_unchecked(&self) -> u64 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 8];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 8)
            });
        u64::from_le_bytes(bytes)
    }
    pub const SERIAL_NUMBER_NULL: u64 = 18446744073709551615_u64;
    pub const SERIAL_NUMBER_MIN: u64 = 0_u64;
    pub const SERIAL_NUMBER_MAX: u64 = 18446744073709551614_u64;
    #[inline]
    pub fn model_year(&self) -> u16 {
        let offset = self.pos + 8;
        u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn model_year_unchecked(&self) -> u16 {
        let offset = self.pos + 8;
        let mut bytes = [0u8; 2];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 2)
            });
        u16::from_le_bytes(bytes)
    }
    pub const MODEL_YEAR_NULL: u16 = 65535_u16;
    pub const MODEL_YEAR_MIN: u16 = 0_u16;
    pub const MODEL_YEAR_MAX: u16 = 65534_u16;
    #[inline]
    pub fn available(&self) -> BooleanType {
        let offset = self.pos + 10;
        BooleanType::from_raw(
            u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap()),
        )
    }
    #[inline]
    pub const unsafe fn available_unchecked(&self) -> BooleanType {
        let offset = self.pos + 10;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        BooleanType::from_raw(u8::from_le_bytes(bytes))
    }
    pub const AVAILABLE_NULL: BooleanType = BooleanType::NullVal;
    #[inline]
    pub fn code(&self) -> Model {
        let offset = self.pos + 11;
        Model::from_raw(u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap()))
    }
    #[inline]
    pub const unsafe fn code_unchecked(&self) -> Model {
        let offset = self.pos + 11;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        Model::from_raw(u8::from_le_bytes(bytes))
    }
    pub const CODE_NULL: Model = Model::NullVal;
    #[inline]
    pub const fn some_numbers(&self) -> Result<[u32; 4], sbe_rt::DecodeError> {
        if self.acting_version < 0 || 28 > self.acting_block_length {
            return Ok([0 as u32; 4]);
        }
        let offset = self.pos + 12;
        let size = 16;
        if offset + size > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "someNumbers",
                needed: size,
                available: self.buf.len() - offset,
            });
        }
        let mut res = [0 as u32; 4];
        let mut idx = 0;
        while idx < 4 {
            let offset = self.pos + 12 + idx * 4;
            let mut bytes = [0u8; 4];
            let mut j = 0;
            while j < 4 {
                bytes[j] = self.buf[offset + j];
                j += 1;
            }
            res[idx] = u32::from_le_bytes(bytes);
            idx += 1;
        }
        Ok(res)
    }
    #[inline]
    pub const unsafe fn some_numbers_unchecked(&self) -> [u32; 4] {
        let offset = self.pos + 12;
        let mut res = [0 as u32; 4];
        let mut idx = 0;
        while idx < 4 {
            let offset = self.pos + 12 + idx * 4;
            let mut bytes = [0u8; 4];
            bytes
                .copy_from_slice(unsafe {
                    core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 4)
                });
            res[idx] = u32::from_le_bytes(bytes);
            idx += 1;
        }
        res
    }
    #[inline]
    pub const fn raw_some_numbers(&self) -> [u32; 4] {
        #[allow(unused_unsafe)] unsafe { self.some_numbers_unchecked() }
    }
    pub const SOME_NUMBERS_NULL: u32 = 4294967295_u32;
    pub const SOME_NUMBERS_MIN: u32 = 0_u32;
    pub const SOME_NUMBERS_MAX: u32 = 4294967294_u32;
    #[inline]
    pub const fn vehicle_code(&self) -> Result<[u8; 6], sbe_rt::DecodeError> {
        if self.acting_version < 0 || 34 > self.acting_block_length {
            return Ok([0 as u8; 6]);
        }
        let offset = self.pos + 28;
        let size = 6;
        if offset + size > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "vehicleCode",
                needed: size,
                available: self.buf.len() - offset,
            });
        }
        let mut res = [0 as u8; 6];
        let mut idx = 0;
        while idx < 6 {
            let offset = self.pos + 28 + idx * 1;
            let mut bytes = [0u8; 1];
            let mut j = 0;
            while j < 1 {
                bytes[j] = self.buf[offset + j];
                j += 1;
            }
            res[idx] = u8::from_le_bytes(bytes);
            idx += 1;
        }
        Ok(res)
    }
    #[inline]
    pub const unsafe fn vehicle_code_unchecked(&self) -> [u8; 6] {
        let offset = self.pos + 28;
        let mut res = [0 as u8; 6];
        let mut idx = 0;
        while idx < 6 {
            let offset = self.pos + 28 + idx * 1;
            let mut bytes = [0u8; 1];
            bytes
                .copy_from_slice(unsafe {
                    core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
                });
            res[idx] = u8::from_le_bytes(bytes);
            idx += 1;
        }
        res
    }
    #[inline]
    pub const fn raw_vehicle_code(&self) -> [u8; 6] {
        #[allow(unused_unsafe)] unsafe { self.vehicle_code_unchecked() }
    }
    pub const VEHICLE_CODE_NULL: u8 = 0_u8;
    pub const VEHICLE_CODE_MIN: u8 = 32_u8;
    pub const VEHICLE_CODE_MAX: u8 = 126_u8;
    #[inline]
    pub fn extras(&self) -> OptionalExtras {
        let offset = self.pos + 34;
        OptionalExtras(u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap()))
    }
    #[inline]
    pub const unsafe fn extras_unchecked(&self) -> OptionalExtras {
        let offset = self.pos + 34;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        OptionalExtras(u8::from_le_bytes(bytes))
    }
    #[inline]
    pub const fn discounted_model(&self) -> Model {
        Model::C
    }
    #[inline]
    pub const unsafe fn discounted_model_unchecked(&self) -> Model {
        let offset = self.pos + 35;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        Model::from_raw(u8::from_le_bytes(bytes))
    }
    pub const DISCOUNTED_MODEL_NULL: Model = Model::NullVal;
    #[inline]
    pub fn engine(&self) -> Engine {
        let offset = self.pos + 35;
        Engine(self.buf[offset..][..6].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn engine_unchecked(&self) -> Engine {
        let offset = self.pos + 35;
        let mut bytes = [0u8; 6];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 6)
            });
        Engine(bytes)
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
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
    pub fn fuel_figures(&self) -> Result<FuelFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        FuelFiguresDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn performance_figures(
        &self,
    ) -> Result<PerformanceFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        PerformanceFiguresDecoder::wrap(self.buf, offset, self.acting_version)
    }
    #[inline]
    pub fn manufacturer(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        let bytes: [u8; 4] = self.buf[offset..offset + 4].try_into().unwrap();
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "manufacturer",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    pub fn manufacturer_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.manufacturer()?;
        core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))
    }
    #[inline]
    pub unsafe fn manufacturer_as_str_unchecked(&self) -> &'a str {
        let data = self.manufacturer().unwrap_or(&[]);
        unsafe { core::str::from_utf8_unchecked(data) }
    }
    #[inline]
    pub fn manufacturer_as_string(&self) -> Result<String, sbe_rt::DecodeError> {
        Ok(self.manufacturer_as_str()?.to_string())
    }
    #[inline]
    pub fn manufacturer_as_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.manufacturer()
    }
    #[inline]
    pub fn model(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_3()?;
        let bytes: [u8; 4] = self.buf[offset..offset + 4].try_into().unwrap();
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "model",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    pub fn model_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.model()?;
        core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))
    }
    #[inline]
    pub unsafe fn model_as_str_unchecked(&self) -> &'a str {
        let data = self.model().unwrap_or(&[]);
        unsafe { core::str::from_utf8_unchecked(data) }
    }
    #[inline]
    pub fn model_as_string(&self) -> Result<String, sbe_rt::DecodeError> {
        Ok(self.model_as_str()?.to_string())
    }
    #[inline]
    pub fn model_as_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.model()
    }
    #[inline]
    pub fn activation_code(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_4()?;
        let bytes: [u8; 4] = self.buf[offset..offset + 4].try_into().unwrap();
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        if len > 1073741824 {
            return Err(sbe_rt::DecodeError::InvalidVarDataLength {
                field: "activation_code",
                length: len as u32,
                max_length: 1073741824,
            });
        }
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    #[inline]
    pub fn activation_code_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.activation_code()?;
        core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))
    }
    #[inline]
    pub unsafe fn activation_code_as_str_unchecked(&self) -> &'a str {
        let data = self.activation_code().unwrap_or(&[]);
        unsafe { core::str::from_utf8_unchecked(data) }
    }
    #[inline]
    pub fn activation_code_as_string(&self) -> Result<String, sbe_rt::DecodeError> {
        Ok(self.activation_code_as_str()?.to_string())
    }
    #[inline]
    pub fn activation_code_as_slice(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        self.activation_code()
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
                    field: "fuel_figures",
                    offset,
                });
            }
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            let bytes: [u8; 4] = buf[offset..offset + 4].try_into().unwrap();
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
            write!(f, "serial_number: {}", v)?;
        }
        {
            let v = self.model_year();
            write!(f, ", model_year: {}", v)?;
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
            write!(f, ", fuel_figures: {} entries", g.len())?;
        }
        if let Ok(g) = self.performance_figures() {
            write!(f, ", performance_figures: {} entries", g.len())?;
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
}
impl<'a> FuelFiguresDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
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
                    field: "fuelFigures",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSizeEncoding(bytes);
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
        if n > self.count {
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
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "fuelFigures",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(FuelFiguresEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
}
impl<'a> Iterator for FuelFiguresDecoder<'a> {
    type Item = FuelFiguresEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = FuelFiguresEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        let size = match entry.encoded_length() {
            Ok(s) => s,
            Err(_) => {
                self.count = 0;
                return Some(entry);
            }
        };
        self.pos += size;
        self.count -= 1;
        Some(entry)
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
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
}
pub const SPEED_NULL: u16 = 65535_u16;
pub const SPEED_MIN: u16 = 0_u16;
pub const SPEED_MAX: u16 = 65534_u16;
pub const MPG_NULL: f32 = f32::from_bits(2139095041u32);
pub const MPG_MIN: f32 = f32::from_bits(4286578687u32);
pub const MPG_MAX: f32 = f32::from_bits(2139095039u32);
impl<'a> FuelFiguresEntryDecoder<'a> {
    #[inline]
    pub fn speed(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn speed_unchecked(&self) -> u16 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 2];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 2)
            });
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_speed(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.speed_unchecked() }
    }
    #[inline]
    pub fn mpg(&self) -> f32 {
        let offset = self.pos + 2;
        f32::from_le_bytes(self.buf[offset..][..4].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn mpg_unchecked(&self) -> f32 {
        let offset = self.pos + 2;
        let mut bytes = [0u8; 4];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 4)
            });
        f32::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_mpg(&self) -> f32 {
        #[allow(unused_unsafe)] unsafe { self.mpg_unchecked() }
    }
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
}
impl<'a> FuelFiguresEntryDecoder<'a> {
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
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
}
impl<'a> FuelFiguresEntryDecoder<'a> {
    #[inline]
    pub fn usage_description(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        let bytes: [u8; 4] = self.buf[offset..offset + 4].try_into().unwrap();
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
}
impl<'a> FuelFiguresEntryDecoder<'a> {
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
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_1()
    }
}
pub struct PerformanceFiguresDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> PerformanceFiguresDecoder<'a> {
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
                    field: "performanceFigures",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSizeEncoding(bytes);
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
        if n > self.count {
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
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "performanceFigures",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(PerformanceFiguresEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
}
impl<'a> Iterator for PerformanceFiguresDecoder<'a> {
    type Item = PerformanceFiguresEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = PerformanceFiguresEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        let size = match entry.encoded_length() {
            Ok(s) => s,
            Err(_) => {
                self.count = 0;
                return Some(entry);
            }
        };
        self.pos += size;
        self.count -= 1;
        Some(entry)
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
}
impl<'a> PerformanceFiguresEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
}
pub const OCTANE_RATING_NULL: u8 = 255_u8;
pub const OCTANE_RATING_MIN: u8 = 90_u8;
pub const OCTANE_RATING_MAX: u8 = 110_u8;
impl<'a> PerformanceFiguresEntryDecoder<'a> {
    #[inline]
    pub fn octane_rating(&self) -> u8 {
        let offset = self.pos + 0;
        u8::from_le_bytes(self.buf[offset..][..1].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn octane_rating_unchecked(&self) -> u8 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 1];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 1)
            });
        u8::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_octane_rating(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.octane_rating_unchecked() }
    }
}
impl<'a> PerformanceFiguresEntryDecoder<'a> {
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
}
impl<'a> PerformanceFiguresEntryDecoder<'a> {
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
        let bytes: [u8; 4] = self.buf[start..start + 4].try_into().unwrap();
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        let block_len = header.block_length() as usize;
        let mut pos = start + 4;
        let mut idx = 0;
        while idx < count {
            pos = AccelerationEntryDecoder::skip(
                self.buf,
                pos,
                block_len,
                self.acting_version,
            )?;
            idx += 1;
        }
        Ok(pos)
    }
}
impl<'a> PerformanceFiguresEntryDecoder<'a> {
    #[inline]
    pub fn acceleration(&self) -> Result<AccelerationDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        AccelerationDecoder::wrap(self.buf, offset, self.acting_version)
    }
}
impl<'a> PerformanceFiguresEntryDecoder<'a> {
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
        let entry = Self::wrap(buf, pos, acting_version);
        entry.tail_offset_1()
    }
}
pub struct AccelerationDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    start: usize,
    total: usize,
    acting_version: u16,
}
impl<'a> AccelerationDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
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
                    field: "acceleration",
                    needed: 4,
                    available: buf.len() - pos,
                }
            })?
            .try_into()
            .unwrap();
        let header = GroupSizeEncoding(bytes);
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
        self.count
    }
    #[inline]
    pub fn rewind(&mut self) -> &mut Self {
        self.pos = self.start;
        self.count = self.total;
        self
    }
}
impl<'a> AccelerationDecoder<'a> {
    #[inline]
    pub fn skip_n(&mut self, n: usize) -> Result<(), sbe_rt::DecodeError> {
        if n > self.count {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: n * Self::ENTRY_BLOCK_LENGTH,
                available: self.count * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        self.pos += n * Self::ENTRY_BLOCK_LENGTH;
        self.count -= n;
        Ok(())
    }
}
impl<'a> AccelerationDecoder<'a> {
    #[inline]
    pub fn nth(
        &self,
        idx: usize,
    ) -> Result<AccelerationEntryDecoder<'a>, sbe_rt::DecodeError> {
        if idx >= self.total {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: (idx + 1) * Self::ENTRY_BLOCK_LENGTH,
                available: self.total * Self::ENTRY_BLOCK_LENGTH,
            });
        }
        let offset = self.start + idx * Self::ENTRY_BLOCK_LENGTH;
        if offset + Self::ENTRY_BLOCK_LENGTH > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: Self::ENTRY_BLOCK_LENGTH,
                available: self.buf.len() - offset,
            });
        }
        Ok(AccelerationEntryDecoder::wrap(self.buf, offset, self.acting_version))
    }
}
impl<'a> AccelerationDecoder<'a> {
    #[inline]
    pub fn as_chunks(&self) -> Result<&'a [[u8; 6]], sbe_rt::DecodeError> {
        let len = self.count * 6;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                field: "acceleration",
                needed: len,
                available: self.buf.len() - self.pos,
            });
        }
        let bytes = &self.buf[self.pos..self.pos + len];
        let (chunks, _) = bytes.as_chunks::<6>();
        Ok(chunks)
    }
}
impl<'a> Iterator for AccelerationDecoder<'a> {
    type Item = AccelerationEntryDecoder<'a>;
    fn next(&mut self) -> Option<Self::Item> {
        if self.count == 0 {
            return None;
        }
        let entry = AccelerationEntryDecoder::wrap(
            self.buf,
            self.pos,
            self.acting_version,
        );
        self.pos += Self::ENTRY_BLOCK_LENGTH;
        self.count -= 1;
        Some(entry)
    }
}
impl<'a> ExactSizeIterator for AccelerationDecoder<'a> {
    fn len(&self) -> usize {
        self.count
    }
}
pub struct AccelerationEntryDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    acting_version: u16,
}
impl<'a> AccelerationEntryDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    #[inline]
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
}
pub const MPH_NULL: u16 = 65535_u16;
pub const MPH_MIN: u16 = 0_u16;
pub const MPH_MAX: u16 = 65534_u16;
pub const SECONDS_NULL: f32 = f32::from_bits(2139095041u32);
pub const SECONDS_MIN: f32 = f32::from_bits(4286578687u32);
pub const SECONDS_MAX: f32 = f32::from_bits(2139095039u32);
impl<'a> AccelerationEntryDecoder<'a> {
    #[inline]
    pub fn mph(&self) -> u16 {
        let offset = self.pos + 0;
        u16::from_le_bytes(self.buf[offset..][..2].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn mph_unchecked(&self) -> u16 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 2];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 2)
            });
        u16::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_mph(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.mph_unchecked() }
    }
    #[inline]
    pub fn seconds(&self) -> f32 {
        let offset = self.pos + 2;
        f32::from_le_bytes(self.buf[offset..][..4].try_into().unwrap())
    }
    #[inline]
    pub const unsafe fn seconds_unchecked(&self) -> f32 {
        let offset = self.pos + 2;
        let mut bytes = [0u8; 4];
        bytes
            .copy_from_slice(unsafe {
                core::slice::from_raw_parts(self.buf.as_ptr().add(offset), 4)
            });
        f32::from_le_bytes(bytes)
    }
    #[inline]
    pub const fn raw_seconds(&self) -> f32 {
        #[allow(unused_unsafe)] unsafe { self.seconds_unchecked() }
    }
}
impl<'a> AccelerationEntryDecoder<'a> {
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
}
impl<'a> AccelerationEntryDecoder<'a> {
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
pub mod car_encoder_state {
    pub struct NeedsFuelFigures;
    pub struct NeedsPerformanceFigures;
    pub struct NeedsManufacturer;
    pub struct NeedsModel;
    pub struct NeedsActivationCode;
    pub struct Complete;
}
#[must_use = "encoder must be consumed to write the message"]
pub struct CarEncoder<'a, State = car_encoder_state::NeedsFuelFigures> {
    buf: &'a mut [u8],
    message_start: usize,
    pos: usize,
    _phantom: core::marker::PhantomData<State>,
}
impl<'a, State> CarEncoder<'a, State> {
    pub const SCHEMA_ID: u16 = 1;
    pub const SCHEMA_VERSION: u16 = 0;
    pub const TEMPLATE_ID: u16 = 1;
    pub const BLOCK_LENGTH: usize = 41;
    const _BLOCK_LEN: () = assert!(Self::BLOCK_LENGTH == 41);
    /// MAX_ENCODED_LENGTH exceeds the 64KB stack limit; use `Vec::with_capacity(Self::MAX_ENCODED_LENGTH)` for heap allocation
    pub const MAX_ENCODED_LENGTH: usize = 65536;
    const _MAX_ENCODED_LEN: () = assert!(Self::MAX_ENCODED_LENGTH >= Self::BLOCK_LENGTH);
    pub const HEADER_TEMPLATE: [u8; 8] = [41, 0, 1, 0, 1, 0, 0, 0];
    const _HEADER_TEMPLATE_LEN: () = assert!(Self::HEADER_TEMPLATE.len() == 8);
    #[inline]
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            message_start: pos,
            pos: pos + 8 + 41,
            _phantom: core::marker::PhantomData,
        }
    }
    #[inline]
    pub fn wrap_and_apply_header(
        buf: &'a mut [u8],
        pos: usize,
    ) -> Result<Self, sbe_rt::EncodeError> {
        let needed = 8 + 41;
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
    pub fn serial_number(&mut self, val: u64) -> &mut Self {
        let offset = self.message_start + 8 + 0;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 8].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn model_year(&mut self, val: u16) -> &mut Self {
        let offset = self.message_start + 8 + 8;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 2].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn available(&mut self, val: BooleanType) -> &mut Self {
        let offset = self.message_start + 8 + 10;
        let val_bytes = (val as u8).to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn available_bool(&mut self, val: bool) -> &mut Self {
        let offset = self.message_start + 8 + 10;
        let enum_val: BooleanType = val.into();
        let val_bytes = (enum_val as u8).to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn code(&mut self, val: Model) -> &mut Self {
        let offset = self.message_start + 8 + 11;
        let val_bytes = (val as u8).to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn some_numbers(&mut self, val: [u32; 4]) -> &mut Self {
        let offset = self.message_start + 8 + 12;
        let mut idx = 0;
        while idx < 4 {
            let val_bytes = val[idx].to_le_bytes();
            self.buf[offset + idx * 4..offset + idx * 4 + 4].copy_from_slice(&val_bytes);
            idx += 1;
        }
        self
    }
    #[must_use]
    pub fn vehicle_code(&mut self, val: [u8; 6]) -> &mut Self {
        let offset = self.message_start + 8 + 28;
        let mut idx = 0;
        while idx < 6 {
            let val_bytes = val[idx].to_le_bytes();
            self.buf[offset + idx * 1..offset + idx * 1 + 1].copy_from_slice(&val_bytes);
            idx += 1;
        }
        self
    }
    #[must_use]
    pub fn extras(&mut self, val: OptionalExtras) -> &mut Self {
        let offset = self.message_start + 8 + 34;
        let val_bytes = val.0.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn engine(&mut self, val: Engine) -> &mut Self {
        let offset = self.message_start + 8 + 35;
        self.buf[offset..offset + 6].copy_from_slice(&val.0);
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
impl<'a> CarEncoder<'a, car_encoder_state::NeedsFuelFigures> {
    #[must_use]
    pub fn fuel_figures<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        CarEncoder<'a, car_encoder_state::NeedsPerformanceFigures>,
        sbe_rt::EncodeError,
    >
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
        Ok(CarEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::NeedsPerformanceFigures> {
    #[must_use]
    pub fn performance_figures<F>(
        mut self,
        count: u16,
        f: F,
    ) -> Result<
        CarEncoder<'a, car_encoder_state::NeedsManufacturer>,
        sbe_rt::EncodeError,
    >
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
        Ok(CarEncoder {
            buf: group.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::NeedsManufacturer> {
    #[must_use]
    pub fn manufacturer(
        mut self,
        data: &[u8],
    ) -> Result<CarEncoder<'a, car_encoder_state::NeedsModel>, sbe_rt::EncodeError> {
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
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
    #[must_use]
    pub fn manufacturer_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarEncoder<'a, car_encoder_state::NeedsModel>, sbe_rt::EncodeError> {
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
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::NeedsModel> {
    #[must_use]
    pub fn model(
        mut self,
        data: &[u8],
    ) -> Result<
        CarEncoder<'a, car_encoder_state::NeedsActivationCode>,
        sbe_rt::EncodeError,
    > {
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
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
    #[must_use]
    pub fn model_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<
        CarEncoder<'a, car_encoder_state::NeedsActivationCode>,
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
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::NeedsActivationCode> {
    #[must_use]
    pub fn activation_code(
        mut self,
        data: &[u8],
    ) -> Result<CarEncoder<'a, car_encoder_state::Complete>, sbe_rt::EncodeError> {
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
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
    #[must_use]
    pub fn activation_code_unchecked(
        mut self,
        data: &[u8],
    ) -> Result<CarEncoder<'a, car_encoder_state::Complete>, sbe_rt::EncodeError> {
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
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::Complete> {
    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[self.message_start..self.pos]
    }
}
impl<'a> AsRef<[u8]> for CarEncoder<'a, car_encoder_state::Complete> {
    fn as_ref(&self) -> &[u8] {
        self.as_bytes()
    }
}
impl<'a, State> sbe_rt::private::Sealed for CarEncoder<'a, State> {}
impl<'a, State> sbe_rt::SbeMessage for CarEncoder<'a, State> {
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
        let mut entry = FuelFiguresEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 2].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn mpg(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 4].copy_from_slice(&val_bytes);
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
        let mut entry = PerformanceFiguresEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 1].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn acceleration<F>(
        &mut self,
        count: u16,
        f: F,
    ) -> Result<&mut Self, sbe_rt::EncodeError>
    where
        F: FnOnce(&mut AccelerationEncoder<'a>),
    {
        if self.pos + 4 > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: 4,
                available: self.buf.len() - self.pos,
            });
        }
        self.buf[self.pos..self.pos + 4]
            .copy_from_slice(&AccelerationEncoder::GROUP_DIM_TEMPLATE);
        self.buf[self.pos + 2..self.pos + 2 + 2].copy_from_slice(&count.to_le_bytes());
        let mut group = AccelerationEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        self.pos = group.pos;
        Ok(self)
    }
}
#[must_use = "group encoder must call add() to write entries"]
pub struct AccelerationEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> AccelerationEncoder<'a> {
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
        F: FnOnce(&mut AccelerationEntryEncoder<'b>),
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
        let mut entry = AccelerationEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
#[must_use = "entry encoder fields must be set before the next entry"]
pub struct AccelerationEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> AccelerationEntryEncoder<'a> {
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
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 2].copy_from_slice(&val_bytes);
        self
    }
    #[must_use]
    pub fn seconds(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        let val_bytes = val.to_le_bytes();
        self.buf[offset..offset + 4].copy_from_slice(&val_bytes);
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
                let total_len = match decoder.encoded_length_with_header() {
                    Ok(len) => len,
                    Err(e) => return Err(e),
                };
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

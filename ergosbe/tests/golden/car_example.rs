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
        BufferTooShort { needed: usize, available: usize },
        WrongSchema { expected: u16, actual: u16 },
        UnknownTemplateLength { template_id: u16 },
        InvalidVarDataLength { field: &'static str, length: u32 },
        Utf8(core::str::Utf8Error),
    }
    impl core::fmt::Display for DecodeError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::BufferTooShort { needed, available } => {
                    write!(
                        f, "buffer too short: needed {}, available {}", needed, available
                    )
                }
                Self::WrongSchema { expected, actual } => {
                    write!(
                        f, "wrong schema id: expected {}, actual {}", expected, actual
                    )
                }
                Self::UnknownTemplateLength { template_id } => {
                    write!(f, "unknown template length for template id {}", template_id)
                }
                Self::InvalidVarDataLength { field, length } => {
                    write!(f, "invalid var data length for field {}: {}", field, length)
                }
                Self::Utf8(err) => write!(f, "UTF-8 decode error: {}", err),
            }
        }
    }
    impl core::error::Error for DecodeError {}
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum EncodeError {
        BufferTooShort { needed: usize, available: usize },
    }
    impl core::fmt::Display for EncodeError {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            match self {
                Self::BufferTooShort { needed, available } => {
                    write!(
                        f, "buffer too short: needed {}, available {}", needed, available
                    )
                }
            }
        }
    }
    impl core::error::Error for EncodeError {}
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct BooleanType(pub u8);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum BooleanTypeKind {
    F = 0,
    T = 1,
}
impl BooleanType {
    pub const F: Self = Self(0);
    pub const T: Self = Self(1);
    pub const fn kind(self) -> Option<BooleanTypeKind> {
        match self.0 {
            0 => Some(BooleanTypeKind::F),
            1 => Some(BooleanTypeKind::T),
            _ => None,
        }
    }
    pub const fn into_kind(self) -> Option<BooleanTypeKind> {
        self.kind()
    }
    pub const fn raw(self) -> u8 {
        self.0
    }
}
impl From<u8> for BooleanType {
    #[inline(always)]
    fn from(val: u8) -> Self {
        Self(val)
    }
}
impl From<BooleanType> for u8 {
    #[inline(always)]
    fn from(val: BooleanType) -> Self {
        val.0
    }
}
impl TryFrom<BooleanType> for BooleanTypeKind {
    type Error = ();
    #[inline]
    fn try_from(val: BooleanType) -> Result<Self, Self::Error> {
        val.kind().ok_or(())
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Model(pub u8);
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(u8)]
pub enum ModelKind {
    A = b'A',
    B = b'B',
    C = b'C',
}
impl Model {
    pub const A: Self = Self(b'A');
    pub const B: Self = Self(b'B');
    pub const C: Self = Self(b'C');
    pub const fn kind(self) -> Option<ModelKind> {
        match self.0 {
            b'A' => Some(ModelKind::A),
            b'B' => Some(ModelKind::B),
            b'C' => Some(ModelKind::C),
            _ => None,
        }
    }
    pub const fn into_kind(self) -> Option<ModelKind> {
        self.kind()
    }
    pub const fn raw(self) -> u8 {
        self.0
    }
}
impl From<u8> for Model {
    #[inline(always)]
    fn from(val: u8) -> Self {
        Self(val)
    }
}
impl From<Model> for u8 {
    #[inline(always)]
    fn from(val: Model) -> Self {
        val.0
    }
}
impl TryFrom<Model> for ModelKind {
    type Error = ();
    #[inline]
    fn try_from(val: Model) -> Result<Self, Self::Error> {
        val.kind().ok_or(())
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct MessageHeader(pub [u8; 8]);
impl MessageHeader {
    pub const fn block_length(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    pub const fn template_id(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[2 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    pub const fn schema_id(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[4 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct GroupSizeEncoding(pub [u8; 4]);
impl GroupSizeEncoding {
    pub const fn block_length(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VarStringEncoding(pub [u8; 5]);
impl VarStringEncoding {
    pub const fn length(&self) -> u32 {
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u32::from_le_bytes(bytes)
    }
    pub const fn var_data(&self) -> u8 {
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.0[4 + j];
            j += 1;
        }
        u8::from_le_bytes(bytes)
    }
    pub const fn new(length: u32, var_data: u8) -> Self {
        let mut bytes = [0u8; 5];
        let val_bytes = length.to_le_bytes();
        let mut j = 0;
        while j < 4 {
            bytes[0 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = var_data.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            bytes[4 + j] = val_bytes[j];
            j += 1;
        }
        Self(bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VarAsciiEncoding(pub [u8; 5]);
impl VarAsciiEncoding {
    pub const fn length(&self) -> u32 {
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u32::from_le_bytes(bytes)
    }
    pub const fn var_data(&self) -> u8 {
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.0[4 + j];
            j += 1;
        }
        u8::from_le_bytes(bytes)
    }
    pub const fn new(length: u32, var_data: u8) -> Self {
        let mut bytes = [0u8; 5];
        let val_bytes = length.to_le_bytes();
        let mut j = 0;
        while j < 4 {
            bytes[0 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = var_data.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            bytes[4 + j] = val_bytes[j];
            j += 1;
        }
        Self(bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct VarDataEncoding(pub [u8; 5]);
impl VarDataEncoding {
    pub const fn length(&self) -> u32 {
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u32::from_le_bytes(bytes)
    }
    pub const fn var_data(&self) -> u8 {
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.0[4 + j];
            j += 1;
        }
        u8::from_le_bytes(bytes)
    }
    pub const fn new(length: u32, var_data: u8) -> Self {
        let mut bytes = [0u8; 5];
        let val_bytes = length.to_le_bytes();
        let mut j = 0;
        while j < 4 {
            bytes[0 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = var_data.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            bytes[4 + j] = val_bytes[j];
            j += 1;
        }
        Self(bytes)
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Booster(pub [u8; 1]);
impl Booster {
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
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[repr(transparent)]
pub struct Engine(pub [u8; 7]);
impl Engine {
    pub const fn capacity(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[0 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    pub const fn num_cylinders(&self) -> u8 {
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.0[2 + j];
            j += 1;
        }
        u8::from_le_bytes(bytes)
    }
    pub const fn max_rpm(&self) -> u16 {
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.0[3 + j];
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    pub const fn manufacturer_code(&self) -> u8 {
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.0[5 + j];
            j += 1;
        }
        u8::from_le_bytes(bytes)
    }
    pub const fn fuel(&self) -> u8 {
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.0[6 + j];
            j += 1;
        }
        u8::from_le_bytes(bytes)
    }
    pub const fn new(
        capacity: u16,
        num_cylinders: u8,
        max_rpm: u16,
        manufacturer_code: u8,
        fuel: u8,
    ) -> Self {
        let mut bytes = [0u8; 7];
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
        let val_bytes = max_rpm.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            bytes[3 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = manufacturer_code.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            bytes[5 + j] = val_bytes[j];
            j += 1;
        }
        let val_bytes = fuel.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            bytes[6 + j] = val_bytes[j];
            j += 1;
        }
        Self(bytes)
    }
}
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
    pub const BLOCK_LENGTH: usize = 43;
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
    pub const fn wrap_and_apply_header(
        buf: &'a [u8],
        pos: usize,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if pos + 8 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: pos + 8,
                available: buf.len(),
            });
        }
        let mut header_bytes = [0u8; 8];
        let mut j = 0;
        while j < 8 {
            header_bytes[j] = buf[pos + j];
            j += 1;
        }
        let header = MessageHeader(header_bytes);
        if header.schema_id() != Self::SCHEMA_ID {
            return Err(sbe_rt::DecodeError::WrongSchema {
                expected: Self::SCHEMA_ID,
                actual: header.schema_id(),
            });
        }
        Ok(Self::wrap(buf, pos + 8, header.block_length() as usize, header.version()))
    }
    pub const fn acting_version(&self) -> u16 {
        self.acting_version
    }
    pub const fn acting_block_length(&self) -> usize {
        self.acting_block_length
    }
    pub const fn serial_number(&self) -> Result<u64, sbe_rt::DecodeError> {
        let offset = self.pos + 0;
        if offset + 8 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 8,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 8];
        let mut j = 0;
        while j < 8 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(u64::from_le_bytes(bytes))
    }
    pub const unsafe fn serial_number_unchecked(&self) -> u64 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 8];
        let mut j = 0;
        while j < 8 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        u64::from_le_bytes(bytes)
    }
    pub const fn raw_serial_number(&self) -> u64 {
        #[allow(unused_unsafe)] unsafe { self.serial_number_unchecked() }
    }
    pub const fn model_year(&self) -> Result<u16, sbe_rt::DecodeError> {
        let offset = self.pos + 8;
        if offset + 2 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 2,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(u16::from_le_bytes(bytes))
    }
    pub const unsafe fn model_year_unchecked(&self) -> u16 {
        let offset = self.pos + 8;
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    pub const fn raw_model_year(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.model_year_unchecked() }
    }
    pub const fn available(&self) -> Result<BooleanType, sbe_rt::DecodeError> {
        if self.acting_version < 0 || 11 > self.acting_block_length {
            return Ok(BooleanType(0 as u8));
        }
        let offset = self.pos + 10;
        if offset + 1 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 1,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(BooleanType(u8::from_le_bytes(bytes)))
    }
    pub const unsafe fn available_unchecked(&self) -> BooleanType {
        let offset = self.pos + 10;
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        BooleanType(u8::from_le_bytes(bytes))
    }
    pub const fn code(&self) -> Result<Model, sbe_rt::DecodeError> {
        if self.acting_version < 0 || 12 > self.acting_block_length {
            return Ok(Model(0 as u8));
        }
        let offset = self.pos + 11;
        if offset + 1 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 1,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(Model(u8::from_le_bytes(bytes)))
    }
    pub const unsafe fn code_unchecked(&self) -> Model {
        let offset = self.pos + 11;
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        Model(u8::from_le_bytes(bytes))
    }
    pub const fn some_numbers(&self) -> Result<[u32; 4], sbe_rt::DecodeError> {
        if self.acting_version < 0 || 28 > self.acting_block_length {
            return Ok([0 as u32; 4]);
        }
        let offset = self.pos + 12;
        let size = 16;
        if offset + size > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + size,
                available: self.buf.len(),
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
    pub const unsafe fn some_numbers_unchecked(&self) -> [u32; 4] {
        let offset = self.pos + 12;
        let mut res = [0 as u32; 4];
        let mut idx = 0;
        while idx < 4 {
            let offset = self.pos + 12 + idx * 4;
            let mut bytes = [0u8; 4];
            let mut j = 0;
            while j < 4 {
                bytes[j] = *self.buf.as_ptr().add(offset + j);
                j += 1;
            }
            res[idx] = u32::from_le_bytes(bytes);
            idx += 1;
        }
        res
    }
    pub const fn raw_some_numbers(&self) -> [u32; 4] {
        #[allow(unused_unsafe)] unsafe { self.some_numbers_unchecked() }
    }
    pub const fn vehicle_code(&self) -> Result<[u8; 6], sbe_rt::DecodeError> {
        if self.acting_version < 0 || 34 > self.acting_block_length {
            return Ok([0 as u8; 6]);
        }
        let offset = self.pos + 28;
        let size = 6;
        if offset + size > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + size,
                available: self.buf.len(),
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
    pub const unsafe fn vehicle_code_unchecked(&self) -> [u8; 6] {
        let offset = self.pos + 28;
        let mut res = [0 as u8; 6];
        let mut idx = 0;
        while idx < 6 {
            let offset = self.pos + 28 + idx * 1;
            let mut bytes = [0u8; 1];
            let mut j = 0;
            while j < 1 {
                bytes[j] = *self.buf.as_ptr().add(offset + j);
                j += 1;
            }
            res[idx] = u8::from_le_bytes(bytes);
            idx += 1;
        }
        res
    }
    pub const fn raw_vehicle_code(&self) -> [u8; 6] {
        #[allow(unused_unsafe)] unsafe { self.vehicle_code_unchecked() }
    }
    pub const fn extras(&self) -> Result<OptionalExtras, sbe_rt::DecodeError> {
        if self.acting_version < 0 || 35 > self.acting_block_length {
            return Ok(OptionalExtras(0 as u8));
        }
        let offset = self.pos + 34;
        if offset + 1 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 1,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(OptionalExtras(u8::from_le_bytes(bytes)))
    }
    pub const unsafe fn extras_unchecked(&self) -> OptionalExtras {
        let offset = self.pos + 34;
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        OptionalExtras(u8::from_le_bytes(bytes))
    }
    pub const fn discounted_model(&self) -> Result<Model, sbe_rt::DecodeError> {
        if self.acting_version < 0 || 36 > self.acting_block_length {
            return Ok(Model(0 as u8));
        }
        let offset = self.pos + 35;
        if offset + 1 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 1,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(Model(u8::from_le_bytes(bytes)))
    }
    pub const unsafe fn discounted_model_unchecked(&self) -> Model {
        let offset = self.pos + 35;
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        Model(u8::from_le_bytes(bytes))
    }
    pub const fn engine(&self) -> Result<Engine, sbe_rt::DecodeError> {
        if self.acting_version < 0 || 43 > self.acting_block_length {
            return Ok(Engine([0u8; 7]));
        }
        let offset = self.pos + 36;
        if offset + 7 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 7,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 7];
        let mut j = 0;
        while j < 7 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(Engine(bytes))
    }
    pub const unsafe fn engine_unchecked(&self) -> Engine {
        let offset = self.pos + 36;
        let mut bytes = [0u8; 7];
        let mut j = 0;
        while j < 7 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
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
                needed: start + 4,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[start + j];
            j += 1;
        }
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
                needed: start + 4,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[start + j];
            j += 1;
        }
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
                needed: start + 4,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[start + j];
            j += 1;
        }
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: start + 4 + len,
                available: self.buf.len(),
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn tail_offset_4(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_3()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: start + 4,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[start + j];
            j += 1;
        }
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: start + 4 + len,
                available: self.buf.len(),
            });
        }
        Ok(start + 4 + len)
    }
    #[inline]
    fn tail_offset_5(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_4()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: start + 4,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[start + j];
            j += 1;
        }
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: start + 4 + len,
                available: self.buf.len(),
            });
        }
        Ok(start + 4 + len)
    }
    pub fn fuel_figures(&self) -> Result<FuelFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        FuelFiguresDecoder::wrap(self.buf, offset, self.acting_version)
    }
    pub fn performance_figures(
        &self,
    ) -> Result<PerformanceFiguresDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_1()?;
        PerformanceFiguresDecoder::wrap(self.buf, offset, self.acting_version)
    }
    pub fn manufacturer(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_2()?;
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    pub fn manufacturer_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.manufacturer()?;
        core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))
    }
    pub fn model(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_3()?;
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        let header = VarStringEncoding(bytes);
        let len = header.length() as usize;
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    pub fn model_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.model()?;
        core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))
    }
    pub fn activation_code(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_4()?;
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    pub fn activation_code_as_str(&self) -> Result<&'a str, sbe_rt::DecodeError> {
        let bytes = self.activation_code()?;
        core::str::from_utf8(bytes).map_err(|e| sbe_rt::DecodeError::Utf8(e))
    }
    pub fn encoded_length(&self) -> Result<usize, sbe_rt::DecodeError> {
        let end = self.tail_offset_5()?;
        Ok(end - self.pos)
    }
    pub fn encoded_length_with_header(&self) -> Result<usize, sbe_rt::DecodeError> {
        let len = self.encoded_length()?;
        Ok(len + 8)
    }
    pub fn as_bytes(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let len = self.encoded_length_with_header()?;
        let start = self.pos - 8;
        Ok(&self.buf[start..start + len])
    }
}
impl<'a> sbe_rt::private::Sealed for CarDecoder<'a> {}
impl<'a> sbe_rt::SbeMessage for CarDecoder<'a> {
    const TEMPLATE_ID: u16 = 1;
    const BLOCK_LENGTH: usize = 43;
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
pub struct FuelFiguresDecoder<'a> {
    buf: &'a [u8],
    pos: usize,
    count: usize,
    acting_version: u16,
}
impl<'a> FuelFiguresDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if pos + 4 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: pos + 4,
                available: buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = buf[pos + j];
            j += 1;
        }
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            acting_version,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
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
        let size = entry.encoded_length();
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
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    pub const fn speed(&self) -> Result<u16, sbe_rt::DecodeError> {
        let offset = self.pos + 0;
        if offset + 2 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 2,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(u16::from_le_bytes(bytes))
    }
    pub const unsafe fn speed_unchecked(&self) -> u16 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    pub const fn raw_speed(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.speed_unchecked() }
    }
    pub const fn mpg(&self) -> Result<f32, sbe_rt::DecodeError> {
        let offset = self.pos + 2;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 4,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(f32::from_le_bytes(bytes))
    }
    pub const unsafe fn mpg_unchecked(&self) -> f32 {
        let offset = self.pos + 2;
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        f32::from_le_bytes(bytes)
    }
    pub const fn raw_mpg(&self) -> f32 {
        #[allow(unused_unsafe)] unsafe { self.mpg_unchecked() }
    }
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: start + 4,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[start + j];
            j += 1;
        }
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        if start + 4 + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: start + 4 + len,
                available: self.buf.len(),
            });
        }
        Ok(start + 4 + len)
    }
    pub fn usage_description(&self) -> Result<&'a [u8], sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        let header = VarAsciiEncoding(bytes);
        let len = header.length() as usize;
        let data_offset = offset + 4;
        Ok(&self.buf[data_offset..data_offset + len])
    }
    pub fn encoded_length(&self) -> usize {
        self.tail_offset_1().unwrap_or(self.pos + Self::ENTRY_BLOCK_LENGTH) - self.pos
    }
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
    acting_version: u16,
}
impl<'a> PerformanceFiguresDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if pos + 4 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: pos + 4,
                available: buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = buf[pos + j];
            j += 1;
        }
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            acting_version,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
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
        let size = entry.encoded_length();
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
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    pub const fn octane_rating(&self) -> Result<u8, sbe_rt::DecodeError> {
        let offset = self.pos + 0;
        if offset + 1 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 1,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(u8::from_le_bytes(bytes))
    }
    pub const unsafe fn octane_rating_unchecked(&self) -> u8 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 1];
        let mut j = 0;
        while j < 1 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        u8::from_le_bytes(bytes)
    }
    pub const fn raw_octane_rating(&self) -> u8 {
        #[allow(unused_unsafe)] unsafe { self.octane_rating_unchecked() }
    }
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    #[inline]
    fn tail_offset_1(&self) -> Result<usize, sbe_rt::DecodeError> {
        let start = self.tail_offset_0()?;
        if start + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: start + 4,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[start + j];
            j += 1;
        }
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
    pub fn acceleration(&self) -> Result<AccelerationDecoder<'a>, sbe_rt::DecodeError> {
        let offset = self.tail_offset_0()?;
        AccelerationDecoder::wrap(self.buf, offset, self.acting_version)
    }
    pub fn encoded_length(&self) -> usize {
        self.tail_offset_1().unwrap_or(self.pos + Self::ENTRY_BLOCK_LENGTH) - self.pos
    }
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
    acting_version: u16,
}
impl<'a> AccelerationDecoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    pub fn wrap(
        buf: &'a [u8],
        pos: usize,
        acting_version: u16,
    ) -> Result<Self, sbe_rt::DecodeError> {
        if pos + 4 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: pos + 4,
                available: buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = buf[pos + j];
            j += 1;
        }
        let header = GroupSizeEncoding(bytes);
        let count = header.num_in_group() as usize;
        Ok(Self {
            buf,
            pos: pos + 4,
            count,
            acting_version,
        })
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn as_chunks(&self) -> Result<&'a [[u8; 6]], sbe_rt::DecodeError> {
        let len = self.count * 6;
        if self.pos + len > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: self.pos + len,
                available: self.buf.len(),
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
        let size = entry.encoded_length();
        self.pos += size;
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
    pub const fn wrap(buf: &'a [u8], pos: usize, acting_version: u16) -> Self {
        Self { buf, pos, acting_version }
    }
    pub const fn mph(&self) -> Result<u16, sbe_rt::DecodeError> {
        let offset = self.pos + 0;
        if offset + 2 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 2,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(u16::from_le_bytes(bytes))
    }
    pub const unsafe fn mph_unchecked(&self) -> u16 {
        let offset = self.pos + 0;
        let mut bytes = [0u8; 2];
        let mut j = 0;
        while j < 2 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        u16::from_le_bytes(bytes)
    }
    pub const fn raw_mph(&self) -> u16 {
        #[allow(unused_unsafe)] unsafe { self.mph_unchecked() }
    }
    pub const fn seconds(&self) -> Result<f32, sbe_rt::DecodeError> {
        let offset = self.pos + 2;
        if offset + 4 > self.buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: offset + 4,
                available: self.buf.len(),
            });
        }
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = self.buf[offset + j];
            j += 1;
        }
        Ok(f32::from_le_bytes(bytes))
    }
    pub const unsafe fn seconds_unchecked(&self) -> f32 {
        let offset = self.pos + 2;
        let mut bytes = [0u8; 4];
        let mut j = 0;
        while j < 4 {
            bytes[j] = *self.buf.as_ptr().add(offset + j);
            j += 1;
        }
        f32::from_le_bytes(bytes)
    }
    pub const fn raw_seconds(&self) -> f32 {
        #[allow(unused_unsafe)] unsafe { self.seconds_unchecked() }
    }
    #[inline]
    fn tail_offset_0(&self) -> Result<usize, sbe_rt::DecodeError> {
        Ok(self.pos + Self::ENTRY_BLOCK_LENGTH)
    }
    pub fn encoded_length(&self) -> usize {
        self.tail_offset_0().unwrap_or(self.pos + Self::ENTRY_BLOCK_LENGTH) - self.pos
    }
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
    pub const BLOCK_LENGTH: usize = 43;
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            message_start: pos,
            pos: pos + 8 + 43,
            _phantom: core::marker::PhantomData,
        }
    }
    pub fn wrap_and_apply_header(
        buf: &'a mut [u8],
        pos: usize,
    ) -> Result<Self, sbe_rt::EncodeError> {
        let needed = pos + 8 + 43;
        if needed > buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: buf.len(),
            });
        }
        let header = MessageHeader::new(
            Self::BLOCK_LENGTH as u16,
            Self::TEMPLATE_ID,
            Self::SCHEMA_ID,
            Self::SCHEMA_VERSION,
        );
        let header_bytes = header.0;
        let mut j = 0;
        while j < 8 {
            buf[pos + j] = header_bytes[j];
            j += 1;
        }
        Ok(Self::wrap(buf, pos))
    }
    pub fn serial_number(&mut self, val: u64) -> &mut Self {
        let offset = self.message_start + 8 + 0;
        let val_bytes = val.to_le_bytes();
        let mut j = 0;
        while j < 8 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
    pub fn model_year(&mut self, val: u16) -> &mut Self {
        let offset = self.message_start + 8 + 8;
        let val_bytes = val.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
    pub fn available(&mut self, val: BooleanType) -> &mut Self {
        let offset = self.message_start + 8 + 10;
        let val_bytes = val.0.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
    pub fn code(&mut self, val: Model) -> &mut Self {
        let offset = self.message_start + 8 + 11;
        let val_bytes = val.0.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
    pub fn some_numbers(&mut self, val: [u32; 4]) -> &mut Self {
        let offset = self.message_start + 8 + 12;
        let mut idx = 0;
        while idx < 4 {
            let val_bytes = val[idx].to_le_bytes();
            let mut j = 0;
            while j < 4 {
                self.buf[offset + idx * 4 + j] = val_bytes[j];
                j += 1;
            }
            idx += 1;
        }
        self
    }
    pub fn vehicle_code(&mut self, val: [u8; 6]) -> &mut Self {
        let offset = self.message_start + 8 + 28;
        let mut idx = 0;
        while idx < 6 {
            let val_bytes = val[idx].to_le_bytes();
            let mut j = 0;
            while j < 1 {
                self.buf[offset + idx * 1 + j] = val_bytes[j];
                j += 1;
            }
            idx += 1;
        }
        self
    }
    pub fn extras(&mut self, val: OptionalExtras) -> &mut Self {
        let offset = self.message_start + 8 + 34;
        let val_bytes = val.0.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
    pub fn discounted_model(&mut self, val: Model) -> &mut Self {
        let offset = self.message_start + 8 + 35;
        let val_bytes = val.0.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
    pub fn engine(&mut self, val: Engine) -> &mut Self {
        let offset = self.message_start + 8 + 36;
        let mut j = 0;
        while j < 7 {
            self.buf[offset + j] = val.0[j];
            j += 1;
        }
        self
    }
    pub fn encoded_length(&self) -> usize {
        self.pos - (self.message_start + 8)
    }
    pub fn encoded_length_with_header(&self) -> usize {
        self.pos - self.message_start
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::NeedsFuelFigures> {
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
                needed: self.pos + 4,
                available: self.buf.len(),
            });
        }
        let header = GroupSizeEncoding::new(
            FuelFiguresEncoder::ENTRY_BLOCK_LENGTH as u16,
            count,
        );
        let header_bytes = header.0;
        let mut j = 0;
        while j < 4 {
            self.buf[self.pos + j] = header_bytes[j];
            j += 1;
        }
        let mut group = FuelFiguresEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::NeedsPerformanceFigures> {
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
                needed: self.pos + 4,
                available: self.buf.len(),
            });
        }
        let header = GroupSizeEncoding::new(
            PerformanceFiguresEncoder::ENTRY_BLOCK_LENGTH as u16,
            count,
        );
        let header_bytes = header.0;
        let mut j = 0;
        while j < 4 {
            self.buf[self.pos + j] = header_bytes[j];
            j += 1;
        }
        let mut group = PerformanceFiguresEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: group.pos,
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::NeedsManufacturer> {
    pub fn manufacturer(
        mut self,
        data: &[u8],
    ) -> Result<CarEncoder<'a, car_encoder_state::NeedsModel>, sbe_rt::EncodeError> {
        let needed = self.pos + 4 + data.len();
        if needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len(),
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        let mut j = 0;
        while j < 4 {
            self.buf[self.pos + j] = len_bytes[j];
            j += 1;
        }
        let start = self.pos + 4;
        let mut d = 0;
        while d < data.len() {
            self.buf[start + d] = data[d];
            d += 1;
        }
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::NeedsModel> {
    pub fn model(
        mut self,
        data: &[u8],
    ) -> Result<
        CarEncoder<'a, car_encoder_state::NeedsActivationCode>,
        sbe_rt::EncodeError,
    > {
        let needed = self.pos + 4 + data.len();
        if needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len(),
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        let mut j = 0;
        while j < 4 {
            self.buf[self.pos + j] = len_bytes[j];
            j += 1;
        }
        let start = self.pos + 4;
        let mut d = 0;
        while d < data.len() {
            self.buf[start + d] = data[d];
            d += 1;
        }
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::NeedsActivationCode> {
    pub fn activation_code(
        mut self,
        data: &[u8],
    ) -> Result<CarEncoder<'a, car_encoder_state::Complete>, sbe_rt::EncodeError> {
        let needed = self.pos + 4 + data.len();
        if needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len(),
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        let mut j = 0;
        while j < 4 {
            self.buf[self.pos + j] = len_bytes[j];
            j += 1;
        }
        let start = self.pos + 4;
        let mut d = 0;
        while d < data.len() {
            self.buf[start + d] = data[d];
            d += 1;
        }
        Ok(CarEncoder {
            buf: self.buf,
            message_start: self.message_start,
            pos: start + data.len(),
            _phantom: core::marker::PhantomData,
        })
    }
}
impl<'a> CarEncoder<'a, car_encoder_state::Complete> {
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
    const BLOCK_LENGTH: usize = 43;
    const SCHEMA_ID: u16 = 1;
    const SCHEMA_VERSION: u16 = 0;
}
pub struct FuelFiguresEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> FuelFiguresEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    pub fn wrap(buf: &'a mut [u8], pos: usize, count: u16) -> Self {
        Self {
            buf,
            pos,
            count,
            written: 0,
        }
    }
    pub fn add<F>(&mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(&mut FuelFiguresEntryEncoder<'a>),
    {
        if self.written >= self.count {
            return Ok(());
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: self.pos + block_len,
                available: self.buf.len(),
            });
        }
        let mut entry = FuelFiguresEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
pub struct FuelFiguresEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> FuelFiguresEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    pub fn speed(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        let val_bytes = val.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
    pub fn mpg(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        let val_bytes = val.to_le_bytes();
        let mut j = 0;
        while j < 4 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
    pub fn usage_description(
        &mut self,
        data: &[u8],
    ) -> Result<&mut Self, sbe_rt::EncodeError> {
        let needed = self.pos + 4 + data.len();
        if needed > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed,
                available: self.buf.len(),
            });
        }
        let len_bytes = (data.len() as u32).to_le_bytes();
        let mut j = 0;
        while j < 4 {
            self.buf[self.pos + j] = len_bytes[j];
            j += 1;
        }
        let start = self.pos + 4;
        let mut d = 0;
        while d < data.len() {
            self.buf[start + d] = data[d];
            d += 1;
        }
        self.pos = start + data.len();
        Ok(self)
    }
}
pub struct PerformanceFiguresEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> PerformanceFiguresEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    pub fn wrap(buf: &'a mut [u8], pos: usize, count: u16) -> Self {
        Self {
            buf,
            pos,
            count,
            written: 0,
        }
    }
    pub fn add<F>(&mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(&mut PerformanceFiguresEntryEncoder<'a>),
    {
        if self.written >= self.count {
            return Ok(());
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: self.pos + block_len,
                available: self.buf.len(),
            });
        }
        let mut entry = PerformanceFiguresEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
pub struct PerformanceFiguresEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> PerformanceFiguresEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 1;
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    pub fn octane_rating(&mut self, val: u8) -> &mut Self {
        let offset = self.entry_start + 0;
        let val_bytes = val.to_le_bytes();
        let mut j = 0;
        while j < 1 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
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
                needed: self.pos + 4,
                available: self.buf.len(),
            });
        }
        let header = GroupSizeEncoding::new(
            AccelerationEncoder::ENTRY_BLOCK_LENGTH as u16,
            count,
        );
        let header_bytes = header.0;
        let mut j = 0;
        while j < 4 {
            self.buf[self.pos + j] = header_bytes[j];
            j += 1;
        }
        let mut group = AccelerationEncoder::wrap(self.buf, self.pos + 4, count);
        f(&mut group);
        self.pos = group.pos;
        Ok(self)
    }
}
pub struct AccelerationEncoder<'a> {
    buf: &'a mut [u8],
    pos: usize,
    count: u16,
    written: u16,
}
impl<'a> AccelerationEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    pub fn wrap(buf: &'a mut [u8], pos: usize, count: u16) -> Self {
        Self {
            buf,
            pos,
            count,
            written: 0,
        }
    }
    pub fn add<F>(&mut self, f: F) -> Result<(), sbe_rt::EncodeError>
    where
        F: FnOnce(&mut AccelerationEntryEncoder<'a>),
    {
        if self.written >= self.count {
            return Ok(());
        }
        let block_len = Self::ENTRY_BLOCK_LENGTH;
        if self.pos + block_len > self.buf.len() {
            return Err(sbe_rt::EncodeError::BufferTooShort {
                needed: self.pos + block_len,
                available: self.buf.len(),
            });
        }
        let mut entry = AccelerationEntryEncoder::wrap(self.buf, self.pos);
        f(&mut entry);
        self.pos = entry.pos;
        self.written += 1;
        Ok(())
    }
}
pub struct AccelerationEntryEncoder<'a> {
    buf: &'a mut [u8],
    entry_start: usize,
    pos: usize,
}
impl<'a> AccelerationEntryEncoder<'a> {
    pub const ENTRY_BLOCK_LENGTH: usize = 6;
    pub fn wrap(buf: &'a mut [u8], pos: usize) -> Self {
        Self {
            buf,
            entry_start: pos,
            pos: pos + Self::ENTRY_BLOCK_LENGTH,
        }
    }
    pub fn mph(&mut self, val: u16) -> &mut Self {
        let offset = self.entry_start + 0;
        let val_bytes = val.to_le_bytes();
        let mut j = 0;
        while j < 2 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
    pub fn seconds(&mut self, val: f32) -> &mut Self {
        let offset = self.entry_start + 2;
        let val_bytes = val.to_le_bytes();
        let mut j = 0;
        while j < 4 {
            self.buf[offset + j] = val_bytes[j];
            j += 1;
        }
        self
    }
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
                            needed: self.pos + 4,
                            available: self.buf.len(),
                        }),
                    );
                }
                let mut bytes = [0u8; 4];
                let mut j = 0;
                while j < 4 {
                    bytes[j] = self.buf[self.pos + j];
                    j += 1;
                }
                let len = u32::from_le_bytes(bytes) as usize;
                (4, len)
            }
            FramingPolicy::LengthPrefixU16 => {
                if self.pos + 2 > self.buf.len() {
                    return Some(
                        Err(sbe_rt::DecodeError::BufferTooShort {
                            needed: self.pos + 2,
                            available: self.buf.len(),
                        }),
                    );
                }
                let mut bytes = [0u8; 2];
                let mut j = 0;
                while j < 2 {
                    bytes[j] = self.buf[self.pos + j];
                    j += 1;
                }
                let len = u16::from_le_bytes(bytes) as usize;
                (2, len)
            }
            FramingPolicy::Fixed(len) => (0, len),
        };
        if self.pos + header_len + frame_len > self.buf.len() {
            return Some(
                Err(sbe_rt::DecodeError::BufferTooShort {
                    needed: self.pos + header_len + frame_len,
                    available: self.buf.len(),
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
    pub const fn decode(buf: &'a [u8], pos: usize) -> Result<Self, sbe_rt::DecodeError> {
        if pos + 8 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: pos + 8,
                available: buf.len(),
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
    pub const fn decode_frame(
        buf: &'a [u8],
        pos: usize,
        frame_len: usize,
    ) -> Result<DecodedFrame<'a>, sbe_rt::DecodeError> {
        if pos + 8 > buf.len() {
            return Err(sbe_rt::DecodeError::BufferTooShort {
                needed: pos + 8,
                available: buf.len(),
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
                        needed: pos + frame_len,
                        available: buf.len(),
                    });
                }
                let payload = &buf[body_pos..pos + frame_len];
                Ok(DecodedFrame {
                    message: Self::Unknown { header, payload },
                    range: pos..pos + frame_len,
                    len: frame_len,
                })
            }
        }
    }
}

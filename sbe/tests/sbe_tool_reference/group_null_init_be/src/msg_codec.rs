use crate::*;

pub use decoder::MsgDecoder;
pub use encoder::MsgEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 66;
pub const SBE_TEMPLATE_ID: u16 = 1;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct MsgEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for MsgEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for MsgEncoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }

        /// Set all optional fields to their 'null' values.
        #[inline]
        fn nullify_optional_fields(&mut self) -> &mut Self {
            self.opt_scalar_opt(None);
            self.opt_enum_opt(None);
            {
                let mut composite_encoder = core::mem::take(self).opt_comp_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).opt_nest_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> MsgEncoder<'a> {
        pub fn wrap(mut self, buf: WriteBuf<'a>, offset: usize) -> Self {
            let limit = offset + SBE_BLOCK_LENGTH as usize;
            self.buf = buf;
            self.initial_offset = offset;
            self.offset = offset;
            self.limit = limit;
            self
        }

        #[inline]
        pub const fn encoded_length(&self) -> usize {
            self.limit - self.offset
        }

        #[inline]
        pub fn header(self, offset: usize) -> MessageHeaderEncoder<Self> {
            let mut header = MessageHeaderEncoder::default().wrap(self, offset);
            header.block_length(SBE_BLOCK_LENGTH);
            header.template_id(SBE_TEMPLATE_ID);
            header.schema_id(SBE_SCHEMA_ID);
            header.version(SBE_SCHEMA_VERSION);
            header
        }

        /// primitive field 'req'
        /// - min value: 0
        /// - max value: 4294967294
        /// - null value: 0xffffffff_u32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn req(&mut self, value: u32) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u32_at(offset, value);
            self
        }

        /// primitive field 'optScalar'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_scalar(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 4;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// optional primitive field 'optScalar'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 4
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn opt_scalar_opt(&mut self, value: Option<i32>) -> &mut Self {
            match value {
                Some(value) => self.opt_scalar(value),
                None => self.opt_scalar(-2147483648_i32),
            };
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn opt_enum(&mut self, value: side::Side) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'optEnum'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn opt_enum_opt(&mut self, value: Option<side::Side>) -> &mut Self {
            match value {
                Some(value) => self.opt_enum(value),
                None => self.opt_enum(side::Side::NullVal),
            };
            self
        }

        #[inline]
        pub fn opt_small_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'optSmall'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_small(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(4, value.len());
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'optSmall' from an Iterator
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_small_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'optSmall' with zero padding
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_small_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(4);
            self.opt_small_from_iter(iter);
            self
        }

        #[inline]
        pub fn opt_pair_at(&mut self, index: usize, value: u16) -> &mut Self {
            let offset = self.offset + 13;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset + index * 2, value);
            self
        }

        /// primitive array field 'optPair'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_pair(&mut self, value: &[u16]) -> &mut Self {
            debug_assert_eq!(2, value.len());
            let offset = self.offset + 13;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset, value[0]);
            buf.put_u16_at(offset + 2, value[1]);
            self
        }

        /// primitive array field 'optPair' from an Iterator
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_pair_from_iter(&mut self, iter: impl Iterator<Item = u16>) -> &mut Self {
            let offset = self.offset + 13;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u16_at(offset + i * 2, v);
            }
            self
        }

        /// primitive array field 'optPair' with zero padding
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_pair_zero_padded(&mut self, value: &[u16]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u16)).take(2);
            self.opt_pair_from_iter(iter);
            self
        }

        #[inline]
        pub fn opt_wide_at(&mut self, index: usize, value: i32) -> &mut Self {
            let offset = self.offset + 17;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset + index * 4, value);
            self
        }

        /// primitive array field 'optWide'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn opt_wide(&mut self, value: &[i32]) -> &mut Self {
            debug_assert_eq!(4, value.len());
            let offset = self.offset + 17;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset, value[0]);
            buf.put_i32_at(offset + 4, value[1]);
            buf.put_i32_at(offset + 8, value[2]);
            buf.put_i32_at(offset + 12, value[3]);
            self
        }

        /// primitive array field 'optWide' from an Iterator
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn opt_wide_from_iter(&mut self, iter: impl Iterator<Item = i32>) -> &mut Self {
            let offset = self.offset + 17;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i32_at(offset + i * 4, v);
            }
            self
        }

        /// primitive array field 'optWide' with zero padding
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn opt_wide_zero_padded(&mut self, value: &[i32]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i32)).take(4);
            self.opt_wide_from_iter(iter);
            self
        }

        #[inline]
        pub fn opt_text_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'optText'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn opt_text(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(12, value.len());
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'optText' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn opt_text_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'optText' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn opt_text_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(12);
            self.opt_text_from_iter(iter);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn opt_comp_encoder(self) -> pair_codec::PairEncoder<Self> {
            let offset = self.offset + 45;
            pair_codec::PairEncoder::default().wrap(self, offset)
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn opt_nest_encoder(self) -> outer_codec::OuterEncoder<Self> {
            let offset = self.offset + 53;
            outer_codec::OuterEncoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn opt_late_at(&mut self, index: usize, value: u16) -> &mut Self {
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset + index * 2, value);
            self
        }

        /// primitive array field 'optLate'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 4
        /// - version: 1
        #[inline]
        pub fn opt_late(&mut self, value: &[u16]) -> &mut Self {
            debug_assert_eq!(2, value.len());
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset, value[0]);
            buf.put_u16_at(offset + 2, value[1]);
            self
        }

        /// primitive array field 'optLate' from an Iterator
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 4
        /// - version: 1
        #[inline]
        pub fn opt_late_from_iter(&mut self, iter: impl Iterator<Item = u16>) -> &mut Self {
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u16_at(offset + i * 2, v);
            }
            self
        }

        /// primitive array field 'optLate' with zero padding
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 4
        /// - version: 1
        #[inline]
        pub fn opt_late_zero_padded(&mut self, value: &[u16]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u16)).take(2);
            self.opt_late_from_iter(iter);
            self
        }

        /// GROUP ENCODER (id=20)
        #[inline]
        pub fn rows_encoder(self, count: u16, rows_encoder: RowsEncoder<Self>) -> RowsEncoder<Self> {
            rows_encoder.wrap(self, count)
        }

    }

    #[derive(Debug, Default)]
    pub struct RowsEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for RowsEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for RowsEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }

        /// Set all optional fields to their 'null' values.
        #[inline]
        fn nullify_optional_fields(&mut self) -> &mut Self {
            self.opt_scalar_opt(None);
            self.opt_enum_opt(None);
            {
                let mut composite_encoder = core::mem::take(self).opt_comp_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).opt_nest_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a, P> RowsEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        pub fn wrap(
            mut self,
            mut parent: P,
            count: u16,
        ) -> Self {
            let initial_limit = parent.get_limit();
            parent.set_limit(initial_limit + 4);
            parent.get_buf_mut().put_u16_at(initial_limit, Self::block_length());
            parent.get_buf_mut().put_u16_at(initial_limit + 2, count);
            self.parent = Some(parent);
            self.count = count;
            self.index = usize::MAX;
            self.offset = usize::MAX;
            self.initial_limit = initial_limit;
            self
        }

        #[inline]
        pub const fn block_length() -> u16 {
            66
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// will return Some(current index) when successful otherwise None
        #[inline]
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + Self::block_length() as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        /// primitive field 'req'
        /// - min value: 0
        /// - max value: 4294967294
        /// - null value: 0xffffffff_u32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn req(&mut self, value: u32) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u32_at(offset, value);
            self
        }

        /// primitive field 'optScalar'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_scalar(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 4;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// optional primitive field 'optScalar'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 4
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn opt_scalar_opt(&mut self, value: Option<i32>) -> &mut Self {
            match value {
                Some(value) => self.opt_scalar(value),
                None => self.opt_scalar(-2147483648_i32),
            };
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn opt_enum(&mut self, value: side::Side) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'optEnum'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn opt_enum_opt(&mut self, value: Option<side::Side>) -> &mut Self {
            match value {
                Some(value) => self.opt_enum(value),
                None => self.opt_enum(side::Side::NullVal),
            };
            self
        }

        #[inline]
        pub fn opt_small_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'optSmall'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_small(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(4, value.len());
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'optSmall' from an Iterator
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_small_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'optSmall' with zero padding
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_small_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(4);
            self.opt_small_from_iter(iter);
            self
        }

        #[inline]
        pub fn opt_pair_at(&mut self, index: usize, value: u16) -> &mut Self {
            let offset = self.offset + 13;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset + index * 2, value);
            self
        }

        /// primitive array field 'optPair'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_pair(&mut self, value: &[u16]) -> &mut Self {
            debug_assert_eq!(2, value.len());
            let offset = self.offset + 13;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset, value[0]);
            buf.put_u16_at(offset + 2, value[1]);
            self
        }

        /// primitive array field 'optPair' from an Iterator
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_pair_from_iter(&mut self, iter: impl Iterator<Item = u16>) -> &mut Self {
            let offset = self.offset + 13;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u16_at(offset + i * 2, v);
            }
            self
        }

        /// primitive array field 'optPair' with zero padding
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_pair_zero_padded(&mut self, value: &[u16]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u16)).take(2);
            self.opt_pair_from_iter(iter);
            self
        }

        #[inline]
        pub fn opt_wide_at(&mut self, index: usize, value: i32) -> &mut Self {
            let offset = self.offset + 17;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset + index * 4, value);
            self
        }

        /// primitive array field 'optWide'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn opt_wide(&mut self, value: &[i32]) -> &mut Self {
            debug_assert_eq!(4, value.len());
            let offset = self.offset + 17;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset, value[0]);
            buf.put_i32_at(offset + 4, value[1]);
            buf.put_i32_at(offset + 8, value[2]);
            buf.put_i32_at(offset + 12, value[3]);
            self
        }

        /// primitive array field 'optWide' from an Iterator
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn opt_wide_from_iter(&mut self, iter: impl Iterator<Item = i32>) -> &mut Self {
            let offset = self.offset + 17;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i32_at(offset + i * 4, v);
            }
            self
        }

        /// primitive array field 'optWide' with zero padding
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn opt_wide_zero_padded(&mut self, value: &[i32]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i32)).take(4);
            self.opt_wide_from_iter(iter);
            self
        }

        #[inline]
        pub fn opt_text_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'optText'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn opt_text(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(12, value.len());
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'optText' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn opt_text_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'optText' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn opt_text_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(12);
            self.opt_text_from_iter(iter);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn opt_comp_encoder(self) -> pair_codec::PairEncoder<Self> {
            let offset = self.offset + 45;
            pair_codec::PairEncoder::default().wrap(self, offset)
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn opt_nest_encoder(self) -> outer_codec::OuterEncoder<Self> {
            let offset = self.offset + 53;
            outer_codec::OuterEncoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn opt_late_at(&mut self, index: usize, value: u16) -> &mut Self {
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset + index * 2, value);
            self
        }

        /// primitive array field 'optLate'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 4
        /// - version: 1
        #[inline]
        pub fn opt_late(&mut self, value: &[u16]) -> &mut Self {
            debug_assert_eq!(2, value.len());
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset, value[0]);
            buf.put_u16_at(offset + 2, value[1]);
            self
        }

        /// primitive array field 'optLate' from an Iterator
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 4
        /// - version: 1
        #[inline]
        pub fn opt_late_from_iter(&mut self, iter: impl Iterator<Item = u16>) -> &mut Self {
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u16_at(offset + i * 2, v);
            }
            self
        }

        /// primitive array field 'optLate' with zero padding
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 4
        /// - version: 1
        #[inline]
        pub fn opt_late_zero_padded(&mut self, value: &[u16]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u16)).take(2);
            self.opt_late_from_iter(iter);
            self
        }

        /// GROUP ENCODER (id=30)
        #[inline]
        pub fn cells_encoder(self, count: u16, cells_encoder: CellsEncoder<Self>) -> CellsEncoder<Self> {
            cells_encoder.wrap(self, count)
        }

    }

    #[derive(Debug, Default)]
    pub struct CellsEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for CellsEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for CellsEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }

        /// Set all optional fields to their 'null' values.
        #[inline]
        fn nullify_optional_fields(&mut self) -> &mut Self {
            self.opt_scalar_opt(None);
            self.opt_enum_opt(None);
            {
                let mut composite_encoder = core::mem::take(self).opt_comp_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).opt_nest_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a, P> CellsEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        pub fn wrap(
            mut self,
            mut parent: P,
            count: u16,
        ) -> Self {
            let initial_limit = parent.get_limit();
            parent.set_limit(initial_limit + 4);
            parent.get_buf_mut().put_u16_at(initial_limit, Self::block_length());
            parent.get_buf_mut().put_u16_at(initial_limit + 2, count);
            self.parent = Some(parent);
            self.count = count;
            self.index = usize::MAX;
            self.offset = usize::MAX;
            self.initial_limit = initial_limit;
            self
        }

        #[inline]
        pub const fn block_length() -> u16 {
            66
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// will return Some(current index) when successful otherwise None
        #[inline]
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + Self::block_length() as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        /// primitive field 'req'
        /// - min value: 0
        /// - max value: 4294967294
        /// - null value: 0xffffffff_u32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn req(&mut self, value: u32) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u32_at(offset, value);
            self
        }

        /// primitive field 'optScalar'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_scalar(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 4;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// optional primitive field 'optScalar'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 4
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn opt_scalar_opt(&mut self, value: Option<i32>) -> &mut Self {
            match value {
                Some(value) => self.opt_scalar(value),
                None => self.opt_scalar(-2147483648_i32),
            };
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn opt_enum(&mut self, value: side::Side) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'optEnum'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn opt_enum_opt(&mut self, value: Option<side::Side>) -> &mut Self {
            match value {
                Some(value) => self.opt_enum(value),
                None => self.opt_enum(side::Side::NullVal),
            };
            self
        }

        #[inline]
        pub fn opt_small_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'optSmall'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_small(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(4, value.len());
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'optSmall' from an Iterator
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_small_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 9;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'optSmall' with zero padding
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 9
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_small_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(4);
            self.opt_small_from_iter(iter);
            self
        }

        #[inline]
        pub fn opt_pair_at(&mut self, index: usize, value: u16) -> &mut Self {
            let offset = self.offset + 13;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset + index * 2, value);
            self
        }

        /// primitive array field 'optPair'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_pair(&mut self, value: &[u16]) -> &mut Self {
            debug_assert_eq!(2, value.len());
            let offset = self.offset + 13;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset, value[0]);
            buf.put_u16_at(offset + 2, value[1]);
            self
        }

        /// primitive array field 'optPair' from an Iterator
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_pair_from_iter(&mut self, iter: impl Iterator<Item = u16>) -> &mut Self {
            let offset = self.offset + 13;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u16_at(offset + i * 2, v);
            }
            self
        }

        /// primitive array field 'optPair' with zero padding
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 13
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn opt_pair_zero_padded(&mut self, value: &[u16]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u16)).take(2);
            self.opt_pair_from_iter(iter);
            self
        }

        #[inline]
        pub fn opt_wide_at(&mut self, index: usize, value: i32) -> &mut Self {
            let offset = self.offset + 17;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset + index * 4, value);
            self
        }

        /// primitive array field 'optWide'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn opt_wide(&mut self, value: &[i32]) -> &mut Self {
            debug_assert_eq!(4, value.len());
            let offset = self.offset + 17;
            let buf = self.get_buf_mut();
            buf.put_i32_at(offset, value[0]);
            buf.put_i32_at(offset + 4, value[1]);
            buf.put_i32_at(offset + 8, value[2]);
            buf.put_i32_at(offset + 12, value[3]);
            self
        }

        /// primitive array field 'optWide' from an Iterator
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn opt_wide_from_iter(&mut self, iter: impl Iterator<Item = i32>) -> &mut Self {
            let offset = self.offset + 17;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_i32_at(offset + i * 4, v);
            }
            self
        }

        /// primitive array field 'optWide' with zero padding
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 17
        /// - encodedLength: 16
        /// - version: 0
        #[inline]
        pub fn opt_wide_zero_padded(&mut self, value: &[i32]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_i32)).take(4);
            self.opt_wide_from_iter(iter);
            self
        }

        #[inline]
        pub fn opt_text_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'optText'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn opt_text(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(12, value.len());
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'optText' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn opt_text_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'optText' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn opt_text_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(12);
            self.opt_text_from_iter(iter);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn opt_comp_encoder(self) -> pair_codec::PairEncoder<Self> {
            let offset = self.offset + 45;
            pair_codec::PairEncoder::default().wrap(self, offset)
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn opt_nest_encoder(self) -> outer_codec::OuterEncoder<Self> {
            let offset = self.offset + 53;
            outer_codec::OuterEncoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn opt_late_at(&mut self, index: usize, value: u16) -> &mut Self {
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset + index * 2, value);
            self
        }

        /// primitive array field 'optLate'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 4
        /// - version: 1
        #[inline]
        pub fn opt_late(&mut self, value: &[u16]) -> &mut Self {
            debug_assert_eq!(2, value.len());
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            buf.put_u16_at(offset, value[0]);
            buf.put_u16_at(offset + 2, value[1]);
            self
        }

        /// primitive array field 'optLate' from an Iterator
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 4
        /// - version: 1
        #[inline]
        pub fn opt_late_from_iter(&mut self, iter: impl Iterator<Item = u16>) -> &mut Self {
            let offset = self.offset + 62;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u16_at(offset + i * 2, v);
            }
            self
        }

        /// primitive array field 'optLate' with zero padding
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 62
        /// - encodedLength: 4
        /// - version: 1
        #[inline]
        pub fn opt_late_zero_padded(&mut self, value: &[u16]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u16)).take(2);
            self.opt_late_from_iter(iter);
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct MsgDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for MsgDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for MsgDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for MsgDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> MsgDecoder<'a> {
        pub fn wrap(
            mut self,
            buf: ReadBuf<'a>,
            offset: usize,
            acting_block_length: u16,
            acting_version: u16,
        ) -> Self {
            let limit = offset + acting_block_length as usize;
            self.buf = buf;
            self.initial_offset = offset;
            self.offset = offset;
            self.limit = limit;
            self.acting_block_length = acting_block_length;
            self.acting_version = acting_version;
            self
        }

        #[inline]
        pub const fn encoded_length(&self) -> usize {
            self.limit - self.offset
        }

        #[inline]
        pub fn header(self, mut header: MessageHeaderDecoder<ReadBuf<'a>>, offset: usize) -> Self {
            debug_assert_eq!(SBE_TEMPLATE_ID, header.template_id());
            let acting_block_length = header.block_length();
            let acting_version = header.version();

            self.wrap(
                header.parent().unwrap(),
                offset + message_header_codec::ENCODED_LENGTH,
                acting_block_length,
                acting_version,
            )
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn req(&self) -> u32 {
            self.get_buf().get_u32_at(self.offset)
        }

        /// primitive field - 'OPTIONAL' { null_value: '-2147483648_i32' }
        #[inline]
        pub fn opt_scalar(&self) -> Option<i32> {
            let value = self.get_buf().get_i32_at(self.offset + 4);
            if value == -2147483648_i32 {
                None
            } else {
                Some(value)
            }
        }

        /// REQUIRED enum
        #[inline]
        pub fn opt_enum(&self) -> side::Side {
            self.get_buf().get_u8_at(self.offset + 8).into()
        }

        #[inline]
        pub fn opt_small(&self) -> [u8; 4] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 9)
        }

        #[inline]
        pub fn opt_pair(&self) -> [u16; 2] {
            let buf = self.get_buf();
            [
                buf.get_u16_at(self.offset + 13),
                buf.get_u16_at(self.offset + 13 + 2),
            ]
        }

        #[inline]
        pub fn opt_wide(&self) -> [i32; 4] {
            let buf = self.get_buf();
            [
                buf.get_i32_at(self.offset + 17),
                buf.get_i32_at(self.offset + 17 + 4),
                buf.get_i32_at(self.offset + 17 + 8),
                buf.get_i32_at(self.offset + 17 + 12),
            ]
        }

        #[inline]
        pub fn opt_text(&self) -> [u8; 12] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 33)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn opt_comp_decoder(self) -> pair_codec::PairDecoder<Self> {
            let offset = self.offset + 45;
            pair_codec::PairDecoder::default().wrap(self, offset)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn opt_nest_decoder(self) -> outer_codec::OuterDecoder<Self> {
            let offset = self.offset + 53;
            outer_codec::OuterDecoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn opt_late(&self) -> [u16; 2] {
            if self.acting_version() < 1 {
                return [0xffff_u16; 2];
            }

            let buf = self.get_buf();
            [
                buf.get_u16_at(self.offset + 62),
                buf.get_u16_at(self.offset + 62 + 2),
            ]
        }

        /// GROUP DECODER (id=20)
        #[inline]
        pub fn rows_decoder(self) -> RowsDecoder<Self> {
            RowsDecoder::default().wrap(self)
        }

    }

    #[derive(Debug, Default)]
    pub struct RowsDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for RowsDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for RowsDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for RowsDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> RowsDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        pub fn wrap(
            mut self,
            mut parent: P,
        ) -> Self {
            let initial_offset = parent.get_limit();
            let block_length = parent.get_buf().get_u16_at(initial_offset);
            let count = parent.get_buf().get_u16_at(initial_offset + 2);
            parent.set_limit(initial_offset + 4);
            self.parent = Some(parent);
            self.block_length = block_length;
            self.count = count;
            self.index = usize::MAX;
            self.offset = 0;
            self
        }

        /// group token - Token{signal=BEGIN_GROUP, name='rows', referencedName='null', description='null', packageName='null', id=20, version=0, deprecated=0, encodedLength=66, offset=66, componentTokenCount=96, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        #[inline]
        pub fn acting_version(&mut self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }

        #[inline]
        pub fn count(&self) -> u16 {
            self.count
        }

        /// will return Some(current index) when successful otherwise None
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                 return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + self.block_length as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn req(&self) -> u32 {
            self.get_buf().get_u32_at(self.offset)
        }

        /// primitive field - 'OPTIONAL' { null_value: '-2147483648_i32' }
        #[inline]
        pub fn opt_scalar(&self) -> Option<i32> {
            let value = self.get_buf().get_i32_at(self.offset + 4);
            if value == -2147483648_i32 {
                None
            } else {
                Some(value)
            }
        }

        /// REQUIRED enum
        #[inline]
        pub fn opt_enum(&self) -> side::Side {
            self.get_buf().get_u8_at(self.offset + 8).into()
        }

        #[inline]
        pub fn opt_small(&self) -> [u8; 4] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 9)
        }

        #[inline]
        pub fn opt_pair(&self) -> [u16; 2] {
            let buf = self.get_buf();
            [
                buf.get_u16_at(self.offset + 13),
                buf.get_u16_at(self.offset + 13 + 2),
            ]
        }

        #[inline]
        pub fn opt_wide(&self) -> [i32; 4] {
            let buf = self.get_buf();
            [
                buf.get_i32_at(self.offset + 17),
                buf.get_i32_at(self.offset + 17 + 4),
                buf.get_i32_at(self.offset + 17 + 8),
                buf.get_i32_at(self.offset + 17 + 12),
            ]
        }

        #[inline]
        pub fn opt_text(&self) -> [u8; 12] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 33)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn opt_comp_decoder(self) -> pair_codec::PairDecoder<Self> {
            let offset = self.offset + 45;
            pair_codec::PairDecoder::default().wrap(self, offset)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn opt_nest_decoder(self) -> outer_codec::OuterDecoder<Self> {
            let offset = self.offset + 53;
            outer_codec::OuterDecoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn opt_late(&self) -> [u16; 2] {
            if self.acting_version() < 1 {
                return [0xffff_u16; 2];
            }

            let buf = self.get_buf();
            [
                buf.get_u16_at(self.offset + 62),
                buf.get_u16_at(self.offset + 62 + 2),
            ]
        }

        /// GROUP DECODER (id=30)
        #[inline]
        pub fn cells_decoder(self) -> CellsDecoder<Self> {
            CellsDecoder::default().wrap(self)
        }

    }

    #[derive(Debug, Default)]
    pub struct CellsDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for CellsDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for CellsDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for CellsDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> CellsDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        pub fn wrap(
            mut self,
            mut parent: P,
        ) -> Self {
            let initial_offset = parent.get_limit();
            let block_length = parent.get_buf().get_u16_at(initial_offset);
            let count = parent.get_buf().get_u16_at(initial_offset + 2);
            parent.set_limit(initial_offset + 4);
            self.parent = Some(parent);
            self.block_length = block_length;
            self.count = count;
            self.index = usize::MAX;
            self.offset = 0;
            self
        }

        /// group token - Token{signal=BEGIN_GROUP, name='cells', referencedName='null', description='null', packageName='null', id=30, version=0, deprecated=0, encodedLength=66, offset=66, componentTokenCount=48, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        #[inline]
        pub fn acting_version(&mut self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }

        #[inline]
        pub fn count(&self) -> u16 {
            self.count
        }

        /// will return Some(current index) when successful otherwise None
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                 return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + self.block_length as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn req(&self) -> u32 {
            self.get_buf().get_u32_at(self.offset)
        }

        /// primitive field - 'OPTIONAL' { null_value: '-2147483648_i32' }
        #[inline]
        pub fn opt_scalar(&self) -> Option<i32> {
            let value = self.get_buf().get_i32_at(self.offset + 4);
            if value == -2147483648_i32 {
                None
            } else {
                Some(value)
            }
        }

        /// REQUIRED enum
        #[inline]
        pub fn opt_enum(&self) -> side::Side {
            self.get_buf().get_u8_at(self.offset + 8).into()
        }

        #[inline]
        pub fn opt_small(&self) -> [u8; 4] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 9)
        }

        #[inline]
        pub fn opt_pair(&self) -> [u16; 2] {
            let buf = self.get_buf();
            [
                buf.get_u16_at(self.offset + 13),
                buf.get_u16_at(self.offset + 13 + 2),
            ]
        }

        #[inline]
        pub fn opt_wide(&self) -> [i32; 4] {
            let buf = self.get_buf();
            [
                buf.get_i32_at(self.offset + 17),
                buf.get_i32_at(self.offset + 17 + 4),
                buf.get_i32_at(self.offset + 17 + 8),
                buf.get_i32_at(self.offset + 17 + 12),
            ]
        }

        #[inline]
        pub fn opt_text(&self) -> [u8; 12] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 33)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn opt_comp_decoder(self) -> pair_codec::PairDecoder<Self> {
            let offset = self.offset + 45;
            pair_codec::PairDecoder::default().wrap(self, offset)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn opt_nest_decoder(self) -> outer_codec::OuterDecoder<Self> {
            let offset = self.offset + 53;
            outer_codec::OuterDecoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn opt_late(&self) -> [u16; 2] {
            if self.acting_version() < 1 {
                return [0xffff_u16; 2];
            }

            let buf = self.get_buf();
            [
                buf.get_u16_at(self.offset + 62),
                buf.get_u16_at(self.offset + 62 + 2),
            ]
        }

    }

} // end decoder

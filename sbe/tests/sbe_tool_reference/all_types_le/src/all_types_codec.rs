use crate::*;

pub use decoder::AllTypesDecoder;
pub use encoder::AllTypesEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 60;
pub const SBE_TEMPLATE_ID: u16 = 1;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct AllTypesEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for AllTypesEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for AllTypesEncoder<'a> {
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
            {
                let mut composite_encoder = core::mem::take(self).scalar_composite_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).float_pair_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> AllTypesEncoder<'a> {
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

        /// COMPOSITE ENCODER
        #[inline]
        pub fn scalar_composite_encoder(self) -> all_scalars_codec::AllScalarsEncoder<Self> {
            let offset = self.offset;
            all_scalars_codec::AllScalarsEncoder::default().wrap(self, offset)
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn float_pair_encoder(self) -> float_pair_codec::FloatPairEncoder<Self> {
            let offset = self.offset + 42;
            float_pair_codec::FloatPairEncoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn enum_field(&mut self, value: test_enum::TestEnum) -> &mut Self {
            let offset = self.offset + 50;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn set_field(&mut self, value: test_set::TestSet) {
            let offset = self.offset + 51;
            self.get_buf_mut().put_u8_at(offset, value.0)
        }

        #[inline]
        pub fn fixed_array_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 52;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'fixedArray'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 52
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn fixed_array(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(8, value.len());
            let offset = self.offset + 52;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'fixedArray' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 52
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn fixed_array_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 52;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'fixedArray' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: null
        /// - encodedOffset: 52
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn fixed_array_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(8);
            self.fixed_array_from_iter(iter);
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'None'
        #[inline]
        pub fn var_data(&mut self, value: &[u8]) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u16::MAX - 1) as usize);
            self.set_limit(limit + 2 + data_length);
            self.get_buf_mut().put_u16_at(limit, data_length as u16);
            self.get_buf_mut().put_slice_at(limit + 2, &value[0..data_length]);
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct AllTypesDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for AllTypesDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for AllTypesDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for AllTypesDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> AllTypesDecoder<'a> {
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

        /// COMPOSITE DECODER
        #[inline]
        pub fn scalar_composite_decoder(self) -> all_scalars_codec::AllScalarsDecoder<Self> {
            let offset = self.offset;
            all_scalars_codec::AllScalarsDecoder::default().wrap(self, offset)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn float_pair_decoder(self) -> float_pair_codec::FloatPairDecoder<Self> {
            let offset = self.offset + 42;
            float_pair_codec::FloatPairDecoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn enum_field(&self) -> test_enum::TestEnum {
            self.get_buf().get_u8_at(self.offset + 50).into()
        }

        /// BIT SET DECODER
        #[inline]
        pub fn set_field(&self) -> test_set::TestSet {
            test_set::TestSet::new(self.get_buf().get_u8_at(self.offset + 51))
        }

        #[inline]
        pub fn fixed_array(&self) -> [u8; 8] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 52)
        }

        /// VAR_DATA DECODER - character encoding: 'None'
        #[inline]
        pub fn var_data_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u16_at(offset) as usize;
            self.set_limit(offset + 2 + data_length);
            (offset + 2, data_length)
        }

        #[inline]
        pub fn var_data_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

    }

} // end decoder


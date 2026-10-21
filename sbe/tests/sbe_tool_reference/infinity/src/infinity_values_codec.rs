use crate::*;

pub use decoder::InfinityValuesDecoder;
pub use encoder::InfinityValuesEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 24;
pub const SBE_TEMPLATE_ID: u16 = 1;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct InfinityValuesEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for InfinityValuesEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for InfinityValuesEncoder<'a> {
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
            self.float_positive_null_opt(None);
            self.float_negative_null_opt(None);
            self.double_positive_null_opt(None);
            self.double_negative_null_opt(None);
            self
        }
    }

    impl<'a> InfinityValuesEncoder<'a> {
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

        // skipping CONSTANT floatPositiveInfinity

        // skipping CONSTANT floatNegativeInfinity

        // skipping CONSTANT doublePositiveInfinity

        // skipping CONSTANT doubleNegativeInfinity

        /// primitive field 'floatPositiveNull'
        /// - min value: -Infinity
        /// - max value: 1.5
        /// - null value: f32::INFINITY
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn float_positive_null(&mut self, value: f32) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_f32_at(offset, value);
            self
        }

        /// optional primitive field 'floatPositiveNull'
        /// - min value: -Infinity
        /// - max value: 1.5
        /// - null value: f32::INFINITY
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 4
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn float_positive_null_opt(&mut self, value: Option<f32>) -> &mut Self {
            match value {
                Some(value) => self.float_positive_null(value),
                None => self.float_positive_null(f32::INFINITY),
            };
            self
        }

        /// primitive field 'floatNegativeNull'
        /// - min value: -1.5
        /// - max value: Infinity
        /// - null value: -f32::INFINITY
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn float_negative_null(&mut self, value: f32) -> &mut Self {
            let offset = self.offset + 4;
            self.get_buf_mut().put_f32_at(offset, value);
            self
        }

        /// optional primitive field 'floatNegativeNull'
        /// - min value: -1.5
        /// - max value: Infinity
        /// - null value: -f32::INFINITY
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 4
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn float_negative_null_opt(&mut self, value: Option<f32>) -> &mut Self {
            match value {
                Some(value) => self.float_negative_null(value),
                None => self.float_negative_null(-f32::INFINITY),
            };
            self
        }

        /// primitive field 'doublePositiveNull'
        /// - min value: -Infinity
        /// - max value: 1.5
        /// - null value: f64::INFINITY
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn double_positive_null(&mut self, value: f64) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_f64_at(offset, value);
            self
        }

        /// optional primitive field 'doublePositiveNull'
        /// - min value: -Infinity
        /// - max value: 1.5
        /// - null value: f64::INFINITY
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 8
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn double_positive_null_opt(&mut self, value: Option<f64>) -> &mut Self {
            match value {
                Some(value) => self.double_positive_null(value),
                None => self.double_positive_null(f64::INFINITY),
            };
            self
        }

        /// primitive field 'doubleNegativeNull'
        /// - min value: -1.5
        /// - max value: Infinity
        /// - null value: -f64::INFINITY
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn double_negative_null(&mut self, value: f64) -> &mut Self {
            let offset = self.offset + 16;
            self.get_buf_mut().put_f64_at(offset, value);
            self
        }

        /// optional primitive field 'doubleNegativeNull'
        /// - min value: -1.5
        /// - max value: Infinity
        /// - null value: -f64::INFINITY
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn double_negative_null_opt(&mut self, value: Option<f64>) -> &mut Self {
            match value {
                Some(value) => self.double_negative_null(value),
                None => self.double_negative_null(-f64::INFINITY),
            };
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct InfinityValuesDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for InfinityValuesDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for InfinityValuesDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for InfinityValuesDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> InfinityValuesDecoder<'a> {
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

        /// CONSTANT
        #[inline]
        pub fn float_positive_infinity(&self) -> f32 {
            f32::INFINITY
        }

        /// CONSTANT
        #[inline]
        pub fn float_negative_infinity(&self) -> f32 {
            -f32::INFINITY
        }

        /// CONSTANT
        #[inline]
        pub fn double_positive_infinity(&self) -> f64 {
            f64::INFINITY
        }

        /// CONSTANT
        #[inline]
        pub fn double_negative_infinity(&self) -> f64 {
            -f64::INFINITY
        }

        /// primitive field - 'OPTIONAL' { null_value: 'f32::INFINITY' }
        #[inline]
        pub fn float_positive_null(&self) -> Option<f32> {
            let value = self.get_buf().get_f32_at(self.offset);
            if value == f32::INFINITY {
                None
            } else {
                Some(value)
            }
        }

        /// primitive field - 'OPTIONAL' { null_value: '-f32::INFINITY' }
        #[inline]
        pub fn float_negative_null(&self) -> Option<f32> {
            let value = self.get_buf().get_f32_at(self.offset + 4);
            if value == -f32::INFINITY {
                None
            } else {
                Some(value)
            }
        }

        /// primitive field - 'OPTIONAL' { null_value: 'f64::INFINITY' }
        #[inline]
        pub fn double_positive_null(&self) -> Option<f64> {
            let value = self.get_buf().get_f64_at(self.offset + 8);
            if value == f64::INFINITY {
                None
            } else {
                Some(value)
            }
        }

        /// primitive field - 'OPTIONAL' { null_value: '-f64::INFINITY' }
        #[inline]
        pub fn double_negative_null(&self) -> Option<f64> {
            let value = self.get_buf().get_f64_at(self.offset + 16);
            if value == -f64::INFINITY {
                None
            } else {
                Some(value)
            }
        }

    }

} // end decoder

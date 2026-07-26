use crate::*;

pub use decoder::Issue895Decoder;
pub use encoder::Issue895Encoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 12;
pub const SBE_TEMPLATE_ID: u16 = 1;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct Issue895Encoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for Issue895Encoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for Issue895Encoder<'a> {
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
            self.optional_float_opt(None);
            self.optional_double_opt(None);
            self
        }
    }

    impl<'a> Issue895Encoder<'a> {
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

        /// primitive field 'optional_float'
        /// - min value: -3.4028234663852886E38
        /// - max value: 3.4028234663852886E38
        /// - null value: f32::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn optional_float(&mut self, value: f32) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_f32_at(offset, value);
            self
        }

        /// optional primitive field 'optional_float'
        /// - min value: -3.4028234663852886E38
        /// - max value: 3.4028234663852886E38
        /// - null value: f32::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 4
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn optional_float_opt(&mut self, value: Option<f32>) -> &mut Self {
            match value {
                Some(value) => self.optional_float(value),
                None => self.optional_float(f32::NAN),
            };
            self
        }

        /// primitive field 'optional_double'
        /// - min value: -1.7976931348623157E308
        /// - max value: 1.7976931348623157E308
        /// - null value: f64::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn optional_double(&mut self, value: f64) -> &mut Self {
            let offset = self.offset + 4;
            self.get_buf_mut().put_f64_at(offset, value);
            self
        }

        /// optional primitive field 'optional_double'
        /// - min value: -1.7976931348623157E308
        /// - max value: 1.7976931348623157E308
        /// - null value: f64::NAN
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 4
        /// - encodedLength: 8
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn optional_double_opt(&mut self, value: Option<f64>) -> &mut Self {
            match value {
                Some(value) => self.optional_double(value),
                None => self.optional_double(f64::NAN),
            };
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct Issue895Decoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for Issue895Decoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for Issue895Decoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for Issue895Decoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> Issue895Decoder<'a> {
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

        /// primitive field - 'OPTIONAL' { null_value: 'f32::NAN' }
        #[inline]
        pub fn optional_float(&self) -> Option<f32> {
            let value = self.get_buf().get_f32_at(self.offset);
            if value.is_nan() {
                None
            } else {
                Some(value)
            }
        }

        /// primitive field - 'OPTIONAL' { null_value: 'f64::NAN' }
        #[inline]
        pub fn optional_double(&self) -> Option<f64> {
            let value = self.get_buf().get_f64_at(self.offset + 4);
            if value.is_nan() {
                None
            } else {
                Some(value)
            }
        }

    }

} // end decoder


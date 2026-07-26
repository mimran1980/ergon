use crate::*;

pub use decoder::OptionalEnumNullifyDecoder;
pub use encoder::OptionalEnumNullifyEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 4;
pub const SBE_TEMPLATE_ID: u16 = 1;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct OptionalEnumNullifyEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for OptionalEnumNullifyEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for OptionalEnumNullifyEncoder<'a> {
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
            self.optional_enum_opt(None);
            {
                let mut composite_encoder = core::mem::take(self).optional_composite_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> OptionalEnumNullifyEncoder<'a> {
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

        /// REQUIRED enum
        #[inline]
        pub fn optional_enum(&mut self, value: enum_type::EnumType) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'optionalEnum'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn optional_enum_opt(&mut self, value: Option<enum_type::EnumType>) -> &mut Self {
            match value {
                Some(value) => self.optional_enum(value),
                None => self.optional_enum(enum_type::EnumType::NullVal),
            };
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn required_enum_from_optional_type(&mut self, value: optional_encoding_enum_type::OptionalEncodingEnumType) -> &mut Self {
            let offset = self.offset + 1;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn optional_composite_encoder(self) -> optional_composite_codec::OptionalCompositeEncoder<Self> {
            let offset = self.offset + 2;
            optional_composite_codec::OptionalCompositeEncoder::default().wrap(self, offset)
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct OptionalEnumNullifyDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for OptionalEnumNullifyDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for OptionalEnumNullifyDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for OptionalEnumNullifyDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> OptionalEnumNullifyDecoder<'a> {
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

        /// REQUIRED enum
        #[inline]
        pub fn optional_enum(&self) -> enum_type::EnumType {
            self.get_buf().get_u8_at(self.offset).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn required_enum_from_optional_type(&self) -> optional_encoding_enum_type::OptionalEncodingEnumType {
            self.get_buf().get_u8_at(self.offset + 1).into()
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn optional_composite_decoder(self) -> optional_composite_codec::OptionalCompositeDecoder<Self> {
            let offset = self.offset + 2;
            optional_composite_codec::OptionalCompositeDecoder::default().wrap(self, offset)
        }

    }

} // end decoder

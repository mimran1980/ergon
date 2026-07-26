use crate::*;

pub use decoder::Message1Decoder;
pub use encoder::Message1Encoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 25;
pub const SBE_TEMPLATE_ID: u16 = 1;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct Message1Encoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for Message1Encoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for Message1Encoder<'a> {
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
                let mut composite_encoder = core::mem::take(self).header_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> Message1Encoder<'a> {
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
        pub fn header_encoder(self) -> message_header_codec::MessageHeaderEncoder<Self> {
            let offset = self.offset;
            message_header_codec::MessageHeaderEncoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn ec(&mut self, value: ec_har::EChar) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn e8(&mut self, value: eu_int_8::EUInt8) -> &mut Self {
            let offset = self.offset + 9;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn s8(&mut self, value: su_int_8::SUInt8) {
            let offset = self.offset + 10;
            self.get_buf_mut().put_u8_at(offset, value.0)
        }

        #[inline]
        pub fn s16(&mut self, value: su_int_16::SUInt16) {
            let offset = self.offset + 11;
            self.get_buf_mut().put_u16_at(offset, value.0)
        }

        #[inline]
        pub fn s32(&mut self, value: su_int_32::SUInt32) {
            let offset = self.offset + 13;
            self.get_buf_mut().put_u32_at(offset, value.0)
        }

        #[inline]
        pub fn s64(&mut self, value: su_int_64::SUInt64) {
            let offset = self.offset + 17;
            self.get_buf_mut().put_u64_at(offset, value.0)
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct Message1Decoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for Message1Decoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for Message1Decoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for Message1Decoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> Message1Decoder<'a> {
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
        pub fn header_decoder(self) -> message_header_codec::MessageHeaderDecoder<Self> {
            let offset = self.offset;
            message_header_codec::MessageHeaderDecoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn ec(&self) -> ec_har::EChar {
            self.get_buf().get_u8_at(self.offset + 8).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn e8(&self) -> eu_int_8::EUInt8 {
            self.get_buf().get_u8_at(self.offset + 9).into()
        }

        /// BIT SET DECODER
        #[inline]
        pub fn s8(&self) -> su_int_8::SUInt8 {
            su_int_8::SUInt8::new(self.get_buf().get_u8_at(self.offset + 10))
        }

        /// BIT SET DECODER
        #[inline]
        pub fn s16(&self) -> su_int_16::SUInt16 {
            su_int_16::SUInt16::new(self.get_buf().get_u16_at(self.offset + 11))
        }

        /// BIT SET DECODER
        #[inline]
        pub fn s32(&self) -> su_int_32::SUInt32 {
            su_int_32::SUInt32::new(self.get_buf().get_u32_at(self.offset + 13))
        }

        /// BIT SET DECODER
        #[inline]
        pub fn s64(&self) -> su_int_64::SUInt64 {
            su_int_64::SUInt64::new(self.get_buf().get_u64_at(self.offset + 17))
        }

    }

} // end decoder

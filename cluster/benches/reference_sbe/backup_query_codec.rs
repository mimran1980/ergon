use super::*;

pub use decoder::BackupQueryDecoder;
pub use encoder::BackupQueryEncoder;

pub use super::SBE_SCHEMA_ID;
pub use super::SBE_SCHEMA_VERSION;
pub use super::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 24;
pub const SBE_TEMPLATE_ID: u16 = 77;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct BackupQueryEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for BackupQueryEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for BackupQueryEncoder<'a> {
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
            self.version_opt(None);
            self.log_position_opt(None);
            self
        }
    }

    impl<'a> BackupQueryEncoder<'a> {
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

        /// primitive field 'correlationId'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn correlation_id(&mut self, value: i64) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'responseStreamId'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn response_stream_id(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// primitive field 'version'
        /// - min value: 1
        /// - max value: 16777215
        /// - null value: 0_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 12
        /// - encodedLength: 4
        /// - version: 2
        #[inline]
        pub fn version(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 12;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// optional primitive field 'version'
        /// - min value: 1
        /// - max value: 16777215
        /// - null value: 0_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 12
        /// - encodedLength: 4
        /// - version: 2
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn version_opt(&mut self, value: Option<i32>) -> &mut Self {
            match value {
                Some(value) => self.version(value),
                None => self.version(0_i32),
            };
            self
        }

        /// primitive field 'logPosition'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 16
        #[inline]
        pub fn log_position(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 16;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// optional primitive field 'logPosition'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 16
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn log_position_opt(&mut self, value: Option<i64>) -> &mut Self {
            match value {
                Some(value) => self.log_position(value),
                None => self.log_position(-9223372036854775808_i64),
            };
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'US-ASCII'
        #[inline]
        pub fn response_channel(&mut self, value: &[u8]) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u32::MAX - 1) as usize);
            self.set_limit(limit + 4 + data_length);
            self.get_buf_mut().put_u32_at(limit, data_length as u32);
            self.get_buf_mut().put_slice_at(limit + 4, &value[0..data_length]);
            self
        }

        /// VAR_DATA ENCODER - character encoding: 'None'
        #[inline]
        pub fn encoded_credentials(&mut self, value: &[u8]) -> &mut Self {
            let limit = self.get_limit();
            let data_length = value.len().min((u32::MAX - 1) as usize);
            self.set_limit(limit + 4 + data_length);
            self.get_buf_mut().put_u32_at(limit, data_length as u32);
            self.get_buf_mut().put_slice_at(limit + 4, &value[0..data_length]);
            self
        }
    }
} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct BackupQueryDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for BackupQueryDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for BackupQueryDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for BackupQueryDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> BackupQueryDecoder<'a> {
        pub fn wrap(mut self, buf: ReadBuf<'a>, offset: usize, acting_block_length: u16, acting_version: u16) -> Self {
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
        pub fn correlation_id(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn response_stream_id(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 8)
        }

        /// primitive field - 'OPTIONAL' { null_value: '0_i32' }
        #[inline]
        pub fn version(&self) -> Option<i32> {
            if self.acting_version() < 2 {
                return None;
            }

            let value = self.get_buf().get_i32_at(self.offset + 12);
            if value == 0_i32 { None } else { Some(value) }
        }

        /// primitive field - 'OPTIONAL' { null_value: '-9223372036854775808_i64' }
        #[inline]
        pub fn log_position(&self) -> Option<i64> {
            if self.acting_version() < 16 {
                return None;
            }

            let value = self.get_buf().get_i64_at(self.offset + 16);
            if value == -9223372036854775808_i64 {
                None
            } else {
                Some(value)
            }
        }

        /// VAR_DATA DECODER - character encoding: 'US-ASCII'
        #[inline]
        pub fn response_channel_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u32_at(offset) as usize;
            self.set_limit(offset + 4 + data_length);
            (offset + 4, data_length)
        }

        #[inline]
        pub fn response_channel_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }

        /// VAR_DATA DECODER - character encoding: 'None'
        #[inline]
        pub fn encoded_credentials_decoder(&mut self) -> (usize, usize) {
            let offset = self.get_limit();
            let data_length = self.get_buf().get_u32_at(offset) as usize;
            self.set_limit(offset + 4 + data_length);
            (offset + 4, data_length)
        }

        #[inline]
        pub fn encoded_credentials_slice(&'a self, coordinates: (usize, usize)) -> &'a [u8] {
            debug_assert!(self.get_limit() >= coordinates.0 + coordinates.1);
            self.get_buf().get_slice_at(coordinates.0, coordinates.1)
        }
    }
} // end decoder

use crate::*;

pub use decoder::NewLeadershipTermDecoder;
pub use encoder::NewLeadershipTermEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 96;
pub const SBE_TEMPLATE_ID: u16 = 53;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct NewLeadershipTermEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for NewLeadershipTermEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for NewLeadershipTermEncoder<'a> {
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
            self.app_version_opt(None);
            self
        }
    }

    impl<'a> NewLeadershipTermEncoder<'a> {
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

        /// primitive field 'logLeadershipTermId'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn log_leadership_term_id(&mut self, value: i64) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'nextLeadershipTermId'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn next_leadership_term_id(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'nextTermBaseLogPosition'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn next_term_base_log_position(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 16;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'nextLogPosition'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 24
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn next_log_position(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 24;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'leadershipTermId'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 32
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn leadership_term_id(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 32;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'termBaseLogPosition'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 40
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn term_base_log_position(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 40;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'logPosition'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 48
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn log_position(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 48;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'leaderRecordingId'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 56
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn leader_recording_id(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 56;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'timestamp'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 64
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn timestamp(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 64;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'leaderMemberId'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 72
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn leader_member_id(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 72;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// primitive field 'logSessionId'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 76
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn log_session_id(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 76;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// primitive field 'appVersion'
        /// - min value: 1
        /// - max value: 16777215
        /// - null value: 0_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 80
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn app_version(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 80;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// optional primitive field 'appVersion'
        /// - min value: 1
        /// - max value: 16777215
        /// - null value: 0_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 80
        /// - encodedLength: 4
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn app_version_opt(&mut self, value: Option<i32>) -> &mut Self {
            match value {
                Some(value) => self.app_version(value),
                None => self.app_version(0_i32),
            };
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn is_startup(&mut self, value: boolean_type::BooleanType) -> &mut Self {
            let offset = self.offset + 84;
            self.get_buf_mut().put_i32_at(offset, value as i32);
            self
        }

        /// primitive field 'commitPosition'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 88
        /// - encodedLength: 8
        /// - version: 15
        #[inline]
        pub fn commit_position(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 88;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct NewLeadershipTermDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for NewLeadershipTermDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for NewLeadershipTermDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for NewLeadershipTermDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> NewLeadershipTermDecoder<'a> {
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
        pub fn log_leadership_term_id(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn next_leadership_term_id(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 8)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn next_term_base_log_position(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 16)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn next_log_position(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 24)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn leadership_term_id(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 32)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn term_base_log_position(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 40)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn log_position(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 48)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn leader_recording_id(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 56)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn timestamp(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 64)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn leader_member_id(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 72)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn log_session_id(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 76)
        }

        /// primitive field - 'OPTIONAL' { null_value: '0_i32' }
        #[inline]
        pub fn app_version(&self) -> Option<i32> {
            let value = self.get_buf().get_i32_at(self.offset + 80);
            if value == 0_i32 {
                None
            } else {
                Some(value)
            }
        }

        /// REQUIRED enum
        #[inline]
        pub fn is_startup(&self) -> boolean_type::BooleanType {
            self.get_buf().get_i32_at(self.offset + 84).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn commit_position(&self) -> i64 {
            if self.acting_version() < 15 {
                return -9223372036854775808_i64;
            }

            self.get_buf().get_i64_at(self.offset + 88)
        }

    }

} // end decoder


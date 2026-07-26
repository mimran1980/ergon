use super::*;

pub use decoder::SnapshotMarkerDecoder;
pub use encoder::SnapshotMarkerEncoder;

pub use super::SBE_SCHEMA_ID;
pub use super::SBE_SCHEMA_VERSION;
pub use super::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 40;
pub const SBE_TEMPLATE_ID: u16 = 100;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct SnapshotMarkerEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for SnapshotMarkerEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for SnapshotMarkerEncoder<'a> {
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
            self.time_unit_opt(None);
            self.app_version_opt(None);
            self
        }
    }

    impl<'a> SnapshotMarkerEncoder<'a> {
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

        /// primitive field 'typeId'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 0
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn type_id(&mut self, value: i64) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'logPosition'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 8
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn log_position(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 8;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'leadershipTermId'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 16
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn leadership_term_id(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 16;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// primitive field 'index'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 24
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn index(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 24;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn mark(&mut self, value: snapshot_mark::SnapshotMark) -> &mut Self {
            let offset = self.offset + 28;
            self.get_buf_mut().put_i32_at(offset, value as i32);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn time_unit(&mut self, value: cluster_time_unit::ClusterTimeUnit) -> &mut Self {
            let offset = self.offset + 32;
            self.get_buf_mut().put_i32_at(offset, value as i32);
            self
        }

        /// optional enum field 'timeUnit'
        /// - min value: -2147483647
        /// - max value: 2147483647
        /// - null value: -2147483648_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 32
        /// - encodedLength: 4
        /// - version: 4
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn time_unit_opt(&mut self, value: Option<cluster_time_unit::ClusterTimeUnit>) -> &mut Self {
            match value {
                Some(value) => self.time_unit(value),
                None => self.time_unit(cluster_time_unit::ClusterTimeUnit::NullVal),
            };
            self
        }

        /// primitive field 'appVersion'
        /// - min value: 1
        /// - max value: 16777215
        /// - null value: 0_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 36
        /// - encodedLength: 4
        /// - version: 4
        #[inline]
        pub fn app_version(&mut self, value: i32) -> &mut Self {
            let offset = self.offset + 36;
            self.get_buf_mut().put_i32_at(offset, value);
            self
        }

        /// optional primitive field 'appVersion'
        /// - min value: 1
        /// - max value: 16777215
        /// - null value: 0_i32
        /// - characterEncoding: null
        /// - semanticType: null
        /// - encodedOffset: 36
        /// - encodedLength: 4
        /// - version: 4
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn app_version_opt(&mut self, value: Option<i32>) -> &mut Self {
            match value {
                Some(value) => self.app_version(value),
                None => self.app_version(0_i32),
            };
            self
        }
    }
} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct SnapshotMarkerDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for SnapshotMarkerDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for SnapshotMarkerDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for SnapshotMarkerDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> SnapshotMarkerDecoder<'a> {
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
        pub fn type_id(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn log_position(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 8)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn leadership_term_id(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 16)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn index(&self) -> i32 {
            self.get_buf().get_i32_at(self.offset + 24)
        }

        /// REQUIRED enum
        #[inline]
        pub fn mark(&self) -> snapshot_mark::SnapshotMark {
            self.get_buf().get_i32_at(self.offset + 28).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn time_unit(&self) -> cluster_time_unit::ClusterTimeUnit {
            if self.acting_version() < 4 {
                return cluster_time_unit::ClusterTimeUnit::default();
            }

            self.get_buf().get_i32_at(self.offset + 32).into()
        }

        /// primitive field - 'OPTIONAL' { null_value: '0_i32' }
        #[inline]
        pub fn app_version(&self) -> Option<i32> {
            if self.acting_version() < 4 {
                return None;
            }

            let value = self.get_buf().get_i32_at(self.offset + 36);
            if value == 0_i32 { None } else { Some(value) }
        }
    }
} // end decoder

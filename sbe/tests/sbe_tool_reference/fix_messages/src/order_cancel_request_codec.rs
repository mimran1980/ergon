use crate::*;

pub use decoder::OrderCancelRequestDecoder;
pub use encoder::OrderCancelRequestEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 119;
pub const SBE_TEMPLATE_ID: u16 = 70;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct OrderCancelRequestEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for OrderCancelRequestEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for OrderCancelRequestEncoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> OrderCancelRequestEncoder<'a> {
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

        #[inline]
        pub fn account_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'Account'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn account(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(12, value.len());
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'Account' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn account_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'Account' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn account_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(12);
            self.account_from_iter(iter);
            self
        }

        #[inline]
        pub fn cl_ord_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 12;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'ClOrdID'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 12
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn cl_ord_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 12;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'ClOrdID' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 12
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn cl_ord_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 12;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'ClOrdID' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 12
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn cl_ord_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(20);
            self.cl_ord_id_from_iter(iter);
            self
        }

        /// primitive field 'OrderID'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: int
        /// - encodedOffset: 32
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn order_id(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 32;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        #[inline]
        pub fn orig_cl_ord_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 40;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'OrigClOrdID'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 40
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn orig_cl_ord_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 40;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'OrigClOrdID' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 40
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn orig_cl_ord_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 40;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'OrigClOrdID' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 40
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn orig_cl_ord_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(20);
            self.orig_cl_ord_id_from_iter(iter);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn side(&mut self, value: side::Side) -> &mut Self {
            let offset = self.offset + 60;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn symbol_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 61;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'Symbol'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 61
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(6, value.len());
            let offset = self.offset + 61;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'Symbol' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 61
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 61;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'Symbol' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 61
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(6);
            self.symbol_from_iter(iter);
            self
        }

        /// primitive field 'TransactTime'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: UTCTimestamp
        /// - encodedOffset: 67
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn transact_time(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 67;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn manual_order_indicator(&mut self, value: boolean_type::BooleanType) -> &mut Self {
            let offset = self.offset + 75;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn security_desc_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 76;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'SecurityDesc'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 76
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 76;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'SecurityDesc' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 76
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 76;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'SecurityDesc' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 76
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(20);
            self.security_desc_from_iter(iter);
            self
        }

        #[inline]
        pub fn security_type_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 96;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'SecurityType'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 96
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(3, value.len());
            let offset = self.offset + 96;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'SecurityType' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 96
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 96;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'SecurityType' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 96
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(3);
            self.security_type_from_iter(iter);
            self
        }

        #[inline]
        pub fn correlation_cl_ord_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 99;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'CorrelationClOrdID'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 99
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn correlation_cl_ord_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 99;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'CorrelationClOrdID' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 99
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn correlation_cl_ord_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 99;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'CorrelationClOrdID' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 99
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn correlation_cl_ord_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(20);
            self.correlation_cl_ord_id_from_iter(iter);
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct OrderCancelRequestDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for OrderCancelRequestDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for OrderCancelRequestDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for OrderCancelRequestDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> OrderCancelRequestDecoder<'a> {
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

        #[inline]
        pub fn account(&self) -> [u8; 12] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset)
        }

        #[inline]
        pub fn cl_ord_id(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 12)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn order_id(&self) -> i64 {
            self.get_buf().get_i64_at(self.offset + 32)
        }

        #[inline]
        pub fn orig_cl_ord_id(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 40)
        }

        /// REQUIRED enum
        #[inline]
        pub fn side(&self) -> side::Side {
            self.get_buf().get_u8_at(self.offset + 60).into()
        }

        #[inline]
        pub fn symbol(&self) -> [u8; 6] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 61)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn transact_time(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 67)
        }

        /// REQUIRED enum
        #[inline]
        pub fn manual_order_indicator(&self) -> boolean_type::BooleanType {
            self.get_buf().get_u8_at(self.offset + 75).into()
        }

        #[inline]
        pub fn security_desc(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 76)
        }

        #[inline]
        pub fn security_type(&self) -> [u8; 3] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 96)
        }

        #[inline]
        pub fn correlation_cl_ord_id(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 99)
        }

    }

} // end decoder


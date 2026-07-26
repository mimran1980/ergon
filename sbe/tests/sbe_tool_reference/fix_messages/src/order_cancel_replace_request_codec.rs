use crate::*;

pub use decoder::OrderCancelReplaceRequestDecoder;
pub use encoder::OrderCancelReplaceRequestEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 204;
pub const SBE_TEMPLATE_ID: u16 = 71;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct OrderCancelReplaceRequestEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for OrderCancelReplaceRequestEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for OrderCancelReplaceRequestEncoder<'a> {
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
            self.cust_order_handling_inst_opt(None);
            self.time_in_force_opt(None);
            self.no_allocs_opt(None);
            {
                let mut composite_encoder = core::mem::take(self).order_qty_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).price_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).stop_px_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).min_qty_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).max_show_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a> OrderCancelReplaceRequestEncoder<'a> {
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

        /// REQUIRED enum
        #[inline]
        pub fn hand_inst(&mut self, value: hand_inst::HandInst) -> &mut Self {
            let offset = self.offset + 40;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn order_qty_encoder(self) -> int_qty_32_codec::IntQty32Encoder<Self> {
            let offset = self.offset + 41;
            int_qty_32_codec::IntQty32Encoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn cust_order_handling_inst(&mut self, value: cust_order_handling_inst::CustOrderHandlingInst) -> &mut Self {
            let offset = self.offset + 45;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'CustOrderHandlingInst'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: null
        /// - semanticType: char
        /// - encodedOffset: 45
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn cust_order_handling_inst_opt(&mut self, value: Option<cust_order_handling_inst::CustOrderHandlingInst>) -> &mut Self {
            match value {
                Some(value) => self.cust_order_handling_inst(value),
                None => self.cust_order_handling_inst(cust_order_handling_inst::CustOrderHandlingInst::NullVal),
            };
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn ord_type(&mut self, value: ord_type::OrdType) -> &mut Self {
            let offset = self.offset + 46;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn orig_cl_ord_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 47;
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
        /// - encodedOffset: 47
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn orig_cl_ord_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 47;
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
        /// - encodedOffset: 47
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn orig_cl_ord_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 47;
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
        /// - encodedOffset: 47
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn orig_cl_ord_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(20);
            self.orig_cl_ord_id_from_iter(iter);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn price_encoder(self) -> optional_price_codec::OptionalPriceEncoder<Self> {
            let offset = self.offset + 67;
            optional_price_codec::OptionalPriceEncoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn side(&mut self, value: side::Side) -> &mut Self {
            let offset = self.offset + 76;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn symbol_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 77;
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
        /// - encodedOffset: 77
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(6, value.len());
            let offset = self.offset + 77;
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
        /// - encodedOffset: 77
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 77;
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
        /// - encodedOffset: 77
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(6);
            self.symbol_from_iter(iter);
            self
        }

        #[inline]
        pub fn test_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 83;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'Test'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 83
        /// - encodedLength: 18
        /// - version: 0
        #[inline]
        pub fn test(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(18, value.len());
            let offset = self.offset + 83;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'Test' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 83
        /// - encodedLength: 18
        /// - version: 0
        #[inline]
        pub fn test_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 83;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'Test' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 83
        /// - encodedLength: 18
        /// - version: 0
        #[inline]
        pub fn test_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(18);
            self.test_from_iter(iter);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn time_in_force(&mut self, value: time_in_force::TimeInForce) -> &mut Self {
            let offset = self.offset + 101;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'TimeInForce'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: null
        /// - semanticType: char
        /// - encodedOffset: 101
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn time_in_force_opt(&mut self, value: Option<time_in_force::TimeInForce>) -> &mut Self {
            match value {
                Some(value) => self.time_in_force(value),
                None => self.time_in_force(time_in_force::TimeInForce::NullVal),
            };
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn manual_order_indicator(&mut self, value: boolean_type::BooleanType) -> &mut Self {
            let offset = self.offset + 102;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// primitive field 'TransactTime'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: UTCTimestamp
        /// - encodedOffset: 103
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn transact_time(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 103;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn no_allocs(&mut self, value: no_allocs::NoAllocs) -> &mut Self {
            let offset = self.offset + 111;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'NoAllocs'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: null
        /// - semanticType: char
        /// - encodedOffset: 111
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn no_allocs_opt(&mut self, value: Option<no_allocs::NoAllocs>) -> &mut Self {
            match value {
                Some(value) => self.no_allocs(value),
                None => self.no_allocs(no_allocs::NoAllocs::NullVal),
            };
            self
        }

        #[inline]
        pub fn alloc_account_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 112;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'AllocAccount'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 112
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn alloc_account(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(10, value.len());
            let offset = self.offset + 112;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'AllocAccount' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 112
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn alloc_account_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 112;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'AllocAccount' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 112
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn alloc_account_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(10);
            self.alloc_account_from_iter(iter);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn stop_px_encoder(self) -> optional_price_codec::OptionalPriceEncoder<Self> {
            let offset = self.offset + 122;
            optional_price_codec::OptionalPriceEncoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn security_desc_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 131;
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
        /// - encodedOffset: 131
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 131;
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
        /// - encodedOffset: 131
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 131;
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
        /// - encodedOffset: 131
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(20);
            self.security_desc_from_iter(iter);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn min_qty_encoder(self) -> int_qty_32_codec::IntQty32Encoder<Self> {
            let offset = self.offset + 151;
            int_qty_32_codec::IntQty32Encoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn security_type_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 155;
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
        /// - encodedOffset: 155
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(3, value.len());
            let offset = self.offset + 155;
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
        /// - encodedOffset: 155
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 155;
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
        /// - encodedOffset: 155
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(3);
            self.security_type_from_iter(iter);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn customer_or_firm(&mut self, value: customer_or_firm::CustomerOrFirm) -> &mut Self {
            let offset = self.offset + 158;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn max_show_encoder(self) -> int_qty_32_codec::IntQty32Encoder<Self> {
            let offset = self.offset + 159;
            int_qty_32_codec::IntQty32Encoder::default().wrap(self, offset)
        }

        /// primitive field 'ExpireDate'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: LocalMktDate
        /// - encodedOffset: 163
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn expire_date(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 163;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        #[inline]
        pub fn self_match_prevention_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 165;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'SelfMatchPreventionID'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 165
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn self_match_prevention_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(12, value.len());
            let offset = self.offset + 165;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'SelfMatchPreventionID' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 165
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn self_match_prevention_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 165;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'SelfMatchPreventionID' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 165
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn self_match_prevention_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(12);
            self.self_match_prevention_id_from_iter(iter);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn cti_code(&mut self, value: cti_code::CtiCode) -> &mut Self {
            let offset = self.offset + 177;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn give_up_firm_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 178;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'GiveUpFirm'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 178
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn give_up_firm(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(3, value.len());
            let offset = self.offset + 178;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'GiveUpFirm' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 178
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn give_up_firm_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 178;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'GiveUpFirm' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 178
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn give_up_firm_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(3);
            self.give_up_firm_from_iter(iter);
            self
        }

        #[inline]
        pub fn cmta_giveup_cd_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 181;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'CmtaGiveupCD'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 181
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn cmta_giveup_cd(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(2, value.len());
            let offset = self.offset + 181;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'CmtaGiveupCD' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 181
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn cmta_giveup_cd_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 181;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'CmtaGiveupCD' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 181
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn cmta_giveup_cd_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(2);
            self.cmta_giveup_cd_from_iter(iter);
            self
        }

        #[inline]
        pub fn correlation_cl_ord_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 183;
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
        /// - encodedOffset: 183
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn correlation_cl_ord_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 183;
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
        /// - encodedOffset: 183
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn correlation_cl_ord_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 183;
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
        /// - encodedOffset: 183
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn correlation_cl_ord_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(20);
            self.correlation_cl_ord_id_from_iter(iter);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn ofm_override(&mut self, value: ofm_override::OFMOverride) -> &mut Self {
            let offset = self.offset + 203;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct OrderCancelReplaceRequestDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for OrderCancelReplaceRequestDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for OrderCancelReplaceRequestDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for OrderCancelReplaceRequestDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> OrderCancelReplaceRequestDecoder<'a> {
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

        /// REQUIRED enum
        #[inline]
        pub fn hand_inst(&self) -> hand_inst::HandInst {
            self.get_buf().get_u8_at(self.offset + 40).into()
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn order_qty_decoder(self) -> int_qty_32_codec::IntQty32Decoder<Self> {
            let offset = self.offset + 41;
            int_qty_32_codec::IntQty32Decoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn cust_order_handling_inst(&self) -> cust_order_handling_inst::CustOrderHandlingInst {
            self.get_buf().get_u8_at(self.offset + 45).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn ord_type(&self) -> ord_type::OrdType {
            self.get_buf().get_u8_at(self.offset + 46).into()
        }

        #[inline]
        pub fn orig_cl_ord_id(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 47)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn price_decoder(self) -> optional_price_codec::OptionalPriceDecoder<Self> {
            let offset = self.offset + 67;
            optional_price_codec::OptionalPriceDecoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn side(&self) -> side::Side {
            self.get_buf().get_u8_at(self.offset + 76).into()
        }

        #[inline]
        pub fn symbol(&self) -> [u8; 6] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 77)
        }

        #[inline]
        pub fn test(&self) -> [u8; 18] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 83)
        }

        /// REQUIRED enum
        #[inline]
        pub fn time_in_force(&self) -> time_in_force::TimeInForce {
            self.get_buf().get_u8_at(self.offset + 101).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn manual_order_indicator(&self) -> boolean_type::BooleanType {
            self.get_buf().get_u8_at(self.offset + 102).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn transact_time(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 103)
        }

        /// REQUIRED enum
        #[inline]
        pub fn no_allocs(&self) -> no_allocs::NoAllocs {
            self.get_buf().get_u8_at(self.offset + 111).into()
        }

        #[inline]
        pub fn alloc_account(&self) -> [u8; 10] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 112)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn stop_px_decoder(self) -> optional_price_codec::OptionalPriceDecoder<Self> {
            let offset = self.offset + 122;
            optional_price_codec::OptionalPriceDecoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn security_desc(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 131)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn min_qty_decoder(self) -> int_qty_32_codec::IntQty32Decoder<Self> {
            let offset = self.offset + 151;
            int_qty_32_codec::IntQty32Decoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn security_type(&self) -> [u8; 3] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 155)
        }

        /// REQUIRED enum
        #[inline]
        pub fn customer_or_firm(&self) -> customer_or_firm::CustomerOrFirm {
            self.get_buf().get_u8_at(self.offset + 158).into()
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn max_show_decoder(self) -> int_qty_32_codec::IntQty32Decoder<Self> {
            let offset = self.offset + 159;
            int_qty_32_codec::IntQty32Decoder::default().wrap(self, offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn expire_date(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 163)
        }

        #[inline]
        pub fn self_match_prevention_id(&self) -> [u8; 12] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 165)
        }

        /// REQUIRED enum
        #[inline]
        pub fn cti_code(&self) -> cti_code::CtiCode {
            self.get_buf().get_u8_at(self.offset + 177).into()
        }

        #[inline]
        pub fn give_up_firm(&self) -> [u8; 3] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 178)
        }

        #[inline]
        pub fn cmta_giveup_cd(&self) -> [u8; 2] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 181)
        }

        #[inline]
        pub fn correlation_cl_ord_id(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 183)
        }

        /// REQUIRED enum
        #[inline]
        pub fn ofm_override(&self) -> ofm_override::OFMOverride {
            self.get_buf().get_u8_at(self.offset + 203).into()
        }

    }

} // end decoder

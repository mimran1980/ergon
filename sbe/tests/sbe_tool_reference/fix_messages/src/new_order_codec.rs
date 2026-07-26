use crate::*;

pub use decoder::NewOrderDecoder;
pub use encoder::NewOrderEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 156;
pub const SBE_TEMPLATE_ID: u16 = 68;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct NewOrderEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for NewOrderEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for NewOrderEncoder<'a> {
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

    impl<'a> NewOrderEncoder<'a> {
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

        /// REQUIRED enum
        #[inline]
        pub fn hand_inst(&mut self, value: hand_inst::HandInst) -> &mut Self {
            let offset = self.offset + 32;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn cust_order_handling_inst(&mut self, value: cust_order_handling_inst::CustOrderHandlingInst) -> &mut Self {
            let offset = self.offset + 33;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'CustOrderHandlingInst'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: null
        /// - semanticType: char
        /// - encodedOffset: 33
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

        /// COMPOSITE ENCODER
        #[inline]
        pub fn order_qty_encoder(self) -> int_qty_32_codec::IntQty32Encoder<Self> {
            let offset = self.offset + 34;
            int_qty_32_codec::IntQty32Encoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn ord_type(&mut self, value: ord_type::OrdType) -> &mut Self {
            let offset = self.offset + 38;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn price_encoder(self) -> optional_price_codec::OptionalPriceEncoder<Self> {
            let offset = self.offset + 39;
            optional_price_codec::OptionalPriceEncoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn side(&mut self, value: side::Side) -> &mut Self {
            let offset = self.offset + 48;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn symbol_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 49;
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
        /// - encodedOffset: 49
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(6, value.len());
            let offset = self.offset + 49;
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
        /// - encodedOffset: 49
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 49;
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
        /// - encodedOffset: 49
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(6);
            self.symbol_from_iter(iter);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn time_in_force(&mut self, value: time_in_force::TimeInForce) -> &mut Self {
            let offset = self.offset + 55;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'TimeInForce'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: null
        /// - semanticType: char
        /// - encodedOffset: 55
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

        /// primitive field 'TransactTime'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: UTCTimestamp
        /// - encodedOffset: 56
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn transact_time(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 56;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn manual_order_indicator(&mut self, value: boolean_type::BooleanType) -> &mut Self {
            let offset = self.offset + 64;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn alloc_account_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 65;
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
        /// - encodedOffset: 65
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn alloc_account(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(10, value.len());
            let offset = self.offset + 65;
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
        /// - encodedOffset: 65
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn alloc_account_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 65;
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
        /// - encodedOffset: 65
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
            let offset = self.offset + 75;
            optional_price_codec::OptionalPriceEncoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn security_desc_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 84;
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
        /// - encodedOffset: 84
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 84;
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
        /// - encodedOffset: 84
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 84;
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
        /// - encodedOffset: 84
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
            let offset = self.offset + 104;
            int_qty_32_codec::IntQty32Encoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn security_type_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 108;
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
        /// - encodedOffset: 108
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(3, value.len());
            let offset = self.offset + 108;
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
        /// - encodedOffset: 108
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 108;
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
        /// - encodedOffset: 108
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
            let offset = self.offset + 111;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn max_show_encoder(self) -> int_qty_32_codec::IntQty32Encoder<Self> {
            let offset = self.offset + 112;
            int_qty_32_codec::IntQty32Encoder::default().wrap(self, offset)
        }

        /// primitive field 'ExpireDate'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: LocalMktDate
        /// - encodedOffset: 116
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn expire_date(&mut self, value: u16) -> &mut Self {
            let offset = self.offset + 116;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        #[inline]
        pub fn self_match_prevention_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 118;
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
        /// - encodedOffset: 118
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn self_match_prevention_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(12, value.len());
            let offset = self.offset + 118;
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
        /// - encodedOffset: 118
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn self_match_prevention_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 118;
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
        /// - encodedOffset: 118
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
            let offset = self.offset + 130;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn give_up_firm_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 131;
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
        /// - encodedOffset: 131
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn give_up_firm(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(3, value.len());
            let offset = self.offset + 131;
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
        /// - encodedOffset: 131
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn give_up_firm_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 131;
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
        /// - encodedOffset: 131
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
            let offset = self.offset + 134;
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
        /// - encodedOffset: 134
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn cmta_giveup_cd(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(2, value.len());
            let offset = self.offset + 134;
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
        /// - encodedOffset: 134
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn cmta_giveup_cd_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 134;
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
        /// - encodedOffset: 134
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
            let offset = self.offset + 136;
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
        /// - encodedOffset: 136
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn correlation_cl_ord_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 136;
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
        /// - encodedOffset: 136
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn correlation_cl_ord_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 136;
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
        /// - encodedOffset: 136
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
    pub struct NewOrderDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for NewOrderDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for NewOrderDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for NewOrderDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> NewOrderDecoder<'a> {
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

        /// REQUIRED enum
        #[inline]
        pub fn hand_inst(&self) -> hand_inst::HandInst {
            self.get_buf().get_u8_at(self.offset + 32).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn cust_order_handling_inst(&self) -> cust_order_handling_inst::CustOrderHandlingInst {
            self.get_buf().get_u8_at(self.offset + 33).into()
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn order_qty_decoder(self) -> int_qty_32_codec::IntQty32Decoder<Self> {
            let offset = self.offset + 34;
            int_qty_32_codec::IntQty32Decoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn ord_type(&self) -> ord_type::OrdType {
            self.get_buf().get_u8_at(self.offset + 38).into()
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn price_decoder(self) -> optional_price_codec::OptionalPriceDecoder<Self> {
            let offset = self.offset + 39;
            optional_price_codec::OptionalPriceDecoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn side(&self) -> side::Side {
            self.get_buf().get_u8_at(self.offset + 48).into()
        }

        #[inline]
        pub fn symbol(&self) -> [u8; 6] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 49)
        }

        /// REQUIRED enum
        #[inline]
        pub fn time_in_force(&self) -> time_in_force::TimeInForce {
            self.get_buf().get_u8_at(self.offset + 55).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn transact_time(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 56)
        }

        /// REQUIRED enum
        #[inline]
        pub fn manual_order_indicator(&self) -> boolean_type::BooleanType {
            self.get_buf().get_u8_at(self.offset + 64).into()
        }

        #[inline]
        pub fn alloc_account(&self) -> [u8; 10] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 65)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn stop_px_decoder(self) -> optional_price_codec::OptionalPriceDecoder<Self> {
            let offset = self.offset + 75;
            optional_price_codec::OptionalPriceDecoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn security_desc(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 84)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn min_qty_decoder(self) -> int_qty_32_codec::IntQty32Decoder<Self> {
            let offset = self.offset + 104;
            int_qty_32_codec::IntQty32Decoder::default().wrap(self, offset)
        }

        #[inline]
        pub fn security_type(&self) -> [u8; 3] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 108)
        }

        /// REQUIRED enum
        #[inline]
        pub fn customer_or_firm(&self) -> customer_or_firm::CustomerOrFirm {
            self.get_buf().get_u8_at(self.offset + 111).into()
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn max_show_decoder(self) -> int_qty_32_codec::IntQty32Decoder<Self> {
            let offset = self.offset + 112;
            int_qty_32_codec::IntQty32Decoder::default().wrap(self, offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn expire_date(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset + 116)
        }

        #[inline]
        pub fn self_match_prevention_id(&self) -> [u8; 12] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 118)
        }

        /// REQUIRED enum
        #[inline]
        pub fn cti_code(&self) -> cti_code::CtiCode {
            self.get_buf().get_u8_at(self.offset + 130).into()
        }

        #[inline]
        pub fn give_up_firm(&self) -> [u8; 3] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 131)
        }

        #[inline]
        pub fn cmta_giveup_cd(&self) -> [u8; 2] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 134)
        }

        #[inline]
        pub fn correlation_cl_ord_id(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 136)
        }

    }

} // end decoder


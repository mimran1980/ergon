use crate::*;

pub use decoder::MassQuoteDecoder;
pub use encoder::MassQuoteEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 62;
pub const SBE_TEMPLATE_ID: u16 = 105;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct MassQuoteEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for MassQuoteEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for MassQuoteEncoder<'a> {
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
            self.mm_protection_reset_opt(None);
            self
        }
    }

    impl<'a> MassQuoteEncoder<'a> {
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
        pub fn quote_req_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'QuoteReqID'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 23
        /// - version: 0
        #[inline]
        pub fn quote_req_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(23, value.len());
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'QuoteReqID' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 23
        /// - version: 0
        #[inline]
        pub fn quote_req_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'QuoteReqID' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 23
        /// - version: 0
        #[inline]
        pub fn quote_req_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(23);
            self.quote_req_id_from_iter(iter);
            self
        }

        #[inline]
        pub fn quote_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 23;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'QuoteID'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 23
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn quote_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(10, value.len());
            let offset = self.offset + 23;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'QuoteID' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 23
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn quote_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 23;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'QuoteID' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 23
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn quote_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(10);
            self.quote_id_from_iter(iter);
            self
        }

        #[inline]
        pub fn mm_account_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'MMAccount'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn mm_account(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(12, value.len());
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'MMAccount' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn mm_account_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 33;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'MMAccount' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 33
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn mm_account_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(12);
            self.mm_account_from_iter(iter);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn manual_order_indicator(&mut self, value: boolean_type::BooleanType) -> &mut Self {
            let offset = self.offset + 45;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn cust_order_handling_inst(&mut self, value: cust_order_handling_inst::CustOrderHandlingInst) -> &mut Self {
            let offset = self.offset + 46;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'CustOrderHandlingInst'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: null
        /// - semanticType: char
        /// - encodedOffset: 46
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
        pub fn customer_or_firm(&mut self, value: customer_or_firm::CustomerOrFirm) -> &mut Self {
            let offset = self.offset + 47;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        #[inline]
        pub fn self_match_prevention_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 48;
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
        /// - encodedOffset: 48
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn self_match_prevention_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(12, value.len());
            let offset = self.offset + 48;
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
        /// - encodedOffset: 48
        /// - encodedLength: 12
        /// - version: 0
        #[inline]
        pub fn self_match_prevention_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 48;
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
        /// - encodedOffset: 48
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
            let offset = self.offset + 60;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn mm_protection_reset(&mut self, value: mm_protection_reset::MMProtectionReset) -> &mut Self {
            let offset = self.offset + 61;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'MMProtectionReset'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: null
        /// - semanticType: char
        /// - encodedOffset: 61
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn mm_protection_reset_opt(&mut self, value: Option<mm_protection_reset::MMProtectionReset>) -> &mut Self {
            match value {
                Some(value) => self.mm_protection_reset(value),
                None => self.mm_protection_reset(mm_protection_reset::MMProtectionReset::NullVal),
            };
            self
        }

        /// GROUP ENCODER (id=296)
        #[inline]
        pub fn quote_sets_encoder(self, count: u16, quote_sets_encoder: QuoteSetsEncoder<Self>) -> QuoteSetsEncoder<Self> {
            quote_sets_encoder.wrap(self, count)
        }

    }

    #[derive(Debug, Default)]
    pub struct QuoteSetsEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for QuoteSetsEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for QuoteSetsEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> QuoteSetsEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        pub fn wrap(
            mut self,
            mut parent: P,
            count: u16,
        ) -> Self {
            let initial_limit = parent.get_limit();
            parent.set_limit(initial_limit + 4);
            parent.get_buf_mut().put_u16_at(initial_limit, Self::block_length());
            parent.get_buf_mut().put_u16_at(initial_limit + 2, count);
            self.parent = Some(parent);
            self.count = count;
            self.index = usize::MAX;
            self.offset = usize::MAX;
            self.initial_limit = initial_limit;
            self
        }

        #[inline]
        pub const fn block_length() -> u16 {
            24
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// will return Some(current index) when successful otherwise None
        #[inline]
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + Self::block_length() as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        #[inline]
        pub fn quote_set_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'QuoteSetID'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn quote_set_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(3, value.len());
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'QuoteSetID' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn quote_set_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'QuoteSetID' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn quote_set_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(3);
            self.quote_set_id_from_iter(iter);
            self
        }

        #[inline]
        pub fn underlying_security_desc_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 3;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'UnderlyingSecurityDesc'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 3
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn underlying_security_desc(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 3;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'UnderlyingSecurityDesc' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 3
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn underlying_security_desc_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 3;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'UnderlyingSecurityDesc' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 3
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn underlying_security_desc_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(20);
            self.underlying_security_desc_from_iter(iter);
            self
        }

        /// primitive field 'TotQuoteEntries'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: int
        /// - encodedOffset: 23
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn tot_quote_entries(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 23;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// GROUP ENCODER (id=295)
        #[inline]
        pub fn quote_entries_encoder(self, count: u16, quote_entries_encoder: QuoteEntriesEncoder<Self>) -> QuoteEntriesEncoder<Self> {
            quote_entries_encoder.wrap(self, count)
        }

    }

    #[derive(Debug, Default)]
    pub struct QuoteEntriesEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for QuoteEntriesEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for QuoteEntriesEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }

        /// Set all optional fields to their 'null' values.
        #[inline]
        fn nullify_optional_fields(&mut self) -> &mut Self {
            self.security_id_opt(None);
            self.security_id_source_opt(None);
            self.bid_size_opt(None);
            self.offer_size_opt(None);
            {
                let mut composite_encoder = core::mem::take(self).bid_px_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).offer_px_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a, P> QuoteEntriesEncoder<P> where P: Encoder<'a> + Default {
        #[inline]
        pub fn wrap(
            mut self,
            mut parent: P,
            count: u16,
        ) -> Self {
            let initial_limit = parent.get_limit();
            parent.set_limit(initial_limit + 4);
            parent.get_buf_mut().put_u16_at(initial_limit, Self::block_length());
            parent.get_buf_mut().put_u16_at(initial_limit + 2, count);
            self.parent = Some(parent);
            self.count = count;
            self.index = usize::MAX;
            self.offset = usize::MAX;
            self.initial_limit = initial_limit;
            self
        }

        #[inline]
        pub const fn block_length() -> u16 {
            90
        }

        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        /// will return Some(current index) when successful otherwise None
        #[inline]
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + Self::block_length() as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        #[inline]
        pub fn quote_entry_id_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_u8_at(offset + index, value);
            self
        }

        /// primitive array field 'QuoteEntryID'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn quote_entry_id(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(10, value.len());
            let offset = self.offset;
            let buf = self.get_buf_mut();
            buf.put_slice_at(offset, value);
            self
        }

        /// primitive array field 'QuoteEntryID' from an Iterator
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn quote_entry_id_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset;
            let buf = self.get_buf_mut();
            for (i, v) in iter.enumerate() {
                buf.put_u8_at(offset + i, v);
            }
            self
        }

        /// primitive array field 'QuoteEntryID' with zero padding
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: US-ASCII
        /// - semanticType: String
        /// - encodedOffset: 0
        /// - encodedLength: 10
        /// - version: 0
        #[inline]
        pub fn quote_entry_id_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(10);
            self.quote_entry_id_from_iter(iter);
            self
        }

        #[inline]
        pub fn symbol_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 10;
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
        /// - encodedOffset: 10
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(6, value.len());
            let offset = self.offset + 10;
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
        /// - encodedOffset: 10
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 10;
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
        /// - encodedOffset: 10
        /// - encodedLength: 6
        /// - version: 0
        #[inline]
        pub fn symbol_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(6);
            self.symbol_from_iter(iter);
            self
        }

        #[inline]
        pub fn security_desc_at(&mut self, index: usize, value: u8) -> &mut Self {
            let offset = self.offset + 16;
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
        /// - encodedOffset: 16
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(20, value.len());
            let offset = self.offset + 16;
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
        /// - encodedOffset: 16
        /// - encodedLength: 20
        /// - version: 0
        #[inline]
        pub fn security_desc_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 16;
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
        /// - encodedOffset: 16
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
            let offset = self.offset + 36;
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
        /// - encodedOffset: 36
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type(&mut self, value: &[u8]) -> &mut Self {
            debug_assert_eq!(3, value.len());
            let offset = self.offset + 36;
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
        /// - encodedOffset: 36
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type_from_iter(&mut self, iter: impl Iterator<Item = u8>) -> &mut Self {
            let offset = self.offset + 36;
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
        /// - encodedOffset: 36
        /// - encodedLength: 3
        /// - version: 0
        #[inline]
        pub fn security_type_zero_padded(&mut self, value: &[u8]) -> &mut Self {
            let iter = value.iter().copied().chain(std::iter::repeat(0_u8)).take(3);
            self.security_type_from_iter(iter);
            self
        }

        /// primitive field 'SecurityID'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: int
        /// - encodedOffset: 39
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn security_id(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 39;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// optional primitive field 'SecurityID'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: int
        /// - encodedOffset: 39
        /// - encodedLength: 8
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn security_id_opt(&mut self, value: Option<i64>) -> &mut Self {
            match value {
                Some(value) => self.security_id(value),
                None => self.security_id(-9223372036854775808_i64),
            };
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn security_id_source(&mut self, value: security_id_source::SecurityIDSource) -> &mut Self {
            let offset = self.offset + 47;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// optional enum field 'SecurityIDSource'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: null
        /// - semanticType: char
        /// - encodedOffset: 47
        /// - encodedLength: 1
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn security_id_source_opt(&mut self, value: Option<security_id_source::SecurityIDSource>) -> &mut Self {
            match value {
                Some(value) => self.security_id_source(value),
                None => self.security_id_source(security_id_source::SecurityIDSource::NullVal),
            };
            self
        }

        /// primitive field 'TransactTime'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: UTCTimestamp
        /// - encodedOffset: 48
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn transact_time(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 48;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn bid_px_encoder(self) -> optional_price_codec::OptionalPriceEncoder<Self> {
            let offset = self.offset + 56;
            optional_price_codec::OptionalPriceEncoder::default().wrap(self, offset)
        }

        /// primitive field 'BidSize'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: int
        /// - encodedOffset: 65
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn bid_size(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 65;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// optional primitive field 'BidSize'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: int
        /// - encodedOffset: 65
        /// - encodedLength: 8
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn bid_size_opt(&mut self, value: Option<i64>) -> &mut Self {
            match value {
                Some(value) => self.bid_size(value),
                None => self.bid_size(-9223372036854775808_i64),
            };
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn offer_px_encoder(self) -> optional_price_codec::OptionalPriceEncoder<Self> {
            let offset = self.offset + 73;
            optional_price_codec::OptionalPriceEncoder::default().wrap(self, offset)
        }

        /// primitive field 'OfferSize'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: int
        /// - encodedOffset: 82
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn offer_size(&mut self, value: i64) -> &mut Self {
            let offset = self.offset + 82;
            self.get_buf_mut().put_i64_at(offset, value);
            self
        }

        /// optional primitive field 'OfferSize'
        /// - min value: -9223372036854775807
        /// - max value: 9223372036854775807
        /// - null value: -9223372036854775808_i64
        /// - characterEncoding: null
        /// - semanticType: int
        /// - encodedOffset: 82
        /// - encodedLength: 8
        /// - version: 0
        /// Set to `None` to encode the field null value.
        #[inline]
        pub fn offer_size_opt(&mut self, value: Option<i64>) -> &mut Self {
            match value {
                Some(value) => self.offer_size(value),
                None => self.offer_size(-9223372036854775808_i64),
            };
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct MassQuoteDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for MassQuoteDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for MassQuoteDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for MassQuoteDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> MassQuoteDecoder<'a> {
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
        pub fn quote_req_id(&self) -> [u8; 23] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset)
        }

        #[inline]
        pub fn quote_id(&self) -> [u8; 10] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 23)
        }

        #[inline]
        pub fn mm_account(&self) -> [u8; 12] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 33)
        }

        /// REQUIRED enum
        #[inline]
        pub fn manual_order_indicator(&self) -> boolean_type::BooleanType {
            self.get_buf().get_u8_at(self.offset + 45).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn cust_order_handling_inst(&self) -> cust_order_handling_inst::CustOrderHandlingInst {
            self.get_buf().get_u8_at(self.offset + 46).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn customer_or_firm(&self) -> customer_or_firm::CustomerOrFirm {
            self.get_buf().get_u8_at(self.offset + 47).into()
        }

        #[inline]
        pub fn self_match_prevention_id(&self) -> [u8; 12] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 48)
        }

        /// REQUIRED enum
        #[inline]
        pub fn cti_code(&self) -> cti_code::CtiCode {
            self.get_buf().get_u8_at(self.offset + 60).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn mm_protection_reset(&self) -> mm_protection_reset::MMProtectionReset {
            self.get_buf().get_u8_at(self.offset + 61).into()
        }

        /// GROUP DECODER (id=296)
        #[inline]
        pub fn quote_sets_decoder(self) -> QuoteSetsDecoder<Self> {
            QuoteSetsDecoder::default().wrap(self)
        }

    }

    #[derive(Debug, Default)]
    pub struct QuoteSetsDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for QuoteSetsDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for QuoteSetsDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for QuoteSetsDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> QuoteSetsDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        pub fn wrap(
            mut self,
            mut parent: P,
        ) -> Self {
            let initial_offset = parent.get_limit();
            let block_length = parent.get_buf().get_u16_at(initial_offset);
            let count = parent.get_buf().get_u16_at(initial_offset + 2);
            parent.set_limit(initial_offset + 4);
            self.parent = Some(parent);
            self.block_length = block_length;
            self.count = count;
            self.index = usize::MAX;
            self.offset = 0;
            self
        }

        /// group token - Token{signal=BEGIN_GROUP, name='QuoteSets', referencedName='null', description='null', packageName='null', id=296, version=0, deprecated=0, encodedLength=24, offset=62, componentTokenCount=62, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        #[inline]
        pub fn acting_version(&mut self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }

        #[inline]
        pub fn count(&self) -> u16 {
            self.count
        }

        /// will return Some(current index) when successful otherwise None
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                 return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + self.block_length as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        #[inline]
        pub fn quote_set_id(&self) -> [u8; 3] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset)
        }

        #[inline]
        pub fn underlying_security_desc(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 3)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn tot_quote_entries(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 23)
        }

        /// GROUP DECODER (id=295)
        #[inline]
        pub fn quote_entries_decoder(self) -> QuoteEntriesDecoder<Self> {
            QuoteEntriesDecoder::default().wrap(self)
        }

    }

    #[derive(Debug, Default)]
    pub struct QuoteEntriesDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for QuoteEntriesDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for QuoteEntriesDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for QuoteEntriesDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> QuoteEntriesDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        pub fn wrap(
            mut self,
            mut parent: P,
        ) -> Self {
            let initial_offset = parent.get_limit();
            let block_length = parent.get_buf().get_u16_at(initial_offset);
            let count = parent.get_buf().get_u16_at(initial_offset + 2);
            parent.set_limit(initial_offset + 4);
            self.parent = Some(parent);
            self.block_length = block_length;
            self.count = count;
            self.index = usize::MAX;
            self.offset = 0;
            self
        }

        /// group token - Token{signal=BEGIN_GROUP, name='QuoteEntries', referencedName='null', description='null', packageName='null', id=295, version=0, deprecated=0, encodedLength=90, offset=24, componentTokenCount=47, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
        #[inline]
        pub fn parent(&mut self) -> SbeResult<P> {
            self.parent.take().ok_or(SbeErr::ParentNotSet)
        }

        #[inline]
        pub fn acting_version(&mut self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }

        #[inline]
        pub fn count(&self) -> u16 {
            self.count
        }

        /// will return Some(current index) when successful otherwise None
        pub fn advance(&mut self) -> SbeResult<Option<usize>> {
            let index = self.index.wrapping_add(1);
            if index >= self.count as usize {
                 return Ok(None);
            }
            if let Some(parent) = self.parent.as_mut() {
                self.offset = parent.get_limit();
                parent.set_limit(self.offset + self.block_length as usize);
                self.index = index;
                Ok(Some(index))
            } else {
                Err(SbeErr::ParentNotSet)
            }
        }

        #[inline]
        pub fn quote_entry_id(&self) -> [u8; 10] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset)
        }

        #[inline]
        pub fn symbol(&self) -> [u8; 6] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 10)
        }

        #[inline]
        pub fn security_desc(&self) -> [u8; 20] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 16)
        }

        #[inline]
        pub fn security_type(&self) -> [u8; 3] {
            let buf = self.get_buf();
            ReadBuf::get_bytes_at(buf.data, self.offset + 36)
        }

        /// primitive field - 'OPTIONAL' { null_value: '-9223372036854775808_i64' }
        #[inline]
        pub fn security_id(&self) -> Option<i64> {
            let value = self.get_buf().get_i64_at(self.offset + 39);
            if value == -9223372036854775808_i64 {
                None
            } else {
                Some(value)
            }
        }

        /// REQUIRED enum
        #[inline]
        pub fn security_id_source(&self) -> security_id_source::SecurityIDSource {
            self.get_buf().get_u8_at(self.offset + 47).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn transact_time(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 48)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn bid_px_decoder(self) -> optional_price_codec::OptionalPriceDecoder<Self> {
            let offset = self.offset + 56;
            optional_price_codec::OptionalPriceDecoder::default().wrap(self, offset)
        }

        /// primitive field - 'OPTIONAL' { null_value: '-9223372036854775808_i64' }
        #[inline]
        pub fn bid_size(&self) -> Option<i64> {
            let value = self.get_buf().get_i64_at(self.offset + 65);
            if value == -9223372036854775808_i64 {
                None
            } else {
                Some(value)
            }
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn offer_px_decoder(self) -> optional_price_codec::OptionalPriceDecoder<Self> {
            let offset = self.offset + 73;
            optional_price_codec::OptionalPriceDecoder::default().wrap(self, offset)
        }

        /// primitive field - 'OPTIONAL' { null_value: '-9223372036854775808_i64' }
        #[inline]
        pub fn offer_size(&self) -> Option<i64> {
            let value = self.get_buf().get_i64_at(self.offset + 82);
            if value == -9223372036854775808_i64 {
                None
            } else {
                Some(value)
            }
        }

    }

} // end decoder

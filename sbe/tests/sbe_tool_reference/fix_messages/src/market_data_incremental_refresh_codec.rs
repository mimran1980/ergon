use crate::*;

pub use decoder::MarketDataIncrementalRefreshDecoder;
pub use encoder::MarketDataIncrementalRefreshEncoder;

pub use crate::SBE_SCHEMA_ID;
pub use crate::SBE_SCHEMA_VERSION;
pub use crate::SBE_SEMANTIC_VERSION;

pub const SBE_BLOCK_LENGTH: u16 = 2;
pub const SBE_TEMPLATE_ID: u16 = 88;

pub mod encoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Debug, Default)]
    pub struct MarketDataIncrementalRefreshEncoder<'a> {
        buf: WriteBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
    }

    impl<'a> Writer<'a> for MarketDataIncrementalRefreshEncoder<'a> {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            &mut self.buf
        }
    }

    impl<'a> Encoder<'a> for MarketDataIncrementalRefreshEncoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> MarketDataIncrementalRefreshEncoder<'a> {
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

        /// primitive field 'TradeDate'
        /// - min value: 0
        /// - max value: 65534
        /// - null value: 0xffff_u16
        /// - characterEncoding: null
        /// - semanticType: LocalMktDate
        /// - encodedOffset: 0
        /// - encodedLength: 2
        /// - version: 0
        #[inline]
        pub fn trade_date(&mut self, value: u16) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u16_at(offset, value);
            self
        }

        /// GROUP ENCODER (id=268)
        #[inline]
        pub fn entries_encoder(self, count: u16, entries_encoder: EntriesEncoder<Self>) -> EntriesEncoder<Self> {
            entries_encoder.wrap(self, count)
        }

    }

    #[derive(Debug, Default)]
    pub struct EntriesEncoder<P> {
        parent: Option<P>,
        count: u16,
        index: usize,
        offset: usize,
        initial_limit: usize,
    }

    impl<'a, P> Writer<'a> for EntriesEncoder<P> where P: Writer<'a> + Default {
        #[inline]
        fn get_buf_mut(&mut self) -> &mut WriteBuf<'a> {
            if let Some(parent) = self.parent.as_mut() {
                parent.get_buf_mut()
            } else {
                panic!("parent was None")
            }
        }
    }

    impl<'a, P> Encoder<'a> for EntriesEncoder<P> where P: Encoder<'a> + Default {
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
            {
                let mut composite_encoder = core::mem::take(self).md_entry_px_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).md_entry_size_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).net_chg_prev_day_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            {
                let mut composite_encoder = core::mem::take(self).trade_volume_encoder();
                composite_encoder.nullify_optional_fields();
                *self = composite_encoder.parent().expect("parent missing");
            }
            self
        }
    }

    impl<'a, P> EntriesEncoder<P> where P: Encoder<'a> + Default {
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
            82
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

        /// REQUIRED enum
        #[inline]
        pub fn md_update_action(&mut self, value: md_update_action::MDUpdateAction) -> &mut Self {
            let offset = self.offset;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// primitive field 'MdPriceLevel'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: MDPriceLevel
        /// - encodedOffset: 1
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn md_price_level(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 1;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn md_entry_type(&mut self, value: md_entry_type::MDEntryType) -> &mut Self {
            let offset = self.offset + 2;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// primitive field 'SecurityIdSource'
        /// - min value: 32
        /// - max value: 126
        /// - null value: 0_u8
        /// - characterEncoding: null
        /// - semanticType: SecurityID
        /// - encodedOffset: 3
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn security_id_source(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 3;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        /// primitive field 'SecurityId'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: InstrumentID
        /// - encodedOffset: 4
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn security_id(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 4;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// primitive field 'RptSeq'
        /// - min value: 0
        /// - max value: 254
        /// - null value: 0xff_u8
        /// - characterEncoding: null
        /// - semanticType: SequenceNumber
        /// - encodedOffset: 12
        /// - encodedLength: 1
        /// - version: 0
        #[inline]
        pub fn rpt_seq(&mut self, value: u8) -> &mut Self {
            let offset = self.offset + 12;
            self.get_buf_mut().put_u8_at(offset, value);
            self
        }

        #[inline]
        pub fn quote_condition(&mut self, value: quote_condition::QuoteCondition) {
            let offset = self.offset + 13;
            self.get_buf_mut().put_u8_at(offset, value.0)
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn md_entry_px_encoder(self) -> decimal_64_codec::Decimal64Encoder<Self> {
            let offset = self.offset + 14;
            decimal_64_codec::Decimal64Encoder::default().wrap(self, offset)
        }

        /// primitive field 'NumberOfOrders'
        /// - min value: 0
        /// - max value: 4294967294
        /// - null value: 0xffffffff_u32
        /// - characterEncoding: null
        /// - semanticType: NumberOfOrders
        /// - encodedOffset: 22
        /// - encodedLength: 4
        /// - version: 0
        #[inline]
        pub fn number_of_orders(&mut self, value: u32) -> &mut Self {
            let offset = self.offset + 22;
            self.get_buf_mut().put_u32_at(offset, value);
            self
        }

        /// primitive field 'MdEntryTime'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: UTCTimestamp
        /// - encodedOffset: 26
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn md_entry_time(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 26;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn md_entry_size_encoder(self) -> int_qty_32_codec::IntQty32Encoder<Self> {
            let offset = self.offset + 34;
            int_qty_32_codec::IntQty32Encoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn trading_session_id(&mut self, value: market_state_identifier::MarketStateIdentifier) -> &mut Self {
            let offset = self.offset + 38;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn net_chg_prev_day_encoder(self) -> decimal_64_codec::Decimal64Encoder<Self> {
            let offset = self.offset + 39;
            decimal_64_codec::Decimal64Encoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn tick_direction(&mut self, value: tick_direction::TickDirection) -> &mut Self {
            let offset = self.offset + 47;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn open_close_settle_flag(&mut self, value: open_close_settle_flag::OpenCloseSettleFlag) -> &mut Self {
            let offset = self.offset + 48;
            self.get_buf_mut().put_u16_at(offset, value as u16);
            self
        }

        /// primitive field 'SettleDate'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: UTCTimestamp
        /// - encodedOffset: 50
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn settle_date(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 50;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        #[inline]
        pub fn trade_condition(&mut self, value: trade_condition::TradeCondition) {
            let offset = self.offset + 58;
            self.get_buf_mut().put_u8_at(offset, value.0)
        }

        /// COMPOSITE ENCODER
        #[inline]
        pub fn trade_volume_encoder(self) -> int_qty_32_codec::IntQty32Encoder<Self> {
            let offset = self.offset + 59;
            int_qty_32_codec::IntQty32Encoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn md_quote_type(&mut self, value: md_quote_type::MDQuoteType) -> &mut Self {
            let offset = self.offset + 63;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// primitive field 'FixingBracket'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: UTCTimestamp
        /// - encodedOffset: 64
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn fixing_bracket(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 64;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn aggressor_side(&mut self, value: side::Side) -> &mut Self {
            let offset = self.offset + 72;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// REQUIRED enum
        #[inline]
        pub fn match_event_indicator(&mut self, value: match_event_indicator::MatchEventIndicator) -> &mut Self {
            let offset = self.offset + 73;
            self.get_buf_mut().put_u8_at(offset, value as u8);
            self
        }

        /// primitive field 'TradeId'
        /// - min value: 0
        /// - max value: -2
        /// - null value: 0xffffffffffffffff_u64
        /// - characterEncoding: null
        /// - semanticType: ExecID
        /// - encodedOffset: 74
        /// - encodedLength: 8
        /// - version: 0
        #[inline]
        pub fn trade_id(&mut self, value: u64) -> &mut Self {
            let offset = self.offset + 74;
            self.get_buf_mut().put_u64_at(offset, value);
            self
        }

    }

} // end encoder

pub mod decoder {
    use super::*;
    use message_header_codec::*;

    #[derive(Clone, Copy, Debug, Default)]
    pub struct MarketDataIncrementalRefreshDecoder<'a> {
        buf: ReadBuf<'a>,
        initial_offset: usize,
        offset: usize,
        limit: usize,
        pub acting_block_length: u16,
        pub acting_version: u16,
    }

    impl ActingVersion for MarketDataIncrementalRefreshDecoder<'_> {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.acting_version
        }
    }

    impl<'a> Reader<'a> for MarketDataIncrementalRefreshDecoder<'a> {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            &self.buf
        }
    }

    impl<'a> Decoder<'a> for MarketDataIncrementalRefreshDecoder<'a> {
        #[inline]
        fn get_limit(&self) -> usize {
            self.limit
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.limit = limit;
        }
    }

    impl<'a> MarketDataIncrementalRefreshDecoder<'a> {
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
        pub fn trade_date(&self) -> u16 {
            self.get_buf().get_u16_at(self.offset)
        }

        /// GROUP DECODER (id=268)
        #[inline]
        pub fn entries_decoder(self) -> EntriesDecoder<Self> {
            EntriesDecoder::default().wrap(self)
        }

    }

    #[derive(Debug, Default)]
    pub struct EntriesDecoder<P> {
        parent: Option<P>,
        block_length: u16,
        count: u16,
        index: usize,
        offset: usize,
    }

    impl<'a, P> ActingVersion for EntriesDecoder<P> where P: Reader<'a> + ActingVersion + Default {
        #[inline]
        fn acting_version(&self) -> u16 {
            self.parent.as_ref().unwrap().acting_version()
        }
    }

    impl<'a, P> Reader<'a> for EntriesDecoder<P> where P: Reader<'a> + Default {
        #[inline]
        fn get_buf(&self) -> &ReadBuf<'a> {
            self.parent.as_ref().expect("parent missing").get_buf()
        }
    }

    impl<'a, P> Decoder<'a> for EntriesDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
        #[inline]
        fn get_limit(&self) -> usize {
            self.parent.as_ref().expect("parent missing").get_limit()
        }

        #[inline]
        fn set_limit(&mut self, limit: usize) {
            self.parent.as_mut().expect("parent missing").set_limit(limit);
        }
    }

    impl<'a, P> EntriesDecoder<P> where P: Decoder<'a> + ActingVersion + Default {
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

        /// group token - Token{signal=BEGIN_GROUP, name='Entries', referencedName='null', description='null', packageName='null', id=268, version=0, deprecated=0, encodedLength=82, offset=2, componentTokenCount=136, encoding=Encoding{presence=REQUIRED, primitiveType=null, byteOrder=LITTLE_ENDIAN, minValue=null, maxValue=null, nullValue=null, constValue=null, characterEncoding='null', epoch='null', timeUnit=null, semanticType='null'}}
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

        /// REQUIRED enum
        #[inline]
        pub fn md_update_action(&self) -> md_update_action::MDUpdateAction {
            self.get_buf().get_u8_at(self.offset).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn md_price_level(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 1)
        }

        /// REQUIRED enum
        #[inline]
        pub fn md_entry_type(&self) -> md_entry_type::MDEntryType {
            self.get_buf().get_u8_at(self.offset + 2).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn security_id_source(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 3)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn security_id(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 4)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn rpt_seq(&self) -> u8 {
            self.get_buf().get_u8_at(self.offset + 12)
        }

        /// BIT SET DECODER
        #[inline]
        pub fn quote_condition(&self) -> quote_condition::QuoteCondition {
            quote_condition::QuoteCondition::new(self.get_buf().get_u8_at(self.offset + 13))
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn md_entry_px_decoder(self) -> decimal_64_codec::Decimal64Decoder<Self> {
            let offset = self.offset + 14;
            decimal_64_codec::Decimal64Decoder::default().wrap(self, offset)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn number_of_orders(&self) -> u32 {
            self.get_buf().get_u32_at(self.offset + 22)
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn md_entry_time(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 26)
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn md_entry_size_decoder(self) -> int_qty_32_codec::IntQty32Decoder<Self> {
            let offset = self.offset + 34;
            int_qty_32_codec::IntQty32Decoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn trading_session_id(&self) -> market_state_identifier::MarketStateIdentifier {
            self.get_buf().get_u8_at(self.offset + 38).into()
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn net_chg_prev_day_decoder(self) -> decimal_64_codec::Decimal64Decoder<Self> {
            let offset = self.offset + 39;
            decimal_64_codec::Decimal64Decoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn tick_direction(&self) -> tick_direction::TickDirection {
            self.get_buf().get_u8_at(self.offset + 47).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn open_close_settle_flag(&self) -> open_close_settle_flag::OpenCloseSettleFlag {
            self.get_buf().get_u16_at(self.offset + 48).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn settle_date(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 50)
        }

        /// BIT SET DECODER
        #[inline]
        pub fn trade_condition(&self) -> trade_condition::TradeCondition {
            trade_condition::TradeCondition::new(self.get_buf().get_u8_at(self.offset + 58))
        }

        /// COMPOSITE DECODER
        #[inline]
        pub fn trade_volume_decoder(self) -> int_qty_32_codec::IntQty32Decoder<Self> {
            let offset = self.offset + 59;
            int_qty_32_codec::IntQty32Decoder::default().wrap(self, offset)
        }

        /// REQUIRED enum
        #[inline]
        pub fn md_quote_type(&self) -> md_quote_type::MDQuoteType {
            self.get_buf().get_u8_at(self.offset + 63).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn fixing_bracket(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 64)
        }

        /// REQUIRED enum
        #[inline]
        pub fn aggressor_side(&self) -> side::Side {
            self.get_buf().get_u8_at(self.offset + 72).into()
        }

        /// REQUIRED enum
        #[inline]
        pub fn match_event_indicator(&self) -> match_event_indicator::MatchEventIndicator {
            self.get_buf().get_u8_at(self.offset + 73).into()
        }

        /// primitive field - 'REQUIRED'
        #[inline]
        pub fn trade_id(&self) -> u64 {
            self.get_buf().get_u64_at(self.offset + 74)
        }

    }

} // end decoder

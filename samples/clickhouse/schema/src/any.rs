//! One generated decoder for all of this crate's configured schemas.

include!(concat!(env!("OUT_DIR"), "/any_schema.rs"));

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{market, trading};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    #[test]
    fn market_and_trading_frames_take_their_own_enum() -> TestResult {
        const TRADE: usize = market::TradeEncoder::compute_length_with_header(3, 4, 1);
        let mut trade = [0u8; TRADE];
        let written = market::TradeEncoder::wrap_and_apply_header(&mut trade, 0)
            .fixed(&market::TradeFixedFields {
                ts_event: 1,
                ts_init: 2,
                price: market::Decimal9::new(100),
                size: market::Decimal9::new(1),
                aggressor: market::Side::Buy,
            })
            .symbol(b"BTC")?
            .venue(b"XNAS")?
            .trade_id(b"1")?
            .encoded_length_with_header();
        assert_eq!(written, TRADE);
        // The ingester routes on these two header fields, not on this enum.
        assert_eq!(
            u16::from_le_bytes([trade[2], trade[3]]),
            market::TradeEncoder::TEMPLATE_ID
        );
        assert_eq!(u16::from_le_bytes([trade[4], trade[5]]), market::SCHEMA_ID);

        let AnySchemaMessage::Market(market::AnyMessage::Trade(decoded), _) =
            AnySchemaMessage::decode(&trade, 0)?
        else {
            return Err("trade decoded as another schema".into());
        };
        assert_eq!(decoded.symbol_as_str()?, "BTC");

        const EMA: usize = trading::EmaEncoder::compute_length_with_header(3);
        let mut ema = [0u8; EMA];
        let written = trading::EmaEncoder::wrap_and_apply_header(&mut ema, 0)
            .fixed(&trading::EmaFixedFields {
                ts: 1,
                mid: 2.0,
                ema5m: 2.0,
                ema30m: 2.0,
                ema1h: 2.0,
                ema4h: 2.0,
                ema12h: 2.0,
                ema1d: 2.0,
            })
            .asset(b"ETH")?
            .encoded_length_with_header();
        assert_eq!(written, EMA);
        let decoded = AnySchemaMessage::decode(&ema, 0)?;
        assert_eq!(decoded.schema_id(), trading::SCHEMA_ID);
        let AnySchemaMessage::Trading(trading::AnyMessage::Ema(ema), _) = decoded else {
            return Err("ema decoded as another schema".into());
        };
        assert_eq!(ema.asset_as_str()?, "ETH");
        Ok(())
    }

    #[test]
    fn another_schema_is_not_an_error() -> TestResult {
        let mut header = [0u8; 8];
        header[2] = 9;
        header[4] = 7;
        match AnySchemaMessage::decode(&header, 0)? {
            AnySchemaMessage::Other {
                schema_id: 7,
                template_id: 9,
                ..
            } => Ok(()),
            AnySchemaMessage::Other { .. } => Err("other header fields were misread".into()),
            _ => Err("an unknown schema id was decoded as a known schema".into()),
        }
    }

    #[test]
    fn a_short_buffer_is_an_error() {
        assert!(matches!(
            AnySchemaMessage::decode(&[0, 1, 2], 0),
            Err(SchemaDecodeError::BufferTooShort {
                needed: 8,
                available: 3
            })
        ));
        assert!(matches!(
            AnySchemaMessage::decode(&[0u8; 8], 8),
            Err(SchemaDecodeError::BufferTooShort {
                needed: 8,
                available: 0
            })
        ));
    }
}

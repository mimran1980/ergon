//! One decode for a buffer that may belong to either compiled schema.
//!
//! [`crate::market::AnyMessage`] is every message of `market.xml`.
//! [`crate::trading::AnyMessage`] is every message of `trading.xml`. Template
//! ids start again at 1 in each file, so a match on template id alone is the
//! wrong message. The header's schema id picks the enum.

use std::fmt;

use crate::{market, trading};

const HEADER: usize = market::MESSAGE_HEADER_ENCODED_LENGTH;
const _: () = assert!(trading::MESSAGE_HEADER_ENCODED_LENGTH == HEADER);

/// A frame from `market.xml` or `trading.xml`, or a header from some other schema.
#[non_exhaustive]
pub enum AnySchemaMessage<'a> {
    /// Schema id [`market::SCHEMA_ID`].
    Market(market::AnyMessage<'a>),
    /// Schema id [`trading::SCHEMA_ID`].
    Trading(trading::AnyMessage<'a>),
    /// The header is complete and its schema id is neither of those two.
    /// Event, metric, and trace frames use the persist-client schema and land
    /// here. The body length is not known from the header, so the caller keeps
    /// `buf`.
    Other {
        /// Wire `schemaId`.
        schema_id: u16,
        /// Wire `templateId`.
        template_id: u16,
    },
}

/// [`AnySchemaMessage::decode`] failed before a schema enum could take the frame.
#[derive(Debug)]
pub enum SchemaDecodeError {
    /// Fewer than 8 bytes from `offset`.
    BufferTooShort {
        /// Bytes a message header occupies.
        needed: usize,
        /// Bytes left in `buf` at `offset`.
        available: usize,
    },
    /// `market.xml` rejected the frame.
    Market(market::sbe_rt::DecodeError),
    /// `trading.xml` rejected the frame.
    Trading(trading::sbe_rt::DecodeError),
}

impl fmt::Display for SchemaDecodeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::BufferTooShort { needed, available } => {
                write!(
                    f,
                    "message header needs {needed} bytes, {available} available"
                )
            }
            Self::Market(err) => write!(f, "{err}"),
            Self::Trading(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for SchemaDecodeError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Market(err) => Some(err),
            Self::Trading(err) => Some(err),
            Self::BufferTooShort { .. } => None,
        }
    }
}

impl From<market::sbe_rt::DecodeError> for SchemaDecodeError {
    fn from(err: market::sbe_rt::DecodeError) -> Self {
        Self::Market(err)
    }
}

impl From<trading::sbe_rt::DecodeError> for SchemaDecodeError {
    fn from(err: trading::sbe_rt::DecodeError) -> Self {
        Self::Trading(err)
    }
}

impl fmt::Debug for AnySchemaMessage<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Market(_) => f.write_str("Market(..)"),
            Self::Trading(_) => f.write_str("Trading(..)"),
            Self::Other {
                schema_id,
                template_id,
            } => f
                .debug_struct("Other")
                .field("schema_id", schema_id)
                .field("template_id", template_id)
                .finish(),
        }
    }
}

impl<'a> AnySchemaMessage<'a> {
    /// Read the header at `offset` and decode with the schema it names.
    ///
    /// # Errors
    ///
    /// A short buffer, or a known schema rejecting the frame. An unknown
    /// schema id is [`Self::Other`], not an error.
    #[inline]
    pub fn decode(buf: &'a [u8], offset: usize) -> Result<Self, SchemaDecodeError> {
        let available = buf.len().saturating_sub(offset);
        if available < HEADER {
            return Err(SchemaDecodeError::BufferTooShort {
                needed: HEADER,
                available,
            });
        }
        let schema_id = u16::from_le_bytes([buf[offset + 4], buf[offset + 5]]);
        match schema_id {
            market::SCHEMA_ID => Ok(Self::Market(market::AnyMessage::decode(buf, offset)?)),
            trading::SCHEMA_ID => Ok(Self::Trading(trading::AnyMessage::decode(buf, offset)?)),
            _ => Ok(Self::Other {
                schema_id,
                template_id: u16::from_le_bytes([buf[offset + 2], buf[offset + 3]]),
            }),
        }
    }

    /// The header's schema id.
    #[inline]
    #[must_use]
    pub fn schema_id(&self) -> u16 {
        match self {
            Self::Market(_) => market::SCHEMA_ID,
            Self::Trading(_) => trading::SCHEMA_ID,
            Self::Other { schema_id, .. } => *schema_id,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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

        let AnySchemaMessage::Market(market::AnyMessage::Trade(decoded)) =
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
        let AnySchemaMessage::Trading(trading::AnyMessage::Ema(ema)) = decoded else {
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
            } => Ok(()),
            AnySchemaMessage::Other { .. } => Err("other header fields were misread".into()),
            AnySchemaMessage::Market(_) | AnySchemaMessage::Trading(_) => {
                Err("an unknown schema id was decoded as a known schema".into())
            }
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

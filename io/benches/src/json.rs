//! Zero-copy field scanner for Binance JSON market-data messages.
//!
//! Binance's payloads are compact (no whitespace) and the fields we need are flat
//! integers, so a byte scan is enough and borrows from the decrypted buffer. Both
//! I/O variants run exactly this code, so its cost cancels in the comparison.

/// Splits a combined-stream wrapper `{"stream":"<name>","data":{...}}` into
/// `(name, data)`.
#[must_use]
pub fn combined(payload: &[u8]) -> Option<(&[u8], &[u8])> {
    const PREFIX: &[u8] = b"{\"stream\":\"";
    let rest = payload.strip_prefix(PREFIX)?;
    let end = rest.iter().position(|&b| b == b'"')?;
    let name = &rest[..end];
    let data_key = b",\"data\":";
    let after = &rest[end + 1..];
    let data = after.strip_prefix(data_key.as_slice())?;
    Some((name, data))
}

/// Finds the integer value of `"key":<digits>` in `data`. Matches whose value is not
/// an integer (for example a quoted price) are skipped.
#[must_use]
pub fn find_int(data: &[u8], key: &[u8]) -> Option<i64> {
    let mut i = 0;
    while i + key.len() + 3 <= data.len() {
        let off = data[i..].iter().position(|&b| b == b'"')?;
        let q = i + off;
        let k_start = q + 1;
        let k_end = k_start + key.len();
        if k_end + 2 <= data.len()
            && &data[k_start..k_end] == key
            && data[k_end] == b'"'
            && data[k_end + 1] == b':'
        {
            let mut j = k_end + 2;
            let neg = data.get(j) == Some(&b'-');
            if neg {
                j += 1;
            }
            let mut v: i64 = 0;
            let mut digits = 0;
            while let Some(&c) = data.get(j) {
                if !c.is_ascii_digit() {
                    break;
                }
                v = v.checked_mul(10)?.checked_add(i64::from(c - b'0'))?;
                digits += 1;
                j += 1;
            }
            if digits > 0 {
                return Some(if neg { -v } else { v });
            }
        }
        i = q + 1;
    }
    None
}

/// What kind of Binance stream a name refers to, which decides the id field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// `@trade`: id `t`.
    Trade,
    /// `@aggTrade`: id `a`.
    AggTrade,
    /// `@bookTicker`: id `u`.
    BookTicker,
    /// `@depth...`: id `u` (final update id).
    Depth,
}

impl Kind {
    /// Classifies a stream name by its suffix.
    #[must_use]
    pub fn of(stream: &str) -> Option<Self> {
        let suffix = stream.split('@').nth(1)?;
        if suffix == "trade" {
            Some(Self::Trade)
        } else if suffix == "aggTrade" {
            Some(Self::AggTrade)
        } else if suffix == "bookTicker" {
            Some(Self::BookTicker)
        } else if suffix.starts_with("depth") {
            Some(Self::Depth)
        } else {
            None
        }
    }

    /// The JSON key holding this stream's unique id.
    #[must_use]
    pub fn id_key(self) -> &'static [u8] {
        match self {
            Self::Trade => b"t",
            Self::AggTrade => b"a",
            Self::BookTicker | Self::Depth => b"u",
        }
    }
}

/// Fields extracted from one Binance message.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fields {
    /// Unique id within the stream.
    pub id: i64,
    /// Event time `E` (ms, or µs with `timeUnit=MICROSECOND`).
    pub event_time: i64,
    /// Transaction time `T` (0 when absent).
    pub trade_time: i64,
}

/// Extracts the id, `E` and `T` from a stream's data object.
#[must_use]
pub fn fields(data: &[u8], kind: Kind) -> Option<Fields> {
    Some(Fields {
        id: find_int(data, kind.id_key())?,
        event_time: find_int(data, b"E")?,
        trade_time: find_int(data, b"T").unwrap_or(0),
    })
}

/// Converts a Binance timestamp to nanoseconds, detecting ms vs µs by magnitude.
#[must_use]
pub fn binance_ts_to_ns(ts: i64) -> i64 {
    if ts > 100_000_000_000_000 {
        ts * 1_000
    } else {
        ts * 1_000_000
    }
}

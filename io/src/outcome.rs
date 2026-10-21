//! What became of a request: whether it was sent, whether it ran, and the
//! server's wait.
//!
//! - [`Sent`]: for a request that got no response, whether the server can
//!   have seen it. Only the transport knows; `HttpClient` passes it to
//!   `ClientHandler::on_error`.
//! - [`Verdict`]: one answer per request.
//! - [`generic`]: the verdict from the HTTP status and `Retry-After` alone
//!   (RFC 9110, RFC 6585). The application's venue adapter reads its venue's
//!   body codes and usage headers first, and calls it last.
//! - [`retry_after_ms`], [`header_u64`], [`ms_until_epoch`]: the header
//!   parsing an adapter needs. No allocation; malformed input is an error,
//!   never a default.
//! - [`refused_upgrade`]: a refused WebSocket handshake (429, 418) held or
//!   halted on the connect gate's bucket.
//!
//! | Verdict | Ran? | What to do |
//! |---|---|---|
//! | `Done` | yes | apply |
//! | `Rejected` | no | fix; never re-send unchanged |
//! | `NotRun` | no | re-send through the budget, inside the request's deadline |
//! | `Throttled` | no | the scope is held; re-send after the hold, inside the deadline |
//! | `Banned` | no | the scope stays halted until a person clears it; alert |
//! | `ClockSkew` | no | re-send with a new timestamp, inside the deadline |
//! | `Unknown` | maybe | reconcile by client order id; never re-send an order |
//!
//! A 408 is `Unknown`: the server may have run the request before the wait
//! timed out. A 418 is `Banned` (how Binance bans an IP).
//!
//! Re-send through [`Budget::try_resend`], which drops a request whose own
//! deadline has passed.

use std::num::NonZeroU64;

use crate::error::{Error, Result};
use crate::http::Response;
use crate::limit::{BucketId, Budget};

/// For a request that got no response: could the server have seen it?
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Sent {
    /// Its TLS record was never wholly handed to the kernel. A request is
    /// one record, and the server cannot decrypt part of one, so it cannot
    /// have run. Safe to send again.
    No,
    /// Handed to the kernel, or this cannot be ruled out: it may have run.
    Maybe,
}

/// What became of one request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Verdict {
    /// It ran.
    Done,
    /// It did not run, and would fail again: do not send it again unchanged.
    Rejected,
    /// It did not run; it may be sent again (budget and deadline permitting).
    NotRun,
    /// It did not run: the server says `bucket` is full. `bucket` is held
    /// until `until_ns` (the budget's clock).
    Throttled {
        /// The bucket the server named.
        bucket: BucketId,
        /// End of the hold.
        until_ns: i64,
    },
    /// It did not run: the venue banned the scope (Binance 418, Bybit 403).
    /// The budget halts `bucket` until the application clears it
    /// ([`Budget::clear_halt`], a person's action); alert a person.
    Banned {
        /// The banned scope.
        bucket: BucketId,
        /// End of the server's wait, for the alert. The scope stays halted
        /// after it.
        until_ns: i64,
    },
    /// It did not run: its timestamp was outside the venue's window. Send
    /// it again with a new timestamp.
    ClockSkew,
    /// It may have run (a 5xx, a timeout, a lost connection). Reconcile by
    /// client order id; never send an order again blindly.
    Unknown,
}

impl Verdict {
    /// The verdict for a request that got no response: [`Sent::No`] did not
    /// run, [`Sent::Maybe`] is unknown.
    #[must_use]
    pub const fn from_sent(sent: Sent) -> Self {
        match sent {
            Sent::No => Self::NotRun,
            Sent::Maybe => Self::Unknown,
        }
    }

    /// Whether the request may be sent again at all: it did not run and was
    /// not refused for good (`NotRun`, `Throttled`, `ClockSkew`). Even then,
    /// only through the budget and inside its own deadline
    /// ([`Budget::try_resend`]).
    #[must_use]
    pub const fn may_resend(self) -> bool {
        matches!(
            self,
            Self::NotRun | Self::Throttled { .. } | Self::ClockSkew
        )
    }
}

/// The caller's clocks for one loop pass.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Now {
    /// The budget's clock, ns: monotonic, the same for every budget call.
    pub ns: i64,
    /// UTC, ns since 1970: a `Retry-After` date is measured against it when
    /// the response has no `Date`.
    pub real_ns: i64,
}

/// `Retry-After` as a wait in ms. `Ok(None)`: no `Retry-After`.
///
/// - Delay-seconds, or an HTTP date in any of the three RFC 9110 formats.
/// - A date is measured against the response's own `Date` header (both are
///   the server's clock), else against `server_now_ms`, the server's time as
///   best known (your UTC clock). A date already past is 0.
/// - No allocation unless it fails.
///
/// # Errors
/// [`Error::Http`]: the header, or a `Date` it needs, is malformed or out of
/// range, or the header appears twice with different values.
pub fn retry_after_ms(resp: &Response<'_>, server_now_ms: i64) -> Result<Option<u64>> {
    retry_after_in(resp.headers, server_now_ms)
}

/// [`retry_after_ms`] over a header list (the WebSocket upgrade's too).
pub(crate) fn retry_after_in(
    headers: &[httparse::Header<'_>],
    server_now_ms: i64,
) -> Result<Option<u64>> {
    let Some(value) = single(headers, "retry-after")? else {
        return Ok(None);
    };
    if let Some(s) = decimal(value) {
        return s
            .checked_mul(1_000)
            .map(Some)
            .ok_or_else(|| malformed("Retry-After", value));
    }
    let at = http_date_ms(value, server_now_ms).ok_or_else(|| malformed("Retry-After", value))?;
    let reference = match single(headers, "date")? {
        Some(d) => http_date_ms(d, server_now_ms).ok_or_else(|| malformed("Date", d))?,
        None => server_now_ms,
    };
    Ok(Some(ms_until_epoch(at, reference)))
}

/// A decimal-integer header, such as a usage header
/// (`X-MBX-USED-WEIGHT-1M`). `Ok(None)`: absent. Names are ASCII
/// case-insensitive.
///
/// # Errors
/// [`Error::Http`]: not decimal digits, above `u64::MAX`, or the header
/// appears twice with different values.
pub fn header_u64(resp: &Response<'_>, name: &str) -> Result<Option<u64>> {
    let Some(value) = single(resp.headers, name)? else {
        return Ok(None);
    };
    decimal(value)
        .map(Some)
        .ok_or_else(|| malformed(name, value))
}

/// The wait until a server epoch time in ms (Binance WebSocket
/// `retryAfter`, Bybit's reset time): 0 if past. Pass the server's time as
/// best known (your UTC clock), never a clock of another epoch.
#[must_use]
pub const fn ms_until_epoch(epoch_ms: i64, server_now_ms: i64) -> u64 {
    if epoch_ms > server_now_ms {
        epoch_ms.abs_diff(server_now_ms)
    } else {
        0
    }
}

/// The shortest ban this function assumes when the server gives no wait:
/// 2 minutes, Binance's documented minimum ("from 2 minutes to 3 days").
///
/// A venue adapter that knows a longer one (Bybit: 10 minutes) calls
/// [`Budget::banned`] itself before falling through here.
pub const BAN_MIN_MS: NonZeroU64 = NonZeroU64::MIN.saturating_add(119_999);

/// The verdict from the status and `Retry-After` alone, with no venue
/// knowledge.
///
/// | Status | Verdict |
/// |---|---|
/// | 2xx | `Done` |
/// | 408 | `Unknown` (see below) |
/// | 418 | `Banned`: `scope` halted until a person clears it |
/// | 429 | `Throttled`: `scope` held for the server's wait, or the bucket's backoff without one |
/// | 3xx (no redirect is followed), other 4xx | `Rejected` |
/// | 5xx, 0, anything else | `Unknown` |
///
/// - **408 is Unknown, not `NotRun`.** Binance USDⓈ-M futures documents 408 as
///   "a timeout has occurred while waiting for a response from the backend
///   server": the order may have run. A safe re-send would place it twice.
///   Reconcile by client order id.
/// - **418 halts.** It is how Binance bans an IP, and the same treatment as
///   a refused WebSocket handshake ([`refused_upgrade`]): a ban stays stopped
///   until a person clears it ([`Budget::clear_halt`]). A venue adapter that
///   knows a 418 is not a ban handles the status itself before calling this.
///
/// # Errors
/// - A 429 whose `Retry-After` (or the `Date` it needs) is malformed
///   ([`Error::Http`]). `scope` is held for the bucket's backoff first: a
///   429 always holds. A 418 whose wait is malformed halts on
///   [`BAN_MIN_MS`] first: a ban always halts.
/// - `scope` is not a bucket of `budget` (from `Budget::throttled`).
pub fn generic(
    resp: &Response<'_>,
    scope: BucketId,
    budget: &mut Budget,
    now: Now,
) -> Result<Verdict> {
    Ok(match resp.status {
        200..=299 => Verdict::Done,
        408 => Verdict::Unknown,
        418 => {
            let wait = retry_after_ms(resp, now.real_ns.div_euclid(1_000_000));
            let ms = wait
                .as_ref()
                .ok()
                .copied()
                .flatten()
                .and_then(NonZeroU64::new)
                .unwrap_or(BAN_MIN_MS);
            let until_ns = budget.banned(scope, ms, now.ns)?;
            wait?;
            Verdict::Banned {
                bucket: scope,
                until_ns,
            }
        }
        429 => {
            let wait = retry_after_ms(resp, now.real_ns.div_euclid(1_000_000));
            let until_ns =
                budget.throttled(scope, wait.as_ref().ok().copied().flatten(), now.ns)?;
            wait?;
            Verdict::Throttled {
                bucket: scope,
                until_ns,
            }
        }
        300..=499 => Verdict::Rejected,
        _ => Verdict::Unknown,
    })
}

/// A refused WebSocket handshake ([`Error::Upgrade`]), on `scope`: the
/// bucket that gates connects on that IP (`Reconnect::poll_gated`).
///
/// | Refusal | Budget | Verdict |
/// |---|---|---|
/// | 429 | [`Budget::throttled`]: held for `Retry-After`, or the bucket's backoff without one | `Throttled` |
/// | 418 | [`Budget::banned`]: halted until a person clears it. The server's wait is `Retry-After`, or `ban_wait_ms` (the venue's documented minimum) without one or with 0 | `Banned` |
/// | anything else | untouched | `None` |
///
/// - Then `Reconnect::defer(link, until_ns - now.ns)` keeps the refused
///   link from trying before the wait ends, and the gate keeps every link
///   on `scope` from trying until the hold ends or the halt is cleared.
/// - A 418 is how Binance bans an IP. A ban sent as a 403 (Binance's WAF,
///   Bybit) is venue knowledge: the venue adapter calls
///   [`Budget::banned`] itself.
///
/// # Errors
/// - [`Error::Http`]: the refusal's `Retry-After` could not be read
///   (`retry_after_invalid`). `scope` is held (429) or halted (418) on the
///   fallback wait first: a refusal always holds.
/// - [`Error::Config`]: `scope` is not a bucket of `budget`.
pub fn refused_upgrade(
    error: &Error,
    scope: BucketId,
    budget: &mut Budget,
    ban_wait_ms: NonZeroU64,
    now: Now,
) -> Result<Option<Verdict>> {
    let (status, retry_after_ms, invalid) = match error {
        Error::Upgrade {
            status,
            retry_after_ms,
            retry_after_invalid,
            ..
        } => (*status, *retry_after_ms, *retry_after_invalid),
        _ => return Ok(None),
    };
    let verdict = match status {
        429 => Verdict::Throttled {
            bucket: scope,
            until_ns: budget.throttled(scope, retry_after_ms, now.ns)?,
        },
        418 => {
            let wait = retry_after_ms
                .and_then(NonZeroU64::new)
                .unwrap_or(ban_wait_ms);
            Verdict::Banned {
                bucket: scope,
                until_ns: budget.banned(scope, wait, now.ns)?,
            }
        }
        _ => return Ok(None),
    };
    if invalid {
        return Err(Error::Http(format!(
            "refused upgrade ({status}): malformed Retry-After header; held on the fallback wait"
        )));
    }
    Ok(Some(verdict))
}

/// The value of header `name`, trimmed. A header that appears more than
/// once must carry one value.
fn single<'a>(headers: &'a [httparse::Header<'a>], name: &str) -> Result<Option<&'a [u8]>> {
    let mut found: Option<&[u8]> = None;
    for h in headers.iter().filter(|h| h.name.eq_ignore_ascii_case(name)) {
        let v = h.value.trim_ascii();
        match found {
            Some(f) if f != v => {
                return Err(Error::Http(format!(
                    "header {name} appears twice with different values"
                )));
            }
            _ => found = Some(v),
        }
    }
    Ok(found)
}

/// 1 or more ASCII digits, within `u64`.
fn decimal(v: &[u8]) -> Option<u64> {
    if v.is_empty() {
        return None;
    }
    v.iter().try_fold(0u64, |n, &b| {
        if b.is_ascii_digit() {
            n.checked_mul(10)?.checked_add(u64::from(b - b'0'))
        } else {
            None
        }
    })
}

#[cold]
fn malformed(name: &str, value: &[u8]) -> Error {
    let shown = String::from_utf8_lossy(&value[..value.len().min(64)]);
    Error::Http(format!("malformed {name} header: {shown:?}"))
}

const DAYS: [&[u8]; 7] = [b"Sun", b"Mon", b"Tue", b"Wed", b"Thu", b"Fri", b"Sat"];
const DAYS_LONG: [&[u8]; 7] = [
    b"Sunday",
    b"Monday",
    b"Tuesday",
    b"Wednesday",
    b"Thursday",
    b"Friday",
    b"Saturday",
];
const MONTHS: [&[u8]; 12] = [
    b"Jan", b"Feb", b"Mar", b"Apr", b"May", b"Jun", b"Jul", b"Aug", b"Sep", b"Oct", b"Nov", b"Dec",
];

/// An HTTP date (RFC 9110 5.6.7) as ms since 1970: IMF-fixdate
/// (`Sun, 06 Nov 1994 08:49:37 GMT`), RFC 850 (`Sunday, 06-Nov-94 08:49:37
/// GMT`) or asctime (`Sun Nov  6 08:49:37 1994`). Names are case-sensitive,
/// every field is range-checked, and the weekday must match the date. An RFC
/// 850 two-digit year is the nearest year after `now_ms`'s minus 50, up to
/// its plus 50.
fn http_date_ms(v: &[u8], now_ms: i64) -> Option<i64> {
    imf_fixdate(v)
        .or_else(|| rfc850(v, now_ms))
        .or_else(|| asctime(v))
}

fn imf_fixdate(v: &[u8]) -> Option<i64> {
    let mut c = Cursor(v);
    let weekday = c.pick(&DAYS)?;
    c.lit(b", ")?;
    let day = c.digits(2)?;
    c.lit(b" ")?;
    let month = c.pick(&MONTHS)?;
    c.lit(b" ")?;
    let year = c.digits(4)?;
    c.lit(b" ")?;
    let time = c.time()?;
    c.lit(b" GMT")?;
    c.end()?;
    timestamp(weekday, year, month, day, time)
}

fn rfc850(v: &[u8], now_ms: i64) -> Option<i64> {
    let mut c = Cursor(v);
    let weekday = c.pick(&DAYS_LONG)?;
    c.lit(b", ")?;
    let day = c.digits(2)?;
    c.lit(b"-")?;
    let month = c.pick(&MONTHS)?;
    c.lit(b"-")?;
    let yy = c.digits(2)?;
    c.lit(b" ")?;
    let time = c.time()?;
    c.lit(b" GMT")?;
    c.end()?;
    // The year with these last two digits nearest to now, but never more
    // than 50 years ahead (RFC 9110 5.6.7).
    let now_year = year_of_days(now_ms.div_euclid(86_400_000));
    let mut year = now_year - now_year.rem_euclid(100) + yy;
    if year > now_year + 50 {
        year -= 100;
    } else if year <= now_year - 50 {
        year += 100;
    }
    timestamp(weekday, year, month, day, time)
}

fn asctime(v: &[u8]) -> Option<i64> {
    let mut c = Cursor(v);
    let weekday = c.pick(&DAYS)?;
    c.lit(b" ")?;
    let month = c.pick(&MONTHS)?;
    c.lit(b" ")?;
    let day = if c.lit(b" ").is_some() {
        c.digits(1)?
    } else {
        c.digits(2)?
    };
    c.lit(b" ")?;
    let time = c.time()?;
    c.lit(b" ")?;
    let year = c.digits(4)?;
    c.end()?;
    timestamp(weekday, year, month, day, time)
}

/// ms since 1970 for a checked date; `month` from 0.
fn timestamp(
    weekday: usize,
    year: i64,
    month: usize,
    day: i64,
    (h, m, s): (i64, i64, i64),
) -> Option<i64> {
    let month = i64::try_from(month).ok()? + 1;
    if day < 1 || day > days_in_month(year, month) || h > 23 || m > 59 || s > 60 {
        return None;
    }
    let days = days_from_civil(year, month, day);
    if usize::try_from((days + 4).rem_euclid(7)).ok()? != weekday {
        return None;
    }
    Some(((days * 24 + h) * 60 + m) * 60_000 + s * 1_000)
}

const fn days_in_month(year: i64, month: i64) -> i64 {
    match month {
        2 if (year % 4 == 0 && year % 100 != 0) || year % 400 == 0 => 29,
        2 => 28,
        4 | 6 | 9 | 11 => 30,
        _ => 31,
    }
}

/// Days since 1970-01-01 (proleptic Gregorian; H. Hinnant's algorithm).
const fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// The year of a day counted from 1970-01-01.
const fn year_of_days(days: i64) -> i64 {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    // March-based year: January and February belong to the next one.
    if mp >= 10 {
        yoe + era * 400 + 1
    } else {
        yoe + era * 400
    }
}

/// A parser over the bytes still unread.
struct Cursor<'a>(&'a [u8]);

impl Cursor<'_> {
    fn lit(&mut self, s: &[u8]) -> Option<()> {
        self.0 = self.0.strip_prefix(s)?;
        Some(())
    }

    fn digits(&mut self, n: usize) -> Option<i64> {
        let (d, rest) = self.0.split_at_checked(n)?;
        if !d.iter().all(u8::is_ascii_digit) {
            return None;
        }
        self.0 = rest;
        Some(d.iter().fold(0, |a, &b| a * 10 + i64::from(b - b'0')))
    }

    fn pick(&mut self, names: &[&[u8]]) -> Option<usize> {
        let i = names.iter().position(|n| self.0.starts_with(n))?;
        self.0 = &self.0[names[i].len()..];
        Some(i)
    }

    /// `hh:mm:ss`.
    fn time(&mut self) -> Option<(i64, i64, i64)> {
        let h = self.digits(2)?;
        self.lit(b":")?;
        let m = self.digits(2)?;
        self.lit(b":")?;
        Some((h, m, self.digits(2)?))
    }

    const fn end(&self) -> Option<()> {
        if self.0.is_empty() { Some(()) } else { None }
    }
}

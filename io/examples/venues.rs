//! Venue adapters: Binance spot, Bybit v5 and OKX v5 responses turned into a
//! [`Verdict`] and the matching [`Budget`] calls.
//!
//! - Every platform: plain functions over a `Response` and a `Budget`. No
//!   network, no I/O.
//! - Run: `cargo run -p ergon-io --example venues` replays the recorded
//!   responses in `io/tests/fixtures/venues/` and prints each decision.
//! - `io/tests/example_venues.rs` checks every adapter against them.
//! - In this repository only: venues change their rules without notice, so
//!   these tables are not part of the published crate. Check them against
//!   each venue's docs before use (links in the fixture files).
//!
//! The rules it follows:
//!
//! - **One sender per account and IP.** The budget sees only this process.
//!   Another process, host or script on the same account or IP spends the
//!   same venue limit unseen.
//! - **A ban stops sending until a person clears it.** The adapter halts the
//!   banned scope; [`alerts`] names it on every loop pass;
//!   `Budget::clear_halt` is a person's action, never the loop's.
//! - **Only a request that did not run is sent again,** through the budget
//!   and inside its own deadline ([`next`]); late, it is dropped.
//! - **An order that may have run is never sent again:** look it up by its
//!   client order id.
//! - **An adapter error** (a response it cannot read) means: treat the
//!   request as unknown.

use std::io::Write;
use std::num::NonZeroU64;
use std::time::Duration;

use ergon_io::limit::{
    BucketId, BucketSpec, BucketStats, Budget, Charge, Class, MAX_CHARGES, Resend, Ticket,
};
use ergon_io::outcome::{self, Now};
use ergon_io::{Error, Response, Result, Verdict};

/// Binance spot responses (`### name` entries).
pub const BINANCE: &str = include_str!("../tests/fixtures/venues/binance.http");
/// An `exchangeInfo` body with the `rateLimits` Binance documents as its
/// example numbers (written, not captured). Load the live one at start.
pub const BINANCE_EXCHANGE_INFO: &str =
    include_str!("../tests/fixtures/venues/binance-exchange-info.json");
/// Bybit v5 responses.
pub const BYBIT: &str = include_str!("../tests/fixtures/venues/bybit.http");
/// OKX v5 responses.
pub const OKX: &str = include_str!("../tests/fixtures/venues/okx.http");

/// The buckets one request charges: at most [`MAX_CHARGES`], on the stack.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Parts {
    list: [Charge; MAX_CHARGES],
    len: usize,
}

impl Parts {
    /// `first` alone.
    #[must_use]
    pub const fn new(first: Charge) -> Self {
        Self {
            list: [first; MAX_CHARGES],
            len: 1,
        }
    }

    /// Adds a part.
    ///
    /// # Errors
    /// [`Error::Config`]: already [`MAX_CHARGES`] parts.
    pub fn push(&mut self, part: Charge) -> Result<()> {
        let slot = self
            .list
            .get_mut(self.len)
            .ok_or_else(|| Error::Config(format!("more than {MAX_CHARGES} buckets")))?;
        *slot = part;
        self.len += 1;
        Ok(())
    }

    /// The parts, for `Budget::try_charge`.
    #[must_use]
    pub fn as_slice(&self) -> &[Charge] {
        self.list.get(..self.len).unwrap_or(&[])
    }
}

/// A rolling bucket of `limit` per `window_ms`, in sub-windows of `sub_ms`.
fn rolling(name: impl Into<String>, limit: u32, window_ms: u64, sub_ms: u64) -> Result<BucketSpec> {
    let name = name.into();
    let bad = |what: &str| Error::Config(format!("bucket {name}: {what}"));
    let window = u32::try_from(window_ms).map_err(|_| bad("window too long"))?;
    let slots = window_ms
        .checked_div(sub_ms)
        .and_then(|s| u16::try_from(s).ok())
        .ok_or_else(|| bad("too many sub-windows"))?;
    Ok(BucketSpec::rolling(name, limit, window, slots))
}

/// The server's time as best known, ms: this host's UTC (`CLOCK_REALTIME`,
/// kept by chrony). A wait the server states as a time is measured on the
/// response's own clock instead, where it has one (Bybit's `time`).
const fn utc_ms(now: Now) -> i64 {
    now.real_ns.div_euclid(1_000_000)
}

/// A 429 (or a venue code meaning the same) on `bucket`: held for the
/// server's `Retry-After`, or the bucket's backoff without one. A
/// malformed `Retry-After` still holds (on the backoff), then is reported.
fn throttle(r: &Response<'_>, bucket: BucketId, budget: &mut Budget, now: Now) -> Result<Verdict> {
    let wait = outcome::retry_after_ms(r, utc_ms(now));
    let until_ns = budget.throttled(bucket, wait.as_ref().ok().copied().flatten(), now.ns)?;
    wait?;
    Ok(Verdict::Throttled { bucket, until_ns })
}

/// A ban on `bucket`: halted until a person clears it. The server's wait is
/// `Retry-After`, else `min_ms`, the venue's documented shortest ban.
fn ban(
    r: &Response<'_>,
    bucket: BucketId,
    min_ms: NonZeroU64,
    budget: &mut Budget,
    now: Now,
) -> Result<Verdict> {
    let wait = outcome::retry_after_ms(r, utc_ms(now));
    let ms = wait
        .as_ref()
        .ok()
        .copied()
        .flatten()
        .and_then(NonZeroU64::new)
        .unwrap_or(min_ms);
    let until_ns = budget.banned(bucket, ms, now.ns)?;
    wait?;
    Ok(Verdict::Banned { bucket, until_ns })
}

/// A usage header as a count: `Ok(None)` when absent.
fn usage(r: &Response<'_>, name: &str) -> Result<Option<u32>> {
    outcome::header_u64(r, name)?
        .map(|v| u32::try_from(v).map_err(|_| Error::Http(format!("{name}: {v} is out of range"))))
        .transpose()
}

/// The body as JSON when it looks like JSON; `Ok(None)` for an empty or
/// HTML body (a proxy's error page). JSON that does not parse is an error.
fn json_body<'a>(r: &Response<'a>) -> Result<Option<json::Value<'a>>> {
    match r.body.trim_ascii().first() {
        Some(b'{' | b'[') => json::Value::parse(r.body).map(Some),
        _ => Ok(None),
    }
}

fn contains_ignore_case(hay: &[u8], needle: &[u8]) -> bool {
    !needle.is_empty()
        && hay
            .windows(needle.len())
            .any(|w| w.eq_ignore_ascii_case(needle))
}

pub mod binance {
    //! Binance spot REST.
    //!
    //! - Limits come from `GET /api/v3/exchangeInfo` at start ([`limits`]),
    //!   never from constants: they differ by product and change.
    //! - Scopes: request weight and raw requests per IP; orders per account
    //!   (every key on it), per 10 s and per day.
    //! - The IP's weight bucket is the IP scope: every request charges it,
    //!   so a ban halts it.

    use super::{Error, NonZeroU64, Now, Parts, Response, Result, Verdict, json, outcome};
    use ergon_io::limit::{BucketId, BucketSpec, Budget, Charge};

    /// Binance's shortest IP ban ("from 2 minutes to 3 days"): the server's
    /// wait for a ban that gives none (the WAF's 403).
    pub const BAN_MIN_MS: NonZeroU64 = NonZeroU64::MIN.saturating_add(119_999);

    /// What a `rateLimits` entry counts.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Kind {
        /// `REQUEST_WEIGHT`, per IP.
        Weight,
        /// `ORDERS`, per account.
        Orders,
        /// `RAW_REQUESTS`, per IP.
        Raw,
    }

    /// One `rateLimits` entry as a bucket.
    #[derive(Debug, Clone)]
    pub struct Limit {
        /// What it counts.
        pub kind: Kind,
        /// The bucket: named like `ORDERS 10S`.
        pub spec: BucketSpec,
        /// Its usage header (`X-MBX-USED-WEIGHT-1M`); raw requests have none.
        pub header: Option<String>,
        /// How a -1015 message names its interval: `per 10 SECOND`.
        pub per: String,
    }

    /// Whether `body` names the interval `per` describes (`per 10 SECOND`):
    /// the number and the unit word, any case, singular or plural. The
    /// errors page gives the format `"… per %s."` with no spelling for the
    /// interval, so this accepts the common ones.
    fn names_interval(body: &[u8], per: &str) -> bool {
        let Some(tail) = per.strip_prefix("per ") else {
            return false;
        };
        let mut parts = tail.splitn(2, ' ');
        let (Some(num), Some(unit)) = (parts.next(), parts.next()) else {
            return false;
        };
        let unit = unit.to_ascii_lowercase();
        let stem = unit.strip_suffix('s').unwrap_or(&unit);
        [
            format!("{num} {stem}"),
            format!("{num} {stem}s"),
            format!("{num}{stem}"),
        ]
        .iter()
        .any(|n| super::contains_ignore_case(body, n.as_bytes()))
    }

    /// Every `rateLimits` entry of an `exchangeInfo` body.
    ///
    /// Sub-windows: 100 ms for a SECOND limit, 1 s for MINUTE, 1 min for
    /// DAY; the guard is one of them.
    ///
    /// # Errors
    /// [`Error::Http`]: not JSON, no `rateLimits`, a field missing or of the
    /// wrong type, or a kind or interval this adapter does not know (a limit
    /// it would not enforce). [`Error::Config`]: a zero or oversized limit or
    /// window.
    pub fn limits(exchange_info: &[u8]) -> Result<Vec<Limit>> {
        let bad = |what: String| Error::Http(format!("exchangeInfo: {what}"));
        let info = json::Value::parse(exchange_info)?;
        let list = info
            .get("rateLimits")?
            .ok_or_else(|| bad("no rateLimits".into()))?;
        let mut out = Vec::new();
        for entry in list.items()? {
            let field = |name: &str| {
                entry
                    .get(name)?
                    .ok_or_else(|| bad(format!("an entry has no {name}")))
            };
            let kind_name = field("rateLimitType")?.as_str()?;
            let interval = field("interval")?.as_str()?;
            let num = field("intervalNum")?.as_i64()?;
            let limit = field("limit")?.as_i64()?;
            let (kind, header_root) = match kind_name {
                "REQUEST_WEIGHT" => (Kind::Weight, Some("X-MBX-USED-WEIGHT")),
                "ORDERS" => (Kind::Orders, Some("X-MBX-ORDER-COUNT")),
                "RAW_REQUESTS" => (Kind::Raw, None),
                other => return Err(bad(format!("unknown rateLimitType {other}"))),
            };
            let (unit_ms, sub_ms, letter) = match interval {
                "SECOND" => (1_000, 100, 'S'),
                "MINUTE" => (60_000, 1_000, 'M'),
                "HOUR" => (3_600_000, 60_000, 'H'),
                "DAY" => (86_400_000, 60_000, 'D'),
                other => return Err(bad(format!("unknown interval {other}"))),
            };
            let num = u64::try_from(num)
                .ok()
                .filter(|&n| n > 0)
                .ok_or_else(|| bad(format!("intervalNum {num}")))?;
            let limit = u32::try_from(limit).map_err(|_| bad(format!("limit {limit}")))?;
            let window_ms = num
                .checked_mul(unit_ms)
                .ok_or_else(|| bad(format!("intervalNum {num}")))?;
            out.push(Limit {
                kind,
                spec: super::rolling(
                    format!("{kind_name} {num}{letter}"),
                    limit,
                    window_ms,
                    sub_ms,
                )?,
                header: header_root.map(|h| format!("{h}-{num}{letter}")),
                per: format!("per {num} {interval}"),
            });
        }
        Ok(out)
    }

    /// The adapter: which buckets each request charges, and what each
    /// response means.
    #[derive(Debug, Clone)]
    pub struct Binance {
        /// The IP scope: a gate with no count. Every request and every
        /// connect charges it, so a ban that halts it stops them all.
        ip: BucketId,
        weight: BucketId,
        raw: Option<BucketId>,
        /// Order buckets and how a -1015 names each.
        orders: Vec<(BucketId, String)>,
        /// Usage headers and their buckets.
        usage: Vec<(String, BucketId)>,
    }

    impl Binance {
        /// The adapter and a budget with one bucket per limit. The weight
        /// bucket keeps 2 % for cancels and reduce-only orders: cancels do
        /// not count as orders, so the order buckets have no reserve.
        ///
        /// # Errors
        /// [`Error::Http`]: no `REQUEST_WEIGHT` or no `ORDERS` limit, or two
        /// `REQUEST_WEIGHT` or `RAW_REQUESTS` limits (one would go unchecked).
        /// [`Error::Config`]: the budget refused a bucket.
        pub fn new(limits: Vec<Limit>) -> Result<(Self, Budget)> {
            // The IP scope first: a gate every request and every connect
            // charges. A ban halts it, so one ban stops them all.
            let mut specs = Vec::with_capacity(limits.len() + 1);
            specs.push(BucketSpec::gate("ip"));
            let mut kinds = Vec::with_capacity(limits.len());
            for l in limits {
                let spec = if l.kind == Kind::Weight {
                    let reserve = match l.spec.model {
                        ergon_io::limit::Model::Rolling { limit, .. } => limit / 50,
                        _ => 0,
                    };
                    l.spec.with_reserve(reserve)
                } else {
                    l.spec
                };
                kinds.push((l.kind, spec.name.clone(), l.header, l.per));
                specs.push(spec);
            }
            let budget = Budget::new(specs)?;
            let id = |name: &str| {
                budget
                    .id(name)
                    .ok_or_else(|| Error::Config(format!("no bucket {name}")))
            };
            let (mut weight, mut raw, mut orders, mut usage) = (None, None, Vec::new(), Vec::new());
            let ip = id("ip")?;
            let twice = |what: &str| {
                Error::Http(format!(
                    "exchangeInfo: two {what} limits; this adapter charges one"
                ))
            };
            for (kind, name, header, per) in kinds {
                let b = id(&name)?;
                match kind {
                    Kind::Weight if weight.is_some() => return Err(twice("REQUEST_WEIGHT")),
                    Kind::Raw if raw.is_some() => return Err(twice("RAW_REQUESTS")),
                    Kind::Weight => weight = Some(b),
                    Kind::Raw => raw = Some(b),
                    Kind::Orders => orders.push((b, per)),
                }
                if let Some(h) = header {
                    usage.push((h, b));
                }
            }
            let missing = |what: &str| Error::Http(format!("exchangeInfo: no {what} limit"));
            let weight = weight.ok_or_else(|| missing("REQUEST_WEIGHT"))?;
            if orders.is_empty() {
                return Err(missing("ORDERS"));
            }
            Ok((
                Self {
                    ip,
                    weight,
                    raw,
                    orders,
                    usage,
                },
                budget,
            ))
        }

        /// The IP scope: every request and every connect charges it; a ban
        /// halts it.
        #[must_use]
        pub const fn ip(&self) -> BucketId {
            self.ip
        }

        /// The `REQUEST_WEIGHT` count.
        #[must_use]
        pub const fn weight(&self) -> BucketId {
            self.weight
        }

        /// A WebSocket connect: the IP gate, the bucket
        /// [`crate::reconnect::Reconnect::poll_gated`] and
        /// [`crate::client::ClientHandler::connect_gate`] charge. The same
        /// gate a ban halts, so a banned IP cannot reconnect either.
        #[must_use]
        pub const fn connect(&self) -> Parts {
            Parts::new(Charge::new(self.ip, 1))
        }

        /// Any request but a new order (a cancel, a query): the IP gate and
        /// its weight.
        ///
        /// # Errors
        /// More buckets than one charge may name.
        pub fn request(&self, weight: u32) -> Result<Parts> {
            let mut p = Parts::new(Charge::new(self.ip, 1));
            p.push(Charge::new(self.weight, weight))?;
            if let Some(raw) = self.raw {
                p.push(Charge::new(raw, 1))?;
            }
            Ok(p)
        }

        /// A new order (`POST /api/v3/order`, weight 1): every order bucket too.
        ///
        /// # Errors
        /// More buckets than one charge may name.
        pub fn new_order(&self) -> Result<Parts> {
            let mut p = self.request(1)?;
            for &(b, _) in &self.orders {
                p.push(Charge::new(b, 1))?;
            }
            Ok(p)
        }

        /// What `r` means, and the budget calls it needs.
        ///
        /// Holds and halts are applied before any header is parsed: a
        /// malformed usage header on a 429/418/403-ban is reported, but the
        /// scope is already held or halted (the same order as
        /// [`outcome::generic`]).
        ///
        /// # Errors
        /// A usage header, `Retry-After` or JSON body that cannot be read
        /// (after the hold or halt stands). Treat the request as unknown.
        pub fn on_response(
            &self,
            r: &Response<'_>,
            budget: &mut Budget,
            now: Now,
        ) -> Result<Verdict> {
            // ANCHOR: binance_codes
            match r.status {
                // Bans (403: the WAF limit, 418): the IP scope stops until a
                // person clears it. Before any header: a ban always halts.
                // The gate every request and every connect charges, so one
                // ban stops them all.
                403 | 418 => return super::ban(r, self.ip, BAN_MIN_MS, budget, now),
                _ => {}
            }
            let code = match super::json_body(r)? {
                Some(body) if r.status >= 400 => {
                    body.get("code")?.map(json::Value::as_i64).transpose()?
                }
                _ => None,
            };
            // 409 cancel-replace: -2022 ran nothing (both halves failed);
            // -2021 may have run one half.
            if r.status == 409 {
                return Ok(match code {
                    Some(-2022) => Verdict::NotRun,
                    _ => Verdict::Unknown,
                });
            }
            let verdict = match code {
                Some(-1003) => {
                    // "Too many requests queued." is a queue depth, not the
                    // weight window: filling the weight bucket would stop
                    // everything on the IP for a whole window. The queued
                    // variant holds the raw-requests scope, else a short hold
                    // on weight that does not fill it.
                    if super::contains_ignore_case(r.body, b"queued") {
                        if let Some(raw) = self.raw {
                            super::throttle(r, raw, budget, now)
                        } else {
                            let wait = outcome::retry_after_ms(r, super::utc_ms(now))?.or(Some(0));
                            let until_ns = budget.throttled(self.weight, wait, now.ns)?;
                            Ok(Verdict::Throttled {
                                bucket: self.weight,
                                until_ns,
                            })
                        }
                    } else {
                        super::throttle(r, self.weight, budget, now)
                    }
                }
                Some(-1015) => self.too_many_orders(r, budget, now),
                Some(-1021) => Ok(Verdict::ClockSkew),
                // -1006, -1007 and -1008: -1006/-1007 lost track of the
                // request; spot -1008 is only "server is currently
                // overloaded" and does not say the order failed. All three
                // mean it may have run.
                Some(-1008..=-1006) => Ok(Verdict::Unknown),
                // Status alone: 2xx done, 429 throttled, 4xx rejected, 5xx unknown.
                _ => outcome::generic(r, self.weight, budget, now),
            }?;
            // Server truth, after the hold or halt stands: other senders on
            // this IP or account show up here. A malformed usage header is
            // reported but does not undo the hold.
            for (name, bucket) in &self.usage {
                if let Some(used) = super::usage(r, name)? {
                    budget.observe_used(*bucket, used, now.ns)?;
                }
            }
            Ok(verdict)
            // ANCHOR_END: binance_codes
        }

        /// -1015: hold the order bucket the message names. The documented
        /// format is `"Too many new orders; current limit is %s orders per
        /// %s."` with no spelling given for the interval, so the match is
        /// loose: the interval number and its unit word, any case, singular
        /// or plural (`10 second`, `10 seconds`, `1 DAY`, `1 day`).
        ///
        /// A message that names none holds every order bucket (a wrong guess
        /// would send into a 429, which risks a ban). With Binance's
        /// `Retry-After` the hold is the whole cost: the day bucket is not
        /// marked full, so new orders come back when the server says so
        /// (cancels, which are not orders, keep going meanwhile).
        fn too_many_orders(
            &self,
            r: &Response<'_>,
            budget: &mut Budget,
            now: Now,
        ) -> Result<Verdict> {
            let named = self
                .orders
                .iter()
                .find(|(_, per)| names_interval(r.body, per));
            if let Some(&(b, _)) = named {
                return super::throttle(r, b, budget, now);
            }
            let mut last = Ok(Verdict::NotRun);
            for &(b, _) in &self.orders {
                last = super::throttle(r, b, budget, now);
            }
            last
        }
    }
}

pub mod bybit {
    //! Bybit v5 REST.
    //!
    //! - Scopes: 600 requests per 5 s per IP; per endpoint and account
    //!   (UID), a rolling 1 s limit that the response headers state.
    //! - Errors come as HTTP 200 with a `retCode`.

    use super::{Error, NonZeroU64, Now, Parts, Response, Result, Verdict, json, outcome};
    use ergon_io::limit::{BucketId, Budget, Charge};

    /// "Wait for at least 10 minutes" after `403 access too frequent`.
    pub const BAN_MIN_MS: NonZeroU64 = NonZeroU64::MIN.saturating_add(599_999);

    /// The adapter.
    #[derive(Debug, Clone)]
    pub struct Bybit {
        /// The IP scope: a gate with no count. Every request and every
        /// connect charges it, so a ban that halts it stops them all.
        gate: BucketId,
        /// The IP's own count (600 per 5 s).
        ip: BucketId,
        /// The 429 "system level frequency protection" gate: no count, only
        /// holds. Not this IP's quota.
        system: BucketId,
        endpoints: Vec<(String, BucketId)>,
    }

    impl Bybit {
        /// The adapter and its budget: the IP gate (no count, a ban halts
        /// it), the IP bucket (600 per 5 s, 2 % kept for cancels), a system
        /// gate for HTTP 429, and one rolling 1 s bucket per endpoint, at
        /// the limit given; the headers correct it on the first response.
        ///
        /// # Errors
        /// [`Error::Config`]: a zero limit, or a path given twice.
        pub fn new(endpoints: &[(&str, u32)]) -> Result<(Self, Budget)> {
            let mut specs = vec![
                ergon_io::limit::BucketSpec::gate("ip-gate"),
                super::rolling("ip", 600, 5_000, 100)?.with_reserve(12),
                ergon_io::limit::BucketSpec::gate("system"),
            ];
            for &(path, per_second) in endpoints {
                specs.push(super::rolling(path, per_second, 1_000, 100)?);
            }
            let budget = Budget::new(specs)?;
            let id = |name: &str| {
                budget
                    .id(name)
                    .ok_or_else(|| Error::Config(format!("no bucket {name}")))
            };
            let gate = id("ip-gate")?;
            let ip = id("ip")?;
            let system = id("system")?;
            let endpoints = endpoints
                .iter()
                .map(|&(path, _)| Ok((path.to_owned(), id(path)?)))
                .collect::<Result<_>>()?;
            Ok((
                Self {
                    gate,
                    ip,
                    system,
                    endpoints,
                },
                budget,
            ))
        }

        /// The IP scope: every request and every connect charges it; a ban
        /// halts it.
        #[must_use]
        pub const fn gate(&self) -> BucketId {
            self.gate
        }

        /// The IP's own count (600 per 5 s). A ban does not name it.
        #[must_use]
        pub const fn ip(&self) -> BucketId {
            self.ip
        }

        /// The 429 system gate: a hold, not this IP's count.
        #[must_use]
        pub const fn system(&self) -> BucketId {
            self.system
        }

        /// The bucket of endpoint `path`.
        #[must_use]
        pub fn endpoint(&self, path: &str) -> Option<BucketId> {
            self.endpoints
                .iter()
                .find(|(p, _)| p == path)
                .map(|&(_, b)| b)
        }

        /// A WebSocket connect: the IP gate, the bucket
        /// [`crate::reconnect::Reconnect::poll_gated`] and
        /// [`crate::client::ClientHandler::connect_gate`] charge. The same
        /// gate a ban halts, so a banned IP cannot reconnect either.
        #[must_use]
        pub const fn connect(&self) -> Parts {
            Parts::new(Charge::new(self.gate, 1))
        }

        /// A request to `path`: the IP gate, the IP count and the endpoint.
        ///
        /// # Errors
        /// [`Error::Config`]: an endpoint this adapter was not given.
        pub fn request(&self, path: &str) -> Result<Parts> {
            let endpoint = self
                .endpoint(path)
                .ok_or_else(|| Error::Config(format!("no bucket for {path}")))?;
            let mut p = Parts::new(Charge::new(self.gate, 1));
            p.push(Charge::new(self.ip, 1))?;
            p.push(Charge::new(endpoint, 1))?;
            Ok(p)
        }

        /// What `r` (a response from `endpoint`) means.
        ///
        /// Holds and halts are applied before any header is parsed.
        ///
        /// # Errors
        /// A header or body that cannot be read, or more requests left than
        /// the limit (after the hold or halt stands). Treat the request as
        /// unknown.
        pub fn on_response(
            &self,
            endpoint: BucketId,
            r: &Response<'_>,
            budget: &mut Budget,
            now: Now,
        ) -> Result<Verdict> {
            match r.status {
                // A 403 is an IP ban ("403, access too frequent"). The other
                // documented 403s (a GET with an empty JSON body, a U.S. IP)
                // are caller errors or permanent: halting on every 403 is
                // the safe side, and a person clears it. An HTML or
                // reworded ban page is caught here too. Halts first, on the
                // IP gate every request and every connect charges, so one
                // ban stops them all.
                403 => return super::ban(r, self.gate, BAN_MIN_MS, budget, now),
                // "System level frequency protection": no quota of this IP
                // is named. Hold the system gate, not the IP count.
                429 => return super::throttle(r, self.system, budget, now),
                200..=299 => {}
                _ => return outcome::generic(r, endpoint, budget, now),
            }
            let limit = super::usage(r, "X-Bapi-Limit")?;
            if let Some(limit) = limit {
                budget.set_limit(endpoint, limit)?;
            }
            if let Some(left) = super::usage(r, "X-Bapi-Limit-Status")? {
                let limit = match limit {
                    Some(l) => l,
                    None => budget.stats(endpoint, now.ns)?.limit,
                };
                let used = limit.checked_sub(left).ok_or_else(|| {
                    Error::Http(format!(
                        "X-Bapi-Limit-Status {left} is above the limit {limit}"
                    ))
                })?;
                budget.observe_used(endpoint, used, now.ns)?;
            }
            let body = super::json_body(r)?
                .ok_or_else(|| Error::Http("Bybit: a 2xx body that is not JSON".into()))?;
            let code = body
                .get("retCode")?
                .ok_or_else(|| Error::Http("Bybit: no retCode".into()))?
                .as_i64()?;
            Ok(match code {
                0 => Verdict::Done,
                10006 => {
                    // Held until the endpoint's reset, measured on the
                    // response's own clock (`time`) when it has one.
                    let server_ms = body.get("time")?.map(json::Value::as_i64).transpose()?;
                    let wait = outcome::header_u64(r, "X-Bapi-Limit-Reset-Timestamp")?
                        .map(|reset| -> Result<u64> {
                            let reset = i64::try_from(reset).map_err(|_| {
                                Error::Http("X-Bapi-Limit-Reset-Timestamp out of range".into())
                            })?;
                            Ok(outcome::ms_until_epoch(
                                reset,
                                server_ms.unwrap_or_else(|| super::utc_ms(now)),
                            ))
                        })
                        .transpose()?;
                    Verdict::Throttled {
                        bucket: endpoint,
                        until_ns: budget.throttled(endpoint, wait, now.ns)?,
                    }
                }
                10018 => Verdict::Throttled {
                    bucket: self.ip,
                    until_ns: budget.throttled(self.ip, None, now.ns)?,
                },
                10002 => Verdict::ClockSkew,
                10000 | 10016 => Verdict::Unknown, // server timeout, server error
                _ => Verdict::Rejected,
            })
        }
    }
}

pub mod okx {
    //! OKX v5 REST.
    //!
    //! - Scopes: place, cancel and amend each 60 per 2 s per instrument;
    //!   new and amended orders 1,000 per 2 s per sub-account; batch orders
    //!   300 per 2 s per instrument (each order in the batch counts).
    //!   REST and WebSocket share them.
    //! - Options share one bucket per instrument **family**, not per
    //!   instrument ("Rate limit rule (except Options): User ID + Instrument
    //!   ID; (Options only): User ID + Instrument Family"): pass the family
    //!   as `inst_id` to [`Okx::add_instrument`].
    //! - No usage headers: only the local count, so model the limits
    //!   exactly. Cancels have buckets of their own.
    //! - Codes come in the body as text (`"50011"`). 50011 is a generic
    //!   "rate limit reached" and names no bucket: this holds the scope the
    //!   request charged. If a different bucket fired, it keeps sending —
    //!   model the limits exactly and watch the venue's own counters.

    use super::{Error, Now, Parts, Response, Result, Verdict, outcome};
    use ergon_io::limit::{BucketId, Budget, Charge};

    /// Which order endpoint.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum Op {
        /// `POST /api/v5/trade/order`.
        Place,
        /// `POST /api/v5/trade/cancel-order`.
        Cancel,
        /// `POST /api/v5/trade/amend-order`.
        Amend,
        /// `POST /api/v5/trade/batch-orders`: 300 orders per 2 s per
        /// instrument, each order in the batch counted.
        Batch,
    }

    /// The adapter.
    #[derive(Debug, Clone)]
    pub struct Okx {
        sub_account: BucketId,
        /// Instrument (or option family), then its place, cancel, amend and
        /// batch buckets.
        instruments: Vec<(String, [BucketId; 4])>,
    }

    impl Okx {
        /// The adapter and its budget, with the sub-account bucket.
        ///
        /// # Errors
        /// [`Error::Config`] from the budget.
        pub fn new() -> Result<(Self, Budget)> {
            let budget = Budget::new(vec![super::rolling("sub-account", 1_000, 2_000, 100)?])?;
            let sub_account = budget
                .id("sub-account")
                .ok_or_else(|| Error::Config("no sub-account bucket".into()))?;
            Ok((
                Self {
                    sub_account,
                    instruments: Vec::new(),
                },
                budget,
            ))
        }

        /// Buckets for an instrument (or an option family): at start, or when
        /// one is listed. Allocates: not on the send path.
        ///
        /// # Errors
        /// [`Error::Config`]: the instrument was added before.
        pub fn add_instrument(&mut self, budget: &mut Budget, inst_id: &str) -> Result<()> {
            let mut ids = [self.sub_account; 4];
            for (slot, (op, limit)) in
                ids.iter_mut()
                    .zip([("place", 60), ("cancel", 60), ("amend", 60), ("batch", 300)])
            {
                *slot = budget.add(super::rolling(
                    format!("{op} {inst_id}"),
                    limit,
                    2_000,
                    100,
                )?)?;
            }
            self.instruments.push((inst_id.to_owned(), ids));
            Ok(())
        }

        /// The sub-account scope.
        #[must_use]
        pub const fn sub_account(&self) -> BucketId {
            self.sub_account
        }

        /// The bucket of `op` on `inst_id`.
        #[must_use]
        pub fn bucket(&self, op: Op, inst_id: &str) -> Option<BucketId> {
            let (_, ids) = self.instruments.iter().find(|(i, _)| i == inst_id)?;
            ids.get(op as usize).copied()
        }

        /// One order request: its endpoint on that instrument, and the
        /// sub-account for a new or amended order. Not for
        /// [`Op::Batch`]: use [`Okx::batch_request`].
        ///
        /// # Errors
        /// [`Error::Config`]: an instrument not added.
        pub fn request(&self, op: Op, inst_id: &str) -> Result<Parts> {
            let b = self
                .bucket(op, inst_id)
                .ok_or_else(|| Error::Config(format!("no buckets for {inst_id}")))?;
            let mut p = Parts::new(Charge::new(b, 1));
            if op != Op::Cancel {
                p.push(Charge::new(self.sub_account, 1))?;
            }
            Ok(p)
        }

        /// One batch-orders request of `orders` orders on `inst_id`: the
        /// batch bucket is charged `orders` (the venue counts each order),
        /// and the sub-account one for the new orders. A batch larger than
        /// the bucket's limit is `Why::Never`: split it.
        ///
        /// # Errors
        /// [`Error::Config`]: an instrument not added, or `orders` is 0.
        pub fn batch_request(&self, inst_id: &str, orders: u32) -> Result<Parts> {
            if orders == 0 {
                return Err(Error::Config("a batch of 0 orders".into()));
            }
            let b = self
                .bucket(Op::Batch, inst_id)
                .ok_or_else(|| Error::Config(format!("no buckets for {inst_id}")))?;
            let mut p = Parts::new(Charge::new(b, orders));
            p.push(Charge::new(self.sub_account, 1))?;
            Ok(p)
        }

        /// What `r` (a response to a request on `scope`, its endpoint and
        /// instrument bucket) means.
        ///
        /// # Errors
        /// A body or `Retry-After` that cannot be read (holds are applied
        /// first). Treat the request as unknown.
        pub fn on_response(
            &self,
            scope: BucketId,
            r: &Response<'_>,
            budget: &mut Budget,
            now: Now,
        ) -> Result<Verdict> {
            let Some(body) = super::json_body(r)? else {
                return outcome::generic(r, scope, budget, now);
            };
            let code = body
                .get("code")?
                .ok_or_else(|| Error::Http("OKX: no code".into()))?
                .code()?;
            match code {
                0 => Ok(Verdict::Done),
                50011 => super::throttle(r, scope, budget, now),
                50061 => super::throttle(r, self.sub_account, budget, now),
                50102 => Ok(Verdict::ClockSkew),
                // 50004: "does not mean that the request was successful or
                // failed"; 2: a batch partly ran; a 5xx: may have run.
                50004 | 2 => Ok(Verdict::Unknown),
                _ if r.status >= 500 => Ok(Verdict::Unknown),
                _ => Ok(Verdict::Rejected),
            }
        }
    }
}

/// One order the application sent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Order {
    /// Unique per order: without it an unknown outcome cannot be resolved.
    pub client_order_id: u64,
    /// What it charges.
    pub parts: Parts,
    /// `Class::Reduce` for cancels and reduce-only orders.
    pub class: Class,
    /// After this (the budget's clock) it is stale: never sent again.
    pub deadline_ns: i64,
}

/// What to do next with an order, after its verdict.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Next {
    /// It ran: apply the response.
    Applied,
    /// Rejected, or stale: drop it, nothing charged.
    Dropped,
    /// Send it again now, re-signed with a new timestamp: the budget took it.
    SendNow(Ticket),
    /// Ask again at this time; it is still inside its deadline.
    RetryAt(i64),
    /// It may have run: look it up by this client order id. Never re-send.
    Reconcile(u64),
    /// The venue banned this scope: alert a person. Nothing goes out on it
    /// until they clear the halt.
    Halted(BucketId),
}

// ANCHOR: next
/// The re-send rules: only what did not run, through the budget, inside
/// its own deadline; never an order that may have run.
///
/// [`Budget::resend`] makes that last rule structural: it answers
/// `Reconcile` for an unknown outcome instead of a re-send.
#[must_use]
pub fn next(order: &Order, verdict: Verdict, budget: &mut Budget, now_ns: i64) -> Next {
    match verdict {
        Verdict::Done => Next::Applied,
        Verdict::Rejected => Next::Dropped,
        Verdict::Banned { bucket, .. } => Next::Halted(bucket),
        _ => match budget.resend(
            verdict,
            order.parts.as_slice(),
            order.class,
            order.deadline_ns,
            now_ns,
        ) {
            Resend::Send(ticket) => Next::SendNow(ticket),
            Resend::Wait(limited) => Next::RetryAt(limited.ready_at_ns),
            Resend::Stale(_) => Next::Dropped,
            Resend::Reconcile => Next::Reconcile(order.client_order_id),
        },
    }
}
// ANCHOR_END: next

/// Calls `alert` for every halted bucket, with its name and counters, and
/// returns how many. Cheap (one pass over the buckets): run it every loop
/// pass, or after every `Banned` verdict.
///
/// # Errors
/// [`Error::Config`]: never for ids from `budget.halted()`.
pub fn alerts(
    budget: &Budget,
    now_ns: i64,
    mut alert: impl FnMut(&str, &BucketStats),
) -> Result<usize> {
    let mut n = 0;
    for b in budget.halted() {
        alert(budget.name(b).unwrap_or("?"), &budget.stats(b, now_ns)?);
        n += 1;
    }
    Ok(n)
}

/// A recorded response: status, headers and body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Recorded {
    /// The status code.
    pub status: u16,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

impl Recorded {
    /// Calls `f` with this response as the HTTP client delivers it.
    pub fn with<T>(&self, f: impl FnOnce(&Response<'_>) -> T) -> T {
        let headers: Vec<_> = self
            .headers
            .iter()
            .map(|(name, value)| httparse::Header {
                name: name.as_str(),
                value: value.as_bytes(),
            })
            .collect();
        f(&Response {
            status: self.status,
            headers: &headers,
            body: &self.body,
        })
    }
}

/// Every `### name` entry of a fixture file. Lines starting with `#`
/// before an entry's status line are comments.
///
/// # Errors
/// [`Error::Http`]: an entry without a status line, or a header line
/// without a colon.
pub fn recorded(file: &str) -> Result<Vec<(String, Recorded)>> {
    let bad = |what: String| Error::Http(format!("fixture: {what}"));
    let mut out = Vec::new();
    for entry in file.split("\n### ").skip(1) {
        let (name, rest) = entry.split_once('\n').unwrap_or((entry, ""));
        let mut lines = rest.lines().skip_while(|l| l.starts_with('#'));
        let status_line = lines
            .next()
            .ok_or_else(|| bad(format!("{name}: no status line")))?;
        let status = status_line
            .split(' ')
            .nth(1)
            .and_then(|s| s.parse().ok())
            .ok_or_else(|| bad(format!("{name}: bad status line {status_line:?}")))?;
        let mut headers = Vec::new();
        for line in lines.by_ref().take_while(|l| !l.is_empty()) {
            let (n, v) = line
                .split_once(':')
                .ok_or_else(|| bad(format!("{name}: bad header {line:?}")))?;
            headers.push((n.trim().to_owned(), v.trim().to_owned()));
        }
        let body = lines
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .as_bytes()
            .to_vec();
        out.push((
            name.trim().to_owned(),
            Recorded {
                status,
                headers,
                body,
            },
        ));
    }
    Ok(out)
}

/// Entry `name` of a fixture file.
///
/// # Errors
/// [`Error::Http`]: the file does not parse, or has no such entry.
pub fn fixture(file: &str, name: &str) -> Result<Recorded> {
    recorded(file)?
        .into_iter()
        .find(|(n, _)| n == name)
        .map(|(_, r)| r)
        .ok_or_else(|| Error::Http(format!("fixture: no entry {name}")))
}

/// Every recorded response through its venue's adapter, each on a fresh
/// budget: the verdict, then an alert line for each halted bucket.
///
/// # Errors
/// A fixture does not parse, or writing fails.
pub fn replay(out: &mut impl Write) -> std::result::Result<(), Box<dyn std::error::Error>> {
    // The budget's clock (monotonic) and UTC, when each response arrives.
    let now = Now {
        ns: 1_000_000_000,
        real_ns: 1_700_000_000_000_000_000,
    };
    let limits = binance::limits(BINANCE_EXCHANGE_INFO.as_bytes())?;
    for (name, r) in recorded(BINANCE)? {
        let (b, mut budget) = binance::Binance::new(limits.clone())?;
        let v = r.with(|r| b.on_response(r, &mut budget, now));
        report(out, "binance", &name, &v, &budget, now.ns)?;
    }
    for (name, r) in recorded(BYBIT)? {
        let (b, mut budget) = bybit::Bybit::new(&[("/v5/order/create", 10)])?;
        let create = b.endpoint("/v5/order/create").ok_or("no endpoint")?;
        let v = r.with(|r| b.on_response(create, r, &mut budget, now));
        report(out, "bybit", &name, &v, &budget, now.ns)?;
    }
    for (name, r) in recorded(OKX)? {
        let (mut o, mut budget) = okx::Okx::new()?;
        o.add_instrument(&mut budget, "BTC-USDT")?;
        let place = o.bucket(okx::Op::Place, "BTC-USDT").ok_or("no bucket")?;
        let v = r.with(|r| o.on_response(place, r, &mut budget, now));
        report(out, "okx", &name, &v, &budget, now.ns)?;
    }
    Ok(())
}

fn report(
    out: &mut impl Write,
    venue: &str,
    name: &str,
    verdict: &Result<Verdict>,
    budget: &Budget,
    now_ns: i64,
) -> std::result::Result<(), Box<dyn std::error::Error>> {
    let held = |bucket: BucketId, until_ns: i64| {
        let secs =
            Duration::from_nanos(u64::try_from(until_ns - now_ns).unwrap_or(0)).as_secs_f64();
        format!("{} for {secs:.1} s", budget.name(bucket).unwrap_or("?"))
    };
    match verdict {
        Ok(Verdict::Throttled { bucket, until_ns }) => {
            writeln!(
                out,
                "{venue} {name}: Throttled: holds {}",
                held(*bucket, *until_ns)
            )?;
        }
        Ok(Verdict::Banned { bucket, until_ns }) => {
            writeln!(
                out,
                "{venue} {name}: Banned: halts {}, then until cleared",
                held(*bucket, *until_ns)
            )?;
        }
        Ok(v) => writeln!(out, "{venue} {name}: {v:?}")?,
        Err(e) => writeln!(
            out,
            "{venue} {name}: cannot read it ({e}): treat as Unknown"
        )?,
    }
    let mut alerts_out = Vec::new();
    alerts(budget, now_ns, |bucket, stats| {
        alerts_out.push(format!(
            "  ALERT: {bucket} halted after {} ban(s): stopped until a person clears it",
            stats.bans
        ));
    })?;
    for line in alerts_out {
        writeln!(out, "{line}")?;
    }
    Ok(())
}

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    replay(&mut std::io::stdout().lock())
}

pub mod json {
    //! A strict JSON reader over a borrowed body: it finds fields without
    //! allocating or building a tree. Enough for these adapters; an
    //! application uses its own parser.

    use super::{Error, Result};

    /// Deeper nesting is refused (a stack bound against hostile input).
    const MAX_DEPTH: u32 = 64;

    /// One JSON value: its bytes as sent, without surrounding whitespace.
    /// Only [`Value::parse`] makes one, so it is always valid JSON.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Value<'a>(&'a [u8]);

    impl<'a> Value<'a> {
        /// `body` as one JSON value, whitespace around it allowed.
        ///
        /// # Errors
        /// [`Error::Http`]: not UTF-8, or not exactly one valid value.
        pub fn parse(body: &'a [u8]) -> Result<Self> {
            std::str::from_utf8(body).map_err(|_| bad("not UTF-8"))?;
            let start = ws(body, 0);
            let end = skip(body, start, 0)?;
            if ws(body, end) != body.len() {
                return Err(bad("bytes after the value"));
            }
            Ok(Self(body.get(start..end).unwrap_or_default()))
        }

        /// Field `key` of this object; `Ok(None)` if it has none.
        ///
        /// # Errors
        /// [`Error::Http`]: not an object, or `key` appears twice.
        pub fn get(self, key: &str) -> Result<Option<Self>> {
            let mut found = None;
            for (k, v) in self.members(b'{')? {
                if k == key.as_bytes() {
                    if found.is_some() {
                        return Err(bad("a key appears twice"));
                    }
                    found = Some(v);
                }
            }
            Ok(found)
        }

        /// The items of this array. Allocates: for start-up parsing.
        ///
        /// # Errors
        /// [`Error::Http`]: not an array.
        pub fn items(self) -> Result<Vec<Self>> {
            Ok(self.members(b'[')?.map(|(_, v)| v).collect())
        }

        /// An integer: no fraction, no exponent, within `i64`.
        ///
        /// # Errors
        /// [`Error::Http`]: anything else.
        pub fn as_i64(self) -> Result<i64> {
            let (neg, digits) = match self.0.split_first() {
                Some((b'-', rest)) => (true, rest),
                _ => (false, self.0),
            };
            if digits.is_empty() || !digits.iter().all(u8::is_ascii_digit) {
                return Err(bad("not an integer"));
            }
            digits
                .iter()
                .try_fold(0_i64, |n, &d| {
                    let d = i64::from(d - b'0');
                    n.checked_mul(10)?.checked_add(if neg { -d } else { d })
                })
                .ok_or_else(|| bad("integer out of range"))
        }

        /// A string's text. Escapes are refused: these adapters compare
        /// plain names and codes only.
        ///
        /// # Errors
        /// [`Error::Http`]: not a string, or one with an escape.
        pub fn as_str(self) -> Result<&'a str> {
            let inner = self
                .0
                .strip_prefix(b"\"")
                .and_then(|s| s.strip_suffix(b"\""))
                .ok_or_else(|| bad("not a string"))?;
            if inner.contains(&b'\\') {
                return Err(bad("an escaped string"));
            }
            std::str::from_utf8(inner).map_err(|_| bad("not UTF-8"))
        }

        /// A venue code: an integer, or a string holding one (OKX `"50011"`).
        ///
        /// # Errors
        /// [`Error::Http`]: neither.
        pub fn code(self) -> Result<i64> {
            if self.0.first() == Some(&b'"') {
                let text = self.as_str()?;
                Self(text.as_bytes()).as_i64()
            } else {
                self.as_i64()
            }
        }

        /// An object's (key, value) pairs, or an array's items (empty keys).
        fn members(self, open: u8) -> Result<Members<'a>> {
            if self.0.first() != Some(&open) {
                return Err(bad(if open == b'{' {
                    "not an object"
                } else {
                    "not an array"
                }));
            }
            Ok(Members {
                s: self.0,
                i: 1,
                object: open == b'{',
                done: false,
            })
        }
    }

    /// Walks a value [`Value::parse`] already checked, so no step can fail;
    /// one that did would end the walk early.
    struct Members<'a> {
        s: &'a [u8],
        i: usize,
        object: bool,
        done: bool,
    }

    impl<'a> Iterator for Members<'a> {
        type Item = (&'a [u8], Value<'a>);

        fn next(&mut self) -> Option<Self::Item> {
            if self.done {
                return None;
            }
            let (s, close) = (self.s, if self.object { b'}' } else { b']' });
            let mut i = ws(s, self.i);
            if s.get(i) == Some(&close) {
                self.done = true;
                return None;
            }
            let mut key: &[u8] = &[];
            if self.object {
                let end = string_end(s, i).ok()?;
                key = s.get(i + 1..end - 1)?;
                i = ws(s, ws(s, end) + 1); // past ':'
            }
            let end = skip(s, i, 1).ok()?;
            let value = Value(s.get(i..end)?);
            i = ws(s, end);
            if s.get(i) == Some(&close) {
                self.done = true;
            } else {
                self.i = i + 1; // past ','
            }
            Some((key, value))
        }
    }

    fn bad(what: &str) -> Error {
        Error::Http(format!("malformed JSON body: {what}"))
    }

    fn ws(s: &[u8], mut i: usize) -> usize {
        while matches!(s.get(i), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            i += 1;
        }
        i
    }

    /// The end of the value at `i`.
    fn skip(s: &[u8], i: usize, depth: u32) -> Result<usize> {
        if depth > MAX_DEPTH {
            return Err(bad("nested too deep"));
        }
        match s.get(i) {
            Some(b'{') => seq(s, i + 1, b'}', depth, true),
            Some(b'[') => seq(s, i + 1, b']', depth, false),
            Some(b'"') => string_end(s, i),
            Some(b't') => word(s, i, b"true"),
            Some(b'f') => word(s, i, b"false"),
            Some(b'n') => word(s, i, b"null"),
            Some(b'-' | b'0'..=b'9') => number_end(s, i),
            _ => Err(bad("expected a value")),
        }
    }

    /// An object's members or an array's items, from just after the
    /// opening bracket: the end after the closing one.
    fn seq(s: &[u8], i: usize, close: u8, depth: u32, object: bool) -> Result<usize> {
        let mut i = ws(s, i);
        if s.get(i) == Some(&close) {
            return Ok(i + 1);
        }
        loop {
            if object {
                if s.get(i) != Some(&b'"') {
                    return Err(bad("expected a key"));
                }
                i = ws(s, string_end(s, i)?);
                if s.get(i) != Some(&b':') {
                    return Err(bad("expected ':'"));
                }
                i = ws(s, i + 1);
            }
            i = ws(s, skip(s, i, depth + 1)?);
            match s.get(i) {
                Some(b',') => i = ws(s, i + 1),
                Some(&c) if c == close => return Ok(i + 1),
                _ => return Err(bad("expected ',' or a closing bracket")),
            }
        }
    }

    /// The end of the string at `i` (after its closing quote).
    fn string_end(s: &[u8], i: usize) -> Result<usize> {
        if s.get(i) != Some(&b'"') {
            return Err(bad("expected a string"));
        }
        let mut j = i + 1;
        loop {
            match s.get(j) {
                Some(b'"') => return Ok(j + 1),
                Some(b'\\') => match s.get(j + 1) {
                    Some(b'"' | b'\\' | b'/' | b'b' | b'f' | b'n' | b'r' | b't') => j += 2,
                    Some(b'u')
                        if s.get(j + 2..j + 6)
                            .is_some_and(|h| h.iter().all(u8::is_ascii_hexdigit)) =>
                    {
                        j += 6;
                    }
                    _ => return Err(bad("a bad escape")),
                },
                Some(&c) if c < 0x20 => return Err(bad("a control character in a string")),
                Some(_) => j += 1,
                None => return Err(bad("an unterminated string")),
            }
        }
    }

    fn word(s: &[u8], i: usize, w: &[u8]) -> Result<usize> {
        if s.get(i..i + w.len()) == Some(w) {
            Ok(i + w.len())
        } else {
            Err(bad("expected true, false or null"))
        }
    }

    fn digits(s: &[u8], mut j: usize) -> usize {
        while s.get(j).is_some_and(u8::is_ascii_digit) {
            j += 1;
        }
        j
    }

    /// The end of the number at `i`: `-?(0|[1-9][0-9]*)(.[0-9]+)?([eE][+-]?[0-9]+)?`.
    fn number_end(s: &[u8], i: usize) -> Result<usize> {
        let mut j = i;
        if s.get(j) == Some(&b'-') {
            j += 1;
        }
        j = match s.get(j) {
            Some(b'0') => j + 1,
            Some(b'1'..=b'9') => digits(s, j),
            _ => return Err(bad("a bad number")),
        };
        if s.get(j) == Some(&b'.') {
            let k = digits(s, j + 1);
            if k == j + 1 {
                return Err(bad("a bad number"));
            }
            j = k;
        }
        if matches!(s.get(j), Some(b'e' | b'E')) {
            j += 1;
            if matches!(s.get(j), Some(b'+' | b'-')) {
                j += 1;
            }
            let k = digits(s, j);
            if k == j {
                return Err(bad("a bad number"));
            }
            j = k;
        }
        Ok(j)
    }
}

//! The engine's logic: each instrument's order book from the feeds, their
//! aggregate per asset, the EMAs and the strategy, and [`agent::Engine`], the
//! runtime agent that runs them live, in replay and in a backtest.

pub mod agent;
pub mod replay;

use std::collections::BTreeMap;

use arrayvec::ArrayVec;

/// Decimal9 mantissas per unit.
pub const SCALE: f64 = 1e9;

/// The EMA horizons, in seconds: 5m, 30m, 1h, 4h, 12h, 1d.
pub const HORIZONS: [f64; 6] = [300.0, 1_800.0, 3_600.0, 14_400.0, 43_200.0, 86_400.0];

/// How to read an instrument's sizes: its `InstrumentSpec` from the feed.
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    /// Its base currency (BTC): what the engine aggregates by.
    pub asset: String,
    pub multiplier: f64,
    /// Sized in quote currency (Deribit's USD perpetuals).
    pub inverse: bool,
}

impl Spec {
    /// `size` at `price`, in base quantity. USDT, USDC and USD quotes are
    /// taken as one currency.
    #[must_use]
    pub fn base(&self, size: f64, price: f64) -> f64 {
        if self.inverse {
            size * self.multiplier / price
        } else {
            size * self.multiplier
        }
    }
}

/// A book change, as the md feed sends it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    /// A level's size (0 removes it).
    Set {
        bid: bool,
        price: i64,
        size: i64,
    },
    Delete {
        bid: bool,
        price: i64,
    },
    Clear,
}

/// One instrument's L2 book: Decimal9 mantissas, price to size.
#[derive(Debug, Default)]
pub struct Book {
    pub bids: BTreeMap<i64, i64>,
    pub asks: BTreeMap<i64, i64>,
    /// Built from a snapshot of the current session: usable.
    pub synced: bool,
}

impl Book {
    /// Replace it with a snapshot's levels.
    pub fn snapshot(
        &mut self,
        bids: impl IntoIterator<Item = (i64, i64)>,
        asks: impl IntoIterator<Item = (i64, i64)>,
    ) {
        self.bids.clear();
        self.asks.clear();
        self.bids.extend(bids);
        self.asks.extend(asks);
        self.synced = true;
    }

    pub fn apply(&mut self, change: Change) {
        match change {
            Change::Set { bid, price, size } if size > 0 => {
                self.side(bid).insert(price, size);
            }
            Change::Set { bid, price, .. } | Change::Delete { bid, price } => {
                self.side(bid).remove(&price);
            }
            Change::Clear => {
                self.bids.clear();
                self.asks.clear();
            }
        }
    }

    fn side(&mut self, bid: bool) -> &mut BTreeMap<i64, i64> {
        if bid { &mut self.bids } else { &mut self.asks }
    }

    /// Forget it until the next snapshot: its publisher restarted.
    pub fn reset(&mut self) {
        self.bids.clear();
        self.asks.clear();
        self.synced = false;
    }

    #[must_use]
    pub fn best_bid(&self) -> Option<i64> {
        self.synced.then(|| self.bids.keys().next_back().copied())?
    }

    #[must_use]
    pub fn best_ask(&self) -> Option<i64> {
        self.synced.then(|| self.asks.keys().next().copied())?
    }
}

/// One side's best `N` levels across books, best first: `(price mantissa,
/// base size, venue)`. `books` are `(spec, book, venue)`. On the stack: each
/// level goes into place as it comes, and a book's side stops at the first
/// level that cannot make the top `N`. Equal prices keep the books' order.
#[must_use]
pub fn aggregate<'a, const N: usize>(
    books: impl Iterator<Item = (&'a Spec, &'a Book, &'a str)>,
    bid: bool,
) -> ArrayVec<(i64, f64, &'a str), N> {
    let mut top = ArrayVec::new();
    let better = |a: i64, b: i64| if bid { a > b } else { a < b };
    for (spec, book, venue) in books.filter(|(_, book, _)| book.synced) {
        let mut add = |(&price, &size): (&i64, &i64)| {
            if top.is_full()
                && !top
                    .last()
                    .is_some_and(|l: &(i64, f64, &str)| better(price, l.0))
            {
                return false;
            }
            let at = top
                .iter()
                .position(|l| better(price, l.0))
                .unwrap_or(top.len());
            if top.is_full() {
                top.pop();
            }
            let p = price as f64 / SCALE;
            top.insert(at, (price, spec.base(size as f64 / SCALE, p), venue));
            true
        };
        if bid {
            book.bids.iter().rev().take(N).all(&mut add);
        } else {
            book.asks.iter().take(N).all(&mut add);
        }
    }
    top
}

/// Time-decayed EMAs over [`HORIZONS`]: `α = 1 − e^(−Δt/τ)`, so each moves
/// on every update however irregular.
#[derive(Clone, Copy, Debug, Default)]
pub struct Emas {
    pub values: [f64; 6],
    last_ns: Option<i64>,
}

impl Emas {
    pub fn update(&mut self, now_ns: i64, x: f64) {
        match self.last_ns {
            None => self.values = [x; 6],
            Some(last) => {
                let dt = (now_ns - last).max(0) as f64 / 1e9;
                for (v, tau) in self.values.iter_mut().zip(HORIZONS) {
                    // `libm`, not the platform's: the same bits on every host, so a
                    // backtest gives the same result wherever it runs.
                    *v += (1.0 - libm::exp(-dt / tau)) * (x - *v);
                }
            }
        }
        self.last_ns = Some(now_ns);
    }
}

/// The strategy, per asset. A trade signal is the mid crossing its 5m EMA;
/// it is taken when it agrees with the trend (5m EMA against 30m) or
/// reduces the position, at most one order per [`Strategy::THROTTLE_NS`],
/// never past [`Strategy::CAP`].
#[derive(Clone, Copy, Debug, Default)]
pub struct Strategy {
    /// The mid was above its 5m EMA at the last update.
    above: Option<bool>,
    last_order_ns: Option<i64>,
    /// Filled, in base quantity.
    pub position: f64,
    /// Cash from fills, in quote currency.
    pub cash: f64,
}

impl Strategy {
    /// Base quantity per order.
    pub const QTY: f64 = 0.001;
    /// Largest position either way.
    pub const CAP: f64 = 0.01;
    pub const THROTTLE_NS: i64 = 30_000_000_000;

    /// `Some(true)` to buy, `Some(false)` to sell.
    pub fn decide(&mut self, now_ns: i64, mid: f64, emas: &Emas) -> Option<bool> {
        let [ema5m, ema30m, ..] = emas.values;
        let above = mid > ema5m;
        if self.above.replace(above) != Some(!above) {
            return None; // no cross
        }
        let buy = above;
        let with_trend = buy == (ema5m > ema30m);
        let reduces = if buy {
            self.position < 0.0
        } else {
            self.position > 0.0
        };
        let next = self.position + if buy { Self::QTY } else { -Self::QTY };
        let throttled = self
            .last_order_ns
            .is_some_and(|t| now_ns - t < Self::THROTTLE_NS);
        if throttled || !(with_trend || reduces) || next.abs() > Self::CAP + 1e-12 {
            return None;
        }
        self.last_order_ns = Some(now_ns);
        Some(buy)
    }

    /// A fill of `qty` at `price`.
    pub fn fill(&mut self, buy: bool, qty: f64, price: f64) {
        let signed = if buy { qty } else { -qty };
        self.position += signed;
        self.cash -= signed * price;
    }

    #[must_use]
    pub fn pnl(&self, mid: f64) -> f64 {
        self.cash + self.position * mid
    }
}

/// How a venue's book `sequence` advances, so a missed message shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GapRule {
    /// Each message is the last plus one.
    Contiguous,
    /// Only a regression or a repeat is a gap: the venue skips numbers
    /// (Binance's diff depth carries `U`..`u` ranges).
    Monotonic,
    /// The venue has no usable sequence.
    None,
}

impl GapRule {
    /// `contiguous`, `monotonic` or `none`.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "contiguous" => Some(Self::Contiguous),
            "monotonic" => Some(Self::Monotonic),
            "none" => Some(Self::None),
            _ => None,
        }
    }

    /// `seq` after `last` breaks the rule. A zero sequence is the venue
    /// sending none, never a gap.
    #[must_use]
    pub const fn is_gap(self, last: u64, seq: u64) -> bool {
        if seq == 0 || last == 0 {
            return false;
        }
        match self {
            Self::Contiguous => seq != last.wrapping_add(1),
            Self::Monotonic => seq <= last,
            Self::None => false,
        }
    }
}

/// Where one instrument's book stands.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum FeedState {
    /// No snapshot yet.
    #[default]
    Initializing,
    /// Current: trade it.
    Active,
    /// A gap: stop trading it until the next snapshot. No incrementals are
    /// buffered; snapshots arrive regularly.
    Recovering,
    /// No update for a while: stop trading it until the next one.
    Stale,
}

/// One instrument's feed state and last sequence.
#[derive(Clone, Copy, Debug, Default)]
pub struct Sequenced {
    pub state: FeedState,
    pub last: u64,
}

impl Sequenced {
    /// A snapshot: current from its sequence on.
    pub const fn snapshot(&mut self, seq: u64) {
        self.state = FeedState::Active;
        self.last = seq;
    }

    /// Incrementals at `seq`: whether to apply them. A gap moves to
    /// `Recovering`; a stale book with no gap is current again.
    pub const fn deltas(&mut self, rule: GapRule, seq: u64) -> bool {
        match self.state {
            FeedState::Initializing | FeedState::Recovering => false,
            FeedState::Active | FeedState::Stale => {
                if rule.is_gap(self.last, seq) {
                    self.state = FeedState::Recovering;
                    return false;
                }
                if seq != 0 {
                    self.last = seq;
                }
                self.state = FeedState::Active;
                true
            }
        }
    }

    /// The stale timer fired: an active book stops trading.
    pub fn stale(&mut self) {
        if self.state == FeedState::Active {
            self.state = FeedState::Stale;
        }
    }

    /// Current enough to trade.
    #[must_use]
    pub fn tradable(&self) -> bool {
        self.state == FeedState::Active
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const fn d(x: i64) -> i64 {
        x * 1_000_000_000
    }

    #[test]
    fn sizes_normalise_to_base_quantity() {
        let spot = Spec {
            asset: "BTC".into(),
            multiplier: 1.0,
            inverse: false,
        };
        let okx = Spec {
            multiplier: 0.01,
            ..spot.clone()
        };
        // Deribit's BTC-PERPETUAL, as the lab's instrument table has it:
        // multiplier 1, sized in USD.
        let deribit = Spec {
            inverse: true,
            ..spot.clone()
        };
        assert_eq!(spot.base(2.0, 50_000.0), 2.0);
        assert!((okx.base(100.0, 50_000.0) - 1.0).abs() < 1e-12);
        assert!((deribit.base(25_000.0, 50_000.0) - 0.5).abs() < 1e-12);
    }

    #[test]
    fn a_book_follows_snapshots_and_changes() {
        let mut book = Book::default();
        book.apply(Change::Set {
            bid: true,
            price: d(1),
            size: d(1),
        });
        assert_eq!(book.best_bid(), None, "unusable before a snapshot");
        book.snapshot([(d(99), d(1)), (d(98), d(2))], [(d(101), d(1))]);
        assert_eq!(
            (book.best_bid(), book.best_ask()),
            (Some(d(99)), Some(d(101)))
        );
        book.apply(Change::Set {
            bid: true,
            price: d(100),
            size: d(3),
        });
        book.apply(Change::Delete {
            bid: false,
            price: d(101),
        });
        book.apply(Change::Set {
            bid: false,
            price: d(102),
            size: d(1),
        });
        assert_eq!(
            (book.best_bid(), book.best_ask()),
            (Some(d(100)), Some(d(102)))
        );
        book.apply(Change::Set {
            bid: true,
            price: d(100),
            size: 0,
        });
        assert_eq!(book.best_bid(), Some(d(99)), "size 0 removes a level");
        book.reset();
        assert_eq!(book.best_bid(), None);
    }

    #[test]
    fn the_aggregate_merges_venues_best_first_in_base_quantity() {
        let spot = Spec {
            asset: "BTC".into(),
            multiplier: 1.0,
            inverse: false,
        };
        let inverse = Spec {
            inverse: true,
            ..spot.clone()
        };
        let (mut a, mut b, unsynced) = (Book::default(), Book::default(), Book::default());
        a.snapshot([(d(100), d(1)), (d(98), d(1))], [(d(101), d(1))]);
        b.snapshot([(d(99), d(990))], [(d(100), d(500))]);
        let books = || {
            [
                (&spot, &a, "A"),
                (&inverse, &b, "B"),
                (&spot, &unsynced, "C"),
            ]
            .into_iter()
        };
        let bids = aggregate::<2>(books(), true);
        assert_eq!(bids.as_slice(), [(d(100), 1.0, "A"), (d(99), 10.0, "B")]);
        let asks = aggregate::<5>(books(), false);
        assert_eq!(asks.as_slice(), [(d(100), 5.0, "B"), (d(101), 1.0, "A")]);
    }

    #[test]
    fn emas_decay_by_time() {
        let mut e = Emas::default();
        e.update(0, 100.0);
        assert_eq!(e.values, [100.0; 6]);
        // One 5m time constant later at 200: 63% of the way.
        e.update(300_000_000_000, 200.0);
        let expected = 100.0 + 100.0 * (1.0 - (-1.0f64).exp());
        assert!((e.values[0] - expected).abs() < 1e-9);
        assert!(e.values[1] < e.values[0] && e.values[5] < e.values[4]);
    }

    #[test]
    fn the_strategy_trades_crosses_with_the_trend_throttled_and_capped() {
        let emas = |fast, slow| Emas {
            values: [fast, slow, 0.0, 0.0, 0.0, 0.0],
            last_ns: Some(0),
        };
        let up = emas(100.0, 90.0);
        let mut s = Strategy::default();
        assert_eq!(
            s.decide(0, 99.0, &up),
            None,
            "the first update only sets the side"
        );
        assert_eq!(
            s.decide(1, 101.0, &up),
            Some(true),
            "crossed up in an uptrend"
        );
        assert_eq!(
            s.decide(2, 99.0, &up),
            None,
            "no sell against the trend while flat"
        );
        assert_eq!(s.decide(3, 101.0, &up), None, "throttled");
        let later = Strategy::THROTTLE_NS + 10;
        assert_eq!(s.decide(later, 99.0, &up), None);
        assert_eq!(s.decide(later + 1, 101.0, &up), Some(true));
        // Long: a sell against the trend reduces, so it is taken.
        s.fill(true, Strategy::QTY, 100.0);
        let later = later + 1 + Strategy::THROTTLE_NS;
        assert_eq!(s.decide(later, 99.0, &up), Some(false));
        // At the cap: no more buys.
        s.position = Strategy::CAP;
        let later = later + Strategy::THROTTLE_NS;
        assert_eq!(s.decide(later, 101.0, &up), None);
    }

    #[test]
    fn pnl_marks_the_position_to_the_mid() {
        let mut s = Strategy::default();
        s.fill(true, 0.5, 100.0);
        s.fill(false, 0.25, 110.0);
        assert!((s.position - 0.25).abs() < 1e-12);
        assert!((s.pnl(120.0) - (-50.0 + 27.5 + 30.0)).abs() < 1e-9);
    }

    #[test]
    fn a_contiguous_venue_recovers_from_a_gap_at_the_next_snapshot() {
        let mut f = Sequenced::default();
        assert!(
            !f.deltas(GapRule::Contiguous, 5),
            "nothing before a snapshot"
        );
        f.snapshot(10);
        assert!(f.deltas(GapRule::Contiguous, 11));
        assert!(!f.deltas(GapRule::Contiguous, 13), "12 was missed");
        assert_eq!(f.state, FeedState::Recovering);
        assert!(!f.deltas(GapRule::Contiguous, 14), "waits for a snapshot");
        f.snapshot(20);
        assert!(f.deltas(GapRule::Contiguous, 21));
        assert!(f.tradable());
    }

    #[test]
    fn a_monotonic_venue_gaps_only_on_a_regression_or_repeat() {
        let mut f = Sequenced::default();
        f.snapshot(100);
        assert!(f.deltas(GapRule::Monotonic, 150));
        assert!(!f.deltas(GapRule::Monotonic, 150));
        assert_eq!(f.state, FeedState::Recovering);
        let mut g = Sequenced::default();
        g.snapshot(100);
        assert!(!g.deltas(GapRule::Monotonic, 99));
    }

    #[test]
    fn no_rule_and_no_sequence_never_gap() {
        let mut f = Sequenced::default();
        f.snapshot(7);
        assert!(f.deltas(GapRule::None, 3));
        assert!(f.deltas(GapRule::Contiguous, 0), "a venue with no sequence");
        assert_eq!(GapRule::parse("monotonic"), Some(GapRule::Monotonic));
        assert_eq!(GapRule::parse("sometimes"), None);
    }

    #[test]
    fn a_stale_book_trades_again_on_its_next_update() {
        let mut f = Sequenced::default();
        f.stale();
        assert_eq!(f.state, FeedState::Initializing, "stale needs a book first");
        f.snapshot(1);
        f.stale();
        assert!(!f.tradable());
        assert!(f.deltas(GapRule::Contiguous, 2));
        assert!(f.tradable());
    }
}

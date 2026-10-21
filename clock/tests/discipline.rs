//! The discipline against a scripted host (spec §10.1): a counter and a
//! wall clock whose rate, steps, read noise, preemptions and step-probe
//! glitches are scripted, read through the real [`Sampler`] and
//! [`Discipline`]. Deterministic: fixed-seed xorshift, and the clocks are
//! made from parameters (no host clock is read but the fallback's
//! resolution, which only labels it).
#![allow(missing_docs)]

use ergon_clock::advanced::discipline::{
    Action, Adjustment, Discipline, DisciplineConfig, Phase, flags,
};
use ergon_clock::advanced::{
    Clock, CounterClock, KIND_COUNTER, KIND_MONOTONIC, MonotonicClock, PARAMS_VERSION, Params,
    SampleSource, Sampler, Scale,
};

type Res = Result<(), Box<dyn std::error::Error>>;

const NS: u64 = 1_000_000_000;
const PPT: i128 = 1_000_000_000_000;
/// Where the scripted wall clock starts: 2026-10-16, ns since the epoch.
const EPOCH_2026: i128 = 1_792_108_800_000_000_000;

/// xorshift64*: deterministic, no dependency.
struct Rng(u64);

impl Rng {
    const fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15) | 1)
    }
    const fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_f491_4f6c_dd1d)
    }
    const fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    /// Uniform in `lo..=hi`.
    fn between(&mut self, lo: i64, hi: i64) -> i64 {
        let span = u64::try_from(i128::from(hi) - i128::from(lo) + 1).unwrap_or(u64::MAX);
        lo.wrapping_add(self.below(span).cast_signed())
    }
    #[allow(clippy::cast_precision_loss)]
    fn unit(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1_u64 << 53) as f64
    }
    fn chance(&mut self, p: f64) -> bool {
        self.unit() < p
    }
    /// About N(0, 1): the sum of 12 uniforms less 6 (no transcendental, so
    /// the same on every platform).
    fn gauss(&mut self) -> f64 {
        (0..12).map(|_| self.unit()).sum::<f64>() - 6.0
    }
    /// Log-uniform in `lo..=hi` (both positive).
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        clippy::cast_precision_loss
    )]
    fn log_uniform(&mut self, lo: u64, hi: u64) -> u64 {
        let (a, b) = ((lo as f64).ln(), (hi as f64).ln());
        (b - a).mul_add(self.unit(), a).exp().round() as u64
    }
}

/// The wall clock as a function of the counter: `CLOCK_REALTIME` runs at
/// the counter's nominal rate plus scripted rates (ppt), and steps;
/// `CLOCK_MONOTONIC` shares its rate (one timekeeper), so their difference
/// moves only on steps of the wall clock, and not at all on a counter jump.
#[derive(Clone, Debug)]
struct Wall {
    hz: u64,
    r0: i128,
    /// (from tick, rate ppt), in tick order.
    rates: Vec<(u64, i64)>,
    /// (at tick, ns, whether `CLOCK_MONOTONIC` moved too: a counter jump).
    steps: Vec<(u64, i64, bool)>,
}

impl Wall {
    const fn new(hz: u64, r0: i128) -> Self {
        Self {
            hz,
            r0,
            rates: Vec::new(),
            steps: Vec::new(),
        }
    }

    /// The counter's nominal ns at `ticks`.
    fn ns(&self, ticks: u64) -> i128 {
        i128::from(ticks) * i128::from(NS) / i128::from(self.hz)
    }

    /// `CLOCK_REALTIME` at `t`.
    fn at(&self, t: u64) -> i128 {
        let mut v = self.r0 + self.ns(t);
        for (i, &(from, rate)) in self.rates.iter().enumerate() {
            if t <= from {
                break;
            }
            let to = self.rates.get(i + 1).map_or(t, |&(next, _)| next.min(t));
            v += (self.ns(to) - self.ns(from)) * i128::from(rate) / PPT;
        }
        for &(at, ns, _) in &self.steps {
            if t >= at {
                v += i128::from(ns);
            }
        }
        v
    }

    /// `CLOCK_REALTIME - CLOCK_MONOTONIC` at `t`: moves on wall steps only.
    fn d(&self, t: u64) -> i128 {
        self.steps
            .iter()
            .filter(|&&(at, _, mono_too)| t >= at && !mono_too)
            .map(|&(_, ns, _)| i128::from(ns))
            .sum::<i128>()
            + 1_000_000_000_000_000_000
    }

    fn mono(&self, t: u64) -> i128 {
        self.at(t) - self.d(t)
    }

    /// The wall clock's rate from `t` on, ppt against the counter.
    fn rate_from(&mut self, t: u64, ppt: i64) {
        self.rates.push((t, ppt));
    }

    /// The wall clock steps by `ns` at `t` (`settimeofday`, `makestep`).
    fn step(&mut self, t: u64, ns: i64) {
        self.steps.push((t, ns, false));
    }

    /// The counter jumps ahead by `ns` at `t` (live migration): seen from
    /// the counter, both kernel clocks step back by `ns`.
    fn jump(&mut self, t: u64, ns: i64) {
        self.steps.push((t, -ns, true));
    }
}

/// A host: the counter, its wall clock, and the costs and accidents of
/// reading them.
struct Host {
    wall: Wall,
    tick: u64,
    rng: Rng,
    /// Ticks a read takes.
    cost: u64,
    /// ns of noise on a wall-clock read's position in its bracket.
    sigma: f64,
    /// ns of noise on a step probe's wall read (the spec's sim: 5 ns).
    probe_sigma: f64,
    /// Whether the step probe exists (Linux).
    linux: bool,
    /// Ticks added to every wall read while set: a sample taken across
    /// interrupts, every bracket wide.
    storm: u64,
    /// The chance one wall read is preempted for 50 µs.
    preempt: f64,
    /// ns added to the step probe's wall reads: a glitched probe.
    glitch: i64,
    monotonic_reads: u64,
}

impl Host {
    fn new(wall: Wall, tick: u64, seed: u64) -> Self {
        let cost = (wall.hz * 25 / NS).max(1);
        Self {
            wall,
            tick,
            rng: Rng::new(seed),
            cost,
            sigma: 15.0,
            probe_sigma: 5.0,
            linux: true,
            storm: 0,
            preempt: 0.0,
            glitch: 0,
            monotonic_reads: 0,
        }
    }

    const fn ticks_per_s(&self) -> u64 {
        self.wall.hz
    }
}

impl SampleSource for Host {
    fn ticks_ordered(&mut self) -> u64 {
        self.tick += self.cost;
        self.tick
    }

    #[allow(clippy::cast_possible_truncation)]
    fn realtime_ns(&mut self) -> i64 {
        let mut extra = self.storm;
        if self.preempt > 0.0 && self.rng.chance(self.preempt) {
            extra += self.wall.hz * 50_000 / NS;
        }
        // read at the middle of its bracket (the ordered reads either side
        // cost as much), or somewhere in the extra: where the interrupt fell
        let before = if extra > 0 {
            self.rng.below(extra + 1)
        } else {
            0
        };
        self.tick += self.cost + before;
        let probe = self.monotonic_reads % 2 == 1;
        let sigma = if probe { self.probe_sigma } else { self.sigma };
        let noise = (sigma * self.rng.gauss()).round() as i128;
        let mut v = self.wall.at(self.tick) + noise;
        if probe {
            v += i128::from(self.glitch);
        }
        self.tick += extra - before;
        i64::try_from(v).unwrap_or(i64::MAX)
    }

    fn monotonic_ns(&mut self) -> Option<i64> {
        if !self.linux {
            return None;
        }
        self.monotonic_reads += 1;
        self.tick += self.cost;
        i64::try_from(self.wall.mono(self.tick)).ok()
    }
}

/// A counter clock at `hz` calibrated to read `base_ns` at `base_ticks`, in
/// the realtime domain.
fn counter_clock(
    hz: u64,
    base_ticks: u64,
    base_ns: i128,
) -> Result<Clock, Box<dyn std::error::Error>> {
    let scale = Scale::from_hz(hz, 0, 0).ok_or("rate refused")?;
    counter_clock_mult(scale.mult(), base_ticks, base_ns)
}

fn counter_clock_mult(
    mult: u64,
    base_ticks: u64,
    base_ns: i128,
) -> Result<Clock, Box<dyn std::error::Error>> {
    let scale = Scale::from_mult(mult, 0, 0).ok_or("factor refused")?;
    Ok(Clock::Counter(CounterClock::from_params(&Params {
        version: PARAMS_VERSION,
        kind: KIND_COUNTER,
        domain: 1,
        reserved: 0,
        base_ticks,
        base_ns: u64::try_from(base_ns)?,
        mult,
        hz: scale.hz(),
    })?))
}

/// The fallback, calibrated to read `base_ns` at monotonic `base_ticks`.
fn monotonic_clock(base_ticks: u64, base_ns: i128) -> Result<Clock, Box<dyn std::error::Error>> {
    Ok(Clock::Monotonic(MonotonicClock::from_params(&Params {
        version: PARAMS_VERSION,
        kind: KIND_MONOTONIC,
        domain: 1,
        reserved: 0,
        base_ticks,
        base_ns: u64::try_from(base_ns)?,
        mult: 1 << 32,
        hz: NS,
    })?))
}

/// What one update saw: the true residual at its sample before it, the
/// adjustment, and when.
#[derive(Clone, Copy, Debug)]
struct Seen {
    /// The counter's nominal seconds since the run began, ns.
    at_ns: i128,
    truth: i128,
    adjustment: Adjustment,
}

/// A run: a host, the owner's clock, its discipline and sampler, and the
/// checks every re-anchor must pass (P1: no read goes back; P2: the factor
/// within the cap, exactly).
struct Run {
    host: Host,
    clock: Clock,
    disc: Discipline,
    sampler: Sampler,
    cap_ppm: u32,
    sane_ppm: u32,
    start_tick: u64,
    started: bool,
    seen: Vec<Seen>,
    /// The verifier: the true residual between samples, (ns, residual).
    between: Vec<(i128, i128)>,
    reanchors: u64,
}

impl Run {
    /// A counter at `hz` calibrated against `wall` at `tick`, the
    /// calibration `cal_err` ns off.
    fn counter(
        wall: Wall,
        tick: u64,
        cal_err: i64,
        cfg: DisciplineConfig,
        seed: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let clock = counter_clock(wall.hz, tick, wall.at(tick) + i128::from(cal_err))?;
        Self::with(Host::new(wall, tick, seed), clock, cfg)
    }

    /// The fallback (ticks are monotonic ns: `wall.hz` must be 10^9).
    fn monotonic(
        wall: Wall,
        tick: u64,
        cal_err: i64,
        cfg: DisciplineConfig,
        seed: u64,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let clock = monotonic_clock(tick, wall.at(tick) + i128::from(cal_err))?;
        Self::with(Host::new(wall, tick, seed), clock, cfg)
    }

    fn with(
        host: Host,
        clock: Clock,
        cfg: DisciplineConfig,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let disc = Discipline::new(&clock, cfg)?;
        let sampler = Sampler::for_clock(&clock, 1);
        Ok(Self {
            start_tick: host.tick,
            host,
            clock,
            disc,
            sampler,
            cap_ppm: cfg.cap_ppm,
            sane_ppm: cfg.sane_ppm,
            started: false,
            seen: Vec::new(),
            between: Vec::new(),
            reanchors: 0,
        })
    }

    /// The counter's nominal ns since the run began.
    fn now_ns(&self) -> i128 {
        self.host.wall.ns(self.host.tick) - self.host.wall.ns(self.start_tick)
    }

    /// The true residual at `t`: the wall clock less the owner's clock.
    fn truth_at(&self, t: u64) -> i128 {
        self.host.wall.at(t) - i128::from(self.clock.to_ns(t))
    }

    /// Let `ns` pass, the verifier checking the residual `points` times on
    /// the way, and a few random reads of the segment in force checked for
    /// order.
    fn wait(&mut self, ns: u64, points: u64) -> Res {
        let from = self.host.tick;
        let span = u128::from(ns) * u128::from(self.host.ticks_per_s()) / u128::from(NS);
        let span = u64::try_from(span)?;
        for k in 1..=points {
            let t = from + span * k / (points + 1);
            let at = self.host.wall.ns(t) - self.host.wall.ns(self.start_tick);
            self.between.push((at, self.truth_at(t)));
        }
        let mut ts: Vec<u64> = (0..6)
            .map(|_| from + self.host.rng.below(span.max(1)))
            .collect();
        ts.sort_unstable();
        for w in ts.windows(2) {
            assert!(
                self.clock.to_ns(w[0]) <= self.clock.to_ns(w[1]),
                "a segment went back between {} and {}",
                w[0],
                w[1]
            );
        }
        for (i, &t1) in ts.iter().enumerate() {
            for &t2 in &ts[i + 1..] {
                self.check_interval(t1, t2);
            }
        }
        self.host.tick = from + span;
        Ok(())
    }

    /// P2's interval bound, exactly in integers: two reads of the segment in
    /// force, `dt` ticks apart, are `dc` ns apart with
    /// `|dc·2^32 − dt·m₀| < dt·⌊m₀·cap⌋ + 2^32`: within cap·d + 1 ns of the
    /// counter's nominal count d (each read's floor is under a nanosecond).
    fn check_interval(&self, t1: u64, t2: u64) {
        let m0 = i128::from(self.disc.nominal_mult());
        let lim = m0 * i128::from(self.cap_ppm) / 1_000_000;
        let dt = i128::from(t2 - t1);
        let dc = i128::from(self.clock.to_ns(t2)) - i128::from(self.clock.to_ns(t1));
        let error = (dc * (1_i128 << 32) - dt * m0).abs();
        assert!(
            error < dt * lim + (1_i128 << 32),
            "{dt} ticks read {dc} ns apart: off the nominal count by {} ns, over cap·d + 1 ns",
            error >> 32
        );
    }

    /// One second, 0 to 0.5 ms of deferral late, then an update.
    fn second(&mut self) -> Result<Adjustment, Box<dyn std::error::Error>> {
        let late = self.host.rng.below(500_000);
        self.wait(NS + late, 4)?;
        Ok(self.update())
    }

    /// `n` seconds of updates.
    fn seconds(&mut self, n: u64) -> Res {
        for _ in 0..n {
            self.second()?;
        }
        Ok(())
    }

    /// One update (the first is the start update), checked.
    fn update(&mut self) -> Adjustment {
        let phase = if self.started {
            Phase::Running
        } else {
            Phase::Start
        };
        self.started = true;
        let old = self.clock;
        let mid_truth_tick = self.host.tick;
        let truth = self.truth_at(mid_truth_tick);
        let at_ns = self.now_ns();
        let adjustment =
            self.disc
                .sample_and_apply(&mut self.clock, &mut self.sampler, &mut self.host, phase);
        // the install: here, after the ordered read the update took
        let install = self.host.tick;
        check_reanchor(&old, &self.clock, install);
        self.check_factor(&adjustment);
        self.reanchors += 1;
        self.seen.push(Seen {
            at_ns,
            truth,
            adjustment,
        });
        adjustment
    }

    /// P2: the factor within m₀ ± ⌊m₀·cap⌋, the rate within the cap, and
    /// at the cap exactly at the limit.
    fn check_factor(&self, a: &Adjustment) {
        let m0 = self.disc.nominal_mult();
        let lim = u64::try_from(u128::from(m0) * u128::from(self.cap_ppm) / 1_000_000).unwrap_or(0);
        let mult = a.clock.params().mult;
        assert!(
            mult >= m0 - lim && mult <= m0 + lim,
            "factor {mult} outside {m0} +/- {lim}"
        );
        let cap = i64::from(self.cap_ppm) * 1_000_000;
        assert!(a.rate_ppt.abs() <= cap, "rate {} over the cap", a.rate_ppt);
        // P9: no slope over the sanity limit is ever a rate
        let sane = i64::from(self.sane_ppm) * 1_000_000;
        assert!(
            a.freq_ppt.abs() <= sane,
            "frequency {} over the sanity limit",
            a.freq_ppt
        );
        if a.action != Action::Held && a.rate_ppt == cap {
            assert_eq!(mult, m0 + lim, "at the cap, exactly the limit");
        }
        if a.action != Action::Held && a.rate_ppt == -cap {
            assert_eq!(mult, m0 - lim, "at the cap, exactly the limit");
        }
    }

    /// ρ̂ as the last update reported it: the controller's estimate.
    fn freq_ppt(&self) -> i64 {
        self.seen.last().map_or(0, |s| s.adjustment.freq_ppt)
    }

    /// How many updates carried `flag`.
    fn flagged(&self, flag: u32) -> u64 {
        u64::try_from(
            self.seen
                .iter()
                .filter(|s| s.adjustment.flags & flag != 0)
                .count(),
        )
        .unwrap_or(u64::MAX)
    }

    /// Samples the controller held (rejected).
    fn rejects(&self) -> u64 {
        u64::try_from(
            self.seen
                .iter()
                .filter(|s| s.adjustment.action == Action::Held)
                .count(),
        )
        .unwrap_or(u64::MAX)
    }

    /// Steps applied after the start update.
    fn running_steps(&self) -> usize {
        self.seen
            .iter()
            .filter(|s| {
                s.adjustment.flags & flags::START == 0
                    && matches!(s.adjustment.action, Action::Stepped { .. })
            })
            .count()
    }

    /// The largest |true residual| at the updates and between them, from
    /// `from_ns` on.
    fn peak_since(&self, from_ns: i128) -> i128 {
        let at_updates = self
            .seen
            .iter()
            .filter(|s| s.at_ns >= from_ns)
            .map(|s| s.truth.abs());
        let between = self
            .between
            .iter()
            .filter(|&&(at, _)| at >= from_ns)
            .map(|&(_, r)| r.abs());
        at_updates.chain(between).max().unwrap_or(0)
    }

    /// Seconds from `from_ns` until the true residual at every later
    /// update, and between them, stays under `bound` ns (`None`: never).
    fn settled_after(&self, from_ns: i128, bound: i128) -> Option<i128> {
        let last_bad = self
            .seen
            .iter()
            .map(|s| (s.at_ns, s.truth))
            .chain(self.between.iter().copied())
            .filter(|&(at, r)| at >= from_ns && r.abs() >= bound)
            .map(|(at, _)| at)
            .max();
        let end = self.now_ns();
        match last_bad {
            None => Some(0),
            Some(at) if at + 2 * i128::from(NS) < end => Some((at - from_ns) / i128::from(NS) + 1),
            Some(_) => None,
        }
    }
}

/// P1 at one re-anchor installed at `install`: every read of the old
/// segment up to it (its last at `install`) is at or before every read of
/// the new one from it, checked at ±1, ±2 and ±200 ticks.
fn check_reanchor(old: &Clock, new: &Clock, install: u64) {
    let olds = [200, 2, 1, 0].map(|d| old.to_ns(install.saturating_sub(d)));
    let news = [0, 1, 2, 200].map(|d| new.to_ns(install + d));
    let chain: Vec<u64> = olds.iter().chain(news.iter()).copied().collect();
    for w in chain.windows(2) {
        assert!(
            w[0] <= w[1],
            "time went back across a re-anchor at tick {install}: {chain:?}"
        );
    }
}

/// The steady state of a run, a fixed number of samples from its start.
#[derive(Clone, Copy, Debug)]
struct Stats {
    p50: i128,
    p99: i128,
    max: i128,
    mean: f64,
}

#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn stats(values: &[i128]) -> Stats {
    let mut abs: Vec<i128> = values.iter().map(|v| v.abs()).collect();
    abs.sort_unstable();
    let pick = |q: f64| abs[((abs.len() as f64 - 1.0) * q).round() as usize];
    Stats {
        p50: pick(0.5),
        p99: pick(0.99),
        max: abs.last().copied().unwrap_or(0),
        mean: values.iter().map(|&v| v as f64).sum::<f64>() / values.len() as f64,
    }
}

/// A random trajectory, checked at every re-anchor for P1 (no read goes
/// back) and P2 (the factor within the cap, exactly): wall rates within
/// ±5,000 ppm, steps of ±1 µs to 10 s, counter jumps, update gaps of 1 to
/// 6 s, samples taken across interrupts, preempted reads, a start up to a
/// minute after calibration, hand-overs to a new calibration, counters from
/// 24 MHz to 3 GHz (or the fallback's monotonic ns), caps of 100 and 500
/// ppm, with and without steps after start.
fn random_trajectory(seed: u64, fallback: bool) -> Res {
    let mut rng = Rng::new(seed);
    let hz = if fallback {
        NS
    } else {
        match rng.below(5) {
            0 => 24_000_000,
            1 => 1_000_000_000,
            2 => 2_400_000_000,
            3 => 3_000_000_000,
            _ => rng.between(24_000_000, 3_000_000_000).cast_unsigned(),
        }
    };
    let cfg = DisciplineConfig {
        cap_ppm: if rng.chance(0.5) { 100 } else { 500 },
        step_after_start_ns: rng
            .chance(0.3)
            .then(|| rng.log_uniform(1_000_000, 50_000_000)),
        ..DisciplineConfig::default()
    };
    let tick0 = hz + rng.below(1 << 40);
    let mut wall = Wall::new(
        hz,
        EPOCH_2026 + i128::from(rng.between(-1_000_000_000, 1_000_000_000)),
    );
    if rng.chance(0.8) {
        wall.rate_from(0, rng.between(-20_000_000, 20_000_000));
    }
    // the plan: when each update falls, and what the wall clock does
    let n = 15 + rng.below(20);
    let start_delay = rng.log_uniform(500_000, 60 * NS);
    let mut gaps = vec![start_delay];
    for _ in 1..n {
        gaps.push(if rng.chance(0.15) {
            rng.between(2, 6).cast_unsigned() * NS
        } else {
            NS + rng.below(500_000)
        });
    }
    let mut at = tick0;
    for gap in &gaps {
        let span = u64::try_from(u128::from(*gap) * u128::from(hz) / u128::from(NS))?;
        let when = at + rng.below(span.max(1));
        if rng.chance(0.3) {
            wall.rate_from(when, rng.between(-5_000_000_000, 5_000_000_000));
        }
        if rng.chance(0.15) {
            let size = rng.log_uniform(1_000, 10 * NS).cast_signed();
            wall.step(when, if rng.chance(0.5) { size } else { -size });
        }
        if rng.chance(0.04) {
            wall.jump(when, rng.log_uniform(1_000_000, NS).cast_signed());
        }
        at += span;
    }
    let cal_err = rng.between(-100, 100);
    let mut run = if fallback {
        Run::monotonic(wall, tick0, cal_err, cfg, seed)?
    } else {
        Run::counter(wall, tick0, cal_err, cfg, seed)?
    };
    run.host.sigma = f64::from(u8::try_from(rng.below(31)).unwrap_or(0));
    run.host.preempt = if rng.chance(0.5) { 0.02 } else { 0.0 };
    run.host.linux = rng.chance(0.7);
    for gap in gaps {
        run.wait(gap, 2)?;
        run.host.storm = if rng.chance(0.05) {
            hz * rng.between(5_000, 200_000).cast_unsigned() / NS
        } else {
            0
        };
        run.host.glitch = if rng.chance(0.05) {
            rng.between(-30_000, 30_000)
        } else {
            0
        };
        run.update();
        if rng.chance(0.03) {
            hand_over(&mut run, &mut rng, fallback)?;
        }
    }
    Ok(())
}

/// A hand-over to a fresh calibration of the same ticks, its factor up to
/// 100 ppm off and its anchor up to 10 µs off: never behind the old clock.
fn hand_over(run: &mut Run, rng: &mut Rng, fallback: bool) -> Res {
    let old = run.clock;
    let fresh_base = run.host.wall.at(run.host.tick) + i128::from(rng.between(-10_000, 10_000));
    let new = if fallback {
        monotonic_clock(run.host.tick, fresh_base)?
    } else {
        let m0 = run.disc.nominal_mult();
        let off = i128::from(m0) * i128::from(rng.between(-100, 100)) / 1_000_000;
        let mult = u64::try_from(i128::from(m0) + off)?;
        counter_clock_mult(mult, run.host.tick, fresh_base)?
    };
    let new_ticks = run.host.ticks_ordered();
    let old_after = run.host.ticks_ordered();
    let anchored = run.disc.rebase_at(&old, &new, new_ticks, old_after)?;
    let install = run.host.tick;
    check_reanchor(&old, &anchored, install);
    assert!(
        anchored.to_ns(install) - old.to_ns(install) <= old.ticks_to_ns(install - new_ticks) + 2,
        "a hand-over runs ahead by at most its bracket"
    );
    assert_eq!(
        run.disc.nominal_mult(),
        new.params().mult,
        "m0 is the new calibration's"
    );
    run.clock = anchored;
    Ok(())
}

/// P1 and P2 over 10,000 random counter trajectories.
#[test]
fn no_read_goes_back_and_the_factor_stays_within_the_cap() -> Res {
    for seed in 0..10_000 {
        random_trajectory(seed, false).map_err(|e| format!("seed {seed}: {e}"))?;
    }
    Ok(())
}

/// P11: P1 and P2 for the fallback, whose ticks are monotonic ns and m₀
/// is 2^32.
#[test]
fn the_fallback_never_goes_back_either() -> Res {
    for seed in 0..10_000 {
        random_trajectory(seed + 1_000_000, true).map_err(|e| format!("seed {seed}: {e}"))?;
    }
    Ok(())
}

/// A run at 1 GHz settled on a wall clock running `rho_ppt` against the
/// counter: calibrated, started 0.5 s later, then `settle` seconds of
/// updates.
fn settled(
    rho_ppt: i64,
    sigma: f64,
    settle: u64,
    seed: u64,
) -> Result<Run, Box<dyn std::error::Error>> {
    settled_with(
        NS,
        rho_ppt,
        sigma,
        settle,
        DisciplineConfig::default(),
        seed,
    )
}

/// [`settled`] with exact step probes (no noise on their wall reads), so a
/// step's size is seen exactly.
fn settled_exact(
    rho_ppt: i64,
    sigma: f64,
    settle: u64,
    cfg: DisciplineConfig,
    seed: u64,
) -> Result<Run, Box<dyn std::error::Error>> {
    let mut wall = Wall::new(NS, EPOCH_2026);
    wall.rate_from(0, rho_ppt);
    let mut run = Run::counter(wall, 1_000 * NS, 0, cfg, seed)?;
    run.host.sigma = sigma;
    run.host.probe_sigma = 0.0;
    run.wait(NS / 2, 0)?;
    run.update();
    run.seconds(settle)?;
    Ok(run)
}

fn settled_with(
    hz: u64,
    rho_ppt: i64,
    sigma: f64,
    settle: u64,
    cfg: DisciplineConfig,
    seed: u64,
) -> Result<Run, Box<dyn std::error::Error>> {
    let mut wall = Wall::new(hz, EPOCH_2026);
    wall.rate_from(0, rho_ppt);
    let mut run = Run::counter(wall, 1_000 * hz, 0, cfg, seed)?;
    run.host.sigma = sigma;
    run.wait(NS / 2, 0)?;
    run.update();
    run.seconds(settle)?;
    Ok(run)
}

/// [`settled`] for the fallback (P11): its ticks are monotonic ns and m₀ is
/// 2^32, re-anchored through `MonotonicClock`'s own `retuned`.
fn settled_fallback(
    rho_ppt: i64,
    sigma: f64,
    settle: u64,
    seed: u64,
) -> Result<Run, Box<dyn std::error::Error>> {
    let mut wall = Wall::new(NS, EPOCH_2026);
    wall.rate_from(0, rho_ppt);
    let mut run = Run::monotonic(wall, 1_000 * NS, 0, DisciplineConfig::default(), seed)?;
    run.host.sigma = sigma;
    run.wait(NS / 2, 0)?;
    run.update();
    run.seconds(settle)?;
    Ok(run)
}

/// The controller's own bound on the time to bring a residual `theta0`
/// under 1 µs at wall rate `rho` (both against the counter), seconds:
/// the slew at the cap's margin to the linear region, then the residual
/// halving each second (the pole at 0.5), then a second for the sample.
#[allow(clippy::cast_precision_loss)]
fn convergence_bound_s(theta0_ns: i64, rho_ppt: i64, cfg: &DisciplineConfig) -> f64 {
    let margin = f64::from(cfg.cap_ppm).mul_add(1e6, -(rho_ppt.abs() as f64)) / 1e12; // ns per ns
    let tau_s = cfg.tau_ns as f64 / 1e9;
    let linear_ns = tau_s * 1e9 * margin;
    let theta = theta0_ns.unsigned_abs() as f64;
    let slew = ((theta - linear_ns).max(0.0)) / (margin * 1e9);
    let halving = (theta.min(linear_ns) / 1_000.0).max(1.0).log2();
    slew + halving + 1.0
}

/// P4: from a residual of 2 µs to 1 s either way, at a constant wall rate
/// within ±400 ppm, the residual (at the samples and between them) is
/// under 1 µs within the controller's bound plus 3 s, and stays there: on
/// 10,000 counter runs and (P11) 2,000 of the fallback. Each run is settled
/// first (20 s): the start leaves the clock ahead by the wall clock's rate
/// over the half second since calibration (up to 200 µs at 400 ppm), which
/// takes several seconds to slew out, and the bound is from θ₀ alone.
#[test]
fn a_residual_converges_within_the_bound() -> Res {
    let cfg = DisciplineConfig::default();
    let runs = (0..10_000_u64)
        .map(|s| (s, false))
        .chain((10_000..12_000).map(|s| (s, true)));
    for (seed, fallback) in runs {
        let mut rng = Rng::new(seed + 2_000_000);
        let rho = rng.between(-400_000_000, 400_000_000);
        let size = rng.log_uniform(2_000, NS).cast_signed();
        let theta0 = if rng.chance(0.5) { size } else { -size };
        let mut run = if fallback {
            settled_fallback(rho, 15.0, 20, seed)?
        } else {
            settled(rho, 15.0, 20, seed)?
        };
        run.host.linux = rng.chance(0.8);
        let at = run.host.tick + 1;
        run.host.wall.step(at, theta0);
        let from = run.now_ns();
        let bound = convergence_bound_s(theta0, rho, &cfg) + 3.0;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let horizon = bound.ceil() as u64 + 5;
        // until the bound and five seconds more, or ten seconds under 1 µs
        // at the samples and between them
        let mut under = 0;
        for _ in 0..horizon {
            let late = run.host.rng.below(500_000);
            run.wait(NS + late, 1)?;
            run.update();
            let last_between = run.between.last().map_or(0, |b| b.1);
            if run.seen.last().is_some_and(|s| s.truth.abs() < 1_000) && last_between.abs() < 1_000
            {
                under += 1;
                if under == 10 {
                    break;
                }
            } else {
                under = 0;
            }
        }
        let took = run.settled_after(from, 1_000);
        #[allow(clippy::cast_precision_loss)]
        let ok = took.is_some_and(|s| s as f64 <= bound);
        assert!(
            ok,
            "seed {seed}: theta0 {theta0} ns at {rho} ppt took {took:?} s, bound {bound:.1} s"
        );
    }
    Ok(())
}

/// P5: settled at a constant wall rate, with bracket noise σ up to 20 ns,
/// the residual at the samples and between them has p99 ≤ 6σ + 10 ns,
/// max ≤ 8σ + 20 ns, and a mean within 5 ns of zero: the controller rounds
/// to the nearest everywhere (a floor in the factor biases a 50 GHz
/// counter's residual by about 12 ns). Counters at 1, 2.4 and 50 GHz, and
/// (P11) the fallback.
#[test]
fn the_steady_residual_is_within_the_noise() -> Res {
    let rates = [1_448_000, -135_000, -10_134_000, 0, 37_000_000];
    let mut seed = 3_000_000;
    for hz in [1_000_000_000, 2_400_000_000, 50_000_000_000, 0] {
        for sigma in [4.0, 10.0, 20.0] {
            for rho in rates {
                seed += 1;
                // hz 0: the fallback, whose ticks are monotonic ns
                let mut run = if hz == 0 {
                    settled_fallback(rho, sigma, 60, seed)?
                } else {
                    settled_with(hz, rho, sigma, 60, DisciplineConfig::default(), seed)?
                };
                let from = run.now_ns();
                run.seconds(1_800)?;
                assert_steady(
                    &run,
                    from,
                    sigma,
                    &format!("{hz} Hz, sigma {sigma}, {rho} ppt"),
                );
            }
        }
    }
    Ok(())
}

/// P5's bounds on a run's residual from `from` on, at the samples and
/// between them.
#[allow(clippy::cast_possible_truncation)]
fn assert_steady(run: &Run, from: i128, sigma: f64, what: &str) {
    let values: Vec<i128> = run
        .seen
        .iter()
        .filter(|s| s.at_ns >= from)
        .map(|s| s.truth)
        .chain(run.between.iter().filter(|b| b.0 >= from).map(|b| b.1))
        .collect();
    let st = stats(&values);
    eprintln!(
        "steady: {what}: p50 {} / p99 {} / max {} ns, mean {:.2} ns",
        st.p50, st.p99, st.max, st.mean
    );
    let p99_bound = 6.0_f64.mul_add(sigma, 10.0) as i128;
    let max_bound = 8.0_f64.mul_add(sigma, 20.0) as i128;
    assert!(
        st.p99 <= p99_bound && st.max <= max_bound && st.mean.abs() <= 5.0,
        "{what}: {st:?} (p99 <= {p99_bound}, max <= {max_bound}, |mean| <= 5)"
    );
}

/// P6: a wall-clock step of ±1 µs to ±10 s after start is seen as a step
/// (the frequency estimate stays within 0.01 ppm of the wall clock's rate)
/// and slewed, never stepped, by default; the same step with steps after
/// start opted in, behind by at least the threshold, is stepped by exactly
/// the residual; a clock ahead is never stepped.
#[test]
fn a_wall_step_is_slewed_and_never_spoils_the_frequency() -> Res {
    for seed in 0..1_000_u64 {
        let mut rng = Rng::new(seed + 4_000_000);
        let rho = rng.between(-100_000_000, 100_000_000);
        let size = rng.log_uniform(1_100, 10 * NS).cast_signed();
        let delta = if rng.chance(0.5) { size } else { -size };
        let threshold = rng.log_uniform(1_000_000, 50_000_000);
        let opt_in = rng.chance(0.5);
        let cfg = DisciplineConfig {
            step_after_start_ns: opt_in.then_some(threshold),
            ..DisciplineConfig::default()
        };
        let linux = rng.chance(0.7);
        // a control run, the same in every draw, whose wall clock does not step
        let mut control = settled_exact(rho, 1.0, 4, cfg, seed)?;
        let mut run = settled_exact(rho, 1.0, 4, cfg, seed)?;
        for r in [&mut control, &mut run] {
            r.host.linux = linux;
        }
        let before = run.freq_ppt();
        let at = run.host.tick + 1;
        run.host.wall.step(at, delta);
        let a = run.second()?;
        let c = control.second()?;
        if linux {
            // the step taken out of the slope exactly: the frequency is the
            // control's, and within 0.01 ppm of the wall clock's
            assert_eq!(
                a.flags & flags::WALL_STEP,
                flags::WALL_STEP,
                "seed {seed}: step {delta} not seen"
            );
            assert_eq!(a.wall_step_ns, delta, "seed {seed}");
            assert_eq!(
                a.freq_ppt, c.freq_ppt,
                "seed {seed}: the step spoilt the slope"
            );
            assert!(
                (a.freq_ppt - rho).abs() <= 10_000,
                "seed {seed}: {} against {rho}",
                a.freq_ppt
            );
        } else if delta.abs() >= 1_000_000 {
            // inferred from the innovation: the slope is the estimate's
            // (within its rounding to a nanosecond over the second)
            assert_ne!(
                a.flags & flags::WALL_STEP,
                0,
                "seed {seed}: step {delta} not seen"
            );
            assert!(
                (a.wall_step_ns - delta).abs() <= 1_000,
                "seed {seed}: {}",
                a.wall_step_ns
            );
            assert!(
                (a.freq_ppt - before).abs() <= 1_000,
                "seed {seed}: {} against {before}",
                a.freq_ppt
            );
        }
        let behind_enough = i64::try_from(threshold).is_ok_and(|x| a.residual_ns >= x);
        if opt_in && behind_enough {
            assert_eq!(
                a.action,
                Action::Stepped {
                    ns: a.residual_ns.cast_unsigned()
                },
                "seed {seed}: an opted-in step is the whole residual"
            );
            assert!(
                run.truth_at(run.host.tick).abs() < 2_000,
                "seed {seed}: on the wall clock after the step"
            );
        } else {
            assert_eq!(a.action, Action::Slewed, "seed {seed}: no step after start");
        }
        // and the next second's frequency is the wall clock's again
        let b = run.second()?;
        let d = control.second()?;
        assert_eq!(b.freq_ppt, d.freq_ppt, "seed {seed}: a second later");
        assert_eq!(b.action, Action::Slewed, "seed {seed}: one step at most");
    }
    Ok(())
}

/// P6: the start update steps a clock that is behind forward by its whole
/// residual, exactly once; a clock ahead is slewed back, never stepped.
#[test]
fn the_start_steps_forward_once_and_never_back() -> Res {
    for seed in 0..1_000_u64 {
        let mut rng = Rng::new(seed + 5_000_000);
        let size = rng.log_uniform(1, 10 * NS).cast_signed();
        let cal_err = if rng.chance(0.5) { size } else { -size };
        let mut wall = Wall::new(NS, EPOCH_2026);
        wall.rate_from(0, rng.between(-20_000_000, 20_000_000));
        // the clock is calibrated cal_err ns off the wall clock
        let mut run = Run::counter(
            wall,
            1_000 * NS,
            -cal_err,
            DisciplineConfig::default(),
            seed,
        )?;
        run.host.linux = rng.chance(0.7);
        run.wait(NS / 2, 0)?;
        let first = run.update();
        assert_ne!(first.flags & flags::START, 0);
        if first.residual_ns > 0 {
            assert_eq!(
                first.action,
                Action::Stepped {
                    ns: first.residual_ns.cast_unsigned()
                },
                "seed {seed}: behind by {cal_err}: one step of the residual"
            );
            assert_eq!(
                first.wall_offset_ns, 0,
                "seed {seed}: on the wall clock after it"
            );
        } else {
            assert_eq!(
                first.action,
                Action::Slewed,
                "seed {seed}: ahead by {cal_err}: never back"
            );
            assert_eq!(first.wall_offset_ns, first.residual_ns);
        }
        for _ in 0..5 {
            assert_eq!(
                run.second()?.action,
                Action::Slewed,
                "seed {seed}: one step only"
            );
        }
        assert_eq!(
            run.flagged(flags::STEPPED),
            u64::from(first.residual_ns > 0)
        );
    }
    Ok(())
}

/// P7: one step probe glitched by ±20 µs (a preempted pair) changes
/// nothing but the glitch count: every clock the run installs is the one a
/// run without the glitch installs. A real step on the next sample is
/// still recognised.
#[test]
fn a_glitched_step_probe_changes_nothing() -> Res {
    for seed in 0..200_u64 {
        let rho = 1_448_000;
        let cfg = DisciplineConfig::default();
        let mut clean = settled_exact(rho, 1.0, 6, cfg, seed)?;
        let mut glitched = settled_exact(rho, 1.0, 6, cfg, seed)?;
        let glitch = if seed % 2 == 0 { 20_000 } else { -20_000 };
        glitched.host.glitch = glitch;
        let a = glitched.second()?;
        glitched.host.glitch = 0;
        let b = clean.second()?;
        assert_ne!(
            a.flags & flags::D_GLITCH,
            0,
            "seed {seed}: the glitch is counted"
        );
        assert_eq!(a.wall_step_ns, 0, "seed {seed}: not a step");
        assert_eq!(a.clock, b.clock, "seed {seed}: the same clock installed");
        assert_eq!(glitched.flagged(flags::D_GLITCH), 1);
        for _ in 0..5 {
            let (a, b) = (glitched.second()?, clean.second()?);
            assert_eq!(a.clock, b.clock, "seed {seed}: the same clocks after it");
            assert_eq!(a.flags, b.flags);
        }
        // a glitch, then a real step: recognised against the trusted probe
        glitched.host.glitch = glitch;
        glitched.second()?;
        glitched.host.glitch = 0;
        let at = glitched.host.tick + 1;
        glitched.host.wall.step(at, 5_000_000);
        let s = glitched.second()?;
        assert_ne!(
            s.flags & flags::WALL_STEP,
            0,
            "seed {seed}: the step after a glitch"
        );
        assert_eq!(s.wall_step_ns, 5_000_000);
        assert!(
            (s.freq_ppt - rho).abs() <= 10_000,
            "seed {seed}: {}",
            s.freq_ppt
        );
    }
    Ok(())
}

/// A glitch in the very first step probe (the one every later probe is
/// compared with) is outvoted by the next two, which agree with each other:
/// later steps are recognised again (without it, every later probe would
/// be a "glitch" and no step would be seen).
#[test]
fn a_glitched_first_probe_is_outvoted() -> Res {
    let mut wall = Wall::new(NS, EPOCH_2026);
    wall.rate_from(0, 2_000_000);
    // the probes' own noise (5 ns): later probes agree within their
    // threshold, not exactly
    let mut run = Run::counter(wall, 1_000 * NS, 0, DisciplineConfig::default(), 7)?;
    run.host.sigma = 1.0;
    run.wait(NS / 2, 0)?;
    run.host.glitch = 30_000;
    run.update();
    run.host.glitch = 0;
    run.seconds(4)?;
    assert_eq!(
        run.flagged(flags::D_GLITCH),
        2,
        "outvoted at the second disagreement"
    );
    let at = run.host.tick + 1;
    run.host.wall.step(at, -5_000_000);
    let s = run.second()?;
    assert_ne!(s.flags & flags::WALL_STEP, 0, "the step is seen");
    assert!(
        (s.wall_step_ns + 5_000_000).abs() <= 100,
        "{}",
        s.wall_step_ns
    );
    assert!((s.freq_ppt - 2_000_000).abs() <= 50_000, "{}", s.freq_ppt);
    Ok(())
}

/// P8: a sample taken across interrupts (every bracket wide) is rejected:
/// the clock is re-anchored at the frequency estimate alone, the phase
/// term dropped, with no step, and the estimate kept; the next accepted
/// sample's slope spans both intervals.
#[test]
fn a_wide_sample_is_held_at_the_frequency_estimate() -> Res {
    for seed in 0..200_u64 {
        let rho = -10_134_000;
        let mut run = settled(rho, 1.0, 10, seed)?;
        let before = run.freq_ppt();
        let old = run.clock;
        run.host.storm = 20_000;
        let held = run.second()?;
        assert_eq!(held.action, Action::Held, "seed {seed}");
        assert_ne!(held.flags & flags::REJECTED, 0);
        assert_eq!(held.freq_ppt, before, "the estimate is kept");
        assert_eq!(run.rejects(), 1);
        let m0 = i128::from(run.disc.nominal_mult());
        let want = m0 + (m0 * i128::from(before) + PPT / 2 * i128::from(before.signum())) / PPT;
        assert_eq!(
            i128::from(held.clock.params().mult),
            want,
            "ρ̂ alone, no phase term"
        );
        let at = held.clock.params().base_ticks;
        assert_eq!(held.clock.to_ns(at), old.to_ns(at), "no step");
        run.host.storm = 0;
        let next = run.second()?;
        assert_eq!(next.action, Action::Slewed);
        assert!(
            (next.freq_ppt - rho).abs() <= 10_000,
            "seed {seed}: {}",
            next.freq_ppt
        );
        assert_eq!(run.disc.consecutive_rejects(), 0);
    }
    Ok(())
}

/// A host whose brackets widen for good (another tenant on the core) is
/// followed again: the reject floor remembers only the last 64 samples, so
/// after at most 64 rejects the wider samples are accepted.
#[test]
fn a_noisier_host_is_followed_again_within_64_samples() -> Res {
    let mut run = settled(1_000_000, 4.0, 20, 11)?;
    run.host.storm = 3_000;
    let mut held = 0;
    for _ in 0..80 {
        if run.second()?.action == Action::Held {
            held += 1;
        }
    }
    assert!(held > 0 && held <= 64, "{held} held");
    let last = run.second()?;
    assert_eq!(last.action, Action::Slewed, "accepted again");
    Ok(())
}

/// P9: a slope over the sanity limit (1,000 ppm) never enters the
/// frequency estimate, and is flagged: systemd-timesyncd's 3,546 ppm boot
/// slew, a counter jump.
#[test]
fn a_slope_beyond_the_sanity_limit_is_never_a_rate() -> Res {
    let mut run = settled(0, 10.0, 5, 12)?;
    let at = run.host.tick + 1;
    run.host.wall.rate_from(at, 3_546_000_000);
    for _ in 0..10 {
        let a = run.second()?;
        assert!(a.freq_ppt.abs() <= 1_000_000_000, "{}", a.freq_ppt);
        assert_ne!(a.flags & flags::FAST_SLEW_OR_JUMP, 0);
    }
    let mut run = settled(0, 10.0, 5, 13)?;
    let at = run.host.tick + 1;
    run.host.wall.jump(at, 2_000_000);
    let a = run.second()?;
    assert_ne!(a.flags & flags::FAST_SLEW_OR_JUMP, 0, "a counter jump");
    assert_eq!(a.flags & flags::WALL_STEP, 0, "the step probe did not move");
    assert_eq!(a.freq_ppt, run.seen[run.seen.len() - 2].adjustment.freq_ppt);
    Ok(())
}

/// A named wall-clock event (spec §1.5) on a settled 1 GHz run (σ = 15
/// ns, the default controller), its onset half-way between two updates.
struct Scenario {
    name: &'static str,
    /// Seconds of updates after the onset (and the event's own length).
    run_s: u64,
    /// When the episode ends, s after the onset (a slew's end; 0 for a
    /// step), from which the settling time is counted.
    end_s: u64,
    /// The spec's simulated peak |residual| (ns) and time back under 1 µs
    /// (s), each to be met within 20%.
    peak_ns: i128,
    settle_s: i128,
}

/// The updates' wall-clock events: what happens `at` the onset tick.
type Event = fn(&mut Run, u64) -> Res;

/// Run `event` at the onset of a settled run; the peak |residual| from the
/// onset, and the seconds from the episode's end until it stays under 1 µs.
fn scenario(
    s: &Scenario,
    event: Event,
    linux: bool,
) -> Result<(i128, Option<i128>, Run), Box<dyn std::error::Error>> {
    let mut run = settled(0, 15.0, 20, 6_000_000)?;
    run.host.linux = linux;
    run.wait(NS / 2, 2)?;
    let onset_ns = run.now_ns();
    let at = run.host.tick;
    event(&mut run, at)?;
    let rest = NS / 2 + run.host.rng.below(500_000);
    run.wait(rest, 2)?;
    run.update();
    for _ in 1..s.run_s {
        run.second()?;
    }
    let peak = run.peak_since(onset_ns);
    let settle = run.settled_after(onset_ns + i128::from(s.end_s) * i128::from(NS), 1_000);
    Ok((peak, settle, run))
}

/// Each row's peak and settling time within 20% of the spec's simulation
/// (rounded up to the next whole second for the time).
fn check_scenario(
    s: &Scenario,
    event: Event,
    linux: bool,
) -> Result<Run, Box<dyn std::error::Error>> {
    let (peak, settle, run) = scenario(s, event, linux)?;
    eprintln!(
        "scenario {} ({}): peak {} ns (spec {}), back under 1 us after {:?} s (spec {})",
        s.name,
        if linux { "linux" } else { "no step probe" },
        peak,
        s.peak_ns,
        settle,
        s.settle_s
    );
    assert!(
        peak <= s.peak_ns * 6 / 5,
        "{}: peak {peak} ns over the spec's {} + 20%",
        s.name,
        s.peak_ns
    );
    let limit = (s.settle_s * 6 + 4) / 5;
    assert!(
        settle.is_some_and(|t| t <= limit),
        "{}: settled after {settle:?} s, the spec's {} + 20% is {limit}",
        s.name,
        s.settle_s
    );
    Ok(run)
}

/// A slew of `ppm` for `secs` from `at`.
fn slew(run: &mut Run, at: u64, ppt: i64, secs: u64) {
    run.host.wall.rate_from(at, ppt);
    run.host.wall.rate_from(at + secs * NS, 0);
}

#[test]
fn scenario_chrony_slew_of_102_ppm_for_two_minutes() -> Res {
    let s = Scenario {
        name: "chrony +102 ppm, 120 s",
        run_s: 150,
        end_s: 120,
        peak_ns: 74_500,
        settle_s: 7,
    };
    check_scenario(
        &s,
        |r, at| {
            slew(r, at, 102_000_000, 120);
            Ok(())
        },
        true,
    )?;
    Ok(())
}

#[test]
fn scenario_chrony_slew_of_533_ppm_is_followed_at_the_cap() -> Res {
    let s = Scenario {
        name: "chrony +533 ppm, 60 s",
        run_s: 90,
        end_s: 60,
        peak_ns: 2_310_000,
        settle_s: 12,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            slew(r, at, 533_000_000, 60);
            Ok(())
        },
        true,
    )?;
    assert!(
        run.flagged(flags::SLEW_LIMITED) >= 50,
        "the cap bound: {} of {}",
        run.flagged(flags::SLEW_LIMITED),
        run.reanchors
    );
    Ok(())
}

#[test]
fn scenario_chrony_slew_of_minus_411_ppm() -> Res {
    let s = Scenario {
        name: "chrony -411 ppm, 60 s",
        run_s: 90,
        end_s: 60,
        peak_ns: 300_000,
        settle_s: 10,
    };
    check_scenario(
        &s,
        |r, at| {
            slew(r, at, -411_000_000, 60);
            Ok(())
        },
        true,
    )?;
    Ok(())
}

#[test]
fn scenario_timesyncd_boot_slew_of_3546_ppm() -> Res {
    let s = Scenario {
        name: "timesyncd +3,546 ppm, 10 s",
        run_s: 100,
        end_s: 10,
        peak_ns: 30_500_000,
        settle_s: 69,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            slew(r, at, 3_546_000_000, 10);
            Ok(())
        },
        true,
    )?;
    assert!(
        run.flagged(flags::FAST_SLEW_OR_JUMP) >= 9,
        "over the sanity limit: never a rate"
    );
    Ok(())
}

#[test]
fn scenario_timesyncd_slew_of_minus_900_ppm() -> Res {
    let s = Scenario {
        name: "timesyncd -900 ppm, 70 s",
        run_s: 150,
        end_s: 70,
        peak_ns: 28_100_000,
        settle_s: 64,
    };
    check_scenario(
        &s,
        |r, at| {
            slew(r, at, -900_000_000, 70);
            Ok(())
        },
        true,
    )?;
    Ok(())
}

#[test]
fn scenario_aws_leap_smear() -> Res {
    let s = Scenario {
        name: "leap smear -11.574 ppm",
        run_s: 20,
        end_s: 0,
        peak_ns: 9_300,
        settle_s: 4,
    };
    check_scenario(
        &s,
        |r, at| {
            r.host.wall.rate_from(at, -11_574_000);
            Ok(())
        },
        true,
    )?;
    Ok(())
}

#[test]
fn scenario_rate_step_of_50_ppm() -> Res {
    let s = Scenario {
        name: "rate step +50 ppm",
        run_s: 20,
        end_s: 0,
        peak_ns: 36_000,
        settle_s: 6,
    };
    check_scenario(
        &s,
        |r, at| {
            r.host.wall.rate_from(at, 50_000_000);
            Ok(())
        },
        true,
    )?;
    Ok(())
}

/// The spec's sim gave a peak of 124 µs and 9 s here, which no controller
/// that holds at the old frequency can meet: the wall clock gains 50 ppm ×
/// 3.5 s = 175 µs before the first accepted sample after the onset. The
/// bounds are the controller's own (the phase term dropped on each reject
/// adds nothing).
#[test]
fn scenario_rate_step_of_50_ppm_with_three_rejects() -> Res {
    // settling: from 175 µs, the first accepted slope spans the 4 s since
    // the last accepted sample (43.75 ppm), then the residual halves each
    // second: 13 s from the onset by the controller's own arithmetic
    let s = Scenario {
        name: "rate step +50 ppm, 3 rejects",
        run_s: 20,
        end_s: 0,
        peak_ns: 175_000,
        settle_s: 13,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            r.host.wall.rate_from(at, 50_000_000);
            r.host.storm = 20_000;
            r.wait(NS / 2, 1)?;
            r.update();
            for _ in 0..2 {
                r.wait(NS, 1)?;
                r.update();
            }
            r.host.storm = 0;
            r.wait(NS / 2, 1)
        },
        true,
    )?;
    assert_eq!(run.rejects(), 3);
    Ok(())
}

/// The spec's sim gave 88 µs here, which no sampled controller can meet:
/// with no sample for 6 s the wall clock gains 50 ppm × 6.5 s = 325 µs
/// first. The bound is that, and the overshoot afterwards none.
#[test]
fn scenario_rate_step_of_50_ppm_with_a_six_second_stall() -> Res {
    let s = Scenario {
        name: "rate step +50 ppm, 6 s stall",
        run_s: 20,
        end_s: 6,
        peak_ns: 325_000,
        settle_s: 9,
    };
    check_scenario(
        &s,
        |r, at| {
            r.host.wall.rate_from(at, 50_000_000);
            r.wait(6 * NS, 12)
        },
        true,
    )?;
    Ok(())
}

#[test]
fn scenario_wall_step_of_half_a_millisecond_is_slewed() -> Res {
    let s = Scenario {
        name: "wall step +0.5 ms",
        run_s: 20,
        end_s: 0,
        peak_ns: 500_000,
        settle_s: 9,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            r.host.wall.step(at, 500_000);
            Ok(())
        },
        true,
    )?;
    assert_eq!(run.running_steps(), 0, "slewed, not stepped");
    Ok(())
}

#[test]
fn scenario_wall_step_of_five_milliseconds_is_slewed_at_the_cap() -> Res {
    let s = Scenario {
        name: "wall step +5 ms",
        run_s: 30,
        end_s: 0,
        peak_ns: 5_000_000,
        settle_s: 17,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            r.host.wall.step(at, 5_000_000);
            Ok(())
        },
        true,
    )?;
    assert_eq!(run.running_steps(), 0);
    assert_eq!(run.flagged(flags::WALL_STEP), 1);
    Ok(())
}

#[test]
fn scenario_wall_step_of_fifty_milliseconds_is_slewed_at_the_cap() -> Res {
    let s = Scenario {
        name: "wall step +50 ms",
        run_s: 130,
        end_s: 0,
        peak_ns: 50_000_000,
        settle_s: 107,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            r.host.wall.step(at, 50_000_000);
            Ok(())
        },
        true,
    )?;
    assert_eq!(run.running_steps(), 0);
    Ok(())
}

#[test]
fn scenario_wall_step_of_fifty_milliseconds_stepped_when_opted_in() -> Res {
    let cfg = DisciplineConfig {
        step_after_start_ns: Some(10_000_000),
        ..DisciplineConfig::default()
    };
    let mut run = settled_with(NS, 0, 15.0, 20, cfg, 6_000_000)?;
    let at = run.host.tick + 1;
    run.host.wall.step(at, 50_000_000);
    let a = run.second()?;
    assert!(
        matches!(a.action, Action::Stepped { ns } if ns.abs_diff(50_000_000) < 1_000),
        "{a:?}"
    );
    assert!(
        run.truth_at(run.host.tick).abs() < 1_000,
        "on the wall clock at the next sample"
    );
    run.seconds(10)?;
    assert_eq!(run.running_steps(), 1, "one step");
    Ok(())
}

#[test]
fn scenario_wall_step_back_of_fifty_milliseconds_is_never_followed_back() -> Res {
    let s = Scenario {
        name: "wall step -50 ms",
        run_s: 130,
        end_s: 0,
        peak_ns: 50_000_000,
        settle_s: 107,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            r.host.wall.step(at, -50_000_000);
            Ok(())
        },
        true,
    )?;
    assert_eq!(run.running_steps(), 0);
    Ok(())
}

#[test]
fn scenario_unsmeared_leap_second() -> Res {
    let s = Scenario {
        name: "unsmeared leap, -1 s",
        run_s: 2_040,
        end_s: 0,
        peak_ns: 1_000_000_000,
        settle_s: 2_007,
    };
    check_scenario(
        &s,
        |r, at| {
            r.host.wall.step(at, -1_000_000_000);
            Ok(())
        },
        true,
    )?;
    Ok(())
}

#[test]
fn scenario_step_of_five_milliseconds_without_a_step_probe() -> Res {
    let s = Scenario {
        name: "wall step +5 ms",
        run_s: 30,
        end_s: 0,
        peak_ns: 5_000_000,
        settle_s: 17,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            r.host.wall.step(at, 5_000_000);
            Ok(())
        },
        false,
    )?;
    assert_eq!(
        run.flagged(flags::WALL_STEP),
        1,
        "inferred from the innovation"
    );
    let s = Scenario {
        name: "wall step -50 ms",
        run_s: 130,
        end_s: 0,
        peak_ns: 50_000_000,
        settle_s: 107,
    };
    check_scenario(
        &s,
        |r, at| {
            r.host.wall.step(at, -50_000_000);
            Ok(())
        },
        false,
    )?;
    Ok(())
}

/// The wall clock steps 50 ms forward between calibration and start: the
/// start update steps forward once and leaves the clock on the wall clock.
#[test]
fn scenario_wall_step_before_start_is_one_forward_step() -> Res {
    let mut wall = Wall::new(NS, EPOCH_2026);
    wall.step(1_000 * NS + NS / 4, 50_000_000);
    let mut run = Run::counter(wall, 1_000 * NS, 0, DisciplineConfig::default(), 6_000_001)?;
    run.wait(NS / 2, 0)?;
    let a = run.update();
    assert!(
        matches!(a.action, Action::Stepped { ns } if ns.abs_diff(50_000_000) < 1_000),
        "{a:?}"
    );
    let after = run.truth_at(run.host.tick).abs();
    eprintln!("scenario wall step +50 ms before start: {after} ns after the step (spec 100)");
    assert!(after <= 120, "{after} ns off after the start step");
    run.seconds(5)?;
    assert!(run.peak_since(run.now_ns() - 5 * i128::from(NS)) < 1_000);
    Ok(())
}

/// The wall clock runs 10 ppm slower than the counter and the start comes
/// a minute after calibration (persist waiting for a subscriber): the clock
/// is 600 µs ahead, so it slews back, never steps back.
#[test]
fn scenario_start_a_minute_after_calibration_ahead_of_the_wall_clock() -> Res {
    let mut wall = Wall::new(NS, EPOCH_2026);
    wall.rate_from(0, -10_000_000);
    let mut run = Run::counter(wall, 1_000 * NS, 0, DisciplineConfig::default(), 6_000_002)?;
    run.host.sigma = 15.0;
    run.wait(60 * NS, 0)?;
    let from = run.now_ns();
    let a = run.update();
    assert_eq!(a.action, Action::Slewed, "ahead: never stepped back");
    assert!((a.residual_ns + 600_000).abs() < 1_000, "{a:?}");
    assert!(
        (a.freq_ppt + 10_000_000).abs() < 1_000,
        "the minute's slope: {}",
        a.freq_ppt
    );
    run.seconds(20)?;
    let peak = run.peak_since(from);
    let settle = run.settled_after(from, 1_000);
    eprintln!(
        "scenario start 60 s after calibration: peak {peak} ns (spec 600000), settled after {settle:?} s (spec 9)"
    );
    assert!(
        peak <= 720_000 && settle.is_some_and(|t| t <= 11),
        "{peak} {settle:?}"
    );
    Ok(())
}

#[test]
fn scenario_a_glitched_step_probe_is_ignored() -> Res {
    let s = Scenario {
        name: "probe glitch -20 us",
        run_s: 10,
        end_s: 0,
        peak_ns: 100,
        settle_s: 0,
    };
    let (peak, _, run) = scenario(
        &s,
        |r, _| {
            r.host.glitch = -20_000;
            Ok(())
        },
        true,
    )?;
    eprintln!("scenario probe glitch: peak {peak} ns (spec 100)");
    assert!(peak <= 120, "{peak}");
    assert!(run.flagged(flags::D_GLITCH) >= 1);
    assert_eq!(run.flagged(flags::WALL_STEP), 0);
    Ok(())
}

#[test]
fn scenario_a_glitch_then_a_real_step() -> Res {
    let s = Scenario {
        name: "glitch, then +5 ms",
        run_s: 30,
        end_s: 1,
        peak_ns: 5_000_000,
        settle_s: 18,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            r.host.glitch = -20_000;
            r.wait(NS / 2, 1)?;
            r.update();
            r.host.glitch = 0;
            r.host.wall.step(at + NS, 5_000_000);
            r.wait(NS / 2, 1)
        },
        true,
    )?;
    assert_eq!(run.flagged(flags::WALL_STEP), 1, "the step recognised");
    Ok(())
}

/// A counter jump (live migration): the wall clock seems to step back with
/// the step probe unmoved; flagged, never taken for a rate, slewed at the
/// cap.
#[test]
fn scenario_counter_jump() -> Res {
    let s = Scenario {
        name: "counter jump 5 ms",
        run_s: 30,
        end_s: 0,
        peak_ns: 5_000_000,
        settle_s: 17,
    };
    let run = check_scenario(
        &s,
        |r, at| {
            r.host.wall.jump(at, 5_000_000);
            Ok(())
        },
        true,
    )?;
    assert_eq!(run.flagged(flags::FAST_SLEW_OR_JUMP), 1, "flagged once");
    assert_eq!(run.flagged(flags::WALL_STEP), 0, "not a wall step");
    Ok(())
}

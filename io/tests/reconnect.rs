//! `Reconnect`: backoff, attempt limits, the stable-period reset, the
//! application's veto, the connect gate and `defer`, against a stand-in
//! driver (every platform), then reconnecting a WebSocket through a real
//! driver on every backend (Linux).
#![allow(missing_docs)]

use std::collections::VecDeque;

use ergon_io::outcome::{self, Verdict};
use ergon_io::reconnect::{Closed, Connector, LinkState, Opened, Reconnect, Retry};
use ergon_io::{ConnId, Error};

#[cfg(target_os = "linux")]
mod support;

type R<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

const MS: i64 = 1_000_000;

/// Records each dial it is handed and gives it a fresh id; `refuse` makes
/// `start` fail instead, and so does a dial below `refuse_below`.
#[derive(Default)]
struct Fake {
    started: Vec<(u32, bool, ConnId)>,
    refuse: bool,
    refuse_below: u32,
}

impl Connector<u32> for Fake {
    fn start(&mut self, dial: u32, background: bool) -> ergon_io::Result<ConnId> {
        if self.refuse || dial < self.refuse_below {
            return Err(Error::Busy("refused"));
        }
        let n = u32::try_from(self.started.len()).map_err(|_| Error::Busy("too many"))?;
        let id = ConnId::from_parts(n, 1);
        self.started.push((dial, background, id));
        Ok(id)
    }
}

/// No jitter: every delay is exact.
const fn exact() -> Retry {
    Retry {
        initial_ms: 100,
        max_ms: 1_000,
        multiplier: 2.0,
        jitter: 0.0,
        max_attempts: 0,
        stable_ms: 5_000,
        seed: 1,
        background: true,
    }
}

/// The id of the connect started last.
fn last(f: &Fake) -> R<ConnId> {
    Ok(f.started.last().ok_or("nothing started")?.2)
}

#[test]
fn retry_defaults() {
    let r = Retry::default();
    assert_eq!((r.initial_ms, r.max_ms), (250, 30_000));
    assert!((r.multiplier - 2.0).abs() < f64::EPSILON);
    assert!((r.jitter - 0.2).abs() < f64::EPSILON);
    assert_eq!(r.max_attempts, 0, "unlimited");
    assert_eq!(r.stable_ms, 60_000);
    assert!(r.background, "handshakes off the loop");
}

#[test]
fn bad_policies_are_refused() {
    let ok = exact();
    for (r, why) in [
        (
            Retry {
                initial_ms: 0,
                ..ok
            },
            "zero initial delay",
        ),
        (Retry { max_ms: 50, ..ok }, "max below initial"),
        (
            Retry {
                multiplier: 0.5,
                ..ok
            },
            "shrinking delays",
        ),
        (
            Retry {
                multiplier: f64::NAN,
                ..ok
            },
            "NaN multiplier",
        ),
        (Retry { jitter: 1.5, ..ok }, "jitter above 1"),
        (Retry { jitter: -0.1, ..ok }, "negative jitter"),
    ] {
        assert!(Reconnect::<u32>::new(r).is_err(), "accepted {why}");
    }
}

// ANCHOR: backoff
#[test]
fn delays_grow_by_the_multiplier_up_to_the_max() -> R {
    let mut rc = Reconnect::new(exact())?;
    let mut f = Fake::default();
    let link = rc.add(|| Ok(7));
    assert_eq!(rc.poll_at(&mut f, 0), 1, "the first connect is immediate");
    let mut now = 0;
    let mut delays = Vec::new();
    for _ in 0..6 {
        let id = last(&f)?;
        match rc.on_closed_at(id, now) {
            Some(Closed::Retry {
                link: l,
                attempt,
                delay_ms,
            }) => {
                assert_eq!(l, link);
                delays.push((attempt, delay_ms));
                now += i64::try_from(delay_ms)? * MS;
            }
            other => return Err(format!("{other:?}").into()),
        }
        // Not yet due a moment before, due at the delay.
        assert_eq!(rc.poll_at(&mut f, now - 1), 0);
        assert_eq!(rc.poll_at(&mut f, now), 1);
    }
    assert_eq!(
        delays,
        vec![(1, 100), (2, 200), (3, 400), (4, 800), (5, 1000), (6, 1000)]
    );
    assert!(f.started.iter().all(|&(dial, bg, _)| dial == 7 && bg));
    Ok(())
}
// ANCHOR_END: backoff

#[test]
fn jitter_shortens_each_delay_the_same_way_for_a_seed() -> R {
    let run = |seed: u64| -> R<Vec<u64>> {
        let mut rc = Reconnect::new(Retry {
            jitter: 0.5,
            seed,
            ..exact()
        })?;
        let mut f = Fake::default();
        rc.add(|| Ok(1));
        rc.poll_at(&mut f, 0);
        let mut out = Vec::new();
        let mut now = 0;
        for _ in 0..20 {
            let Some(Closed::Retry { delay_ms, .. }) = rc.on_closed_at(last(&f)?, now) else {
                return Err("no retry".into());
            };
            out.push(delay_ms);
            now += i64::try_from(delay_ms)? * MS;
            rc.poll_at(&mut f, now);
        }
        Ok(out)
    };
    let a = run(42)?;
    assert_eq!(a, run(42)?, "same seed, same delays");
    assert_ne!(a, run(43)?, "another seed, other delays");
    // Each delay lies in [base * (1 - jitter), base].
    for (i, &d) in a.iter().enumerate() {
        let base = (100u64 << i.min(10)).min(1_000);
        assert!(
            d >= base / 2 && d <= base,
            "attempt {}: {d} of {base}",
            i + 1
        );
    }
    assert!(a.windows(2).any(|w| w[0] != w[1] && w[0] >= 1_000 / 2));
    Ok(())
}

#[test]
fn max_attempts_gives_up_until_resumed() -> R {
    let mut rc = Reconnect::new(Retry {
        max_attempts: 2,
        ..exact()
    })?;
    let mut f = Fake::default();
    let link = rc.add(|| Ok(1));
    rc.poll_at(&mut f, 0);
    let mut now = 0;
    for want in 1..=2 {
        let Some(Closed::Retry {
            attempt, delay_ms, ..
        }) = rc.on_closed_at(last(&f)?, now)
        else {
            return Err("no retry".into());
        };
        assert_eq!(attempt, want);
        now += i64::try_from(delay_ms)? * MS;
        assert_eq!(rc.poll_at(&mut f, now), 1);
    }
    assert_eq!(
        rc.on_closed_at(last(&f)?, now),
        Some(Closed::GaveUp { link, attempts: 2 })
    );
    assert_eq!(rc.state(link), Some(LinkState::GaveUp));
    assert_eq!(
        rc.poll_at(&mut f, now + 3_600_000 * MS),
        0,
        "stays given up"
    );
    rc.resume(link);
    assert_eq!(rc.poll_at(&mut f, now), 1, "resume connects at once");
    Ok(())
}

#[test]
fn a_stable_connection_resets_the_attempts() -> R {
    let mut rc = Reconnect::new(exact())?;
    let mut f = Fake::default();
    let link = rc.add(|| Ok(1));
    rc.poll_at(&mut f, 0);
    // Two quick failures: the delay grows.
    let mut now = 0;
    for _ in 0..2 {
        let Some(Closed::Retry { delay_ms, .. }) = rc.on_closed_at(last(&f)?, now) else {
            return Err("no retry".into());
        };
        now += i64::try_from(delay_ms)? * MS;
        rc.poll_at(&mut f, now);
    }
    // Open, but closed again before the stable period: still growing.
    let id = last(&f)?;
    // Its first open: nothing can be stale yet.
    assert_eq!(
        rc.on_open_at(id, now),
        Some(Opened {
            link,
            reconnect: false
        })
    );
    assert_eq!(rc.state(link), Some(LinkState::Open(id)));
    now += 4_999 * MS;
    assert!(matches!(
        rc.on_closed_at(id, now),
        Some(Closed::Retry { attempt: 3, .. })
    ));
    rc.poll_at(&mut f, now + 400 * MS);
    // Open for the stable period: the next delay is the first again.
    let id = last(&f)?;
    now += 400 * MS;
    assert_eq!(
        rc.on_open_at(id, now),
        Some(Opened {
            link,
            reconnect: true
        })
    );
    now += 5_000 * MS;
    assert!(matches!(
        rc.on_closed_at(id, now),
        Some(Closed::Retry {
            attempt: 1,
            delay_ms: 100,
            ..
        })
    ));
    Ok(())
}

#[test]
fn the_app_can_veto_and_resume() -> R {
    let mut rc = Reconnect::new(exact())?;
    let mut f = Fake::default();
    let link = rc.add(|| Ok(1));
    rc.poll_at(&mut f, 0);
    let id = last(&f)?;
    assert_eq!(
        rc.on_open_at(id, 0),
        Some(Opened {
            link,
            reconnect: false
        })
    );
    assert_eq!(rc.link_of(id), Some(link));
    // Stopped while open: the close is reported, nothing is scheduled.
    rc.stop(link);
    assert_eq!(rc.on_closed_at(id, MS), Some(Closed::Stopped { link }));
    assert_eq!(rc.poll_at(&mut f, 3_600_000 * MS), 0);
    assert_eq!(rc.state(link), Some(LinkState::Stopped));
    // Vetoing a scheduled retry.
    rc.resume(link);
    rc.poll_at(&mut f, 2 * MS);
    let id = last(&f)?;
    assert!(matches!(
        rc.on_closed_at(id, 3 * MS),
        Some(Closed::Retry { .. })
    ));
    rc.stop(link);
    assert_eq!(rc.poll_at(&mut f, 3_600_000 * MS), 0);
    // Unknown and stale ids are not ours.
    assert_eq!(rc.on_closed_at(id, 4 * MS), None);
    assert_eq!(rc.on_open_at(ConnId::from_parts(99, 1), 4 * MS), None);
    Ok(())
}

#[test]
fn a_failed_start_counts_as_a_failed_attempt() -> R {
    let mut rc = Reconnect::new(exact())?;
    let mut f = Fake {
        refuse: true,
        ..Fake::default()
    };
    let calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let c = calls.clone();
    let link = rc.add(move || {
        c.set(c.get() + 1);
        if c.get() == 1 {
            Err(Error::Config("first dial fails".into()))
        } else {
            Ok(1)
        }
    });
    // The dial itself fails, then the start fails: two failed attempts.
    assert_eq!(rc.poll_at(&mut f, 0), 0);
    assert!(matches!(
        rc.state(link),
        Some(LinkState::Waiting { attempt: 1, .. })
    ));
    assert_eq!(rc.poll_at(&mut f, 100 * MS), 0);
    assert!(matches!(
        rc.state(link),
        Some(LinkState::Waiting { attempt: 2, .. })
    ));
    f.refuse = false;
    assert_eq!(rc.poll_at(&mut f, 300 * MS), 1);
    assert!(matches!(rc.state(link), Some(LinkState::Connecting(_))));
    assert_eq!(calls.get(), 3);
    Ok(())
}

/// A driver whose `connect_background` has no CPU for its helper: a pod
/// with one CPU, without `inline_connect_fallback`.
#[derive(Default)]
struct OneCpuPod {
    tries: u32,
}

impl Connector<u32> for OneCpuPod {
    fn start(&mut self, _: u32, background: bool) -> ergon_io::Result<ConnId> {
        self.tries += 1;
        assert!(background, "Retry::background is the default");
        Err(ergon_io::placement::CpuError::NoSpareCpu {
            helper: ergon_io::placement::HelperThread::Dial,
            polling: Some(0),
            allowed: ergon_tune::CpuSet::of([0]),
        }
        .into())
    }
}

/// No retry can succeed while the process has one CPU: the link gives up
/// at once instead of retrying forever unseen; `resume` tries again.
#[test]
fn no_cpu_for_the_connect_helper_gives_up_at_once() -> R {
    let mut rc = Reconnect::new(exact())?;
    let mut pod = OneCpuPod::default();
    let link = rc.add(|| Ok(7));
    assert_eq!(rc.poll_at(&mut pod, 0), 0);
    assert_eq!(rc.state(link), Some(LinkState::GaveUp));
    assert_eq!(rc.poll_at(&mut pod, 60_000 * MS), 0, "no retry");
    assert_eq!(pod.tries, 1);
    rc.resume(link);
    assert_eq!(rc.poll_at(&mut pod, 60_001 * MS), 0);
    assert_eq!((pod.tries, rc.state(link)), (2, Some(LinkState::GaveUp)));
    Ok(())
}

// ---- The connect gate and `defer` ----

/// `MS` as an unsigned wait.
const MSU: u64 = 1_000_000;

// ANCHOR: pace
/// The application's connect pacing (ergon-io has none): at most `limit`
/// starts in any `window_ns`, a hold until a time (a 429's wait), and a
/// halt that only `clear` ends (a ban).
struct Pace {
    limit: usize,
    window_ns: i64,
    starts: VecDeque<i64>,
    held_until: i64,
    halted: bool,
}

impl Pace {
    const fn new(limit: usize, window_ns: i64) -> Self {
        Self {
            limit,
            window_ns,
            starts: VecDeque::new(),
            held_until: 0,
            halted: false,
        }
    }

    /// The gate: `Ok(())` and count a start, or `Err(wait_ns)`.
    fn try_start(&mut self, now: i64) -> Result<(), u64> {
        if self.halted {
            return Err(u64::MAX);
        }
        if now < self.held_until {
            return Err(self.held_until.abs_diff(now));
        }
        while self
            .starts
            .front()
            .is_some_and(|&t| t + self.window_ns <= now)
        {
            self.starts.pop_front();
        }
        match self.starts.front() {
            Some(&oldest) if self.starts.len() >= self.limit => {
                Err((oldest + self.window_ns).abs_diff(now))
            }
            _ => {
                self.starts.push_back(now);
                Ok(())
            }
        }
    }
}
// ANCHOR_END: pace

/// One pass with `pace` as the gate.
fn gated(rc: &mut Reconnect<u32>, f: &mut Fake, pace: &mut Pace, now: i64) -> usize {
    rc.poll_at_gated(f, now, || pace.try_start(now))
}

#[test]
fn a_gate_paces_fifty_links_and_starts_every_one() -> R {
    // ANCHOR: gate
    // Connect attempts per IP: 3 per second (an illustrative service
    // limit). One pacer, the gate of every WebSocket link on that IP.
    let mut pace = Pace::new(3, 1_000 * MS);
    let mut links = Reconnect::new(Retry::default())?;
    // ANCHOR_END: gate
    let mut f = Fake::default();
    // Fifty links added at once: their first attempts pass the gate too.
    let ids: Vec<_> = (0..50).map(|i| links.add(move || Ok(i))).collect();
    let mut starts = Vec::new();
    let mut now = 0;
    while now <= 30_000 * MS {
        let before = f.started.len();
        // ANCHOR: gate_poll
        // Once per loop pass; `now` is the pacer's clock, read once.
        // Ok(()): connect now; Err(wait_ns): ask again after the wait.
        let started = links.poll_at_gated(&mut f, now, || pace.try_start(now));
        // ANCHOR_END: gate_poll
        assert_eq!(started, f.started.len() - before);
        starts.extend(f.started[before..].iter().map(|s| (now, s.0)));
        if now == 5_000 * MS {
            // A refusal is not a failed attempt: no backoff grew.
            for &l in &ids {
                if let Some(LinkState::Waiting { attempt, .. }) = links.state(l) {
                    assert_eq!(attempt, 0, "{l:?}");
                }
            }
        }
        now += MS;
    }
    let mut dials: Vec<u32> = starts.iter().map(|s| s.1).collect();
    dials.sort_unstable();
    assert_eq!(
        dials,
        (0..50).collect::<Vec<_>>(),
        "every link started once"
    );
    assert!(
        ids.iter()
            .all(|&l| matches!(links.state(l), Some(LinkState::Connecting(_))))
    );
    // Never more than 3 starts in any second.
    for (i, &(t, _)) in starts.iter().enumerate() {
        let n = starts[i..]
            .iter()
            .take_while(|s| s.0 < t + 1_000 * MS)
            .count();
        assert!(n <= 3, "{n} starts in the second from {t} ns");
    }
    Ok(())
}

#[test]
fn a_gate_refusal_is_not_a_failed_attempt() -> R {
    let mut rc = Reconnect::new(exact())?;
    let mut f = Fake::default();
    let link = rc.add(|| Ok(1));
    assert_eq!(rc.poll_at_gated(&mut f, 0, || Ok(())), 1);
    assert!(matches!(
        rc.on_closed_at(last(&f)?, 0),
        Some(Closed::Retry {
            attempt: 1,
            delay_ms: 100,
            ..
        })
    ));
    // Five refusals of 50 ms each: still the first attempt, and the gate
    // is not asked again before each wait ends.
    let mut now = 100 * MS;
    for _ in 0..5 {
        assert_eq!(rc.poll_at_gated(&mut f, now, || Err(50 * MSU)), 0);
        assert_eq!(
            rc.state(link),
            Some(LinkState::Waiting {
                at_ns: now + 50 * MS,
                attempt: 1
            })
        );
        let mut asked = false;
        rc.poll_at_gated(&mut f, now + 50 * MS - 1, || {
            asked = true;
            Ok(())
        });
        assert!(!asked, "asked before the wait ended");
        now += 50 * MS;
    }
    assert_eq!(rc.poll_at_gated(&mut f, now, || Ok(())), 1);
    // The next close: attempt 2 after 200 ms, as if the gate never refused.
    assert!(matches!(
        rc.on_closed_at(last(&f)?, now),
        Some(Closed::Retry {
            attempt: 2,
            delay_ms: 200,
            ..
        })
    ));
    // A wait longer than `max_ms` (a halted gate may answer about 292
    // years) is cut to it: the gate is asked again, so a cleared halt
    // restarts the link.
    now += 200 * MS;
    assert_eq!(rc.poll_at_gated(&mut f, now, || Err(u64::MAX)), 0);
    assert_eq!(
        rc.state(link),
        Some(LinkState::Waiting {
            at_ns: now + 1_000 * MS,
            attempt: 2
        })
    );
    Ok(())
}

#[test]
fn no_link_starves_behind_links_that_keep_failing() -> R {
    // Links 0 and 1 fail on every attempt and are due again 10 ms later;
    // the gate passes one attempt per 100 ms. Each pass starts after the
    // last link the gate passed, so link 2 gets its turn.
    let mut rc = Reconnect::new(Retry {
        initial_ms: 10,
        max_ms: 10,
        ..exact()
    })?;
    let mut f = Fake {
        refuse_below: 2,
        ..Fake::default()
    };
    for i in 0..3 {
        rc.add(move || Ok(i));
    }
    let mut next_pass = 0;
    let mut now = 0;
    while f.started.is_empty() {
        assert!(now < 1_000 * MS, "link 2 starved");
        rc.poll_at_gated(&mut f, now, || {
            if now >= next_pass {
                next_pass = now + 100 * MS;
                Ok(())
            } else {
                Err(u64::try_from(next_pass - now).unwrap_or(0))
            }
        });
        now += MS;
    }
    assert_eq!(f.started[0].0, 2);
    Ok(())
}

#[test]
fn defer_wins_over_the_backoff_and_survives_a_later_close() -> R {
    let mut rc = Reconnect::new(exact())?;
    let mut f = Fake::default();
    let link = rc.add(|| Ok(1));
    // Before the first attempt: the first connect waits too.
    rc.defer_at(link, 500 * MSU, 0);
    assert_eq!(rc.poll_at(&mut f, 500 * MS - 1), 0);
    assert_eq!(rc.poll_at(&mut f, 500 * MS), 1);
    // While connecting (a server's Retry-After), then closed: the 100 ms
    // backoff gives way to the wait.
    let mut now = 500 * MS;
    rc.defer_at(link, 2_000 * MSU, now);
    assert_eq!(
        rc.on_closed_at(last(&f)?, now + 10 * MS),
        Some(Closed::Retry {
            link,
            attempt: 1,
            delay_ms: 1_990
        })
    );
    assert_eq!(rc.poll_at(&mut f, now + 2_000 * MS - 1), 0);
    assert_eq!(rc.poll_at(&mut f, now + 2_000 * MS), 1);
    now += 2_000 * MS;
    // Once past, the backoff again.
    assert!(matches!(
        rc.on_closed_at(last(&f)?, now),
        Some(Closed::Retry {
            attempt: 2,
            delay_ms: 200,
            ..
        })
    ));
    // A shorter wait never brings an attempt forward; a longer one pushes
    // it out.
    rc.defer_at(link, 50 * MSU, now);
    assert_eq!(
        rc.state(link),
        Some(LinkState::Waiting {
            at_ns: now + 200 * MS,
            attempt: 2
        })
    );
    rc.defer_at(link, 700 * MSU, now);
    assert_eq!(
        rc.state(link),
        Some(LinkState::Waiting {
            at_ns: now + 700 * MS,
            attempt: 2
        })
    );
    // `resume` after a `stop` waits for it too, on the gated path as well.
    rc.stop(link);
    rc.resume(link);
    assert_eq!(rc.poll_at_gated(&mut f, now + 700 * MS - 1, || Ok(())), 0);
    assert_eq!(rc.poll_at_gated(&mut f, now + 700 * MS, || Ok(())), 1);
    Ok(())
}

#[test]
fn a_refused_upgrade_holds_every_link_on_its_gate() -> R {
    let mut pace = Pace::new(5, 1_000 * MS);
    let mut rc = Reconnect::new(exact())?;
    let mut f = Fake::default();
    let a = rc.add(|| Ok(1));
    let b = rc.add(|| Ok(2));
    assert_eq!(gated(&mut rc, &mut f, &mut pace, 0), 2);
    let ids = [f.started[0].2, f.started[1].2];

    // ANCHOR: refused
    // In `Handler::on_closed(id, error)`: a refused handshake (429) holds
    // the connect gate for the server's wait, and the link waits it out.
    let on_closed = |rc: &mut Reconnect<u32>, pace: &mut Pace, id, error: &Error, now: i64| -> R {
        let link = rc.link_of(id).ok_or("not a link")?;
        if let Some(Verdict::Throttled { retry_after_ms }) = outcome::refused_upgrade(error)? {
            // No wait named: this application's own backoff, 1 s.
            let wait_ns = retry_after_ms.unwrap_or(1_000) * 1_000_000;
            pace.held_until = now + i64::try_from(wait_ns)?;
            rc.defer_at(link, wait_ns, now);
        }
        rc.on_closed_at(id, now);
        Ok(())
    };
    // ANCHOR_END: refused

    // A 429 with Retry-After 5 s on one link, a plain close on the other.
    let throttled = Error::Upgrade {
        status: 429,
        body: String::new(),
        retry_after_ms: Some(5_000),
        retry_after_invalid: false,
    };
    let mut now = 10 * MS;
    on_closed(&mut rc, &mut pace, ids[0], &throttled, now)?;
    on_closed(&mut rc, &mut pace, ids[1], &Error::Closed, now)?;
    let until = pace.held_until;
    assert_eq!(until, now + 5_000 * MS, "the server's wait");
    // Neither link starts before the hold ends; both start once it has.
    while f.started.len() < 4 {
        let n = gated(&mut rc, &mut f, &mut pace, now);
        assert!(
            n == 0 || now >= until,
            "a start at {now} ns, held until {until}"
        );
        assert!(now < until + 100 * MS, "not restarted after the hold");
        now += MS;
    }

    // Application policy can halt the shared gate (a ban): it answers
    // `u64::MAX` until a person clears it.
    let ids = [f.started[2].2, f.started[3].2];
    pace.halted = true;
    on_closed(&mut rc, &mut pace, ids[0], &Error::Closed, now)?;
    on_closed(&mut rc, &mut pace, ids[1], &Error::Closed, now)?;
    let hour = now + 3_600_000 * MS;
    while now < hour {
        assert_eq!(gated(&mut rc, &mut f, &mut pace, now), 0);
        now += 10 * MS;
    }
    assert!(matches!(rc.state(a), Some(LinkState::Waiting { .. })));
    pace.halted = false;
    let cleared = now;
    while f.started.len() < 6 {
        gated(&mut rc, &mut f, &mut pace, now);
        assert!(
            now <= cleared + 1_000 * MS,
            "not restarted within max_ms of the clear"
        );
        now += MS;
    }
    assert!(matches!(rc.state(b), Some(LinkState::Connecting(_))));
    Ok(())
}

/// A WebSocket that the server closes is reopened, with the backoff, through
/// a real driver on every backend: first a close with a code, then refused
/// connects while nothing listens, then a server again.
#[cfg(target_os = "linux")]
mod with_driver {
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    use ergon_io::placement::HelperThread;
    use ergon_io::reconnect::{Closed, Reconnect, Retry};
    use ergon_io::{
        Backend, Config, ConnId, Driver, Error, Handler, RxMeta, WsConn, WsDial, WsHandler,
        WsOptions, ws,
    };

    use super::{R, support};

    // ANCHOR: app
    /// The application: the reconnect helper lives in the handler, which
    /// feeds it every open and close.
    struct App {
        links: Reconnect<WsDial>,
        /// Connections to subscribe on (new and reopened alike).
        subscribe: Vec<ConnId>,
        msgs: Vec<Vec<u8>>,
        retries: Vec<Closed>,
        errors: Vec<String>,
    }

    impl Handler for App {
        fn on_open(&mut self, id: ConnId) {
            if self.links.on_open(id).is_some() {
                // Sends wait until `poll` returns: remember the connection.
                self.subscribe.push(id);
            }
        }
        fn on_closed(&mut self, id: ConnId, error: Error) {
            self.errors.push(error.to_string());
            if let Some(next) = self.links.on_closed(id) {
                // To veto: `self.links.stop(link)` here.
                self.retries.push(next);
            }
        }
    }

    impl WsHandler for App {
        fn on_message(&mut self, _: ConnId, _: u8, payload: &[u8], _: &RxMeta, _: u16) {
            self.msgs.push(payload.to_vec());
        }
    }

    /// One pass of the loop: the driver, the helper, then the subscriptions.
    fn turn(d: &mut Driver<WsConn>, app: &mut App) -> R {
        d.poll(app)?;
        app.links.poll(d);
        for id in app.subscribe.drain(..) {
            if let Some(mut c) = d.get_mut(id) {
                c.send_text("subscribe")?;
            }
        }
        Ok(())
    }
    // ANCHOR_END: app

    fn backend(b: Backend) -> R {
        let lab = support::Lab::new()?;
        let addr = lab.addr()?;
        let target = lab.target("/")?;
        let tls = lab.client_cfg.clone();
        let retry = Retry {
            initial_ms: 20,
            max_ms: 80,
            jitter: 0.0,
            ..Retry::default()
        };
        let mut app = App {
            links: Reconnect::new(retry)?,
            subscribe: Vec::new(),
            msgs: Vec::new(),
            retries: Vec::new(),
            errors: Vec::new(),
        };
        let opts = WsOptions {
            ping_interval_ms: 0,
            ..WsOptions::default()
        };
        app.links
            .add(move || WsDial::with_options(target.clone(), tls.clone(), opts.clone()));
        let mut cfg = Config::new(b);
        // A pod with one CPU has no core for the connect helper: opt in to
        // opening on the loop there, so the reconnect lifecycle runs on any
        // pod (the refusal without this is `no_cpu_for_the_connect_helper_
        // gives_up_at_once`, and the helper-side placement is driver.rs's
        // `background_helper_leaves_the_polling_cpu`).
        cfg.inline_connect_fallback = true;
        let mut d: Driver<WsConn> = Driver::new(cfg)?;
        let until = |d: &mut Driver<WsConn>, app: &mut App, done: &dyn Fn(&App) -> bool| -> R {
            let end = Instant::now() + Duration::from_secs(10);
            while !done(app) {
                turn(d, app)?;
                if Instant::now() > end {
                    return Err(format!("timed out: {:?}", app.errors).into());
                }
            }
            Ok(())
        };
        // First connection: subscribed, then closed by the server.
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_within(Duration::from_secs(15))?;
                assert_eq!(p.recv()?, (ws::OP_TEXT, b"subscribe".to_vec()));
                p.send(ws::OP_TEXT, true, b"first")?;
                p.send_close(Some(1001), "going away")?;
                assert_eq!(p.recv()?.0, ws::OP_CLOSE);
                Ok(())
            });
            until(&mut d, &mut app, &|a| !a.retries.is_empty())?;
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
        assert!(app.errors[0].contains("1001"), "{:?}", app.errors);
        // Nothing listens: connects are refused and retried with backoff.
        drop(lab.listener);
        until(&mut d, &mut app, &|a| a.retries.len() >= 4)?;
        assert!(
            app.retries
                .iter()
                .skip(1)
                .all(|r| matches!(r, Closed::Retry { .. })),
            "{:?}",
            app.retries
        );
        let delays: Vec<u64> = app
            .retries
            .iter()
            .filter_map(|r| match r {
                Closed::Retry { delay_ms, .. } => Some(*delay_ms),
                _ => None,
            })
            .collect();
        assert_eq!(&delays[..4], &[20, 40, 80, 80], "{b}");
        // A server again: the link reopens and subscribes again.
        let listener = TcpListener::bind(addr)?;
        let lab = support::Lab {
            listener,
            server_cfg: lab.server_cfg,
            client_cfg: lab.client_cfg,
            cert_pem: lab.cert_pem,
        };
        std::thread::scope(|s| -> R {
            let srv = s.spawn(|| -> R {
                let mut p = lab.accept_within(Duration::from_secs(15))?;
                assert_eq!(p.recv()?, (ws::OP_TEXT, b"subscribe".to_vec()));
                p.send(ws::OP_TEXT, true, b"second")?;
                assert!(p.client_closed()?);
                Ok(())
            });
            until(&mut d, &mut app, &|a| a.msgs.len() == 2)?;
            drop(d);
            srv.join().map_err(|_| "server panicked")??;
            Ok(())
        })?;
        assert_eq!(app.msgs, vec![b"first".to_vec(), b"second".to_vec()]);
        Ok(())
    }

    #[test]
    fn a_closed_websocket_is_reopened_on_every_backend() -> R {
        let mut ran = 0;
        for b in Backend::ALL {
            match backend(b) {
                Ok(()) => ran += 1,
                // A pod with one CPU has no core for `uring-sqpoll`'s kernel
                // thread: `Driver::new` refuses typed (asserted; covered in
                // full by driver.rs's
                // `sqpoll_thread_never_shares_the_polling_cpu`). The body
                // still runs on every other backend.
                Err(e)
                    if b == Backend::UringSqpoll && support::refused(&*e, HelperThread::SqPoll) => {
                }
                Err(e) => return Err(format!("{b}: {e}").into()),
            }
        }
        assert!(ran >= Backend::ALL.len() - 1, "only {ran} backends ran");
        Ok(())
    }
}

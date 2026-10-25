//! The IPC baseline: `std::sync::mpsc::sync_channel`.
//!
//! One bounded channel per stream, `try_send` / `try_recv` busy-spin on both
//! ends (never the unbounded channel: it allocates per block). Both ends in
//! one process (`--role local`), each on its own thread:
//!
//! | `--round-trip` | thread 0 (`--cpus` first) | thread 1 (second) | row |
//! |---|---|---|---|
//! | absent | sender: paces, sends stream `k mod N` | receiver: polls the N receivers round robin, records | `oneway` |
//! | given | ping: paces, sends, polls the N reply channels, records | echo: polls the N ping channels, echoes on the stream's reply channel | `load` |
//!
//! The message is the core's 64 bytes, sent by value (`[u8; 64]`). Every
//! measurement, the echo and the files are [`point`](super::point)'s.

#![deny(unsafe_code, missing_docs)]
#![warn(
    clippy::all,
    clippy::pedantic,
    clippy::nursery,
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic
)]

use super::point::{
    self, Common, Error, POLL_LIMIT, Progress, Recorder, Report, Role, Rx, START_DELAY_NS,
    SendStats, Sent, Tx, thread_allocs,
};
use ergon_latency_core::clock::TscClock;
use ergon_latency_core::manifest::RunManifest;
use ergon_latency_core::message::{MESSAGE_LEN, template};
use std::sync::Barrier;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel};

/// One message as the channel carries it.
pub type Msg = [u8; MESSAGE_LEN];

/// Busy threads: two, whatever the variant.
pub const BUSY_THREADS: u32 = 2;

/// `--cpus` order.
pub const THREAD_ORDER: &str =
    "sender,receiver (one way) or ping,echo (--round-trip): two CPUs, thread 0 first";

/// The sending ends, one per stream.
#[derive(Debug)]
pub struct MpscTx {
    tx: Vec<SyncSender<Msg>>,
}

/// The receiving ends, one per stream.
#[derive(Debug)]
pub struct MpscRx {
    rx: Vec<Receiver<Msg>>,
    closed: bool,
}

/// `streams` channels of `capacity` messages each.
#[must_use]
pub fn channels(streams: u32, capacity: usize) -> (MpscTx, MpscRx) {
    let (tx, rx) = (0..streams).map(|_| sync_channel(capacity)).unzip();
    (MpscTx { tx }, MpscRx { rx, closed: false })
}

impl Tx for MpscTx {
    #[inline]
    fn try_send(&mut self, stream: u32, msg: &Msg) -> Result<Sent, Error> {
        let tx = self
            .tx
            .get(stream as usize)
            .ok_or_else(|| Error::Transport(format!("no channel for stream {stream}")))?;
        match tx.try_send(*msg) {
            Ok(()) => Ok(Sent::Sent),
            Err(TrySendError::Full(_)) => Ok(Sent::Busy),
            Err(TrySendError::Disconnected(_)) => {
                Err(Error::Transport("the receiving thread is gone".into()))
            }
        }
    }
}

impl Rx for MpscRx {
    #[inline]
    fn poll<F: FnMut(&[u8])>(&mut self, on: &mut F) -> Result<usize, Error> {
        let mut read = 0;
        let mut gone = 0;
        for rx in &self.rx {
            for _ in 0..POLL_LIMIT {
                match rx.try_recv() {
                    Ok(m) => {
                        on(&m);
                        read += 1;
                    }
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        gone += 1;
                        break;
                    }
                }
            }
        }
        if gone == self.rx.len() {
            self.closed = true;
        }
        Ok(read)
    }

    fn closed(&self) -> bool {
        self.closed
    }
}

/// The arm's settings.
#[derive(Debug, Clone)]
pub struct Options {
    /// `--capacity`: messages each channel holds.
    pub capacity: usize,
}

fn joined<T>(r: std::thread::Result<Result<T, Error>>, who: &str) -> Result<T, Error> {
    r.map_err(|_| Error::Transport(format!("the {who} thread panicked")))?
}

/// What the measuring threads hand back.
struct Ends {
    rec: Recorder,
    send: SendStats,
    allocs: u64,
    start_realtime_ns: u128,
}

fn oneway(c: &Common, clock: &TscClock, o: &Options) -> Result<Ends, Error> {
    let (mut tx, mut rx) = channels(c.streams, o.capacity);
    let (barrier, start, start_rt) = (Barrier::new(2), AtomicU64::new(0), AtomicU64::new(0));
    let progress = Progress::default();
    let (barrier, start, start_rt, progress) = (&barrier, &start, &start_rt, &progress);
    std::thread::scope(|s| {
        // Each end owns its channel ends, so one that stops drops them and
        // the other's next send or receive sees it.
        let sender = s.spawn(move || {
            let pinned = point::pin(c.cpu(0));
            barrier.wait();
            let (t, rt) = point::start_at(clock, START_DELAY_NS);
            start.store(t, Ordering::Release);
            start_rt.store(u64::try_from(rt).unwrap_or(u64::MAX), Ordering::Release);
            barrier.wait();
            pinned.inspect_err(|_| progress.fail())?;
            let a0 = thread_allocs();
            let send = point::send_oneway(clock, c, t, &mut tx, progress)?;
            Ok((send, thread_allocs() - a0))
        });
        let receiver = s.spawn(move || {
            let pinned = point::pin(c.cpu(1));
            barrier.wait();
            barrier.wait();
            pinned.inspect_err(|_| progress.fail())?;
            let t = start.load(Ordering::Acquire);
            let mut rec = Recorder::new(clock, t, c).inspect_err(|_| progress.fail())?;
            let a0 = thread_allocs();
            point::recv_oneway(clock, c, t, &mut rx, &mut rec, progress)?;
            Ok((rec, thread_allocs() - a0))
        });
        let ((send, sa), (rec, ra)) = point::both(
            joined(sender.join(), "sender"),
            joined(receiver.join(), "receiver"),
        )?;
        Ok(Ends {
            rec,
            send,
            allocs: sa + ra,
            start_realtime_ns: u128::from(start_rt.load(Ordering::Acquire)),
        })
    })
}

fn round_trip(c: &Common, clock: &TscClock, o: &Options) -> Result<Ends, Error> {
    let (mut ping_tx, mut ping_rx) = channels(c.streams, o.capacity);
    let (mut echo_tx, mut echo_rx) = channels(c.streams, o.capacity);
    let (barrier, stop) = (Barrier::new(2), AtomicBool::new(false));
    let (barrier, stop) = (&barrier, &stop);
    std::thread::scope(|s| {
        // Each end owns its channel ends: an echo that stops drops them, and
        // the ping's next send fails.
        let echo = s.spawn(move || {
            let pinned = point::pin(c.cpu(1));
            barrier.wait();
            pinned?;
            let a0 = thread_allocs();
            point::echo(&mut ping_rx, &mut echo_tx, &mut || {
                !stop.load(Ordering::Acquire)
            })?;
            Ok(thread_allocs() - a0)
        });
        let ping = s.spawn(move || {
            let pinned = point::pin(c.cpu(0));
            barrier.wait();
            let run = || {
                pinned?;
                let (t, rt) = point::start_at(clock, START_DELAY_NS);
                let mut rec = Recorder::new(clock, t, c)?;
                let a0 = thread_allocs();
                let send = point::ping(clock, c, t, &mut ping_tx, &mut echo_rx, &mut rec)?;
                let allocs = thread_allocs() - a0;
                Ok(Ends {
                    rec,
                    send,
                    allocs,
                    start_realtime_ns: rt,
                })
            };
            let r = run();
            stop.store(true, Ordering::Release);
            r
        });
        let mut ends = joined(ping.join(), "ping")?;
        ends.allocs += joined(echo.join(), "echo")?;
        Ok(ends)
    })
}

/// Runs one point as `c` and `o` say and writes its files; returns the row.
///
/// # Errors
/// [`Error::Usage`] for a role other than `local` or a wrong `--cpus`,
/// else what the point met.
pub fn run(c: &Common, o: &Options) -> Result<String, Error> {
    if c.role != Role::Local {
        return Err(Error::Usage(format!(
            "--role {}: mpsc_lat is local only (both ends in this process)",
            c.role.name()
        )));
    }
    if o.capacity == 0 {
        return Err(Error::Usage(
            "--capacity 0 is a rendezvous channel: try_send never succeeds without a waiting receiver"
                .into(),
        ));
    }
    c.check_cpus(2, THREAD_ORDER)?;
    let clock = TscClock::new();
    let ends = if c.round_trip {
        round_trip(c, &clock, o)?
    } else {
        oneway(c, &clock, o)?
    };
    let end = clock.raw();
    let mut manifest = RunManifest::new("mpsc", &template(), MESSAGE_LEN as u64);
    manifest
        .setting("transport", "mpsc")
        .setting("capacity", &o.capacity.to_string())
        .setting("thread_order", THREAD_ORDER)
        .setting("channel_kind", "std::sync::mpsc::sync_channel");
    point::finish(
        c,
        &clock,
        ends.rec,
        ends.send,
        ends.start_realtime_ns,
        end,
        Report {
            channel: "ipc",
            retrans: None,
            wire_bytes: MESSAGE_LEN as u64,
            busy_threads: BUSY_THREADS,
            hot_path_allocs: ends.allocs,
            manifest,
        },
    )
}

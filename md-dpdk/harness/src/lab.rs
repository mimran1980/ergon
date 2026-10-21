//! Helpers shared by the `rtt`, `load` and `tput` benchmark clients (ported from
//! rusteron's x86 lab so the kernel-UDP and DPDK numbers come from the same
//! client code as rusteron's published ones).
//!
//! `HARNESS_DRIVER` picks the media driver a client runs on: unset or
//! `external`, the one at `AERON_DIR`, running on threads of its own; or
//! `invoker`, one the client embeds (Aeron's `INVOKER` threading mode),
//! configured from the environment exactly as the bundled `media_driver` is
//! (`AERON_*`, `RUSTERON_DPDK_*`, `RUSTERON_MEDIA_DRIVER_TRANSPORT`: the
//! library's [`ConfiguredDriver`]), with its directory at `AERON_DIR` (or
//! `RUSTERON_MEDIA_DRIVER_DIR`). Its threading mode is `INVOKER` and its
//! conductor CPU (where Aeron pins the thread that starts it) the client's own
//! CPU argument, whatever `AERON_THREADING_MODE` and
//! `AERON_CONDUCTOR_CPU_AFFINITY` say; it never idles, so its shared idle
//! strategy is `noop`. One thread, the main one, then runs the client's work,
//! its Aeron conductor (the agent invoker) and the driver, every pass of its
//! loop ([`Duty`]). At exit the driver is left unclosed, so its `CnC` keeps
//! every counter for the lab's `counters` snapshots
//! ([`EnvMediaDriver::abandon`]); the directory is never deleted on shutdown.
//! Each client prints, on stderr, a `harness driver:` line naming the driver
//! and, embedded, the threading mode in use.

use ergon_md_dpdk::driver::{ConfiguredDriver, EnvMediaDriver, threading_mode_name};
use rusteron_client::{Aeron, AeronCError, AeronContext, AeronErrorType, AeronExclusivePublication, AeronSubscription};
use rusteron_client::{AeronImage, Handlers, IntoCString};
use rusteron_media_driver::bindings::aeron_threading_mode_t;
use rusteron_media_driver::{AeronDriverContext, AeronIdleStrategyKind};
use std::ffi::{CStr, CString};
use std::hint::spin_loop;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

/// Set by SIGTERM once [`stop_on_sigterm`] has installed its handler: the lab
/// has ended the client's run, so its loops stop, a back-pressured retry's
/// too ([`Retry`]). Never set in a client that leaves SIGTERM's default (it
/// dies of it).
pub static STOP: AtomicBool = AtomicBool::new(false);

extern "C" fn on_sigterm(_signal: libc::c_int) {
    STOP.store(true, Ordering::Relaxed);
}

/// Takes SIGTERM as [`STOP`] from now on.
#[allow(dead_code)] // rtt xpong's, which writes what it recorded as it stops
pub fn stop_on_sigterm() {
    // SAFETY: the handler only stores to an atomic, which is async-signal-safe.
    unsafe { libc::signal(libc::SIGTERM, on_sigterm as *const () as libc::sighandler_t) };
}

/// How long a back-pressured retry goes on with its publication not connected
/// (no subscriber's status for the driver's publication connection timeout,
/// 5 s by default) before it is given up: nobody is left to read what it
/// would send.
#[allow(dead_code)] // rtt's and load's
pub const NOT_CONNECTED_LIMIT: Duration = Duration::from_secs(1);

/// Why a back-pressured retry was given up ([`Retry`]).
#[allow(dead_code)] // rtt's and load's
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Halt {
    /// SIGTERM ([`STOP`]): the lab ended the run.
    Stopped,
    /// The image the retry answers closed: the other side is gone.
    ImageClosed,
    /// The publication not connected for longer than [`NOT_CONNECTED_LIMIT`].
    NotConnected,
}

impl std::fmt::Display for Halt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Stopped => "SIGTERM",
            Self::ImageClosed => "the other side's image closed",
            Self::NotConnected => "the publication not connected for over 1 s",
        })
    }
}

/// A back-pressured retry's watch on whether it can still make progress. A
/// retry waits on a slow peer for as long as that takes, but not past the
/// run's end ([`STOP`]), the closing of the image it answers (the other
/// side gone), or its publication staying not connected for longer than
/// [`NOT_CONNECTED_LIMIT`]. An echo retries inside its image's poll, which
/// holds the image open (the driver lets an image go only once it is read
/// dry), so for an echo the image closes only when the client loses its
/// driver; a ping retries between polls, its image read dry, so it sees
/// the pong's go. Asked only on the retry path: an offer or claim that goes
/// through costs nothing more, and a check reads three flags (and the clock
/// only while not connected); it never blocks or allocates.
#[allow(dead_code)] // rtt's and load's
#[derive(Default)]
pub struct Retry {
    not_connected_since: Option<Instant>,
}

#[allow(dead_code)] // rtt's and load's
impl Retry {
    /// `Ok` to retry again; else why not.
    #[inline]
    pub fn check(&mut self, publication: &AeronExclusivePublication, image: &AeronImage) -> Result<(), Halt> {
        if STOP.load(Ordering::Relaxed) {
            return Err(Halt::Stopped);
        }
        if image.is_closed() {
            return Err(Halt::ImageClosed);
        }
        if publication.is_connected() {
            self.not_connected_since = None;
            return Ok(());
        }
        let since = *self.not_connected_since.get_or_insert_with(Instant::now);
        if since.elapsed() > NOT_CONNECTED_LIMIT {
            Err(Halt::NotConnected)
        } else {
            Ok(())
        }
    }
}

/// Echoes `message`, read from `image`, on `publication`, retrying a
/// back-pressured claim (an embedded driver, run by `duty` meanwhile, sends
/// what holds it back) until it goes through or [`Retry`] gives it up: why.
#[allow(dead_code)] // rtt's and load's
#[inline]
pub fn echo(
    duty: &impl Duty,
    publication: &AeronExclusivePublication,
    image: &AeronImage,
    message: &[u8],
) -> Result<(), Halt> {
    let mut retry = Retry::default();
    loop {
        match publication.try_claim_owned(message.len()) {
            Ok(mut claim) => {
                claim.data().copy_from_slice(message);
                claim.commit().unwrap();
                return Ok(());
            }
            Err(e) if e.is_retryable() => {
                retry.check(publication, image)?;
                if duty.run() == 0 {
                    spin_loop();
                }
            }
            Err(e) => panic!("claim: {e:?}"),
        }
    }
}

/// Pins the calling thread to `cpu`; `None` leaves it where it is.
#[cfg(target_os = "linux")]
pub fn pin(cpu: Option<usize>) {
    let Some(cpu) = cpu else { return };
    // SAFETY: a zeroed cpu_set_t is the empty set, and the call only reads it.
    unsafe {
        let mut set: libc::cpu_set_t = std::mem::zeroed();
        libc::CPU_SET(cpu, &mut set);
        let rc = libc::sched_setaffinity(0, std::mem::size_of::<libc::cpu_set_t>(), &set);
        assert_eq!(rc, 0, "could not pin to cpu {cpu}");
    }
}

/// Thread pinning is Linux only; elsewhere the clients run unpinned (smoke tests).
#[cfg(not(target_os = "linux"))]
pub fn pin(_cpu: Option<usize>) {}

/// `-` or a CPU number.
pub fn cpu_arg(arg: Option<String>) -> Option<usize> {
    arg.filter(|a| a != "-").map(|a| a.parse().expect("cpu number or -"))
}

/// The external driver's dir, from `AERON_DIR`.
pub fn aeron_dir() -> CString {
    CString::new(std::env::var("AERON_DIR").expect("AERON_DIR: the external driver's dir")).unwrap()
}

/// A subscription's channel: `channel` with `SUB_CHANNEL_PARAMS` appended when
/// set (e.g. `reliable=false|group=false|tether=false`: no NAKs, gaps filled),
/// for experiments on the receiving side only.
pub fn sub_channel(channel: &std::ffi::CStr) -> CString {
    with_params(channel, "SUB_CHANNEL_PARAMS")
}

/// A publication's channel: `channel` with `PUB_CHANNEL_PARAMS` appended when
/// set (`key=value|key=value`, e.g. `term-length=65536`), as
/// `SUB_CHANNEL_PARAMS` is to a subscription's, for experiments on the sending
/// side. [`Lab::add_publication`] applies it to every publication.
pub fn pub_channel(channel: &std::ffi::CStr) -> CString {
    with_params(channel, "PUB_CHANNEL_PARAMS")
}

/// `channel` with the parameters in `var` appended, when it is set and not empty.
fn with_params(channel: &std::ffi::CStr, var: &str) -> CString {
    match std::env::var(var).ok().filter(|p| !p.is_empty()) {
        Some(params) => CString::new(format!("{}|{params}", channel.to_string_lossy())).unwrap(),
        None => channel.to_owned(),
    }
}

/// What a measuring loop runs, every pass, besides the client's own work.
pub trait Duty {
    /// Runs it once: the work it found, 0 when none.
    fn run(&self) -> i32;
}

/// An external driver and the client's conductor run on threads of their own:
/// nothing, compiled away.
pub struct External;

impl Duty for External {
    #[inline(always)]
    fn run(&self) -> i32 {
        0
    }
}

/// `HARNESS_DRIVER=invoker`: the client's conductor and the embedded driver's
/// agents, one duty cycle each.
pub struct Invoker<'a> {
    aeron: &'a Aeron,
    driver: &'a EnvMediaDriver,
}

impl Duty for Invoker<'_> {
    #[inline(always)]
    fn run(&self) -> i32 {
        let client = self
            .aeron
            .main_do_work()
            .unwrap_or_else(|e| invoker_failed("the client's conductor", &e));
        let driver = self
            .driver
            .do_work()
            .unwrap_or_else(|e| invoker_failed("the embedded driver", &e));
        client + driver
    }
}

#[cold]
#[inline(never)]
fn invoker_failed(what: &str, error: &dyn std::fmt::Debug) -> ! {
    panic!("{what} failed: {error:?}")
}

/// Whether `HARNESS_DRIVER=invoker`: the client embeds its driver, so its
/// measuring thread must be the one that starts it ([`Lab::connect`]).
#[allow(dead_code)] // rtt's (idle) and tput's (its publisher's thread); load's never asks
pub fn embedded() -> bool {
    std::env::var("HARNESS_DRIVER").is_ok_and(|d| d == "invoker")
}

/// The process's Aeron client and, under `HARNESS_DRIVER=invoker`, the media
/// driver it embeds (see the module's documentation).
pub struct Lab {
    // Dropped first: the client closes before its driver is left.
    aeron: Aeron,
    embedded: Option<Embedded>,
}

/// The embedded driver, left unclosed when the client is done.
struct Embedded(Option<EnvMediaDriver>);

impl Drop for Embedded {
    fn drop(&mut self) {
        if let Some(driver) = self.0.take()
            && let Err(e) = driver.abandon()
        {
            eprintln!("harness: the embedded driver: {e}");
        }
    }
}

impl Lab {
    /// The client of the driver `HARNESS_DRIVER` picks. `cpu` is the client's
    /// CPU argument: under `invoker` the embedded driver's conductor CPU, so
    /// Aeron pins this thread there as the driver starts. Panics, naming the
    /// variable, for any other `HARNESS_DRIVER` than `external` or `invoker`.
    pub fn connect(cpu: Option<usize>) -> Self {
        match std::env::var("HARNESS_DRIVER").as_deref() {
            Err(_) | Ok("" | "external") => {
                let dir = aeron_dir();
                eprintln!("harness driver: external dir={}", dir.to_string_lossy());
                Self {
                    aeron: client(&dir, false),
                    embedded: None,
                }
            }
            Ok("invoker") => {
                let driver = embed(cpu).unwrap_or_else(|e| panic!("HARNESS_DRIVER=invoker: {e}"));
                let dir = driver.dir().to_string();
                let aeron = client(&dir.clone().into_c_string(), true);
                // what the driver runs with, as it was started
                let conductor = driver.conductor_cpu_affinity();
                eprintln!(
                    "harness driver: embedded threading={} transport={} cpu={} dir={dir}",
                    threading_mode_name(driver.threading_mode()),
                    driver.backend(),
                    if conductor < 0 {
                        "-".to_string()
                    } else {
                        conductor.to_string()
                    },
                );
                Self {
                    aeron,
                    embedded: Some(Embedded(Some(driver))),
                }
            }
            Ok(other) => panic!("HARNESS_DRIVER={other:?}: external (or unset) or invoker"),
        }
    }

    /// The embedded driver's duty, under `HARNESS_DRIVER=invoker`.
    pub fn invoker(&self) -> Option<Invoker<'_>> {
        let driver = self.embedded.as_ref()?.0.as_ref()?;
        Some(Invoker {
            aeron: &self.aeron,
            driver,
        })
    }

    /// Lets time pass outside a measured loop: under `invoker` running the
    /// duty (resting a millisecond when it finds nothing), else sleeping.
    pub fn idle_for(&self, period: Duration) {
        match self.invoker() {
            None => std::thread::sleep(period),
            Some(duty) => {
                let until = Instant::now() + period;
                while Instant::now() < until {
                    if duty.run() == 0 {
                        std::thread::sleep(Duration::from_millis(1).min(period));
                    }
                }
            }
        }
    }

    /// Waits on `ready` until it holds, failing after `secs`.
    pub fn wait_until(&self, what: &str, secs: u64, mut ready: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(secs);
        while !ready() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            self.idle_for(Duration::from_millis(1));
        }
    }

    /// An exclusive publication on `channel` with `PUB_CHANNEL_PARAMS`.
    pub fn add_publication(&self, channel: &CStr, stream: i32) -> AeronExclusivePublication {
        let channel = pub_channel(channel);
        self.retry_closing("publication", || {
            let pending = self.aeron.async_add_exclusive_publication(&channel, stream)?;
            match self.invoker() {
                None => pending.poll_blocking(Duration::from_secs(5)),
                Some(duty) => resolve(&duty, || pending.poll()),
            }
        })
    }

    /// A subscription on `channel` (`sub_channel` is the caller's).
    pub fn add_subscription(&self, channel: &CStr, stream: i32) -> AeronSubscription {
        self.retry_closing("subscription", || {
            let pending = self
                .aeron
                .async_add_subscription(channel, stream, Handlers::NONE, Handlers::NONE)?;
            match self.invoker() {
                None => pending.poll_blocking(Duration::from_secs(5)),
                Some(duty) => resolve(&duty, || pending.poll()),
            }
        })
    }

    /// Retries `add` (adding a publication or subscription and waiting for it)
    /// while the driver answers that the channel's endpoint is still closing,
    /// a previous run's on the same channel ("please retry"), for up to 10 s,
    /// an embedded driver run meanwhile. Any other error, or one past that,
    /// fails the client: a run that silently lost its publication would
    /// report nothing as a measurement.
    fn retry_closing<T>(&self, what: &str, mut add: impl FnMut() -> Result<T, AeronCError>) -> T {
        let deadline = Instant::now() + Duration::from_secs(10);
        loop {
            match add() {
                Ok(added) => return added,
                Err(e) if format!("{e:?}").contains("please retry") && Instant::now() < deadline => {
                    self.idle_for(Duration::from_millis(20));
                }
                Err(e) => panic!("adding the {what}: {e:?}"),
            }
        }
    }
}

/// An add the client's conductor completes as `duty` runs it: within 5 s.
fn resolve<T>(duty: &Invoker<'_>, mut poll: impl FnMut() -> Result<Option<T>, AeronCError>) -> Result<T, AeronCError> {
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(added) = poll()? {
            return Ok(added);
        }
        if Instant::now() > deadline {
            return Err(AeronErrorType::TimedOut.into());
        }
        if duty.run() == 0 {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
}

/// The driver `HARNESS_DRIVER=invoker` embeds: the bundled `media_driver`'s
/// configuration from the environment, in INVOKER mode on `cpu`, never
/// idling and never deleting its directory, started on this thread.
fn embed(cpu: Option<usize>) -> Result<EnvMediaDriver, Box<dyn std::error::Error>> {
    let context = AeronDriverContext::new()?;
    context.set_threading_mode(aeron_threading_mode_t::AERON_THREADING_MODE_INVOKER)?;
    context.set_conductor_cpu_affinity(cpu.map_or(Ok(-1), i32::try_from)?)?;
    context.set_shared_idle_strategy_kind(AeronIdleStrategyKind::NoOp)?;
    context.set_dir_delete_on_shutdown(false)?;
    Ok(ConfiguredDriver::from_env(context)?.start()?)
}

/// A client of the driver at `dir`, its conductor on a thread of its own or,
/// with `invoker`, run by the caller ([`Duty`]).
fn client(dir: &CStr, invoker: bool) -> Aeron {
    let context = AeronContext::new().unwrap();
    context.set_dir(dir).unwrap();
    if invoker {
        context.set_use_conductor_agent_invoker(true).unwrap();
    }
    let aeron = Aeron::new(&context).unwrap();
    aeron.start().unwrap();
    aeron
}

/// `LABEL`, which names the run in the result line.
pub fn label() -> String {
    std::env::var("LABEL").unwrap_or_else(|_| "-".into())
}

/// This process's locked memory, `VmLck` from `/proc/self/status`, in kB.
/// `mlockall` reports success in the process that called it and is not
/// inherited across `execve` (mlock(2)); only this number says whether the
/// running process holds the lock.
fn vmlck_kb() -> u64 {
    let Ok(status) = std::fs::read_to_string("/proc/self/status") else {
        return 0;
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmLck:") {
            return rest.split_whitespace().next().and_then(|v| v.parse().ok()).unwrap_or(0);
        }
    }
    0
}

/// Locks this process's memory when `RUSTERON_MEDIA_DRIVER_MLOCKALL=1`
/// ([`ergon_md_dpdk::lock_memory_from_env`]), then verifies the running
/// process actually holds it: `mlockall` before an `exec` is gone by the time
/// the target runs, so a wrapper cannot lock for it. Prints one
/// `harness memory:` line. Panics where a lock was asked for and is not there
/// (fail closed: the benchmark records the missing grant, it never runs as if
/// it were).
#[allow(dead_code)] // every client's main
pub fn lock_memory() {
    let asked = std::env::var("RUSTERON_MEDIA_DRIVER_MLOCKALL").is_ok_and(|v| v == "1");
    let locked = ergon_md_dpdk::lock_memory_from_env().unwrap_or_else(|e| panic!("harness memory: {e}"));
    let vmlck = vmlck_kb();
    if asked && (!locked || vmlck == 0) {
        panic!(
            "harness memory: RUSTERON_MEDIA_DRIVER_MLOCKALL=1 but the running process holds \
             nothing (VmLck: {vmlck} kB); mlockall does not survive execve, so it has to run here"
        );
    }
    eprintln!(
        "harness memory: {}",
        if locked {
            format!("locked (VmLck: {vmlck} kB)")
        } else {
            "not locked (RUSTERON_MEDIA_DRIVER_MLOCKALL unset or 0)".to_owned()
        }
    );
}

/// The clients' clock: ergon-clock's calibrated hardware counter, in
/// nanoseconds (`rdtsc` on x86-64, `CNTVCT_EL0` on aarch64, scaled by its
/// 32.32 factor; clock/src), so the clients, the transport and the stack
/// stamp on one implementation.
///
/// - [`stamp`](Self::stamp), every latency stamp (a round trip's send and
///   echo, a load message's arrival, its send lag): ergon-clock's ordered
///   read, taken after every earlier instruction (`rdtscp`; on Arm
///   `CNTVCTSS_EL0` where the CPU has `FEAT_ECV`, else `isb` then
///   `CNTVCT_EL0`), as the harness's own clock read every stamp before.
/// - [`now`](Self::now), pacing, deadlines, loop gaps and throughput
///   windows: the unordered read.
///
/// Calibrated at [`clock_config`]: the counter wherever ergon-clock's own
/// measurement and verification against `CLOCK_MONOTONIC_RAW` accept it
/// (`Trust::CrossCheck`: also under `kvm-clock` and Azure's TSC page, where
/// the kernel keeps time with a hypervisor's view of the same counter, as
/// the harness's clock did on every lab host), else ergon-clock's
/// `CLOCK_MONOTONIC` fallback, said so. `BENCH_CLOCK=monotonic` forces the
/// fallback, to compare the two. Its report goes to stderr as one
/// `harness clock:` line. Output units are nanoseconds as ever, so results
/// compare with runs of the harness's own clock.
#[derive(Clone, Copy, Debug)]
pub struct Clock(ergon_clock::advanced::Clock);

/// The harness's calibration: ergon-clock's defaults at cross-check trust
/// (see [`Clock`]), nanoseconds since boot (the monotonic domain),
/// `Policy::Monotonic` under `BENCH_CLOCK=monotonic`, and no probe of how
/// `CLOCK_REALTIME` is kept (the lab records chrony's view itself; a
/// client's stamps are on one host's counter).
pub fn clock_config() -> ergon_clock::Config {
    let monotonic = std::env::var("BENCH_CLOCK").is_ok_and(|c| c == "monotonic");
    ergon_clock::Config {
        policy: if monotonic {
            ergon_clock::advanced::Policy::Monotonic
        } else {
            ergon_clock::advanced::Policy::Auto
        },
        trust: ergon_clock::advanced::Trust::CrossCheck,
        domain: ergon_clock::Domain::Monotonic,
        time_sync: ergon_clock::advanced::TimeSyncPolicy::Off,
        ..ergon_clock::Config::default()
    }
}

impl Clock {
    /// Calibrates (tens of milliseconds); call before the measured loop.
    /// Prints ergon-clock's report on stderr (`harness clock: ...`). Panics
    /// only where a kernel clock cannot be read at all.
    pub fn calibrate() -> Self {
        let calibrated =
            ergon_clock::advanced::calibrate(&clock_config()).unwrap_or_else(|e| panic!("ergon-clock: {e}"));
        eprintln!("harness clock: {}", calibrated.report);
        Self(calibrated.clock)
    }

    /// Whether it reads the hardware counter (`false`: ergon-clock's
    /// monotonic fallback).
    pub fn is_counter(&self) -> bool {
        self.0.is_counter()
    }

    /// A latency stamp, ns: the ordered read.
    #[allow(dead_code)] // rtt's and load's; tput times windows alone
    #[inline(always)]
    pub fn stamp(&self) -> u64 {
        self.0.now_ns_ordered()
    }

    /// The time now, ns, unordered: pacing, deadlines, loop gaps.
    #[inline(always)]
    pub fn now(&self) -> u64 {
        self.0.now_ns()
    }

    /// ergon-clock's clock itself (clockcheck's comparisons).
    #[allow(dead_code)] // clockcheck's
    pub fn inner(&self) -> ergon_clock::advanced::Clock {
        self.0
    }
}

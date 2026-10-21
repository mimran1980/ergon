//! The host and NIC checks a driver runs, built on [`ergon_tune`].
//!
//! Each finding is one stderr line (`ergon-io warning: ...` or
//! `ergon-io info: ...`), written by ergon-io's warning thread, never by the
//! polling thread.
//!
//! | Check | When | On which thread |
//! |---|---|---|
//! | **I-host**: T1 (the loop CPU isolated and `nohz_full`), T7 (no `SCHED_FIFO`/`SCHED_RR`), T8 (no busy SMT sibling), T9 (no CPU quota on the process's cgroup); T3 (transparent hugepages) as information | `Driver::new` | the thread calling it |
//! | **I3**: `SO_RCVBUF` granted as asked | `Driver::new`, when `Config::rcvbuf` is set (one probe socket) | the thread calling it |
//! | **I1**: `TCP_NODELAY` set, `TCP_CORK` reported | `Driver::add` (the driver's own connects set `TCP_NODELAY` when they create the socket) | the polling thread: three socket options |
//! | **I2**: the route's quick ACK ([`crate::route`]) | each connection | `require`: where the connection is set up; `warn`: the warning thread |
//! | **I-dev**: T2 (NIC interrupts off the loop CPU and `sqpoll_cpu`), N1 (ENA adaptive RX moderation), N2 (deferred interrupts, interrupt-driven backends only), N3 (an Azure VF) on the route's egress device | after the first route lookup to the device, and again after each reset of it (`carrier_changes` moved, then settled) | the warning thread |
//! | **I4**: a NIC-polling backend's socket has a NAPI id | after the handshake, non-local routes | the warning thread (the id is read where the connection is set up) |
//!
//! - The loop CPU: [`Config::loop_cpu`], else the CPU the thread calling
//!   `Driver::new` is pinned to (when exactly one). With neither, T1, T8 and
//!   T2 are skipped, with one information line.
//! - Where the helper threads go ([`crate::placement`]):
//!   [`HostReport::helpers`], one information line at start
//!   ([`HostReport::helper_line`]), and one warning when the process may use
//!   the polling CPU only (a pod with one CPU).
//! - `quickack_check=off` skips I2, I-dev and I4: they start from the route
//!   lookup.
//! - Each line prints once per process (I4 once per device). I-dev prints
//!   again after each reset of the device (`carrier_changes` moved, then
//!   held for [`SETTLE_READS`] one-second reads): an ENA reset re-places
//!   queue interrupts and turns adaptive moderation back on.
//! - A check that cannot read what it needs reports `unknown` as
//!   information, never a warning (a pod's network namespace has no host NIC
//!   files).
//! - The functions are pure (they read the [`Host`] they are given), so they
//!   run against fixture trees on any OS.

use std::fmt::Write as _;

use ergon_tune::{
    CheckId, Component, Ethtool, EthtoolCache, Finding, Host, HostCheckError, HostCheckPolicy,
    HotSet, Needs, Netlink, Nic, Role, SILENCE, Severity, Status, Thread, carrier_changes,
    host_check_line, k8s_checks, n1_adaptive_rx, n2_deferred_irqs, n3_azure_vf, t1_isolation,
    t2_nic_irqs, t3_thp, t7_sched_policy, t8_smt_siblings, t9_cpu_quota, t10_smt_on,
    thread_affinity,
};

use crate::backend::{Backend, Config};
use crate::endpoint::ConnId;
use crate::placement::{self, CpuError, Placement, Why};
use crate::route::{Flow, Route};

/// How the loop's CPU was found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoopCpu {
    /// [`Config::loop_cpu`].
    Configured(u32),
    /// The CPU the thread calling `Driver::new` is pinned to.
    Pinned(u32),
    /// That thread may run on these CPUs (a kernel CPU list) and no
    /// `loop_cpu` is configured: the CPU checks are skipped.
    Unpinned(String),
    /// That thread's affinity could not be read (the reason).
    Unknown(String),
}

impl LoopCpu {
    /// The CPU, when known.
    #[must_use]
    pub const fn cpu(&self) -> Option<u32> {
        match self {
            Self::Configured(c) | Self::Pinned(c) => Some(*c),
            Self::Unpinned(_) | Self::Unknown(_) => None,
        }
    }
}

/// What `Driver::new` found about the host ([`host_report`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HostReport {
    /// The loop's CPU.
    pub loop_cpu: LoopCpu,
    /// The CPUs the checks treat as hot: the loop's, and `sqpoll_cpu`'s for
    /// `uring-sqpoll` (empty when the loop's CPU is not known).
    pub hot: HotSet,
    /// T1 and T8 (when the loop's CPU is known), T7, T9 and T3, each with
    /// what it found, including what is fine.
    pub findings: Vec<Finding>,
    /// Where ergon-io's helper threads run ([`crate::placement`]); the error
    /// when `Config::helper_cpus` or `ERGON_HELPER_CPUS` names CPUs they
    /// cannot use (`Driver::new` returns it).
    pub helpers: Result<Placement, CpuError>,
}

impl HostReport {
    /// The lines to print under [`HostCheckPolicy::On`]: the note when the
    /// loop's CPU is not known, then a line per finding that is not fine
    /// ([`line()`]), each warning ending with [`SILENCE`].
    #[must_use]
    pub fn lines(&self) -> Vec<String> {
        self.lines_under(HostCheckPolicy::On)
    }

    /// [`Self::lines`] under `policy` (`ERGON_HOST_CHECKS`):
    /// [`HostCheckPolicy::Off`] drops the warning lines and keeps the rest,
    /// the findings themselves are untouched.
    #[must_use]
    pub fn lines_under(&self, policy: HostCheckPolicy) -> Vec<String> {
        let mut out = Vec::new();
        match &self.loop_cpu {
            LoopCpu::Unpinned(cpus) => out.push(format!(
                "ergon-io info: the thread creating the driver is not pinned to one CPU (it may \
                 run on CPUs {cpus}) and loop_cpu is not set, so the polling loop's CPU is not \
                 known: T1 (isolation), T8 (SMT siblings) and T2 (NIC interrupts) are skipped, \
                 and the helper threads may include the CPU the loop ends up on. Pin the polling \
                 thread to an isolated CPU before Driver::new, or set loop_cpu."
            )),
            LoopCpu::Unknown(why) => out.push(format!(
                "ergon-io info: cannot read the affinity of the thread creating the driver \
                 ({why}) and loop_cpu is not set, so the polling loop's CPU is not known: T1 \
                 (isolation), T8 (SMT siblings) and T2 (NIC interrupts) are skipped, and the \
                 helper threads may include the CPU the loop ends up on. Set loop_cpu."
            )),
            LoopCpu::Configured(_) | LoopCpu::Pinned(_) => {}
        }
        out.extend(policy.lines("ergon-io", &self.findings));
        if let Some(cpu) = self.only_cpu()
            && policy != HostCheckPolicy::Off
        {
            out.push(format!(
                "ergon-io warning: this process may use the polling CPU only (CPU {cpu}: a pod \
                 with one CPU), so ergon-io's helper threads (connect_background's dial helper, the \
                 warning thread, uring-sqpoll's SQPOLL thread) have no other core. \
                 connect_background refuses (or, with inline_connect_fallback, connects as \
                 Driver::connect does) and uring-sqpoll refuses: Driver::connect does the \
                 handshake on this core, about 0.7 ms of CPU per TLS handshake. The warning \
                 thread shares it, parked until there is work. For production give the pod 2 \
                 CPUs: Guaranteed QoS, the kubelet's static CPU manager policy, whole-number CPU \
                 requests (on x86 with full-pcpus-only, one core's two hyperthreads). {SILENCE}"
            ));
        }
        out
    }

    /// Under [`HostCheckPolicy::Require`], the error `Driver::new` must fail
    /// with when anything here warned: the findings, and the one-CPU helper
    /// warning (which is not a finding).
    #[must_use]
    pub fn refusal(&self, policy: HostCheckPolicy) -> Option<HostCheckError> {
        let mut r = policy.refusal(&self.findings);
        if policy == HostCheckPolicy::Require && self.only_cpu().is_some() {
            if let Some(HostCheckError::Warnings { count, ids }) = &mut r {
                *count += 1;
                ids.push_str(", helpers");
            } else {
                r = Some(HostCheckError::Warnings {
                    count: 1,
                    ids: "helpers".to_owned(),
                });
            }
        }
        r
    }

    /// The polling CPU, when this process may use that CPU alone (a pod with
    /// one CPU): the warning in [`Self::lines_under`] and the start refusal
    /// under [`HostCheckPolicy::Require`] both cover it.
    fn only_cpu(&self) -> Option<u32> {
        if let Ok(p) = &self.helpers
            && let (Why::NoSpareCpu, Some(cpu)) = (p.why, p.polling)
            && p.allowed.iter().eq([cpu])
        {
            Some(cpu)
        } else {
            None
        }
    }

    /// The start-up line saying where the helper threads go and why; `None`
    /// when they have no CPU (the warning in [`HostReport::lines`] says so)
    /// or the configured CPUs are refused.
    #[must_use]
    pub fn helper_line(&self) -> Option<String> {
        let p = self.helpers.as_ref().ok().filter(|p| !p.cpus.is_empty())?;
        let cpus =
            |s: &ergon_tune::CpuSet| format!("CPU{} {s}", if s.len() == 1 { "" } else { "s" });
        // `uring-sqpoll`'s SQPOLL thread: there with the helpers when the hot
        // set has it there (the automatic choices), else on a CPU of the
        // process's own outside an explicit helper list (the shared-work
        // CPUs: never a spinner there).
        let sqpoll = self
            .hot
            .iter()
            .find(|h| h.role == Role::SqPoll)
            .map_or_else(String::new, |h| {
                if p.cpus.contains(h.cpu) {
                    " Uring-sqpoll's SQPOLL thread goes there too.".to_owned()
                } else {
                    format!(
                        " Uring-sqpoll's SQPOLL thread goes on CPU {} of the process's own, off \
                         those (never a spinner on shared CPUs).",
                        h.cpu
                    )
                }
            });
        Some(format!(
            "ergon-io info: connect_background's dial helper and the warning thread go on {}: \
             {}.{} This process may use {} ({}){}.",
            cpus(&p.cpus),
            p.why,
            sqpoll,
            cpus(&p.allowed),
            p.source,
            p.polling.map_or_else(
                || {
                    "; the polling CPU is unknown (no loop_cpu, and the creating thread is not \
                     pinned to one CPU): these may include it. Set loop_cpu or pin the loop \
                     thread before Driver::new"
                        .to_owned()
                },
                |c| format!("; the polling CPU is {c}")
            )
        ))
    }
}

/// The I-host checks for a driver configured as `cfg`.
///
/// For a driver created by the calling thread: T1 and T8 on the loop CPU
/// (and `sqpoll_cpu` for `uring-sqpoll`) when it is known, T7 on the
/// calling thread, T9 on the process's cgroup, and T3 as information.
#[must_use]
pub fn host_report(host: &Host, cfg: &Config) -> HostReport {
    let loop_cpu = cfg.loop_cpu.map_or_else(
        || match thread_affinity(host, Thread::Current) {
            Ok(set) => match (set.len(), set.iter().next()) {
                (1, Some(c)) => LoopCpu::Pinned(c),
                _ => LoopCpu::Unpinned(set.to_string()),
            },
            Err(e) => LoopCpu::Unknown(e.to_string()),
        },
        LoopCpu::Configured,
    );
    // The running host's environment goes with it; a fixture tree has none.
    let env = host
        .is_live()
        .then(|| std::env::var(placement::HELPER_CPUS_ENV).ok())
        .flatten();
    let helpers = placement::of(host, &cfg.helper_cpus, env.as_deref(), loop_cpu.cpu());
    let mut cpus = HotSet::new();
    let mut findings = Vec::new();
    if let Some(cpu) = loop_cpu.cpu() {
        cpus.push(cpu, Role::Loop);
        // `SQPOLL` on `sqpoll_cpu`, else [`placement::sqpoll_cpu`]'s default
        // (off an explicit helper list: never a spinner on the shared-work
        // CPUs, else the first helper CPU). `None` where that refuses: the
        // same error `Driver::new` returns.
        let sqpoll = cfg.sqpoll_cpu.or_else(|| {
            helpers
                .as_ref()
                .ok()
                .and_then(|p| placement::sqpoll_cpu(p, None).ok())
        });
        if let (Backend::UringSqpoll, Some(sq)) = (cfg.backend, sqpoll) {
            cpus.push(sq, Role::SqPoll);
        }
        findings.extend(t1_isolation(host, &cpus));
        findings.extend(t8_smt_siblings(host, &cpus));
    }
    findings.push(t7_sched_policy(host, Thread::Current, Role::Loop));
    findings.push(t9_cpu_quota(host));
    findings.extend(t3_thp(host, Component::Io));
    findings.push(t10_smt_on(host));
    // Inside a pod only: what its settings do to this layout (K1-K7, with
    // T7 and T9 again). Busy polling needs `CAP_NET_ADMIN`.
    let mut needs = Needs::new(Thread::Current, Role::Loop);
    if cfg.backend.polls_nic() || cfg.prefer_busy_poll || cfg.busy_poll_us > 0 {
        needs = needs.busy_polls();
    }
    findings.extend(k8s_checks(host, &cpus, &needs));
    HostReport {
        loop_cpu,
        hot: cpus,
        findings,
        helpers,
    }
}

/// The line printed for `f` at [`Severity::Warn`] or [`Severity::Info`], and
/// `None` for what is fine. ([`host_check_line`].)
///
/// The warn line reads `ergon-io warning: <finding> (ERGON_HOST_CHECKS=off
/// silences host-check warnings)`; the info line reads `ergon-io info:
/// <finding>`, and so does what could not be read.
#[must_use]
pub fn line(f: &Finding) -> Option<String> {
    host_check_line("ergon-io", f)
}

/// What the egress-device checks need from the driver.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DevContext {
    /// The driver's hot CPUs ([`HostReport::hot`]): T2 keeps the device's
    /// interrupts off them.
    pub hot: HotSet,
    /// Whether the backend waits for interrupts (does not poll the NIC):
    /// only then are deferred interrupts (N2) a cost.
    pub interrupt_driven: bool,
}

impl DevContext {
    /// The context of a driver configured as `cfg` with hot CPUs `hot`.
    #[must_use]
    pub const fn new(cfg: &Config, hot: HotSet) -> Self {
        Self {
            hot,
            interrupt_driven: !cfg.backend.polls_nic(),
        }
    }
}

/// How many more reads (one a second on the warning thread) a device's
/// `carrier_changes` must hold a new value before [`DevChecks::recheck`]
/// checks it again: 2–3 s after the counter last moved.
pub const SETTLE_READS: u8 = 2;

/// A device checked, in a context, and its `carrier_changes` then.
#[derive(Debug)]
struct Watch {
    dev: String,
    ctx: DevContext,
    carrier: Option<u64>,
    /// A new value of the counter, and how many reads after the first have
    /// seen it unchanged: the device is checked again once that reaches
    /// [`SETTLE_READS`].
    moved: Option<(u64, u8)>,
}

/// The I-dev checks with what they remember.
///
/// They keep the ethtool answers (until the interface resets) and each
/// device checked, so it is reported once and again after its carrier
/// changes. ergon-io's warning thread owns one.
///
/// A device is checked again only by [`DevChecks::recheck`], and only once
/// its `carrier_changes` has held a new value for [`SETTLE_READS`] further
/// reads: the counter moves at carrier down, before the driver has come
/// back up, and a link-event watcher that re-pins the interrupts (the fix
/// T2 recommends) hears of the change only after it (the kernel's link
/// events for a carrier going down are delayed up to a second), so a check
/// at the first move would warn about a host that fixes itself.
#[derive(Debug)]
pub struct DevChecks<E: Ethtool = Netlink> {
    cache: EthtoolCache<E>,
    watches: Vec<Watch>,
}

impl DevChecks<Netlink> {
    /// Nothing checked yet; ethtool over netlink.
    #[must_use]
    pub const fn new() -> Self {
        Self::with_cache(EthtoolCache::new())
    }
}

impl Default for DevChecks<Netlink> {
    fn default() -> Self {
        Self::new()
    }
}

impl<E: Ethtool> DevChecks<E> {
    /// Nothing checked yet; ethtool answers from `cache` (a stand-in in
    /// tests).
    #[must_use]
    pub const fn with_cache(cache: EthtoolCache<E>) -> Self {
        Self {
            cache,
            watches: Vec::new(),
        }
    }

    /// T2, N1, N2 (interrupt-driven backends) and N3 for egress device
    /// `dev` in `ctx`, the first time this device is seen in this context;
    /// nothing after that, even if its carrier changed since (re-checks are
    /// [`DevChecks::recheck`]'s, whose findings are reported again). T2 is
    /// skipped when `ctx` has no hot CPU. A device with no device here (a
    /// pod's veth) gives one unknown finding and is not watched: there is
    /// nothing to check again.
    pub fn check(&mut self, host: &Host, dev: &str, ctx: &DevContext) -> Vec<Finding> {
        if self.watches.iter().any(|w| w.dev == dev && w.ctx == *ctx) {
            return Vec::new();
        }
        if let Err(why) = visible(host, dev) {
            return vec![invisible(dev, why)];
        }
        self.watches.push(Watch {
            dev: dev.to_owned(),
            ctx: ctx.clone(),
            carrier: carrier_changes(host, dev).ok(),
            moved: None,
        });
        self.run(host, dev, ctx)
    }

    /// The checks again for every device whose `carrier_changes` moved since
    /// it was last checked and has then held still for [`SETTLE_READS`]
    /// further calls (an ENA reset re-places its queue interrupts and turns
    /// adaptive moderation back on). One sysfs read per device; the warning
    /// thread calls it once a second while it watches a device.
    pub fn recheck(&mut self, host: &Host) -> Vec<Finding> {
        let mut due = Vec::new();
        for w in &mut self.watches {
            let Ok(now) = carrier_changes(host, &w.dev) else {
                continue;
            };
            if w.carrier == Some(now) {
                w.moved = None;
                continue;
            }
            match w.moved {
                Some((seen, held)) if seen == now => {
                    if held + 1 >= SETTLE_READS {
                        w.carrier = Some(now);
                        w.moved = None;
                        due.push((w.dev.clone(), w.ctx.clone()));
                    } else {
                        w.moved = Some((seen, held + 1));
                    }
                }
                _ => w.moved = Some((now, 0)),
            }
        }
        due.iter()
            .flat_map(|(dev, ctx)| self.run(host, dev, ctx))
            .collect()
    }

    /// Whether any device is being watched (then [`DevChecks::recheck`] is
    /// worth calling).
    #[must_use]
    pub const fn watching(&self) -> bool {
        !self.watches.is_empty()
    }

    fn run(&mut self, host: &Host, dev: &str, ctx: &DevContext) -> Vec<Finding> {
        if let Err(why) = visible(host, dev) {
            return vec![invisible(dev, why)];
        }
        let mut out = Vec::new();
        if !ctx.hot.spinning().is_empty() {
            out.extend(t2_nic_irqs(host, &Nic::Interface(dev.to_owned()), &ctx.hot));
        }
        out.push(n1_adaptive_rx(host, dev, &mut self.cache));
        if ctx.interrupt_driven {
            out.extend(n2_deferred_irqs(host, dev, Some(false)));
        }
        out.push(n3_azure_vf(host, dev));
        out
    }
}

/// The one unknown finding for a device the checks cannot see.
fn invisible(dev: &str, why: String) -> Finding {
    Finding {
        id: CheckId::T2,
        severity: Severity::Warn,
        status: Status::Unknown(why),
        message: format!(
            "cannot check {dev}'s interrupts, interrupt moderation, deferral or \
             accelerated-networking VF (T2, N1, N2, N3)"
        ),
        evidence: "",
    }
}

/// Whether `dev` has a device the checks can read: a `device` link, or an
/// interface below it (`lower_*`: an Azure VF, the NICs under a VLAN or a
/// bond). The reason when not.
fn visible(host: &Host, dev: &str) -> Result<(), String> {
    if host.is_live() && !cfg!(target_os = "linux") {
        return Err("not Linux".to_owned());
    }
    if dev.is_empty() || dev.contains('/') || dev == "." || dev == ".." {
        return Err(format!("{dev:?} is not an interface name"));
    }
    let base = host.path(&format!("/sys/class/net/{dev}"));
    let lower = || {
        std::fs::read_dir(&base).is_ok_and(|d| {
            d.filter_map(Result::ok)
                .any(|e| e.file_name().to_string_lossy().starts_with("lower_"))
        })
    };
    if base.join("device").exists() || lower() {
        Ok(())
    } else {
        Err(format!(
            "{dev} has no device here (no /sys/class/net/{dev}/device and no lower_* \
             interface): a pod's veth, a bridge or a tunnel, whose NIC is in the host's network \
             namespace"
        ))
    }
}

/// I3: the line for an `SO_RCVBUF` of `requested` bytes that the kernel
/// granted as `granted` (what `getsockopt` returns: twice the size kept),
/// when the kernel cut it (`granted` below twice the request), else `None`.
#[must_use]
pub fn rcvbuf_line(requested: u32, granted: u32) -> Option<String> {
    if u64::from(granted) >= 2 * u64::from(requested) {
        return None;
    }
    Some(format!(
        "ergon-io warning: SO_RCVBUF asked {requested} bytes, granted {}: the kernel caps it at \
         net.core.rmem_max (socket(7)), and setting SO_RCVBUF also turns the kernel's \
         receive-buffer autotuning off (tcp(7)), so connections get the smaller, fixed buffer. \
         Fix: `sysctl -w net.core.rmem_max={requested}` (in the host's network namespace), or \
         rcvbuf=0 to leave the buffer to autotuning.",
        granted / 2
    ))
}

/// I1: the line for a connection handed to `Driver::add` with `TCP_CORK`
/// set.
#[must_use]
pub fn cork_line(conn: ConnId) -> String {
    format!(
        "ergon-io warning: connection {} has TCP_CORK set: partial frames wait up to 200 ms \
         before they are sent (tcp(7)). Clear it (setsockopt TCP_CORK 0) before Driver::add.",
        conn.index()
    )
}

/// I4: the line for a NIC-polling connection that polls no NIC.
///
/// That is a connection whose backend polls the NIC but whose socket had no
/// NAPI id after its handshake (`napi_id` `Some(0)`), on a route that is not
/// local; `None` otherwise, or when the id could not be read.
#[must_use]
pub fn napi_line(
    backend: Backend,
    conn: ConnId,
    flow: Option<&Flow>,
    route: &Route,
    napi_id: Option<u32>,
) -> Option<String> {
    if !backend.polls_nic() || route.is_local() || napi_id != Some(0) {
        return None;
    }
    let mut who = format!("connection {}", conn.index());
    if let Some(f) = flow {
        let _ = write!(who, " (TCP {} -> {})", f.local, f.peer);
    }
    let dev = route
        .dev
        .as_deref()
        .map_or_else(String::new, |d| format!(" on {d}"));
    Some(format!(
        "ergon-io warning: {who} is on {backend}, which polls the NIC, but its socket had no \
         NAPI id after the handshake: busy polling reaches no NIC queue{dev} (a device without \
         NAPI, such as a pod's veth), so the loop waits for interrupts. Paired per message on \
         Azure, NIC polling decoded the same market-data message 10.8–10.9 µs earlier at the \
         median than an interrupt-driven loop (Δ max 1,556–8,180 µs; io/benches/BENCHMARK.md, \
         NIC polling without sleeping). Fix: give the loop a NIC with NAPI (the host's network \
         namespace, or a CNI that passes the NIC through), or use an interrupt-driven backend \
         (epoll-spin, uring-spin), which is what this one amounts to here."
    ))
}

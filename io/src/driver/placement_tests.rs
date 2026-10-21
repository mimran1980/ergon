//! The driver's helper threads in a pod: fixture trees stand for the
//! process's cpuset (cgroup v2, its own namespace) and the polling thread's
//! CPU, so a one-CPU pod is tested on any Linux host. Only refusals and
//! connects on the polling thread are tested here: a helper placed on a
//! fixture's CPU could be outside the real cpuset.

use std::net::SocketAddr;
use std::path::PathBuf;

use ergon_tune::{CpuSet, Host};

use super::Driver;
use crate::backend::{Backend, Config};
use crate::dns::DnsConfig;
use crate::error::Error;
use crate::net::Target;
use crate::placement::{CpuError, HelperThread};
use crate::reconnect::{LinkState, Reconnect, Retry};
use crate::ws_conn::{WsConn, WsDial};

type R<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

/// A 10-CPU host (SMT pairs 0+1, 2+3, ...) where the process may use
/// `cpuset` and the thread creating the driver is pinned to `polling`.
struct Pod(PathBuf);

impl Drop for Pod {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

impl Pod {
    fn new(name: &str, cpuset: &str, polling: u32) -> R<Self> {
        let root =
            std::env::temp_dir().join(format!("ergon-io-driver-pod-{name}-{}", std::process::id()));
        if root.exists() {
            std::fs::remove_dir_all(&root)?;
        }
        let pod = Self(root);
        pod.file("/sys/devices/system/cpu/online", "0-9\n")?;
        pod.file("/sys/devices/system/cpu/isolated", "\n")?;
        for c in 0..10u32 {
            let pair = c & !1;
            pod.file(
                &format!("/sys/devices/system/cpu/cpu{c}/topology/thread_siblings_list"),
                &format!("{pair}-{}\n", pair + 1),
            )?;
        }
        pod.file("/proc/self/cgroup", "0::/\n")?;
        pod.file(
            "/sys/fs/cgroup/cpuset.cpus.effective",
            &format!("{cpuset}\n"),
        )?;
        pod.file(
            "/proc/thread-self/status",
            &format!("Name:\tloop\nCpus_allowed_list:\t{polling}\n"),
        )?;
        Ok(pod)
    }

    fn file(&self, abs: &str, text: &str) -> R {
        let p = self.0.join(abs.trim_start_matches('/'));
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(p, text)?;
        Ok(())
    }

    fn driver(&self, cfg: Config) -> crate::Result<Driver<WsConn>> {
        Driver::on_host(cfg, DnsConfig::system(), &Host::at(&self.0))
    }
}

/// A dial to a closed local port, never polled to completion.
fn dial() -> crate::Result<WsDial> {
    let addr = SocketAddr::from(([127, 0, 0, 1], 9));
    let tls = crate::tls::client_config(rustls::RootCertStore::empty())?;
    WsDial::new(
        Target::new("localhost", 9, "/").with_addr(addr),
        tls,
        1 << 20,
    )
}

fn cpu_error<T>(r: crate::Result<T>) -> R<CpuError> {
    match r {
        Err(Error::Cpu(e)) => Ok(*e),
        Err(other) => Err(format!("expected Error::Cpu, got {other}").into()),
        Ok(_) => Err("expected Error::Cpu, got Ok".into()),
    }
}

/// `cpu: 1`: `connect_background` refuses at once, says what to do, and
/// starts no thread; nothing is left connecting.
#[test]
fn in_a_single_cpu_pod_connect_background_refuses_at_once() -> R {
    let _helpers = super::background::dial_helper_test_lock();
    let pod = Pod::new("refuse", "2", 2)?;
    let mut d = pod.driver(Config::new(Backend::EpollSpin))?;
    let e = cpu_error(d.connect_background(dial()?))?;
    assert_eq!(
        e,
        CpuError::NoSpareCpu {
            helper: HelperThread::Dial,
            polling: Some(2),
            allowed: CpuSet::of([2])
        }
    );
    let text = Error::from(e).to_string();
    for part in ["Driver::connect", "inline_connect_fallback", "second CPU"] {
        assert!(text.contains(part), "{part:?} not in {text:?}");
    }
    assert!(d.background.is_none(), "a helper thread was started");
    assert!(d.background_ready().is_err());
    assert_eq!(d.connecting(), 0);
    assert!(d.is_empty());
    Ok(())
}

/// `cpu: 1` with `inline_connect_fallback`: the connect runs on the
/// polling thread, as `Driver::connect`.
#[test]
fn in_a_single_cpu_pod_the_fallback_connects_on_the_loop() -> R {
    let _helpers = super::background::dial_helper_test_lock();
    let pod = Pod::new("fallback", "2", 2)?;
    let mut cfg = Config::new(Backend::EpollSpin);
    cfg.inline_connect_fallback = true;
    let mut d = pod.driver(cfg)?;
    assert!(d.background_ready().is_ok());
    d.connect_background(dial()?)?;
    assert_eq!(d.connecting(), 1, "not set up on the polling thread");
    assert!(d.background.is_none(), "a helper thread was started");
    Ok(())
}

/// `cpu: 1`: `uring-sqpoll`'s `SQPOLL` thread would spin on the polling
/// CPU, so `Driver::new` refuses before creating the ring.
#[cfg(feature = "uring")]
#[test]
fn in_a_single_cpu_pod_uring_sqpoll_refuses() -> R {
    let pod = Pod::new("sqpoll", "2", 2)?;
    let e = cpu_error(pod.driver(Config::new(Backend::UringSqpoll)))?;
    assert_eq!(
        e,
        CpuError::NoSpareCpu {
            helper: HelperThread::SqPoll,
            polling: Some(2),
            allowed: CpuSet::of([2])
        }
    );
    assert!(Error::from(e).to_string().contains("recv-busy"));
    Ok(())
}

/// CPU settings outside the pod's cpuset are refused with the CPU and the
/// cpuset named; `connect_cpu` may not be the polling CPU either.
#[test]
fn cpu_settings_outside_the_pod_are_refused() -> R {
    let _helpers = super::background::dial_helper_test_lock();
    let pod = Pod::new("outside", "2,3", 2)?;
    let mut cfg = Config::new(Backend::EpollSpin);
    cfg.helper_cpus = vec![7];
    assert_eq!(
        cpu_error(pod.driver(cfg))?,
        CpuError::Outside {
            setting: "helper_cpus",
            cpu: 7,
            allowed: CpuSet::of([2, 3])
        }
    );
    for (cpu, want) in [
        (
            7,
            CpuError::Outside {
                setting: "connect_cpu",
                cpu: 7,
                allowed: CpuSet::of([2, 3]),
            },
        ),
        (
            2,
            CpuError::PollingCpu {
                setting: "connect_cpu",
                cpu: 2,
            },
        ),
    ] {
        let mut cfg = Config::new(Backend::EpollSpin);
        cfg.connect_cpu = Some(cpu);
        let mut d = pod.driver(cfg)?;
        assert_eq!(cpu_error(d.connect_background(dial()?))?, want);
        assert!(d.background.is_none());
    }
    #[cfg(feature = "uring")]
    {
        let mut cfg = Config::new(Backend::UringSqpoll);
        cfg.sqpoll_cpu = Some(7);
        assert_eq!(
            cpu_error(pod.driver(cfg))?,
            CpuError::Outside {
                setting: "sqpoll_cpu",
                cpu: 7,
                allowed: CpuSet::of([2, 3])
            }
        );
    }
    Ok(())
}

/// `loop_cpu` outside the pod's cpuset is refused at `Driver::new`, like
/// the other CPU settings: the polling thread runs inside the cpuset
/// whatever the setting claims, so helpers must not be placed around the
/// claimed CPU.
#[test]
fn a_loop_cpu_outside_the_pod_is_refused() -> R {
    let pod = Pod::new("loopcpu", "2,3", 2)?;
    let mut cfg = Config::new(Backend::EpollSpin);
    cfg.loop_cpu = Some(7);
    assert_eq!(
        cpu_error(pod.driver(cfg))?,
        CpuError::Outside {
            setting: "loop_cpu",
            cpu: 7,
            allowed: CpuSet::of([2, 3])
        }
    );
    Ok(())
}

/// `helper_cpus` hold every CPU but the polling one (the shared-work CPUs
/// a node plugin adds): `uring-sqpoll`'s spinner has no CPU of the pod's
/// own outside that list, so `Driver::new` refuses with `NoExclusiveCpu`
/// before creating the ring, rather than spin on a shared helper CPU.
#[cfg(feature = "uring")]
#[test]
fn sqpoll_will_not_default_onto_the_shared_helpers() -> R {
    let pod = Pod::new("sqshared", "2,7", 2)?;
    let mut cfg = Config::new(Backend::UringSqpoll);
    cfg.helper_cpus = vec![7];
    let e = cpu_error(pod.driver(cfg))?;
    assert_eq!(
        e,
        CpuError::NoExclusiveCpu {
            helper: HelperThread::SqPoll,
            setting: "helper_cpus",
            helpers: CpuSet::of([7]),
            polling: Some(2),
            allowed: CpuSet::of([2, 7])
        }
    );
    let text = Error::from(e).to_string();
    for part in ["helper_cpus", "sqpoll_cpu", "recv-busy"] {
        assert!(text.contains(part), "{part:?} not in {text:?}");
    }
    Ok(())
}

/// The host report names the choice; in a one-CPU pod it warns once.
#[test]
fn the_host_report_says_where_the_helpers_go() -> R {
    let one = Pod::new("report1", "2", 2)?;
    let d = one.driver(Config::new(Backend::EpollSpin))?;
    assert!(
        d.host_report()
            .lines()
            .iter()
            .any(|l| l.contains("the polling CPU only")),
        "{:#?}",
        d.host_report().lines()
    );
    let two = Pod::new("report2", "2,3", 2)?;
    let d = two.driver(Config::new(Backend::EpollSpin))?;
    let p = d.host_report().helpers.clone()?;
    assert_eq!(p.cpus, CpuSet::of([3]), "the loop's SMT sibling");
    Ok(())
}

fn ws_dial() -> crate::Result<WsDial> {
    dial()
}

/// `Reconnect` on a driver in a one-CPU pod: the link gives up at once;
/// with `inline_connect_fallback` it connects on the loop.
#[test]
fn reconnect_in_a_single_cpu_pod() -> R {
    let pod = Pod::new("reconnect", "2", 2)?;
    let retry = Retry {
        seed: 1,
        ..Retry::default()
    };
    let mut d = pod.driver(Config::new(Backend::EpollSpin))?;
    let mut rc = Reconnect::new(retry)?;
    let link = rc.add(ws_dial);
    assert_eq!(rc.poll_at(&mut d, 0), 0);
    assert_eq!(rc.state(link), Some(LinkState::GaveUp));
    let mut cfg = Config::new(Backend::EpollSpin);
    cfg.inline_connect_fallback = true;
    let mut d = pod.driver(cfg)?;
    let mut rc = Reconnect::new(retry)?;
    let link = rc.add(ws_dial);
    assert_eq!(rc.poll_at(&mut d, 0), 1);
    assert!(matches!(rc.state(link), Some(LinkState::Connecting(_))));
    assert_eq!(d.connecting(), 1);
    Ok(())
}

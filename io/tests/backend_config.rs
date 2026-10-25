//! Backend configuration strings.
#![allow(missing_docs)]

use ergon_io::{Backend, Config};

type R = Result<(), Box<dyn std::error::Error>>;

#[test]
fn every_backend_name_round_trips() -> R {
    for b in Backend::ALL {
        assert_eq!(b.name().parse::<Backend>()?, b);
        assert_eq!(Config::parse(b.name())?, Config::new(b));
    }
    Ok(())
}

// ANCHOR: config_strings
#[test]
fn keys_override_defaults() -> R {
    let c = Config::parse("uring-sqpoll, sqpoll_cpu=3, rcvbuf=0, rx_timestamps=false")?;
    assert_eq!(c.backend, Backend::UringSqpoll);
    assert_eq!(c.sqpoll_cpu, Some(3));
    assert_eq!(c.rcvbuf, None);
    assert!(!c.rx_timestamps);
    Ok(())
}
// ANCHOR_END: config_strings

#[test]
fn rx_read_timestamp_is_off_by_default_and_parses() -> R {
    assert!(!Config::new(Backend::RecvBusy).rx_read_timestamp);
    assert!(Config::parse("recv-busy, rx_read_timestamp=true")?.rx_read_timestamp);
    assert!(!Config::parse("recv-busy, rx_read_timestamp=false")?.rx_read_timestamp);
    assert!(Config::parse("recv-busy, rx_read_timestamp=yes").is_err());
    Ok(())
}

#[test]
fn malformed_input_is_rejected() {
    for bad in [
        "",
        "epoll",
        "epoll-spin,rcvbuf",
        "epoll-spin,rcvbuf=x",
        "epoll-spin,nope=1",
    ] {
        assert!(Config::parse(bad).is_err(), "{bad:?} accepted");
    }
}

#[test]
fn per_queue_applies_to_recv_busy_only() -> R {
    assert!(!Config::new(Backend::RecvBusy).per_queue);
    assert!(Config::parse("recv-busy,per_queue=true")?.per_queue);
    // ANCHOR: per_queue_errors
    for bad in [
        "epoll-spin,per_queue=true",   // error
        "uring-sqpoll,per_queue=true", // error
        "recv-busy,per_queue=1",       // error: not true/false
    ] {
        assert!(Config::parse(bad).is_err(), "{bad:?} accepted");
    }
    // ANCHOR_END: per_queue_errors
    Ok(())
}

// ANCHOR: one_core_backends
/// The backends that cost one core each. `uring-sqpoll` is not here: its
/// SQPOLL kernel thread spins on a CPU of its own.
#[test]
fn the_one_core_backends() -> R {
    // recv-busy: one recvmsg per socket per poll. Best at 1-4 sockets.
    let plain = Config::parse("recv-busy")?;
    assert_eq!(plain.backend, Backend::RecvBusy);
    assert!(!plain.per_queue);

    // per_queue: one busy-polled read per NIC queue, then only the sockets
    // epoll_wait(0) reports ready. The same latency as plain at 1-16 sockets,
    // and the only one-core choice still flat at 256.
    let queued = Config::parse("recv-busy,per_queue=true")?;
    assert_eq!(queued.backend, Backend::RecvBusy);
    assert!(queued.per_queue);

    // epoll-spin: epoll_wait(0), then recvmsg on the ready sockets. Cheapest
    // per socket, but it waits for the NIC interrupt (mio's mechanism).
    let epoll = Config::parse("epoll-spin")?;
    assert_eq!(epoll.backend, Backend::EpollSpin);
    assert!(!epoll.per_queue);

    Ok(())
}
// ANCHOR_END: one_core_backends

// ANCHOR: per_queue_struct
/// The same choice as struct fields, for when more than one setting changes.
#[test]
fn per_queue_as_struct_fields() {
    let mut cfg = Config::new(Backend::RecvBusy);
    cfg.per_queue = true; // one read per NIC queue, then epoll_wait(0)
    cfg.rcvbuf = Some(4 << 20);

    assert!(cfg.per_queue);
    assert_eq!(cfg.rcvbuf, Some(4 << 20));
}
// ANCHOR_END: per_queue_struct

#[test]
fn the_quickack_check_is_a_key_on_every_backend() -> R {
    use ergon_io::route::QuickAckCheck;
    for b in Backend::ALL {
        assert_eq!(Config::new(b).quickack_check, QuickAckCheck::Warn, "{b}");
        for p in [
            QuickAckCheck::Off,
            QuickAckCheck::Warn,
            QuickAckCheck::Require,
        ] {
            let c = Config::parse(&format!("{b},quickack_check={p}"))?;
            assert_eq!(c.quickack_check, p, "{b}");
        }
    }
    for bad in [
        "recv-busy,quickack_check=",
        "recv-busy,quickack_check=on",
        "recv-busy,quickack_check=true",
    ] {
        assert!(Config::parse(bad).is_err(), "{bad:?} accepted");
    }
    Ok(())
}

#[test]
fn tcp_keepalive_is_on_by_default_and_configurable() -> R {
    for b in Backend::ALL {
        let c = Config::new(b);
        assert_eq!(c.keepalive_idle_s, 10, "{b}");
        assert_eq!(c.keepalive_interval_s, 5, "{b}");
        assert_eq!(c.keepalive_count, 3, "{b}");
        // Idle plus every probe: the same deadline for unacknowledged data.
        assert_eq!(c.tcp_user_timeout_ms, 25_000, "{b}");
    }
    let c = Config::parse(
        "recv-busy,keepalive_idle_s=30,keepalive_interval_s=2,keepalive_count=4,tcp_user_timeout_ms=0",
    )?;
    assert_eq!(
        (
            c.keepalive_idle_s,
            c.keepalive_interval_s,
            c.keepalive_count,
            c.tcp_user_timeout_ms
        ),
        (30, 2, 4, 0)
    );
    assert_eq!(
        Config::parse("recv-busy,keepalive_idle_s=0")?.keepalive_idle_s,
        0
    );
    for bad in [
        // The kernel's limits: idle and interval up to 32767 s, count up to 127.
        "recv-busy,keepalive_idle_s=32768",
        "recv-busy,keepalive_interval_s=0",
        "recv-busy,keepalive_interval_s=32768",
        "recv-busy,keepalive_count=0",
        "recv-busy,keepalive_count=128",
        "recv-busy,tcp_user_timeout_ms=2147483648",
        "recv-busy,keepalive_idle_s=x",
    ] {
        assert!(Config::parse(bad).is_err(), "{bad:?} accepted");
    }
    // With keepalive off, the probe settings are not checked.
    assert!(Config::parse("recv-busy,keepalive_idle_s=0,keepalive_count=0").is_ok());
    Ok(())
}

/// The helper threads' CPUs (`helper_cpus`, a kernel CPU list whose commas
/// the parser keeps with the key) and the opt-in to connect on the loop when
/// the process has no CPU for the connect helper.
#[test]
fn helper_cpus_and_the_inline_connect_fallback_parse() -> R {
    let d = Config::new(Backend::RecvBusy);
    assert!(d.helper_cpus.is_empty(), "chosen at Driver::new by default");
    assert!(!d.inline_connect_fallback, "refuse by default");
    let c = Config::parse("recv-busy,helper_cpus=1,3-4,loop_cpu=2,inline_connect_fallback=true")?;
    assert_eq!(c.helper_cpus, vec![1, 3, 4]);
    assert_eq!(c.loop_cpu, Some(2));
    assert!(c.inline_connect_fallback);
    assert_eq!(
        Config::parse("recv-busy,helper_cpus=7")?.helper_cpus,
        vec![7]
    );
    for bad in [
        "recv-busy,helper_cpus=x",
        "recv-busy,helper_cpus=3-1",
        "recv-busy,helper_cpus=1,x",
        // A bare number belongs to helper_cpus only.
        "recv-busy,loop_cpu=2,3",
        "recv-busy,inline_connect_fallback=1",
    ] {
        assert!(Config::parse(bad).is_err(), "{bad:?} accepted");
    }
    Ok(())
}

/// The per-socket TCP timers (`TCP_RTO_MIN_US`, `TCP_DELACK_MAX_US`, Linux
/// 6.15+) and passive `TCP_INFO` sampling: all off by default, and a value
/// no kernel accepts is refused here, before any socket. The floor is 2 ms:
/// two timer ticks at `CONFIG_HZ=1000`, the finest tick there is (a coarser
/// tick is checked against the running kernel at `Driver::new`); the
/// ceiling is the kernel's own 200 ms (`HZ / 5` on every `CONFIG_HZ`).
#[test]
fn tcp_timers_and_tcp_info_are_off_by_default_and_bounded() -> R {
    for b in Backend::ALL {
        let c = Config::new(b);
        assert_eq!(c.tcp_rto_min_us, None, "{b}");
        assert_eq!(c.tcp_delack_max_us, None, "{b}");
        assert_eq!(c.tcp_info_interval_ms, 0, "{b}");
    }
    let c = Config::parse(
        "recv-busy,tcp_rto_min_us=5000,tcp_delack_max_us=2000,tcp_info_interval_ms=1000",
    )?;
    assert_eq!(c.tcp_rto_min_us, Some(5_000));
    assert_eq!(c.tcp_delack_max_us, Some(2_000));
    assert_eq!(c.tcp_info_interval_ms, 1_000);
    // 0 leaves the kernel's own value (200 ms, or a route's `rto_min`).
    let c = Config::parse("uring-sqpoll,tcp_rto_min_us=0,tcp_delack_max_us=0")?;
    assert_eq!((c.tcp_rto_min_us, c.tcp_delack_max_us), (None, None));
    assert!(Config::parse("epoll-spin,tcp_rto_min_us=200000,tcp_delack_max_us=200000").is_ok());
    for bad in [
        "recv-busy,tcp_rto_min_us=1999",
        "recv-busy,tcp_rto_min_us=200001",
        "recv-busy,tcp_delack_max_us=1",
        "recv-busy,tcp_delack_max_us=200001",
        "recv-busy,tcp_rto_min_us=x",
        "recv-busy,tcp_info_interval_ms=-1",
    ] {
        let e = Config::parse(bad)
            .err()
            .ok_or_else(|| format!("{bad:?} accepted"))?;
        assert!(matches!(e, ergon_io::Error::Config(_)), "{bad:?}: {e}");
    }
    // A struct-built config is checked the same way.
    let mut c = Config::new(Backend::EpollSpin);
    c.tcp_rto_min_us = Some(1_000);
    assert!(c.validate().is_err(), "a floor below 2 ms validated");
    Ok(())
}

/// Only the backends the benchmarks measured in a busy-spin loop are left:
/// `epoll-spin` (interrupt-driven), `recv-busy` and `uring-sqpoll` (NIC
/// polling). `uring-spin`, `epoll-busy` and `uring-napi` are unknown names.
#[test]
fn only_the_three_measured_backends_exist() -> R {
    let names: Vec<&str> = Backend::ALL.iter().map(|b| b.name()).collect();
    assert_eq!(names, ["epoll-spin", "recv-busy", "uring-sqpoll"]);
    for gone in ["uring-spin", "epoll-busy", "uring-napi"] {
        let e = gone
            .parse::<Backend>()
            .err()
            .ok_or_else(|| format!("{gone:?} accepted"))?;
        assert!(matches!(e, ergon_io::Error::Config(_)), "{gone}: {e}");
        assert!(Config::parse(gone).is_err(), "{gone:?} accepted");
    }
    Ok(())
}

/// The busy-poll knobs that only the sleeping backends used, or that no run
/// measured, are unknown keys: the driver sets one NAPI pass per empty read
/// itself.
#[test]
fn the_unmeasured_busy_poll_keys_are_unknown() -> R {
    for key in [
        "busy_poll_us=1",
        "wait_us=0",
        "prefer_busy_poll=false",
        "busy_poll_budget=8",
    ] {
        let s = format!("recv-busy,{key}");
        let e = Config::parse(&s)
            .err()
            .ok_or_else(|| format!("{s:?} accepted"))?;
        assert!(
            matches!(&e, ergon_io::Error::Config(m) if m.contains("unknown key")),
            "{s}: {e}"
        );
    }
    Ok(())
}

/// The `epoll_wait` batch and the UDP read burst: defaults 64 and 16, keys
/// on every backend, out-of-range values refused typed (`epoll_wait` takes
/// a positive count; a burst of 0 would never read).
#[test]
fn epoll_events_and_datagram_burst_parse_and_are_bounded() -> R {
    for b in Backend::ALL {
        let c = Config::new(b);
        assert_eq!((c.epoll_events, c.datagram_burst), (64, 16), "{b}");
    }
    let c = Config::parse("epoll-spin,epoll_events=256,datagram_burst=1")?;
    assert_eq!((c.epoll_events, c.datagram_burst), (256, 1));
    let c = Config::parse("epoll-spin,epoll_events=65536,datagram_burst=1024")?;
    assert_eq!((c.epoll_events, c.datagram_burst), (65_536, 1_024));
    for bad in [
        "epoll-spin,epoll_events=0",
        "epoll-spin,epoll_events=65537",
        "epoll-spin,epoll_events=x",
        "epoll-spin,datagram_burst=0",
        "epoll-spin,datagram_burst=1025",
        "epoll-spin,datagram_burst=-1",
    ] {
        let e = Config::parse(bad)
            .err()
            .ok_or_else(|| format!("{bad:?} accepted"))?;
        assert!(matches!(e, ergon_io::Error::Config(_)), "{bad:?}: {e}");
    }
    let mut c = Config::new(Backend::EpollSpin);
    c.epoll_events = 0;
    assert!(c.validate().is_err(), "an empty epoll batch validated");
    let mut c = Config::new(Backend::EpollSpin);
    c.datagram_burst = 0;
    assert!(c.validate().is_err(), "a zero burst validated");
    Ok(())
}

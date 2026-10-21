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
    let c =
        Config::parse("uring-sqpoll, sqpoll_cpu=3, busy_poll_us=2, rcvbuf=0, rx_timestamps=false")?;
    assert_eq!(c.backend, Backend::UringSqpoll);
    assert_eq!(c.sqpoll_cpu, Some(3));
    assert_eq!(c.busy_poll_us, 2);
    assert_eq!(c.rcvbuf, None);
    assert!(!c.rx_timestamps);
    Ok(())
}
// ANCHOR_END: config_strings

#[test]
fn busy_poll_preference_and_budget_parse() -> R {
    let d = Config::new(Backend::RecvBusy);
    assert!(!d.prefer_busy_poll);
    assert_eq!(d.busy_poll_budget, None);
    let c = Config::parse("recv-busy, prefer_busy_poll=true, busy_poll_budget=64")?;
    assert!(c.prefer_busy_poll);
    assert_eq!(c.busy_poll_budget, Some(64));
    assert_eq!(
        Config::parse("recv-busy,busy_poll_budget=0")?.busy_poll_budget,
        None
    );
    for bad in [
        "recv-busy,prefer_busy_poll=1",
        "recv-busy,busy_poll_budget=65536",
        // Only backends that poll the NIC can prefer busy polling, and only the
        // socket-level ones take a packet budget.
        "epoll-spin,prefer_busy_poll=true",
        "uring-spin,prefer_busy_poll=true",
        "uring-sqpoll,busy_poll_budget=16",
        // The kernel takes the busy-poll time as a signed int.
        "recv-busy,busy_poll_us=2147483648",
    ] {
        assert!(Config::parse(bad).is_err(), "{bad:?} accepted");
    }
    let mut c = Config::new(Backend::UringSpin);
    c.prefer_busy_poll = true;
    assert!(c.validate().is_err(), "struct-built config not validated");
    assert!(Config::parse("uring-sqpoll,prefer_busy_poll=true")?.prefer_busy_poll);
    assert!(Config::parse("epoll-busy,prefer_busy_poll=true,busy_poll_budget=32").is_ok());
    assert!(Config::parse("recv-busy,busy_poll_us=2147483647").is_ok());
    Ok(())
}

#[test]
fn malformed_input_is_rejected() {
    for bad in [
        "",
        "epoll",
        "epoll-spin,wait_us",
        "epoll-spin,wait_us=x",
        "epoll-spin,nope=1",
    ] {
        assert!(Config::parse(bad).is_err(), "{bad:?} accepted");
    }
}

#[test]
fn only_busy_poll_wait_backends_may_sleep() {
    for b in Backend::ALL {
        assert_eq!(
            b.may_sleep(),
            matches!(b, Backend::EpollBusy | Backend::UringNapi),
            "{b}"
        );
    }
}

#[test]
fn per_queue_applies_to_recv_busy_only() -> R {
    assert!(!Config::new(Backend::RecvBusy).per_queue);
    assert!(Config::parse("recv-busy,per_queue=true")?.per_queue);
    // ANCHOR: per_queue_errors
    for bad in [
        "epoll-spin,per_queue=true",   // error
        "uring-spin,per_queue=true",   // error
        "uring-sqpoll,per_queue=true", // error
        "recv-busy,per_queue=1",       // error: not true/false
    ] {
        assert!(Config::parse(bad).is_err(), "{bad:?} accepted");
    }
    // ANCHOR_END: per_queue_errors
    Ok(())
}

// ANCHOR: one_core_backends
/// The four backends that cost one core each. `uring-sqpoll` is not here:
/// its SQPOLL kernel thread spins on a CPU of its own.
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
    assert_eq!(queued.busy_poll_us, 1);

    // epoll-spin: epoll_wait(0), then recvmsg on the ready sockets. Cheapest
    // per socket, but it waits for the NIC interrupt.
    let epoll = Config::parse("epoll-spin")?;
    assert_eq!(epoll.backend, Backend::EpollSpin);
    assert!(!epoll.per_queue);

    // uring-spin: IORING_OP_RECVMSG with DEFER_TASKRUN. The fewest syscalls of
    // any spin loop; also interrupt-driven.
    let uring = Config::parse("uring-spin")?;
    assert_eq!(uring.backend, Backend::UringSpin);
    assert!(!uring.per_queue);

    Ok(())
}
// ANCHOR_END: one_core_backends

// ANCHOR: per_queue_struct
/// The same choice as struct fields, for when more than one setting changes.
#[test]
fn per_queue_as_struct_fields() {
    let mut cfg = Config::new(Backend::RecvBusy);
    cfg.per_queue = true; // one read per NIC queue, then epoll_wait(0)
    cfg.busy_poll_us = 1; // one NAPI pass per empty read
    cfg.rcvbuf = Some(4 << 20);

    assert!(cfg.per_queue);
    assert_eq!(cfg.busy_poll_us, 1);
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

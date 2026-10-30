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
    for b in Backend::KERNEL {
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
    for b in Backend::KERNEL {
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

/// Only the kernel backends the benchmarks measured in a busy-spin loop are
/// left: `epoll-spin` (interrupt-driven), `recv-busy` and `uring-sqpoll`
/// (NIC polling); `fstack-spin` and `smoltcp-spin` bypass the kernel.
/// `uring-spin`, `epoll-busy` and `uring-napi` are unknown names.
#[test]
fn only_the_three_measured_backends_exist() -> R {
    let names: Vec<&str> = Backend::KERNEL.iter().map(|b| b.name()).collect();
    assert_eq!(names, ["epoll-spin", "recv-busy", "uring-sqpoll"]);
    let all: Vec<&str> = Backend::ALL.iter().map(|b| b.name()).collect();
    assert_eq!(
        all,
        [
            "epoll-spin",
            "recv-busy",
            "uring-sqpoll",
            "fstack-spin",
            "smoltcp-spin"
        ]
    );
    assert!(
        Backend::ALL
            .iter()
            .all(|b| b.bypasses_kernel() != Backend::KERNEL.contains(b))
    );
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
    for key in ["wait_us=0", "prefer_busy_poll=false", "busy_poll_budget=8"] {
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

/// `busy_poll_us`: `SO_BUSY_POLL` on every socket `epoll-spin` owns, 0 (the
/// default: no busy polling) up to the kernel's `INT_MAX`. The other
/// backends set their own (`recv-busy` 1 µs, `uring-sqpoll` 0) and refuse
/// the key typed.
#[test]
fn busy_poll_us_is_epoll_spins_and_bounded() -> R {
    for b in Backend::ALL {
        assert_eq!(Config::new(b).busy_poll_us, 0, "{b}");
    }
    assert_eq!(
        Config::parse("epoll-spin,busy_poll_us=50")?.busy_poll_us,
        50
    );
    assert_eq!(Config::parse("epoll-spin,busy_poll_us=0")?.busy_poll_us, 0);
    let max = Config::parse("epoll-spin,busy_poll_us=2147483647")?;
    assert_eq!(max.busy_poll_us, 2_147_483_647);
    for bad in [
        "epoll-spin,busy_poll_us=2147483648",
        "epoll-spin,busy_poll_us=-1",
        "epoll-spin,busy_poll_us=x",
        "recv-busy,busy_poll_us=50",
        "uring-sqpoll,busy_poll_us=1",
    ] {
        let e = Config::parse(bad)
            .err()
            .ok_or_else(|| format!("{bad:?} accepted"))?;
        assert!(
            matches!(&e, ergon_io::Error::Config(m) if m.contains("busy_poll_us")),
            "{bad}: {e}"
        );
    }
    let mut c = Config::new(Backend::RecvBusy);
    c.busy_poll_us = 1;
    assert!(matches!(c.validate(), Err(ergon_io::Error::Config(_))));
    let mut c = Config::new(Backend::EpollSpin);
    c.busy_poll_us = u32::MAX;
    assert!(matches!(c.validate(), Err(ergon_io::Error::Config(_))));
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

/// `fstack-spin`'s defaults leave out what F-Stack lacks (receive
/// timestamps, the kernel's quick-ACK route check, `TCP_USER_TIMEOUT`); its
/// `fstack_*` keys parse (a vdev's commas written `;`), and are refused on
/// every other backend.
#[test]
fn fstack_spin_defaults_and_keys() -> R {
    use ergon_io::route::QuickAckCheck;
    let defaults = Config::new(Backend::FstackSpin);
    assert!(!defaults.rx_timestamps);
    assert_eq!(defaults.quickack_check, QuickAckCheck::Off);
    assert_eq!(defaults.tcp_user_timeout_ms, 0);
    assert_eq!(
        (
            defaults.keepalive_idle_s,
            defaults.keepalive_interval_s,
            defaults.keepalive_count
        ),
        (10, 5, 3)
    );
    assert!(defaults.fstack.is_default());
    let parsed = Config::parse(
        "fstack-spin,loop_cpu=2,fstack_addr=10.9.0.1,fstack_prefix_len=24,fstack_gateway=10.9.0.254,\
         fstack_vdev=net_af_packet0;iface=va,fstack_no_huge=true,fstack_memory_mb=512,\
         fstack_file_prefix=client,fstack_sysctl=net.inet.tcp.blackhole=0,fstack_hz=1000",
    )?;
    let fs = &parsed.fstack;
    assert_eq!(fs.addr, Some([10, 9, 0, 1].into()));
    assert_eq!(fs.gateway, Some([10, 9, 0, 254].into()));
    assert_eq!(fs.vdev.as_deref(), Some("net_af_packet0,iface=va"));
    assert!(fs.no_huge && fs.pci.is_none());
    assert_eq!(
        (fs.memory_mb, fs.hz, fs.file_prefix.as_str()),
        (512, 1000, "client")
    );
    assert_eq!(
        fs.sysctl,
        [("net.inet.tcp.blackhole".to_owned(), "0".to_owned())]
    );
    fs.check()?;
    for b in Backend::KERNEL {
        let line = format!("{b},fstack_addr=10.9.0.1");
        let e = Config::parse(&line)
            .err()
            .ok_or_else(|| format!("{line:?} accepted"))?;
        assert!(
            matches!(&e, ergon_io::Error::Config(m) if m.contains("fstack-spin")),
            "{line}: {e}"
        );
    }
    Ok(())
}

/// What the kernel has and F-Stack does not is refused typed, naming what
/// F-Stack does instead.
#[test]
fn fstack_spin_refuses_kernel_only_keys() -> R {
    for bad in [
        "fstack-spin,tcp_user_timeout_ms=1000",
        "fstack-spin,tcp_rto_min_us=5000",
        "fstack-spin,tcp_delack_max_us=5000",
        "fstack-spin,quickack_check=warn",
        "fstack-spin,rx_timestamps=true",
        "fstack-spin,per_queue=true",
        "fstack-spin,busy_poll_us=1",
        "fstack-spin,fstack_addr=10.9.0",
        "fstack-spin,fstack_sysctl=novalue",
    ] {
        let e = Config::parse(bad)
            .err()
            .ok_or_else(|| format!("{bad:?} accepted"))?;
        assert!(matches!(e, ergon_io::Error::Config(_)), "{bad:?}: {e}");
    }
    // Incomplete settings parse (a `Config` is built up), and are refused
    // where F-Stack would start.
    let c = Config::parse("fstack-spin,fstack_addr=10.9.0.1")?;
    let e = c
        .fstack
        .check()
        .err()
        .ok_or("no gateway or port accepted")?;
    assert!(e.to_string().contains("fstack_gateway"), "{e}");
    Ok(())
}

/// `smoltcp-spin`: the kernel's per-socket keys it cannot honour are
/// refused, the rest parse; its stack's settings are `StackConfig`'s.
#[test]
fn smoltcp_spin_refuses_what_its_stack_cannot_do() -> R {
    let c = Config::parse("smoltcp-spin")?;
    assert_eq!(c.backend, Backend::SmoltcpSpin);
    assert!(c.backend.polls_nic());
    for bad in [
        // The RTO floor is the stack's (StackConfig::rto_min_us).
        "smoltcp-spin,tcp_rto_min_us=8000",
        "smoltcp-spin,quickack_check=require",
        "smoltcp-spin,per_queue=true",
        "smoltcp-spin,busy_poll_us=50",
        // smoltcp aborts a connection that hears nothing for the user
        // timeout, idle or not: keepalive must be on and shorter.
        "smoltcp-spin,keepalive_idle_s=0",
        "smoltcp-spin,keepalive_idle_s=30,tcp_user_timeout_ms=25000",
    ] {
        let e = Config::parse(bad)
            .err()
            .ok_or_else(|| format!("{bad:?} accepted"))?;
        assert!(matches!(e, ergon_io::Error::Config(_)), "{bad}: {e}");
    }
    for good in [
        "smoltcp-spin,tcp_delack_max_us=5000",
        "smoltcp-spin,keepalive_idle_s=0,tcp_user_timeout_ms=0",
        "smoltcp-spin,keepalive_idle_s=5,tcp_user_timeout_ms=6000",
        "smoltcp-spin,quickack_check=off,rcvbuf=0,rx_timestamps=false",
    ] {
        Config::parse(good)?;
    }
    Ok(())
}

/// `StackConfig::parse`: every key, `eal=` last and taking the rest
/// (commas included), and the checks.
#[test]
fn the_smoltcp_stack_config_parses_and_checks() -> R {
    use ergon_io::smoltcp_dpdk::{Congestion, StackConfig};
    let c = StackConfig::parse(
        "port=0000:00:06.0,ip=10.1.2.3/20,gw=10.1.0.1,gw_mac=02:aa:bb:cc:dd:ee,mtu=9001,\
         rx_desc=2048,tx_desc=512,mbufs=16383,max_sockets=8,tcp_rx_buf=65536,tcp_tx_buf=32768,\
         udp_buf=4096,udp_packets=16,rto_min_us=8000,cc=none,timestamps=false,\
         tx_csum_offload=false,link_wait_ms=100,eal=-a 0000:00:06.0,llq_policy=1 --huge-dir /mnt/h",
    )?;
    assert_eq!(c.port, "0000:00:06.0");
    assert_eq!((c.ip, c.prefix_len), ("10.1.2.3".parse()?, 20));
    assert_eq!(c.gateway, Some("10.1.0.1".parse()?));
    assert_eq!(c.gateway_mac, Some([2, 0xaa, 0xbb, 0xcc, 0xdd, 0xee]));
    assert_eq!(
        (c.mtu, c.rx_desc, c.tx_desc, c.mbufs),
        (9001, 2048, 512, 16383)
    );
    assert_eq!(
        (c.max_sockets, c.tcp_rx_buf, c.tcp_tx_buf),
        (8, 65536, 32768)
    );
    assert_eq!((c.udp_buf, c.udp_packets, c.rto_min_us), (4096, 16, 8000));
    assert_eq!(c.congestion, Congestion::None);
    assert!(!c.tcp_timestamps && !c.tx_checksum_offload);
    assert_eq!(c.link_wait_ms, 100);
    assert_eq!(
        c.eal_args,
        ["-a", "0000:00:06.0,llq_policy=1", "--huge-dir", "/mnt/h"]
    );
    assert!((c.clock_scale() - 125.0).abs() < 1e-9);
    assert!(c.on_link("10.1.15.255".parse()?) && !c.on_link("10.1.16.1".parse()?));

    // Defaults: a /24, a 10 ms RTO floor, Reno, timestamps on.
    let d = StackConfig::parse("ip=192.168.7.9")?;
    assert_eq!((d.prefix_len, d.rto_min_us, d.mtu), (24, 10_000, 1500));
    assert_eq!(d.congestion, Congestion::Reno);
    assert!(d.tcp_timestamps && d.tx_checksum_offload && d.eal_args.is_empty());
    assert_eq!(
        d,
        StackConfig {
            ip: "192.168.7.9".parse()?,
            ..StackConfig::default()
        }
    );

    for bad in [
        "",                                                 // no address
        "ip=0.0.0.0",                                       // not unicast
        "ip=10.0.0.2/33",                                   // prefix
        "ip=10.0.0.2,gw=10.0.1.1",                          // gateway off the prefix
        "ip=10.0.0.2,gw=10.0.0.2",                          // gateway is us
        "ip=10.0.0.2,gw_mac=02:00:00:00:00:01",             // MAC without a gateway
        "ip=10.0.0.2,gw=10.0.0.1,gw_mac=01:00:5e:00:00:01", // multicast MAC
        "ip=10.0.0.2,gw=10.0.0.1,gw_mac=02:00:00:00:00",    // short MAC
        "ip=10.0.0.2,mtu=500",
        "ip=10.0.0.2,mtu=70000",
        "ip=10.0.0.2,rto_min_us=1000",
        "ip=10.0.0.2,rto_min_us=2000000",
        "ip=10.0.0.2,mbufs=100",
        "ip=10.0.0.2,max_sockets=0",
        "ip=10.0.0.2,tcp_rx_buf=100",
        "ip=10.0.0.2,udp_packets=0",
        "ip=10.0.0.2,cc=cubic",
        "ip=10.0.0.2,timestamps=yes",
        "ip=10.0.0.2,port=",
        "ip=10.0.0.2,nope=1",
        "ip=10.0.0.2,mtu",
    ] {
        let e = StackConfig::parse(bad)
            .err()
            .ok_or_else(|| format!("{bad:?} accepted"))?;
        assert!(matches!(e, ergon_io::Error::Config(_)), "{bad}: {e}");
    }
    Ok(())
}

/// Without the `smoltcp-dpdk` feature, or off Linux, the stack cannot start
/// and neither can a `smoltcp-spin` driver: `Error::Unsupported`, after the
/// settings are checked.
#[cfg(not(all(target_os = "linux", feature = "smoltcp-dpdk")))]
#[test]
fn without_the_feature_the_stack_is_unsupported() -> R {
    use ergon_io::smoltcp_dpdk::{Stack, StackConfig};
    let cfg = StackConfig::parse("ip=10.0.0.2")?;
    let e = Stack::init(&cfg).err().ok_or("a stack started")?;
    assert!(matches!(e, ergon_io::Error::Unsupported(_)), "{e}");
    let bad = StackConfig::default();
    let e = Stack::init(&bad).err().ok_or("a stack started")?;
    assert!(matches!(e, ergon_io::Error::Config(_)), "{e}");
    Ok(())
}

/// The ENA device plugin's NIC pair, as a pod granted
/// `rusteron.io/dpdk-ena-pair` sees it.
const PAIR: [(&str, &str); 3] = [
    ("RUSTERON_DPDK_SENDER_PCI", "0000:00:06.0"),
    ("RUSTERON_DPDK_SENDER_IPV4_CIDR", "10.0.1.20/24"),
    ("RUSTERON_DPDK_SENDER_GATEWAY", "10.0.1.1"),
];

fn env<'a>(vars: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
    move |name| {
        vars.iter()
            .find(|(k, _)| *k == name)
            .map(|(_, v)| (*v).to_owned())
    }
}

/// A bare `fstack-spin` takes the address, gateway and PCI device from
/// the pair the device plugin granted; the kernel backends never read it.
#[test]
fn a_bare_fstack_spin_takes_the_granted_pair() -> R {
    let c = Config::parse_with_env("fstack-spin,loop_cpu=2", env(&PAIR))?;
    assert_eq!(c.fstack.addr, Some("10.0.1.20".parse()?));
    assert_eq!(c.fstack.prefix_len, 24);
    assert_eq!(c.fstack.gateway, Some("10.0.1.1".parse()?));
    assert_eq!(c.fstack.pci.as_deref(), Some("0000:00:06.0"));
    assert_eq!(c.fstack.vdev, None);
    c.fstack.check()?;
    for b in [Backend::EpollSpin, Backend::RecvBusy, Backend::SmoltcpSpin] {
        assert_eq!(
            Config::parse_with_env(b.name(), env(&PAIR))?,
            Config::new(b)
        );
    }
    Ok(())
}

/// Any NIC key in the line wins: the pair is then not read at all, so a
/// line never mixes typed and granted values.
#[test]
fn a_named_nic_is_never_mixed_with_the_granted_pair() -> R {
    let c = Config::parse_with_env("fstack-spin,fstack_vdev=net_tap0", env(&PAIR))?;
    assert_eq!(c.fstack.vdev.as_deref(), Some("net_tap0"));
    assert_eq!(
        (c.fstack.addr, c.fstack.gateway, c.fstack.pci),
        (None, None, None)
    );
    let c = Config::parse_with_env("fstack-spin,fstack_addr=10.9.0.2", env(&PAIR))?;
    assert_eq!(c.fstack.addr, Some("10.9.0.2".parse()?));
    assert_eq!((c.fstack.gateway, c.fstack.pci), (None, None));
    Ok(())
}

/// No pair and no NIC keys: the check names both ways to give the NIC.
#[test]
fn without_a_pair_fstack_spin_names_both_ways() -> R {
    let c = Config::parse_with_env("fstack-spin", env(&[]))?;
    let e = c.fstack.check().err().ok_or("no NIC must fail the check")?;
    let m = e.to_string();
    assert!(m.contains("fstack_addr"), "{m}");
    assert!(m.contains("RUSTERON_DPDK_SENDER_PCI"), "{m}");
    Ok(())
}

/// A pair that is granted but broken fails the parse, naming the key.
#[test]
fn a_broken_pair_fails_the_parse_with_the_key() -> R {
    let half = [PAIR[0], PAIR[1]];
    let e = Config::parse_with_env("fstack-spin", env(&half))
        .err()
        .ok_or("a half pair must fail")?;
    assert!(
        e.to_string().contains("RUSTERON_DPDK_SENDER_GATEWAY"),
        "{e}"
    );
    Ok(())
}

/// A smoltcp stack line that names no NIC (no `port`, `ip`, `gw` or
/// `eal`) takes the granted pair; one that names any of them does not.
#[test]
fn a_smoltcp_line_without_a_nic_takes_the_granted_pair() -> R {
    use ergon_io::smoltcp_dpdk::StackConfig;
    for line in ["", "mtu=9001"] {
        let s = StackConfig::parse_with_env(line, env(&PAIR))?;
        assert_eq!(s.port, "0000:00:06.0", "{line:?}");
        assert_eq!((s.ip, s.prefix_len), ("10.0.1.20".parse()?, 24), "{line:?}");
        assert_eq!(s.gateway, Some("10.0.1.1".parse()?), "{line:?}");
        assert_eq!(s.eal_args, ["-a", "0000:00:06.0"], "{line:?}");
    }
    assert_eq!(
        StackConfig::parse_with_env("mtu=9001", env(&PAIR))?.mtu,
        9001
    );
    let typed = StackConfig::parse_with_env("ip=10.9.0.2", env(&PAIR))?;
    assert_eq!((typed.ip, typed.gateway), ("10.9.0.2".parse()?, None));
    assert_eq!(typed.eal_args, Vec::<String>::new());
    let e = StackConfig::parse_with_env("mtu=9001", env(&[]))
        .err()
        .ok_or("no NIC must fail")?;
    assert!(e.to_string().contains("RUSTERON_DPDK_SENDER_PCI"), "{e}");
    Ok(())
}

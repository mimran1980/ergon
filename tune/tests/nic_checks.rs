//! The NIC checks N1-N4 against fixture trees, with a stand-in for the
//! kernel's ethtool answers, and the cache that keeps those answers until
//! the interface resets.
#![cfg(feature = "nic")]
#![allow(missing_docs)]

mod common;

use std::collections::HashMap;

use common::{azure, c7i_tuned, c9g_tuned, debian_default, pod};
use ergon_tune::{
    CheckId, Coalesce, Ethtool, EthtoolCache, EthtoolError, Finding, Host, Netlink, Rings,
    Severity, Status, n1_adaptive_rx, n2_deferred_irqs, n3_azure_vf, n4_rx_ring,
};

type R = Result<(), Box<dyn std::error::Error>>;

const EOPNOTSUPP: i32 = 95;
const ENODEV: i32 = 19;

/// Answers by interface index, counting the questions.
#[derive(Default)]
struct Fake {
    coalesce: HashMap<u32, Result<Coalesce, EthtoolError>>,
    rings: HashMap<u32, Result<Rings, EthtoolError>>,
    asked: Vec<(&'static str, u32)>,
}

impl Ethtool for Fake {
    fn coalesce(&mut self, _host: &Host, ifindex: u32) -> Result<Coalesce, EthtoolError> {
        self.asked.push(("coalesce", ifindex));
        self.coalesce
            .get(&ifindex)
            .cloned()
            .unwrap_or(Err(EthtoolError::Kernel(ENODEV)))
    }

    fn rings(&mut self, _host: &Host, ifindex: u32) -> Result<Rings, EthtoolError> {
        self.asked.push(("rings", ifindex));
        self.rings
            .get(&ifindex)
            .cloned()
            .unwrap_or(Err(EthtoolError::Kernel(ENODEV)))
    }
}

/// ENA's coalescing with adaptive moderation `on`.
const fn adaptive(on: bool) -> Coalesce {
    Coalesce {
        use_adaptive_rx: Some(on),
        rx_usecs: Some(20),
    }
}

/// An RX ring of `rx` entries of at most `max`.
const fn ring(rx: u32, max: u32) -> Rings {
    Rings {
        rx: Some(rx),
        rx_max: Some(max),
    }
}

fn cache_with(f: impl FnOnce(&mut Fake)) -> EthtoolCache<Fake> {
    let mut fake = Fake::default();
    f(&mut fake);
    EthtoolCache::with_source(fake)
}

// ---------------------------------------------------------------- N1

// ANCHOR: n1
#[test]
fn n1_warns_while_ena_adaptive_moderation_is_on() -> R {
    let f = c7i_tuned("n1-on")?;
    let mut cache = cache_with(|k| {
        k.coalesce.insert(3, Ok(adaptive(true)));
    });
    let v = n1_adaptive_rx(&f.host(), "ens6", &mut cache);
    assert_eq!(v.id, CheckId::N1);
    assert!(v.is_warning(), "{v:?}");
    assert!(
        v.message.contains("ethtool -C ens6 adaptive-rx off"),
        "{}",
        v.message
    );
    assert!(v.message.contains("18–33 a second"), "{}", v.message);
    assert!(
        v.message.contains("p99.99 was 72–76 against 117–119 µs"),
        "{}",
        v.message
    );
    assert!(v.message.contains("is not measured"), "{}", v.message);
    // The c7i 100 k/s moderation pair (p50 106 → 71 µs) was withdrawn: those
    // figures came from an idle, single-rep run with moderation on. It must not
    // come back into a live finding.
    assert!(!v.message.contains("106 → 71"), "{}", v.message);
    assert!(!v.message.contains("606 → 560"), "{}", v.message);
    assert!(v.message.contains("every reset"), "{}", v.message);
    assert!(!v.message.contains("Graviton"), "{}", v.message);
    // The same advice on Graviton, with AWS's runbook.
    let g = c9g_tuned("n1-graviton")?;
    let v = n1_adaptive_rx(&g.host(), "ens6", &mut cache);
    assert!(v.is_warning() && v.message.contains("Graviton"), "{v:?}");
    Ok(())
}
// ANCHOR_END: n1

#[test]
fn n1_is_ok_when_off_not_offered_or_not_ena() -> R {
    let f = c7i_tuned("n1-off")?;
    let mut cache = cache_with(|k| {
        k.coalesce.insert(3, Ok(adaptive(false)));
    });
    assert_eq!(
        n1_adaptive_rx(&f.host(), "ens6", &mut cache).status,
        Status::Ok
    );
    let mut cache = cache_with(|k| {
        k.coalesce.insert(3, Ok(Coalesce::default()));
    });
    let v = n1_adaptive_rx(&f.host(), "ens6", &mut cache);
    assert_eq!(v.status, Status::Ok, "{v:?}");
    let mut cache = cache_with(|k| {
        k.coalesce.insert(3, Err(EthtoolError::Kernel(EOPNOTSUPP)));
    });
    assert_eq!(
        n1_adaptive_rx(&f.host(), "ens6", &mut cache).status,
        Status::Ok
    );
    // Azure's netvsc and MANA: not ENA, not asked.
    let a = azure("n1-azure")?;
    let mut cache = cache_with(|_| {});
    for i in ["eth1", "enP30832s1"] {
        let v = n1_adaptive_rx(&a.host(), i, &mut cache);
        assert_eq!(v.status, Status::Ok, "{v:?}");
        assert!(v.message.contains("ENA"), "{v:?}");
    }
    assert_eq!(cache.queries(), 0);
    Ok(())
}

#[test]
fn n1_is_unknown_without_a_device_or_an_answer() -> R {
    let p = pod("n1-pod")?;
    let mut cache = cache_with(|_| {});
    let v = n1_adaptive_rx(&p.host(), "eth0", &mut cache);
    assert!(
        matches!(&v.status, Status::Unknown(r) if r.contains("no device")),
        "{v:?}"
    );
    let f = c7i_tuned("n1-timeout")?;
    let mut cache = cache_with(|k| {
        k.coalesce.insert(3, Err(EthtoolError::Timeout));
    });
    assert!(n1_adaptive_rx(&f.host(), "ens6", &mut cache).is_unknown());
    // The real client never asks the kernel about a fixture tree.
    let mut real = EthtoolCache::new();
    let v = n1_adaptive_rx(&f.host(), "ens6", &mut real);
    assert!(
        matches!(&v.status, Status::Unknown(r) if r.contains("not the running host")),
        "{v:?}"
    );
    Ok(())
}

// ---------------------------------------------------------------- cache

#[test]
fn the_cache_asks_again_only_after_carrier_changes_moves() -> R {
    let f = c7i_tuned("cache")?;
    let mut cache = cache_with(|k| {
        k.coalesce.insert(3, Ok(adaptive(false)));
        k.rings.insert(3, Ok(ring(4096, 8192)));
    });
    for _ in 0..3 {
        assert_eq!(
            n1_adaptive_rx(&f.host(), "ens6", &mut cache).status,
            Status::Ok
        );
        assert_eq!(n4_rx_ring(&f.host(), "ens6", &mut cache).status, Status::Ok);
    }
    assert_eq!(cache.queries(), 2, "one coalesce and one rings query");
    // An ENA reset: the carrier went down and up, and adaptive moderation
    // is back on.
    f.file("/sys/class/net/ens6/carrier_changes", "4\n")?;
    cache.source_mut().coalesce.insert(3, Ok(adaptive(true)));
    assert!(n1_adaptive_rx(&f.host(), "ens6", &mut cache).is_warning());
    assert_eq!(cache.queries(), 3);
    assert!(n1_adaptive_rx(&f.host(), "ens6", &mut cache).is_warning());
    assert_eq!(cache.queries(), 3);
    // Transient failures are not kept.
    cache
        .source_mut()
        .rings
        .insert(3, Err(EthtoolError::Timeout));
    f.file("/sys/class/net/ens6/carrier_changes", "6\n")?;
    assert!(n4_rx_ring(&f.host(), "ens6", &mut cache).is_unknown());
    assert!(n4_rx_ring(&f.host(), "ens6", &mut cache).is_unknown());
    assert_eq!(cache.queries(), 5);
    assert_eq!(
        cache.source_mut().asked,
        [
            ("coalesce", 3),
            ("rings", 3),
            ("coalesce", 3),
            ("rings", 3),
            ("rings", 3)
        ]
    );
    Ok(())
}

// ---------------------------------------------------------------- N2

#[test]
fn n2_warns_for_deferred_irqs_with_no_busy_poller_on_the_vf_too() -> R {
    let f = azure("n2-azure")?;
    let v = n2_deferred_irqs(&f.host(), "eth1", Some(false));
    let found: Vec<&Finding> = v.iter().filter(|f| f.is_finding()).collect();
    assert_eq!(found.len(), 1, "{v:#?}");
    assert!(found[0].is_warning());
    let m = &found[0].message;
    assert!(m.starts_with("enP30832s1"), "{m}");
    assert!(
        m.contains("napi_defer_hard_irqs 2, gro_flush_timeout 200000 ns"),
        "{m}"
    );
    assert!(m.contains("200 µs timer"), "{m}");
    assert!(
        m.contains("echo 0 > /sys/class/net/enP30832s1/napi_defer_hard_irqs"),
        "{m}"
    );
    assert!(m.contains("turn busy polling on"), "{m}");
    // Busy polling on: the deferral is how it is set up.
    let v = n2_deferred_irqs(&f.host(), "eth1", Some(true));
    assert!(v.len() == 1 && v[0].status == Status::Ok, "{v:#?}");
    // Not known whether the sockets busy-poll.
    let v = n2_deferred_irqs(&f.host(), "eth1", None);
    assert!(v.len() == 1 && v[0].is_unknown(), "{v:#?}");
    Ok(())
}

#[test]
fn n2_does_not_suggest_busy_polling_on_graviton() -> R {
    let f = c9g_tuned("n2-graviton")?;
    f.file("/sys/class/net/ens6/napi_defer_hard_irqs", "2\n")?
        .file("/sys/class/net/ens6/gro_flush_timeout", "200000\n")?;
    let v = n2_deferred_irqs(&f.host(), "ens6", Some(false));
    assert!(v[0].is_warning(), "{v:#?}");
    assert!(
        v[0].message.contains("busy polling is not suggested"),
        "{}",
        v[0].message
    );
    assert!(
        !v[0].message.contains("turn busy polling on"),
        "{}",
        v[0].message
    );
    Ok(())
}

#[test]
fn n2_is_ok_without_deferral_or_on_an_older_kernel() -> R {
    let f = debian_default("n2-ok")?;
    let v = n2_deferred_irqs(&f.host(), "ens5", Some(false));
    assert!(v.len() == 1 && v[0].status == Status::Ok, "{v:#?}");
    f.remove("/sys/class/net/ens5/napi_defer_hard_irqs")?
        .file("/sys/class/net/ens5/gro_flush_timeout", "200000\n")?;
    let v = n2_deferred_irqs(&f.host(), "ens5", Some(false));
    assert!(v.len() == 1 && v[0].status == Status::Ok, "{v:#?}");
    let p = pod("n2-pod")?;
    let v = n2_deferred_irqs(&p.host(), "eth0", Some(false));
    assert!(v.len() == 1 && v[0].is_unknown(), "{v:#?}");
    Ok(())
}

#[test]
fn n2_without_deferral_is_ok_even_when_busy_polling_is_not_known() -> R {
    // Nothing is deferred, so whether the sockets busy-poll does not
    // matter: the data answers the check.
    let f = debian_default("n2-nodefer")?;
    let v = n2_deferred_irqs(&f.host(), "ens5", None);
    assert!(v.len() == 1 && v[0].status == Status::Ok, "{v:#?}");
    // Deferred and not known: unknown, naming what is deferred.
    f.file("/sys/class/net/ens5/napi_defer_hard_irqs", "2\n")?
        .file("/sys/class/net/ens5/gro_flush_timeout", "200000\n")?;
    let v = n2_deferred_irqs(&f.host(), "ens5", None);
    assert!(v.len() == 1 && v[0].is_unknown(), "{v:#?}");
    assert!(v[0].message.contains("ens5"), "{v:#?}");
    // On Azure only the VF defers: the master's line is ok, the VF's
    // unknown.
    let a = azure("n2-azure-unknown")?;
    let v = n2_deferred_irqs(&a.host(), "eth1", None);
    assert!(!v.iter().any(Finding::is_finding), "{v:#?}");
    assert!(
        v.iter()
            .any(|f| f.is_unknown() && f.message.contains("enP30832s1")),
        "{v:#?}"
    );
    Ok(())
}

// ---------------------------------------------------------------- N3

#[test]
fn n3_checks_azure_interfaces_for_a_vf_that_is_up() -> R {
    let f = azure("n3")?;
    assert_eq!(n3_azure_vf(&f.host(), "eth1").status, Status::Ok);
    for (i, why) in [
        ("eth2", "enP4660s1 is down"),
        ("eth3", "no accelerated-networking VF"),
    ] {
        let v = n3_azure_vf(&f.host(), i);
        assert!(v.is_warning(), "{i}: {v:?}");
        assert!(v.message.contains(why), "{}", v.message);
        assert!(v.message.contains("synthetic VMBus path"), "{}", v.message);
        // The command Azure documents, with the arguments it needs, and
        // its precondition (manage-accelerated-networking).
        assert!(
            v.message.contains(
                "az network nic update --name <NIC> --resource-group <resource group> \
                 --accelerated-networking true"
            ),
            "{}",
            v.message
        );
        assert!(v.message.contains("deallocate"), "{}", v.message);
    }
    // Only `up` is up: a VF whose link is below it is not carrying traffic.
    for state in ["lowerlayerdown", "dormant", "notpresent"] {
        f.file("/sys/class/net/enP30832s1/operstate", &format!("{state}\n"))?;
        let v = n3_azure_vf(&f.host(), "eth1");
        assert!(v.is_warning() && v.message.contains(state), "{v:?}");
    }
    let c = c9g_tuned("n3-aws")?;
    let v = n3_azure_vf(&c.host(), "ens6");
    assert_eq!(v.status, Status::Ok, "{v:?}");
    let p = pod("n3-pod")?;
    assert!(n3_azure_vf(&p.host(), "eth0").is_unknown());
    Ok(())
}

// ---------------------------------------------------------------- N4

#[test]
fn n4_warns_below_the_smaller_of_4096_and_the_nics_maximum() -> R {
    let f = c7i_tuned("n4")?;
    let mut cache = cache_with(|k| {
        k.rings.insert(3, Ok(ring(1024, 8192)));
    });
    let v = n4_rx_ring(&f.host(), "ens6", &mut cache);
    assert_eq!((v.id, v.severity), (CheckId::N4, Severity::Warn));
    assert!(v.is_warning(), "{v:?}");
    assert!(v.message.contains("RX ring 1024"), "{}", v.message);
    assert!(
        v.message.contains("ethtool -G ens6 rx 4096"),
        "{}",
        v.message
    );
    assert!(v.message.contains("resets the interface"), "{}", v.message);
    assert!(v.message.contains("not measured here"), "{}", v.message);
    // A NIC that takes at most 256 (MANA) is at its maximum.
    let mut cache = cache_with(|k| {
        k.rings.insert(3, Ok(ring(256, 256)));
    });
    assert_eq!(n4_rx_ring(&f.host(), "ens6", &mut cache).status, Status::Ok);
    let mut cache = cache_with(|k| {
        k.rings.insert(3, Ok(ring(512, 1024)));
    });
    let v = n4_rx_ring(&f.host(), "ens6", &mut cache);
    assert!(v.message.contains("rx 1024"), "{v:?}");
    Ok(())
}

#[test]
fn n4_reads_the_azure_vfs_ring() -> R {
    let f = azure("n4-azure")?;
    let mut cache = cache_with(|k| {
        k.rings.insert(5, Ok(ring(1024, 8192)));
    });
    let v = n4_rx_ring(&f.host(), "eth1", &mut cache);
    assert!(v.is_warning(), "{v:?}");
    assert!(
        v.message.contains("ethtool -G enP30832s1 rx 4096"),
        "{}",
        v.message
    );
    assert!(!v.message.contains("resets the interface"), "{}", v.message);
    assert_eq!(cache.source_mut().asked, [("rings", 5)]);
    // No VF: the synthetic path's ring cannot be read.
    assert!(n4_rx_ring(&f.host(), "eth3", &mut cache).is_unknown());
    Ok(())
}

#[test]
fn n4_is_unknown_without_ring_figures() -> R {
    let f = c7i_tuned("n4-unknown")?;
    let mut cache = cache_with(|k| {
        k.rings.insert(3, Ok(Rings::default()));
    });
    assert!(n4_rx_ring(&f.host(), "ens6", &mut cache).is_unknown());
    let mut cache = cache_with(|k| {
        k.rings.insert(3, Err(EthtoolError::Kernel(EOPNOTSUPP)));
    });
    assert!(n4_rx_ring(&f.host(), "ens6", &mut cache).is_unknown());
    Ok(())
}

#[test]
fn the_netlink_client_answers_only_for_the_running_linux_host() -> R {
    let f = c7i_tuned("netlink")?;
    let mut n = Netlink::new();
    assert_eq!(n.rings(&f.host(), 3), Err(EthtoolError::NotLive));
    if !cfg!(target_os = "linux") {
        assert_eq!(n.coalesce(&Host::system(), 1), Err(EthtoolError::NotLinux));
    }
    Ok(())
}
